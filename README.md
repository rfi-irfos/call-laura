# call-laura

[![license](https://img.shields.io/badge/license-LGPL--3.0--or--later%20%2F%20BSL--1.1-blue)](./LICENSE)
[![status](https://img.shields.io/badge/status-live-brightgreen)](#status)
[![crates.io](https://img.shields.io/crates/v/call-laura-core.svg)](https://crates.io/crates/call-laura-core)

**Structured document review grounded in Laura Serna Gaviria's Human–AI
Co-Evolution research.** An MCP (Model Context Protocol) server: any agent submits
a plan or document, gets back structured findings across four lenses — no opaque
score, every finding cites the exact span of your text it's reacting to.

**Fully deterministic and local.** No network call, no API key, no external
dependency. The same document always produces the same review — every finding
traces back to a plain keyword/pattern match you can read directly in
`laura-core/src/lenses/`. This is a deliberate trade: real semantic
understanding for full reproducibility, transparency, and zero cost. See "Why
it's different" below for what that trade actually costs.

Co-designed by Laura Serna Gaviria (Emergent Interaction Lab), Simeon Kepp
(RFI-IRFOS), and Claude. Part of RFI-IRFOS's open-core model.

## Status

**Live**, 2026-07-12. Laura reviewed real sample output before this shipped.
`call-laura-core` and `laura-mcp` are published on crates.io; `laura-api` is
deployed at [laura-api.fly.dev](https://laura-api.fly.dev), both `/mcp`
(what the [Smithery](https://smithery.ai) listing uses) and `/review`. 32 unit
tests, verified end-to-end against both the local stdio server and the live
public URL.

License terms in `LICENSE-LGPL`/`LICENSE-BSL` are still explicitly marked as a
starting proposal pending Laura's own final confirmation — read the NOTE at
the top of each before relying on them for reuse.

An earlier version of this tool called an LLM (NVIDIA-hosted) per lens. That
path is gone — not deferred, removed — after the NVIDIA account hit a
persistent `403 Forbidden` on every inference call despite `/v1/models`
succeeding (an account-side entitlement gap, confirmed across 5 different
models). Rather than wait on that, every lens was rewritten as deterministic
keyword/pattern matching. This turned out to be a genuine improvement, not just
a workaround: zero cost, zero external dependency, and every decision is now
fully inspectable in source rather than living inside an LLM's judgment call.

## Attribution & Sourcing — read this before trusting any output

This is the most important section in this README. Every one of the four lenses
below carries a mandatory `source` field in its output, and it is exactly one of:

| `source` | Meaning |
|---|---|
| `laura-8layer-2025` / `laura-uip-2025` | Directly Laura Serna Gaviria's own published framework — see her OSF preprint, *Human–AI Interaction Emergent Co-Evolution*. |
| `rfi-irfos-operationalization` | A concept she names in her paper, but this project's own operational definition of how to measure/apply it — **not verbatim hers**. |
| `rfi-irfos-addition` | Not from her framework at all. |

Only `eight_layer` and `uip_check` are directly hers — specifically, the
category/rule *names*. The classification method (keyword matching) applying
those names to your text is this project's own operationalization, not
something her paper specifies an algorithm for. `resonance` uses the same
general idea as her paper's CCET metric (compare passages, measure similarity)
applied to a different question (cross-section agreement in a static document,
not turn-to-turn stability in a live conversation) and a cruder mechanism
(shared words, not shared meaning) — it is explicitly **not** CCET.
`ecocentric` is entirely RFI-IRFOS's own addition and has no connection to her
research; it ships in this tool because the team co-designing it chose to
include it, not because it's grounded in her work.

This discipline is inherited directly from the production platform this project
was extracted from (`emergent-interaction-lab`), which holds itself to the same
standard for the same reason: **a review tool bearing a named researcher's
identity only has integrity if you can verify, per finding, whether it's really
her work or this project's own judgment call.**

## Why it's different

- **No opaque score.** Every finding names a `claim`, quotes the exact `evidence`
  span from your input it's reacting to, and carries a `severity`.
  `uip_check`/`ecocentric` findings are only ever constructed from spans actually
  present in your text — there's no separate "generate then verify" step to get
  wrong, the finding *is* the matched sentence.
- **Four independent lenses, not one blended verdict:**

  | Lens | What it checks | How | Source |
  |---|---|---|---|
  | `eight_layer` | Classifies each section against Laura's 8-Layer taxonomy, flags entirely-absent layers | Keyword triggers per layer | `laura-8layer-2025` |
  | `uip_check` | Her UIP's four rules: stated constraints, unsupported absolute claims, unverified "done" claims, un-auditable conclusions | Sentence-level pattern matching | `laura-uip-2025` |
  | `resonance` | Flags low lexical overlap between sections that should plausibly agree (e.g. goals vs. success criteria) | Local term-frequency cosine similarity | `rfi-irfos-operationalization` |
  | `ecocentric` | Missing environmental/downstream/long-term/systemic consideration | Keyword-category presence check | `rfi-irfos-addition` |

- **Honest, explicit limitations.** Every lens's `attribution_note` states plainly
  what keyword matching can't do (miss things phrased without trigger words,
  false-positive on words used in an unrelated sense, no real semantic
  understanding). This isn't hedging — it's the actual shape of the trade this
  version makes, said out loud rather than implied by confident-sounding output.
- **Honest partial failure.** A lens with no classifiable input (e.g. `resonance`
  on a single-section document) reports its own `error` field rather than a
  fabricated result — `call_laura_core::review` runs every lens independently.

## Quick start

```bash
cargo install laura-mcp
claude mcp add laura -s user -- laura-mcp
```

No API key, no environment setup. Then, from any MCP-connected agent:

```jsonc
// tools/call, name: "review_plan"
{ "text": "# Goals\n...\n# Success Criteria\n..." }
// omit "lenses" to run all four; or request a subset, e.g. ["eight_layer","uip_check"]
```

## Hosted API

`laura-api` is deployed to Fly.io. Two surfaces on the same server:

- `POST /mcp` — MCP JSON-RPC over HTTP, keyless, rate-limited only. This is what
  the [Smithery listing](https://smithery.ai) points at, so any MCP-connected
  agent can use `review_plan` with zero setup.
- `POST /review` — plain REST convenience endpoint, `Authorization: Bearer
  <key>` required, same JSON body/response shape. `GET /health` for a liveness
  check. 10 req/min per IP by default on both — abuse/DoS hygiene, not cost
  protection (there's no external API cost per request anymore).

## Workspace layout

```
laura-core/   pure lens logic — package name "call-laura-core" on crates.io,
              open-core, LGPL-3.0-or-later
laura-mcp/    stdio MCP server binary (BSL-1.1)
laura-api/    Fly-hosted HTTP surface, deploy-only, not published (BSL-1.1)
```

## License

`call-laura-core`: LGPL-3.0-or-later. `laura-mcp`/`laura-api`: Business Source License
1.1 with a non-commercial/research use grant. **Draft, pending Laura Serna
Gaviria's confirmation** — see the NOTE at the top of `LICENSE-LGPL` and
`LICENSE-BSL`. Full terms in `LICENSE`.
