use crate::models::UrlCheckResult;
use futures::stream::{self, StreamExt};
use reqwest::redirect::Policy;
use reqwest::Client;
use std::time::{Duration, Instant};
use tokio::time::sleep;

const USER_AGENT: &str = "sitepulse/0.1 (+https://example.local)";
const MAX_REDIRECTS: usize = 10;
const RETRY_BACKOFF_MS: u64 = 250;

pub async fn check_urls(
    urls: &[String],
    concurrency: usize,
    timeout_secs: u64,
    retries: usize,
) -> Vec<UrlCheckResult> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(Policy::limited(MAX_REDIRECTS))
        .build()
        .expect("failed to build HTTP client");

    stream::iter(urls.iter().cloned())
        .map(|url| {
            let client = client.clone();
            async move { check_url_with_retries(&client, url, retries).await }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await
}

async fn check_url_with_retries(client: &Client, url: String, retries: usize) -> UrlCheckResult {
    let max_attempts = retries + 1;
    let mut attempt = 1;

    loop {
        let result = check_url_once(client, url.clone(), attempt).await;
        if attempt >= max_attempts || !should_retry(&result) {
            return result;
        }

        let backoff = RETRY_BACKOFF_MS * attempt as u64;
        sleep(Duration::from_millis(backoff)).await;
        attempt += 1;
    }
}

fn should_retry(result: &UrlCheckResult) -> bool {
    result.error.is_some() || result.status.map(|status| status >= 500).unwrap_or(true)
}

async fn check_url_once(client: &Client, url: String, attempts: usize) -> UrlCheckResult {
    let started = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let elapsed = started.elapsed().as_millis();
            let status = response.status().as_u16();
            let final_url = response.url().to_string();
            let redirected = final_url != url;
            UrlCheckResult {
                url,
                status: Some(status),
                time_ms: elapsed,
                redirected,
                final_url,
                error: None,
                attempts,
            }
        }
        Err(err) => UrlCheckResult {
            url: url.clone(),
            status: None,
            time_ms: started.elapsed().as_millis(),
            redirected: false,
            final_url: url,
            error: Some(err.to_string()),
            attempts,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(status: Option<u16>, error: Option<&str>) -> UrlCheckResult {
        UrlCheckResult {
            url: "https://example.com".to_string(),
            status,
            time_ms: 10,
            redirected: false,
            final_url: "https://example.com".to_string(),
            error: error.map(str::to_string),
            attempts: 1,
        }
    }

    #[test]
    fn retries_network_errors_and_5xx() {
        assert!(should_retry(&result(None, Some("timeout"))));
        assert!(should_retry(&result(Some(500), None)));
        assert!(should_retry(&result(Some(503), None)));
    }

    #[test]
    fn does_not_retry_success_or_4xx() {
        assert!(!should_retry(&result(Some(200), None)));
        assert!(!should_retry(&result(Some(301), None)));
        assert!(!should_retry(&result(Some(404), None)));
    }
}
