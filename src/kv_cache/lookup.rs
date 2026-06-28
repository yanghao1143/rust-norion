use std::cmp::Ordering;

use super::cache::KvFusionCache;
use super::model::{MemoryEntry, MemoryMatch};
use super::ops::{cosine_similarity, memory_visible_to_scope};
use crate::tenant_scope::{TenantAccessKind, TenantScope};

impl KvFusionCache {
    pub fn lookup(&self, query: &[f32], limit: usize) -> Vec<MemoryMatch> {
        ranked_matches(self.entries.iter(), query, limit)
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
        )
    }
}

fn ranked_matches<'a>(
    entries: impl Iterator<Item = &'a MemoryEntry>,
    query: &[f32],
    limit: usize,
) -> Vec<MemoryMatch> {
    let mut matches = entries
        .map(|entry| MemoryMatch {
            id: entry.id,
            key: entry.key.clone(),
            similarity: cosine_similarity(query, &entry.vector),
            strength: entry.strength,
            vector: entry.vector.clone(),
        })
        .filter(|item| item.similarity > 0.05)
        .collect::<Vec<_>>();

    matches.sort_by(|a, b| {
        let a_score = a.similarity * a.strength;
        let b_score = b.similarity * b.strength;
        b_score.partial_cmp(&a_score).unwrap_or(Ordering::Equal)
    });
    matches.truncate(limit);
    matches
}
