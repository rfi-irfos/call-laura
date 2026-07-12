# laura-mcp

## What this is

A stdio [MCP](https://modelcontextprotocol.io) (Model Context Protocol) server exposing a
single tool, `review_plan`, backed by
[`call-laura-core`](https://crates.io/crates/call-laura-core) (source: `laura-core/`). Any
MCP-connected agent — Claude Code, Claude Desktop, or anything else speaking the protocol — can
call `review_plan` on a plan or document and get back structured, evidence-anchored findings
across four independent lenses, with zero setup beyond installing the binary.

## Why it matters

An agent that reviews its own plan before acting on it catches a specific, recurring failure
mode: absolute claims with nothing backing them ("this will scale," with no stated constraint),
"done" claims with no way to verify them, sections that quietly contradict each other, or a whole
category of consideration (e.g. long-term/systemic effects) that never gets mentioned at all.
`review_plan` exists to surface exactly that, deterministically, before the plan ships — not as
a second opinion from another LLM call (which would just be one more unverifiable judgment on
top of the first), but as a fixed, inspectable set of pattern checks that produce the same
answer every time on the same input.

**Read the workspace root README first** for what this is actually grounded in and, critically,
the "Attribution & Sourcing" section — every lens result tells you whether it's genuinely Laura
Serna Gaviria's own published framework or this project's own addition. That distinction matters
for how much weight you should put on any given finding.

## Install

```bash
cargo install laura-mcp
claude mcp add laura -s user -- laura-mcp
```

No API key or environment setup needed — `call-laura-core` is fully deterministic and local, so
there's nothing to configure and no external service this depends on at review time.

## Using it

Once connected, call the `review_plan` tool with a JSON body matching `call_laura_core`'s
`ReviewRequest`:

```jsonc
// tools/call, name: "review_plan"
{
  "text": "# Goals\n...\n# Success Criteria\n...",
  "lenses": ["eight_layer", "uip_check"] // omit entirely to run all four
}
```

The response is `call_laura_core`'s `ReviewResponse`: a `summary` string plus one `LensResult`
per lens actually run, each carrying its `source` attribution tag, an `attribution_note`
explaining that lens's limits in plain language, and a list of `findings` — each one a `claim`,
a verbatim `evidence` span from your own input, and a `severity`. See the `call-laura-core`
README for the full type shapes.

## License

Business Source License 1.1 (non-commercial/research use permitted). See the workspace root
`LICENSE-BSL`.
