//! The resonance lens — compares each section of the document and flags pairs of
//! sections that should plausibly agree (e.g. stated goals vs. stated success
//! criteria) but show low similarity, plus the single most disconnected pair
//! overall as a general internal-consistency signal.
//!
//! **Deterministic, local term-frequency cosine similarity** (`crate::similarity`)
//! — no NVIDIA embeddings, no network call. Reuses `text::segment_text` for
//! sectioning. The section-pairing heuristic (keyword-matched "should agree" pairs,
//! plus a naive all-pairs floor) is unchanged from the earlier design.
//!
//! Attribution: `rfi-irfos-operationalization`. This is explicitly NOT the CCET
//! metric from Laura's paper or from the EIL platform, and — since the switch away
//! from embeddings — it is now a cruder, purely lexical measure (shared words, not
//! shared meaning) rather than semantic similarity. Two sections that agree in
//! substance but use different vocabulary will score lower here than an
//! embedding-based version would have shown. `attribution_note` says this plainly.

use crate::schema::{Finding, Lens, LensResult, Severity, Source};
use crate::similarity::{cosine, tokenize};
use crate::text::segment_text;

const PAIRED_LOW_THRESHOLD: f32 = 0.15;
const GENERAL_LOW_THRESHOLD: f32 = 0.05;

const ATTRIBUTION_NOTE: &str = "This lens is RFI-IRFOS's own operationalization, not a metric from Laura Serna Gaviria's paper or from the CCET implementation in the Emergent Interaction Lab platform. It measures purely lexical overlap (shared words after stopword removal) between sections of your document, not semantic meaning — two sections that say the same thing in different words will score low here. Treat findings as a prompt to re-read the cited sections yourself, not as a validated measurement of whether they actually agree.";

struct KeywordGroup {
    name: &'static str,
    keywords: &'static [&'static str],
}

const GOALS: KeywordGroup = KeywordGroup { name: "goals/objectives", keywords: &["goal", "objective", "aim", "purpose"] };
const SUCCESS: KeywordGroup =
    KeywordGroup { name: "success criteria/outcomes", keywords: &["success", "criteria", "outcome", "result", "kpi", "verification"] };
const RISKS: KeywordGroup = KeywordGroup { name: "risks/limitations", keywords: &["risk", "limitation", "concern", "constraint"] };
const MITIGATION: KeywordGroup = KeywordGroup { name: "mitigation/response", keywords: &["mitigation", "response", "contingency", "fallback"] };

const EXPECTED_PAIRS: &[(&KeywordGroup, &KeywordGroup)] = &[(&GOALS, &SUCCESS), (&RISKS, &MITIGATION)];

fn matches_group(heading: &str, group: &KeywordGroup) -> bool {
    let lower = heading.to_lowercase();
    group.keywords.iter().any(|k| lower.contains(k))
}

pub fn run(text: &str) -> LensResult {
    let segments = segment_text(text);
    if segments.len() < 2 {
        return LensResult {
            lens: Lens::Resonance,
            source: Source::RfiIrfosOperationalization,
            attribution_note: ATTRIBUTION_NOTE.to_string(),
            findings: vec![],
            data: None,
            error: Some("document has fewer than 2 sections — resonance requires at least 2 to compare".to_string()),
        };
    }

    let vectors: Vec<_> = segments
        .iter()
        .map(|seg| {
            let combined = match &seg.heading {
                Some(h) => format!("{h} {}", seg.body),
                None => seg.body.clone(),
            };
            tokenize(&combined)
        })
        .collect();

    let mut findings = Vec::new();
    let mut lowest_pair: Option<(usize, usize, f32)> = None;
    let mut pair_scores = Vec::new();

    for i in 0..segments.len() {
        for j in (i + 1)..segments.len() {
            let sim = cosine(&vectors[i], &vectors[j]);
            pair_scores.push(sim);
            if lowest_pair.map(|(_, _, s)| sim < s).unwrap_or(true) {
                lowest_pair = Some((i, j, sim));
            }
        }
    }

    for (group_a, group_b) in EXPECTED_PAIRS {
        let a_idx = segments.iter().position(|s| s.heading.as_deref().map(|h| matches_group(h, group_a)).unwrap_or(false));
        let b_idx = segments.iter().position(|s| s.heading.as_deref().map(|h| matches_group(h, group_b)).unwrap_or(false));
        if let (Some(ai), Some(bi)) = (a_idx, b_idx) {
            let sim = cosine(&vectors[ai], &vectors[bi]);
            if sim < PAIRED_LOW_THRESHOLD {
                findings.push(Finding {
                    claim: format!(
                        "'{}' and '{}' sections share very little vocabulary ({:.2} lexical similarity) despite being the kind of sections that should plausibly reinforce each other.",
                        group_a.name, group_b.name, sim
                    ),
                    evidence: truncate(&segments[ai].body, 150),
                    severity: Severity::Flag,
                });
            }
        }
    }

    if let Some((i, j, sim)) = lowest_pair {
        if sim < GENERAL_LOW_THRESHOLD {
            findings.push(Finding {
                claim: format!("The most lexically disconnected pair of sections shares almost no vocabulary (similarity {sim:.2}) — worth checking whether they belong in the same document."),
                evidence: format!("A: {} | B: {}", truncate(&segments[i].body, 100), truncate(&segments[j].body, 100)),
                severity: Severity::Note,
            });
        }
    }

    let mean = if pair_scores.is_empty() { 0.0 } else { pair_scores.iter().sum::<f32>() / pair_scores.len() as f32 };
    findings.insert(
        0,
        Finding {
            claim: format!("Compared {} section(s) across {} pair(s), mean lexical similarity {mean:.2}.", segments.len(), pair_scores.len()),
            evidence: segments.first().map(|s| truncate(&s.body, 150)).unwrap_or_default(),
            severity: Severity::Info,
        },
    );

    LensResult {
        lens: Lens::Resonance,
        source: Source::RfiIrfosOperationalization,
        attribution_note: ATTRIBUTION_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "sections": segments.len(), "mean_pairwise_similarity": mean })),
        error: None,
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_group_is_case_insensitive() {
        assert!(matches_group("Project GOALS", &GOALS));
        assert!(matches_group("success criteria", &SUCCESS));
        assert!(!matches_group("random heading", &GOALS));
    }

    #[test]
    fn run_reports_error_on_single_section() {
        let result = run("just one paragraph, no headings, no blank-line breaks");
        assert!(result.error.is_some());
    }

    #[test]
    fn run_computes_distribution_on_multi_section_doc() {
        let text = "# Goals\nShip faster and safer.\n\n# Success Criteria\nFaster shipping with fewer bugs.";
        let result = run(text);
        assert!(result.error.is_none());
        assert!(result.data.is_some());
    }
}
