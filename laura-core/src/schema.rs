//! Shared request/response types used by every lens and both server binaries
//! (`laura-mcp`, `laura-api`).
//!
//! Attribution discipline (see plan `~/.claude/plans/buzzing-foraging-lemur.md` Sec.
//! 0 and README "Attribution & Sourcing"): every `LensResult` carries a `source` that
//! must be one of the three `Source` variants below — no lens is allowed to imply
//! authorship it doesn't have. This is not decoration; it's the thing that keeps this
//! tool honest about what's genuinely Laura Serna Gaviria's published work versus
//! this project's own operationalization or addition, matching the same disclosure
//! convention already established in `emergent-interaction-lab`'s `DEFINITIONS_NOTE`
//! (`chat.rs`, `thinking_fragments.rs`) and in the OSF preprint's own transparency
//! notes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lens {
    EightLayer,
    UipCheck,
    Resonance,
    Ecocentric,
}

impl Lens {
    pub const ALL: [Lens; 4] = [Lens::EightLayer, Lens::UipCheck, Lens::Resonance, Lens::Ecocentric];

    pub fn as_str(&self) -> &'static str {
        match self {
            Lens::EightLayer => "eight_layer",
            Lens::UipCheck => "uip_check",
            Lens::Resonance => "resonance",
            Lens::Ecocentric => "ecocentric",
        }
    }
}

/// Where a lens's underlying logic actually comes from — the mandatory,
/// impossible-to-omit attribution field on every `LensResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    /// Directly from Laura Serna Gaviria's published framework (the OSF preprint).
    /// Currently: the UIP's four rules, and the 8-Layer taxonomy's eight category
    /// names themselves.
    #[serde(rename = "laura-8layer-2025")]
    Laura8Layer2025,
    #[serde(rename = "laura-uip-2025")]
    LauraUip2025,
    /// This project's own operationalization of a concept Laura's paper names but
    /// does not itself define an algorithm for (e.g. how a specific passage gets
    /// classified into a layer, or what "low resonance between sections" means
    /// numerically). Not verbatim hers — matches the CCET disclosure precedent in
    /// `emergent-interaction-lab/backend/src/chat.rs`.
    #[serde(rename = "rfi-irfos-operationalization")]
    RfiIrfosOperationalization,
    /// Not from Laura's framework at all. Currently: the entire `ecocentric` lens.
    #[serde(rename = "rfi-irfos-addition")]
    RfiIrfosAddition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Note,
    Flag,
}

/// One observation from a lens. `evidence` MUST be a quoted span from the actual
/// input text — findings that can't point at what they're reacting to don't ship.
/// This is the concrete mechanism behind "genuinely useful feedback, not vibes."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub claim: String,
    pub evidence: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LensResult {
    pub lens: Lens,
    pub source: Source,
    pub attribution_note: String,
    pub findings: Vec<Finding>,
    /// Lens-specific structured payload (e.g. eight_layer's per-layer distribution).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Populated when the lens failed to run (e.g. NVIDIA API error) rather than
    /// silently omitting it or fabricating a result — an honest empty lens beats a
    /// convincing fictional one, matching this whole project's no-fabrication line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewRequest {
    pub text: String,
    #[serde(default = "Lens::ALL_vec")]
    pub lenses: Vec<Lens>,
    #[serde(default)]
    pub metadata: Option<ReviewMetadata>,
}

impl Lens {
    #[allow(non_snake_case)]
    fn ALL_vec() -> Vec<Lens> {
        Lens::ALL.to_vec()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewMetadata {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReviewResponse {
    pub summary: String,
    pub lenses: Vec<LensResult>,
}
