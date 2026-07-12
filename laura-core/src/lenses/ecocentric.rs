//! The ecocentric lens — checks whether the document mentions any of a small set
//! of stakeholder/impact categories: environmental, downstream/third-party,
//! long-term/second-order, and at-scale/systemic effects.
//!
//! **This lens is entirely RFI-IRFOS's own addition. It is NOT part of Laura Serna
//! Gaviria's Human-AI Co-Evolution framework in any form** — her OSF preprint makes
//! no ecocentric claim. It ships alongside the other three lenses because the room
//! co-designing this project (Laura, Simeon, Claude) chose to include it, not
//! because it's grounded in her research.
//!
//! **Deterministic keyword-category presence check** — no LLM call. This is
//! deliberately the crudest lens in this crate: it can only tell you a category's
//! keywords never appear, never that the category is actually relevant or
//! irrelevant to this particular document. A plan about renaming an internal
//! variable will "fail" the environmental category exactly like a plan about a new
//! factory would — the lens has no way to tell those apart. `attribution_note`
//! says this without hedging.

use crate::schema::{Finding, Lens, LensResult, Severity, Source};

const ATTRIBUTION_NOTE: &str = "This lens is RFI-IRFOS's own addition and is NOT part of Laura Serna Gaviria's published Human-AI Co-Evolution framework (UIP/EIA/8-Layer/CCET) in any way. It ships in this tool by the co-designing team's own choice. Findings are a blunt keyword-category presence check, not a relevance judgment — it cannot tell whether a missing category is actually relevant to your document, only that its keywords don't appear. Read every finding with that limitation in mind, especially for documents where a category is plausibly irrelevant.";

struct Category {
    name: &'static str,
    keywords: &'static [&'static str],
}

const CATEGORIES: &[Category] = &[
    Category { name: "environmental/ecological impact", keywords: &["environment", "sustainab", "carbon", "ecolog", "energy use", "resource use"] },
    Category {
        name: "downstream/third-party stakeholders",
        keywords: &["downstream", "third part", "customer impact", "community", "affected users", "external stakeholder"],
    },
    Category {
        name: "long-term/second-order consequences",
        keywords: &["long-term", "long term", "years from now", "future generations", "second-order", "unintended consequence", "knock-on"],
    },
    Category { name: "at-scale/systemic effects", keywords: &["at scale", "if adopted broadly", "systemic", "industry-wide", "widespread adoption"] },
];

pub fn run(text: &str) -> LensResult {
    if text.trim().is_empty() {
        return LensResult {
            lens: Lens::Ecocentric,
            source: Source::RfiIrfosAddition,
            attribution_note: ATTRIBUTION_NOTE.to_string(),
            findings: vec![],
            data: None,
            error: Some("input text is empty".to_string()),
        };
    }

    let lower = text.to_lowercase();
    let mut findings = Vec::new();
    let mut present = Vec::new();
    let mut absent = Vec::new();

    for cat in CATEGORIES {
        if cat.keywords.iter().any(|k| lower.contains(k)) {
            present.push(cat.name);
        } else {
            absent.push(cat.name);
        }
    }

    // Anchor absence findings to the closing segment of the doc (where a plan's own
    // scope/limitations would typically be discussed), so there's still a real
    // quoted span even for a "missing" finding.
    let anchor = last_nonempty_chunk(text);

    for name in &absent {
        findings.push(Finding {
            claim: format!("No mention found of '{name}' — consider whether this is relevant to your document; this lens cannot judge relevance, only keyword presence."),
            evidence: anchor.clone(),
            severity: Severity::Note,
        });
    }

    if present.is_empty() {
        findings.insert(
            0,
            Finding {
                claim: "None of the four stakeholder/impact categories this lens checks for were mentioned anywhere in the document.".to_string(),
                evidence: anchor,
                severity: Severity::Info,
            },
        );
    }

    LensResult {
        lens: Lens::Ecocentric,
        source: Source::RfiIrfosAddition,
        attribution_note: ATTRIBUTION_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "categories_mentioned": present, "categories_absent": absent })),
        error: None,
    }
}

fn last_nonempty_chunk(text: &str) -> String {
    let chunk = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .last()
        .unwrap_or(text.trim());
    if chunk.chars().count() <= 200 {
        chunk.to_string()
    } else {
        let truncated: String = chunk.chars().take(200).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_present_category() {
        let result = run("This will affect the environment through increased carbon output.");
        let data = result.data.unwrap();
        let present = data["categories_mentioned"].as_array().unwrap();
        assert!(present.iter().any(|v| v.as_str().unwrap().contains("environmental")));
    }

    #[test]
    fn flags_all_categories_absent_on_narrow_doc() {
        let result = run("Rename the internal variable `foo` to `bar` in the config parser.");
        let data = result.data.unwrap();
        assert_eq!(data["categories_mentioned"].as_array().unwrap().len(), 0);
        assert_eq!(data["categories_absent"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn empty_text_reports_error() {
        assert!(run("").error.is_some());
    }
}
