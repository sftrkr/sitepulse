# sitepulse export formats

sitepulse supports multiple export formats for local audits, CI pipelines, and human-readable reporting.

## URL check exports

### CSV

Use CSV when you want to open results in spreadsheets or BI tools.

    sitepulse check https://example.com/sitemap.xml --export report.csv

### JSON

Use JSON when another tool or script will process the full URL result list.

    sitepulse check https://example.com/sitemap.xml --export-json report.json

### HTML

Use HTML for a portable visual report with summary cards, result rows, and slowest URLs.

    sitepulse check https://example.com/sitemap.xml --export-html report.html

### JUnit XML

Use JUnit XML for CI systems that understand test reports. Each URL is exported as a testcase. Network errors, timeouts, 4xx responses, and 5xx responses are exported as failures.

    sitepulse check https://example.com/sitemap.xml --export-junit report.xml

### SARIF

Use SARIF for code scanning or security-style dashboards. Failed URL checks are exported as SARIF results with URL, status, final URL, attempts, method, and timing metadata.

    sitepulse check https://example.com/sitemap.xml --export-sarif report.sarif

## Agent readiness exports

Agent readiness results can be exported separately as JSON and HTML.

    sitepulse agent-ready https://example.com --export-json agent-ready.json --export-html agent-ready.html

The same export flags are also available through the sitemap check command with agent readiness enabled.

    sitepulse check https://example.com/sitemap.xml --agent-ready --agent-ready-export-json agent-ready.json --agent-ready-export-html agent-ready.html

## CI examples

A strict CI run can combine non-zero exit behavior with machine-readable exports.

    sitepulse check https://example.com/sitemap.xml --fail-on-errors --export-junit report.xml --export-sarif report.sarif

For agent readiness thresholds:

    sitepulse agent-ready https://example.com --fail-under 80 --export-json agent-ready.json --export-html agent-ready.html
