use super::cache::KvFusionCache;
use super::model::MemoryEntry;
use super::ops::{fuse_vector, merge_key, scoped_memory_key};
use crate::gist_memory::GistRecord;
use crate::tenant_scope::{TenantResourceLane, TenantScope};

impl KvFusionCache {
    pub fn store_gist_memory(&mut self, record: &GistRecord, vector: Vec<f32>) -> u64 {
        self.store_or_fuse(record.gist_memory_key(), vector, record.importance)
    }

    pub fn store_or_fuse(
        &mut self,
        key: impl Into<String>,
        vector: Vec<f32>,
        usefulness: f32,
    ) -> u64 {
        self.store_key_or_fuse(key.into(), vector, usefulness, None)
    }

    pub fn store_scoped_or_fuse(
        &mut self,
        scope: &TenantScope,
        lane: TenantResourceLane,
        local_key: impl AsRef<str>,
        vector: Vec<f32>,
        usefulness: f32,
    ) -> u64 {
        let key = scoped_memory_key(scope, lane, local_key.as_ref());
        self.store_key_or_fuse(key, vector, usefulness, Some(scope))
    }

    fn store_key_or_fuse(
        &mut self,
        key: String,
        vector: Vec<f32>,
        usefulness: f32,
        scope: Option<&TenantScope>,
    ) -> u64 {
        let usefulness = usefulness.clamp(0.05, 1.0);
        let now = self.tick();
        let best_match = match scope {
            Some(scope) => self.scoped_best_match_index(scope, &key, &vector),
            None => self.best_match_index(&key, &vector),
        };

        if let Some((index, score)) = best_match
            && score >= self.similarity_threshold
        {
            let id = self.entries[index].id;
            self.capture_entry_for_rollback(id);
            let entry = &mut self.entries[index];
            fuse_vector(&mut entry.vector, &vector, entry.strength, usefulness);
            if scope.is_none() {
                entry.key = merge_key(&entry.key, &key);
            }
            entry.strength = (entry.strength + usefulness * 0.28).clamp(0.01, 3.0);
            entry.hits += 1;
            entry.last_score = score;
            entry.last_access = now;
            return entry.id;
        }

        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(MemoryEntry {
            id,
            key,
            vector,
            strength: usefulness.max(0.2),
            hits: 0,
            failures: 0,
            last_score: 1.0,
            created_at: now,
            last_access: now,
        });
        self.prune_if_needed();
        id
    }
}
