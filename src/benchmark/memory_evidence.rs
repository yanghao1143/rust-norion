use crate::engine::InferenceOutcome;
use crate::hardware::DeviceClass;

use super::{BenchmarkCase, explicit_device_count, push_unique_device};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkMemoryGovernanceEvidence {
    pub cases: usize,
    pub retention_activity_cases: usize,
    pub compaction_activity_cases: usize,
    pub total_retention_decayed: usize,
    pub total_retention_removed: usize,
    pub total_compaction_merged: usize,
    pub total_compaction_removed: usize,
    pub failures: Vec<String>,
    pub(super) governance_devices: Vec<DeviceClass>,
    pub(super) retention_activity_devices: Vec<DeviceClass>,
    pub(super) compaction_activity_devices: Vec<DeviceClass>,
}

impl BenchmarkMemoryGovernanceEvidence {
    pub(super) fn record(&mut self, case: &BenchmarkCase, outcome: &InferenceOutcome) {
        self.cases += 1;
        let device = outcome.hardware_plan.device;
        push_unique_device(&mut self.governance_devices, device);

        let retention = &outcome.retention_report;
        let retention_removed = retention.removed.len();
        self.total_retention_decayed += retention.decayed;
        self.total_retention_removed += retention_removed;
        if retention.decayed > 0 || retention_removed > 0 {
            self.retention_activity_cases += 1;
            push_unique_device(&mut self.retention_activity_devices, device);
        }

        if outcome.memory_retention_policy.stale_after == 0 {
            self.failures.push(format!(
                "{}:{} retention stale_after must be > 0",
                device.as_str(),
                case.name
            ));
        }
        if !(0.0..=0.95).contains(&outcome.memory_retention_policy.decay_rate) {
            self.failures.push(format!(
                "{}:{} retention decay_rate {:.6} outside 0.0..=0.95",
                device.as_str(),
                case.name,
                outcome.memory_retention_policy.decay_rate
            ));
        }
        if !(0.0..=3.0).contains(&outcome.memory_retention_policy.remove_below_strength) {
            self.failures.push(format!(
                "{}:{} retention remove_below_strength {:.6} outside 0.0..=3.0",
                device.as_str(),
                case.name,
                outcome.memory_retention_policy.remove_below_strength
            ));
        }
        if outcome.memory_retention_policy.remove_after_failures == 0 {
            self.failures.push(format!(
                "{}:{} retention remove_after_failures must be > 0",
                device.as_str(),
                case.name
            ));
        }
        if retention.decayed > retention.before {
            self.failures.push(format!(
                "{}:{} retention decayed {} exceeds before {}",
                device.as_str(),
                case.name,
                retention.decayed,
                retention.before
            ));
        }
        if retention_removed > retention.before {
            self.failures.push(format!(
                "{}:{} retention removed {} exceeds before {}",
                device.as_str(),
                case.name,
                retention_removed,
                retention.before
            ));
        }
        if retention.after > retention.before {
            self.failures.push(format!(
                "{}:{} retention after {} exceeds before {}",
                device.as_str(),
                case.name,
                retention.after,
                retention.before
            ));
        }
        if retention.after.saturating_add(retention_removed) != retention.before {
            self.failures.push(format!(
                "{}:{} retention before {} does not match after+removed {}",
                device.as_str(),
                case.name,
                retention.before,
                retention.after.saturating_add(retention_removed)
            ));
        }

        let compaction = &outcome.memory_compaction_report;
        let compaction_merged = compaction.merged.len();
        let compaction_removed = compaction.removed.len();
        self.total_compaction_merged += compaction_merged;
        self.total_compaction_removed += compaction_removed;
        if compaction_merged > 0 || compaction_removed > 0 {
            self.compaction_activity_cases += 1;
            push_unique_device(&mut self.compaction_activity_devices, device);
        }

        if !(0.10..=0.999).contains(&outcome.memory_compaction_policy.similarity_threshold) {
            self.failures.push(format!(
                "{}:{} memory_compaction similarity_threshold {:.6} outside 0.10..=0.999",
                device.as_str(),
                case.name,
                outcome.memory_compaction_policy.similarity_threshold
            ));
        }
        if compaction.merged.len() != compaction.removed.len() {
            self.failures.push(format!(
                "{}:{} memory_compaction merged {} does not match removed {}",
                device.as_str(),
                case.name,
                compaction_merged,
                compaction_removed
            ));
        }
        if compaction_merged > outcome.memory_compaction_policy.max_merges {
            self.failures.push(format!(
                "{}:{} memory_compaction merged {} exceeds max_merges {}",
                device.as_str(),
                case.name,
                compaction_merged,
                outcome.memory_compaction_policy.max_merges
            ));
        }
        if compaction_removed > compaction.before {
            self.failures.push(format!(
                "{}:{} memory_compaction removed {} exceeds before {}",
                device.as_str(),
                case.name,
                compaction_removed,
                compaction.before
            ));
        }
        if compaction.after > compaction.before {
            self.failures.push(format!(
                "{}:{} memory_compaction after {} exceeds before {}",
                device.as_str(),
                case.name,
                compaction.after,
                compaction.before
            ));
        }
        if compaction.after.saturating_add(compaction_removed) != compaction.before {
            self.failures.push(format!(
                "{}:{} memory_compaction before {} does not match after+removed {}",
                device.as_str(),
                case.name,
                compaction.before,
                compaction.after.saturating_add(compaction_removed)
            ));
        }
        if (compaction.before < 2
            || outcome.memory_compaction_policy.max_candidates < 2
            || outcome.memory_compaction_policy.max_merges == 0)
            && (compaction_merged > 0
                || compaction_removed > 0
                || compaction.after != compaction.before)
        {
            self.failures.push(format!(
                    "{}:{} memory_compaction skipped state requires merged=0 removed=0 after=before, got merged={} removed={} before={} after={}",
                    device.as_str(),
                    case.name,
                    compaction_merged,
                    compaction_removed,
                    compaction.before,
                    compaction.after
                ));
        }
    }

    pub fn device_profiles(&self) -> usize {
        explicit_device_count(&self.governance_devices)
    }

    pub fn retention_activity_device_profiles(&self) -> usize {
        explicit_device_count(&self.retention_activity_devices)
    }

    pub fn compaction_activity_device_profiles(&self) -> usize {
        explicit_device_count(&self.compaction_activity_devices)
    }
}
