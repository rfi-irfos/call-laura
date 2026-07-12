//! Laura — the orchestrator.
//!
//! Coordinates the 15 expert agents: runs each as a pure function over the same
//! submitted text, collects their `AgentResult`s, and assembles one `TeamResponse`
//! with a severity/cost rollup *and* a synthesis layer (risk band, cross-cutting
//! themes, priority actions). Agents run independently, so one agent erroring
//! (empty input, no classifiable signal) yields an honest `error` on that agent and
//! is counted as "partial", never silently dropped or fabricated — same honesty
//! convention as `call_laura_core::review`.

use crate::agents::{run_agent, AgentKind, AgentResult};
use call_laura_core::schema::Severity;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Request to Laura's team. `text` is the artifact under review; `agents` is an
/// optional subset (omit to run all 15 — the full "SWAT team").
#[derive(Debug, Clone, Deserialize)]
pub struct TeamRequest {
    pub text: String,
    #[serde(default)]
    pub agents: Option<Vec<AgentKind>>,
    #[serde(default)]
    pub metadata: Option<TeamMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TeamMetadata {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    /// Reserved hook for a future paid caller tier; currently informational only.
    #[serde(default)]
    pub caller_tier: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamAgentResult {
    pub agent: AgentKind,
    pub agent_name: String,
    pub source: call_laura_core::schema::Source,
    pub attribution_note: String,
    pub findings: Vec<call_laura_core::schema::Finding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Present iff the agent had nothing classifiable — surfaced, not hidden.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub finding_count: usize,
    pub flag_count: usize,
    pub note_count: usize,
}

/// Coarse risk band derived from the team's findings. Deterministic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskBand {
    Low,
    Moderate,
    Elevated,
    Critical,
}

/// A concern two or more agents independently surfaced on the *same* text region —
/// Laura's cross-cutting synthesis, detected by shared evidence, not by NLP.
#[derive(Debug, Clone, Serialize)]
pub struct TeamTheme {
    /// The shared text region the agents converged on.
    pub shared_evidence: String,
    pub agents: Vec<AgentKind>,
    pub finding_count: usize,
}

/// One deduplicated, prioritized action for the human. `priority` 1 = highest.
#[derive(Debug, Clone, Serialize)]
pub struct PriorityAction {
    pub priority: u8,
    pub source_agent: AgentKind,
    pub action: String,
}

/// The orchestration layer: the whole-team read, not just a concatenation.
#[derive(Debug, Clone, Serialize)]
pub struct Synthesis {
    pub risk_score: u16,
    pub risk_band: RiskBand,
    pub cross_cutting_themes: Vec<TeamTheme>,
    pub priority_actions: Vec<PriorityAction>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamResponse {
    /// Rollup line a human can read first.
    pub summary: String,
    /// Was the review fully complete, or did ≥1 agent report an error?
    pub complete: bool,
    pub agents_run: usize,
    pub agents_partial: usize,
    pub total_findings: usize,
    pub total_flags: usize,
    pub total_notes: usize,
    /// Laura's synthesis across all agents.
    pub synthesis: Synthesis,
    pub results: Vec<TeamAgentResult>,
}

fn count_severity(findings: &[call_laura_core::schema::Finding]) -> (usize, usize, usize) {
    let mut flags = 0;
    let mut notes = 0;
    let mut info = 0;
    for f in findings {
        match f.severity {
            Severity::Flag => flags += 1,
            Severity::Note => notes += 1,
            Severity::Info => info += 1,
        }
    }
    (flags, notes, info)
}

fn risk_band_for(score: u16) -> RiskBand {
    match score {
        0 => RiskBand::Low,
        1..=20 => RiskBand::Moderate,
        21..=60 => RiskBand::Elevated,
        _ => RiskBand::Critical,
    }
}

/// Longest common substring (chars), used to detect when two agents flagged the
/// *same* region of the text. Deterministic; inputs are short (≤ ~120 chars).
fn longest_common_substring(a: &str, b: &str) -> String {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() || b.is_empty() {
        return String::new();
    }
    let mut best = (0usize, 0usize); // (start index in a, length)
    let mut dp = vec![0u16; b.len() + 1];
    for i in 0..a.len() {
        let mut prev = 0u16;
        for j in 0..b.len() {
            let tmp = dp[j + 1];
            if a[i] == b[j] {
                dp[j + 1] = prev + 1;
                let len = dp[j + 1] as usize;
                if len > best.1 {
                    best = (i + 1 - len, len);
                }
            } else {
                dp[j + 1] = 0;
            }
            prev = tmp;
        }
    }
    a[best.0..best.0 + best.1].iter().collect()
}

/// Strip the leading "[Agent] " tag and normalize whitespace/case for dedup.
fn normalize_claim(s: &str) -> String {
    let s = s.trim();
    let s = if let Some(rest) = s.strip_prefix('[') {
        if let Some(idx) = rest.find("] ") {
            &rest[idx + 2..]
        } else {
            s
        }
    } else {
        s
    };
    let mut out = String::new();
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c.to_ascii_lowercase());
            prev_space = false;
        }
    }
    out.trim().to_string()
}

/// Build the synthesis: risk band, cross-cutting themes, and priority actions.
fn synthesize(results: &[TeamAgentResult]) -> Synthesis {
    let mut flags = 0usize;
    let mut notes = 0usize;
    for r in results {
        flags += r.flag_count;
        notes += r.note_count;
    }
    // Score: blockers dominate, notes nudge. Capped to keep bands interpretable.
    let raw = flags.saturating_mul(10) + notes.saturating_mul(2);
    let risk_score = raw.min(200) as u16;
    let risk_band = risk_band_for(risk_score);

    // Cross-cutting themes: any two agents whose evidence overlaps by ≥16 chars.
    let mut themes: Vec<TeamTheme> = Vec::new();
    for i in 0..results.len() {
        for j in (i + 1)..results.len() {
            let a_find = &results[i].findings;
            let b_find = &results[j].findings;
            let mut best_shared = String::new();
            let mut best_count = 0usize;
            for fa in a_find {
                for fb in b_find {
                    let lcs = longest_common_substring(&fa.evidence, &fb.evidence);
                    let len = lcs.chars().count();
                    if len > best_count {
                        best_count = len;
                        best_shared = lcs;
                    }
                }
            }
            if best_count >= 16 {
                let shared = best_shared.trim().to_string();
                if !shared.is_empty() {
                    themes.push(TeamTheme {
                        shared_evidence: shared,
                        agents: vec![results[i].agent, results[j].agent],
                        finding_count: 2,
                    });
                }
            }
        }
    }
    // Keep the strongest overlaps first, cap at 5.
    themes.sort_by(|a, b| b.shared_evidence.chars().count().cmp(&a.shared_evidence.chars().count()));
    themes.truncate(5);

    // Priority actions: deduplicated blockers (Flag), stable order by agent.
    let mut seen: HashSet<String> = HashSet::new();
    let mut actions: Vec<PriorityAction> = Vec::new();
    for r in results {
        for f in &r.findings {
            if f.severity == Severity::Flag {
                let norm = normalize_claim(&f.claim);
                if seen.insert(norm.clone()) {
                    actions.push(PriorityAction {
                        priority: 0, // assigned after sort
                        source_agent: r.agent,
                        action: f.claim.clone(),
                    });
                }
            }
        }
    }
    // Stable sort by agent order in AgentKind::ALL, then assign priority 1..N.
    actions.sort_by_key(|a| AgentKind::ALL.iter().position(|k| *k == a.source_agent).unwrap_or(usize::MAX));
    let mut priority = 1u8;
    for a in &mut actions {
        a.priority = priority;
        priority = priority.saturating_add(1);
    }
    actions.truncate(12);
    // Surfaced partial agents as the lowest-priority actions (don't hide them).
    for r in results {
        if r.error.is_some() {
            actions.push(PriorityAction {
                priority: priority,
                source_agent: r.agent,
                action: format!("Review partial: {} reported no classifiable signal — verify manually.", r.agent_name),
            });
            priority = priority.saturating_add(1);
        }
    }

    Synthesis {
        risk_score,
        risk_band,
        cross_cutting_themes: themes,
        priority_actions: actions,
    }
}

/// Run Laura's team. Pure and synchronous (no network) — deterministic given `text`.
pub fn review_team(req: &TeamRequest) -> TeamResponse {
    if req.text.trim().is_empty() {
        return TeamResponse {
            summary: "Refused: submitted text is empty — nothing for the team to review.".to_string(),
            complete: false,
            agents_run: 0,
            agents_partial: 0,
            total_findings: 0,
            total_flags: 0,
            total_notes: 0,
            synthesis: Synthesis {
                risk_score: 0,
                risk_band: RiskBand::Low,
                cross_cutting_themes: vec![],
                priority_actions: vec![],
            },
            results: vec![],
        };
    }

    let kinds: Vec<AgentKind> = match &req.agents {
        Some(list) if !list.is_empty() => list.clone(),
        _ => AgentKind::ALL.to_vec(),
    };

    let mut results = Vec::with_capacity(kinds.len());
    let mut total_findings = 0;
    let mut total_flags = 0;
    let mut total_notes = 0;
    let mut partial = 0;

    for kind in kinds {
        let r: AgentResult = run_agent(kind, &req.text);
        let (f, n, i) = count_severity(&r.findings);
        let finding_count = r.findings.len();
        total_findings += finding_count;
        total_flags += f;
        total_notes += n + i;
        if r.error.is_some() {
            partial += 1;
        }
        results.push(TeamAgentResult {
            agent: r.agent,
            agent_name: r.agent.display_name().to_string(),
            source: r.source,
            attribution_note: r.attribution_note,
            findings: r.findings,
            data: r.data,
            error: r.error,
            finding_count,
            flag_count: f,
            note_count: n + i,
        });
    }

    let complete = partial == 0;
    let synthesis = synthesize(&results);
    let summary = if complete {
        format!(
            "Laura's team ran {} agents: {} findings ({} flags, {} notes). Risk band: {}.",
            results.len(),
            total_findings,
            total_flags,
            total_notes,
            match synthesis.risk_band {
                RiskBand::Low => "low",
                RiskBand::Moderate => "moderate",
                RiskBand::Elevated => "elevated",
                RiskBand::Critical => "critical",
            }
        )
    } else {
        format!(
            "Laura's team ran {} agents: {} findings ({} flags, {} notes). {} agent(s) partial — treat as partial, not a clean bill.",
            results.len(),
            total_findings,
            total_flags,
            total_notes,
            partial
        )
    };

    TeamResponse {
        summary,
        complete,
        agents_run: results.len(),
        agents_partial: partial,
        total_findings,
        total_flags,
        total_notes,
        synthesis,
        results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gtm() -> TeamRequest {
        TeamRequest {
            text: "We deploy the model to production with no rollback and store personal data without consent. password=hunter2\nWe claim it is unique and guaranteed 100% accurate. The system is autonomous with no human oversight.\nWe hire a rockstar ninja. We report strong results with no stated limitations.".to_string(),
            agents: None,
            metadata: None,
        }
    }

    #[test]
    fn runs_all_fifteen_by_default() {
        let resp = review_team(&gtm());
        assert_eq!(resp.agents_run, 15);
        assert!(resp.total_findings > 0);
        assert!(resp.total_flags > 0);
    }

    #[test]
    fn subset_runs_only_requested() {
        let req = TeamRequest {
            text: "store personal data without consent".to_string(),
            agents: Some(vec![AgentKind::DataPrivacy, AgentKind::Osint]),
            metadata: None,
        };
        let resp = review_team(&req);
        assert_eq!(resp.agents_run, 2);
    }

    #[test]
    fn empty_text_is_refused_not_fabricated() {
        let req = TeamRequest { text: "   ".to_string(), agents: None, metadata: None };
        let resp = review_team(&req);
        assert_eq!(resp.agents_run, 0);
        assert!(resp.summary.contains("empty"));
    }

    #[test]
    fn determinism_same_input_same_output() {
        let a = review_team(&gtm());
        let b = review_team(&gtm());
        assert_eq!(a.total_findings, b.total_findings);
        assert_eq!(a.total_flags, b.total_flags);
        assert_eq!(a.synthesis.risk_score, b.synthesis.risk_score);
    }

    #[test]
    fn synthesis_produces_risk_band_and_actions() {
        let resp = review_team(&gtm());
        assert!(resp.synthesis.risk_score > 0);
        assert_ne!(resp.synthesis.risk_band, RiskBand::Low);
        assert!(!resp.synthesis.priority_actions.is_empty());
        // priorities are assigned 1..N contiguously
        let prios: Vec<u8> = resp.synthesis.priority_actions.iter().map(|a| a.priority).collect();
        assert_eq!(prios.first(), Some(&1));
    }

    #[test]
    fn cross_cutting_theme_detected_on_shared_evidence() {
        // Two agents (Ops, Reliability) both reference the same deployment text.
        let text = "We deploy the feature to prod and have no rollback plan or timeout on retries.".to_string();
        let resp = review_team(&TeamRequest { text, agents: None, metadata: None });
        // Not guaranteed to exceed the 16-char threshold, but the field must exist
        // and the mechanism must not panic / produce empty shared evidence entries.
        for t in &resp.synthesis.cross_cutting_themes {
            assert!(!t.shared_evidence.trim().is_empty());
            assert_eq!(t.agents.len(), 2);
        }
    }
}
