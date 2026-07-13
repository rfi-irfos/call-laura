# lauras-api

(Renamed 2026-07-13 from `laura-api`.)

## What this is

The Fly-hosted HTTP surface for the lauras family. Same review logic as
[`lauras-mcp`](https://crates.io/crates/lauras-mcp), three routes instead of a stdio loop:

- **`POST /review`** — the free 4-lens core (backed by
  [`lauras-core`](https://crates.io/crates/lauras-core)). Key-gated, rate-limited.
- **`POST /team`** — Laura's 15-agent expert team (backed by
  [`lauras-team`](https://crates.io/crates/lauras-team)). Key-gated, rate-limited.
- **`POST /mcp`** — MCP JSON-RPC over HTTP, keyless, rate-limited only. This is what the
  [Smithery](https://smithery.ai) listing points at, so any MCP-connected agent can use
  `review_plan` with zero setup, no key needed.
- **`GET /health`** — liveness check.

**Live** at [laura-api.fly.dev](https://laura-api.fly.dev). This crate is also published so the
source is inspectable and buildable independently of the Fly deployment, but running your own
instance needs `LAURA_API_KEYS` (comma-separated allowlist) set — the binary refuses to start
with zero valid keys rather than silently serving every request unauthorized.

## Why it matters

Both `/review` and `/team` are fully synchronous, local computation — no network call, no API
key, no external cost per request. The key-gate and 10-req/min-per-IP rate limit exist purely
for abuse/DoS hygiene on a public endpoint, not to protect a metered inference bill. `/mcp` stays
keyless on purpose, matching RFI-IRFOS's `ternlang-api` precedent: any agent that can reach the
URL can call `review_plan` immediately, no signup step between "found this" and "used this."

## Using it

```bash
curl -X POST https://laura-api.fly.dev/review \
  -H "Authorization: Bearer $LAURA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"text": "# Goals\n...\n# Success Criteria\n..."}'

curl -X POST https://laura-api.fly.dev/team \
  -H "Authorization: Bearer $LAURA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"text": "We deploy with no rollback and store personal data without consent."}'
```

Response shapes match `lauras-core`'s `ReviewResponse` and `lauras-team`'s `TeamResponse`
respectively — see each crate's own README for the full type definitions and the mandatory
`source` attribution field on every finding.

## Deploy your own

```bash
fly deploy
```

Requires `LAURA_API_KEYS` set as a Fly secret. `PORT` defaults to `8080`.

## The lauras family

- **[`lauras-core`](https://crates.io/crates/lauras-core)** — the deterministic 4-lens engine
  everything here is built on. Read its "Attribution & Sourcing" section first.
- **[`lauras-mcp`](https://crates.io/crates/lauras-mcp)** — the same two tools over stdio
  instead of HTTP, for local MCP-connected agents.
- **[`lauras-team`](https://crates.io/crates/lauras-team)** — the 15-agent expert team behind
  `/team`.

## License

Business Source License 1.1 (source-available; commercial/production use requires a license
from RFI-IRFOS). See the workspace root `LICENSE-BSL`.
