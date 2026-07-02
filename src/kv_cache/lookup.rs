use std::cmp::Ordering;

use super::cache::KvFusionCache;
use super::model::{MemoryEntry, MemoryMatch};
use super::ops::{cosine_similarity, memory_value_score, memory_visible_to_scope};
use crate::tenant_scope::{TenantAccessKind, TenantScope};

impl KvFusionCache {
    pub fn lookup(&self, query: &[f32], limit: usize) -> Vec<MemoryMatch> {
        ranked_matches(self.entries.iter(), query, limit, self.clock)
    }

    pub fn lookup_scoped(
        &self,
        scope: &TenantScope,
        query: &[f32],
        limit: usize,
    ) -> Vec<MemoryMatch> {
        ranked_matches(
            self.entries
                .iter()
                .filter(|entry| memory_visible_to_scope(scope, &entry.key, TenantAccessKind::Read)),
            query,
            limit,
            self.clock,
        )
    }
}

fn ranked_matches<'a>(
    entries: impl Iterator<Item = &'a MemoryEntry>,
    query: &[f32],
    limit: usize,
    now: u64,
) -> Vec<MemoryMatch> {
    let mut matches = entries
        .map(|entry| {
            let similarity = cosine_similarity(query, &entry.vector);
            let retrieval_score = similarity * memory_value_score(entry, now).max(0.01);
            (
                MemoryMatch {
                    id: entry.id,
                    key: entry.key.clone(),
                    similarity,
                    strength: entry.strength,
                    vector: entry.vector.clone(),
                },
                retrieval_score,
            )
        })
        .filter(|(item, _)| item.similarity > 0.05)
        .collect::<Vec<_>>();

    matches.sort_by(|a, b| {
        let a_score = a.1;
        let b_score = b.1;
        b_score.partial_cmp(&a_score).unwrap_or(Ordering::Equal)
    });
    matches.truncate(limit);
    matches.into_iter().map(|(item, _)| item).collect()
}
