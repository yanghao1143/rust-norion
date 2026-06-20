use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};

use super::super::device::DeviceTier;
use super::super::probe::HardwareSnapshot;

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryGovernancePlan {
    pub retention_policy: MemoryRetentionPolicy,
    pub compaction_policy: MemoryCompactionPolicy,
    pub notes: Vec<String>,
}

impl MemoryGovernancePlan {
    pub fn summary(&self) -> String {
        format!(
            "retention=(stale_after={} decay_rate={:.3} remove_below={:.3} remove_after_failures={}) compaction=(threshold={:.3} max_candidates={} max_merges={}) notes={}",
            self.retention_policy.stale_after,
            self.retention_policy.decay_rate,
            self.retention_policy.remove_below_strength,
            self.retention_policy.remove_after_failures,
            self.compaction_policy.similarity_threshold,
            self.compaction_policy.max_candidates,
            self.compaction_policy.max_merges,
            self.notes.join("+")
        )
    }
}

pub(super) fn memory_governance_plan(
    snapshot: HardwareSnapshot,
    mut retention_policy: MemoryRetentionPolicy,
    mut compaction_policy: MemoryCompactionPolicy,
) -> MemoryGovernancePlan {
    let pressure = snapshot.pressure();
    let mut notes = vec![
        format!("device:{}", snapshot.device.as_str()),
        format!("tier:{}", snapshot.device.tier().as_str()),
    ];

    match snapshot.device.tier() {
        DeviceTier::Tiny => {
            tighten_retention(&mut retention_policy, 16, 0.12, 0.10, 2);
            tighten_compaction(&mut compaction_policy, 0.96, 32, 2);
            notes.push("memory_policy:tiny_minimal_state".to_owned());
        }
        DeviceTier::Constrained => {
            tighten_retention(&mut retention_policy, 40, 0.07, 0.06, 3);
            tighten_compaction(&mut compaction_policy, 0.94, 128, 8);
            notes.push("memory_policy:constrained_bounded_state".to_owned());
        }
        DeviceTier::Balanced => {
            floor_retention(&mut retention_policy, 64, 0.05, 0.04, 4);
            limit_compaction_candidates(&mut compaction_policy, 384, 24);
            notes.push("memory_policy:balanced_default_state".to_owned());
        }
        DeviceTier::Accelerated => {
            expand_retention(&mut retention_policy, 96, 0.035, 0.035, 5);
            expand_compaction(&mut compaction_policy, 0.91, 768, 48);
            notes.push("memory_policy:accelerated_keep_more_context".to_owned());
        }
        DeviceTier::Distributed => {
            expand_retention(&mut retention_policy, 128, 0.025, 0.030, 6);
            expand_compaction(&mut compaction_policy, 0.90, 1024, 64);
            notes.push("memory_policy:distributed_wide_memory_scan".to_owned());
        }
        DeviceTier::Auto => {
            notes.push("memory_policy:auto_keep_base_policy".to_owned());
        }
    }

    if pressure >= 0.88 {
        tighten_retention(&mut retention_policy, 20, 0.14, 0.12, 2);
        tighten_compaction(&mut compaction_policy, 0.965, 48, 2);
        notes.push("pressure:critical_shrink_memory_governance".to_owned());
    } else if pressure >= 0.72 {
        tighten_retention(&mut retention_policy, 28, 0.10, 0.09, 3);
        tighten_compaction(&mut compaction_policy, 0.955, 80, 4);
        notes.push("pressure:high_shrink_memory_governance".to_owned());
    } else if pressure >= 0.45 {
        retention_policy.stale_after = retention_policy.stale_after.clamp(1, 48);
        retention_policy.decay_rate = retention_policy.decay_rate.max(0.06).clamp(0.0, 0.95);
        retention_policy.remove_below_strength = retention_policy
            .remove_below_strength
            .max(0.05)
            .clamp(0.0, 3.0);
        compaction_policy.max_candidates = compaction_policy.max_candidates.clamp(2, 192);
        compaction_policy.max_merges = compaction_policy.max_merges.min(12);
        compaction_policy.similarity_threshold = compaction_policy
            .similarity_threshold
            .max(0.94)
            .clamp(0.10, 0.999);
        notes.push("pressure:medium_conserve_memory_governance".to_owned());
    } else {
        notes.push("pressure:low_keep_device_memory_governance".to_owned());
    }

    retention_policy.stale_after = retention_policy.stale_after.max(1);
    retention_policy.decay_rate = retention_policy.decay_rate.clamp(0.0, 0.95);
    retention_policy.remove_below_strength = retention_policy.remove_below_strength.clamp(0.0, 3.0);
    retention_policy.remove_after_failures = retention_policy.remove_after_failures.max(1);
    compaction_policy.similarity_threshold =
        compaction_policy.similarity_threshold.clamp(0.10, 0.999);
    compaction_policy.max_candidates = compaction_policy.max_candidates.max(2);

    MemoryGovernancePlan {
        retention_policy,
        compaction_policy,
        notes,
    }
}

fn tighten_retention(
    policy: &mut MemoryRetentionPolicy,
    max_stale_after: u64,
    min_decay_rate: f32,
    min_remove_below_strength: f32,
    max_remove_after_failures: u64,
) {
    policy.stale_after = policy.stale_after.min(max_stale_after).max(1);
    policy.decay_rate = policy.decay_rate.max(min_decay_rate).clamp(0.0, 0.95);
    policy.remove_below_strength = policy
        .remove_below_strength
        .max(min_remove_below_strength)
        .clamp(0.0, 3.0);
    policy.remove_after_failures = policy
        .remove_after_failures
        .min(max_remove_after_failures)
        .max(1);
}

fn floor_retention(
    policy: &mut MemoryRetentionPolicy,
    min_stale_after: u64,
    max_decay_rate: f32,
    max_remove_below_strength: f32,
    min_remove_after_failures: u64,
) {
    policy.stale_after = policy.stale_after.max(min_stale_after);
    policy.decay_rate = policy.decay_rate.min(max_decay_rate).clamp(0.0, 0.95);
    policy.remove_below_strength = policy
        .remove_below_strength
        .min(max_remove_below_strength)
        .clamp(0.0, 3.0);
    policy.remove_after_failures = policy.remove_after_failures.max(min_remove_after_failures);
}

fn expand_retention(
    policy: &mut MemoryRetentionPolicy,
    min_stale_after: u64,
    max_decay_rate: f32,
    max_remove_below_strength: f32,
    min_remove_after_failures: u64,
) {
    floor_retention(
        policy,
        min_stale_after,
        max_decay_rate,
        max_remove_below_strength,
        min_remove_after_failures,
    );
}

fn tighten_compaction(
    policy: &mut MemoryCompactionPolicy,
    min_similarity_threshold: f32,
    max_candidates: usize,
    max_merges: usize,
) {
    policy.similarity_threshold = policy
        .similarity_threshold
        .max(min_similarity_threshold)
        .clamp(0.10, 0.999);
    policy.max_candidates = policy.max_candidates.min(max_candidates).max(2);
    policy.max_merges = policy.max_merges.min(max_merges);
}

fn limit_compaction_candidates(
    policy: &mut MemoryCompactionPolicy,
    max_candidates: usize,
    max_merges: usize,
) {
    policy.max_candidates = policy.max_candidates.min(max_candidates).max(2);
    policy.max_merges = policy.max_merges.min(max_merges);
}

fn expand_compaction(
    policy: &mut MemoryCompactionPolicy,
    max_similarity_threshold: f32,
    min_candidates: usize,
    min_merges: usize,
) {
    policy.similarity_threshold = policy
        .similarity_threshold
        .min(max_similarity_threshold)
        .clamp(0.10, 0.999);
    policy.max_candidates = policy.max_candidates.max(min_candidates);
    policy.max_merges = policy.max_merges.max(min_merges);
}
