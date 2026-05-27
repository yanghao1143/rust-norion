use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::kv_cache::{MemoryEntry, MemoryMatch};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfiniMemoryScope {
    LocalWindow,
    GlobalMemory,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct InfiniMemoryItem {
    pub id: u64,
    pub key: String,
    pub scope: InfiniMemoryScope,
    pub score: f32,
    pub estimated_tokens: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InfiniMemoryCounts {
    pub local_window: usize,
    pub global_memory: usize,
    pub skipped: usize,
    pub local_tokens: usize,
    pub global_tokens: usize,
    pub skipped_tokens: usize,
}

#[derive(Debug, Clone, Default)]
pub struct InfiniMemoryPlan {
    local_window: Vec<InfiniMemoryItem>,
    global_memory: Vec<InfiniMemoryItem>,
    skipped: Vec<InfiniMemoryItem>,
}

impl InfiniMemoryPlan {
    pub fn new(
        local_window: Vec<InfiniMemoryItem>,
        global_memory: Vec<InfiniMemoryItem>,
        skipped: Vec<InfiniMemoryItem>,
    ) -> Self {
        Self {
            local_window,
            global_memory,
            skipped,
        }
    }

    pub fn local_window(&self) -> &[InfiniMemoryItem] {
        &self.local_window
    }

    pub fn global_memory(&self) -> &[InfiniMemoryItem] {
        &self.global_memory
    }

    pub fn skipped(&self) -> &[InfiniMemoryItem] {
        &self.skipped
    }

    pub fn counts(&self) -> InfiniMemoryCounts {
        InfiniMemoryCounts {
            local_window: self.local_window.len(),
            global_memory: self.global_memory.len(),
            skipped: self.skipped.len(),
            local_tokens: self
                .local_window
                .iter()
                .map(|item| item.estimated_tokens)
                .sum(),
            global_tokens: self
                .global_memory
                .iter()
                .map(|item| item.estimated_tokens)
                .sum(),
            skipped_tokens: self.skipped.iter().map(|item| item.estimated_tokens).sum(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InfiniMemoryPlanner {
    local_capacity: usize,
    global_capacity: usize,
    min_local_score: f32,
    min_global_score: f32,
    redundancy_threshold: f32,
    local_token_budget: usize,
    global_token_budget: usize,
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
            .map(|memory| local_item(memory))
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
                scope: InfiniMemoryScope::Skipped,
                score: memory.similarity * memory.strength,
                estimated_tokens: estimate_tokens(&memory.key),
                reason: "sparse_filter:missing_entry".to_owned(),
            });
        }

        InfiniMemoryPlan::new(local_window, global_memory, skipped)
    }
}

fn local_item(memory: &MemoryMatch) -> InfiniMemoryItem {
    let normalized_strength = (memory.strength / 3.0).clamp(0.0, 1.0);
    let score = (memory.similarity.clamp(0.0, 1.0) * 0.72) + normalized_strength * 0.28;

    InfiniMemoryItem {
        id: memory.id,
        key: memory.key.clone(),
        scope: InfiniMemoryScope::LocalWindow,
        score,
        estimated_tokens: estimate_tokens(&memory.key),
        reason: format!(
            "local_window:similarity={:.3}:strength={:.3}",
            memory.similarity, memory.strength
        ),
    }
}

fn global_item(entry: &MemoryEntry, max_access: u64) -> InfiniMemoryItem {
    let attempts = entry.hits + entry.failures;
    let reliability = if attempts == 0 {
        0.5
    } else {
        entry.hits as f32 / attempts as f32
    };
    let recency = if max_access == 0 {
        1.0
    } else {
        1.0 - (max_access.saturating_sub(entry.last_access) as f32 / max_access as f32).min(1.0)
    };
    let score = (entry.strength / 3.0).clamp(0.0, 1.0) * 0.50
        + entry.last_score.clamp(0.0, 1.0) * 0.25
        + reliability * 0.15
        + recency * 0.10;

    InfiniMemoryItem {
        id: entry.id,
        key: entry.key.clone(),
        scope: InfiniMemoryScope::GlobalMemory,
        score,
        estimated_tokens: estimate_tokens(&entry.key),
        reason: format!(
            "global_memory:strength={:.3}:last_score={:.3}:reliability={:.3}:recency={:.3}",
            entry.strength, entry.last_score, reliability, recency
        ),
    }
}

fn skipped_item(mut item: InfiniMemoryItem, reason: &str) -> InfiniMemoryItem {
    item.scope = InfiniMemoryScope::Skipped;
    item.reason = format!("{reason}:source_score={:.3}", item.score);
    item
}

fn sort_items(items: &mut [InfiniMemoryItem]) {
    items.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn is_redundant(candidate: &str, selected: &[String], threshold: f32) -> bool {
    selected
        .iter()
        .any(|key| token_overlap(candidate, key) >= threshold)
}

fn token_overlap(left: &str, right: &str) -> f32 {
    let left_tokens = tokenize_key(left);
    let right_tokens = tokenize_key(right);

    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }

    let intersection = left_tokens.intersection(&right_tokens).count() as f32;
    let union = left_tokens.union(&right_tokens).count() as f32;
    intersection / union.max(1.0)
}

fn tokenize_key(value: &str) -> HashSet<String> {
    value
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn estimate_tokens(value: &str) -> usize {
    let token_count = value
        .split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation())
        .filter(|part| !part.is_empty())
        .count();
    token_count.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_matches_fill_local_window() {
        let planner = InfiniMemoryPlanner::with_limits(1, 4);
        let entries = vec![entry(1, "rust router", 0.7), entry(2, "long memory", 1.3)];
        let matches = vec![memory_match(1, "rust router", 0.9, 0.7)];

        let plan = planner.plan(&entries, &matches);

        assert_eq!(plan.local_window().len(), 1);
        assert_eq!(plan.local_window()[0].id, 1);
        assert_eq!(plan.counts().local_window, 1);
    }

    #[test]
    fn strong_non_active_memory_goes_global() {
        let planner = InfiniMemoryPlanner::with_limits(1, 4);
        let entries = vec![
            entry(1, "current query", 0.8),
            entry(2, "durable global memory", 2.2),
        ];
        let matches = vec![memory_match(1, "current query", 0.9, 0.8)];

        let plan = planner.plan(&entries, &matches);

        assert!(plan.global_memory().iter().any(|item| item.id == 2));
    }

    #[test]
    fn weak_memory_is_sparse_skipped() {
        let planner = InfiniMemoryPlanner::with_limits(1, 4);
        let mut weak = entry(9, "weak stale memory", 0.02);
        weak.hits = 0;
        weak.failures = 3;
        weak.last_score = 0.0;
        weak.last_access = 0;
        let entries = vec![weak];
        let plan = planner.plan(&entries, &[]);

        assert!(plan.global_memory().is_empty());
        assert_eq!(plan.skipped()[0].id, 9);
    }

    #[test]
    fn redundant_global_memory_is_skipped() {
        let planner = InfiniMemoryPlanner {
            redundancy_threshold: 0.5,
            ..InfiniMemoryPlanner::with_limits(1, 4)
        };
        let entries = vec![
            entry(1, "rust router adaptive memory", 2.0),
            entry(2, "rust router adaptive memory copy", 1.9),
        ];
        let plan = planner.plan(&entries, &[]);

        assert_eq!(plan.global_memory().len(), 1);
        assert_eq!(plan.skipped().len(), 1);
        assert!(
            plan.skipped()
                .iter()
                .any(|item| item.reason.contains("redundant"))
        );
    }

    #[test]
    fn local_token_budget_skips_overflow() {
        let planner = InfiniMemoryPlanner::with_limits(4, 4).with_token_budgets(3, 32);
        let entries = vec![
            entry(1, "short local", 0.8),
            entry(2, "very long local memory key", 0.8),
        ];
        let matches = vec![
            memory_match(1, "short local", 0.9, 0.8),
            memory_match(2, "very long local memory key", 0.88, 0.8),
        ];

        let plan = planner.plan(&entries, &matches);

        assert_eq!(plan.counts().local_tokens, 2);
        assert!(
            plan.skipped()
                .iter()
                .any(|item| item.reason.contains("local_token_budget"))
        );
    }

    #[test]
    fn global_token_budget_skips_overflow() {
        let planner = InfiniMemoryPlanner::with_limits(1, 4).with_token_budgets(16, 3);
        let entries = vec![
            entry(1, "compact global", 2.2),
            entry(2, "very long durable global memory", 2.1),
        ];

        let plan = planner.plan(&entries, &[]);

        assert_eq!(plan.counts().global_tokens, 2);
        assert!(
            plan.skipped()
                .iter()
                .any(|item| item.reason.contains("global_token_budget"))
        );
    }

    fn entry(id: u64, key: &str, strength: f32) -> MemoryEntry {
        MemoryEntry {
            id,
            key: key.to_owned(),
            vector: vec![id as f32, strength],
            strength,
            hits: 2,
            failures: 0,
            last_score: 0.9,
            created_at: id,
            last_access: id + 1,
        }
    }

    fn memory_match(id: u64, key: &str, similarity: f32, strength: f32) -> MemoryMatch {
        MemoryMatch {
            id,
            key: key.to_owned(),
            similarity,
            strength,
            vector: vec![similarity, strength],
        }
    }
}
