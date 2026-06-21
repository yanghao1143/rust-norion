use crate::hierarchy::{HierarchyWeights, TaskProfile};

use super::super::device::{
    ComputeLane, DeviceClass, DeviceMemoryMode, DeviceTier, RuntimeAdapterHint,
};
use super::super::probe::HardwareSnapshot;
use super::allocator::HardwareAllocator;
use super::runtime_budget::RuntimeBudgetReport;

#[derive(Debug, Clone)]
pub struct DeviceExecutionPlan {
    pub primary_lane: ComputeLane,
    pub fallback_lane: ComputeLane,
    pub memory_mode: DeviceMemoryMode,
    pub adapter_hints: Vec<RuntimeAdapterHint>,
    pub max_parallel_chunks: usize,
    pub kv_prefetch_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
    pub allow_disk_spill: bool,
}

impl DeviceExecutionPlan {
    pub fn summary(&self) -> String {
        let adapters = self
            .adapter_hints
            .iter()
            .map(|adapter| adapter.as_str())
            .collect::<Vec<_>>()
            .join("+");
        format!(
            "primary={} fallback={} memory={} adapters={} parallel_chunks={} kv_prefetch={} kv_bits={}/{} disk_spill={}",
            self.primary_lane.as_str(),
            self.fallback_lane.as_str(),
            self.memory_mode.as_str(),
            adapters,
            self.max_parallel_chunks,
            self.kv_prefetch_blocks,
            self.hot_kv_precision_bits,
            self.cold_kv_precision_bits,
            self.allow_disk_spill
        )
    }
}

#[derive(Debug, Clone)]
pub struct HardwarePlan {
    pub device: DeviceClass,
    pub tier: DeviceTier,
    pub pressure: f32,
    pub latency_budget_ms: Option<u64>,
    pub local_kv_token_budget: usize,
    pub global_kv_token_budget: usize,
    pub hierarchy: HierarchyWeights,
    pub execution: DeviceExecutionPlan,
    pub runtime_budget: RuntimeBudgetReport,
    pub notes: Vec<String>,
}

impl Default for HardwarePlan {
    fn default() -> Self {
        HardwareAllocator::new().plan(
            HardwareSnapshot::default(),
            TaskProfile::General,
            0,
            HierarchyWeights::default(),
        )
    }
}

impl HardwarePlan {
    pub fn compute_headroom(&self) -> f32 {
        self.tier.compute_headroom()
    }

    pub fn runtime_contract_summary(&self) -> String {
        let adapters = self
            .execution
            .adapter_hints
            .iter()
            .map(|adapter| adapter.as_str())
            .collect::<Vec<_>>()
            .join("+");
        format!(
            "device={} tier={} pressure={:.3} compute_headroom={:.2} primary={} fallback={} memory={} adapters={} parallel_chunks={} kv_prefetch={} kv_bits={}/{} hot_kv_bits={} cold_kv_bits={} disk_spill={} local_kv_tokens={} global_kv_tokens={} latency_budget_ms={}",
            self.device.as_str(),
            self.tier.as_str(),
            self.pressure,
            self.compute_headroom(),
            self.execution.primary_lane.as_str(),
            self.execution.fallback_lane.as_str(),
            self.execution.memory_mode.as_str(),
            adapters,
            self.execution.max_parallel_chunks,
            self.execution.kv_prefetch_blocks,
            self.execution.hot_kv_precision_bits,
            self.execution.cold_kv_precision_bits,
            self.execution.hot_kv_precision_bits,
            self.execution.cold_kv_precision_bits,
            self.execution.allow_disk_spill,
            self.local_kv_token_budget,
            self.global_kv_token_budget,
            self.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned())
        )
    }

    pub fn summary(&self) -> String {
        format!(
            "device={} tier={} pressure={:.3} compute_headroom={:.2} latency_budget_ms={} local_kv_tokens={} global_kv_tokens={} hierarchy=({:.2},{:.2},{:.2}) execution=({}) runtime_budget=({})",
            self.device.as_str(),
            self.tier.as_str(),
            self.pressure,
            self.compute_headroom(),
            self.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.local_kv_token_budget,
            self.global_kv_token_budget,
            self.hierarchy.global,
            self.hierarchy.local,
            self.hierarchy.convolution,
            self.execution.summary(),
            self.runtime_budget.summary()
        )
    }
}
