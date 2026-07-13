# lauras-core

(Package name: `lauras-core` — the crates.io name; source lives in the `laura-core/` directory, unchanged, to avoid a needless folder rename. Renamed 2026-07-13 from `call-laura-core`.)

## What this is

Pure review logic behind the lauras family: you hand it a plan or document, it hands back
structured findings across four independent lenses. No opaque score, no "looks good to me" —
every finding names a `claim`, quotes the exact `evidence` span of your text it's reacting to,
and carries a `severity`. If a finding can't point at the words that triggered it, this crate
doesn't emit it.

It's grounded in Laura Serna Gaviria's Human–AI Co-Evolution research (her 8-Layer taxonomy,
her UIP rules — see her OSF preprint), plus two lenses this project added on top. Which lens is
genuinely hers and which is this project's own addition is not left implicit: every `LensResult`
carries a mandatory `source` field, one of `laura-8layer-2025` / `laura-uip-2025` (her published
framework, category names verbatim), `rfi-irfos-operationalization` (a concept she names but
doesn't herself specify an algorithm for — this project's own implementation of it), or
`rfi-irfos-addition` (not from her work at all). **Read the "Attribution & Sourcing" section
below before trusting any output** — that distinction is load-bearing for how you should read
anything this crate produces.

## Why it matters

Most "AI reviews your plan" tooling either (a) calls an LLM and trusts whatever prose comes back,
which means a different run can produce a different verdict on the identical input, or (b)
computes an opaque numeric score with no way to tell what actually drove it. This crate does
neither. It's fully deterministic — the same document always produces the exact same review,
because every finding traces back to a plain keyword/pattern match you can read directly in
`laura-core/src/lenses/`. That's a real trade: this crate doesn't understand meaning, only
matches patterns. But it means the review is fully reproducible, fully inspectable, and fully
local: no network call, no API key, no async runtime, ever.

## The four lenses

| Lens | What it checks | Source |
|---|---|---|
| `eight_layer` | Classifies each section against Laura's 8-Layer taxonomy, flags entirely-absent layers | `laura-8layer-2025` |
| `uip_check` | Her UIP's four rules: stated constraints, unsupported absolute claims, unverified "done" claims, un-auditable conclusions | `laura-uip-2025` |
| `resonance` | Flags low lexical overlap between sections that should plausibly agree (e.g. goals vs. success criteria) | `rfi-irfos-operationalization` |
| `ecocentric` | Missing environmental/downstream/long-term/systemic consideration | `rfi-irfos-addition` |

A lens that has nothing to classify (e.g. `resonance` on a single-section document) reports its
own `error` field rather than fabricating a result — `review()` runs every lens independently, so
one lens's failure never silently drops or contaminates another's.

## Attribution & Sourcing — read this before trusting any output

| `source` | Meaning |
|---|---|
| `laura-8layer-2025` / `laura-uip-2025` | Directly Laura Serna Gaviria's own published framework — category/rule *names* verbatim from her OSF preprint, *Human–AI Interaction Emergent Co-Evolution*. |
| `rfi-irfos-operationalization` | A concept she names in her paper, but this project's own operational definition of how to measure/apply it — **not verbatim hers**. |
| `rfi-irfos-addition` | Not from her framework at all. |

Only `eight_layer` and `uip_check` are directly hers. The classification method (keyword
matching) applying those names to your text is this project's own operationalization, not
something her paper specifies an algorithm for. `resonance` uses the same general idea as her
paper's CCET metric applied to a different question with a cruder mechanism — it is explicitly
**not** CCET. `ecocentric` is entirely RFI-IRFOS's own addition. A review tool bearing a named
researcher's identity only has integrity if you can verify, per finding, whether it's really her
work or this project's own judgment call — that's the whole point of this section.

## Usage

```rust
use lauras_core::schema::{Lens, ReviewRequest};

let req = ReviewRequest {
    text: "your document here".into(),
    lenses: Lens::ALL.to_vec(),
    metadata: None,
};
let response = lauras_core::review(&req);
```

### Output shape

```rust
pub struct ReviewResponse {
    pub summary: String,
    pub lenses: Vec<LensResult>,
}

pub struct LensResult {
    pub lens: Lens,               // eight_layer | uip_check | resonance | ecocentric
    pub source: Source,           // which of the three attribution buckets above
    pub attribution_note: String, // plain-language statement of what this lens can't do
    pub findings: Vec<Finding>,   // claim + evidence (verbatim span) + severity
    pub data: Option<serde_json::Value>, // lens-specific payload, e.g. eight_layer's per-layer distribution
    pub error: Option<String>,    // populated instead of `findings` on partial failure
}
```

Run a subset instead of all four by passing e.g. `vec![Lens::EightLayer, Lens::UipCheck]` as
`lenses` — each lens executes independently either way.

## The lauras family

This crate is the free, deterministic foundation everything else builds on:

- **[`lauras-mcp`](https://crates.io/crates/lauras-mcp)** — stdio MCP server exposing this
  crate's review as `review_plan`, plus Laura's 15-agent team review as `review_team`, zero
  setup beyond `cargo install`.
- **[`lauras-team`](https://crates.io/crates/lauras-team)** — 15 domain-specialist agents
  (security, legal, finance, privacy, ethics...) running the same evidence-anchored discipline
  as this crate, reconciled into one prioritized synthesis.
- **`lauras-api`** — the same two tools over HTTP, deployed at
  [laura-api.fly.dev](https://laura-api.fly.dev), also the endpoint behind the
  [Smithery](https://smithery.ai) MCP listing.

## License

LGPL-3.0-or-later. See the workspace root `LICENSE-LGPL`.
