use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{
    MemoryAdapter, MemoryAdapterCapability, MemoryAdapterDescriptor, MemoryAdapterHealth,
    MemoryResult, RetentionMemoryEntry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfiniMemoryScope {
    LocalWindow,
    GlobalMemory,
    Skipped,
}

impl InfiniMemoryScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalWindow => "local_window",
            Self::GlobalMemory => "global_memory",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InfiniMemoryActiveMatch {
    pub id: String,
    pub key: String,
    pub vector: Vec<f32>,
    pub similarity: f32,
    pub strength: f32,
}

impl InfiniMemoryActiveMatch {
    pub fn new(
        id: impl Into<String>,
        key: impl Into<String>,
        vector: Vec<f32>,
        similarity: f32,
        strength: f32,
    ) -> Self {
        Self {
            id: id.into(),
            key: key.into(),
            vector,
            similarity: similarity.clamp(0.0, 1.0),
            strength: strength.clamp(0.0, 3.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InfiniMemoryItem {
    pub id: String,
    pub key: String,
    pub vector: Vec<f32>,
    pub scope: InfiniMemoryScope,
    pub score: f32,
    pub estimated_tokens: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InfiniMemoryCounts {
    pub local_window: usize,
    pub global_memory: usize,
    pub skipped: usize,
    pub local_tokens: usize,
    pub global_tokens: usize,
    pub skipped_tokens: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InfiniMemoryPlan {
    pub local_window: Vec<InfiniMemoryItem>,
    pub global_memory: Vec<InfiniMemoryItem>,
    pub skipped: Vec<InfiniMemoryItem>,
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

    pub fn selected_ids(&self) -> Vec<String> {
        self.local_window
            .iter()
            .chain(self.global_memory.iter())
            .map(|item| item.id.clone())
            .collect()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        self.local_window
            .iter()
            .chain(self.global_memory.iter())
            .chain(self.skipped.iter())
            .filter_map(|item| reason_code(&item.reason))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.local_window
            .iter()
            .map(|item| format!("local_window:{}", hex_id(&item.id)))
            .chain(
                self.global_memory
                    .iter()
                    .map(|item| format!("global_memory:{}", hex_id(&item.id))),
            )
            .chain(self.skipped.iter().filter_map(|item| {
                reason_code(&item.reason)
                    .map(|reason| format!("skipped:{reason}:{}", hex_id(&item.id)))
            }))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes_for_scope(&self, scope: InfiniMemoryScope) -> Vec<String> {
        match scope {
            InfiniMemoryScope::LocalWindow => self
                .local_window
                .iter()
                .map(|item| format!("local_window:{}", hex_id(&item.id)))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            InfiniMemoryScope::GlobalMemory => self
                .global_memory
                .iter()
                .map(|item| format!("global_memory:{}", hex_id(&item.id)))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            InfiniMemoryScope::Skipped => self
                .skipped
                .iter()
                .filter_map(|item| {
                    reason_code(&item.reason)
                        .map(|reason| format!("skipped:{reason}:{}", hex_id(&item.id)))
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        }
    }

    pub fn skipped_detail_codes(&self) -> Vec<String> {
        self.detail_codes_for_scope(InfiniMemoryScope::Skipped)
    }

    pub fn skipped_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let Some(reason) = reason_code(reason) else {
            return Vec::new();
        };
        let prefix = format!("skipped:{reason}:");
        self.skipped_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn summary_line(&self) -> String {
        let counts = self.counts();
        format!(
            "infini_memory local_window={} global_memory={} skipped={} local_tokens={} global_tokens={} skipped_tokens={} selected={} reason_codes={} detail_codes={}",
            counts.local_window,
            counts.global_memory,
            counts.skipped,
            counts.local_tokens,
            counts.global_tokens,
            counts.skipped_tokens,
            self.selected_ids().len(),
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

pub trait InfiniMemoryPlanner {
    fn plan(
        &self,
        entries: &[RetentionMemoryEntry],
        active_matches: &[InfiniMemoryActiveMatch],
    ) -> InfiniMemoryPlan;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefaultInfiniMemoryPlanner {
    pub local_capacity: usize,
    pub global_capacity: usize,
    pub min_local_score: f32,
    pub min_global_score: f32,
    pub redundancy_threshold: f32,
    pub local_token_budget: usize,
    pub global_token_budget: usize,
}

impl Default for DefaultInfiniMemoryPlanner {
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

impl DefaultInfiniMemoryPlanner {
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
}

impl MemoryAdapter for DefaultInfiniMemoryPlanner {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_infini_memory_planner",
            vec![MemoryAdapterCapability::InfiniMemoryPlanning],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl InfiniMemoryPlanner for DefaultInfiniMemoryPlanner {
    fn plan(
        &self,
        entries: &[RetentionMemoryEntry],
        active_matches: &[InfiniMemoryActiveMatch],
    ) -> InfiniMemoryPlan {
        let entries_by_id = entries
            .iter()
            .map(|entry| (entry.id.clone(), entry))
            .collect::<HashMap<_, _>>();
        let mut selected_ids = BTreeSet::new();
        let mut skipped_ids = BTreeSet::new();
        let mut selected_keys = Vec::new();
        let mut skipped = Vec::new();

        let mut local_candidates = active_matches
            .iter()
            .map(local_item)
            .filter(|item| item.score >= self.min_local_score)
            .collect::<Vec<_>>();
        sort_items(&mut local_candidates);

        let mut local_window = Vec::new();
        let mut local_tokens = 0;
        for item in local_candidates {
            if local_window.len() >= self.local_capacity {
                skipped_ids.insert(item.id.clone());
                skipped.push(skipped_item(item, "sparse_filter:local_capacity"));
                continue;
            }
            if local_tokens + item.estimated_tokens > self.local_token_budget {
                skipped_ids.insert(item.id.clone());
                skipped.push(skipped_item(item, "sparse_filter:local_token_budget"));
                continue;
            }
            local_tokens += item.estimated_tokens;
            selected_ids.insert(item.id.clone());
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
                skipped_ids.insert(item.id.clone());
                skipped.push(skipped_item(item, "sparse_filter:global_capacity"));
                continue;
            }
            if global_tokens + item.estimated_tokens > self.global_token_budget {
                skipped_ids.insert(item.id.clone());
                skipped.push(skipped_item(item, "sparse_filter:global_token_budget"));
                continue;
            }
            if is_redundant(&item.key, &selected_keys, self.redundancy_threshold) {
                skipped_ids.insert(item.id.clone());
                skipped.push(skipped_item(item, "sparse_filter:redundant_key_overlap"));
                continue;
            }
            global_tokens += item.estimated_tokens;
            selected_ids.insert(item.id.clone());
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
                id: memory.id.clone(),
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

fn local_item(memory: &InfiniMemoryActiveMatch) -> InfiniMemoryItem {
    let normalized_strength = (memory.strength / 3.0).clamp(0.0, 1.0);
    let score = (memory.similarity.clamp(0.0, 1.0) * 0.72) + normalized_strength * 0.28;
    InfiniMemoryItem {
        id: memory.id.clone(),
        key: memory.key.clone(),
        vector: memory.vector.clone(),
        scope: InfiniMemoryScope::LocalWindow,
        score,
        estimated_tokens: estimate_tokens(&memory.key),
        reason: format!(
            "local_window:similarity={:.3}:strength={:.3}",
            memory.similarity, memory.strength
        ),
    }
}

fn global_item(entry: &RetentionMemoryEntry, max_access: u64) -> InfiniMemoryItem {
    let attempts = entry.hits.saturating_add(entry.failures);
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
        + reliability * 0.15
        + recency * 0.10
        + value_hint_score(entry) * 0.25;
    InfiniMemoryItem {
        id: entry.id.clone(),
        key: entry.key.clone(),
        vector: entry.vector.clone(),
        scope: InfiniMemoryScope::GlobalMemory,
        score,
        estimated_tokens: estimate_tokens(&entry.key),
        reason: format!(
            "global_memory:strength={:.3}:reliability={:.3}:recency={:.3}",
            entry.strength, reliability, recency
        ),
    }
}

fn value_hint_score(entry: &RetentionMemoryEntry) -> f32 {
    (entry.hits as f32 * 0.08 - entry.failures as f32 * 0.12 + entry.strength / 3.0).clamp(0.0, 1.0)
}

fn skipped_item(mut item: InfiniMemoryItem, reason: &str) -> InfiniMemoryItem {
    item.scope = InfiniMemoryScope::Skipped;
    item.reason = format!("{reason}:source_score={:.3}", item.score);
    item
}

fn reason_code(reason: &str) -> Option<String> {
    if reason.is_empty() {
        return None;
    }
    if reason.starts_with("local_window:") {
        return Some("local_window".to_owned());
    }
    if reason.starts_with("global_memory:") {
        return Some("global_memory".to_owned());
    }
    if reason.starts_with("sparse_filter:") {
        let mut parts = reason.split(':');
        let family = parts.next()?;
        let code = parts.next()?;
        return Some(format!("{family}:{code}"));
    }
    Some(
        reason
            .split_once('=')
            .map_or(reason, |(code, _)| code)
            .to_owned(),
    )
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn hex_id(id: &str) -> String {
    id.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn sort_items(items: &mut [InfiniMemoryItem]) {
    items.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
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
    value
        .split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation())
        .filter(|part| !part.is_empty())
        .count()
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        id: &str,
        key: &str,
        strength: f32,
        hits: u64,
        failures: u64,
        last_access: u64,
    ) -> RetentionMemoryEntry {
        RetentionMemoryEntry::new(id, key, vec![0.1, 0.2], strength)
            .with_feedback(hits, failures)
            .with_access(1, last_access)
    }

    #[test]
    fn infini_planner_selects_local_and_global_memory() {
        let entries = vec![
            entry("global", "semantic:durable lesson", 2.4, 4, 0, 10),
            entry("weak", "semantic:weak lesson", 0.1, 0, 3, 1),
        ];
        let active = vec![InfiniMemoryActiveMatch::new(
            "local",
            "semantic:active prompt",
            vec![0.3, 0.4],
            0.9,
            1.5,
        )];

        let plan = DefaultInfiniMemoryPlanner::with_limits(2, 2).plan(&entries, &active);
        let counts = plan.counts();

        assert_eq!(counts.local_window, 1);
        assert_eq!(counts.global_memory, 1);
        assert_eq!(plan.local_window[0].id, "local");
        assert_eq!(plan.global_memory[0].id, "global");
        assert!(plan.skipped.iter().any(|item| item.id == "weak"));
        assert_eq!(
            plan.summary_line(),
            "infini_memory local_window=1 global_memory=1 skipped=1 local_tokens=3 global_tokens=3 skipped_tokens=3 selected=2 reason_codes=global_memory|local_window|sparse_filter:low_score detail_codes=global_memory:676c6f62616c|local_window:6c6f63616c|skipped:sparse_filter:low_score:7765616b"
        );
        assert_eq!(
            plan.reason_codes(),
            vec![
                "global_memory".to_owned(),
                "local_window".to_owned(),
                "sparse_filter:low_score".to_owned()
            ]
        );
        assert_eq!(
            plan.detail_codes(),
            vec![
                "global_memory:676c6f62616c".to_owned(),
                "local_window:6c6f63616c".to_owned(),
                "skipped:sparse_filter:low_score:7765616b".to_owned(),
            ]
        );
        assert_eq!(
            plan.detail_codes_for_scope(InfiniMemoryScope::LocalWindow),
            vec!["local_window:6c6f63616c".to_owned()]
        );
        assert_eq!(
            plan.detail_codes_for_scope(InfiniMemoryScope::GlobalMemory),
            vec!["global_memory:676c6f62616c".to_owned()]
        );
        assert_eq!(
            plan.skipped_detail_codes(),
            vec!["skipped:sparse_filter:low_score:7765616b".to_owned()]
        );
        assert_eq!(
            plan.skipped_detail_codes_for_reason("sparse_filter:low_score"),
            vec!["skipped:sparse_filter:low_score:7765616b".to_owned()]
        );
        assert_eq!(
            plan.skipped_detail_codes_for_reason("sparse_filter:missing_entry"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn infini_evidence_uses_hex_ids_without_key_payloads() {
        let local_key_secret = "INFECTED_ACTIVE_KEY_DO_NOT_LOG";
        let global_key_secret = "INFECTED_GLOBAL_KEY_DO_NOT_LOG";
        let entries = vec![entry("global-clean-id", global_key_secret, 2.4, 4, 0, 10)];
        let active = vec![InfiniMemoryActiveMatch::new(
            "local-clean-id",
            local_key_secret,
            vec![0.3, 0.4],
            0.9,
            1.5,
        )];

        let plan = DefaultInfiniMemoryPlanner::with_limits(2, 2).plan(&entries, &active);
        let summary_line = plan.summary_line();
        let detail_codes = plan.detail_codes();

        assert_eq!(plan.counts().local_window, 1);
        assert_eq!(plan.counts().global_memory, 1);
        assert!(summary_line.contains("local_window:6c6f63616c2d636c65616e2d6964"));
        assert!(summary_line.contains("global_memory:676c6f62616c2d636c65616e2d6964"));
        assert!(detail_codes.contains(&"local_window:6c6f63616c2d636c65616e2d6964".to_owned()));
        assert!(detail_codes.contains(&"global_memory:676c6f62616c2d636c65616e2d6964".to_owned()));
        for forbidden in [local_key_secret, global_key_secret] {
            assert!(
                !summary_line.contains(forbidden),
                "infini summary leaked key payload: {forbidden}"
            );
            assert!(
                !detail_codes.iter().any(|code| code.contains(forbidden)),
                "infini detail codes leaked key payload: {forbidden}"
            );
        }
    }

    #[test]
    fn infini_planner_respects_capacity_and_token_budgets() {
        let entries = vec![
            entry("global-a", "semantic:a b c d e", 2.4, 4, 0, 10),
            entry("global-b", "semantic:f g h i j", 2.3, 4, 0, 9),
        ];
        let active = vec![
            InfiniMemoryActiveMatch::new("local-a", "a b c", vec![0.1], 0.9, 1.0),
            InfiniMemoryActiveMatch::new("local-b", "d e f", vec![0.1], 0.8, 1.0),
        ];
        let planner = DefaultInfiniMemoryPlanner::with_limits(1, 1).with_token_budgets(8, 8);

        let plan = planner.plan(&entries, &active);

        assert_eq!(plan.counts().local_window, 1);
        assert_eq!(plan.counts().global_memory, 1);
        assert!(
            plan.skipped
                .iter()
                .any(|item| item.reason.starts_with("sparse_filter:local_capacity"))
        );
        assert!(
            plan.skipped
                .iter()
                .any(|item| item.reason.starts_with("sparse_filter:global_capacity"))
        );
        assert_eq!(
            plan.reason_codes(),
            vec![
                "global_memory".to_owned(),
                "local_window".to_owned(),
                "sparse_filter:global_capacity".to_owned(),
                "sparse_filter:local_capacity".to_owned()
            ]
        );
    }

    #[test]
    fn infini_planner_skips_redundant_global_keys() {
        let entries = vec![
            entry("same", "semantic active prompt", 2.4, 4, 0, 10),
            entry("different", "semantic durable other", 2.3, 4, 0, 9),
        ];
        let active = vec![InfiniMemoryActiveMatch::new(
            "local",
            "semantic active prompt",
            vec![0.3],
            0.9,
            1.5,
        )];

        let plan = DefaultInfiniMemoryPlanner::with_limits(2, 4).plan(&entries, &active);

        assert!(plan.skipped.iter().any(|item| {
            item.id == "same"
                && item
                    .reason
                    .starts_with("sparse_filter:redundant_key_overlap")
        }));
        assert!(plan.global_memory.iter().any(|item| item.id == "different"));
        assert!(
            plan.reason_codes()
                .contains(&"sparse_filter:redundant_key_overlap".to_owned())
        );
    }

    #[test]
    fn infini_planner_marks_active_match_without_entry_as_missing() {
        let active = vec![InfiniMemoryActiveMatch::new(
            "missing",
            "semantic missing",
            vec![0.3],
            0.01,
            0.1,
        )];

        let plan = DefaultInfiniMemoryPlanner::with_limits(1, 1).plan(&[], &active);

        assert_eq!(plan.local_window.len(), 0);
        assert_eq!(plan.skipped[0].id, "missing");
        assert_eq!(plan.skipped[0].reason, "sparse_filter:missing_entry");
        assert_eq!(
            plan.summary_line(),
            "infini_memory local_window=0 global_memory=0 skipped=1 local_tokens=0 global_tokens=0 skipped_tokens=2 selected=0 reason_codes=sparse_filter:missing_entry detail_codes=skipped:sparse_filter:missing_entry:6d697373696e67"
        );
    }

    #[test]
    fn infini_planner_reports_read_only_adapter_capability() {
        let planner = DefaultInfiniMemoryPlanner::default();
        let descriptor = planner.descriptor();

        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::InfiniMemoryPlanning)
        );
        assert!(planner.health().unwrap().ready);
    }
}
