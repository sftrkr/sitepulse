use crate::meta::extract_page_meta;
use crate::models::{RequestMethod, UrlCheckResult};
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
    method: RequestMethod,
    analyze_meta: bool,
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
            let method = method.clone();
            async move { check_url_with_retries(&client, url, retries, method, analyze_meta).await }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await
}

async fn check_url_with_retries(
    client: &Client,
    url: String,
    retries: usize,
    method: RequestMethod,
    analyze_meta: bool,
) -> UrlCheckResult {
    let max_attempts = retries + 1;
    let mut attempt = 1;

    loop {
        let result = check_url_once(client, url.clone(), attempt, &method, analyze_meta).await;
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

async fn check_url_once(
    client: &Client,
    url: String,
    attempts: usize,
    method: &RequestMethod,
    analyze_meta: bool,
) -> UrlCheckResult {
    let started = Instant::now();
    let effective_method = if analyze_meta {
        &RequestMethod::Get
    } else {
        method
    };
    let request = match effective_method {
        RequestMethod::Get => client.get(&url),
        RequestMethod::Head => client.head(&url),
    };

    match request.send().await {
        Ok(response) => {
            let method_used = if matches!(effective_method, RequestMethod::Head)
                && response.status().as_u16() == 405
            {
                RequestMethod::Get
            } else {
                method.clone()
            };
            let response = if matches!(effective_method, RequestMethod::Head)
                && response.status().as_u16() == 405
            {
                match client.get(&url).send().await {
                    Ok(response) => response,
                    Err(err) => {
                        return UrlCheckResult {
                            url: url.clone(),
                            status: None,
                            time_ms: started.elapsed().as_millis(),
                            redirected: false,
                            final_url: url,
                            error: Some(err.to_string()),
                            attempts,
                            method: RequestMethod::Get.to_string(),
                            title: None,
                            meta_description: None,
                            canonical_url: None,
                        };
                    }
                }
            } else {
                response
            };

            let status = response.status().as_u16();
            let final_url = response.url().to_string();
            let redirected = final_url != url;
            let page_meta = if analyze_meta {
                response
                    .text()
                    .await
                    .ok()
                    .map(|body| extract_page_meta(&body))
                    .unwrap_or_default()
            } else {
                Default::default()
            };
            let elapsed = started.elapsed().as_millis();
            UrlCheckResult {
                url,
                status: Some(status),
                time_ms: elapsed,
                redirected,
                final_url,
                error: None,
                attempts,
                method: method_used.to_string(),
                title: page_meta.title,
                meta_description: page_meta.description,
                canonical_url: page_meta.canonical_url,
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
            method: effective_method.to_string(),
            title: None,
            meta_description: None,
            canonical_url: None,
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
            method: "GET".to_string(),
            title: None,
            meta_description: None,
            canonical_url: None,
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
