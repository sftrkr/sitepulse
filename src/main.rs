mod checker;
mod cli;
mod export;
mod models;
mod report;
mod sitemap;

use anyhow::Result;
use checker::check_urls;
use clap::Parser;
use cli::{Cli, Commands};
use export::{export_csv, export_json};
use report::{print_results, print_summary, summarize};
use sitemap::discover_urls;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check(args) => {
            if args.concurrency == 0 {
                anyhow::bail!("--concurrency must be greater than 0");
            }
            if args.timeout == 0 {
                anyhow::bail!("--timeout must be greater than 0");
            }

            println!("Checking sitemap: {}", args.sitemap_url);
            println!("Concurrency: {}", args.concurrency);
            println!("Timeout: {}s", args.timeout);
            println!("Retries: {}", args.retries);
            println!();

            let mut urls = discover_urls(&args.sitemap_url, args.timeout).await?;
            let discovered_count = urls.len();
            if let Some(max_urls) = args.max_urls {
                urls.truncate(max_urls);
            }
            println!("Discovered URLs: {}", discovered_count);
            if urls.len() != discovered_count {
                println!("Checking URLs: {}", urls.len());
            }
            println!();

            let results = check_urls(&urls, args.concurrency, args.timeout, args.retries).await;
            print_results(&results, args.only_errors);

            if let Some(path) = args.export.as_deref() {
                export_csv(path, &results)?;
                println!("\nCSV report written to: {}", path.display());
            }

            if let Some(path) = args.export_json.as_deref() {
                export_json(path, &results)?;
                println!("\nJSON report written to: {}", path.display());
            }

            let summary = summarize(&results);
            print_summary(&summary);

            if args.fail_on_errors && summary.has_errors() {
                std::process::exit(2);
            }
        }
    }

    Ok(())
}
