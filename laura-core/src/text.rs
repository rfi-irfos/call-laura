//! Shared text-segmentation heuristic used by `eight_layer` (per-segment layer
//! classification) and `resonance` (per-section embedding comparison).
//!
//! Naive on purpose for v1, per the plan (Sec. 2): split on markdown headings first;
//! if none are found, fall back to blank-line-separated paragraph blocks. Good enough
//! to bootstrap real usage and revisit once real documents show where it breaks.

#[derive(Debug, Clone)]
pub struct Segment {
    /// Heading text if this segment started with a markdown heading, else `None`.
    pub heading: Option<String>,
    pub body: String,
}

pub fn segment_text(text: &str) -> Vec<Segment> {
    let has_headings = text.lines().any(|l| l.trim_start().starts_with('#'));
    if has_headings {
        segment_by_headings(text)
    } else {
        segment_by_paragraphs(text)
    }
}

fn segment_by_headings(text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_body = String::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            if !current_body.trim().is_empty() || current_heading.is_some() {
                segments.push(Segment { heading: current_heading.take(), body: current_body.trim().to_string() });
            }
            current_heading = Some(trimmed.trim_start_matches('#').trim().to_string());
            current_body = String::new();
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if !current_body.trim().is_empty() || current_heading.is_some() {
        segments.push(Segment { heading: current_heading, body: current_body.trim().to_string() });
    }
    segments.into_iter().filter(|s| !s.body.is_empty() || s.heading.is_some()).collect()
}

fn segment_by_paragraphs(text: &str) -> Vec<Segment> {
    text.split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| Segment { heading: None, body: p.to_string() })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_markdown_headings() {
        let text = "# Intro\nfirst part\n\n# Goals\nsecond part\nmore";
        let segs = segment_text(text);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].heading.as_deref(), Some("Intro"));
        assert_eq!(segs[0].body, "first part");
        assert_eq!(segs[1].heading.as_deref(), Some("Goals"));
    }

    #[test]
    fn falls_back_to_paragraphs_when_no_headings() {
        let text = "first paragraph here.\n\nsecond paragraph here.\n\nthird one.";
        let segs = segment_text(text);
        assert_eq!(segs.len(), 3);
        assert!(segs.iter().all(|s| s.heading.is_none()));
    }

    #[test]
    fn empty_input_yields_no_segments() {
        assert!(segment_text("").is_empty());
        assert!(segment_text("   \n\n  ").is_empty());
    }
}
