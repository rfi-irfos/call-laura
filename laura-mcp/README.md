# lauras-mcp

(Renamed 2026-07-13 from `laura-mcp`.)

## What this is

A stdio [MCP](https://modelcontextprotocol.io) (Model Context Protocol) server exposing two
tools:

- **`review_plan`** — free, deterministic, backed by
  [`lauras-core`](https://crates.io/crates/lauras-core) (source: `laura-core/`). Four independent
  lenses, evidence-anchored, zero setup.
- **`review_team`** — Laura's 15-agent expert team, backed by
  [`lauras-team`](https://crates.io/crates/lauras-team) (source: `laura-team/`). Security, legal,
  finance, privacy, ethics, and more, each a deterministic pass over the same text, reconciled
  into one prioritized synthesis.

Any MCP-connected agent — Claude Code, Claude Desktop, or anything else speaking the protocol —
can call either tool with zero setup beyond installing the binary.

## Why it matters

An agent that reviews its own plan before acting on it catches a specific, recurring failure
mode: absolute claims with nothing backing them ("this will scale," with no stated constraint),
"done" claims with no way to verify them, sections that quietly contradict each other, or a whole
category of consideration (security, legal exposure, accessibility, long-term/systemic effects)
that never gets mentioned at all. `review_plan` and `review_team` exist to surface exactly that,
deterministically, before the plan ships — not as a second opinion from another LLM call (which
would just be one more unverifiable judgment on top of the first), but as fixed, inspectable
sets of pattern checks that produce the same answer every time on the same input.

**Read [`lauras-core`'s "Attribution & Sourcing" section](https://crates.io/crates/lauras-core)
first** for what `review_plan` is actually grounded in — every lens result tells you whether it's
genuinely Laura Serna Gaviria's own published framework or this project's own addition. That
distinction matters for how much weight you should put on any given finding.

## Install

```bash
cargo install lauras-mcp
claude mcp add laura -s user -- lauras-mcp
```

No API key or environment setup needed — both `lauras-core` and `lauras-team` are fully
deterministic and local, so there's nothing to configure and no external service either tool
depends on at review time.

## Using it

Call `review_plan` with a JSON body matching `lauras-core`'s `ReviewRequest`:

```jsonc
// tools/call, name: "review_plan"
{
  "text": "# Goals\n...\n# Success Criteria\n...",
  "lenses": ["eight_layer", "uip_check"] // omit entirely to run all four
}
```

Call `review_team` with a JSON body matching `lauras-team`'s `TeamRequest`:

```jsonc
// tools/call, name: "review_team"
{
  "text": "We deploy with no rollback and store personal data without consent.",
  "agents": null // omit or null = run all 15; or e.g. ["osint", "data_privacy"]
}
```

The `review_plan` response is `lauras-core`'s `ReviewResponse`: a `summary` string plus one
`LensResult` per lens actually run, each carrying its `source` attribution tag, an
`attribution_note` explaining that lens's limits in plain language, and a list of `findings` —
each one a `claim`, a verbatim `evidence` span from your own input, and a `severity`. The
`review_team` response is `lauras-team`'s `TeamResponse`: a risk band, cross-cutting themes
(regions multiple independent agents flagged), and prioritized actions. See each crate's own
README for the full type shapes.

## The lauras family

- **[`lauras-core`](https://crates.io/crates/lauras-core)** — the deterministic 4-lens engine
  this server's free tool is built on. Read it first for the attribution discipline everything
  else inherits.
- **[`lauras-team`](https://crates.io/crates/lauras-team)** — the 15-agent expert team this
  server's `review_team` tool wraps.
- **`lauras-api`** — the same two tools over HTTP instead of stdio, deployed at
  [laura-api.fly.dev](https://laura-api.fly.dev).

## License

Business Source License 1.1 (source-available; commercial/production use requires a license
from RFI-IRFOS). See the workspace root `LICENSE-BSL`.
