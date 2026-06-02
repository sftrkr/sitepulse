# Agent readiness audit

`sitepulse` includes an agent readiness audit for checking whether a website exposes signals that are useful for AI agents, crawlers, and emerging agent-web protocols.

Run it as part of a sitemap check:

~~~bash
sitepulse check https://example.com/sitemap.xml --agent-ready
~~~

Or run it directly against a site root:

~~~bash
sitepulse agent-ready https://example.com
~~~

## Output

The audit prints a score and a PASS/WARN/FAIL checklist:

~~~text
Agent Readiness:
Site: https://example.com/
Score: 180/270 (67%)

PASS robots.txt           robots.txt is accessible (10/10)
WARN llms.txt             llms.txt returned HTTP 404 (0/20)
PASS Homepage title       homepage title found (10/10)
PASS JSON-LD              JSON-LD structured data found: Organization (10/10)
~~~

## Exports

~~~bash
sitepulse agent-ready https://example.com \
  --export-json agent-ready.json \
  --export-html agent-ready.html
~~~

~~~bash
sitepulse check https://example.com/sitemap.xml \
  --agent-ready \
  --agent-ready-export-json agent-ready.json \
  --agent-ready-export-html agent-ready.html
~~~

## CI threshold

~~~bash
sitepulse agent-ready https://example.com --fail-under 80
~~~

~~~bash
sitepulse check https://example.com/sitemap.xml \
  --agent-ready \
  --agent-ready-fail-under 80
~~~

If the score percentage is below the threshold, `sitepulse` exits with code `3`.

## Checks

### Discoverability

- `robots.txt` accessibility
- sitemap directives in `robots.txt`
- homepage `Link` headers
- DNS-AID TXT records

### Content accessibility

- `llms.txt`
- `llms-full.txt`
- Markdown content negotiation

### Bot access control

- AI bot-specific `robots.txt` rules
- known AI bot allow/block status
- Content Signals headers and metadata
- Web Bot Auth signals

Known AI bot names currently include:

- `GPTBot`
- `ChatGPT-User`
- `ClaudeBot`
- `Claude-User`
- `PerplexityBot`
- `Google-Extended`
- `CCBot`

### Protocol discovery

- MCP Server Card
- Agent Skills
- WebMCP
- A2A Agent Card
- API catalog
- OAuth discovery
- OAuth Protected Resource metadata
- `auth.md`

### Page intelligence signals

- homepage title
- meta description
- canonical URL
- OpenGraph metadata
- JSON-LD structured data and schema types
- semantic HTML signals such as `<main>` and `<h1>`

### Commerce readiness

Experimental agentic commerce signals:

- x402
- MPP
- UCP
- ACP

## Notes

Agent-web conventions are still evolving. Some checks are intentionally best-effort and may become stricter or more specific as standards mature.
