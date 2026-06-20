use std::collections::BTreeSet;

use crate::{
    MemoryAdapter, MemoryAdapterCapability, MemoryAdapterDescriptor, MemoryAdapterHealth,
    MemoryResult,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RetentionMemoryEntry {
    pub id: String,
    pub key: String,
    pub vector: Vec<f32>,
    pub strength: f32,
    pub hits: u64,
    pub failures: u64,
    pub created_at: u64,
    pub last_access: u64,
}

impl RetentionMemoryEntry {
    pub fn new(
        id: impl Into<String>,
        key: impl Into<String>,
        vector: Vec<f32>,
        strength: f32,
    ) -> Self {
        Self {
            id: id.into(),
            key: key.into(),
            vector,
            strength: strength.clamp(0.0, 3.0),
            hits: 0,
            failures: 0,
            created_at: 0,
            last_access: 0,
        }
    }

    pub fn with_access(mut self, created_at: u64, last_access: u64) -> Self {
        self.created_at = created_at;
        self.last_access = last_access;
        self
    }

    pub fn with_feedback(mut self, hits: u64, failures: u64) -> Self {
        self.hits = hits;
        self.failures = failures;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryRetentionPolicy {
    pub stale_after: u64,
    pub decay_rate: f32,
    pub remove_below_strength: f32,
    pub remove_after_failures: u64,
}

impl Default for MemoryRetentionPolicy {
    fn default() -> Self {
        Self {
            stale_after: 64,
            decay_rate: 0.04,
            remove_below_strength: 0.04,
            remove_after_failures: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryDecay {
    pub id: String,
    pub idle_ticks: u64,
    pub strength_before: f32,
    pub strength_after: f32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRetentionRemoval {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemoryRetentionPlan {
    pub before: usize,
    pub after_estimate: usize,
    pub decays: Vec<MemoryDecay>,
    pub removals: Vec<MemoryRetentionRemoval>,
}

impl MemoryRetentionPlan {
    pub fn is_empty(&self) -> bool {
        self.decays.is_empty() && self.removals.is_empty()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        self.decays
            .iter()
            .map(|decay| decay.reason.as_str())
            .chain(self.removals.iter().map(|removal| removal.reason.as_str()))
            .filter(|reason| !reason.is_empty())
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.decays
            .iter()
            .filter(|decay| !decay.reason.is_empty())
            .map(|decay| format!("decay:{}:{}", decay.reason, hex_id(&decay.id)))
            .chain(
                self.removals
                    .iter()
                    .filter(|removal| !removal.reason.is_empty())
                    .map(|removal| format!("remove:{}:{}", removal.reason, hex_id(&removal.id))),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_retention before={} after_estimate={} decays={} removals={} empty={} reason_codes={} detail_codes={}",
            self.before,
            self.after_estimate,
            self.decays.len(),
            self.removals.len(),
            self.is_empty(),
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }

    pub fn removed_ids(&self) -> Vec<String> {
        self.removals
            .iter()
            .map(|removal| removal.id.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryCompactionPolicy {
    pub similarity_threshold: f32,
    pub max_candidates: usize,
    pub max_merges: usize,
}

impl Default for MemoryCompactionPolicy {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.92,
            max_candidates: 512,
            max_merges: 32,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryCompactionMerge {
    pub primary_id: String,
    pub removed_id: String,
    pub similarity: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemoryCompactionPlan {
    pub before: usize,
    pub after_estimate: usize,
    pub merges: Vec<MemoryCompactionMerge>,
    pub removed_ids: Vec<String>,
    pub skipped_reason: Option<String>,
}

impl MemoryCompactionPlan {
    pub fn skipped(current_len: usize, reason: impl Into<String>) -> Self {
        Self {
            before: current_len,
            after_estimate: current_len,
            skipped_reason: Some(reason.into()),
            ..Self::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.merges.is_empty() && self.removed_ids.is_empty()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        self.merges
            .iter()
            .map(|merge| merge.reason.as_str())
            .chain(self.skipped_reason.as_deref())
            .filter(|reason| !reason.is_empty())
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.merges
            .iter()
            .filter(|merge| !merge.reason.is_empty())
            .map(|merge| {
                format!(
                    "merge:{}:{}:{}",
                    merge.reason,
                    hex_id(&merge.primary_id),
                    hex_id(&merge.removed_id)
                )
            })
            .chain(
                self.removed_ids
                    .iter()
                    .map(|removed_id| format!("remove:{}", hex_id(removed_id))),
            )
            .chain(
                self.skipped_reason
                    .iter()
                    .filter(|reason| !reason.is_empty())
                    .map(|reason| format!("skipped:{reason}")),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_compaction before={} after_estimate={} merges={} removals={} skipped={} empty={} reason_codes={} detail_codes={}",
            self.before,
            self.after_estimate,
            self.merges.len(),
            self.removed_ids.len(),
            self.skipped_reason.as_deref().unwrap_or("none"),
            self.is_empty(),
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
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

pub trait MemoryRetentionPlanner {
    fn plan_retention(
        &self,
        entries: &[RetentionMemoryEntry],
        now: u64,
        policy: MemoryRetentionPolicy,
    ) -> MemoryRetentionPlan;
}

pub trait MemoryCompactionPlanner {
    fn plan_compaction(
        &self,
        entries: &[RetentionMemoryEntry],
        protected_ids: &[String],
        now: u64,
        policy: MemoryCompactionPolicy,
    ) -> MemoryCompactionPlan;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultMemoryRetentionPlanner;

impl MemoryAdapter for DefaultMemoryRetentionPlanner {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_memory_retention_planner",
            vec![
                MemoryAdapterCapability::RetentionPlanning,
                MemoryAdapterCapability::CompactionPlanning,
            ],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl MemoryRetentionPlanner for DefaultMemoryRetentionPlanner {
    fn plan_retention(
        &self,
        entries: &[RetentionMemoryEntry],
        now: u64,
        policy: MemoryRetentionPolicy,
    ) -> MemoryRetentionPlan {
        let stale_after = policy.stale_after.max(1);
        let decay_rate = policy.decay_rate.clamp(0.0, 0.95);
        let mut decays = Vec::new();
        let mut removals = Vec::new();

        for entry in entries {
            let idle = now.saturating_sub(entry.last_access);
            let strength_after = if idle > policy.stale_after {
                let periods = (idle - policy.stale_after) as f32 / stale_after as f32;
                let decay = (decay_rate * periods.max(1.0)).clamp(0.0, 0.95);
                let strength_after = (entry.strength * (1.0 - decay)).clamp(0.0, 3.0);
                if strength_after < entry.strength {
                    decays.push(MemoryDecay {
                        id: entry.id.clone(),
                        idle_ticks: idle,
                        strength_before: entry.strength,
                        strength_after,
                        reason: "stale_decay".to_owned(),
                    });
                }
                strength_after
            } else {
                entry.strength
            };

            let weak_and_stale = strength_after <= policy.remove_below_strength
                && idle > policy.stale_after
                && entry.failures >= entry.hits;
            let repeatedly_failed =
                entry.failures >= policy.remove_after_failures && entry.hits == 0;
            let reason = match (weak_and_stale, repeatedly_failed) {
                (true, true) => Some("weak_stale_and_repeated_failures"),
                (true, false) => Some("weak_stale"),
                (false, true) => Some("repeated_failures"),
                (false, false) => None,
            };
            if let Some(reason) = reason {
                removals.push(MemoryRetentionRemoval {
                    id: entry.id.clone(),
                    reason: reason.to_owned(),
                });
            }
        }

        MemoryRetentionPlan {
            before: entries.len(),
            after_estimate: entries.len().saturating_sub(removals.len()),
            decays,
            removals,
        }
    }
}

impl MemoryCompactionPlanner for DefaultMemoryRetentionPlanner {
    fn plan_compaction(
        &self,
        entries: &[RetentionMemoryEntry],
        protected_ids: &[String],
        now: u64,
        policy: MemoryCompactionPolicy,
    ) -> MemoryCompactionPlan {
        if entries.len() < 2 {
            return MemoryCompactionPlan::skipped(entries.len(), "not_enough_entries");
        }
        if policy.max_merges == 0 || policy.max_candidates < 2 {
            return MemoryCompactionPlan::skipped(entries.len(), "policy_disabled");
        }

        let threshold = policy.similarity_threshold.clamp(0.10, 0.999);
        let protected = protected_ids.iter().cloned().collect::<BTreeSet<_>>();
        let mut candidates = entries
            .iter()
            .map(|entry| (entry.id.clone(), memory_value_score(entry, now)))
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        candidates.truncate(policy.max_candidates.min(candidates.len()));
        let candidate_ids = candidates.into_iter().map(|(id, _)| id).collect::<Vec<_>>();
        let mut removed = BTreeSet::new();
        let mut merges = Vec::new();

        'outer: for left_pos in 0..candidate_ids.len() {
            for right_pos in (left_pos + 1)..candidate_ids.len() {
                if merges.len() >= policy.max_merges {
                    break 'outer;
                }
                let left_id = &candidate_ids[left_pos];
                let right_id = &candidate_ids[right_pos];
                if removed.contains(left_id) || removed.contains(right_id) {
                    continue;
                }
                let Some(left) = entries.iter().find(|entry| &entry.id == left_id) else {
                    continue;
                };
                let Some(right) = entries.iter().find(|entry| &entry.id == right_id) else {
                    continue;
                };
                if memory_namespace(&left.key) != memory_namespace(&right.key) {
                    continue;
                }
                let similarity = cosine_similarity(&left.vector, &right.vector);
                if similarity < threshold {
                    continue;
                }
                let Some((primary_id, removed_id)) =
                    choose_compaction_pair(left, right, &protected, now)
                else {
                    continue;
                };
                removed.insert(removed_id.clone());
                merges.push(MemoryCompactionMerge {
                    primary_id,
                    removed_id,
                    similarity,
                    reason: "same_namespace_high_similarity".to_owned(),
                });
            }
        }

        let removed_ids = removed.into_iter().collect::<Vec<_>>();
        MemoryCompactionPlan {
            before: entries.len(),
            after_estimate: entries.len().saturating_sub(removed_ids.len()),
            merges,
            removed_ids,
            skipped_reason: None,
        }
    }
}

fn choose_compaction_pair(
    left: &RetentionMemoryEntry,
    right: &RetentionMemoryEntry,
    protected: &BTreeSet<String>,
    now: u64,
) -> Option<(String, String)> {
    let left_protected = protected.contains(&left.id);
    let right_protected = protected.contains(&right.id);

    match (left_protected, right_protected) {
        (true, true) => None,
        (true, false) => Some((left.id.clone(), right.id.clone())),
        (false, true) => Some((right.id.clone(), left.id.clone())),
        (false, false) => {
            let left_score = memory_value_score(left, now);
            let right_score = memory_value_score(right, now);
            if left_score > right_score
                || ((left_score - right_score).abs() < 0.0001 && left.id < right.id)
            {
                Some((left.id.clone(), right.id.clone()))
            } else {
                Some((right.id.clone(), left.id.clone()))
            }
        }
    }
}

fn memory_namespace(key: &str) -> &'static str {
    if key.starts_with("runtime_kv:") {
        "runtime_kv"
    } else if key.starts_with("gist:") {
        "gist"
    } else {
        "semantic"
    }
}

fn memory_value_score(entry: &RetentionMemoryEntry, now: u64) -> f32 {
    let idle = now.saturating_sub(entry.last_access) as f32;
    let idle_drag = (idle / 256.0).min(0.35);
    entry.strength - entry.failures as f32 * 0.08 + entry.hits as f32 * 0.02 - idle_drag
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let len = left.len().max(right.len());
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    for index in 0..len {
        let l = left.get(index).copied().unwrap_or(0.0);
        let r = right.get(index).copied().unwrap_or(0.0);
        dot += l * r;
        left_norm += l * l;
        right_norm += r * r;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        let raw = (dot / (left_norm.sqrt() * right_norm.sqrt())).clamp(-1.0, 1.0);
        (raw * dimension_compatibility(left, right)).clamp(-1.0, 1.0)
    }
}

fn dimension_compatibility(left: &[f32], right: &[f32]) -> f32 {
    if left.len() == right.len() {
        return 1.0;
    }

    let shorter = left.len().min(right.len()) as f32;
    let longer = left.len().max(right.len()) as f32;
    if shorter == 0.0 || longer == 0.0 {
        0.0
    } else {
        (shorter / longer).powi(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_plan_decays_stale_memory_without_mutating_entries() {
        let planner = DefaultMemoryRetentionPlanner;
        let entries = vec![
            RetentionMemoryEntry::new("stale", "semantic:lesson", vec![0.3, 0.4], 0.8)
                .with_feedback(3, 0)
                .with_access(1, 1),
        ];

        let plan = planner.plan_retention(
            &entries,
            20,
            MemoryRetentionPolicy {
                stale_after: 4,
                decay_rate: 0.20,
                remove_below_strength: 0.01,
                remove_after_failures: 8,
            },
        );

        assert_eq!(plan.before, 1);
        assert_eq!(plan.after_estimate, 1);
        assert_eq!(plan.decays.len(), 1);
        assert_eq!(plan.decays[0].id, "stale");
        assert!(plan.decays[0].strength_after < 0.8);
        assert!(plan.removals.is_empty());
        assert_eq!(
            plan.summary_line(),
            "memory_retention before=1 after_estimate=1 decays=1 removals=0 empty=false reason_codes=stale_decay detail_codes=decay:stale_decay:7374616c65"
        );
        assert_eq!(plan.reason_codes(), vec!["stale_decay".to_owned()]);
        assert_eq!(
            plan.detail_codes(),
            vec!["decay:stale_decay:7374616c65".to_owned()]
        );
        assert_eq!(entries[0].strength, 0.8);
    }

    #[test]
    fn retention_plan_removes_weak_stale_failed_memory() {
        let planner = DefaultMemoryRetentionPlanner;
        let entries = vec![
            RetentionMemoryEntry::new("failed", "semantic:bad", vec![0.1, 0.2], 0.02)
                .with_feedback(0, 4)
                .with_access(1, 1),
        ];

        let plan = planner.plan_retention(
            &entries,
            20,
            MemoryRetentionPolicy {
                stale_after: 4,
                decay_rate: 0.10,
                remove_below_strength: 0.04,
                remove_after_failures: 4,
            },
        );

        assert_eq!(plan.after_estimate, 0);
        assert_eq!(plan.removed_ids(), vec!["failed".to_owned()]);
        assert_eq!(plan.removals[0].reason, "weak_stale_and_repeated_failures");
        assert_eq!(
            plan.reason_codes(),
            vec![
                "stale_decay".to_owned(),
                "weak_stale_and_repeated_failures".to_owned()
            ]
        );
        assert_eq!(
            plan.detail_codes(),
            vec![
                "decay:stale_decay:6661696c6564".to_owned(),
                "remove:weak_stale_and_repeated_failures:6661696c6564".to_owned()
            ]
        );
    }

    #[test]
    fn compaction_plan_merges_same_namespace_similar_entries() {
        let planner = DefaultMemoryRetentionPlanner;
        let strong = RetentionMemoryEntry::new("strong", "runtime_kv:route", vec![1.0, 0.0], 0.9)
            .with_feedback(2, 0)
            .with_access(1, 9);
        let weak =
            RetentionMemoryEntry::new("weak", "runtime_kv:route copy", vec![0.99, 0.01], 0.2)
                .with_access(1, 8);
        let unrelated = RetentionMemoryEntry::new("gist", "gist:summary", vec![1.0, 0.0], 0.8)
            .with_access(1, 9);
        let entries = vec![weak, unrelated, strong];

        let plan = planner.plan_compaction(
            &entries,
            &[],
            10,
            MemoryCompactionPolicy {
                similarity_threshold: 0.92,
                max_candidates: 8,
                max_merges: 4,
            },
        );

        assert_eq!(plan.before, 3);
        assert_eq!(plan.after_estimate, 2);
        assert_eq!(plan.merges.len(), 1);
        assert_eq!(plan.merges[0].primary_id, "strong");
        assert_eq!(plan.merges[0].removed_id, "weak");
        assert_eq!(plan.removed_ids, vec!["weak".to_owned()]);
        assert_eq!(
            plan.summary_line(),
            "memory_compaction before=3 after_estimate=2 merges=1 removals=1 skipped=none empty=false reason_codes=same_namespace_high_similarity detail_codes=merge:same_namespace_high_similarity:7374726f6e67:7765616b|remove:7765616b"
        );
        assert_eq!(
            plan.reason_codes(),
            vec!["same_namespace_high_similarity".to_owned()]
        );
        assert_eq!(
            plan.detail_codes(),
            vec![
                "merge:same_namespace_high_similarity:7374726f6e67:7765616b".to_owned(),
                "remove:7765616b".to_owned()
            ]
        );
    }

    #[test]
    fn compaction_plan_respects_protected_ids() {
        let planner = DefaultMemoryRetentionPlanner;
        let protected = RetentionMemoryEntry::new("protected", "semantic:lesson", vec![1.0], 0.1)
            .with_access(1, 1);
        let stronger =
            RetentionMemoryEntry::new("stronger", "semantic:lesson copy", vec![1.0], 2.0)
                .with_access(1, 9);
        let protected_ids = vec!["protected".to_owned()];

        let plan = planner.plan_compaction(
            &[protected, stronger],
            &protected_ids,
            10,
            MemoryCompactionPolicy::default(),
        );

        assert_eq!(plan.merges[0].primary_id, "protected");
        assert_eq!(plan.merges[0].removed_id, "stronger");
    }

    #[test]
    fn compaction_skips_cross_namespace_and_disabled_policies() {
        let planner = DefaultMemoryRetentionPlanner;
        let entries = vec![
            RetentionMemoryEntry::new("runtime", "runtime_kv:item", vec![1.0], 1.0),
            RetentionMemoryEntry::new("gist", "gist:item", vec![1.0], 1.0),
        ];

        let cross_namespace =
            planner.plan_compaction(&entries, &[], 1, MemoryCompactionPolicy::default());
        assert!(cross_namespace.is_empty());
        assert_eq!(cross_namespace.after_estimate, 2);

        let disabled = planner.plan_compaction(
            &entries,
            &[],
            1,
            MemoryCompactionPolicy {
                similarity_threshold: 0.92,
                max_candidates: 1,
                max_merges: 4,
            },
        );
        assert_eq!(disabled.skipped_reason.as_deref(), Some("policy_disabled"));
        assert_eq!(disabled.reason_codes(), vec!["policy_disabled".to_owned()]);
        assert_eq!(
            disabled.summary_line(),
            "memory_compaction before=2 after_estimate=2 merges=0 removals=0 skipped=policy_disabled empty=true reason_codes=policy_disabled detail_codes=skipped:policy_disabled"
        );
        assert_eq!(
            disabled.detail_codes(),
            vec!["skipped:policy_disabled".to_owned()]
        );
    }

    #[test]
    fn retention_planner_reports_read_only_adapter_capabilities() {
        let planner = DefaultMemoryRetentionPlanner;
        let descriptor = planner.descriptor();

        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::RetentionPlanning)
        );
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::CompactionPlanning)
        );
        assert!(planner.health().unwrap().ready);
    }
}
