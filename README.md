# sitepulse

`sitepulse` is a Rust-based CLI tool for technical SEO, sitemap health checks, and AI agent readiness audits.

It discovers URLs from a `sitemap.xml`, checks each page's HTTP status, response time, redirect state, final URL, and optional metadata, then produces terminal, CSV, JSON, and HTML reports. It also includes an `--agent-ready` audit inspired by emerging agent-web standards such as `llms.txt`, AI crawler rules, discovery headers, protocol discovery, structured data, DNS-AID, and agentic commerce signals.

The project is designed for WordPress, WooCommerce, e-commerce, publisher, and SaaS websites that need to detect broken links, `404`/`500` errors, redirect issues, slow pages, metadata gaps, and whether the site is ready for AI agents and crawlers.

## Status

The first working version has been implemented.

Current features:

- `sitepulse check <SITEMAP_URL>` command
- Standard sitemap parsing
- Sitemap index support
- Gzip sitemap support (`.xml.gz`)
- Maximum sitemap index depth: `2`
- Extract URLs from `<loc>...</loc>` entries
- Deduplicate repeated URLs
- HTTP status code reporting
- Response time measurement
- Redirect following
- Final URL reporting
- Timeout support
- Custom User-Agent support
- Concurrency support
- Per-request delay support for politeness/rate limiting
- Option to show only errors
- Retry support for network errors and `5xx` responses
- GET/HEAD check method selection
- Optional title, meta description, and canonical URL extraction
- Same-host filtering option
- Optional robots.txt filtering
- Initial agent readiness audit (`--agent-ready`)
- CI-friendly agent readiness score threshold
- Maximum URL limit option
- CSV export
- JSON export
- HTML report export
- CI-friendly non-zero exit option
- Summary report
- Top 10 slowest URLs
- Custom User-Agent

```text
sitepulse/0.1 (+https://example.local)
```

## Installation

Requirements:

- Rust stable
- Cargo

Build the project:

```bash
cargo build
```

Build a release binary:

```bash
cargo build --release
```

Generated binary:

```bash
./target/release/sitepulse
```

## Usage

Basic usage:

```bash
cargo run -- check https://example.com/sitemap.xml
```

Using the compiled binary:

```bash
sitepulse check https://example.com/sitemap.xml
```

## CLI options

```bash
sitepulse check <SITEMAP_URL> [OPTIONS]
```

Options:

| Option | Description | Default |
| --- | --- | --- |
| `--config <FILE>` | Load check options from a JSON config file | None |
| `--concurrency <N>` | Number of concurrent HTTP checks | `10` |
| `--delay-ms <MS>` | Delay before each URL check request in milliseconds | `0` |
| `--timeout <SECONDS>` | Request timeout in seconds | `10` |
| `--user-agent <VALUE>` | Custom User-Agent for all HTTP requests | `sitepulse/0.1 (+https://example.local)` |
| `--method <METHOD>` | HTTP method for URL checks: `get` or `head` | `get` |
| `--analyze-meta` | Extract page title, meta description, and canonical URL. Uses GET even with `--method=head` | Disabled |
| `--only-errors` | Show only network errors and `4xx`/`5xx` responses | Disabled |
| `--export <FILE>` | Write results to a CSV file | None |
| `--export-json <FILE>` | Write results to a JSON file | None |
| `--export-html <FILE>` | Write an HTML report | None |
| `--export-junit <FILE>` | Write URL check results as JUnit XML for CI systems | None |
| `--export-sarif <FILE>` | Write URL check findings as SARIF for code scanning systems | None |
| `--fail-on-errors` | Exit with code `2` if any `4xx`, `5xx`, timeout, or network error is found | Disabled |
| `--retries <N>` | Retry failed URL checks and `5xx` responses | `0` |
| `--sitemap-retries <N>` | Retry sitemap downloads before failing | `2` |
| `--max-urls <N>` | Limit how many discovered URLs are checked | None |
| `--same-host-only` | Only check URLs whose host matches the sitemap URL host | Disabled |
| `--respect-robots` | Filter out URLs disallowed by robots.txt | Disabled |
| `--agent-ready` | Run an agent readiness audit for the sitemap host | Disabled |
| `--agent-ready-export-json <FILE>` | Write agent readiness results to a JSON file | None |
| `--agent-ready-export-html <FILE>` | Write agent readiness results to an HTML file | None |
| `--agent-ready-fail-under <PERCENT>` | Exit with code `3` if agent readiness score is below the threshold | None |

Examples:

```bash
cargo run -- check https://example.com/sitemap.xml --concurrency 20
```

```bash
cargo run -- check https://example.com/sitemap.xml --timeout 15
```

```bash
cargo run -- check https://example.com/sitemap.xml --method head
```

```bash
cargo run -- check https://example.com/sitemap.xml --analyze-meta
```

```bash
cargo run -- check https://example.com/sitemap.xml --only-errors
```

```bash
cargo run -- check https://example.com/sitemap.xml --export report.csv
```

```bash
cargo run -- check https://example.com/sitemap.xml --retries 2
```

```bash
cargo run -- check https://example.com/sitemap.xml --max-urls 100
```

```bash
cargo run -- check https://example.com/sitemap.xml --same-host-only
```

```bash
cargo run -- check https://example.com/sitemap.xml --respect-robots
```

```bash
cargo run -- check https://example.com/sitemap.xml --agent-ready
```

```bash
cargo run -- check https://example.com/sitemap.xml --sitemap-retries 3
```

```bash
cargo run -- check https://example.com/sitemap.xml \
  --agent-ready \
  --agent-ready-export-json agent-ready.json \
  --agent-ready-export-html agent-ready.html \
  --agent-ready-fail-under 80
```

Multiple options can be used together:

```bash
cargo run -- check https://example.com/sitemap.xml \
  --concurrency 20 \
  --timeout 10 \
  --method head \
  --analyze-meta \
  --retries 2 \
  --sitemap-retries 3 \
  --max-urls 1000 \
  --same-host-only \
  --respect-robots \
  --only-errors \
  --export report.csv \
  --export-json report.json \
  --export-html report.html \
  --agent-ready \
  --agent-ready-export-json agent-ready.json \
  --agent-ready-export-html agent-ready.html
```
## Example terminal output

```text
Checking sitemap: https://example.com/sitemap.xml
Concurrency: 20
Timeout: 10s
User-Agent: sitepulse/0.1 (+https://example.local)
Method: HEAD
Analyze meta: yes
Retries: 2
Sitemap retries: 2

Discovered URLs: 1240

STATUS      TIME ATTEMPTS  METHOD  REDIRECT    ERROR URL
------------------------------------------------------------------------------------------
200        184ms        1     HEAD        no       no https://example.com/
301         96ms        1     HEAD       yes       no https://example.com/old -> https://example.com/new
404        121ms        1     HEAD        no       no https://example.com/missing-page
500        430ms        3     HEAD        no       no https://example.com/broken

Summary:
Total: 1240
2xx: 1190
3xx: 22
4xx: 20
5xx: 4
Errors: 4
Average response time: 218ms

Slowest URLs:
1. 3820ms https://example.com/category/electronics
2. 2910ms https://example.com/product/example
```

## Export

Export to CSV:

```bash
cargo run -- check https://example.com/sitemap.xml --export report.csv
```

Export to JSON:

```bash
cargo run -- check https://example.com/sitemap.xml --export-json report.json
```

Export to HTML:

```bash
cargo run -- check https://example.com/sitemap.xml --export-html report.html
```

CSV, JSON, and HTML result fields include:

- `url`
- `status`
- `time_ms`
- `redirected`
- `final_url`
- `error`
- `attempts`
- `method`
- `title`
- `meta_description`
- `canonical_url`

## Project structure

```text
src/
  main.rs      # Application entry point
  cli.rs       # CLI arguments and command definitions
  sitemap.rs   # Sitemap download, parsing, and discovery
  checker.rs   # URL HTTP checks
  report.rs    # Terminal output and summary report
  export.rs    # CSV, JSON, and HTML export
  models.rs    # Shared data models

examples/
  sitemap.xml  # Example sitemap for testing
```

## Configuration file

``--config`` accepts a JSON file with check options. Example:

```json
{
  "concurrency": 5,
  "timeout": 15,
  "method": "head",
  "analyze_meta": true,
  "same_host_only": true,
  "respect_robots": true,
  "agent_ready": true,
  "agent_ready_fail_under": 70
}
```

Command-line options are parsed first, then config values are applied. For repeated audits, keep shared defaults in a config file and pass target-specific values such as the sitemap URL on the command line.

## Development

Format code:

```bash
cargo fmt
```

Run compile checks:

```bash
cargo check
```

Run tests:

```bash
cargo test
```

## Roadmap

Completed:

- [x] Project skeleton
- [x] `Cargo.toml`
- [x] CLI command
- [x] Sitemap download
- [x] URL parsing
- [x] HTTP checks
- [x] Concurrency
- [x] Per-request delay support for politeness/rate limiting
- [x] Timeout
- [x] Custom User-Agent support
- [x] `--only-errors`
- [x] Retry support
- [x] Sitemap download retry support
- [x] GET/HEAD check method selection
- [x] Optional title, meta description, and canonical URL extraction
- [x] Same-host filtering option
- [x] Optional robots.txt filtering
- [x] Initial agent readiness audit (`--agent-ready`)
- [x] CI-friendly agent readiness score threshold
- [x] Maximum URL limit option
- [x] CSV export
- [x] JSON export
- [x] HTML report export
- [x] CI-friendly `--fail-on-errors` option
- [x] Sitemap index support
- [x] Gzip sitemap support
- [x] Slow URL list
- [x] README
- [x] Integration tests with a local HTTP server

- [x] Expanded agent readiness audit (`--agent-ready`)
  - [x] Discoverability checks: `robots.txt`, sitemap directives, `Link` headers, DNS-AID
  - [x] Content accessibility checks: `llms.txt`, `llms-full.txt`, Markdown negotiation
  - [x] Bot access control checks: AI bot rules, allow/block detection, Content Signals, Web Bot Auth
  - [x] Protocol discovery checks: MCP, Agent Skills, WebMCP, A2A, API catalog, OAuth, `auth.md`
  - [x] Page intelligence checks: title, meta description, canonical URL, OpenGraph, JSON-LD, semantic HTML
  - [x] Commerce readiness checks: x402, MPP, UCP, ACP
  - [x] Scoring/reporting: score, PASS/WARN/FAIL checklist, JSON/HTML exports

- [x] Add GitHub release workflow for tagged binary releases
- [x] Add configuration file support for repeated audits
- [x] Add basic per-request politeness delay
- [x] Add JUnit and SARIF CI exports

- [x] Richer structured data validation for JSON-LD schema types
- [x] Per-host concurrency controls
- [x] Add Homebrew tap formula draft
Potential next improvements:

- [x] Add `v0.1.0` release notes draft
- [ ] Publish GitHub release notes and binaries for `v0.1.0`
- [ ] Publish prebuilt release binaries
- [x] Add advanced per-host rate window controls

## Notes

- HTTP errors do not crash the program; they are reported per URL.
- If the sitemap cannot be downloaded or the XML is invalid, the program returns a clear error.
- Redirects are followed and the final URL is recorded.
- Duplicate URLs are deduplicated.


## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.


## Security

Please see [SECURITY.md](SECURITY.md) for vulnerability reporting guidelines.


## Changelog

Please see [CHANGELOG.md](CHANGELOG.md) for release history.
