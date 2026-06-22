use std::cmp::Ordering;
use std::collections::HashSet;

use super::cache::KvFusionCache;
use super::model::{MemoryCompactionMerge, MemoryCompactionPolicy, MemoryCompactionReport};
use super::ops::{
    choose_compaction_pair, cosine_similarity, memory_namespace, memory_value_score,
    merge_memory_entry,
};

impl KvFusionCache {
    pub fn compact_similar(&mut self, policy: MemoryCompactionPolicy) -> MemoryCompactionReport {
        self.compact_similar_with_protected(policy, &[])
    }

    pub fn compact_similar_with_protected(
        &mut self,
        policy: MemoryCompactionPolicy,
        protected_ids: &[u64],
    ) -> MemoryCompactionReport {
        let before = self.entries.len();
        if before < 2 || policy.max_merges == 0 || policy.max_candidates < 2 {
            return MemoryCompactionReport::skipped(before);
        }

        let now = self.tick();
        let threshold = policy.similarity_threshold.clamp(0.10, 0.999);
        let protected = protected_ids.iter().copied().collect::<HashSet<_>>();
        let mut candidates = self
            .entries
            .iter()
            .map(|entry| (entry.id, memory_value_score(entry, now)))
            .collect::<Vec<_>>();

        candidates.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        candidates.truncate(policy.max_candidates.min(candidates.len()));

        let candidate_ids = candidates.into_iter().map(|(id, _)| id).collect::<Vec<_>>();
        let mut removed = HashSet::new();
        let mut merges = Vec::new();

        'outer: for left_pos in 0..candidate_ids.len() {
            for right_pos in (left_pos + 1)..candidate_ids.len() {
                if merges.len() >= policy.max_merges {
                    break 'outer;
                }

                let left_id = candidate_ids[left_pos];
                let right_id = candidate_ids[right_pos];
                if removed.contains(&left_id) || removed.contains(&right_id) {
                    continue;
                }

                let Some(left_index) = self.entry_index(left_id) else {
                    continue;
                };
                let Some(right_index) = self.entry_index(right_id) else {
                    continue;
                };
                if memory_namespace(&self.entries[left_index].key)
                    != memory_namespace(&self.entries[right_index].key)
                {
                    continue;
                }
                let similarity = cosine_similarity(
                    &self.entries[left_index].vector,
                    &self.entries[right_index].vector,
                );
                if similarity < threshold {
                    continue;
                }

                let Some((primary_id, removed_id)) = choose_compaction_pair(
                    &self.entries[left_index],
                    &self.entries[right_index],
                    &protected,
                    now,
                ) else {
                    continue;
                };
                let Some(primary_index) = self.entry_index(primary_id) else {
                    continue;
                };
                let Some(removed_index) = self.entry_index(removed_id) else {
                    continue;
                };

                let namespace = memory_namespace(&self.entries[primary_index].key).to_owned();
                let primary_vector_dimensions = self.entries[primary_index].vector.len();
                let removed_vector_dimensions = self.entries[removed_index].vector.len();
                let primary_protected = protected.contains(&primary_id);
                let removed_protected = protected.contains(&removed_id);
                let duplicate = self.entries[removed_index].clone();
                merge_memory_entry(
                    &mut self.entries[primary_index],
                    &duplicate,
                    similarity,
                    now,
                );
                removed.insert(removed_id);
                merges.push(MemoryCompactionMerge {
                    primary_id,
                    removed_id,
                    similarity,
                    namespace,
                    primary_vector_dimensions,
                    removed_vector_dimensions,
                    primary_protected,
                    removed_protected,
                });
            }
        }

        let mut removed_ids = removed.into_iter().collect::<Vec<_>>();
        removed_ids.sort_unstable();
        self.entries
            .retain(|entry| removed_ids.binary_search(&entry.id).is_err());

        MemoryCompactionReport {
            before,
            after: self.entries.len(),
            merged: merges,
            removed: removed_ids,
        }
    }
}
