use crate::runtime::RuntimeTokenId;

pub(super) fn local_tokenize(text: &str) -> Vec<String> {
    let tokens = text
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| ch.is_ascii_punctuation())
                .to_owned()
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if !tokens.is_empty() {
        return tokens;
    }

    text.chars()
        .filter(|ch| !ch.is_whitespace())
        .map(|ch| ch.to_string())
        .collect()
}

pub(super) fn embed_tokens(tokens: &[RuntimeTokenId], dimensions: usize) -> Vec<f32> {
    let dimensions = dimensions.max(1);
    let mut vector = vec![0.0; dimensions];

    for token in tokens {
        let hash = stable_hash(&token.text);
        for offset in 0..4 {
            let index = ((hash >> (offset * 8)) as usize) % dimensions;
            vector[index] += 1.0 / (offset as f32 + 1.0);
        }
    }

    normalize(&mut vector);
    vector
}

pub(super) fn estimated_entropy(token: &str) -> f32 {
    let unique_chars = token
        .chars()
        .collect::<std::collections::HashSet<_>>()
        .len();
    (0.12 + unique_chars as f32 / 32.0).clamp(0.05, 1.25)
}

pub(super) fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(super) fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

pub(super) fn normalize(values: &mut [f32]) {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in values {
            *value /= norm;
        }
    }
}
