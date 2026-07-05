use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::engine::NoironEngine;
use crate::tenant_scope::{TenantResourceLane, TenantScopedKey};

use super::{StateMemorySummary, StateMemoryVectorDimensions};

pub(super) fn top_memory_summaries(
    engine: &NoironEngine,
    limit: usize,
    include: impl Fn(&str) -> bool,
) -> Vec<StateMemorySummary> {
    let mut top_memories = engine
        .cache
        .entries()
        .iter()
        .filter(|entry| include(&entry.key))
        .map(|entry| {
            let value_score =
                entry.strength + entry.hits as f32 * 0.04 - entry.failures as f32 * 0.10;
            (value_score, entry)
        })
        .collect::<Vec<_>>();
    top_memories.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.1.id.cmp(&right.1.id))
    });

    top_memories
        .into_iter()
        .take(limit)
        .map(|(_, entry)| StateMemorySummary {
            id: entry.id,
            key: compact(&entry.key, 120),
            vector_dimensions: entry.vector.len(),
            strength: entry.strength,
            hits: entry.hits,
            failures: entry.failures,
            last_score: entry.last_score,
        })
        .collect()
}

pub(super) fn memory_vector_dimensions(engine: &NoironEngine) -> Vec<StateMemoryVectorDimensions> {
    let mut buckets = BTreeMap::<usize, usize>::new();
    for entry in engine.cache.entries() {
        *buckets.entry(entry.vector.len()).or_insert(0) += 1;
    }

    buckets
        .into_iter()
        .map(|(dimensions, count)| StateMemoryVectorDimensions { dimensions, count })
        .collect()
}

pub(super) fn runtime_kv_vector_dimensions(
    engine: &NoironEngine,
) -> Vec<StateMemoryVectorDimensions> {
    let mut buckets = BTreeMap::<usize, usize>::new();
    for entry in engine
        .cache
        .entries()
        .iter()
        .filter(|entry| is_runtime_kv_memory_key(&entry.key))
    {
        *buckets.entry(entry.vector.len()).or_insert(0) += 1;
    }

    buckets
        .into_iter()
        .map(|(dimensions, count)| StateMemoryVectorDimensions { dimensions, count })
        .collect()
}

pub(super) fn is_runtime_kv_memory_key(key: &str) -> bool {
    key.starts_with("runtime_kv:")
        || TenantScopedKey::parse(key)
            .is_some_and(|scoped| scoped.lane == TenantResourceLane::RuntimeKv)
}

pub(super) fn format_memory_vector_dimensions(buckets: &[StateMemoryVectorDimensions]) -> String {
    if buckets.is_empty() {
        return "none".to_owned();
    }

    buckets
        .iter()
        .map(|bucket| format!("{}:{}", bucket.dimensions, bucket.count))
        .collect::<Vec<_>>()
        .join("|")
}

pub(super) fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}
