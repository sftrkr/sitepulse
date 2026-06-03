use crate::agent::{audit_agent_readiness, score_percent};
use crate::checker::{check_urls, CheckOptions};
use crate::config::load_check_config;
use crate::models::{RequestMethod, Summary, UrlCheckResult};
use crate::report::summarize;
use crate::robots::{fetch_robots_rules, filter_allowed_by_robots};
use crate::sitemap::discover_urls_with_retries;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use url::Url;

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const DEFAULT_USER_AGENT: &str = "sitepulse/0.1 (+https://example.local)";

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
    method: Option<String>,
    analyze_meta: Option<bool>,
    same_host_only: Option<bool>,
    respect_robots: Option<bool>,
    agent_ready: Option<bool>,
    user_agent: Option<String>,
    rate_limit_per_second: Option<u64>,
    per_host_concurrency: Option<usize>,
    per_host_rate_limit_per_second: Option<u64>,
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
    summary: Summary,
    results: Vec<UrlCheckResult>,
    agent_readiness: Option<crate::agent::AgentReadinessReport>,
}

pub async fn run_mcp_server() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.context("failed to read MCP stdin")?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => handle_request(request).await,
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

async fn handle_request(request: JsonRpcRequest) -> Option<Value> {
    let id = request.id.clone();
    let result = match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "sitepulse", "version": env!("CARGO_PKG_VERSION") }
        })),
        "notifications/initialized" => return None,
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => call_tool(request.params).await,
        _ => Err(json!({ "code": -32601, "message": "Method not found" })),
    };

    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
    })
}

async fn call_tool(params: Value) -> std::result::Result<Value, Value> {
    let params: ToolCallParams = serde_json::from_value(params)
        .map_err(|error| rpc_error(-32602, &format!("Invalid tool call params: {error}")))?;

    let output = match params.name.as_str() {
        "check_sitemap" => {
            let args: CheckSitemapArgs = parse_args(params.arguments)?;
            serde_json::to_value(run_check_sitemap(args).await.map_err(tool_error)?)
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

async fn run_check_sitemap(args: CheckSitemapArgs) -> Result<CheckSitemapOutput> {
    let timeout = args.timeout.unwrap_or(10);
    let user_agent = args
        .user_agent
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
            delay_ms: 0,
            rate_limit_per_second: args.rate_limit_per_second,
            per_host_concurrency: args.per_host_concurrency,
            per_host_rate_limit_per_second: args.per_host_rate_limit_per_second,
        },
    )
    .await;
    let summary = summarize(&results);
    let agent_readiness = if args.agent_ready.unwrap_or(false) {
        Some(audit_agent_readiness(&args.sitemap_url, timeout, &user_agent).await?)
    } else {
        None
    };

    Ok(CheckSitemapOutput {
        discovered_urls,
        checked_urls: results.len(),
        summary,
        results,
        agent_readiness,
    })
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
                    "agent_ready": { "type": "boolean" }
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

    #[tokio::test]
    async fn handles_tools_list_request() {
        let request = JsonRpcRequest {
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: Value::Null,
        };
        let response = handle_request(request).await.unwrap();
        assert!(response["result"]["tools"].as_array().unwrap().len() >= 3);
    }
}
