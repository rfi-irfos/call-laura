//! The UIP-check lens — evaluates the submitted document against Laura Serna
//! Gaviria's User Integrity Protocol (UIP), the four operational rules from her
//! OSF preprint (Appendix A): no reinterpretation of explicit negations/consent, no
//! fabrication or hallucination of facts, triple verification (factual, logical,
//! formal), and transparency/auditability of every claim.
//!
//! **Deterministic, sentence-level heuristics** — no LLM call, no external
//! dependency. Each rule below is checked via plain substring/keyword matching over
//! individually-split sentences, deliberately scoped to what a rule-based approach
//! can honestly claim to detect:
//! - Rule 1 (negations/constraints): surfaced as informational — every explicit
//!   negation/constraint statement is listed for the reader to verify isn't
//!   contradicted elsewhere; this version does NOT claim to have detected an actual
//!   contradiction (that needs semantic understanding this approach doesn't have).
//! - Rule 2 (fabrication): flags absolute/certainty language with no nearby hedge
//!   or source reference.
//! - Rule 3 (verification): flags completion claims ("done", "works") with no
//!   nearby verification-method word in the same sentence.
//! - Rule 4 (transparency): flags sentences opening with a conclusion marker with
//!   no reasoning-connector word in the same or previous sentence.
//!
//! Every finding's evidence is a verbatim substring of the input — enforced
//! structurally here, since findings are only ever constructed from sentences
//! actually present in the text, not generated separately and checked after the fact.

use crate::schema::{Finding, Lens, LensResult, Severity, Source};

const ATTRIBUTION_NOTE: &str = "The four rules checked here are Laura Serna Gaviria's own User Integrity Protocol (UIP), stated directly in her OSF preprint (Appendix A). The findings below come from deterministic keyword/pattern matching over your document, not semantic understanding or Laura's own review — this trades real detection power (e.g. it cannot tell whether a stated constraint is ACTUALLY contradicted elsewhere, only that constraints exist and are worth checking) for being fully reproducible and inspectable. Read findings as prompts to re-check the cited sentence yourself, not as a verified verdict.";

fn split_sentences(text: &str) -> Vec<&str> {
    text.split(|c| c == '.' || c == '!' || c == '?' || c == '\n')
        .map(|s| s.trim())
        .filter(|s| s.len() > 3)
        .collect()
}

const NEGATION_MARKERS: &[&str] = &["must not", "will not", "won't", "never ", "no third-party", "not be shared", "not permitted", "shall not", "cannot be", "is not allowed"];
const ABSOLUTE_MARKERS: &[&str] = &["guarantee", "100%", "always works", "never fails", "proven to", "certainly will", "definitely will", "no exceptions", "without fail"];
const HEDGE_OR_SOURCE_MARKERS: &[&str] = &["approximately", "estimated", "based on", "see ", "source:", "according to", "in testing", "in our tests", "roughly", "in most cases"];
const COMPLETION_MARKERS: &[&str] = &["is complete", "is done", "is finished", "works correctly", "has been tested", "has passed", "is working", "is ready"];
const VERIFICATION_MARKERS: &[&str] = &["test", "verified", "confirmed", "validated", "checked", "reviewed by", "audit"];
const CONCLUSION_MARKERS: &[&str] = &["therefore", "thus", "so we ", "in conclusion", "as a result we", "this proves"];
const REASONING_MARKERS: &[&str] = &["because", "since", "given that", "due to", "as a result of"];

pub fn run(text: &str) -> LensResult {
    let sentences = split_sentences(text);
    if sentences.is_empty() {
        return LensResult {
            lens: Lens::UipCheck,
            source: Source::LauraUip2025,
            attribution_note: ATTRIBUTION_NOTE.to_string(),
            findings: vec![],
            data: None,
            error: Some("input text had no checkable sentences".to_string()),
        };
    }

    let mut findings = Vec::new();

    for (i, sentence) in sentences.iter().enumerate() {
        let lower = sentence.to_lowercase();

        // Rule 1: surface stated constraints (informational, not an accusation).
        if NEGATION_MARKERS.iter().any(|m| lower.contains(m)) {
            findings.push(Finding {
                claim: "[UIP rule 1] States an explicit constraint/negation — worth confirming it's honored consistently across the rest of the document.".to_string(),
                evidence: sentence.to_string(),
                severity: Severity::Info,
            });
        }

        // Rule 2: absolute claim without nearby hedge/source.
        if ABSOLUTE_MARKERS.iter().any(|m| lower.contains(m)) && !HEDGE_OR_SOURCE_MARKERS.iter().any(|m| lower.contains(m)) {
            findings.push(Finding {
                claim: "[UIP rule 2] Absolute/certainty language with no nearby hedge, source, or evidence reference in the same sentence.".to_string(),
                evidence: sentence.to_string(),
                severity: Severity::Flag,
            });
        }

        // Rule 3: completion claim without nearby verification word.
        if COMPLETION_MARKERS.iter().any(|m| lower.contains(m)) && !VERIFICATION_MARKERS.iter().any(|m| lower.contains(m)) {
            findings.push(Finding {
                claim: "[UIP rule 3] States something is complete/working with no verification method mentioned in the same sentence.".to_string(),
                evidence: sentence.to_string(),
                severity: Severity::Flag,
            });
        }

        // Rule 4: conclusion marker with no reasoning connector nearby (this or previous sentence).
        if CONCLUSION_MARKERS.iter().any(|m| lower.starts_with(m) || lower.contains(m)) {
            let prev_has_reasoning = i > 0 && REASONING_MARKERS.iter().any(|m| sentences[i - 1].to_lowercase().contains(m));
            let this_has_reasoning = REASONING_MARKERS.iter().any(|m| lower.contains(m));
            if !prev_has_reasoning && !this_has_reasoning {
                findings.push(Finding {
                    claim: "[UIP rule 4] Conclusion stated with no visible reasoning connector in this sentence or the one before it.".to_string(),
                    evidence: sentence.to_string(),
                    severity: Severity::Note,
                });
            }
        }
    }

    LensResult {
        lens: Lens::UipCheck,
        source: Source::LauraUip2025,
        attribution_note: ATTRIBUTION_NOTE.to_string(),
        findings,
        data: None,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_absolute_claim_without_hedge() {
        let result = run("We guarantee this will never fail under any load.");
        assert!(result.findings.iter().any(|f| f.claim.contains("rule 2") && f.severity == Severity::Flag));
    }

    #[test]
    fn does_not_flag_absolute_claim_with_hedge() {
        let result = run("Based on our tests, we guarantee this works for the cases we checked.");
        assert!(!result.findings.iter().any(|f| f.claim.contains("rule 2")));
    }

    #[test]
    fn flags_completion_claim_without_verification() {
        let result = run("The migration is complete and ready for production use.");
        assert!(result.findings.iter().any(|f| f.claim.contains("rule 3")));
    }

    #[test]
    fn does_not_flag_completion_claim_with_verification() {
        let result = run("The migration is complete; it has been tested against the staging environment.");
        assert!(!result.findings.iter().any(|f| f.claim.contains("rule 3")));
    }

    #[test]
    fn surfaces_negation_as_info_not_flag() {
        let result = run("User data will not be shared with third parties.");
        let f = result.findings.iter().find(|f| f.claim.contains("rule 1")).expect("should find rule 1");
        assert_eq!(f.severity, Severity::Info);
    }

    #[test]
    fn every_evidence_is_verbatim_from_input() {
        let text = "We guarantee this will never fail. The system is complete.";
        let result = run(text);
        for f in &result.findings {
            assert!(text.contains(&f.evidence), "evidence not found verbatim in source: {}", f.evidence);
        }
    }

    #[test]
    fn empty_text_reports_error() {
        assert!(run("").error.is_some());
    }
}
