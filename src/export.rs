use crate::models::{Summary, UrlCheckResult};
use anyhow::{Context, Result};
use std::fmt::Write as _;
use std::path::Path;

pub fn export_csv(path: &Path, results: &[UrlCheckResult]) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)
        .with_context(|| format!("failed to create CSV file: {}", path.display()))?;

    for result in results {
        writer.serialize(result)?;
    }

    writer.flush()?;
    Ok(())
}

pub fn export_json(path: &Path, results: &[UrlCheckResult]) -> Result<()> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("failed to create JSON file: {}", path.display()))?;
    serde_json::to_writer_pretty(file, results)
        .with_context(|| format!("failed to write JSON file: {}", path.display()))?;
    Ok(())
}

pub fn export_html(path: &Path, results: &[UrlCheckResult], summary: &Summary) -> Result<()> {
    let mut html = String::new();
    write_html_header(&mut html);
    write_summary(&mut html, summary);
    write_results_table(&mut html, results);
    write_slowest(&mut html, summary);
    html.push_str("</body></html>\n");

    std::fs::write(path, html)
        .with_context(|| format!("failed to write HTML report: {}", path.display()))?;
    Ok(())
}

fn write_html_header(html: &mut String) {
    html.push_str(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>sitepulse report</title>
  <style>
    body { font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; margin: 2rem; color: #172033; }
    h1, h2 { margin-bottom: .5rem; }
    .cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 1rem; margin: 1rem 0 2rem; }
    .card { border: 1px solid #d8dee9; border-radius: 10px; padding: 1rem; background: #f8fafc; }
    .value { font-size: 1.6rem; font-weight: 700; }
    table { border-collapse: collapse; width: 100%; margin-top: 1rem; }
    th, td { border-bottom: 1px solid #e5e7eb; padding: .55rem; text-align: left; vertical-align: top; }
    th { background: #f3f4f6; position: sticky; top: 0; }
    tr.error { background: #fff1f2; }
    tr.redirect { background: #fffbeb; }
    code { word-break: break-all; }
    .muted { color: #64748b; }
  </style>
</head>
<body>
  <h1>sitepulse report</h1>
"#,
    );
}

fn write_summary(html: &mut String, summary: &Summary) {
    html.push_str("  <h2>Summary</h2>\n  <div class=\"cards\">\n");
    let cards = [
        ("Total", summary.total.to_string()),
        ("2xx", summary.ok_2xx.to_string()),
        ("3xx", summary.redirect_3xx.to_string()),
        ("4xx", summary.client_4xx.to_string()),
        ("5xx", summary.server_5xx.to_string()),
        ("Errors", summary.errors.to_string()),
        ("Average", format!("{}ms", summary.average_time_ms)),
    ];
    for (label, value) in cards {
        let _ = write!(
            html,
            "    <div class=\"card\"><div class=\"muted\">{}</div><div class=\"value\">{}</div></div>\n",
            escape_html(label),
            escape_html(&value)
        );
    }
    html.push_str("  </div>\n");
}

fn write_results_table(html: &mut String, results: &[UrlCheckResult]) {
    html.push_str(
        "  <h2>Results</h2>\n  <table>\n    <thead><tr><th>Status</th><th>Time</th><th>Attempts</th><th>Redirect</th><th>URL</th><th>Final URL</th><th>Error</th></tr></thead>\n    <tbody>\n",
    );

    for result in results {
        let class = if result.is_error() {
            "error"
        } else if result.redirected {
            "redirect"
        } else {
            ""
        };
        let status = result
            .status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "ERR".to_string());
        let _ = write!(
            html,
            "      <tr class=\"{}\"><td>{}</td><td>{}ms</td><td>{}</td><td>{}</td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td></tr>\n",
            class,
            escape_html(&status),
            result.time_ms,
            result.attempts,
            if result.redirected { "yes" } else { "no" },
            escape_html(&result.url),
            escape_html(&result.final_url),
            escape_html(result.error.as_deref().unwrap_or(""))
        );
    }

    html.push_str("    </tbody>\n  </table>\n");
}

fn write_slowest(html: &mut String, summary: &Summary) {
    if summary.slowest.is_empty() {
        return;
    }

    html.push_str("  <h2>Slowest URLs</h2>\n  <ol>\n");
    for result in &summary.slowest {
        let _ = write!(
            html,
            "    <li>{}ms <code>{}</code></li>\n",
            result.time_ms,
            escape_html(&result.url)
        );
    }
    html.push_str("  </ol>\n");
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_html_special_characters() {
        assert_eq!(
            escape_html("<script>alert('x') & more</script>"),
            "&lt;script&gt;alert(&#39;x&#39;) &amp; more&lt;/script&gt;"
        );
    }
}
