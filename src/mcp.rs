use crate::agent::{audit_agent_readiness, score_percent};
use crate::checker::{check_urls, CheckOptions};
use crate::config::load_check_config;
use crate::export::{export_csv, export_html, export_json, export_junit, export_sarif};
use crate::models::{RequestMethod, Summary, UrlCheckResult};
use crate::report::summarize;
use crate::robots::{fetch_robots_rules, filter_allowed_by_robots};
use crate::sitemap::discover_urls_with_retries;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::{Component, Path, PathBuf};
use url::Url;

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const DEFAULT_USER_AGENT: &str = "sitepulse/0.1 (+https://example.local)";

#[derive(Debug, Clone, Default)]
pub struct McpServerOptions {
    pub export_root: Option<PathBuf>,
    pub allow_absolute_export_paths: bool,
}

fn print_mcp_startup_info(options: &McpServerOptions) {
    eprintln!("sitepulse MCP server started");
    eprintln!("Transport: stdio");
    eprintln!("Command: sitepulse mcp");
    eprintln!("Codex MCP config:");
    eprintln!(r#"{{"mcpServers":{{"sitepulse":{{"command":"sitepulse","args":["mcp"]}}}}}}"#);
    eprintln!("Available tools: check_sitemap, agent_ready, validate_config");
    if let Some(export_root) = &options.export_root {
        eprintln!("MCP export root: {}", export_root.display());
    }
    if options.allow_absolute_export_paths {
        eprintln!("Absolute MCP export paths: enabled");
    } else {
        eprintln!("Absolute MCP export paths: disabled");
    }
    eprintln!("Waiting for MCP JSON-RPC messages on stdin...");
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Deserialize)]
struct CheckSitemapArgs {
    sitemap_url: String,
    concurrency: Option<usize>,
    timeout: Option<u64>,
    max_urls: Option<usize>,
    retries: Option<usize>,
    sitemap_retries: Option<usize>,
    delay_ms: Option<u64>,
    dry_run: Option<bool>,
    fail_on_errors: Option<bool>,
    method: Option<String>,
    analyze_meta: Option<bool>,
    same_host_only: Option<bool>,
    respect_robots: Option<bool>,
    agent_ready: Option<bool>,
    user_agent: Option<String>,
    rate_limit_per_second: Option<u64>,
    per_host_concurrency: Option<usize>,
    per_host_rate_limit_per_second: Option<u64>,
    export: Option<PathBuf>,
    export_json: Option<PathBuf>,
    export_html: Option<PathBuf>,
    export_junit: Option<PathBuf>,
    export_sarif: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct AgentReadyArgs {
    site_url: String,
    timeout: Option<u64>,
    user_agent: Option<String>,
    fail_under: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct ValidateConfigArgs {
    path: PathBuf,
}

#[derive(Debug, Serialize)]
struct CheckSitemapOutput {
    discovered_urls: usize,
    checked_urls: usize,
    dry_run: bool,
    failed: bool,
    failure_reason: Option<String>,
    exported_files: Vec<String>,
    summary: Summary,
    results: Vec<UrlCheckResult>,
    agent_readiness: Option<crate::agent::AgentReadinessReport>,
}

pub async fn run_mcp_server(options: McpServerOptions) -> Result<()> {
    print_mcp_startup_info(&options);
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.context("failed to read MCP stdin")?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => handle_request(request, &options).await,
            Err(error) => Some(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": format!("Parse error: {error}") }
            })),
        };

        if let Some(response) = response {
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }
    }

    Ok(())
}

async fn handle_request(request: JsonRpcRequest, options: &McpServerOptions) -> Option<Value> {
    let id = request.id.clone();
    let result = match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "sitepulse", "version": env!("CARGO_PKG_VERSION") }
        })),
        "notifications/initialized" => return None,
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => call_tool(request.params, options).await,
        _ => Err(json!({ "code": -32601, "message": "Method not found" })),
    };

    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
    })
}

async fn call_tool(params: Value, options: &McpServerOptions) -> std::result::Result<Value, Value> {
    let params: ToolCallParams = serde_json::from_value(params)
        .map_err(|error| rpc_error(-32602, &format!("Invalid tool call params: {error}")))?;

    let output = match params.name.as_str() {
        "check_sitemap" => {
            let args: CheckSitemapArgs = parse_args(params.arguments)?;
            serde_json::to_value(run_check_sitemap(args, options).await.map_err(tool_error)?)
                .map_err(|error| rpc_error(-32603, &error.to_string()))?
        }
        "agent_ready" => {
            let args: AgentReadyArgs = parse_args(params.arguments)?;
            run_agent_ready(args).await.map_err(tool_error)?
        }
        "validate_config" => {
            let args: ValidateConfigArgs = parse_args(params.arguments)?;
            load_check_config(&args.path).map_err(tool_error)?;
            json!({ "valid": true, "path": args.path })
        }
        _ => return Err(rpc_error(-32602, "Unknown tool")),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&output).unwrap_or_else(|_| output.to_string())
        }]
    }))
}

fn parse_args<T: for<'de> Deserialize<'de>>(value: Value) -> std::result::Result<T, Value> {
    serde_json::from_value(value)
        .map_err(|error| rpc_error(-32602, &format!("Invalid arguments: {error}")))
}

async fn run_check_sitemap(
    args: CheckSitemapArgs,
    options: &McpServerOptions,
) -> Result<CheckSitemapOutput> {
    let timeout = args.timeout.unwrap_or(10);
    let user_agent = args
        .user_agent
        .clone()
        .unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());
    let mut urls = discover_urls_with_retries(
        &args.sitemap_url,
        timeout,
        args.sitemap_retries.unwrap_or(2),
        &user_agent,
    )
    .await?;
    let discovered_urls = urls.len();

    if args.same_host_only.unwrap_or(false) {
        urls = filter_same_host(urls, &args.sitemap_url)?;
    }
    if args.respect_robots.unwrap_or(false) {
        let rules = fetch_robots_rules(&args.sitemap_url, timeout, &user_agent).await?;
        urls = filter_allowed_by_robots(urls, &rules);
    }
    if let Some(max_urls) = args.max_urls {
        urls.truncate(max_urls);
    }

    if args.dry_run.unwrap_or(false) {
        return Ok(CheckSitemapOutput {
            discovered_urls,
            checked_urls: 0,
            dry_run: true,
            failed: false,
            failure_reason: None,
            exported_files: Vec::new(),
            summary: summarize(&[]),
            results: Vec::new(),
            agent_readiness: None,
        });
    }

    let method = match args
        .method
        .as_deref()
        .unwrap_or("get")
        .to_ascii_lowercase()
        .as_str()
    {
        "head" => RequestMethod::Head,
        _ => RequestMethod::Get,
    };
    let results = check_urls(
        &urls,
        CheckOptions {
            concurrency: args.concurrency.unwrap_or(10),
            timeout_secs: timeout,
            retries: args.retries.unwrap_or(0),
            method,
            analyze_meta: args.analyze_meta.unwrap_or(false),
            user_agent: &user_agent,
            delay_ms: args.delay_ms.unwrap_or(0),
            rate_limit_per_second: args.rate_limit_per_second,
            per_host_concurrency: args.per_host_concurrency,
            per_host_rate_limit_per_second: args.per_host_rate_limit_per_second,
        },
    )
    .await;
    let summary = summarize(&results);
    let mut exported_files = Vec::new();
    export_mcp_reports(&args, &results, &summary, &mut exported_files, options)?;

    let agent_readiness = if args.agent_ready.unwrap_or(false) {
        Some(audit_agent_readiness(&args.sitemap_url, timeout, &user_agent).await?)
    } else {
        None
    };
    let failed =
        args.fail_on_errors.unwrap_or(false) && results.iter().any(UrlCheckResult::is_error);
    let failure_reason = failed.then(|| "URL errors found".to_string());

    Ok(CheckSitemapOutput {
        discovered_urls,
        checked_urls: results.len(),
        dry_run: false,
        failed,
        failure_reason,
        exported_files,
        summary,
        results,
        agent_readiness,
    })
}

fn export_mcp_reports(
    args: &CheckSitemapArgs,
    results: &[UrlCheckResult],
    summary: &Summary,
    exported_files: &mut Vec<String>,
    options: &McpServerOptions,
) -> Result<()> {
    if let Some(path) = args.export.as_deref() {
        let path = validate_export_path(path, options)?;
        export_csv(&path, results)?;
        exported_files.push(path_display(&path));
    }
    if let Some(path) = args.export_json.as_deref() {
        let path = validate_export_path(path, options)?;
        export_json(&path, results)?;
        exported_files.push(path_display(&path));
    }
    if let Some(path) = args.export_html.as_deref() {
        let path = validate_export_path(path, options)?;
        export_html(&path, results, summary)?;
        exported_files.push(path_display(&path));
    }
    if let Some(path) = args.export_junit.as_deref() {
        let path = validate_export_path(path, options)?;
        export_junit(&path, results)?;
        exported_files.push(path_display(&path));
    }
    if let Some(path) = args.export_sarif.as_deref() {
        let path = validate_export_path(path, options)?;
        export_sarif(&path, results)?;
        exported_files.push(path_display(&path));
    }
    Ok(())
}

fn validate_export_path(path: &Path, options: &McpServerOptions) -> Result<PathBuf> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!("MCP export paths must not contain '..'");
    }

    if path.is_absolute() {
        if !options.allow_absolute_export_paths {
            anyhow::bail!("absolute MCP export paths are disabled; use --allow-absolute-export-paths to enable them");
        }
        if let Some(root) = &options.export_root {
            let root = root.canonicalize().unwrap_or_else(|_| root.clone());
            if !path.starts_with(&root) {
                anyhow::bail!("absolute MCP export path must be inside --export-root");
            }
        }
        return Ok(path.to_path_buf());
    }

    Ok(match &options.export_root {
        Some(root) => root.join(path),
        None => path.to_path_buf(),
    })
}

fn path_display(path: &Path) -> String {
    path.display().to_string()
}

async fn run_agent_ready(args: AgentReadyArgs) -> Result<Value> {
    let timeout = args.timeout.unwrap_or(10);
    let user_agent = args
        .user_agent
        .unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());
    let report = audit_agent_readiness(&args.site_url, timeout, &user_agent).await?;
    let percent = score_percent(&report);
    Ok(json!({
        "report": report,
        "score_percent": percent,
        "passed_threshold": args.fail_under.map(|threshold| percent >= threshold)
    }))
}

fn tool_error(error: anyhow::Error) -> Value {
    rpc_error(-32603, &error.to_string())
}

fn rpc_error(code: i64, message: &str) -> Value {
    json!({ "code": code, "message": message })
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "check_sitemap",
            "description": "Discover URLs from a sitemap, check URL health, and optionally run agent readiness.",
            "inputSchema": {
                "type": "object",
                "required": ["sitemap_url"],
                "properties": {
                    "sitemap_url": { "type": "string" },
                    "max_urls": { "type": "integer", "minimum": 1 },
                    "concurrency": { "type": "integer", "minimum": 1 },
                    "timeout": { "type": "integer", "minimum": 1 },
                    "method": { "type": "string", "enum": ["get", "head"] },
                    "analyze_meta": { "type": "boolean" },
                    "same_host_only": { "type": "boolean" },
                    "respect_robots": { "type": "boolean" },
                    "agent_ready": { "type": "boolean" },
                    "delay_ms": { "type": "integer", "minimum": 0 },
                    "dry_run": { "type": "boolean" },
                    "fail_on_errors": { "type": "boolean" },
                    "rate_limit_per_second": { "type": "integer", "minimum": 1 },
                    "per_host_concurrency": { "type": "integer", "minimum": 1 },
                    "per_host_rate_limit_per_second": { "type": "integer", "minimum": 1 },
                    "export": { "type": "string" },
                    "export_json": { "type": "string" },
                    "export_html": { "type": "string" },
                    "export_junit": { "type": "string" },
                    "export_sarif": { "type": "string" }
                }
            }
        },
        {
            "name": "agent_ready",
            "description": "Run an agent readiness audit for a site URL.",
            "inputSchema": {
                "type": "object",
                "required": ["site_url"],
                "properties": {
                    "site_url": { "type": "string" },
                    "timeout": { "type": "integer", "minimum": 1 },
                    "fail_under": { "type": "integer", "minimum": 0, "maximum": 100 }
                }
            }
        },
        {
            "name": "validate_config",
            "description": "Validate a sitepulse JSON config file.",
            "inputSchema": {
                "type": "object",
                "required": ["path"],
                "properties": { "path": { "type": "string" } }
            }
        }
    ])
}

fn filter_same_host(urls: Vec<String>, sitemap_url: &str) -> Result<Vec<String>> {
    let sitemap_host = Url::parse(sitemap_url)
        .context("invalid sitemap URL")?
        .host_str()
        .context("sitemap URL does not contain a host")?
        .to_ascii_lowercase();

    Ok(urls
        .into_iter()
        .filter(|url| {
            Url::parse(url)
                .ok()
                .and_then(|parsed| {
                    parsed
                        .host_str()
                        .map(|host| host.eq_ignore_ascii_case(&sitemap_host))
                })
                .unwrap_or(false)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_dir_export_paths() {
        let error = validate_export_path(Path::new("../report.json"), &McpServerOptions::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("must not contain"));
    }

    #[test]
    fn joins_relative_export_paths_to_root() {
        let options = McpServerOptions {
            export_root: Some(PathBuf::from("reports")),
            allow_absolute_export_paths: false,
        };
        assert_eq!(
            validate_export_path(Path::new("report.json"), &options).unwrap(),
            PathBuf::from("reports/report.json")
        );
    }

    #[tokio::test]
    async fn handles_tools_list_request() {
        let request = JsonRpcRequest {
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: Value::Null,
        };
        let response = handle_request(request, &McpServerOptions::default())
            .await
            .unwrap();
        assert!(response["result"]["tools"].as_array().unwrap().len() >= 3);
    }
}
