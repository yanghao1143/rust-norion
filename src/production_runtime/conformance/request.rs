use crate::agent_team::AgentTeamPlan;
use crate::hardware::{HardwareAllocator, HardwareSnapshot, RuntimeManifestDeviceGateReport};
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::kv_exchange::RuntimeKvBlock;
use crate::router::RouteBudget;
use crate::runtime::RuntimeRequest;
use crate::runtime_manifest::RuntimeManifest;
use crate::toolsmith::ToolsmithPlan;
use crate::transformer::TransformerRefactorPlan;

use super::report::ProductionKernelConformanceReport;
use super::util::deterministic_vector;

pub(super) fn conformance_import_blocks(manifest: &RuntimeManifest) -> Vec<RuntimeKvBlock> {
    if !manifest.kv_policy.import_enabled || manifest.kv_policy.max_import_blocks == 0 {
        return Vec::new();
    }

    let dims = manifest
        .architecture
        .hidden_size
        .max(manifest.metadata.embedding_dimensions)
        .clamp(1, 16);
    vec![RuntimeKvBlock::new(
        0,
        0,
        0,
        1,
        deterministic_vector("conformance-key", dims),
        deterministic_vector("conformance-value", dims),
    )]
}

pub(super) fn conformance_request(
    manifest: &RuntimeManifest,
    device_gate: &RuntimeManifestDeviceGateReport,
) -> RuntimeRequest {
    let prompt = format!(
        "Run production kernel conformance for {} with KV import and export diagnostics.",
        manifest.metadata.model_id
    );
    let prompt_tokens = prompt.split_whitespace().count().max(1);
    let mut hardware_plan = HardwareAllocator::new().plan(
        HardwareSnapshot::new(device_gate.device, 0.35, 0.30, 0.45, 0.20),
        TaskProfile::Coding,
        prompt_tokens,
        HierarchyWeights::default(),
    );
    hardware_plan.execution.primary_lane = device_gate.primary_lane;
    hardware_plan.execution.fallback_lane = device_gate.fallback_lane;
    hardware_plan.execution.memory_mode = device_gate.memory_mode;
    hardware_plan.execution.adapter_hints = device_gate.adapter_hints.clone();
    hardware_plan.execution.max_parallel_chunks = device_gate.max_parallel_chunks;
    hardware_plan.execution.kv_prefetch_blocks = device_gate.kv_prefetch_blocks;
    hardware_plan.execution.hot_kv_precision_bits = device_gate.hot_kv_precision_bits;
    hardware_plan.execution.cold_kv_precision_bits = device_gate.cold_kv_precision_bits;
    hardware_plan.execution.allow_disk_spill = device_gate.allow_disk_spill;
    hardware_plan.local_kv_token_budget = device_gate.local_kv_token_budget;
    hardware_plan.global_kv_token_budget = device_gate.global_kv_token_budget;
    hardware_plan.latency_budget_ms = device_gate.latency_budget_ms;

    RuntimeRequest {
        prompt,
        profile: TaskProfile::Coding,
        runtime_metadata: manifest.runtime_metadata(),
        runtime_architecture: manifest.architecture,
        memory_hints: Vec::new(),
        infini_memory_hints: Vec::new(),
        experience_hints: Vec::new(),
        runtime_adapter_observations: Vec::new(),
        toolsmith_plan: ToolsmithPlan::default(),
        agent_team_plan: AgentTeamPlan::default(),
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 2,
            fast_tokens: 1,
            attention_fraction: 2.0 / 3.0,
        },
        hierarchy: HierarchyWeights::default(),
        transformer_plan: TransformerRefactorPlan::default(),
        recursive_schedule: crate::recursive_scheduler::RecursiveSchedule::default(),
        hardware_plan,
        imported_kv_blocks: Vec::new(),
        max_tokens: 32,
    }
}

pub(super) fn evaluate_conformance_request_contract(
    manifest: &RuntimeManifest,
    device_gate: &RuntimeManifestDeviceGateReport,
    request: &RuntimeRequest,
    report: &mut ProductionKernelConformanceReport,
) {
    if report.manifest_hot_kv_bits != manifest.quantization.hot_kv.width()
        || report.manifest_cold_kv_bits != manifest.quantization.cold_kv.width()
    {
        report.failures.push(format!(
            "conformance report manifest KV precision {}/{} does not match runtime manifest {}/{}",
            report.manifest_hot_kv_bits,
            report.manifest_cold_kv_bits,
            manifest.quantization.hot_kv.width(),
            manifest.quantization.cold_kv.width()
        ));
    }
    if report.device_hot_kv_bits != device_gate.hot_kv_precision_bits
        || report.device_cold_kv_bits != device_gate.cold_kv_precision_bits
    {
        report.failures.push(format!(
            "conformance report device KV precision {}/{} does not match device gate {}/{}",
            report.device_hot_kv_bits,
            report.device_cold_kv_bits,
            device_gate.hot_kv_precision_bits,
            device_gate.cold_kv_precision_bits
        ));
    }
    if request.runtime_metadata.hot_kv_precision_bits != manifest.quantization.hot_kv.width()
        || request.runtime_metadata.cold_kv_precision_bits != manifest.quantization.cold_kv.width()
    {
        report.failures.push(format!(
            "conformance request runtime KV precision {}/{} does not match manifest KV precision {}/{}",
            request.runtime_metadata.hot_kv_precision_bits,
            request.runtime_metadata.cold_kv_precision_bits,
            manifest.quantization.hot_kv.width(),
            manifest.quantization.cold_kv.width()
        ));
    }
    if request.hardware_plan.execution.hot_kv_precision_bits != device_gate.hot_kv_precision_bits
        || request.hardware_plan.execution.cold_kv_precision_bits
            != device_gate.cold_kv_precision_bits
    {
        report.failures.push(format!(
            "conformance request device KV precision {}/{} does not match production device gate {}/{}",
            request.hardware_plan.execution.hot_kv_precision_bits,
            request.hardware_plan.execution.cold_kv_precision_bits,
            device_gate.hot_kv_precision_bits,
            device_gate.cold_kv_precision_bits
        ));
    }
    if device_gate.hot_kv_precision_bits > manifest.quantization.hot_kv.width() {
        report.failures.push(format!(
            "production device gate hot KV precision {} exceeds manifest hot KV precision {}",
            device_gate.hot_kv_precision_bits,
            manifest.quantization.hot_kv.width()
        ));
    }
    if device_gate.cold_kv_precision_bits > manifest.quantization.cold_kv.width() {
        report.failures.push(format!(
            "production device gate cold KV precision {} exceeds manifest cold KV precision {}",
            device_gate.cold_kv_precision_bits,
            manifest.quantization.cold_kv.width()
        ));
    }
    if request.runtime_metadata.cold_kv_precision_bits
        > request.runtime_metadata.hot_kv_precision_bits
    {
        report.failures.push(format!(
            "conformance request runtime cold KV precision {} must not exceed hot KV precision {}",
            request.runtime_metadata.cold_kv_precision_bits,
            request.runtime_metadata.hot_kv_precision_bits
        ));
    }
    if request.hardware_plan.execution.cold_kv_precision_bits
        > request.hardware_plan.execution.hot_kv_precision_bits
    {
        report.failures.push(format!(
            "conformance request device cold KV precision {} must not exceed hot KV precision {}",
            request.hardware_plan.execution.cold_kv_precision_bits,
            request.hardware_plan.execution.hot_kv_precision_bits
        ));
    }
}
