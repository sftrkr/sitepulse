# Usage guide

This guide covers common `sitepulse` workflows for sitemap health checks, technical SEO audits, CI usage, and AI agent readiness checks.

## Basic sitemap check

```bash
sitepulse check https://example.com/sitemap.xml
```

For large sitemaps, start with a small limit:

```bash
sitepulse check https://example.com/sitemap.xml --max-urls 10
```

## Production-safe scan

Use conservative concurrency and politeness controls for production sites:

```bash
sitepulse check https://example.com/sitemap.xml \
  --concurrency 3 \
  --rate-limit-per-second 1 \
  --per-host-concurrency 1 \
  --per-host-rate-limit-per-second 1 \
  --timeout 15
```

## Discover URLs without checking pages

Use `--dry-run` to download and parse sitemaps, apply filters, and stop before page checks:

```bash
sitepulse check https://example.com/sitemap.xml \
  --same-host-only \
  --respect-robots \
  --max-urls 100 \
  --dry-run
```

## Show only problems

```bash
sitepulse check https://example.com/sitemap.xml --only-errors
```

For summary-only terminal output:

```bash
sitepulse check https://example.com/sitemap.xml --summary-only
```

## Find slow URLs

```bash
sitepulse check https://example.com/sitemap.xml \
  --slow-threshold-ms 1500 \
  --only-slow
```

## Metadata checks

```bash
sitepulse check https://example.com/sitemap.xml --analyze-meta
```

This extracts page title, meta description, and canonical URL when available.

## HEAD checks

```bash
sitepulse check https://example.com/sitemap.xml --method head
```

If a server returns `405 Method Not Allowed` for HEAD, `sitepulse` falls back to GET for that URL.

## Export reports

```bash
sitepulse check https://example.com/sitemap.xml \
  --export report.csv \
  --export-json report.json \
  --export-html report.html \
  --export-junit report.xml \
  --export-sarif report.sarif
```

## Agent readiness

Run as part of a sitemap check:

```bash
sitepulse check https://example.com/sitemap.xml --agent-ready
```

Or run the standalone command:

```bash
sitepulse agent-ready https://example.com
```

## Config file

```bash
sitepulse check https://example.com/sitemap.xml --config sitepulse.json
```

Validate the config first:

```bash
sitepulse config validate sitepulse.json
```

See [Configuration](configuration.md) for supported fields.

## MCP server

Expose `sitepulse` tools to MCP-compatible AI clients:

```bash
sitepulse mcp
```

See [MCP support](mcp.md) for Codex-compatible configuration examples.
