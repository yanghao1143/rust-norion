use super::cache::KvFusionCache;
use super::model::MemoryEntry;
use super::ops::{fuse_vector, merge_key};

impl KvFusionCache {
    pub fn store_or_fuse(
        &mut self,
        key: impl Into<String>,
        vector: Vec<f32>,
        usefulness: f32,
    ) -> u64 {
        let key = key.into();
        let usefulness = usefulness.clamp(0.05, 1.0);
        let now = self.tick();

        if let Some((index, score)) = self.best_match_index(&key, &vector)
            && score >= self.similarity_threshold
        {
            let entry = &mut self.entries[index];
            fuse_vector(&mut entry.vector, &vector, entry.strength, usefulness);
            entry.key = merge_key(&entry.key, &key);
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
