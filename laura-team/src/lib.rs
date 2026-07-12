//! `laura-team` — Laura (the orchestrator) and the 15 deterministic expert agents.
//!
//! **Architecture invariant (inherited from `call-laura-core`):** every agent is a
//! *pure function of the submitted text*. No network call, no API key, same input always
//! produces the same output. This is the paid, proprietary expansion of the free
//! dogfood `review` core: the free tier keeps the 4 deterministic lenses
//! (`eight_layer`, `uip_check`, `resonance`, `ecocentric`); this crate adds Laura's
//! 15-agent "SWAT team" as the paid analysis module.
//!
//! **Honesty discipline (same as `call-laura-core`):** every `Finding` quotes a verbatim
//! span of the input as `evidence`; every agent result carries a mandatory `source`
//! (reusing `call_laura_core::schema::Source`) so a consumer can always tell whose logic
//! produced a given finding. Agents that have nothing classifiable return an `error`
//! field rather than fabricating — honest partial failure, surfaced by the orchestrator.
//!
//! **On "OSINT":** the `osint` agent is a *deterministic exposure-footprint lens* — it
//! flags leaked secrets, embedded credentials, exposed PII, and metadata signals that are
//! already present in the submitted text. It does NOT perform live external scraping; that
//! would break the deterministic-local invariant this whole project is built on (and Patent B's
//! novelty). Live OSINT gathering, if ever added, would be a separate, clearly-flagged
//! non-deterministic tool — not this agent.

pub mod agents;
pub mod orchestrator;

pub use orchestrator::{review_team, TeamRequest, TeamResponse, TeamAgentResult};
