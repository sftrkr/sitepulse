# MCP support

`sitepulse` can be exposed to AI assistants through Model Context Protocol (MCP) using the `sitepulse mcp` command.

The MCP server runs over stdio and is intended for local agent apps such as Codex-compatible clients.

## Start the MCP server

```bash
sitepulse mcp
```

## Codex app configuration

```json
{
  "mcpServers": {
    "sitepulse": {
      "command": "sitepulse",
      "args": ["mcp"]
    }
  }
}
```

During development from a checkout:

```json
{
  "mcpServers": {
    "sitepulse": {
      "command": "cargo",
      "args": ["run", "--quiet", "--", "mcp"]
    }
  }
}
```

## Tools

### `check_sitemap`

Runs a sitemap URL check and returns structured JSON containing discovered URL count, checked URL count, summary, per-URL results, and optional agent readiness report.

### `agent_ready`

Runs only the agent readiness audit for a site URL.

### `validate_config`

Validates a `sitepulse` JSON config file.

## Notes

- MCP tool outputs are JSON strings so AI clients can parse them reliably.
- For large sites, use conservative `max_urls`, `concurrency`, `rate_limit_per_second`, and `per_host_concurrency` values.
- The MCP server reuses the same core logic as the CLI.

## JSON-RPC smoke test

List available tools:

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"tools/list"}
' | sitepulse mcp
```

Call `agent_ready`:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "agent_ready",
    "arguments": {
      "site_url": "https://example.com",
      "timeout": 15
    }
  }
}
```

## CLI parity additions

The `check_sitemap` MCP tool supports the main operational CLI controls used by agents:

- `delay_ms`
- `dry_run`
- `fail_on_errors`
- `rate_limit_per_second`
- `per_host_concurrency`
- `per_host_rate_limit_per_second`
- `export`, `export_json`, `export_html`, `export_junit`, `export_sarif`

Unlike the CLI, MCP does not exit with process status codes for `fail_on_errors`; it returns `failed: true` and `failure_reason` in the structured tool result.
