use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "sitepulse",
    version,
    about = "Check sitemap URLs for SEO and uptime signals"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Check all URLs discovered from a sitemap.xml URL
    Check(CheckArgs),
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Sitemap XML URL to check
    pub sitemap_url: String,

    /// Number of concurrent HTTP checks
    #[arg(long, default_value_t = 10)]
    pub concurrency: usize,

    /// Request timeout in seconds
    #[arg(long, default_value_t = 10)]
    pub timeout: u64,

    /// Print only HTTP/network errors and 4xx/5xx responses
    #[arg(long)]
    pub only_errors: bool,

    /// Export full result table as CSV
    #[arg(long)]
    pub export: Option<PathBuf>,

    /// Export full result table as JSON
    #[arg(long)]
    pub export_json: Option<PathBuf>,

    /// Export an HTML report
    #[arg(long)]
    pub export_html: Option<PathBuf>,

    /// Retry failed requests and 5xx responses this many times
    #[arg(long, default_value_t = 0)]
    pub retries: usize,

    /// Limit the number of discovered URLs to check
    #[arg(long)]
    pub max_urls: Option<usize>,

    /// Exit with a non-zero status code if any error is found
    #[arg(long)]
    pub fail_on_errors: bool,
}
