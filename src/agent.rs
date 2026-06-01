use crate::meta::extract_page_meta;
use anyhow::{Context, Result};
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use reqwest::header::{HeaderMap, HeaderName, ACCEPT, CONTENT_TYPE, LINK};
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use url::Url;

const USER_AGENT: &str = "sitepulse/0.1 (+https://example.local)";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum AgentCheckStatus {
    Pass,
    Warn,
    Fail,
}

impl AgentCheckStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReadinessCheck {
    pub status: AgentCheckStatus,
    pub name: String,
    pub message: String,
    pub points: u8,
    pub max_points: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReadinessReport {
    pub site_url: String,
    pub score: u16,
    pub max_score: u16,
    pub checks: Vec<AgentReadinessCheck>,
}

pub async fn audit_agent_readiness(
    sitemap_url: &str,
    timeout_secs: u64,
) -> Result<AgentReadinessReport> {
    let site_url = site_root_url(sitemap_url)?;
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("failed to build HTTP client")?;

    let robots = fetch_optional_text(&client, site_url.join("/robots.txt")?.as_str()).await;
    let llms = fetch_optional_text(&client, site_url.join("/llms.txt")?.as_str()).await;
    let llms_full = fetch_optional_text(&client, site_url.join("/llms-full.txt")?.as_str()).await;
    let homepage = fetch_optional_text(&client, site_url.as_str()).await;
    let homepage_headers = fetch_homepage_headers(&client, site_url.as_str()).await;
    let markdown = fetch_markdown_negotiation(&client, site_url.as_str()).await;
    let protocol_discovery = fetch_protocol_discovery(&client, &site_url).await;
    let commerce_discovery = fetch_commerce_discovery(&client, &site_url).await;
    let dns_aid = fetch_dns_aid(&site_url).await;
    let mut checks = Vec::new();

    checks.push(match &robots {
        FetchResult::Ok(body) if !body.trim().is_empty() => {
            check_pass("robots.txt", "robots.txt is accessible", 10)
        }
        FetchResult::Ok(_) => check_warn("robots.txt", "robots.txt is accessible but empty", 5, 10),
        FetchResult::HttpStatus(status) => check_warn(
            "robots.txt",
            &format!("robots.txt returned HTTP {status}"),
            0,
            10,
        ),
        FetchResult::NetworkError(error) => check_warn(
            "robots.txt",
            &format!("robots.txt could not be fetched: {error}"),
            0,
            10,
        ),
    });

    if let FetchResult::Ok(body) = &robots {
        checks.push(if robots_group_disallows_root(body, "*") {
            check_fail(
                "Crawler access",
                "User-agent * is blocked with Disallow: /",
                0,
                15,
            )
        } else {
            check_pass(
                "Crawler access",
                "generic crawlers are not globally blocked",
                15,
            )
        });
        checks.push(if has_sitemap_directive(body) {
            check_pass(
                "Sitemap directive",
                "robots.txt declares at least one sitemap",
                10,
            )
        } else {
            check_warn(
                "Sitemap directive",
                "robots.txt does not declare a sitemap",
                0,
                10,
            )
        });
        checks.push(check_ai_bot_rules(body));
        checks.push(check_ai_bot_access(body));
    } else {
        checks.push(check_warn(
            "Crawler access",
            "robots.txt unavailable, crawler access rules could not be evaluated",
            0,
            15,
        ));
        checks.push(check_warn(
            "Sitemap directive",
            "robots.txt unavailable, sitemap directives could not be evaluated",
            0,
            10,
        ));
        checks.push(check_warn(
            "AI bot rules",
            "robots.txt unavailable, AI bot rules could not be evaluated",
            0,
            10,
        ));
        checks.push(check_warn(
            "AI bot access",
            "robots.txt unavailable, known AI bot access could not be evaluated",
            0,
            15,
        ));
    }

    checks.push(check_text_file("llms.txt", &llms, 20));
    checks.push(check_text_file("llms-full.txt", &llms_full, 10));
    checks.push(check_link_headers(&homepage_headers));
    checks.push(check_dns_aid(&dns_aid));
    checks.push(check_content_signals(
        &homepage_headers,
        homepage_body(&homepage),
    ));
    checks.push(check_web_bot_auth(&homepage_headers));
    checks.push(check_markdown_negotiation(&markdown));
    checks.extend(check_protocol_discovery(&protocol_discovery));
    checks.extend(check_commerce_discovery(&commerce_discovery));

    match &homepage {
        FetchResult::Ok(body) => {
            checks.push(check_pass("Homepage", "homepage is accessible", 5));
            let meta = extract_page_meta(body);
            checks.push(if meta.title.is_some() {
                check_pass("Homepage title", "homepage title found", 10)
            } else {
                check_warn("Homepage title", "homepage title missing", 0, 10)
            });
            checks.push(if meta.description.is_some() {
                check_pass("Meta description", "homepage meta description found", 10)
            } else {
                check_warn(
                    "Meta description",
                    "homepage meta description missing",
                    0,
                    10,
                )
            });
            checks.push(if meta.canonical_url.is_some() {
                check_pass("Canonical URL", "homepage canonical URL found", 10)
            } else {
                check_warn("Canonical URL", "homepage canonical URL missing", 0, 10)
            });
            checks.push(check_open_graph(body));
            checks.push(if has_json_ld(body) {
                check_pass("JSON-LD", "JSON-LD structured data found", 10)
            } else {
                check_warn("JSON-LD", "JSON-LD structured data missing", 0, 10)
            });
            checks.push(if has_tag(body, "main") {
                check_pass("Semantic HTML", "<main> element found", 5)
            } else {
                check_warn("Semantic HTML", "<main> element missing", 0, 5)
            });
            checks.push(if has_tag(body, "h1") {
                check_pass("H1", "<h1> element found", 5)
            } else {
                check_warn("H1", "<h1> element missing", 0, 5)
            });
        }
        FetchResult::HttpStatus(status) => checks.push(check_fail(
            "Homepage",
            &format!("homepage returned HTTP {status}"),
            0,
            55,
        )),
        FetchResult::NetworkError(error) => checks.push(check_fail(
            "Homepage",
            &format!("homepage could not be fetched: {error}"),
            0,
            55,
        )),
    }

    let score = checks.iter().map(|c| c.points as u16).sum();
    let max_score = checks.iter().map(|c| c.max_points as u16).sum();
    Ok(AgentReadinessReport {
        site_url: site_url.to_string(),
        score,
        max_score,
        checks,
    })
}

pub fn score_percent(report: &AgentReadinessReport) -> u8 {
    if report.max_score == 0 {
        0
    } else {
        ((report.score as f32 / report.max_score as f32) * 100.0).round() as u8
    }
}

pub fn print_agent_readiness_report(report: &AgentReadinessReport) {
    println!("\nAgent Readiness:");
    println!("Site: {}", report.site_url);
    println!(
        "Score: {}/{} ({}%)\n",
        report.score,
        report.max_score,
        score_percent(report)
    );
    for c in &report.checks {
        println!(
            "{:<4} {:<20} {} ({}/{})",
            c.status.as_str(),
            c.name,
            c.message,
            c.points,
            c.max_points
        );
    }
}

#[derive(Debug)]
enum FetchResult {
    Ok(String),
    HttpStatus(u16),
    NetworkError(String),
}

async fn fetch_optional_text(client: &Client, url: &str) -> FetchResult {
    match client.get(url).send().await {
        Ok(r) if r.status().is_success() => match r.text().await {
            Ok(b) => FetchResult::Ok(b),
            Err(e) => FetchResult::NetworkError(e.to_string()),
        },
        Ok(r) => FetchResult::HttpStatus(r.status().as_u16()),
        Err(e) => FetchResult::NetworkError(e.to_string()),
    }
}

#[derive(Debug)]
struct DnsAidResult {
    records: Vec<String>,
    checked_names: Vec<String>,
    error: Option<String>,
}

async fn fetch_dns_aid(site_url: &Url) -> DnsAidResult {
    let Some(host) = site_url.host_str() else {
        return DnsAidResult {
            records: Vec::new(),
            checked_names: Vec::new(),
            error: Some("site URL has no host".to_string()),
        };
    };

    let names = vec![
        format!("_agent.{host}"),
        format!("_agents.{host}"),
        format!("_ai.{host}"),
    ];
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
    let mut records = Vec::new();
    let mut last_error = None;

    for name in &names {
        match resolver.txt_lookup(name.as_str()).await {
            Ok(lookup) => {
                for record in lookup.iter() {
                    let value = record
                        .txt_data()
                        .iter()
                        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
                        .collect::<Vec<_>>()
                        .join("");
                    if !value.trim().is_empty() {
                        records.push(format!("{name}: {value}"));
                    }
                }
            }
            Err(error) => last_error = Some(error.to_string()),
        }
    }

    DnsAidResult {
        records,
        checked_names: names,
        error: last_error,
    }
}

fn check_dns_aid(result: &DnsAidResult) -> AgentReadinessCheck {
    if !result.records.is_empty() {
        check_pass(
            "DNS-AID",
            &format!(
                "DNS AI discovery TXT record(s) found: {}",
                result.records.len()
            ),
            10,
        )
    } else if let Some(error) = &result.error {
        check_warn(
            "DNS-AID",
            &format!(
                "no DNS AI discovery TXT records found across {} name(s); last resolver message: {}",
                result.checked_names.len(),
                error
            ),
            0,
            10,
        )
    } else {
        check_warn(
            "DNS-AID",
            &format!(
                "no DNS AI discovery TXT records found across {} name(s)",
                result.checked_names.len()
            ),
            0,
            10,
        )
    }
}

#[derive(Debug)]
struct HeaderFetchResult {
    status: Option<u16>,
    links: Vec<String>,
    content_signals: Vec<String>,
    x_robots_tag: Vec<String>,
    web_bot_auth: Vec<String>,
    www_authenticate: Vec<String>,
    error: Option<String>,
}

#[derive(Debug)]
struct MarkdownFetchResult {
    status: Option<u16>,
    content_type: Option<String>,
    body: Option<String>,
    error: Option<String>,
}

async fn fetch_homepage_headers(client: &Client, url: &str) -> HeaderFetchResult {
    match client.head(url).send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let headers = response.headers();
            let links = header_values(headers, LINK);
            let content_signals = header_values_by_name(headers, "content-signals");
            let x_robots_tag = header_values_by_name(headers, "x-robots-tag");
            let web_bot_auth = header_values_by_name(headers, "web-bot-auth");
            let www_authenticate = header_values_by_name(headers, "www-authenticate");
            HeaderFetchResult {
                status: Some(status),
                links,
                content_signals,
                x_robots_tag,
                web_bot_auth,
                www_authenticate,
                error: None,
            }
        }
        Err(error) => HeaderFetchResult {
            status: None,
            links: Vec::new(),
            content_signals: Vec::new(),
            x_robots_tag: Vec::new(),
            web_bot_auth: Vec::new(),
            www_authenticate: Vec::new(),
            error: Some(error.to_string()),
        },
    }
}

async fn fetch_markdown_negotiation(client: &Client, url: &str) -> MarkdownFetchResult {
    match client
        .get(url)
        .header(ACCEPT, "text/markdown, text/plain;q=0.9, */*;q=0.1")
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status().as_u16();
            let content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            let body = response.text().await.ok();
            MarkdownFetchResult {
                status: Some(status),
                content_type,
                body,
                error: None,
            }
        }
        Err(error) => MarkdownFetchResult {
            status: None,
            content_type: None,
            body: None,
            error: Some(error.to_string()),
        },
    }
}

fn header_values(headers: &HeaderMap, name: HeaderName) -> Vec<String> {
    headers
        .get_all(name)
        .iter()
        .filter_map(|value| value.to_str().ok().map(str::to_string))
        .collect()
}

fn header_values_by_name(headers: &HeaderMap, name: &str) -> Vec<String> {
    let Ok(name) = HeaderName::from_bytes(name.as_bytes()) else {
        return Vec::new();
    };
    header_values(headers, name)
}

fn homepage_body(homepage: &FetchResult) -> Option<&str> {
    match homepage {
        FetchResult::Ok(body) => Some(body.as_str()),
        _ => None,
    }
}

fn check_content_signals(
    result: &HeaderFetchResult,
    homepage_body: Option<&str>,
) -> AgentReadinessCheck {
    let meta_signals = homepage_body
        .map(has_content_signal_metadata)
        .unwrap_or(false);
    let header_count = result.content_signals.len() + result.x_robots_tag.len();

    if header_count > 0 || meta_signals {
        let source = match (header_count > 0, meta_signals) {
            (true, true) => "headers and metadata",
            (true, false) => "headers",
            (false, true) => "metadata",
            (false, false) => "unknown",
        };
        check_pass(
            "Content Signals",
            &format!("content access signals found in {source}"),
            10,
        )
    } else if let Some(error) = &result.error {
        check_warn(
            "Content Signals",
            &format!("content signals could not be checked: {error}"),
            0,
            10,
        )
    } else {
        check_warn(
            "Content Signals",
            "no Content Signals, X-Robots-Tag, or robots metadata found",
            0,
            10,
        )
    }
}

fn has_content_signal_metadata(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("name=\"robots\"")
        || lower.contains("name='robots'")
        || lower.contains("name=\"googlebot\"")
        || lower.contains("name='googlebot'")
        || lower.contains("name=\"ai-policy\"")
        || lower.contains("name='ai-policy'")
        || lower.contains("tdm-reservation")
        || lower.contains("noai")
        || lower.contains("noimageai")
}

fn check_web_bot_auth(result: &HeaderFetchResult) -> AgentReadinessCheck {
    let link_signal = result
        .links
        .iter()
        .any(|value| value.to_ascii_lowercase().contains("web-bot-auth"));
    let www_auth_signal = result.www_authenticate.iter().any(|value| {
        let lower = value.to_ascii_lowercase();
        lower.contains("bot") || lower.contains("http-message-signatures")
    });
    let explicit_header_signal = !result.web_bot_auth.is_empty();

    if explicit_header_signal || www_auth_signal || link_signal {
        let mut sources = Vec::new();
        if explicit_header_signal {
            sources.push("Web-Bot-Auth header");
        }
        if www_auth_signal {
            sources.push("WWW-Authenticate header");
        }
        if link_signal {
            sources.push("Link header");
        }
        check_pass(
            "Web Bot Auth",
            &format!("Web Bot Auth signal found via {}", sources.join(", ")),
            10,
        )
    } else if let Some(error) = &result.error {
        check_warn(
            "Web Bot Auth",
            &format!("Web Bot Auth signals could not be checked: {error}"),
            0,
            10,
        )
    } else {
        check_warn(
            "Web Bot Auth",
            "no Web Bot Auth signal found in homepage headers",
            0,
            10,
        )
    }
}

fn check_link_headers(result: &HeaderFetchResult) -> AgentReadinessCheck {
    if !result.links.is_empty() {
        check_pass(
            "Link headers",
            &format!("homepage exposes {} Link header(s)", result.links.len()),
            10,
        )
    } else if let Some(error) = &result.error {
        check_warn(
            "Link headers",
            &format!("homepage Link headers could not be checked: {error}"),
            0,
            10,
        )
    } else if let Some(status) = result.status {
        check_warn(
            "Link headers",
            &format!("homepage returned HTTP {status} with no Link headers"),
            0,
            10,
        )
    } else {
        check_warn("Link headers", "homepage exposes no Link headers", 0, 10)
    }
}

fn check_markdown_negotiation(result: &MarkdownFetchResult) -> AgentReadinessCheck {
    let content_type_is_markdown = result
        .content_type
        .as_deref()
        .map(|value| value.to_ascii_lowercase().contains("markdown"))
        .unwrap_or(false);
    let body_looks_markdown = result
        .body
        .as_deref()
        .map(looks_like_markdown)
        .unwrap_or(false);
    if content_type_is_markdown || body_looks_markdown {
        check_pass(
            "Markdown negotiation",
            "homepage returns Markdown-like content for text/markdown Accept",
            15,
        )
    } else if let Some(error) = &result.error {
        check_warn(
            "Markdown negotiation",
            &format!("Markdown negotiation could not be checked: {error}"),
            0,
            15,
        )
    } else if let Some(status) = result.status {
        check_warn(
            "Markdown negotiation",
            &format!("homepage returned HTTP {status} but not Markdown content"),
            0,
            15,
        )
    } else {
        check_warn(
            "Markdown negotiation",
            "homepage did not return Markdown content",
            0,
            15,
        )
    }
}

fn looks_like_markdown(body: &str) -> bool {
    let trimmed = body.trim_start().to_ascii_lowercase();
    !trimmed.starts_with("<!doctype html")
        && !trimmed.starts_with("<html")
        && (trimmed.starts_with('#')
            || trimmed.contains(
                "
#",
            )
            || trimmed.contains(
                "
- ",
            ))
}

#[derive(Debug)]
struct ProtocolEndpointResult {
    name: &'static str,
    path: &'static str,
    status: Option<u16>,
    error: Option<String>,
}

async fn fetch_protocol_discovery(client: &Client, site_url: &Url) -> Vec<ProtocolEndpointResult> {
    let endpoints = [
        ("MCP Server Card", "/.well-known/mcp.json"),
        ("Agent Skills", "/.well-known/agent-skills.json"),
        ("WebMCP", "/.well-known/webmcp.json"),
        ("A2A Agent Card", "/.well-known/agent.json"),
        ("API catalog", "/.well-known/api-catalog.json"),
        ("OAuth discovery", "/.well-known/oauth-authorization-server"),
        (
            "OAuth Protected Resource",
            "/.well-known/oauth-protected-resource",
        ),
        ("auth.md", "/auth.md"),
    ];
    let mut results = Vec::new();
    for (name, path) in endpoints {
        let url = match site_url.join(path) {
            Ok(url) => url,
            Err(error) => {
                results.push(ProtocolEndpointResult {
                    name,
                    path,
                    status: None,
                    error: Some(error.to_string()),
                });
                continue;
            }
        };
        match client.head(url.as_str()).send().await {
            Ok(response) => results.push(ProtocolEndpointResult {
                name,
                path,
                status: Some(response.status().as_u16()),
                error: None,
            }),
            Err(error) => results.push(ProtocolEndpointResult {
                name,
                path,
                status: None,
                error: Some(error.to_string()),
            }),
        }
    }
    results
}

fn check_protocol_discovery(results: &[ProtocolEndpointResult]) -> Vec<AgentReadinessCheck> {
    results
        .iter()
        .map(|result| match result.status {
            Some(200..=299) => check_pass(result.name, &format!("{} is available", result.path), 5),
            Some(status) => check_warn(
                result.name,
                &format!("{} returned HTTP {status}", result.path),
                0,
                5,
            ),
            None => check_warn(
                result.name,
                &format!(
                    "{} could not be checked: {}",
                    result.path,
                    result.error.as_deref().unwrap_or("unknown error")
                ),
                0,
                5,
            ),
        })
        .collect()
}

#[derive(Debug)]
struct CommerceEndpointResult {
    name: &'static str,
    paths: Vec<&'static str>,
    matched_path: Option<&'static str>,
    status: Option<u16>,
    error: Option<String>,
}

async fn fetch_commerce_discovery(client: &Client, site_url: &Url) -> Vec<CommerceEndpointResult> {
    let standards: [(&str, &[&str]); 4] = [
        (
            "x402",
            &["/.well-known/x402", "/.well-known/x402.json", "/x402"],
        ),
        (
            "MPP",
            &["/.well-known/mpp", "/.well-known/mpp.json", "/mpp"],
        ),
        (
            "UCP",
            &["/.well-known/ucp", "/.well-known/ucp.json", "/ucp"],
        ),
        (
            "ACP",
            &["/.well-known/acp", "/.well-known/acp.json", "/acp"],
        ),
    ];

    let mut results = Vec::new();
    for (name, paths) in standards {
        let mut last_status = None;
        let mut last_error = None;
        let mut matched_path = None;

        for path in paths {
            let url = match site_url.join(path) {
                Ok(url) => url,
                Err(error) => {
                    last_error = Some(error.to_string());
                    continue;
                }
            };

            match client.head(url.as_str()).send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    last_status = Some(status);
                    if (200..=299).contains(&status) {
                        matched_path = Some(*path);
                        break;
                    }
                }
                Err(error) => last_error = Some(error.to_string()),
            }
        }

        results.push(CommerceEndpointResult {
            name,
            paths: paths.to_vec(),
            matched_path,
            status: last_status,
            error: last_error,
        });
    }

    results
}

fn check_commerce_discovery(results: &[CommerceEndpointResult]) -> Vec<AgentReadinessCheck> {
    results
        .iter()
        .map(|result| {
            if let Some(path) = result.matched_path {
                check_pass(
                    result.name,
                    &format!("agentic commerce signal found at {path}"),
                    5,
                )
            } else if let Some(status) = result.status {
                check_warn(
                    result.name,
                    &format!("no agentic commerce signal found; last checked status HTTP {status}"),
                    0,
                    5,
                )
            } else {
                check_warn(
                    result.name,
                    &format!(
                        "agentic commerce signal could not be checked across {} path(s): {}",
                        result.paths.len(),
                        result.error.as_deref().unwrap_or("unknown error")
                    ),
                    0,
                    5,
                )
            }
        })
        .collect()
}

fn site_root_url(input: &str) -> Result<Url> {
    let mut url = Url::parse(input).context("invalid sitemap URL")?;
    url.set_path("/");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

fn check_pass(name: &str, message: &str, points: u8) -> AgentReadinessCheck {
    AgentReadinessCheck {
        status: AgentCheckStatus::Pass,
        name: name.into(),
        message: message.into(),
        points,
        max_points: points,
    }
}
fn check_warn(name: &str, message: &str, points: u8, max_points: u8) -> AgentReadinessCheck {
    AgentReadinessCheck {
        status: AgentCheckStatus::Warn,
        name: name.into(),
        message: message.into(),
        points,
        max_points,
    }
}
fn check_fail(name: &str, message: &str, points: u8, max_points: u8) -> AgentReadinessCheck {
    AgentReadinessCheck {
        status: AgentCheckStatus::Fail,
        name: name.into(),
        message: message.into(),
        points,
        max_points,
    }
}
fn check_text_file(name: &str, result: &FetchResult, max_points: u8) -> AgentReadinessCheck {
    match result {
        FetchResult::Ok(body) if !body.trim().is_empty() => {
            check_pass(name, &format!("{name} is available"), max_points)
        }
        FetchResult::Ok(_) => check_warn(
            name,
            &format!("{name} is available but empty"),
            max_points / 2,
            max_points,
        ),
        FetchResult::HttpStatus(status) => check_warn(
            name,
            &format!("{name} returned HTTP {status}"),
            0,
            max_points,
        ),
        FetchResult::NetworkError(error) => check_warn(
            name,
            &format!("{name} could not be fetched: {error}"),
            0,
            max_points,
        ),
    }
}
fn has_sitemap_directive(body: &str) -> bool {
    body.lines()
        .any(|l| l.trim_start().to_ascii_lowercase().starts_with("sitemap:"))
}
const KNOWN_AI_AGENTS: &[&str] = &[
    "gptbot",
    "chatgpt-user",
    "claudebot",
    "claude-user",
    "perplexitybot",
    "google-extended",
    "ccbot",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RobotAccess {
    Allowed,
    Blocked,
    Unspecified,
}

fn check_ai_bot_access(body: &str) -> AgentReadinessCheck {
    let blocked = KNOWN_AI_AGENTS
        .iter()
        .filter(|agent| robot_access_for_agent(body, agent) == RobotAccess::Blocked)
        .count();
    let allowed = KNOWN_AI_AGENTS
        .iter()
        .filter(|agent| robot_access_for_agent(body, agent) == RobotAccess::Allowed)
        .count();

    if blocked == 0 && allowed > 0 {
        check_pass(
            "AI bot access",
            &format!("{allowed} known AI bot(s) are explicitly allowed"),
            15,
        )
    } else if blocked == 0 {
        check_warn(
            "AI bot access",
            "known AI bot access is unspecified; generic robots rules may apply",
            8,
            15,
        )
    } else if blocked < KNOWN_AI_AGENTS.len() {
        check_warn(
            "AI bot access",
            &format!("{blocked} known AI bot(s) are explicitly blocked"),
            5,
            15,
        )
    } else {
        check_fail(
            "AI bot access",
            "all known AI bots are explicitly blocked",
            0,
            15,
        )
    }
}

fn robot_access_for_agent(body: &str, agent: &str) -> RobotAccess {
    let mut current_applies = false;
    let mut matched = false;
    let mut disallow_root = false;
    let mut explicit_allow_root = false;

    for raw in body.lines() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        if key == "user-agent" {
            current_applies = value.eq_ignore_ascii_case(agent);
            if current_applies {
                matched = true;
            }
        } else if current_applies && key == "disallow" && value == "/" {
            disallow_root = true;
        } else if current_applies && key == "allow" && value == "/" {
            explicit_allow_root = true;
        }
    }

    if disallow_root {
        RobotAccess::Blocked
    } else if matched || explicit_allow_root {
        RobotAccess::Allowed
    } else {
        RobotAccess::Unspecified
    }
}

fn check_ai_bot_rules(body: &str) -> AgentReadinessCheck {
    let lower = body.to_ascii_lowercase();
    let count = KNOWN_AI_AGENTS
        .iter()
        .filter(|a| lower.contains(&format!("user-agent: {a}")))
        .count();
    if count > 0 {
        check_pass(
            "AI bot rules",
            &format!("robots.txt contains rules for {count} known AI bot(s)"),
            10,
        )
    } else {
        check_warn(
            "AI bot rules",
            "robots.txt does not define known AI bot-specific rules",
            0,
            10,
        )
    }
}
fn robots_group_disallows_root(body: &str, agent: &str) -> bool {
    let mut applies = false;
    for raw in body.lines() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let key = k.trim().to_ascii_lowercase();
        let value = v.trim();
        if key == "user-agent" {
            applies = value.eq_ignore_ascii_case(agent);
        } else if applies && key == "disallow" && value == "/" {
            return true;
        }
    }
    false
}
fn check_open_graph(html: &str) -> AgentReadinessCheck {
    let lower = html.to_ascii_lowercase();
    let required = ["og:title", "og:description", "og:url", "og:type"];
    let found = required
        .iter()
        .filter(|property| {
            lower.contains(&format!("property=\"{property}\""))
                || lower.contains(&format!("property='{property}'"))
        })
        .count();

    if found >= 3 {
        check_pass(
            "OpenGraph",
            &format!("homepage exposes {found}/4 core OpenGraph properties"),
            10,
        )
    } else if found > 0 {
        check_warn(
            "OpenGraph",
            &format!("homepage exposes only {found}/4 core OpenGraph properties"),
            5,
            10,
        )
    } else {
        check_warn("OpenGraph", "homepage OpenGraph metadata missing", 0, 10)
    }
}

fn has_json_ld(html: &str) -> bool {
    html.to_ascii_lowercase().contains("application/ld+json")
}
fn has_tag(html: &str, tag: &str) -> bool {
    html.to_ascii_lowercase().contains(&format!("<{tag}"))
}

pub fn export_agent_readiness_json(
    path: &std::path::Path,
    report: &AgentReadinessReport,
) -> Result<()> {
    let file = std::fs::File::create(path).with_context(|| {
        format!(
            "failed to create agent readiness JSON file: {}",
            path.display()
        )
    })?;
    serde_json::to_writer_pretty(file, report).with_context(|| {
        format!(
            "failed to write agent readiness JSON file: {}",
            path.display()
        )
    })?;
    Ok(())
}

pub fn export_agent_readiness_html(
    path: &std::path::Path,
    report: &AgentReadinessReport,
) -> Result<()> {
    let mut html = String::new();
    html.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>sitepulse agent readiness report</title><style>body{font-family:system-ui,-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;margin:2rem;color:#172033}.score{font-size:2rem;font-weight:700}.pass{background:#ecfdf5}.warn{background:#fffbeb}.fail{background:#fff1f2}table{border-collapse:collapse;width:100%;margin-top:1rem}th,td{border-bottom:1px solid #e5e7eb;padding:.6rem;text-align:left;vertical-align:top}th{background:#f3f4f6}code{word-break:break-all}</style></head><body>");
    html.push_str("<h1>sitepulse agent readiness report</h1>");
    html.push_str(&format!(
        "<p><strong>Site:</strong> <code>{}</code></p>",
        escape_html(&report.site_url)
    ));
    html.push_str(&format!(
        "<p class=\"score\">Score: {}/{}</p>",
        report.score, report.max_score
    ));
    html.push_str("<table><thead><tr><th>Status</th><th>Check</th><th>Message</th><th>Points</th></tr></thead><tbody>");
    for check in &report.checks {
        let class = match check.status {
            AgentCheckStatus::Pass => "pass",
            AgentCheckStatus::Warn => "warn",
            AgentCheckStatus::Fail => "fail",
        };
        html.push_str(&format!(
            "<tr class=\"{}\"><td>{}</td><td>{}</td><td>{}</td><td>{}/{}</td></tr>",
            class,
            check.status.as_str(),
            escape_html(&check.name),
            escape_html(&check.message),
            check.points,
            check.max_points
        ));
    }
    html.push_str("</tbody></table></body></html>\n");
    std::fs::write(path, html).with_context(|| {
        format!(
            "failed to write agent readiness HTML file: {}",
            path.display()
        )
    })?;
    Ok(())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dns_aid_check_passes_with_records() {
        let result = DnsAidResult {
            records: vec!["_agent.example.com: aid=v1".to_string()],
            checked_names: vec!["_agent.example.com".to_string()],
            error: None,
        };
        let check = check_dns_aid(&result);
        assert_eq!(check.status, AgentCheckStatus::Pass);
    }

    #[test]
    fn calculates_score_percent() {
        let report = AgentReadinessReport {
            site_url: "https://example.com/".to_string(),
            score: 7,
            max_score: 10,
            checks: Vec::new(),
        };
        assert_eq!(score_percent(&report), 70);
    }

    #[test]
    fn builds_site_root_url() {
        assert_eq!(
            site_root_url("https://example.com/path/sitemap.xml?x=1")
                .unwrap()
                .as_str(),
            "https://example.com/"
        );
    }
    #[test]
    fn detects_robots_block_all() {
        assert!(robots_group_disallows_root(
            "User-agent: *\nDisallow: /",
            "*"
        ));
        assert!(!robots_group_disallows_root(
            "User-agent: *\nDisallow: /private",
            "*"
        ));
    }
    #[test]
    fn detects_json_ld_and_semantic_tags() {
        let html = r#"<main><h1>Hello</h1><script type="application/ld+json">{}</script></main>"#;
        assert!(has_json_ld(html));
        assert!(has_tag(html, "main"));
        assert!(has_tag(html, "h1"));
    }

    #[test]
    fn exports_agent_readiness_html() {
        let report = AgentReadinessReport {
            site_url: "https://example.com/".to_string(),
            score: 1,
            max_score: 1,
            checks: vec![check_pass("Example", "works", 1)],
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agent.html");
        export_agent_readiness_html(&path, &report).unwrap();
        let html = std::fs::read_to_string(path).unwrap();
        assert!(html.contains("sitepulse agent readiness report"));
    }

    #[test]
    fn checks_open_graph_metadata() {
        let html = r#"<meta property="og:title" content="Title"><meta property="og:description" content="Desc"><meta property="og:url" content="https://example.com/">"#;
        let check = check_open_graph(html);
        assert_eq!(check.status, AgentCheckStatus::Pass);
    }

    #[test]
    fn detects_web_bot_auth_header() {
        let result = HeaderFetchResult {
            status: Some(200),
            links: Vec::new(),
            content_signals: Vec::new(),
            x_robots_tag: Vec::new(),
            web_bot_auth: vec!["required".to_string()],
            www_authenticate: Vec::new(),
            error: None,
        };
        let check = check_web_bot_auth(&result);
        assert_eq!(check.status, AgentCheckStatus::Pass);
    }

    #[test]
    fn detects_content_signal_metadata() {
        assert!(has_content_signal_metadata(
            r#"<meta name="robots" content="index,follow">"#
        ));
        assert!(has_content_signal_metadata(
            r#"<meta name="ai-policy" content="allow">"#
        ));
        assert!(!has_content_signal_metadata("<html></html>"));
    }

    #[test]
    fn detects_ai_bot_access() {
        let robots = "User-agent: GPTBot
Disallow: /

User-agent: ClaudeBot
Allow: /";
        assert_eq!(
            robot_access_for_agent(robots, "GPTBot"),
            RobotAccess::Blocked
        );
        assert_eq!(
            robot_access_for_agent(robots, "ClaudeBot"),
            RobotAccess::Allowed
        );
        assert_eq!(
            robot_access_for_agent(robots, "PerplexityBot"),
            RobotAccess::Unspecified
        );
    }

    #[test]
    fn detects_markdown_like_content() {
        assert!(looks_like_markdown(
            "# Title

- item"
        ));
        assert!(!looks_like_markdown("<!doctype html><html></html>"));
    }
}
