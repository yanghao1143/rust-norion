use crate::runtime_manifest::RuntimeManifest;

use super::super::{
    ComputeLane, DeviceClass, DeviceMemoryMode, DeviceTier, HardwarePlan, MemoryGovernancePlan,
    RuntimeAdapterHint,
};
use super::validation::{
    validate_device_descriptor, validate_device_plan, validate_memory_governance_plan,
    validate_runtime_device_contract, validate_runtime_manifest_for_device,
};

#[derive(Debug, Clone)]
pub struct DevicePlanGateRow {
    pub device: DeviceClass,
    pub tier: DeviceTier,
    pub scope: &'static str,
    pub alias_count: usize,
    pub primary_lane: ComputeLane,
    pub fallback_lane: ComputeLane,
    pub memory_mode: DeviceMemoryMode,
    pub adapter_hints: Vec<RuntimeAdapterHint>,
    pub max_parallel_chunks: usize,
    pub kv_prefetch_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
    pub allow_disk_spill: bool,
    pub local_kv_token_budget: usize,
    pub global_kv_token_budget: usize,
    pub latency_budget_ms: Option<u64>,
    pub memory_governance: MemoryGovernancePlan,
    pub runtime_adapter: Option<RuntimeAdapterHint>,
    pub runtime_kv_import_enabled: bool,
    pub runtime_kv_export_enabled: bool,
    pub runtime_max_import_blocks: usize,
    pub runtime_max_export_blocks: usize,
    pub runtime_hot_kv_precision_bits: u8,
    pub runtime_cold_kv_precision_bits: u8,
    pub hot_quant_policy_covered: bool,
    pub cold_quant_policy_covered: bool,
    pub runtime_quant_policy_covered: bool,
    pub kv_precision_order_valid: bool,
    pub runtime_device_contract: String,
    pub failures: Vec<String>,
}

impl DevicePlanGateRow {
    pub fn from_plan(
        plan: &HardwarePlan,
        governance: MemoryGovernancePlan,
        runtime_manifest: &RuntimeManifest,
    ) -> Self {
        let descriptor = plan.device.descriptor();
        let mut failures = validate_device_plan(plan);
        let runtime_device_contract = plan.runtime_contract_summary();
        failures.extend(validate_runtime_device_contract(
            plan,
            &runtime_device_contract,
        ));
        failures.extend(validate_memory_governance_plan(&governance));
        failures.extend(validate_device_descriptor(descriptor));
        let runtime_adapter = runtime_manifest.preferred_adapter_for(&plan.execution);
        failures.extend(validate_runtime_manifest_for_device(
            runtime_manifest,
            plan.device,
            &plan.execution,
            runtime_adapter,
        ));

        Self {
            device: plan.device,
            tier: plan.tier,
            scope: descriptor.scope,
            alias_count: descriptor.aliases.len(),
            primary_lane: plan.execution.primary_lane,
            fallback_lane: plan.execution.fallback_lane,
            memory_mode: plan.execution.memory_mode,
            adapter_hints: plan.execution.adapter_hints.clone(),
            max_parallel_chunks: plan.execution.max_parallel_chunks,
            kv_prefetch_blocks: plan.execution.kv_prefetch_blocks,
            hot_kv_precision_bits: plan.execution.hot_kv_precision_bits,
            cold_kv_precision_bits: plan.execution.cold_kv_precision_bits,
            allow_disk_spill: plan.execution.allow_disk_spill,
            local_kv_token_budget: plan.local_kv_token_budget,
            global_kv_token_budget: plan.global_kv_token_budget,
            latency_budget_ms: plan.latency_budget_ms,
            memory_governance: governance,
            runtime_adapter,
            runtime_kv_import_enabled: runtime_manifest.kv_policy.import_enabled,
            runtime_kv_export_enabled: runtime_manifest.kv_policy.export_enabled,
            runtime_max_import_blocks: runtime_manifest.kv_policy.max_import_blocks,
            runtime_max_export_blocks: runtime_manifest.kv_policy.max_export_blocks,
            runtime_hot_kv_precision_bits: runtime_manifest.quantization.hot_kv.width(),
            runtime_cold_kv_precision_bits: runtime_manifest.quantization.cold_kv.width(),
            hot_quant_policy_covered: matches!(plan.execution.hot_kv_precision_bits, 4 | 8),
            cold_quant_policy_covered: matches!(plan.execution.cold_kv_precision_bits, 4 | 8),
            runtime_quant_policy_covered: plan.execution.hot_kv_precision_bits
                <= runtime_manifest.quantization.hot_kv.width()
                && plan.execution.cold_kv_precision_bits
                    <= runtime_manifest.quantization.cold_kv.width(),
            kv_precision_order_valid: plan.execution.cold_kv_precision_bits
                <= plan.execution.hot_kv_precision_bits,
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

    pub fn aliases_csv(&self) -> String {
        self.device.descriptor().aliases_csv()
    }

    pub fn runtime_adapter_name(&self) -> &'static str {
        self.runtime_adapter
            .map(RuntimeAdapterHint::as_str)
            .unwrap_or("none")
    }
}
