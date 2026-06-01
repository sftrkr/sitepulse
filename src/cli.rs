use clap::{Args, Parser, Subcommand, ValueEnum};
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

    /// Load check options from a JSON config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Number of concurrent HTTP checks
    #[arg(long, default_value_t = 10)]
    pub concurrency: usize,

    /// Delay before each URL check request in milliseconds
    #[arg(long, default_value_t = 0)]
    pub delay_ms: u64,

    /// Maximum URL check request starts per second across the run
    #[arg(long)]
    pub rate_limit_per_second: Option<u64>,

    /// Request timeout in seconds
    #[arg(long, default_value_t = 10)]
    pub timeout: u64,

    /// Custom User-Agent for all HTTP requests
    #[arg(long, default_value = "sitepulse/0.1 (+https://example.local)")]
    pub user_agent: String,

    /// HTTP method to use for URL checks
    #[arg(long, value_enum, default_value_t = HttpMethodArg::Get)]
    pub method: HttpMethodArg,

    /// Include page title and meta description in results. Uses GET even when --method=head.
    #[arg(long)]
    pub analyze_meta: bool,

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

    /// Export URL check results as JUnit XML for CI systems
    #[arg(long)]
    pub export_junit: Option<PathBuf>,

    /// Export URL check findings as SARIF for code scanning systems
    #[arg(long)]
    pub export_sarif: Option<PathBuf>,

    /// Retry failed URL checks and 5xx responses this many times
    #[arg(long, default_value_t = 0)]
    pub retries: usize,

    /// Retry sitemap downloads this many times
    #[arg(long, default_value_t = 2)]
    pub sitemap_retries: usize,

    /// Limit the number of discovered URLs to check
    #[arg(long)]
    pub max_urls: Option<usize>,

    /// Only check URLs whose host matches the sitemap URL host
    #[arg(long)]
    pub same_host_only: bool,

    /// Filter out URLs disallowed by robots.txt
    #[arg(long)]
    pub respect_robots: bool,

    /// Run an agent readiness audit for the sitemap host
    #[arg(long)]
    pub agent_ready: bool,

    /// Export agent readiness report as JSON
    #[arg(long)]
    pub agent_ready_export_json: Option<PathBuf>,

    /// Export agent readiness report as HTML
    #[arg(long)]
    pub agent_ready_export_html: Option<PathBuf>,

    /// Exit with code 3 if the agent readiness score percentage is below this value
    #[arg(long)]
    pub agent_ready_fail_under: Option<u8>,

    /// Exit with a non-zero status code if any error is found
    #[arg(long)]
    pub fail_on_errors: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum HttpMethodArg {
    /// Use GET requests for URL checks
    Get,
    /// Use HEAD requests for URL checks; falls back to GET on 405 Method Not Allowed
    Head,
}
