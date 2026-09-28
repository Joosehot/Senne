//! Deterministic diagnostics: when a phrase matches nothing, say so and
//! point at the closest thing the lexicon does know. This is the seam where
//! an optional AI helper could sit later (explaining a failure), strictly
//! outside the generation path.

use crate::lexicon::vocabulary;
use crate::parser::Diag;

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Closest known phrase, if any is reasonably close. Ties go to the
/// alphabetically first phrase, so the hint is deterministic too.
pub fn closest(word: &str) -> Option<&'static str> {
    let word = word.to_lowercase();
    let limit = (word.chars().count() / 3).max(1) + 1;
    vocabulary()
        .into_iter()
        .map(|v| (levenshtein(&word, v), v))
        .filter(|(d, _)| *d <= limit)
        .min()
        .map(|(_, v)| v)
}

pub fn unknown_word(word: &str) -> Diag {
    let d = Diag::new(format!("no rule knows the word \"{word}\""));
    match closest(word) {
        Some(c) => d.hint(format!("did you mean \"{c}\"? (or pass --lenient to ignore unknown words)")),
        None => d.hint("rephrase with known words, or pass --lenient to ignore it (run `senne --vocabulary` to list them)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_close_word() {
        assert_eq!(closest("retyr"), Some("retry"));
        assert_eq!(closest("paarse"), Some("parse"));
    }

    #[test]
    fn no_suggestion_for_garbage() {
        assert_eq!(closest("xylophone"), None);
    }
}
