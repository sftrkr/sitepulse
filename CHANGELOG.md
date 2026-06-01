# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-06-01

### Added

- Initial `sitepulse check <SITEMAP_URL>` CLI command.
- Sitemap discovery, sitemap index support, gzip sitemap support, URL deduplication, and sitemap download retries.
- Concurrent URL HTTP checks with timeout, retries, GET/HEAD method selection, redirect tracking, final URL reporting, and response timing.
- CSV, JSON, and HTML exports.
- Metadata extraction for title, meta description, and canonical URL.
- Same-host and robots.txt filtering.
- AI agent readiness audit with scoring, PASS/WARN/FAIL checks, JSON/HTML exports, and CI-friendly score thresholds.
- Agent readiness checks for robots.txt, AI crawler access, llms.txt, discovery headers, Markdown negotiation, Content Signals, Web Bot Auth, protocol discovery, JSON-LD, semantic HTML, DNS-AID, and agentic commerce signals.
- Unit tests, CLI integration tests, and GitHub Actions CI.

[0.1.0]: https://github.com/sftrkr/sitepulse/releases/tag/v0.1.0
