# sitepulse

`sitepulse` is a Rust-based CLI tool that discovers page URLs from a `sitemap.xml` URL and reports each page's HTTP status, response time, redirect state, and final URL.

The project is designed for technical SEO, WordPress, WooCommerce, and e-commerce websites to detect broken links, `404`/`500` errors, redirects, and slow pages.

## Status

The first working version has been implemented.

Current features:

- `sitepulse check <SITEMAP_URL>` command
- Standard sitemap parsing
- Sitemap index support
- Maximum sitemap index depth: `2`
- Extract URLs from `<loc>...</loc>` entries
- Deduplicate repeated URLs
- HTTP status code reporting
- Response time measurement
- Redirect following
- Final URL reporting
- Timeout support
- Concurrency support
- Option to show only errors
- Retry support for network errors and `5xx` responses
- Maximum URL limit option
- CSV export
- JSON export
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
| `--concurrency <N>` | Number of concurrent HTTP checks | `10` |
| `--timeout <SECONDS>` | Request timeout in seconds | `10` |
| `--only-errors` | Show only network errors and `4xx`/`5xx` responses | Disabled |
| `--export <FILE>` | Write results to a CSV file | None |
| `--export-json <FILE>` | Write results to a JSON file | None |
| `--fail-on-errors` | Exit with code `2` if any `4xx`, `5xx`, timeout, or network error is found | Disabled |
| `--retries <N>` | Retry failed requests and `5xx` responses | `0` |
| `--max-urls <N>` | Limit how many discovered URLs are checked | None |

Examples:

```bash
cargo run -- check https://example.com/sitemap.xml --concurrency 20
```

```bash
cargo run -- check https://example.com/sitemap.xml --timeout 15
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

Multiple options can be used together:

```bash
cargo run -- check https://example.com/sitemap.xml \
  --concurrency 20 \
  --timeout 10 \
  --retries 2 \
  --max-urls 1000 \
  --only-errors \
  --export report.csv
```

## Example terminal output

```text
Checking sitemap: https://example.com/sitemap.xml
Concurrency: 20
Timeout: 10s
Retries: 2

Discovered URLs: 1240

STATUS      TIME ATTEMPTS  REDIRECT    ERROR URL
------------------------------------------------------------------------------------------
200        184ms        1        no       no https://example.com/
301         96ms        1       yes       no https://example.com/old -> https://example.com/new
404        121ms        1        no       no https://example.com/missing-page
500        430ms        3        no       no https://example.com/broken

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

CSV and JSON fields:

- `url`
- `status`
- `time_ms`
- `redirected`
- `final_url`
- `error`
- `attempts`

## Project structure

```text
src/
  main.rs      # Application entry point
  cli.rs       # CLI arguments and command definitions
  sitemap.rs   # Sitemap download, parsing, and discovery
  checker.rs   # URL HTTP checks
  report.rs    # Terminal output and summary report
  export.rs    # CSV export
  models.rs    # Shared data models

examples/
  sitemap.xml  # Example sitemap for testing
```

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
- [x] Timeout
- [x] `--only-errors`
- [x] Retry support
- [x] Maximum URL limit option
- [x] CSV export
- [x] JSON export
- [x] CI-friendly `--fail-on-errors` option
- [x] Sitemap index support
- [x] Slow URL list
- [x] README

Potential next improvements:

- [ ] More readable table output
- [ ] HTML report output
- [ ] Robots.txt checks
- [ ] Canonical URL checks
- [ ] Title/meta description checks
- [ ] Explicit tests for gzip sitemap support

## Notes

- HTTP errors do not crash the program; they are reported per URL.
- If the sitemap cannot be downloaded or the XML is invalid, the program returns a clear error.
- Redirects are followed and the final URL is recorded.
- Duplicate URLs are deduplicated.
