use crate::runtime_manifest::RuntimeManifest;

use super::super::{
    ComputeLane, DeviceClass, DeviceExecutionPlan, DeviceProfileDescriptor, DeviceTier,
    HardwarePlan, MemoryGovernancePlan, RuntimeAdapterHint,
};

pub(in crate::hardware) fn validate_device_plan(plan: &HardwarePlan) -> Vec<String> {
    let mut failures = Vec::new();

    if plan.local_kv_token_budget < 32 {
        failures.push(format!(
            "local_kv_token_budget {} below minimum 32",
            plan.local_kv_token_budget
        ));
    }
    if plan.global_kv_token_budget < 32 {
        failures.push(format!(
            "global_kv_token_budget {} below minimum 32",
            plan.global_kv_token_budget
        ));
    }
    if plan.execution.max_parallel_chunks == 0 {
        failures.push("max_parallel_chunks must be at least 1".to_owned());
    }
    if plan.execution.kv_prefetch_blocks == 0 {
        failures.push("kv_prefetch_blocks must be at least 1".to_owned());
    }
    if !matches!(plan.execution.hot_kv_precision_bits, 4 | 8) {
        failures.push(format!(
            "hot_kv_precision_bits {} must be 4 or 8",
            plan.execution.hot_kv_precision_bits
        ));
    }
    if !matches!(plan.execution.cold_kv_precision_bits, 4 | 8) {
        failures.push(format!(
            "cold_kv_precision_bits {} must be 4 or 8",
            plan.execution.cold_kv_precision_bits
        ));
    }
    if plan.execution.cold_kv_precision_bits > plan.execution.hot_kv_precision_bits {
        failures.push(format!(
            "cold_kv_precision_bits {} must not exceed hot_kv_precision_bits {}",
            plan.execution.cold_kv_precision_bits, plan.execution.hot_kv_precision_bits
        ));
    }
    if plan.execution.adapter_hints.is_empty() {
        failures.push("adapter_hints must not be empty".to_owned());
    }
    if !has_portable_escape_hatch(plan) {
        failures.push("plan must include a CPU or portable Rust fallback".to_owned());
    }
    if matches!(plan.tier, DeviceTier::Tiny | DeviceTier::Constrained)
        && !plan.execution.allow_disk_spill
    {
        failures.push("tiny and constrained devices must allow disk spill".to_owned());
    }
    if plan.tier == DeviceTier::Distributed && plan.execution.max_parallel_chunks < 2 {
        failures.push("distributed devices should expose more than one parallel chunk".to_owned());
    }

    failures
}

pub(in crate::hardware) fn validate_runtime_device_contract(
    plan: &HardwarePlan,
    contract: &str,
) -> Vec<String> {
    let mut failures = Vec::new();
    if contract.trim().is_empty() {
        failures.push("runtime_device_contract must not be empty".to_owned());
        return failures;
    }
    if contract.contains('\n') || contract.contains('\r') {
        failures.push("runtime_device_contract must be a single line".to_owned());
    }
    if contract.contains(',') {
        failures.push("runtime_device_contract must avoid CSV-breaking commas".to_owned());
    }

    let expected_fields = [
        format!("device={}", plan.device.as_str()),
        format!("tier={}", plan.tier.as_str()),
        format!("pressure={:.3}", plan.pressure),
        format!("compute_headroom={:.2}", plan.compute_headroom()),
        format!("primary={}", plan.execution.primary_lane.as_str()),
        format!("fallback={}", plan.execution.fallback_lane.as_str()),
        format!("memory={}", plan.execution.memory_mode.as_str()),
        "adapters=".to_owned(),
        format!("parallel_chunks={}", plan.execution.max_parallel_chunks),
        format!("kv_prefetch={}", plan.execution.kv_prefetch_blocks),
        format!(
            "kv_bits={}/{}",
            plan.execution.hot_kv_precision_bits, plan.execution.cold_kv_precision_bits
        ),
        format!("hot_kv_bits={}", plan.execution.hot_kv_precision_bits),
        format!("cold_kv_bits={}", plan.execution.cold_kv_precision_bits),
        format!("disk_spill={}", plan.execution.allow_disk_spill),
        format!("local_kv_tokens={}", plan.local_kv_token_budget),
        format!("global_kv_tokens={}", plan.global_kv_token_budget),
        format!(
            "latency_budget_ms={}",
            plan.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned())
        ),
    ];

    for field in expected_fields {
        if !contract.contains(&field) {
            failures.push(format!(
                "runtime_device_contract missing required field {field}"
            ));
        }
    }

    for adapter in &plan.execution.adapter_hints {
        if !contract.contains(adapter.as_str()) {
            failures.push(format!(
                "runtime_device_contract missing adapter {}",
                adapter.as_str()
            ));
        }
    }

    failures
}

pub(in crate::hardware) fn validate_runtime_manifest_for_device(
    manifest: &RuntimeManifest,
    device: DeviceClass,
    execution: &DeviceExecutionPlan,
    runtime_adapter: Option<RuntimeAdapterHint>,
) -> Vec<String> {
    let mut failures = Vec::new();
    let validation = manifest.validate();

    failures.extend(
        validation
            .errors
            .iter()
            .map(|error| format!("runtime manifest validation error: {error}")),
    );

    if !manifest.supports_device(device) {
        failures.push(format!(
            "runtime manifest does not support device {}",
            device.as_str()
        ));
    }
    if runtime_adapter.is_none() {
        failures.push(format!(
            "runtime manifest has no adapter intersection with device {}",
            device.as_str()
        ));
    }
    if let Some(adapter) = runtime_adapter {
        if !execution.adapter_hints.contains(&adapter) {
            failures.push(format!(
                "runtime adapter {} is outside device execution adapter hints",
                adapter.as_str()
            ));
        }
        if !manifest.adapter_hints.contains(&adapter) {
            failures.push(format!(
                "runtime adapter {} is outside manifest adapter hints",
                adapter.as_str()
            ));
        }
    }
    if manifest.kv_policy.import_enabled {
        if manifest.kv_policy.max_import_blocks == 0 {
            failures.push(
                "runtime manifest enables KV import but max_import_blocks is zero".to_owned(),
            );
        }
        if execution.kv_prefetch_blocks > manifest.kv_policy.max_import_blocks {
            failures.push(format!(
                "device KV prefetch {} exceeds runtime manifest max_import_blocks {}",
                execution.kv_prefetch_blocks, manifest.kv_policy.max_import_blocks
            ));
        }
    }
    if manifest.kv_policy.export_enabled && manifest.kv_policy.max_export_blocks == 0 {
        failures
            .push("runtime manifest enables KV export but max_export_blocks is zero".to_owned());
    }
    if execution.hot_kv_precision_bits > manifest.quantization.hot_kv.width() {
        failures.push(format!(
            "device hot KV precision {} exceeds runtime manifest hot KV precision {}",
            execution.hot_kv_precision_bits,
            manifest.quantization.hot_kv.width()
        ));
    }
    if execution.cold_kv_precision_bits > manifest.quantization.cold_kv.width() {
        failures.push(format!(
            "device cold KV precision {} exceeds runtime manifest cold KV precision {}",
            execution.cold_kv_precision_bits,
            manifest.quantization.cold_kv.width()
        ));
    }

    failures
}

pub(super) fn validate_memory_governance_plan(plan: &MemoryGovernancePlan) -> Vec<String> {
    let mut failures = Vec::new();

    if plan.retention_policy.stale_after == 0 {
        failures.push("retention stale_after must be at least 1".to_owned());
    }
    if !(0.0..=0.95).contains(&plan.retention_policy.decay_rate) {
        failures.push(format!(
            "retention decay_rate {:.3} outside 0.0..=0.95",
            plan.retention_policy.decay_rate
        ));
    }
    if !(0.0..=3.0).contains(&plan.retention_policy.remove_below_strength) {
        failures.push(format!(
            "retention remove_below_strength {:.3} outside 0.0..=3.0",
            plan.retention_policy.remove_below_strength
        ));
    }
    if plan.retention_policy.remove_after_failures == 0 {
        failures.push("retention remove_after_failures must be at least 1".to_owned());
    }
    if !(0.10..=0.999).contains(&plan.compaction_policy.similarity_threshold) {
        failures.push(format!(
            "compaction similarity_threshold {:.3} outside 0.10..=0.999",
            plan.compaction_policy.similarity_threshold
        ));
    }
    if plan.compaction_policy.max_candidates < 2 {
        failures.push(format!(
            "compaction max_candidates {} below minimum 2",
            plan.compaction_policy.max_candidates
        ));
    }
    if !plan.notes.iter().any(|note| note.starts_with("device:")) {
        failures.push("memory governance notes missing device marker".to_owned());
    }
    if !plan.notes.iter().any(|note| note.starts_with("tier:")) {
        failures.push("memory governance notes missing tier marker".to_owned());
    }
    if !plan
        .notes
        .iter()
        .any(|note| note.starts_with("memory_policy:"))
    {
        failures.push("memory governance notes missing memory_policy marker".to_owned());
    }

    failures
}

pub(super) fn validate_device_descriptor(descriptor: DeviceProfileDescriptor) -> Vec<String> {
    let mut failures = Vec::new();

    if descriptor.aliases.is_empty() {
        failures.push(format!(
            "device descriptor for {} must include at least one alias",
            descriptor.device.as_str()
        ));
    }
    if descriptor.tier != descriptor.device.tier() {
        failures.push(format!(
            "device descriptor tier {} does not match computed tier {}",
            descriptor.tier.as_str(),
            descriptor.device.tier().as_str()
        ));
    }
    for alias in descriptor.aliases {
        match alias.parse::<DeviceClass>() {
            Ok(parsed) if parsed == descriptor.device => {}
            Ok(parsed) => failures.push(format!(
                "alias {alias} maps to {} instead of {}",
                parsed.as_str(),
                descriptor.device.as_str()
            )),
            Err(error) => failures.push(format!("alias {alias} is not parseable: {error}")),
        }
    }

    failures
}

fn has_portable_escape_hatch(plan: &HardwarePlan) -> bool {
    matches!(
        plan.execution.fallback_lane,
        ComputeLane::CpuPortable | ComputeLane::CpuVector | ComputeLane::DiskBackedStreaming
    ) || plan.execution.adapter_hints.iter().any(|adapter| {
        matches!(
            adapter,
            RuntimeAdapterHint::PortableRust | RuntimeAdapterHint::CpuSimd
        )
    })
}
