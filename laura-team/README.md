# laura-team

**Call Laura's proprietary analysis module** — Laura (orchestrator) + a 15-agent expert
"SWAT team". This is the *paid* layer: the deterministic, LGPL `call-laura-core`
(review / `review_plan`) stays free; `laura-team` is BSL-1.1 and ships the full stack.

## Why it matters

`call-laura-core`'s four lenses catch a specific, narrow set of gaps in a plan or document.
Real-world review pulls in far more domains at once — security, legal exposure, finance,
operations, hiring language, accessibility — and a single generalist pass tends to either miss
whole categories or blur them into one vague verdict. `laura-team` runs 15 domain-specialist
agents over the *same* text in parallel, each one a narrow, deterministic pass looking for its
own category of concern, then synthesizes what they found: which regions of the text multiple
independent specialists flagged without being told to agree, and what the single highest-
priority set of blockers actually is. That's the novelty — not "one more LLM opinion," but 15
independently-reasoned, evidence-anchored passes reconciled into one prioritized answer, with
the same no-fabrication discipline as the free core: nothing is flagged that isn't a verbatim
span of your own input.

## What it is

Laura coordinates 15 deterministic expert agents. Each agent is a **pure function** of the
submitted text — run it twice on the same input, get the same output. No network, no LLM,
no API key at analysis time. This keeps the module consistent with the platform's
*evidence-anchored, attribution-tagged* discipline: every `Finding` quotes a verbatim span
of the input and carries a `Source` (here, `RfiIrfosAddition`), so nothing is hallucinated.

## The 15 agents

| # | Agent | What it flags |
|---|-------|---------------|
| 1 | OSINT / Exposure | Secrets/creds & PII *already* in the text (assignment-form only — `password=`, not "password manager"; email-shape only, not a bare `@`). Deterministic footprint lens, **not** live scraping. |
| 2 | Security | `eval`/`exec` of input, SQL concat, weak hashing, missing auth, `verify=false`. |
| 3 | Legal & Compliance | GDPR/CCPA/HIPAA/EU-AI-Act language, missing disclaimer/liability. Not legal advice. |
| 4 | Finance | Price/cost/revenue language; free/premium boundary; sourced numbers. |
| 5 | Operations | Deploy-without-rollback, deploy-without-monitoring. |
| 6 | Strategy | Differentiation claims; competitive/moat language. |
| 7 | UX / Accessibility | a11y/usability signals; missing accessibility coverage. |
| 8 | Data Privacy | Consent/retention/minimization; personal data without lawful basis. |
| 9 | Ethics | Fairness/bias/dual-use; autonomy without human oversight. |
| 10 | Threat Model | Adversary/abuse language; missing threat model. |
| 11 | Reliability | Retry-without-timeout, failure-mode coverage. |
| 12 | Hiring / People | Gendered/biased terms ("rockstar", "ninja"). |
| 13 | Brand & Comms | Audience/tone definition; external-comms readiness. |
| 14 | Research Method | Reproducibility; results without limitations. |
| 15 | Product | User-need/success-metric; feature without a success criterion. |

## Orchestration (what makes Laura more than a loop)

`review_team` fans out to all 15 (or a requested subset), then synthesizes:

- **Risk band** — `low | moderate | elevated | critical`, derived from a deterministic
  score (`flags × 10 + notes × 2`, capped).
- **Cross-cutting themes** — regions of the text that *two or more agents independently*
  flagged (detected by longest-common-substring on evidence, ≥16 chars). Surfaces the
  concerns multiple experts converge on.
- **Priority actions** — deduplicated blockers (`Flag` severity), stable-ordered by agent,
  with `priority` 1..N, plus any partial-agent "verify manually" notes.

## Honesty invariants

- Empty input → **refused**, not fabricated.
- An agent with nothing classifiable → reports an `error` field, counted as *partial*.
  The whole response is marked `complete: false`. Never silently dropped.
- Every finding's `evidence` is a verbatim substring of the input (or a truncated `…`).

## Usage

```rust
use laura_team::orchestrator::{review_team, TeamRequest};

let req = TeamRequest {
    text: "We deploy with no rollback and store personal data without consent.".into(),
    agents: None,            // None = run all 15; or Some(vec![Osint, DataPrivacy])
    metadata: None,
};
let resp = review_team(&req);
println!("{}", resp.summary);
println!("risk band: {:?}", resp.synthesis.risk_band);
```

### HTTP (paid route)

`laura-api` exposes `POST /team` (key-gated, rate-limited) returning the same
`TeamResponse` as JSON. The free `POST /review` (4-lens core) is unchanged.

## Build & test

```bash
cargo test -p laura-team
```
