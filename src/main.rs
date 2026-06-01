mod agent;
mod checker;
mod cli;
mod config;
mod export;
mod meta;
mod models;
mod report;
mod robots;
mod sitemap;

use agent::{
    audit_agent_readiness, export_agent_readiness_html, export_agent_readiness_json,
    print_agent_readiness_report, score_percent,
};
use anyhow::Result;
use checker::check_urls;
use clap::Parser;
use cli::{Cli, Commands, HttpMethodArg};
use config::{apply_check_config, load_check_config};
use export::{export_csv, export_html, export_json};
use models::RequestMethod;
use report::{print_results, print_summary, summarize};
use robots::{fetch_robots_rules, filter_allowed_by_robots};
use sitemap::discover_urls_with_retries;
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check(mut args) => {
            if let Some(path) = args.config.as_deref() {
                let config = load_check_config(path)?;
                apply_check_config(&mut args, config);
            }

            if args.concurrency == 0 {
                anyhow::bail!("--concurrency must be greater than 0");
            }
            if args.timeout == 0 {
                anyhow::bail!("--timeout must be greater than 0");
            }
            if let Some(threshold) = args.agent_ready_fail_under {
                if threshold > 100 {
                    anyhow::bail!("--agent-ready-fail-under must be between 0 and 100");
                }
            }

            println!("Checking sitemap: {}", args.sitemap_url);
            println!("Concurrency: {}", args.concurrency);
            let method = match args.method {
                HttpMethodArg::Get => RequestMethod::Get,
                HttpMethodArg::Head => RequestMethod::Head,
            };

            println!("Timeout: {}s", args.timeout);
            println!("User-Agent: {}", args.user_agent);
            println!("Method: {}", method);
            println!(
                "Analyze meta: {}",
                if args.analyze_meta { "yes" } else { "no" }
            );
            println!("Retries: {}", args.retries);
            println!("Sitemap retries: {}", args.sitemap_retries);
            println!();

            let mut urls = discover_urls_with_retries(
                &args.sitemap_url,
                args.timeout,
                args.sitemap_retries,
                &args.user_agent,
            )
            .await?;
            let discovered_count = urls.len();

            if args.same_host_only {
                urls = filter_same_host(urls, &args.sitemap_url)?;
            }

            let same_host_count = urls.len();
            if args.respect_robots {
                let rules =
                    fetch_robots_rules(&args.sitemap_url, args.timeout, &args.user_agent).await?;
                urls = filter_allowed_by_robots(urls, &rules);
            }

            let filtered_count = urls.len();
            if let Some(max_urls) = args.max_urls {
                urls.truncate(max_urls);
            }
            println!("Discovered URLs: {}", discovered_count);
            if args.same_host_only {
                println!("After same-host filter: {}", same_host_count);
            }
            if args.respect_robots {
                println!("After robots.txt filter: {}", filtered_count);
            }
            if urls.len() != filtered_count {
                println!("Checking URLs: {}", urls.len());
            }
            println!();

            let results = check_urls(
                &urls,
                args.concurrency,
                args.timeout,
                args.retries,
                method,
                args.analyze_meta,
                &args.user_agent,
            )
            .await;
            print_results(&results, args.only_errors);

            if let Some(path) = args.export.as_deref() {
                export_csv(path, &results)?;
                println!("\nCSV report written to: {}", path.display());
            }

            let summary = summarize(&results);

            if let Some(path) = args.export_json.as_deref() {
                export_json(path, &results)?;
                println!("\nJSON report written to: {}", path.display());
            }

            if let Some(path) = args.export_html.as_deref() {
                export_html(path, &results, &summary)?;
                println!("\nHTML report written to: {}", path.display());
            }

            print_summary(&summary);

            if args.agent_ready
                || args.agent_ready_export_json.is_some()
                || args.agent_ready_export_html.is_some()
            {
                let agent_report =
                    audit_agent_readiness(&args.sitemap_url, args.timeout, &args.user_agent)
                        .await?;
                print_agent_readiness_report(&agent_report);

                if let Some(path) = args.agent_ready_export_json.as_deref() {
                    export_agent_readiness_json(path, &agent_report)?;
                    println!(
                        "\nAgent readiness JSON report written to: {}",
                        path.display()
                    );
                }

                if let Some(path) = args.agent_ready_export_html.as_deref() {
                    export_agent_readiness_html(path, &agent_report)?;
                    println!(
                        "\nAgent readiness HTML report written to: {}",
                        path.display()
                    );
                }

                if let Some(threshold) = args.agent_ready_fail_under {
                    if score_percent(&agent_report) < threshold {
                        std::process::exit(3);
                    }
                }
            }

            if args.fail_on_errors && summary.has_errors() {
                std::process::exit(2);
            }
        }
    }

    Ok(())
}

fn filter_same_host(urls: Vec<String>, sitemap_url: &str) -> Result<Vec<String>> {
    let sitemap_host = Url::parse(sitemap_url)
        .map_err(|err| anyhow::anyhow!("invalid sitemap URL: {err}"))?
        .host_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("sitemap URL does not include a host"))?;

    Ok(urls
        .into_iter()
        .filter(|url| {
            Url::parse(url)
                .ok()
                .and_then(|parsed| parsed.host_str().map(str::to_string))
                .map(|host| host.eq_ignore_ascii_case(&sitemap_host))
                .unwrap_or(false)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_host_filter_keeps_only_matching_hosts() {
        let urls = vec![
            "https://example.com/a".to_string(),
            "https://cdn.example.com/a".to_string(),
            "https://example.org/a".to_string(),
            "not-a-url".to_string(),
            "https://EXAMPLE.com/b".to_string(),
        ];

        let filtered = filter_same_host(urls, "https://example.com/sitemap.xml").unwrap();

        assert_eq!(
            filtered,
            vec![
                "https://example.com/a".to_string(),
                "https://EXAMPLE.com/b".to_string()
            ]
        );
    }
}
