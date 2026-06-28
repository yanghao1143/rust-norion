use std::cmp::Ordering;

use super::model::MemoryEntry;
use super::ops::{
    cosine_similarity, memory_keys_can_merge, memory_namespace, memory_value_score,
    memory_visible_to_scope,
};
use crate::tenant_scope::{TenantAccessKind, TenantScope};

#[derive(Debug, Clone)]
pub struct KvFusionCache {
    pub(super) entries: Vec<MemoryEntry>,
    pub(super) similarity_threshold: f32,
    pub(super) max_entries: usize,
    pub(super) next_id: u64,
    pub(super) clock: u64,
}

impl Default for KvFusionCache {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            similarity_threshold: 0.78,
            max_entries: 4096,
            next_id: 1,
            clock: 0,
        }
    }
}

impl KvFusionCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limits(similarity_threshold: f32, max_entries: usize) -> Self {
        Self {
            similarity_threshold: similarity_threshold.clamp(0.1, 0.99),
            max_entries: max_entries.max(1),
            ..Self::default()
        }
    }

    pub fn entries(&self) -> &[MemoryEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clock(&self) -> u64 {
        self.clock
    }

    pub(super) fn tick(&mut self) -> u64 {
        self.clock = self.clock.saturating_add(1);
        self.clock
    }

    pub(super) fn best_match_index(&self, key: &str, vector: &[f32]) -> Option<(usize, f32)> {
        let namespace = memory_namespace(key);
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| memory_namespace(&entry.key) == namespace)
            .filter(|(_, entry)| memory_keys_can_merge(key, &entry.key))
            .map(|(index, entry)| (index, cosine_similarity(vector, &entry.vector)))
            .max_by(|(_, left), (_, right)| left.partial_cmp(right).unwrap_or(Ordering::Equal))
    }

    pub(super) fn scoped_best_match_index(
        &self,
        scope: &TenantScope,
        key: &str,
        vector: &[f32],
    ) -> Option<(usize, f32)> {
        let namespace = memory_namespace(key);
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| memory_namespace(&entry.key) == namespace)
            .filter(|(_, entry)| memory_keys_can_merge(key, &entry.key))
            .filter(|(_, entry)| {
                memory_visible_to_scope(scope, &entry.key, TenantAccessKind::Write)
            })
            .map(|(index, entry)| (index, cosine_similarity(vector, &entry.vector)))
            .max_by(|(_, left), (_, right)| left.partial_cmp(right).unwrap_or(Ordering::Equal))
    }

    pub(super) fn entry_index(&self, id: u64) -> Option<usize> {
        self.entries.iter().position(|entry| entry.id == id)
    }

    pub(super) fn prune_if_needed(&mut self) {
        if self.entries.len() <= self.max_entries {
            return;
        }

        self.entries.sort_by(|a, b| {
            let left = memory_value_score(a, self.clock);
            let right = memory_value_score(b, self.clock);
            right.partial_cmp(&left).unwrap_or(Ordering::Equal)
        });
        self.entries.truncate(self.max_entries);
    }
}
