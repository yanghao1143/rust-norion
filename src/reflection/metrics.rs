use std::collections::HashSet;

use super::model::DraftToken;

pub(super) fn contains_conflicting_markers(answer: &str) -> bool {
    let lower = answer.to_ascii_lowercase();
    (lower.contains("certain") || lower.contains("guaranteed"))
        && (lower.contains("uncertain") || lower.contains("unknown") || lower.contains("maybe"))
}

pub(super) fn lexical_overlap(prompt: &str, answer: &str) -> f32 {
    let prompt_chars = prompt
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .collect::<HashSet<_>>();
    let answer_chars = answer
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .collect::<HashSet<_>>();

    if prompt_chars.is_empty() || answer_chars.is_empty() {
        return 0.0;
    }

    let shared = prompt_chars.intersection(&answer_chars).count() as f32;
    let denom = prompt_chars.len().min(answer_chars.len()) as f32;
    (shared / denom).clamp(0.0, 1.0)
}

pub(super) fn repetition_ratio(answer: &str) -> f32 {
    let words = answer
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|ch| !ch.is_ascii_punctuation())
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    if words.len() < 6 {
        return 0.0;
    }

    let unique = words.iter().collect::<HashSet<_>>().len();
    (1.0 - unique as f32 / words.len() as f32).clamp(0.0, 1.0)
}

pub(super) fn token_uncertainty(tokens: &[DraftToken]) -> Option<f32> {
    let mut scores = Vec::new();

    for token in tokens {
        if let Some(entropy) = token.entropy {
            scores.push((entropy / 4.0).clamp(0.0, 1.0));
        }
        if let Some(logprob) = token.logprob {
            scores.push((-logprob / 4.0).clamp(0.0, 1.0));
        }
    }

    if scores.is_empty() {
        None
    } else {
        Some(scores.iter().sum::<f32>() / scores.len() as f32)
    }
}
