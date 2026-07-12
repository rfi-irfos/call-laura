//! Local, deterministic text similarity — replaces the earlier NVIDIA-embedding-based
//! approach for the `resonance` lens. Plain term-frequency cosine similarity over a
//! small stopword-filtered bag of words. No network call, no API key, same input
//! always produces the same output.
//!
//! Traded away versus embeddings: genuine semantic similarity (two sections that say
//! the same thing in different words won't score highly here). Gained: zero cost,
//! zero external dependency, fully inspectable — you can read exactly why two
//! sections scored the way they did. `resonance`'s `attribution_note` says this
//! plainly; this is a deliberate trade, not a silent downgrade.

use std::collections::HashMap;

const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "if", "then", "of", "to", "in", "on", "for", "with", "as", "is", "are",
    "was", "were", "be", "been", "being", "this", "that", "these", "those", "it", "its", "at", "by", "from", "we",
    "you", "your", "our", "will", "would", "should", "can", "could", "not", "no", "do", "does", "did", "has", "have",
    "had", "so", "than", "into", "about", "which", "what", "who", "how", "when", "where", "there", "here", "also",
];

pub type TermVector = HashMap<String, u32>;

pub fn tokenize(text: &str) -> TermVector {
    let mut tv = TermVector::new();
    for raw in text.split(|c: char| !c.is_alphanumeric()) {
        let word = raw.to_lowercase();
        if word.len() <= 2 || STOPWORDS.contains(&word.as_str()) {
            continue;
        }
        *tv.entry(word).or_insert(0) += 1;
    }
    tv
}

pub fn cosine(a: &TermVector, b: &TermVector) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().map(|(term, count_a)| *count_a as f32 * *b.get(term).unwrap_or(&0) as f32).sum();
    let norm_a: f32 = (a.values().map(|c| (*c as f32).powi(2)).sum::<f32>()).sqrt();
    let norm_b: f32 = (b.values().map(|c| (*c as f32).powi(2)).sum::<f32>()).sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text_scores_one() {
        let a = tokenize("the plan will ship features quickly and safely");
        assert!((cosine(&a, &a) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn disjoint_text_scores_zero() {
        let a = tokenize("apples oranges bananas grapefruit");
        let b = tokenize("rockets satellites telescopes astronomy");
        assert_eq!(cosine(&a, &b), 0.0);
    }

    #[test]
    fn overlapping_text_scores_between_zero_and_one() {
        let a = tokenize("the goal is faster deployment and fewer bugs");
        let b = tokenize("success means faster deployment with fewer regressions");
        let sim = cosine(&a, &b);
        assert!(sim > 0.0 && sim < 1.0);
    }

    #[test]
    fn empty_text_is_zero_not_nan() {
        let a = tokenize("");
        let b = tokenize("something here");
        let result = cosine(&a, &b);
        assert_eq!(result, 0.0);
        assert!(!result.is_nan());
    }

    #[test]
    fn stopwords_and_short_tokens_are_excluded() {
        let tv = tokenize("the a an if it is to of we");
        assert!(tv.is_empty());
    }
}
