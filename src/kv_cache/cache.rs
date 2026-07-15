use std::cmp::Ordering;
use std::collections::HashMap;

use super::model::MemoryEntry;
use super::ops::{
    cosine_similarity, memory_keys_can_merge, memory_namespace, memory_value_score,
    memory_visible_to_scope,
};
use crate::tenant_scope::{TenantAccessKind, TenantScope};

#[derive(Debug)]
pub struct KvFusionCache {
    pub(super) entries: Vec<MemoryEntry>,
    pub(super) similarity_threshold: f32,
    pub(super) max_entries: usize,
    pub(super) next_id: u64,
    pub(super) clock: u64,
    rollback_journal: Option<KvRollbackJournal>,
}

impl Clone for KvFusionCache {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            similarity_threshold: self.similarity_threshold,
            max_entries: self.max_entries,
            next_id: self.next_id,
            clock: self.clock,
            rollback_journal: None,
        }
    }
}

impl Default for KvFusionCache {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            similarity_threshold: 0.78,
            max_entries: 4096,
            next_id: 1,
            clock: 0,
            rollback_journal: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct MemoryEntryMetadata {
    strength: f32,
    hits: u64,
    failures: u64,
    last_score: f32,
    last_access: u64,
}

impl MemoryEntryMetadata {
    fn from_entry(entry: &MemoryEntry) -> Self {
        Self {
            strength: entry.strength,
            hits: entry.hits,
            failures: entry.failures,
            last_score: entry.last_score,
            last_access: entry.last_access,
        }
    }

    fn restore(self, entry: &mut MemoryEntry) {
        entry.strength = self.strength;
        entry.hits = self.hits;
        entry.failures = self.failures;
        entry.last_score = self.last_score;
        entry.last_access = self.last_access;
    }
}

#[derive(Debug)]
struct KvRollbackJournal {
    original_order: Vec<u64>,
    similarity_threshold: f32,
    max_entries: usize,
    next_id: u64,
    clock: u64,
    metadata: HashMap<u64, MemoryEntryMetadata>,
    entries: HashMap<u64, MemoryEntry>,
}

impl KvRollbackJournal {
    fn new(cache: &KvFusionCache) -> Self {
        Self {
            original_order: cache.entries.iter().map(|entry| entry.id).collect(),
            similarity_threshold: cache.similarity_threshold,
            max_entries: cache.max_entries,
            next_id: cache.next_id,
            clock: cache.clock,
            metadata: HashMap::new(),
            entries: HashMap::new(),
        }
    }

    fn contains_original(&self, id: u64) -> bool {
        id < self.next_id
    }

    fn needs_metadata(&self, id: u64) -> bool {
        self.contains_original(id)
            && !self.metadata.contains_key(&id)
            && !self.entries.contains_key(&id)
    }

    fn needs_entry(&self, id: u64) -> bool {
        self.contains_original(id) && !self.entries.contains_key(&id)
    }

    fn capture_entry(&mut self, mut entry: MemoryEntry) {
        if !self.needs_entry(entry.id) {
            return;
        }
        if let Some(metadata) = self.metadata.remove(&entry.id) {
            metadata.restore(&mut entry);
        }
        self.entries.insert(entry.id, entry);
    }

    fn restore(mut self, cache: &mut KvFusionCache) {
        let mut current = cache
            .entries
            .drain(..)
            .map(|entry| (entry.id, entry))
            .collect::<HashMap<_, _>>();
        let mut restored = Vec::with_capacity(self.original_order.len());
        for id in self.original_order {
            let mut entry = self
                .entries
                .remove(&id)
                .or_else(|| current.remove(&id))
                .expect("request rollback journal must retain every original KV entry");
            if let Some(metadata) = self.metadata.remove(&id) {
                metadata.restore(&mut entry);
            }
            restored.push(entry);
        }
        cache.entries = restored;
        cache.similarity_threshold = self.similarity_threshold;
        cache.max_entries = self.max_entries;
        cache.next_id = self.next_id;
        cache.clock = self.clock;
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

    pub fn set_similarity_threshold(&mut self, similarity_threshold: f32) {
        self.similarity_threshold = similarity_threshold.clamp(0.1, 0.99);
    }

    pub fn entries(&self) -> &[MemoryEntry] {
        &self.entries
    }

    pub fn entries_scoped(&self, scope: &TenantScope) -> Vec<MemoryEntry> {
        self.entries
            .iter()
            .filter(|entry| memory_visible_to_scope(scope, &entry.key, TenantAccessKind::Read))
            .cloned()
            .collect()
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

    pub(crate) fn begin_request_rollback(&mut self) {
        assert!(
            self.rollback_journal.is_none(),
            "nested KV request rollback is not supported"
        );
        self.rollback_journal = Some(KvRollbackJournal::new(self));
    }

    pub(crate) fn commit_request_rollback(&mut self) {
        self.rollback_journal
            .take()
            .expect("KV request rollback must be active before commit");
    }

    pub(crate) fn request_rollback_active(&self) -> bool {
        self.rollback_journal.is_some()
    }

    pub(crate) fn rollback_request(&mut self) {
        let journal = self
            .rollback_journal
            .take()
            .expect("KV request rollback must be active before rollback");
        journal.restore(self);
    }

    pub(super) fn capture_entry_metadata_for_rollback(&mut self, id: u64) {
        let should_capture = self
            .rollback_journal
            .as_ref()
            .is_some_and(|journal| journal.needs_metadata(id));
        if !should_capture {
            return;
        }
        let Some(entry) = self.entries.iter().find(|entry| entry.id == id) else {
            return;
        };
        let metadata = MemoryEntryMetadata::from_entry(entry);
        self.rollback_journal
            .as_mut()
            .expect("rollback journal checked above")
            .metadata
            .insert(id, metadata);
    }

    pub(super) fn capture_entry_for_rollback(&mut self, id: u64) {
        let should_capture = self
            .rollback_journal
            .as_ref()
            .is_some_and(|journal| journal.needs_entry(id));
        if !should_capture {
            return;
        }
        let Some(entry) = self.entries.iter().find(|entry| entry.id == id).cloned() else {
            return;
        };
        self.rollback_journal
            .as_mut()
            .expect("rollback journal checked above")
            .capture_entry(entry);
    }

    pub(super) fn capture_removed_entry_for_rollback(&mut self, entry: MemoryEntry) {
        if let Some(journal) = self.rollback_journal.as_mut() {
            journal.capture_entry(entry);
        }
    }

    pub(super) fn remove_entries_with_rollback(&mut self, removed_ids: &[u64]) {
        if removed_ids.is_empty() {
            return;
        }
        let mut kept = Vec::with_capacity(self.entries.len().saturating_sub(removed_ids.len()));
        for entry in self.entries.drain(..) {
            if removed_ids.contains(&entry.id) {
                if let Some(journal) = self.rollback_journal.as_mut() {
                    journal.capture_entry(entry);
                }
            } else {
                kept.push(entry);
            }
        }
        self.entries = kept;
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
        let removed = self.entries.split_off(self.max_entries);
        for entry in removed {
            self.capture_removed_entry_for_rollback(entry);
        }
    }
}
