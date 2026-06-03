# WordPress and WooCommerce guide

This guide shows safe ways to use sitepulse with WordPress and WooCommerce websites.

## Common sitemap URLs

WordPress core sitemap:

```text
https://example.com/wp-sitemap.xml
```

Popular SEO plugin sitemap index URLs:

```text
https://example.com/sitemap_index.xml
https://example.com/sitemap.xml
```

WooCommerce product and category URLs are usually included in one of those sitemap indexes.

## Start with discovery only

Large WordPress and WooCommerce stores can contain many URLs. Start with dry-run mode:

```bash
sitepulse check https://example.com/wp-sitemap.xml --dry-run
```

Add filters before checking URLs:

```bash
sitepulse check https://example.com/wp-sitemap.xml \
  --same-host-only \
  --respect-robots \
  --max-urls 100 \
  --dry-run
```

## Safe production scan

Use conservative concurrency and rate limits:

```bash
sitepulse check https://example.com/wp-sitemap.xml \
  --concurrency 3 \
  --rate-limit-per-second 1 \
  --per-host-concurrency 1 \
  --timeout 15 \
  --sitemap-retries 3
```

## WooCommerce checks

For WooCommerce stores, useful checks include:

- broken product URLs
- category redirect issues
- slow product pages
- missing titles or meta descriptions
- missing canonical URLs
- product JSON-LD presence through agent readiness checks

Example:

```bash
sitepulse check https://example.com/sitemap_index.xml \
  --same-host-only \
  --respect-robots \
  --analyze-meta \
  --slow-threshold-ms 1500 \
  --export-html report.html
```

## CI example

```bash
sitepulse check https://example.com/wp-sitemap.xml \
  --config sitepulse-ci.json \
  --summary-only \
  --fail-on-errors
```

## Agent readiness for WordPress

Run:

```bash
sitepulse agent-ready https://example.com
```

Recommended improvements for WordPress sites:

- publish `llms.txt` with important site and documentation links
- make sure `robots.txt` exposes sitemap directives
- avoid accidentally blocking useful AI crawlers unless intentional
- ensure homepage title, meta description, canonical URL, OpenGraph metadata, and JSON-LD are present
- add Organization, WebSite, BreadcrumbList, Product, Article, or FAQPage schema where appropriate

## Notes

- Always test on staging or with `--max-urls` before scanning large stores.
- Use `--respect-robots` if you want sitepulse to filter URLs according to robots.txt.
- Use `--same-host-only` if sitemap entries contain CDN or external URLs.
