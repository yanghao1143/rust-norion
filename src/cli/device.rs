use rust_norion::{
    DeviceClass, DevicePlanGateReport, HardwareAllocator, HardwareSnapshot, HierarchyWeights,
    MemoryCompactionPolicy, MemoryRetentionPolicy, TaskProfile,
};

use crate::Args;

pub(crate) fn print_device_matrix_and_exit() -> ! {
    let allocator = HardwareAllocator::new();
    let base_hierarchy = HierarchyWeights::default();

    println!("Noiron device matrix");
    println!(
        "profile,tier,scope,aliases,primary_lane,fallback_lane,memory_mode,adapters,parallel_chunks,kv_prefetch,kv_bits,hot_quant_covered,cold_quant_covered,kv_order_valid,disk_spill,local_kv_tokens,global_kv_tokens,latency_budget_ms,retention_stale_after,retention_decay_rate,retention_remove_below,retention_remove_after_failures,compaction_threshold,compaction_max_candidates,compaction_max_merges"
    );

    for device in DeviceClass::explicit_profiles() {
        let descriptor = device.descriptor();
        let snapshot = HardwareSnapshot::new(*device, 0.35, 0.30, 0.45, 0.20);
        let plan = allocator.plan(snapshot, TaskProfile::General, 4096, base_hierarchy);
        let governance = allocator.memory_governance_plan(
            snapshot,
            MemoryRetentionPolicy::default(),
            MemoryCompactionPolicy::default(),
        );
        let adapters = plan
            .execution
            .adapter_hints
            .iter()
            .map(|adapter| adapter.as_str())
            .collect::<Vec<_>>()
            .join("+");
        let fields = vec![
            device.as_str().to_owned(),
            plan.tier.as_str().to_owned(),
            descriptor.scope.to_owned(),
            descriptor.aliases_csv(),
            plan.execution.primary_lane.as_str().to_owned(),
            plan.execution.fallback_lane.as_str().to_owned(),
            plan.execution.memory_mode.as_str().to_owned(),
            adapters,
            plan.execution.max_parallel_chunks.to_string(),
            plan.execution.kv_prefetch_blocks.to_string(),
            format!(
                "{}/{}",
                plan.execution.hot_kv_precision_bits, plan.execution.cold_kv_precision_bits
            ),
            matches!(plan.execution.hot_kv_precision_bits, 4 | 8).to_string(),
            matches!(plan.execution.cold_kv_precision_bits, 4 | 8).to_string(),
            (plan.execution.cold_kv_precision_bits <= plan.execution.hot_kv_precision_bits)
                .to_string(),
            plan.execution.allow_disk_spill.to_string(),
            plan.local_kv_token_budget.to_string(),
            plan.global_kv_token_budget.to_string(),
            plan.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            governance.retention_policy.stale_after.to_string(),
            format!("{:.3}", governance.retention_policy.decay_rate),
            format!("{:.3}", governance.retention_policy.remove_below_strength),
            governance
                .retention_policy
                .remove_after_failures
                .to_string(),
            format!("{:.3}", governance.compaction_policy.similarity_threshold),
            governance.compaction_policy.max_candidates.to_string(),
            governance.compaction_policy.max_merges.to_string(),
        ];
        println!("{}", fields.join(","));
    }

    std::process::exit(0);
}

pub(crate) fn print_device_probe_report(args: &Args) {
    let report = args.effective_probe_report();
    let plan = args.runtime_manifest_device_plan();
    let governance = HardwareAllocator::new().memory_governance_plan(
        report.snapshot(),
        MemoryRetentionPolicy::default(),
        MemoryCompactionPolicy::default(),
    );

    println!("Noiron device probe");
    println!("{}", report.summary());
    println!("auto_probe: {}", args.auto_device_probe.is_some());
    println!("device_flag_provided: {}", args.device_flag_provided);
    println!("profile: {:?}", args.profile);
    println!("prompt_tokens: {}", args.prompt_token_estimate());
    println!("hardware_plan: {}", plan.summary());
    println!(
        "runtime_device_contract: {}",
        plan.runtime_contract_summary()
    );
    println!("memory_governance: {}", governance.summary());
    for evidence in &report.evidence {
        println!("probe_evidence: {evidence}");
    }
}

pub(crate) fn print_device_gate_report(report: &DevicePlanGateReport) {
    println!("Noiron device compatibility gate");
    println!("{}", report.summary_line());
    println!(
        "kv_precision_policy: {}",
        report.kv_precision_policy_summary().summary_line()
    );
    println!(
        "profile,tier,scope,aliases,primary_lane,fallback_lane,memory_mode,adapters,runtime_adapter,parallel_chunks,kv_prefetch,kv_bits,hot_quant_covered,cold_quant_covered,runtime_quant_covered,kv_order_valid,disk_spill,runtime_kv_import,runtime_kv_export,runtime_max_import,runtime_max_export,runtime_kv_bits,local_kv_tokens,global_kv_tokens,latency_budget_ms,runtime_device_contract,retention_stale_after,retention_decay_rate,retention_remove_below,retention_remove_after_failures,compaction_threshold,compaction_max_candidates,compaction_max_merges,passed"
    );

    for row in &report.rows {
        let fields = vec![
            row.device.as_str().to_owned(),
            row.tier.as_str().to_owned(),
            row.scope.to_owned(),
            row.aliases_csv(),
            row.primary_lane.as_str().to_owned(),
            row.fallback_lane.as_str().to_owned(),
            row.memory_mode.as_str().to_owned(),
            row.adapters_csv(),
            row.runtime_adapter_name().to_owned(),
            row.max_parallel_chunks.to_string(),
            row.kv_prefetch_blocks.to_string(),
            format!(
                "{}/{}",
                row.hot_kv_precision_bits, row.cold_kv_precision_bits
            ),
            row.hot_quant_policy_covered.to_string(),
            row.cold_quant_policy_covered.to_string(),
            row.runtime_quant_policy_covered.to_string(),
            row.kv_precision_order_valid.to_string(),
            row.allow_disk_spill.to_string(),
            row.runtime_kv_import_enabled.to_string(),
            row.runtime_kv_export_enabled.to_string(),
            row.runtime_max_import_blocks.to_string(),
            row.runtime_max_export_blocks.to_string(),
            format!(
                "{}/{}",
                row.runtime_hot_kv_precision_bits, row.runtime_cold_kv_precision_bits
            ),
            row.local_kv_token_budget.to_string(),
            row.global_kv_token_budget.to_string(),
            row.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            row.runtime_device_contract.clone(),
            row.memory_governance
                .retention_policy
                .stale_after
                .to_string(),
            format!("{:.3}", row.memory_governance.retention_policy.decay_rate),
            format!(
                "{:.3}",
                row.memory_governance.retention_policy.remove_below_strength
            ),
            row.memory_governance
                .retention_policy
                .remove_after_failures
                .to_string(),
            format!(
                "{:.3}",
                row.memory_governance.compaction_policy.similarity_threshold
            ),
            row.memory_governance
                .compaction_policy
                .max_candidates
                .to_string(),
            row.memory_governance
                .compaction_policy
                .max_merges
                .to_string(),
            row.passed().to_string(),
        ];
        println!("{}", fields.join(","));

        for failure in &row.failures {
            println!("device_gate_failure: {}: {}", row.device.as_str(), failure);
        }
    }
}
