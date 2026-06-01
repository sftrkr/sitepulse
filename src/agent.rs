use crate::meta::extract_page_meta;
use anyhow::{Context, Result};
use reqwest::header::{ACCEPT, CONTENT_TYPE, LINK};
use reqwest::Client;
use std::time::Duration;
use url::Url;

const USER_AGENT: &str = "sitepulse/0.1 (+https://example.local)";

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub struct AgentReadinessCheck {
    pub status: AgentCheckStatus,
    pub name: String,
    pub message: String,
    pub points: u8,
    pub max_points: u8,
}

#[derive(Debug, Clone)]
pub struct AgentReadinessReport {
    pub site_url: String,
    pub score: u8,
    pub max_score: u8,
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
    let homepage_links = fetch_link_headers(&client, site_url.as_str()).await;
    let markdown = fetch_markdown_negotiation(&client, site_url.as_str()).await;
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
    }

    checks.push(check_text_file("llms.txt", &llms, 20));
    checks.push(check_text_file("llms-full.txt", &llms_full, 10));
    checks.push(check_link_headers(&homepage_links));
    checks.push(check_markdown_negotiation(&markdown));

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

    let score = checks.iter().map(|c| c.points).sum();
    let max_score = checks.iter().map(|c| c.max_points).sum();
    Ok(AgentReadinessReport {
        site_url: site_url.to_string(),
        score,
        max_score,
        checks,
    })
}

pub fn print_agent_readiness_report(report: &AgentReadinessReport) {
    println!("\nAgent Readiness:");
    println!("Site: {}", report.site_url);
    println!("Score: {}/{}\n", report.score, report.max_score);
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
struct HeaderFetchResult {
    status: Option<u16>,
    links: Vec<String>,
    error: Option<String>,
}

#[derive(Debug)]
struct MarkdownFetchResult {
    status: Option<u16>,
    content_type: Option<String>,
    body: Option<String>,
    error: Option<String>,
}

async fn fetch_link_headers(client: &Client, url: &str) -> HeaderFetchResult {
    match client.head(url).send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let links = response
                .headers()
                .get_all(LINK)
                .iter()
                .filter_map(|value| value.to_str().ok().map(str::to_string))
                .collect();
            HeaderFetchResult {
                status: Some(status),
                links,
                error: None,
            }
        }
        Err(error) => HeaderFetchResult {
            status: None,
            links: Vec::new(),
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
fn check_ai_bot_rules(body: &str) -> AgentReadinessCheck {
    let agents = [
        "gptbot",
        "chatgpt-user",
        "claudebot",
        "claude-user",
        "perplexitybot",
        "google-extended",
        "ccbot",
    ];
    let lower = body.to_ascii_lowercase();
    let count = agents
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

#[cfg(test)]
mod tests {
    use super::*;
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
    fn checks_open_graph_metadata() {
        let html = r#"<meta property="og:title" content="Title"><meta property="og:description" content="Desc"><meta property="og:url" content="https://example.com/">"#;
        let check = check_open_graph(html);
        assert_eq!(check.status, AgentCheckStatus::Pass);
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
