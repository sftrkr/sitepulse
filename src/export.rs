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

pub fn export_junit(path: &Path, results: &[UrlCheckResult]) -> Result<()> {
    let failures = results.iter().filter(|result| result.is_error()).count();
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        xml,
        "<testsuite name=\"sitepulse\" tests=\"{}\" failures=\"{}\" errors=\"0\">",
        results.len(),
        failures
    );

    for result in results {
        let status = result
            .status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "ERR".to_string());
        let _ = writeln!(
            xml,
            "  <testcase classname=\"sitepulse.url\" name=\"{}\" time=\"{:.3}\">",
            escape_xml(&result.url),
            result.time_ms as f64 / 1000.0
        );
        if result.is_error() {
            let message = result
                .error
                .clone()
                .unwrap_or_else(|| format!("HTTP status {status}"));
            let _ = writeln!(
                xml,
                "    <failure message=\"{}\">status={} final_url={} attempts={}</failure>",
                escape_xml(&message),
                escape_xml(&status),
                escape_xml(&result.final_url),
                result.attempts
            );
        }
        xml.push_str("  </testcase>\n");
    }
    xml.push_str("</testsuite>\n");
    std::fs::write(path, xml)
        .with_context(|| format!("failed to write JUnit XML file: {}", path.display()))?;
    Ok(())
}

pub fn export_sarif(path: &Path, results: &[UrlCheckResult]) -> Result<()> {
    let mut sarif_results = Vec::new();
    for result in results.iter().filter(|result| result.is_error()) {
        let level = match result.status {
            Some(500..=599) | None => "error",
            Some(400..=499) => "warning",
            _ => "note",
        };
        let status = result
            .status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "network-error".to_string());
        let message = result
            .error
            .clone()
            .unwrap_or_else(|| format!("URL returned HTTP status {status}"));
        sarif_results.push(serde_json::json!({
            "ruleId": "sitepulse.url-check",
            "level": level,
            "message": { "text": message },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": result.url }
                }
            }],
            "properties": {
                "status": result.status,
                "time_ms": result.time_ms,
                "redirected": result.redirected,
                "final_url": result.final_url,
                "attempts": result.attempts,
                "method": result.method
            }
        }));
    }
    let sarif = serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "sitepulse",
                    "informationUri": "https://github.com/sftrkr/sitepulse",
                    "rules": [{
                        "id": "sitepulse.url-check",
                        "name": "URL health check",
                        "shortDescription": { "text": "A sitemap URL returned an HTTP or network error" },
                        "helpUri": "https://github.com/sftrkr/sitepulse"
                    }]
                }
            },
            "results": sarif_results
        }]
    });
    let file = std::fs::File::create(path)
        .with_context(|| format!("failed to create SARIF file: {}", path.display()))?;
    serde_json::to_writer_pretty(file, &sarif)
        .with_context(|| format!("failed to write SARIF file: {}", path.display()))?;
    Ok(())
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
        let _ = writeln!(
            html,
            "    <div class=\"card\"><div class=\"muted\">{}</div><div class=\"value\">{}</div></div>",
            escape_html(label),
            escape_html(&value)
        );
    }
    html.push_str("  </div>\n");
}

fn write_results_table(html: &mut String, results: &[UrlCheckResult]) {
    html.push_str(
        "  <h2>Results</h2>\n  <table>\n    <thead><tr><th>Status</th><th>Time</th><th>Attempts</th><th>Method</th><th>Redirect</th><th>URL</th><th>Final URL</th><th>Title</th><th>Meta Description</th><th>Canonical URL</th><th>Error</th></tr></thead>\n    <tbody>\n",
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
        let _ = writeln!(
            html,
            "      <tr class=\"{}\"><td>{}</td><td>{}ms</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            class,
            escape_html(&status),
            result.time_ms,
            result.attempts,
            escape_html(&result.method),
            if result.redirected { "yes" } else { "no" },
            escape_html(&result.url),
            escape_html(&result.final_url),
            escape_html(result.title.as_deref().unwrap_or("")),
            escape_html(result.meta_description.as_deref().unwrap_or("")),
            escape_html(result.canonical_url.as_deref().unwrap_or("")),
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
        let _ = writeln!(
            html,
            "    <li>{}ms <code>{}</code></li>",
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
    fn exports_junit_xml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.xml");
        let results = vec![sample_error_result()];
        export_junit(&path, &results).unwrap();
        let xml = std::fs::read_to_string(path).unwrap();
        assert!(xml.contains("<testsuite"));
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("&amp;"));
    }

    #[test]
    fn exports_sarif() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.sarif");
        let results = vec![sample_error_result()];
        export_sarif(&path, &results).unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(value["version"], "2.1.0");
        assert_eq!(value["runs"][0]["results"].as_array().unwrap().len(), 1);
    }

    fn sample_error_result() -> UrlCheckResult {
        UrlCheckResult {
            url: "https://example.com/missing?a=1&b=2".to_string(),
            status: Some(404),
            time_ms: 100,
            redirected: false,
            final_url: "https://example.com/missing".to_string(),
            error: None,
            attempts: 1,
            method: "GET".to_string(),
            title: None,
            meta_description: None,
            canonical_url: None,
        }
    }

    #[test]
    fn escapes_html_special_characters() {
        assert_eq!(
            escape_html("<script>alert('x') & more</script>"),
            "&lt;script&gt;alert(&#39;x&#39;) &amp; more&lt;/script&gt;"
        );
    }
}
