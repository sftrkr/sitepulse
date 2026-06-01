use crate::models::UrlCheckResult;
use futures::stream::{self, StreamExt};
use reqwest::redirect::Policy;
use reqwest::Client;
use std::time::{Duration, Instant};

const USER_AGENT: &str = "sitepulse/0.1 (+https://example.local)";

pub async fn check_urls(
    urls: &[String],
    concurrency: usize,
    timeout_secs: u64,
) -> Vec<UrlCheckResult> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(Policy::limited(10))
        .build()
        .expect("failed to build HTTP client");

    stream::iter(urls.iter().cloned())
        .map(|url| {
            let client = client.clone();
            async move { check_url(&client, url).await }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await
}

async fn check_url(client: &Client, url: String) -> UrlCheckResult {
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
            }
        }
        Err(err) => UrlCheckResult {
            url: url.clone(),
            status: None,
            time_ms: started.elapsed().as_millis(),
            redirected: false,
            final_url: url,
            error: Some(err.to_string()),
        },
    }
}
