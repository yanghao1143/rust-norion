use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::runtime_manifest::RuntimeManifest;

use super::super::{DeviceClass, HardwareAllocator, HardwareSnapshot};
use super::{DevicePlanGateRow, KvPrecisionPolicySummary};

#[derive(Debug, Clone)]
pub struct DevicePlanGateReport {
    pub rows: Vec<DevicePlanGateRow>,
}

impl DevicePlanGateReport {
    pub fn evaluate() -> Self {
        Self::evaluate_with_allocator(&HardwareAllocator::new())
    }

    pub fn evaluate_with_allocator(allocator: &HardwareAllocator) -> Self {
        let runtime_manifest = RuntimeManifest::self_developed(
            "noiron-gate-transformer",
            "noiron-gate-tokenizer",
            65_536,
            256,
        );
        Self::evaluate_runtime_manifest_with_allocator(allocator, &runtime_manifest)
    }

    pub fn evaluate_runtime_manifest(runtime_manifest: &RuntimeManifest) -> Self {
        Self::evaluate_runtime_manifest_with_allocator(&HardwareAllocator::new(), runtime_manifest)
    }

    pub fn evaluate_runtime_manifest_with_allocator(
        allocator: &HardwareAllocator,
        runtime_manifest: &RuntimeManifest,
    ) -> Self {
        let base_hierarchy = HierarchyWeights::default();
        let rows = DeviceClass::explicit_profiles()
            .iter()
            .map(|device| {
                let plan = allocator.plan(
                    HardwareSnapshot::new(*device, 0.35, 0.30, 0.45, 0.20),
                    TaskProfile::General,
                    4096,
                    base_hierarchy,
                );
                let governance = allocator.memory_governance_plan(
                    HardwareSnapshot::new(*device, 0.35, 0.30, 0.45, 0.20),
                    MemoryRetentionPolicy::default(),
                    MemoryCompactionPolicy::default(),
                );
                DevicePlanGateRow::from_plan(&plan, governance, runtime_manifest)
            })
            .collect();

        Self { rows }
    }

    pub fn passed(&self) -> bool {
        self.rows.iter().all(DevicePlanGateRow::passed)
    }

    pub fn failure_count(&self) -> usize {
        self.rows.iter().map(|row| row.failures.len()).sum()
    }

    pub fn alias_count(&self) -> usize {
        self.rows.iter().map(|row| row.alias_count).sum()
    }

    pub fn hot_q4_profiles(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.hot_kv_precision_bits == 4)
            .count()
    }

    pub fn hot_q8_profiles(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.hot_kv_precision_bits == 8)
            .count()
    }

    pub fn cold_q4_profiles(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.cold_kv_precision_bits == 4)
            .count()
    }

    pub fn runtime_quant_covered_profiles(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.runtime_quant_policy_covered)
            .count()
    }

    pub fn kv_precision_policy_summary(&self) -> KvPrecisionPolicySummary {
        KvPrecisionPolicySummary {
            profiles: self.rows.len(),
            hot_q4_profiles: self.hot_q4_profiles(),
            hot_q8_profiles: self.hot_q8_profiles(),
            cold_q4_profiles: self.cold_q4_profiles(),
            runtime_covered_profiles: self.runtime_quant_covered_profiles(),
            order_valid_profiles: self
                .rows
                .iter()
                .filter(|row| row.kv_precision_order_valid)
                .count(),
        }
    }

    pub fn summary_line(&self) -> String {
        let kv_summary = self.kv_precision_policy_summary();
        format!(
            "device_gate: passed={} profiles={} aliases={} failures={} kv_precision=({})",
            self.passed(),
            self.rows.len(),
            self.alias_count(),
            self.failure_count(),
            kv_summary.summary_line()
        )
    }
}
