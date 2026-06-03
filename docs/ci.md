# CI guide

sitepulse can be used in CI pipelines to fail builds on broken URLs, export machine-readable reports, and enforce an agent readiness score.

## Fail on URL errors

Use `--fail-on-errors` to exit with code `2` when any checked URL has a network error, timeout, `4xx`, or `5xx` response.

```bash
sitepulse check https://example.com/sitemap.xml \
  --fail-on-errors \
  --summary-only
```

## Export JUnit and SARIF

```bash
sitepulse check https://example.com/sitemap.xml \
  --fail-on-errors \
  --export-junit report.xml \
  --export-sarif report.sarif
```

## Agent readiness threshold

```bash
sitepulse agent-ready https://example.com --fail-under 80
```

```bash
sitepulse check https://example.com/sitemap.xml \
  --agent-ready \
  --agent-ready-fail-under 80
```

If the score is below the threshold, sitepulse exits with code `3`.

## Conservative CI settings

```json
{
  "concurrency": 3,
  "timeout": 15,
  "sitemap_retries": 3,
  "retries": 1,
  "same_host_only": true,
  "respect_robots": true,
  "rate_limit_per_second": 1,
  "per_host_concurrency": 1,
  "fail_on_errors": true,
  "export_junit": "report.xml",
  "export_sarif": "report.sarif"
}
```

## GitHub Actions example

```yaml
name: Site health

on:
  schedule:
    - cron: "0 6 * * *"
  workflow_dispatch:

jobs:
  sitepulse:
    runs-on: ubuntu-latest
    steps:
      - name: Install sitepulse
        run: cargo install --git https://github.com/sftrkr/sitepulse.git --tag v0.2.0

      - name: Check sitemap
        run: |
          sitepulse check https://example.com/sitemap.xml \
            --fail-on-errors \
            --summary-only \
            --export-junit report.xml \
            --export-sarif report.sarif

      - name: Upload reports
        uses: actions/upload-artifact@v5
        if: always()
        with:
          name: sitepulse-reports
          path: |
            report.xml
            report.sarif
```

## Exit codes

| Code | Meaning |
| ---: | --- |
| `0` | Successful run |
| `1` | CLI/config/sitemap/runtime error |
| `2` | `--fail-on-errors` found URL errors |
| `3` | Agent readiness score was below the configured threshold |
