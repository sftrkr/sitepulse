use crate::cli::{CheckArgs, HttpMethodArg};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize)]
pub struct CheckConfig {
    pub concurrency: Option<usize>,
    pub delay_ms: Option<u64>,
    pub rate_limit_per_second: Option<u64>,
    pub per_host_concurrency: Option<usize>,
    pub per_host_rate_limit_per_second: Option<u64>,
    pub timeout: Option<u64>,
    pub user_agent: Option<String>,
    pub method: Option<HttpMethodConfig>,
    pub analyze_meta: Option<bool>,
    pub only_errors: Option<bool>,
    pub summary_only: Option<bool>,
    pub export: Option<PathBuf>,
    pub export_json: Option<PathBuf>,
    pub export_html: Option<PathBuf>,
    pub export_junit: Option<PathBuf>,
    pub export_sarif: Option<PathBuf>,
    pub retries: Option<usize>,
    pub sitemap_retries: Option<usize>,
    pub max_urls: Option<usize>,
    pub dry_run: Option<bool>,
    pub same_host_only: Option<bool>,
    pub respect_robots: Option<bool>,
    pub agent_ready: Option<bool>,
    pub agent_ready_export_json: Option<PathBuf>,
    pub agent_ready_export_html: Option<PathBuf>,
    pub agent_ready_fail_under: Option<u8>,
    pub fail_on_errors: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HttpMethodConfig {
    Get,
    Head,
}

pub fn load_check_config(path: &Path) -> Result<CheckConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse config file as JSON: {}", path.display()))
}

pub fn apply_check_config(args: &mut CheckArgs, config: CheckConfig) {
    if let Some(value) = config.concurrency {
        args.concurrency = value;
    }
    if let Some(value) = config.delay_ms {
        args.delay_ms = value;
    }
    if let Some(value) = config.rate_limit_per_second {
        args.rate_limit_per_second = Some(value);
    }
    if let Some(value) = config.per_host_concurrency {
        args.per_host_concurrency = Some(value);
    }
    if let Some(value) = config.per_host_rate_limit_per_second {
        args.per_host_rate_limit_per_second = Some(value);
    }
    if let Some(value) = config.timeout {
        args.timeout = value;
    }
    if let Some(value) = config.user_agent {
        args.user_agent = value;
    }
    if let Some(value) = config.method {
        args.method = match value {
            HttpMethodConfig::Get => HttpMethodArg::Get,
            HttpMethodConfig::Head => HttpMethodArg::Head,
        };
    }
    if let Some(value) = config.analyze_meta {
        args.analyze_meta = value;
    }
    if let Some(value) = config.only_errors {
        args.only_errors = value;
    }
    if let Some(value) = config.summary_only {
        args.summary_only = value;
    }
    if let Some(value) = config.export {
        args.export = Some(value);
    }
    if let Some(value) = config.export_json {
        args.export_json = Some(value);
    }
    if let Some(value) = config.export_html {
        args.export_html = Some(value);
    }
    if let Some(value) = config.export_junit {
        args.export_junit = Some(value);
    }
    if let Some(value) = config.export_sarif {
        args.export_sarif = Some(value);
    }
    if let Some(value) = config.retries {
        args.retries = value;
    }
    if let Some(value) = config.sitemap_retries {
        args.sitemap_retries = value;
    }
    if let Some(value) = config.max_urls {
        args.max_urls = Some(value);
    }
    if let Some(value) = config.dry_run {
        args.dry_run = value;
    }
    if let Some(value) = config.same_host_only {
        args.same_host_only = value;
    }
    if let Some(value) = config.respect_robots {
        args.respect_robots = value;
    }
    if let Some(value) = config.agent_ready {
        args.agent_ready = value;
    }
    if let Some(value) = config.agent_ready_export_json {
        args.agent_ready_export_json = Some(value);
    }
    if let Some(value) = config.agent_ready_export_html {
        args.agent_ready_export_html = Some(value);
    }
    if let Some(value) = config.agent_ready_fail_under {
        args.agent_ready_fail_under = Some(value);
    }
    if let Some(value) = config.fail_on_errors {
        args.fail_on_errors = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn applies_config_values_to_check_args() {
        let mut cli =
            crate::cli::Cli::parse_from(["sitepulse", "check", "https://example.com/sitemap.xml"]);
        let crate::cli::Commands::Check(ref mut args) = cli.command else {
            panic!("expected check command");
        };
        apply_check_config(
            args,
            CheckConfig {
                concurrency: Some(3),
                delay_ms: Some(50),
                rate_limit_per_second: Some(2),
                per_host_concurrency: Some(1),
                per_host_rate_limit_per_second: Some(3),
                method: Some(HttpMethodConfig::Head),
                summary_only: Some(true),
                dry_run: Some(true),
                agent_ready: Some(true),
                ..CheckConfig::default()
            },
        );
        assert_eq!(args.concurrency, 3);
        assert_eq!(args.delay_ms, 50);
        assert_eq!(args.rate_limit_per_second, Some(2));
        assert_eq!(args.per_host_concurrency, Some(1));
        assert_eq!(args.per_host_rate_limit_per_second, Some(3));
        assert!(matches!(args.method, HttpMethodArg::Head));
        assert!(args.summary_only);
        assert!(args.dry_run);
        assert!(args.agent_ready);
    }
}
