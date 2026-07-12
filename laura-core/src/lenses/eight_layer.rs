//! The 8-Layer lens — classifies each segment of the submitted document against
//! Laura Serna Gaviria's IEIA-2025 "8-Layer Model" taxonomy and surfaces which
//! layers are under-represented.
//!
//! **Deterministic, keyword-based classification.** An earlier version of this
//! lens called out to an LLM per segment (the approach the reference implementation,
//! `emergent-interaction-lab/backend/src/thinking_fragments.rs`, itself uses). This
//! version replaces that with plain keyword matching: fully local, free, and
//! reproducible — the same document always produces the same classification, and
//! you can read the exact keyword list below to see why any given segment was
//! classified the way it was. The trade-off is real: keyword matching misses
//! layers expressed without their trigger words and can mis-fire on words used in
//! an unrelated sense. `attribution_note` says so explicitly.
//!
//! Attribution: the eight category names themselves are Laura's own (from the OSF
//! preprint) — `Source::Laura8Layer2025`. The classification method (keyword
//! matching) is this project's own operationalization, same disclosure convention
//! as the source app's own `DEFINITIONS_NOTE`.

use crate::schema::{Finding, Lens, LensResult, Severity, Source};
use crate::text::segment_text;

pub const LAYER_KEYS: &[&str] =
    &["facts", "analysis", "patterns", "hypotheses", "symbols", "action", "counterarguments", "blind_spot"];

struct LayerKeywords {
    key: &'static str,
    triggers: &'static [&'static str],
}

const LAYERS: &[LayerKeywords] = &[
    LayerKeywords {
        key: "facts",
        triggers: &[
            "data show", "observed", "recorded", "measured", "the fact that", "according to", "confirmed that",
            "we found", "the result was", "test result", "benchmark",
        ],
    },
    LayerKeywords {
        key: "analysis",
        triggers: &[
            "because", "therefore", "this means", "implies", "as a result", "due to", "in other words",
            "this suggests", "the reason", "which explains",
        ],
    },
    LayerKeywords {
        key: "patterns",
        triggers: &[
            "recurring", "repeatedly", "same pattern", "consistent with", "similar to", "every time", "each time",
            "trend", "pattern of", "recurs",
        ],
    },
    LayerKeywords {
        key: "hypotheses",
        triggers: &[
            "might", "may be", "could be", "perhaps", "assume", "hypothesis", "we believe", "possibly",
            "it's possible", "presumably", "likely that",
        ],
    },
    LayerKeywords {
        key: "symbols",
        triggers: &["metaphor", "like a", "as if", "symbolizes", "represents the idea of", "imagine", "picture this", "analogy"],
    },
    LayerKeywords {
        key: "action",
        triggers: &[
            "will ship", "next step", "we will", "plan to", "decide to", "must ", "should ", "todo", "action item",
            "deploy", "implement", "we'll ", "let's ",
        ],
    },
    LayerKeywords {
        key: "counterarguments",
        triggers: &[
            "however", "but ", "on the other hand", "risk", "concern", "limitation", "objection", "alternatively",
            "in contrast", "downside", "trade-off", "tradeoff",
        ],
    },
    LayerKeywords {
        key: "blind_spot",
        triggers: &[
            "unsure", "don't know", "unknown", "unclear", "tbd", "to be determined", "gap in", "not yet clear",
            "uncertain", "open question", "not sure",
        ],
    },
];

fn classify_segment(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    let mut scored: Vec<(&'static str, usize)> = LAYERS
        .iter()
        .map(|l| (l.key, l.triggers.iter().filter(|t| lower.contains(*t)).count()))
        .filter(|(_, count)| *count > 0)
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().take(3).map(|(k, _)| k).collect()
}

const ATTRIBUTION_NOTE: &str = "The eight layer names (facts, analysis, patterns, hypotheses, symbols, action, counterarguments, blind_spot) are Laura Serna Gaviria's own IEIA-2025 \"8-Layer Model\" (see the OSF preprint). Assigning a specific document segment to 1-3 of these layers here is done by deterministic keyword matching (see laura-core source for the exact trigger lists), not semantic understanding — a segment can express a layer without any trigger word and be missed, or contain a trigger word used in an unrelated sense. This is a full trade of semantic nuance for reproducibility, transparency, and zero external dependency; it is not a validated cognitive-science instrument either way.";

pub fn run(text: &str) -> LensResult {
    let segments = segment_text(text);
    if segments.is_empty() {
        return LensResult {
            lens: Lens::EightLayer,
            source: Source::Laura8Layer2025,
            attribution_note: ATTRIBUTION_NOTE.to_string(),
            findings: vec![],
            data: None,
            error: Some("input text had no classifiable segments".to_string()),
        };
    }

    let mut counts: std::collections::HashMap<&str, usize> = LAYER_KEYS.iter().map(|k| (*k, 0)).collect();
    let mut segment_layers: Vec<(String, Vec<&str>)> = Vec::new();

    for seg in &segments {
        let layers = classify_segment(&seg.body);
        for l in &layers {
            if let Some(c) = counts.get_mut(l) {
                *c += 1;
            }
        }
        segment_layers.push((seg.body.clone(), layers));
    }

    let total = segment_layers.len();
    let distribution: serde_json::Value = LAYER_KEYS
        .iter()
        .map(|k| {
            let n = *counts.get(k).unwrap_or(&0);
            (k.to_string(), serde_json::json!({ "count": n, "pct": (n as f64 / total as f64 * 100.0).round() }))
        })
        .collect::<serde_json::Map<_, _>>()
        .into();

    let mut findings = vec![Finding {
        claim: format!("Classified {total} segment(s) across the 8-Layer taxonomy via keyword matching."),
        evidence: segment_layers.first().map(|(t, _)| truncate(t, 200)).unwrap_or_default(),
        severity: Severity::Info,
    }];

    let closing = segment_layers.last().map(|(t, _)| t.clone()).unwrap_or_default();
    let opening = segment_layers.first().map(|(t, _)| t.clone()).unwrap_or_default();
    for key in LAYER_KEYS {
        if *counts.get(key).unwrap_or(&0) == 0 {
            let anchor = match *key {
                "action" | "counterarguments" | "blind_spot" => &closing,
                _ => &opening,
            };
            findings.push(Finding {
                claim: format!("No segment matched any '{key}' keyword — this layer is absent, or present but phrased without a recognized trigger word."),
                evidence: truncate(anchor, 200),
                severity: Severity::Note,
            });
        }
    }

    LensResult {
        lens: Lens::EightLayer,
        source: Source::Laura8Layer2025,
        attribution_note: ATTRIBUTION_NOTE.to_string(),
        findings,
        data: Some(serde_json::json!({ "segments_classified": total, "distribution": distribution })),
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
    fn classify_segment_detects_action_keywords() {
        let layers = classify_segment("We will deploy this next week and implement the fix.");
        assert!(layers.contains(&"action"));
    }

    #[test]
    fn classify_segment_detects_multiple_layers() {
        let layers = classify_segment("However, there is a risk: we believe this might fail because the data show inconsistent results.");
        assert!(layers.contains(&"counterarguments"));
        assert!(layers.contains(&"hypotheses"));
    }

    #[test]
    fn classify_segment_returns_empty_when_no_triggers() {
        let layers = classify_segment("Blue sky green grass");
        assert!(layers.is_empty());
    }

    #[test]
    fn run_flags_absent_layers() {
        let text = "# Plan\nWe will deploy this next week.";
        let result = run(text);
        assert!(result.error.is_none());
        let absent_flags: Vec<_> = result.findings.iter().filter(|f| f.claim.contains("blind_spot")).collect();
        assert_eq!(absent_flags.len(), 1);
    }

    #[test]
    fn run_on_empty_text_reports_error() {
        let result = run("");
        assert!(result.error.is_some());
    }
}
