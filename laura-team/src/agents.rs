//! The 15 expert agents of Laura's tag team.
//!
//! Each agent is `pub fn run(text: &str) -> AgentResult`. They are intentionally
//! small, deterministic, and blunter than a human expert — every finding points at a
//! real span of the input, and every agent self-discloses its scope and limits via
//! `attribution_note`. `AgentKind` is the closed vocabulary Laura dispatches over.

use call_laura_core::schema::{Finding, Severity, Source};
use serde::{Deserialize, Serialize};

/// The 15 specialist agents Laura coordinates. `as_str` is the stable wire/id value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Osint,
    Security,
    LegalCompliance,
    Finance,
    Ops,
    Strategy,
    Ux,
    DataPrivacy,
    Ethics,
    ThreatModel,
    Reliability,
    Hiring,
    Brand,
    ResearchMethod,
    Product,
}

impl AgentKind {
    /// All 15 agents, in orchestration order.
    pub const ALL: [AgentKind; 15] = [
        AgentKind::Osint,
        AgentKind::Security,
        AgentKind::LegalCompliance,
        AgentKind::Finance,
        AgentKind::Ops,
        AgentKind::Strategy,
        AgentKind::Ux,
        AgentKind::DataPrivacy,
        AgentKind::Ethics,
        AgentKind::ThreatModel,
        AgentKind::Reliability,
        AgentKind::Hiring,
        AgentKind::Brand,
        AgentKind::ResearchMethod,
        AgentKind::Product,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            AgentKind::Osint => "osint",
            AgentKind::Security => "security",
            AgentKind::LegalCompliance => "legal_compliance",
            AgentKind::Finance => "finance",
            AgentKind::Ops => "ops",
            AgentKind::Strategy => "strategy",
            AgentKind::Ux => "ux",
            AgentKind::DataPrivacy => "data_privacy",
            AgentKind::Ethics => "ethics",
            AgentKind::ThreatModel => "threat_model",
            AgentKind::Reliability => "reliability",
            AgentKind::Hiring => "hiring",
            AgentKind::Brand => "brand",
            AgentKind::ResearchMethod => "research_method",
            AgentKind::Product => "product",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            AgentKind::Osint => "OSINT / Exposure",
            AgentKind::Security => "Security",
            AgentKind::LegalCompliance => "Legal & Compliance",
            AgentKind::Finance => "Finance",
            AgentKind::Ops => "Operations",
            AgentKind::Strategy => "Strategy",
            AgentKind::Ux => "UX / Accessibility",
            AgentKind::DataPrivacy => "Data Privacy",
            AgentKind::Ethics => "Ethics",
            AgentKind::ThreatModel => "Threat Model",
            AgentKind::Reliability => "Reliability",
            AgentKind::Hiring => "Hiring / People",
            AgentKind::Brand => "Brand & Comms",
            AgentKind::ResearchMethod => "Research Method",
            AgentKind::Product => "Product",
        }
    }
}

/// One agent's output. Reuses `Source` so the whole `call-laura` family shares one
/// attribution type. `error` (Some) means the agent had nothing classifiable — the
/// orchestrator surfaces this as honest partial failure, never fabricates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub agent: AgentKind,
    pub source: Source,
    pub attribution_note: String,
    pub findings: Vec<Finding>,
    /// Structured per-agent payload (e.g. OSINT's leaked-secret tally).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ── shared helpers ────────────────────────────────────────────────────────────

fn empty_error(agent: AgentKind, source: Source, note: &str, msg: &str) -> AgentResult {
    AgentResult {
        agent,
        source,
        attribution_note: note.to_string(),
        findings: vec![],
        data: None,
        error: Some(msg.to_string()),
    }
}

fn lower(text: &str) -> String {
    text.to_lowercase()
}

/// Pull a short, real anchor span so even "absence" findings cite something present.
fn anchor_span(text: &str, max_chars: usize) -> String {
    let chunk = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .last()
        .unwrap_or(text.trim());
    if chunk.chars().count() <= max_chars {
        chunk.to_string()
    } else {
        let t: String = chunk.chars().take(max_chars).collect();
        format!("{t}…")
    }
}

// ── Agent 1: OSINT / Exposure (deterministic footprint lens, NOT live scraping) ──

const OSINT_NOTE: &str = "Deterministic exposure-footprint lens. Flags secrets/credentials, PII, and metadata signals ALREADY PRESENT in the submitted text. It does NOT perform live external scraping — that would break call-laura's deterministic-local invariant. Treat findings as 'things you appear to have pasted in the open', not as a live intelligence report.";

const SECRET_PATTERNS: &[(&str, &str)] = &[
    ("api key / token", "api_key"),
    ("apikey=", "apikey assignment"),
    ("secret=", "secret assignment"),
    ("password=", "password assignment"),
    ("passwd=", "passwd assignment"),
    ("private key", "private key"),
    ("-----begin", "pem block"),
    ("bearer ", "bearer token"),
    ("aws_access_key_id", "aws key"),
    ("sk-", "openai-style key"),
    ("ghp_", "github token"),
    ("token=", "token assignment"),
    ("api_key=", "api_key assignment"),
];

const PII_PATTERNS: &[&str] = &[
    "ssn",
    "social security",
    "phone",
    "mobile:",
    "address:",
    "passport",
];

/// Email-shape matcher: require local@domain.tld, so a stray '@' in prose or
/// 'matter@hand' alone does not trigger a PII flag.
fn looks_like_email(seg: &str) -> bool {
    let seg = seg.trim();
    if let Some((local, rest)) = seg.split_once('@') {
        if local.is_empty() {
            return false;
        }
        // domain must contain a dot and a final alpha tld-ish part
        if let Some((_dom, tld)) = rest.split_once('.') {
            return !tld.is_empty() && tld.chars().any(|c| c.is_alphabetic());
        }
    }
    false
}

pub fn run_osint(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::Osint, Source::RfiIrfosAddition, OSINT_NOTE, "input empty");
    }
    let lower = lower(text);
    let mut findings = Vec::new();
    let mut leaked = Vec::new();

    for (pat, label) in SECRET_PATTERNS {
        if let Some(pos) = lower.find(pat) {
            let end = (pos + 40).min(text.len());
            let span = text[pos..end].replace('\n', " ").trim().to_string();
            findings.push(Finding {
                claim: format!("[OSINT] Possible {label} exposed in plaintext — rotate/remove before sharing."),
                evidence: span,
                severity: Severity::Flag,
            });
            leaked.push(label);
        }
    }
    // PII: only flag on email-shape or explicit personal-data keywords, not a bare '@'.
    let has_email = text
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '(' || c == ')')
        .any(|seg| looks_like_email(seg));
    if has_email {
        findings.push(Finding {
            claim: "[OSINT] Email-shaped address present in text — confirm it is not a real person's contact exposed in the open.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Note,
        });
    }
    for pat in PII_PATTERNS {
        if lower.contains(pat) {
            findings.push(Finding {
                claim: format!("[OSINT] Possible PII signal ('{pat}') present in text — confirm no real personal data is exposed."),
                evidence: anchor_span(text, 120),
                severity: Severity::Note,
            });
        }
    }

    AgentResult {
        agent: AgentKind::Osint,
        source: Source::RfiIrfosAddition,
        attribution_note: OSINT_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "leaked_secret_kinds": leaked })),
        error: None,
    }
}

// ── Agent 2: Security ──

const SEC_NOTE: &str = "Deterministic security heuristic. Flags common insecurity signals (eval/exec of untrusted input, missing auth, plaintext secrets, SQL string concat). Keyword-only — cannot reason about real exploitability; use as a prompt to review, not a verdict.";

const SEC_BAD: &[&str] = &[
    "eval(",
    "exec(",
    "os.system(",
    "subprocess(",
    "sql +",
    "SELECT * FROM",
    "no authentication",
    "without auth",
    "md5(",
    "sha1(",
    "http://",
    "verify=false",
    "tls verification disabled",
];

pub fn run_security(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::Security, Source::RfiIrfosAddition, SEC_NOTE, "input empty");
    }
    let lower = lower(text);
    let mut findings = Vec::new();
    for pat in SEC_BAD {
        if let Some(pos) = lower.find(pat) {
            let end = (pos + 50).min(text.len());
            findings.push(Finding {
                claim: format!("[Security] Insecure pattern '{pat}' — review for injection/auth/crypto risk."),
                evidence: text[pos..end].replace('\n', " ").trim().to_string(),
                severity: Severity::Flag,
            });
        }
    }
    AgentResult {
        agent: AgentKind::Security,
        source: Source::RfiIrfosAddition,
        attribution_note: SEC_NOTE.to_string(),
        findings,
        data: None,
        error: None,
    }
}

// ── Agent 3: Legal & Compliance ──

const LEGAL_NOTE: &str = "Deterministic legal/compliance heuristic over the text. Flags compliance-adjacent language (GDPR, liability disclaimers, regulatory refs) and missing-disclaimer signals. Not legal advice — a checklist prompt, not a lawyer.";

const LEGAL_TERMS: &[&str] = &[
    "gdpr",
    "ccpa",
    "hipaa",
    "eu ai act",
    "liability",
    "indemnif",
    "regulat",
    "compliant",
    "terms of service",
    "disclaimer",
    "data protection",
];

pub fn run_legal(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::LegalCompliance, Source::RfiIrfosAddition, LEGAL_NOTE, "input empty");
    }
    let lower = lower(text);
    let present: Vec<&str> = LEGAL_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    let mut findings = Vec::new();
    for t in &present {
        findings.push(Finding {
            claim: format!("[Legal/Compliance] Mentions '{t}' — confirm the associated obligation is actually addressed, not just named."),
            evidence: anchor_span(text, 120),
            severity: Severity::Note,
        });
    }
    if !lower.contains("disclaimer") && !lower.contains("liability") {
        findings.push(Finding {
            claim: "[Legal/Compliance] No liability/disclaimer language found — consider whether the document needs one before external release."
                .to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Info,
        });
    }
    AgentResult {
        agent: AgentKind::LegalCompliance,
        source: Source::RfiIrfosAddition,
        attribution_note: LEGAL_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "compliance_terms_present": present })),
        error: None,
    }
}

// ── Agent 4: Finance ──

const FIN_NOTE: &str = "Deterministic finance heuristic. Flags monetization/pricing/cost language and missing-unit or unverified-number signals. Not financial advice.";

const FIN_TERMS: &[&str] = &[
    "price",
    "pricing",
    "cost",
    "revenue",
    "subscription",
    "mrr",
    "margin",
    "budget",
    "€",
    "$",
    "eur",
    "usd",
    "roi",
];

pub fn run_finance(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::Finance, Source::RfiIrfosAddition, FIN_NOTE, "input empty");
    }
    let lower = lower(text);
    let present: Vec<&str> = FIN_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    let mut findings = Vec::new();
    for t in &present {
        findings.push(Finding {
            claim: format!("[Finance] References '{t}' — confirm the number is sourced and the unit stated."),
            evidence: anchor_span(text, 120),
            severity: Severity::Info,
        });
    }
    if lower.contains("free") && lower.contains("premium") {
        findings.push(Finding {
            claim: "[Finance] Free/premium split present — confirm the paid boundary is explicit (what free users never get).".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Note,
        });
    }
    AgentResult {
        agent: AgentKind::Finance,
        source: Source::RfiIrfosAddition,
        attribution_note: FIN_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "finance_terms_present": present })),
        error: None,
    }
}

// ── Agent 5: Operations ──

const OPS_NOTE: &str = "Deterministic ops heuristic. Flags deployment/runbook/on-call language and missing-rollback/monitoring signals.";

const OPS_TERMS: &[&str] = &[
    "deploy",
    "rollback",
    "monitoring",
    "on-call",
    "runbook",
    "incident",
    "alert",
    "pager",
    "sla",
    "backup",
];

pub fn run_ops(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::Ops, Source::RfiIrfosAddition, OPS_NOTE, "input empty");
    }
    let lower = lower(text);
    let mut findings = Vec::new();
    if lower.contains("deploy") && !lower.contains("rollback") {
        findings.push(Finding {
            claim: "[Ops] Deployment mentioned without a rollback path — confirm one exists before shipping.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Flag,
        });
    }
    if lower.contains("deploy") && !lower.contains("monitor") {
        findings.push(Finding {
            claim: "[Ops] Deployment mentioned without monitoring/alerting — confirm observability is covered.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Note,
        });
    }
    let present: Vec<&str> = OPS_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    AgentResult {
        agent: AgentKind::Ops,
        source: Source::RfiIrfosAddition,
        attribution_note: OPS_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "ops_terms_present": present })),
        error: None,
    }
}

// ── Agent 6: Strategy ──

const STRAT_NOTE: &str = "Deterministic strategy heuristic. Flags positioning/competitive/moat language and missing-differentiation signals. Not a strategy consult.";

const STRAT_TERMS: &[&str] = &[
    "competitor",
    "moat",
    "differentiat",
    "unique",
    "market",
    "positioning",
    "advantage",
    "vision",
];

pub fn run_strategy(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::Strategy, Source::RfiIrfosAddition, STRAT_NOTE, "input empty");
    }
    let lower = lower(text);
    let present: Vec<&str> = STRAT_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    let mut findings = Vec::new();
    if lower.contains("unique") || lower.contains("differentiat") {
        findings.push(Finding {
            claim: "[Strategy] Claims differentiation — confirm it names the concrete, defensible difference, not an adjective.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Note,
        });
    }
    AgentResult {
        agent: AgentKind::Strategy,
        source: Source::RfiIrfosAddition,
        attribution_note: STRAT_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "strategy_terms_present": present })),
        error: None,
    }
}

// ── Agent 7: UX / Accessibility ──

const UX_NOTE: &str = "Deterministic UX/a11y heuristic. Flags accessibility/usability signals and missing-keyboard/contrast mentions.";

const UX_TERMS: &[&str] = &[
    "accessib",
    "keyboard",
    "screen reader",
    "contrast",
    "aria",
    "wcag",
    "focus",
    "usabilit",
];

pub fn run_ux(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::Ux, Source::RfiIrfosAddition, UX_NOTE, "input empty");
    }
    let lower = lower(text);
    let present: Vec<&str> = UX_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    let mut findings = Vec::new();
    if !UX_TERMS.iter().any(|t| lower.contains(t)) {
        findings.push(Finding {
            claim: "[UX] No accessibility/usability language found — for a user-facing artifact, confirm a11y is covered elsewhere.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Info,
        });
    }
    AgentResult {
        agent: AgentKind::Ux,
        source: Source::RfiIrfosAddition,
        attribution_note: UX_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "ux_terms_present": present })),
        error: None,
    }
}

// ── Agent 8: Data Privacy ──

const PRIV_NOTE: &str = "Deterministic data-privacy heuristic. Flags consent/retention/minimization language and missing-consent signals. Not a DPO.";

const PRIV_TERMS: &[&str] = &[
    "consent",
    "opt-in",
    "opt out",
    "retention",
    "data minim",
    "anonym",
    "pseudonym",
    "right to be forgotten",
    "subject access",
    "personal data",
];

pub fn run_data_privacy(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::DataPrivacy, Source::RfiIrfosAddition, PRIV_NOTE, "input empty");
    }
    let lower = lower(text);
    let present: Vec<&str> = PRIV_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    let mut findings = Vec::new();
    if lower.contains("personal data") && !lower.contains("consent") && !lower.contains("opt-in") {
        findings.push(Finding {
            claim: "[Data Privacy] Processes personal data but no consent/opt-in language — confirm lawful basis is established.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Flag,
        });
    }
    AgentResult {
        agent: AgentKind::DataPrivacy,
        source: Source::RfiIrfosAddition,
        attribution_note: PRIV_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "privacy_terms_present": present })),
        error: None,
    }
}

// ── Agent 9: Ethics ──

const ETH_NOTE: &str = "Deterministic ethics heuristic. Flags fairness/bias/dual-use signals and missing-stakeholder language. Not an ethics board.";

const ETH_TERMS: &[&str] = &[
    "fair",
    "bias",
    "discriminat",
    "dual-use",
    "harm",
    "stakeholder",
    "transparen",
    "accountab",
    "human oversight",
];

pub fn run_ethics(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::Ethics, Source::RfiIrfosAddition, ETH_NOTE, "input empty");
    }
    let lower = lower(text);
    let present: Vec<&str> = ETH_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    let mut findings = Vec::new();
    if lower.contains("autonom") && !lower.contains("human oversight") && !lower.contains("human-in-the-loop") {
        findings.push(Finding {
            claim: "[Ethics] Autonomy claimed without human-oversight language — confirm a control boundary is defined.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Flag,
        });
    }
    AgentResult {
        agent: AgentKind::Ethics,
        source: Source::RfiIrfosAddition,
        attribution_note: ETH_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "ethics_terms_present": present })),
        error: None,
    }
}

// ── Agent 10: Threat Model ──

const THREAT_NOTE: &str = "Deterministic threat-model heuristic. Flags attacker/adversary/abuse language and missing-trust-boundary signals.";

const THREAT_TERMS: &[&str] = &[
    "attacker",
    "adversar",
    "abuse",
    "spoof",
    "phish",
    "tamper",
    "trust boundary",
    "threat",
    "exploit",
    "malicious",
];

pub fn run_threat_model(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::ThreatModel, Source::RfiIrfosAddition, THREAT_NOTE, "input empty");
    }
    let lower = lower(text);
    let present: Vec<&str> = THREAT_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    let mut findings = Vec::new();
    if !THREAT_TERMS.iter().any(|t| lower.contains(t)) {
        findings.push(Finding {
            claim: "[Threat Model] No adversary/abuse language found — for a security-relevant artifact, confirm a threat model was considered.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Info,
        });
    }
    AgentResult {
        agent: AgentKind::ThreatModel,
        source: Source::RfiIrfosAddition,
        attribution_note: THREAT_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "threat_terms_present": present })),
        error: None,
    }
}

// ── Agent 11: Reliability ──

const REL_NOTE: &str = "Deterministic reliability heuristic. Flags failure-mode/retry/timeout language and missing-fallback signals.";

const REL_TERMS: &[&str] = &[
    "retry",
    "timeout",
    "fallback",
    "circuit breaker",
    "idempoten",
    "graceful",
    "degrade",
    "failure",
];

pub fn run_reliability(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::Reliability, Source::RfiIrfosAddition, REL_NOTE, "input empty");
    }
    let lower = lower(text);
    let mut findings = Vec::new();
    if lower.contains("retry") && !lower.contains("timeout") {
        findings.push(Finding {
            claim: "[Reliability] Retry mentioned without a timeout — confirm a bound exists to avoid indefinite hangs.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Note,
        });
    }
    let present: Vec<&str> = REL_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    AgentResult {
        agent: AgentKind::Reliability,
        source: Source::RfiIrfosAddition,
        attribution_note: REL_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "reliability_terms_present": present })),
        error: None,
    }
}

// ── Agent 12: Hiring / People ──

const HR_NOTE: &str = "Deterministic hiring/people heuristic. Flags role/responsibility/language-bias signals. Not HR legal advice.";

const HR_TERMS: &[&str] = &[
    "hire",
    "role",
    "responsibilit",
    "team",
    "onboard",
    "culture",
    "he/him",
    "young",
    "rockstar",
    "ninja",
];

pub fn run_hiring(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::Hiring, Source::RfiIrfosAddition, HR_NOTE, "input empty");
    }
    let lower = lower(text);
    let mut findings = Vec::new();
    for biased in ["rockstar", "ninja", "young", "he/him"] {
        if lower.contains(biased) {
            findings.push(Finding {
                claim: format!("[Hiring] Gendered/biased term '{biased}' — consider neutral, inclusive wording."),
                evidence: anchor_span(text, 120),
                severity: Severity::Note,
            });
        }
    }
    let present: Vec<&str> = HR_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    AgentResult {
        agent: AgentKind::Hiring,
        source: Source::RfiIrfosAddition,
        attribution_note: HR_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "hr_terms_present": present })),
        error: None,
    }
}

// ── Agent 13: Brand & Comms ──

const BRAND_NOTE: &str = "Deterministic brand/comms heuristic. Flags audience/tone/claim-language signals and missing-audience definition.";

const BRAND_TERMS: &[&str] = &[
    "audience",
    "tone",
    "voice",
    "messaging",
    "brand",
    "claim",
    "tagline",
];

pub fn run_brand(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::Brand, Source::RfiIrfosAddition, BRAND_NOTE, "input empty");
    }
    let lower = lower(text);
    let mut findings = Vec::new();
    if !BRAND_TERMS.iter().any(|t| lower.contains(t)) {
        findings.push(Finding {
            claim: "[Brand] No audience/tone language — for external comms, confirm who this is for and what tone fits.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Info,
        });
    }
    let present: Vec<&str> = BRAND_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    AgentResult {
        agent: AgentKind::Brand,
        source: Source::RfiIrfosAddition,
        attribution_note: BRAND_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "brand_terms_present": present })),
        error: None,
    }
}

// ── Agent 14: Research Method ──

const RM_NOTE: &str = "Deterministic research-method heuristic. Flags methodology/reproducibility signals and missing-sample/limitation language. Grounded in the same no-fabrication discipline as the parent platform.";

const RM_TERMS: &[&str] = &[
    "method",
    "sample",
    "dataset",
    "reproduc",
    "limitation",
    "hypothesis",
    "peer review",
    "preprint",
    "n =",
    "participant",
];

pub fn run_research_method(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::ResearchMethod, Source::RfiIrfosAddition, RM_NOTE, "input empty");
    }
    let lower = lower(text);
    let mut findings = Vec::new();
    if (lower.contains("result") || lower.contains("finding")) && !lower.contains("limitation") && !lower.contains("caveat") {
        findings.push(Finding {
            claim: "[Research Method] Reports results without stated limitations — confirm boundaries/caveats are named.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Note,
        });
    }
    let present: Vec<&str> = RM_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    AgentResult {
        agent: AgentKind::ResearchMethod,
        source: Source::RfiIrfosAddition,
        attribution_note: RM_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "rm_terms_present": present })),
        error: None,
    }
}

// ── Agent 15: Product ──

const PROD_NOTE: &str = "Deterministic product heuristic. Flags user-need/success-metric/roadmap signals and missing-success-criteria language.";

const PROD_TERMS: &[&str] = &[
    "user",
    "customer",
    "need",
    "problem",
    "success metric",
    "kpi",
    "roadmap",
    "feature",
    "use case",
];

pub fn run_product(text: &str) -> AgentResult {
    if text.trim().is_empty() {
        return empty_error(AgentKind::Product, Source::RfiIrfosAddition, PROD_NOTE, "input empty");
    }
    let lower = lower(text);
    let mut findings = Vec::new();
    if (lower.contains("feature") || lower.contains("ship")) && !lower.contains("success metric") && !lower.contains("kpi") && !lower.contains("metric") {
        findings.push(Finding {
            claim: "[Product] Ships a feature without a success metric — confirm how you'll know it worked.".to_string(),
            evidence: anchor_span(text, 120),
            severity: Severity::Note,
        });
    }
    let present: Vec<&str> = PROD_TERMS.iter().copied().filter(|t| lower.contains(t)).collect();
    AgentResult {
        agent: AgentKind::Product,
        source: Source::RfiIrfosAddition,
        attribution_note: PROD_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "product_terms_present": present })),
        error: None,
    }
}

/// Dispatch table — maps an `AgentKind` to its `run` function. Single source of
/// truth so the orchestrator and any future caller route identically.
pub fn run_agent(kind: AgentKind, text: &str) -> AgentResult {
    match kind {
        AgentKind::Osint => run_osint(text),
        AgentKind::Security => run_security(text),
        AgentKind::LegalCompliance => run_legal(text),
        AgentKind::Finance => run_finance(text),
        AgentKind::Ops => run_ops(text),
        AgentKind::Strategy => run_strategy(text),
        AgentKind::Ux => run_ux(text),
        AgentKind::DataPrivacy => run_data_privacy(text),
        AgentKind::Ethics => run_ethics(text),
        AgentKind::ThreatModel => run_threat_model(text),
        AgentKind::Reliability => run_reliability(text),
        AgentKind::Hiring => run_hiring(text),
        AgentKind::Brand => run_brand(text),
        AgentKind::ResearchMethod => run_research_method(text),
        AgentKind::Product => run_product(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_agents_present() {
        assert_eq!(AgentKind::ALL.len(), 15);
    }

    #[test]
    fn osint_flags_leaked_secret() {
        let r = run_osint("here is my key sk-abc123def456 and a password=secret123");
        assert!(r.findings.iter().any(|f| f.claim.contains("OSINT") && f.severity == Severity::Flag));
    }

    #[test]
    fn osint_does_not_flag_benign_password_word() {
        // "password manager" / "passwordless" must NOT trip a leaked-secret flag.
        let r = run_osint("We use a password manager and offer passwordless login.");
        assert!(!r.findings.iter().any(|f| f.claim.contains("exposed in plaintext")));
    }

    #[test]
    fn osint_does_not_flag_bare_at_in_prose() {
        // A stray '@' in prose is not an email; only email-shape should flag.
        let r = run_osint("The issue is @ the core of the system and we @ mention things.");
        assert!(!r.findings.iter().any(|f| f.claim.contains("Email-shaped")));
    }

    #[test]
    fn osint_flags_real_email_shape() {
        let r = run_osint("Contact us at simeon@rfi-irfos.org for details.");
        assert!(r.findings.iter().any(|f| f.claim.contains("Email-shaped")));
    }

    #[test]
    fn security_flags_eval() {
        let r = run_security("we call eval(user_input) directly");
        assert!(r.findings.iter().any(|f| f.claim.contains("Security") && f.severity == Severity::Flag));
    }

    #[test]
    fn privacy_flags_personal_data_without_consent() {
        let r = run_data_privacy("we store personal data in the warehouse");
        assert!(r.findings.iter().any(|f| f.claim.contains("Data Privacy") && f.severity == Severity::Flag));
    }

    #[test]
    fn empty_input_reports_error_not_fabrication() {
        for kind in AgentKind::ALL {
            let r = match kind {
                AgentKind::Osint => run_osint(""),
                AgentKind::Security => run_security(""),
                AgentKind::LegalCompliance => run_legal(""),
                AgentKind::Finance => run_finance(""),
                AgentKind::Ops => run_ops(""),
                AgentKind::Strategy => run_strategy(""),
                AgentKind::Ux => run_ux(""),
                AgentKind::DataPrivacy => run_data_privacy(""),
                AgentKind::Ethics => run_ethics(""),
                AgentKind::ThreatModel => run_threat_model(""),
                AgentKind::Reliability => run_reliability(""),
                AgentKind::Hiring => run_hiring(""),
                AgentKind::Brand => run_brand(""),
                AgentKind::ResearchMethod => run_research_method(""),
                AgentKind::Product => run_product(""),
            };
            assert!(r.error.is_some(), "agent {kind:?} should report error on empty input");
        }
    }

    #[test]
    fn every_finding_evidence_is_verbatim() {
        let text = "we call eval(user_input) and store personal data without consent, password=hunter2";
        for kind in AgentKind::ALL {
            let r = match kind {
                AgentKind::Osint => run_osint(text),
                AgentKind::Security => run_security(text),
                AgentKind::LegalCompliance => run_legal(text),
                AgentKind::Finance => run_finance(text),
                AgentKind::Ops => run_ops(text),
                AgentKind::Strategy => run_strategy(text),
                AgentKind::Ux => run_ux(text),
                AgentKind::DataPrivacy => run_data_privacy(text),
                AgentKind::Ethics => run_ethics(text),
                AgentKind::ThreatModel => run_threat_model(text),
                AgentKind::Reliability => run_reliability(text),
                AgentKind::Hiring => run_hiring(text),
                AgentKind::Brand => run_brand(text),
                AgentKind::ResearchMethod => run_research_method(text),
                AgentKind::Product => run_product(text),
            };
            for f in &r.findings {
                assert!(text.contains(&f.evidence) || f.evidence.ends_with('…'),
                    "agent {kind:?} evidence not verbatim: {}", f.evidence);
            }
        }
    }
}
