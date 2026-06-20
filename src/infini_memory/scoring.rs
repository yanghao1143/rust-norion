use std::cmp::Ordering;
use std::collections::HashSet;

use crate::kv_cache::{MemoryEntry, MemoryMatch};

use super::types::{InfiniMemoryItem, InfiniMemoryScope};

pub(super) fn local_item(memory: &MemoryMatch) -> InfiniMemoryItem {
    let normalized_strength = (memory.strength / 3.0).clamp(0.0, 1.0);
    let score = (memory.similarity.clamp(0.0, 1.0) * 0.72) + normalized_strength * 0.28;

    InfiniMemoryItem {
        id: memory.id,
        key: memory.key.clone(),
        vector: memory.vector.clone(),
        scope: InfiniMemoryScope::LocalWindow,
        score,
        estimated_tokens: estimate_tokens(&memory.key),
        reason: format!(
            "local_window:similarity={:.3}:strength={:.3}",
            memory.similarity, memory.strength
        ),
    }
}

pub(super) fn global_item(entry: &MemoryEntry, max_access: u64) -> InfiniMemoryItem {
    let attempts = entry.hits + entry.failures;
    let reliability = if attempts == 0 {
        0.5
    } else {
        entry.hits as f32 / attempts as f32
    };
    let recency = if max_access == 0 {
        1.0
    } else {
        1.0 - (max_access.saturating_sub(entry.last_access) as f32 / max_access as f32).min(1.0)
    };
    let score = (entry.strength / 3.0).clamp(0.0, 1.0) * 0.50
        + entry.last_score.clamp(0.0, 1.0) * 0.25
        + reliability * 0.15
        + recency * 0.10;

    InfiniMemoryItem {
        id: entry.id,
        key: entry.key.clone(),
        vector: entry.vector.clone(),
        scope: InfiniMemoryScope::GlobalMemory,
        score,
        estimated_tokens: estimate_tokens(&entry.key),
        reason: format!(
            "global_memory:strength={:.3}:last_score={:.3}:reliability={:.3}:recency={:.3}",
            entry.strength, entry.last_score, reliability, recency
        ),
    }
}

pub(super) fn skipped_item(mut item: InfiniMemoryItem, reason: &str) -> InfiniMemoryItem {
    item.scope = InfiniMemoryScope::Skipped;
    item.reason = format!("{reason}:source_score={:.3}", item.score);
    item
}

pub(super) fn sort_items(items: &mut [InfiniMemoryItem]) {
    items.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
}

pub(super) fn is_redundant(candidate: &str, selected: &[String], threshold: f32) -> bool {
    selected
        .iter()
        .any(|key| token_overlap(candidate, key) >= threshold)
}

fn token_overlap(left: &str, right: &str) -> f32 {
    let left_tokens = tokenize_key(left);
    let right_tokens = tokenize_key(right);

    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }

    let intersection = left_tokens.intersection(&right_tokens).count() as f32;
    let union = left_tokens.union(&right_tokens).count() as f32;
    intersection / union.max(1.0)
}

fn tokenize_key(value: &str) -> HashSet<String> {
    value
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

pub(super) fn estimate_tokens(value: &str) -> usize {
    let token_count = value
        .split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation())
        .filter(|part| !part.is_empty())
        .count();
    token_count.max(1)
}
