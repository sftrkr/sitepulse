use crate::models::{Summary, UrlCheckResult};

pub fn print_results(results: &[UrlCheckResult], only_errors: bool) {
    let visible_results: Vec<&UrlCheckResult> = results
        .iter()
        .filter(|result| !only_errors || result.is_error())
        .collect();

    if visible_results.is_empty() {
        if only_errors {
            println!("No errors found.");
        }
        return;
    }

    println!(
        "{:<8} {:>7} {:>8} {:>7} {:>9} {:>8} {}",
        "STATUS", "TIME", "ATTEMPTS", "METHOD", "REDIRECT", "ERROR", "URL"
    );
    println!("{}", "-".repeat(90));

    for result in visible_results {
        let status = result
            .status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "ERR".to_string());
        let redirect = if result.redirected { "yes" } else { "no" };
        let has_error = if result.error.is_some() { "yes" } else { "no" };
        let url = if result.redirected {
            format!("{} -> {}", result.url, result.final_url)
        } else {
            result.url.clone()
        };

        println!(
            "{:<8} {:>6}ms {:>8} {:>7} {:>9} {:>8} {}",
            status, result.time_ms, result.attempts, result.method, redirect, has_error, url
        );

        if let Some(error) = &result.error {
            println!("{:<8} {}", "", error);
        }
    }
}

pub fn summarize(results: &[UrlCheckResult]) -> Summary {
    let mut summary = Summary {
        total: results.len(),
        ..Summary::default()
    };

    let mut total_time = 0_u128;
    for result in results {
        total_time += result.time_ms;
        match result.status {
            Some(200..=299) => summary.ok_2xx += 1,
            Some(300..=399) => summary.redirect_3xx += 1,
            Some(400..=499) => summary.client_4xx += 1,
            Some(500..=599) => summary.server_5xx += 1,
            _ => summary.errors += 1,
        }
    }

    summary.average_time_ms = if results.is_empty() {
        0
    } else {
        total_time / results.len() as u128
    };
    let mut slowest = results.to_vec();
    slowest.sort_by(|a, b| b.time_ms.cmp(&a.time_ms));
    slowest.truncate(10);
    summary.slowest = slowest;

    summary
}

pub fn print_summary(summary: &Summary) {
    println!("\nSummary:");
    println!("Total: {}", summary.total);
    println!("2xx: {}", summary.ok_2xx);
    println!("3xx: {}", summary.redirect_3xx);
    println!("4xx: {}", summary.client_4xx);
    println!("5xx: {}", summary.server_5xx);
    println!("Errors: {}", summary.errors);
    println!("Average response time: {}ms", summary.average_time_ms);

    if !summary.slowest.is_empty() {
        println!("\nSlowest URLs:");
        for (idx, result) in summary.slowest.iter().enumerate() {
            println!("{}. {}ms {}", idx + 1, result.time_ms, result.url);
        }
    }
}
