use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use url::Url;

const USER_AGENT: &str = "sitepulse/0.1 (+https://example.local)";

#[derive(Debug, Default)]
pub struct RobotsRules {
    disallow: Vec<String>,
}

impl RobotsRules {
    pub fn allows(&self, url: &str) -> bool {
        let Ok(parsed) = Url::parse(url) else {
            return false;
        };
        let path = parsed.path();
        !self
            .disallow
            .iter()
            .any(|rule| !rule.is_empty() && path.starts_with(rule))
    }
}

pub async fn fetch_robots_rules(sitemap_url: &str, timeout_secs: u64) -> Result<RobotsRules> {
    let robots_url = robots_url_for(sitemap_url)?;
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("failed to build HTTP client")?;

    let response = client.get(robots_url.as_str()).send().await;
    let Ok(response) = response else {
        return Ok(RobotsRules::default());
    };

    if !response.status().is_success() {
        return Ok(RobotsRules::default());
    }

    let body = response
        .text()
        .await
        .context("failed to read robots.txt body")?;
    Ok(parse_robots_txt(&body))
}

fn robots_url_for(sitemap_url: &str) -> Result<Url> {
    let mut url = Url::parse(sitemap_url).context("invalid sitemap URL")?;
    url.set_path("/robots.txt");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

fn parse_robots_txt(body: &str) -> RobotsRules {
    let mut applies_to_star = false;
    let mut rules = RobotsRules::default();

    for raw_line in body.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();

        match key.as_str() {
            "user-agent" => {
                applies_to_star = value == "*" || value.eq_ignore_ascii_case("sitepulse")
            }
            "disallow" if applies_to_star && !value.is_empty() => {
                rules.disallow.push(value.to_string())
            }
            _ => {}
        }
    }

    rules
}

pub fn filter_allowed_by_robots(urls: Vec<String>, rules: &RobotsRules) -> Vec<String> {
    urls.into_iter().filter(|url| rules.allows(url)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_disallow_rules_for_star() {
        let rules = parse_robots_txt(
            r#"
User-agent: *
Disallow: /private
Disallow: /tmp/

User-agent: other
Disallow: /public
"#,
        );

        assert!(!rules.allows("https://example.com/private/page"));
        assert!(!rules.allows("https://example.com/tmp/file"));
        assert!(rules.allows("https://example.com/public/page"));
    }

    #[test]
    fn builds_robots_url_from_sitemap_url() {
        let url = robots_url_for("https://example.com/path/sitemap.xml?x=1").unwrap();
        assert_eq!(url.as_str(), "https://example.com/robots.txt");
    }
}
