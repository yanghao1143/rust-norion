use std::collections::HashSet;

use super::MemoryEntry;

pub(super) fn choose_compaction_pair(
    left: &MemoryEntry,
    right: &MemoryEntry,
    protected: &HashSet<u64>,
    now: u64,
) -> Option<(u64, u64)> {
    let left_protected = protected.contains(&left.id);
    let right_protected = protected.contains(&right.id);

    match (left_protected, right_protected) {
        (true, true) => None,
        (true, false) => Some((left.id, right.id)),
        (false, true) => Some((right.id, left.id)),
        (false, false) => {
            let left_score = memory_value_score(left, now);
            let right_score = memory_value_score(right, now);
            if left_score > right_score
                || ((left_score - right_score).abs() < 0.0001 && left.id < right.id)
            {
                Some((left.id, right.id))
            } else {
                Some((right.id, left.id))
            }
        }
    }
}

pub(super) fn merge_memory_entry(
    primary: &mut MemoryEntry,
    duplicate: &MemoryEntry,
    similarity: f32,
    now: u64,
) {
    let duplicate_weight = duplicate.strength.max(0.05);
    fuse_vector(
        &mut primary.vector,
        &duplicate.vector,
        primary.strength.max(0.05),
        duplicate_weight,
    );
    primary.key = merge_key(&primary.key, &duplicate.key);
    primary.strength = (primary.strength + duplicate.strength * 0.35).clamp(0.01, 3.0);
    primary.hits = primary
        .hits
        .saturating_add(duplicate.hits)
        .saturating_add(1);
    primary.failures = primary.failures.saturating_add(duplicate.failures);
    primary.last_score = primary.last_score.max(similarity);
    primary.created_at = primary.created_at.min(duplicate.created_at);
    primary.last_access = now.max(primary.last_access).max(duplicate.last_access);
}

pub(super) fn memory_namespace(key: &str) -> &'static str {
    if key.starts_with("runtime_kv:") {
        "runtime_kv"
    } else if key.starts_with("gist:") {
        "gist"
    } else {
        "semantic"
    }
}

pub(super) fn fuse_vector(
    existing: &mut Vec<f32>,
    incoming: &[f32],
    existing_weight: f32,
    incoming_weight: f32,
) {
    let len = existing.len().max(incoming.len());
    existing.resize(len, 0.0);
    let total = (existing_weight + incoming_weight).max(0.001);

    for (index, value) in existing.iter_mut().enumerate().take(len) {
        let current = *value * existing_weight;
        let next = incoming.get(index).copied().unwrap_or(0.0) * incoming_weight;
        *value = (current + next) / total;
    }
}

pub(super) fn merge_key(existing: &str, incoming: &str) -> String {
    if existing.contains(incoming) {
        return existing.to_owned();
    }
    if incoming.contains(existing) {
        return incoming.to_owned();
    }

    let mut merged = existing.to_owned();
    if merged.len() > 160 {
        truncate_to_char_boundary(&mut merged, 160);
    }
    merged.push_str(" | ");
    merged.push_str(incoming);
    if merged.len() > 260 {
        truncate_to_char_boundary(&mut merged, 260);
    }
    merged
}

pub(super) fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let len = left.len().max(right.len());
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;

    for index in 0..len {
        let l = left.get(index).copied().unwrap_or(0.0);
        let r = right.get(index).copied().unwrap_or(0.0);
        dot += l * r;
        left_norm += l * l;
        right_norm += r * r;
    }

    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        let raw = (dot / (left_norm.sqrt() * right_norm.sqrt())).clamp(-1.0, 1.0);
        (raw * dimension_compatibility(left, right)).clamp(-1.0, 1.0)
    }
}

pub(super) fn memory_value_score(entry: &MemoryEntry, now: u64) -> f32 {
    let idle = now.saturating_sub(entry.last_access) as f32;
    let idle_drag = (idle / 256.0).min(0.35);
    entry.strength - entry.failures as f32 * 0.08 + entry.hits as f32 * 0.02 - idle_drag
}

fn truncate_to_char_boundary(value: &mut String, max_len: usize) {
    if value.len() <= max_len {
        return;
    }

    let mut boundary = max_len;
    while boundary > 0 && !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
}

fn dimension_compatibility(left: &[f32], right: &[f32]) -> f32 {
    if left.len() == right.len() {
        return 1.0;
    }

    let shorter = left.len().min(right.len()) as f32;
    let longer = left.len().max(right.len()) as f32;
    if shorter == 0.0 || longer == 0.0 {
        0.0
    } else {
        (shorter / longer).powi(2)
    }
}
