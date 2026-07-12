//! `laura-core` — pure review logic behind `call-laura`.
//!
//! Fully deterministic and local: every lens is a pure function of the input text,
//! no network call, no API key, no external dependency. Same document in always
//! produces the same review out. See the crate README for what this is and, just
//! as importantly, what's actually Laura Serna Gaviria's own published work versus
//! this project's own additions — read that before assuming any given lens is "hers."

pub mod lenses;
pub mod mcp;
pub mod schema;
pub mod similarity;
pub mod text;

use schema::{ReviewRequest, ReviewResponse, Severity};
use schema::Lens as LensKind;

/// Runs every requested lens against `req.text` and assembles the overall response.
/// Purely synchronous — there is no I/O left to await.
pub fn review(req: &ReviewRequest) -> ReviewResponse {
    let text = &req.text;
    let results: Vec<_> = req
        .lenses
        .iter()
        .map(|lens| match lens {
            LensKind::EightLayer => lenses::eight_layer::run(text),
            LensKind::UipCheck => lenses::uip_check::run(text),
            LensKind::Resonance => lenses::resonance::run(text),
            LensKind::Ecocentric => lenses::ecocentric::run(text),
        })
        .collect();

    let summary = build_summary(&results);
    ReviewResponse { summary, lenses: results }
}

fn build_summary(results: &[schema::LensResult]) -> String {
    let mut parts = Vec::new();
    let ran: Vec<&schema::LensResult> = results.iter().filter(|r| r.error.is_none()).collect();
    let failed: Vec<&schema::LensResult> = results.iter().filter(|r| r.error.is_some()).collect();

    if ran.is_empty() {
        return "All requested lenses failed to run — see per-lens `error` fields for details. No review could be produced.".to_string();
    }

    let flags: usize = ran.iter().map(|r| r.findings.iter().filter(|f| f.severity == Severity::Flag).count()).sum();
    let notes: usize = ran.iter().map(|r| r.findings.iter().filter(|f| f.severity == Severity::Note).count()).sum();

    parts.push(format!(
        "Ran {} lens(es): {}.",
        ran.len(),
        ran.iter().map(|r| r.lens.as_str()).collect::<Vec<_>>().join(", ")
    ));
    if flags > 0 || notes > 0 {
        parts.push(format!("{flags} flagged issue(s), {notes} note(s) worth a second look — see per-lens findings for what and where."));
    } else {
        parts.push("No significant issues flagged across the requested lenses.".to_string());
    }
    if !failed.is_empty() {
        parts.push(format!(
            "{} lens(es) failed to run ({}) — treat this review as partial, not a clean bill of health.",
            failed.len(),
            failed.iter().map(|r| r.lens.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema::Lens;

    #[test]
    fn review_runs_all_four_lenses_by_default() {
        let req = ReviewRequest {
            text: "# Goals\nShip the feature fast.\n\n# Risks\nMight break existing users.".to_string(),
            lenses: Lens::ALL.to_vec(),
            metadata: None,
        };
        let resp = review(&req);
        assert_eq!(resp.lenses.len(), 4);
        assert!(!resp.summary.is_empty());
    }

    #[test]
    fn review_runs_only_requested_subset() {
        let req = ReviewRequest {
            text: "Some plan text here for testing purposes.".to_string(),
            lenses: vec![Lens::EightLayer],
            metadata: None,
        };
        let resp = review(&req);
        assert_eq!(resp.lenses.len(), 1);
        assert_eq!(resp.lenses[0].lens, Lens::EightLayer);
    }
}
