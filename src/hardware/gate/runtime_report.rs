use crate::runtime_manifest::RuntimeManifest;

use super::super::{
    ComputeLane, DeviceClass, DeviceMemoryMode, DeviceTier, HardwarePlan, RuntimeAdapterHint,
};
use super::validation::{
    validate_device_plan, validate_runtime_device_contract, validate_runtime_manifest_for_device,
};

#[derive(Debug, Clone)]
pub struct RuntimeManifestDeviceGateReport {
    pub device: DeviceClass,
    pub tier: DeviceTier,
    pub primary_lane: ComputeLane,
    pub fallback_lane: ComputeLane,
    pub memory_mode: DeviceMemoryMode,
    pub adapter_hints: Vec<RuntimeAdapterHint>,
    pub runtime_adapter: Option<RuntimeAdapterHint>,
    pub max_parallel_chunks: usize,
    pub kv_prefetch_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
    pub allow_disk_spill: bool,
    pub local_kv_token_budget: usize,
    pub global_kv_token_budget: usize,
    pub latency_budget_ms: Option<u64>,
    pub runtime_device_contract: String,
    pub failures: Vec<String>,
}

impl RuntimeManifestDeviceGateReport {
    pub fn evaluate(manifest: &RuntimeManifest, plan: &HardwarePlan) -> Self {
        let runtime_device_contract = plan.runtime_contract_summary();
        let runtime_adapter = manifest.preferred_adapter_for(&plan.execution);
        let mut failures = validate_device_plan(plan);
        failures.extend(validate_runtime_device_contract(
            plan,
            &runtime_device_contract,
        ));
        failures.extend(validate_runtime_manifest_for_device(
            manifest,
            plan.device,
            &plan.execution,
            runtime_adapter,
        ));

        Self {
            device: plan.device,
            tier: plan.tier,
            primary_lane: plan.execution.primary_lane,
            fallback_lane: plan.execution.fallback_lane,
            memory_mode: plan.execution.memory_mode,
            adapter_hints: plan.execution.adapter_hints.clone(),
            runtime_adapter,
            max_parallel_chunks: plan.execution.max_parallel_chunks,
            kv_prefetch_blocks: plan.execution.kv_prefetch_blocks,
            hot_kv_precision_bits: plan.execution.hot_kv_precision_bits,
            cold_kv_precision_bits: plan.execution.cold_kv_precision_bits,
            allow_disk_spill: plan.execution.allow_disk_spill,
            local_kv_token_budget: plan.local_kv_token_budget,
            global_kv_token_budget: plan.global_kv_token_budget,
            latency_budget_ms: plan.latency_budget_ms,
            runtime_device_contract,
            failures,
        }
    }

    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn adapters_csv(&self) -> String {
        self.adapter_hints
            .iter()
            .map(|adapter| adapter.as_str())
            .collect::<Vec<_>>()
            .join("+")
    }

    pub fn runtime_adapter_name(&self) -> &'static str {
        self.runtime_adapter
            .map(RuntimeAdapterHint::as_str)
            .unwrap_or("none")
    }

    pub fn summary_line(&self) -> String {
        format!(
            "runtime_manifest_device_gate: passed={} device={} tier={} runtime_adapter={} failures={}",
            self.passed(),
            self.device.as_str(),
            self.tier.as_str(),
            self.runtime_adapter_name(),
            self.failures.len()
        )
    }
}
