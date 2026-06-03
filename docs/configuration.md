# Configuration

`sitepulse check` supports JSON configuration files for repeatable audits.

## Usage

Validate a config file before using it:

```bash
sitepulse config validate sitepulse.json
```

Run a check with a config file:

```bash
sitepulse check https://example.com/sitemap.xml --config sitepulse.json
```

Command-line parsing happens first, then values from the config file are applied.

## Example

```json
{
  "concurrency": 5,
  "timeout": 15,
  "user_agent": "sitepulse/0.1 (+https://example.com)",
  "method": "head",
  "max_urls": 100,
  "sitemap_retries": 3,
  "retries": 1,
  "same_host_only": true,
  "respect_robots": true,
  "analyze_meta": true,
  "agent_ready": true,
  "agent_ready_fail_under": 70
}
```

## CI example

```json
{
  "concurrency": 3,
  "timeout": 15,
  "max_urls": 500,
  "same_host_only": true,
  "respect_robots": true,
  "fail_on_errors": true,
  "export_junit": "report.xml",
  "export_sarif": "report.sarif"
}
```

## Supported fields

- `concurrency`
- `delay_ms`
- `rate_limit_per_second`
- `per_host_concurrency`
- `per_host_rate_limit_per_second`
- `timeout`
- `user_agent`
- `method`: `get` or `head`
- `max_redirects`
- `analyze_meta`
- `only_errors`
- `summary_only`
- `quiet`
- `no_color`
- `slow_threshold_ms`
- `only_slow`
- `dry_run`
- `export`
- `export_json`
- `export_html`
- `export_junit`
- `export_sarif`
- `retries`
- `sitemap_retries`
- `max_urls`
- `same_host_only`
- `respect_robots`
- `agent_ready`
- `agent_ready_export_json`
- `agent_ready_export_html`
- `agent_ready_fail_under`
- `fail_on_errors`

## Notes

- JSON field names use snake_case.
- Unknown fields are rejected; use the schema to catch typos early.
- Keep site-specific values such as the sitemap URL on the command line.
- Use conservative politeness settings for production sites.

## JSON schema

A JSON Schema is available at [`schemas/sitepulse-config.schema.json`](../schemas/sitepulse-config.schema.json). Editors that support JSON Schema can use it for autocomplete and validation.

Example config: [`examples/sitepulse.json`](../examples/sitepulse.json).
