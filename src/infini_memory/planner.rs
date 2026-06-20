use std::collections::{HashMap, HashSet};

use crate::kv_cache::{MemoryEntry, MemoryMatch};

use super::scoring::{
    estimate_tokens, global_item, is_redundant, local_item, skipped_item, sort_items,
};
use super::types::{InfiniMemoryItem, InfiniMemoryPlan, InfiniMemoryScope};

#[derive(Debug, Clone)]
pub struct InfiniMemoryPlanner {
    pub(super) local_capacity: usize,
    pub(super) global_capacity: usize,
    pub(super) min_local_score: f32,
    pub(super) min_global_score: f32,
    pub(super) redundancy_threshold: f32,
    pub(super) local_token_budget: usize,
    pub(super) global_token_budget: usize,
}

impl Default for InfiniMemoryPlanner {
    fn default() -> Self {
        Self {
            local_capacity: 4,
            global_capacity: 16,
            min_local_score: 0.08,
            min_global_score: 0.42,
            redundancy_threshold: 0.82,
            local_token_budget: 512,
            global_token_budget: 4096,
        }
    }
}

impl InfiniMemoryPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limits(local_capacity: usize, global_capacity: usize) -> Self {
        Self {
            local_capacity: local_capacity.max(1),
            global_capacity: global_capacity.max(1),
            ..Self::default()
        }
    }

    pub fn with_token_budgets(
        mut self,
        local_token_budget: usize,
        global_token_budget: usize,
    ) -> Self {
        self.local_token_budget = local_token_budget.max(1);
        self.global_token_budget = global_token_budget.max(1);
        self
    }

    pub fn plan(
        &self,
        entries: &[MemoryEntry],
        active_matches: &[MemoryMatch],
    ) -> InfiniMemoryPlan {
        let entries_by_id = entries
            .iter()
            .map(|entry| (entry.id, entry))
            .collect::<HashMap<_, _>>();
        let mut selected_ids = HashSet::new();
        let mut skipped_ids = HashSet::new();
        let mut selected_keys = Vec::new();

        let mut local_candidates = active_matches
            .iter()
            .map(local_item)
            .filter(|item| item.score >= self.min_local_score)
            .collect::<Vec<_>>();
        sort_items(&mut local_candidates);

        let mut local_window = Vec::new();
        let mut local_tokens = 0;
        let mut skipped = Vec::new();
        for item in local_candidates {
            if local_window.len() >= self.local_capacity {
                skipped_ids.insert(item.id);
                skipped.push(skipped_item(item, "sparse_filter:local_capacity"));
                continue;
            }
            if local_tokens + item.estimated_tokens > self.local_token_budget {
                skipped_ids.insert(item.id);
                skipped.push(skipped_item(item, "sparse_filter:local_token_budget"));
                continue;
            }

            local_tokens += item.estimated_tokens;
            selected_ids.insert(item.id);
            selected_keys.push(item.key.clone());
            local_window.push(item);
        }

        let max_access = entries
            .iter()
            .map(|entry| entry.last_access)
            .max()
            .unwrap_or(0);
        let mut global_candidates = entries
            .iter()
            .filter(|entry| !selected_ids.contains(&entry.id))
            .map(|entry| global_item(entry, max_access))
            .filter(|item| item.score >= self.min_global_score)
            .collect::<Vec<_>>();
        sort_items(&mut global_candidates);

        let mut global_memory = Vec::new();
        let mut global_tokens = 0;

        for item in global_candidates {
            if global_memory.len() >= self.global_capacity {
                skipped_ids.insert(item.id);
                skipped.push(skipped_item(item, "sparse_filter:global_capacity"));
                continue;
            }

            if global_tokens + item.estimated_tokens > self.global_token_budget {
                skipped_ids.insert(item.id);
                skipped.push(skipped_item(item, "sparse_filter:global_token_budget"));
                continue;
            }

            if is_redundant(&item.key, &selected_keys, self.redundancy_threshold) {
                skipped_ids.insert(item.id);
                skipped.push(skipped_item(item, "sparse_filter:redundant_key_overlap"));
                continue;
            }

            global_tokens += item.estimated_tokens;
            selected_ids.insert(item.id);
            selected_keys.push(item.key.clone());
            global_memory.push(item);
        }

        for entry in entries {
            if selected_ids.contains(&entry.id) || skipped_ids.contains(&entry.id) {
                continue;
            }

            let item = global_item(entry, max_access);
            skipped.push(skipped_item(item, "sparse_filter:low_score"));
        }

        for memory in active_matches {
            if selected_ids.contains(&memory.id)
                || skipped_ids.contains(&memory.id)
                || entries_by_id.contains_key(&memory.id)
            {
                continue;
            }

            skipped.push(InfiniMemoryItem {
                id: memory.id,
                key: memory.key.clone(),
                vector: memory.vector.clone(),
                scope: InfiniMemoryScope::Skipped,
                score: memory.similarity * memory.strength,
                estimated_tokens: estimate_tokens(&memory.key),
                reason: "sparse_filter:missing_entry".to_owned(),
            });
        }

        InfiniMemoryPlan::new(local_window, global_memory, skipped)
    }
}
