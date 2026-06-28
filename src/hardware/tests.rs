use super::*;

fn assert_evidence(report: &HardwareProbeReport, expected: &str) {
    assert!(
        report.evidence.iter().any(|item| item == expected),
        "missing evidence {expected}; report={}",
        report.summary()
    );
}

fn assert_evidence_prefix(report: &HardwareProbeReport, expected: &str) {
    assert!(
        report
            .evidence
            .iter()
            .any(|item| item.starts_with(expected)),
        "missing evidence prefix {expected}; report={}",
        report.summary()
    );
}

#[test]
fn cpu_only_high_pressure_tightens_budget_and_boosts_convolution() {
    let allocator = HardwareAllocator::new();
    let base = HierarchyWeights::new(0.3, 0.4, 0.3);

    let plan = allocator.plan(
        HardwareSnapshot::new(DeviceClass::CpuOnly, 0.92, 0.0, 0.88, 0.40),
        TaskProfile::LongDocument,
        16_384,
        base,
    );

    assert!(plan.latency_budget_ms.is_some());
    assert!(plan.local_kv_token_budget < 512);
    assert!(plan.global_kv_token_budget < 4096);
    assert!(plan.hierarchy.convolution > base.convolution);
}

#[test]
fn server_low_pressure_expands_kv_budget() {
    let allocator = HardwareAllocator::new();

    let plan = allocator.plan(
        HardwareSnapshot::new(DeviceClass::Server, 0.10, 0.20, 0.18, 0.08),
        TaskProfile::Coding,
        1024,
        HierarchyWeights::new(0.2, 0.6, 0.2),
    );

    assert!(plan.latency_budget_ms.is_none());
    assert!(plan.local_kv_token_budget > 512);
    assert!(plan.global_kv_token_budget > 4096);
}

#[test]
fn mobile_and_embedded_profiles_tighten_kv_budgets() {
    let allocator = HardwareAllocator::new();
    let base = HierarchyWeights::new(0.30, 0.40, 0.30);

    let mobile = allocator.plan(
        HardwareSnapshot::new(DeviceClass::Mobile, 0.30, 0.35, 0.82, 0.10),
        TaskProfile::General,
        2048,
        base,
    );
    let embedded = allocator.plan(
        HardwareSnapshot::new(DeviceClass::Embedded, 0.45, 0.0, 0.80, 0.20),
        TaskProfile::LongDocument,
        2048,
        base,
    );
    let browser = allocator.plan(
        HardwareSnapshot::new(DeviceClass::BrowserWasm, 0.30, 0.18, 0.70, 0.08),
        TaskProfile::General,
        2048,
        base,
    );
    let microcontroller = allocator.plan(
        HardwareSnapshot::new(DeviceClass::Microcontroller, 0.55, 0.0, 0.78, 0.20),
        TaskProfile::LongDocument,
        2048,
        base,
    );

    assert!(mobile.local_kv_token_budget < 512);
    assert!(mobile.global_kv_token_budget < 4096);
    assert!(mobile.hierarchy.convolution > base.convolution);
    assert!(embedded.local_kv_token_budget < mobile.local_kv_token_budget);
    assert!(embedded.global_kv_token_budget < mobile.global_kv_token_budget);
    assert!(browser.local_kv_token_budget < mobile.local_kv_token_budget);
    assert!(browser.global_kv_token_budget < mobile.global_kv_token_budget);
    assert!(microcontroller.local_kv_token_budget < browser.local_kv_token_budget);
    assert!(microcontroller.global_kv_token_budget < browser.global_kv_token_budget);
}

#[test]
fn memory_governance_scales_by_device_tier_and_pressure() {
    let allocator = HardwareAllocator::new();
    let retention = MemoryRetentionPolicy::default();
    let compaction = MemoryCompactionPolicy::default();

    let tiny = allocator.memory_governance_plan(
        HardwareSnapshot::new(DeviceClass::Microcontroller, 0.30, 0.0, 0.35, 0.10),
        retention,
        compaction.clone(),
    );
    let server = allocator.memory_governance_plan(
        HardwareSnapshot::new(DeviceClass::Server, 0.10, 0.15, 0.20, 0.10),
        retention,
        compaction.clone(),
    );
    let overloaded_server = allocator.memory_governance_plan(
        HardwareSnapshot::new(DeviceClass::Server, 0.95, 0.95, 0.95, 0.90),
        retention,
        compaction,
    );

    assert!(tiny.retention_policy.stale_after < retention.stale_after);
    assert!(tiny.retention_policy.decay_rate > retention.decay_rate);
    assert!(
        tiny.compaction_policy.max_candidates < MemoryCompactionPolicy::default().max_candidates
    );
    assert!(
        tiny.compaction_policy.similarity_threshold
            > MemoryCompactionPolicy::default().similarity_threshold
    );
    assert!(server.retention_policy.stale_after > retention.stale_after);
    assert!(server.retention_policy.decay_rate < retention.decay_rate);
    assert!(
        server.compaction_policy.max_candidates > MemoryCompactionPolicy::default().max_candidates
    );
    assert!(overloaded_server.retention_policy.stale_after < server.retention_policy.stale_after);
    assert!(
        overloaded_server.compaction_policy.max_candidates
            < server.compaction_policy.max_candidates
    );
    assert!(
        overloaded_server.compaction_policy.similarity_threshold
            > server.compaction_policy.similarity_threshold
    );
}

#[test]
fn every_supported_device_profile_has_memory_governance_plan() {
    let allocator = HardwareAllocator::new();

    for device in DeviceClass::supported_profiles() {
        let plan = allocator.memory_governance_plan(
            HardwareSnapshot::new(*device, 0.35, 0.30, 0.45, 0.20),
            MemoryRetentionPolicy::default(),
            MemoryCompactionPolicy::default(),
        );

        assert!(plan.retention_policy.stale_after >= 1);
        assert!((0.0..=0.95).contains(&plan.retention_policy.decay_rate));
        assert!((0.0..=3.0).contains(&plan.retention_policy.remove_below_strength));
        assert!(plan.retention_policy.remove_after_failures >= 1);
        assert!((0.10..=0.999).contains(&plan.compaction_policy.similarity_threshold));
        assert!(plan.compaction_policy.max_candidates >= 2);
        assert!(plan.notes.iter().any(|note| note.starts_with("device:")));
        assert!(plan.notes.iter().any(|note| note.starts_with("tier:")));
        assert!(
            plan.notes
                .iter()
                .any(|note| note.starts_with("memory_policy:"))
        );
    }
}

#[test]
fn accelerator_profiles_parse_and_expand_when_capacity_exists() {
    let allocator = HardwareAllocator::new();
    let multi_gpu = "cluster".parse::<DeviceClass>().unwrap();
    let npu = "ane".parse::<DeviceClass>().unwrap();

    assert_eq!(multi_gpu, DeviceClass::MultiGpu);
    assert_eq!(npu, DeviceClass::NpuAccelerator);

    let plan = allocator.plan(
        HardwareSnapshot::new(multi_gpu, 0.12, 0.18, 0.20, 0.12),
        TaskProfile::LongDocument,
        4096,
        HierarchyWeights::new(0.30, 0.40, 0.30),
    );

    assert!(plan.local_kv_token_budget > 512);
    assert!(plan.global_kv_token_budget > 4096);
    assert!(plan.hierarchy.global > 0.30);
    assert!(plan.latency_budget_ms.is_none());
}

#[test]
fn common_device_aliases_parse_to_profiles() {
    assert_eq!(
        "unknown".parse::<DeviceClass>().unwrap(),
        DeviceClass::CpuOnly
    );
    assert_eq!(
        "loongarch64".parse::<DeviceClass>().unwrap(),
        DeviceClass::CpuOnly
    );
    assert_eq!(
        "laptop".parse::<DeviceClass>().unwrap(),
        DeviceClass::IntegratedGpu
    );
    assert_eq!(
        "handheld-console".parse::<DeviceClass>().unwrap(),
        DeviceClass::IntegratedGpu
    );
    assert_eq!(
        "steamdeck".parse::<DeviceClass>().unwrap(),
        DeviceClass::IntegratedGpu
    );
    assert_eq!(
        "rtx".parse::<DeviceClass>().unwrap(),
        DeviceClass::DiscreteGpu
    );
    assert_eq!(
        "directml".parse::<DeviceClass>().unwrap(),
        DeviceClass::DiscreteGpu
    );
    assert_eq!(
        "macbook".parse::<DeviceClass>().unwrap(),
        DeviceClass::UnifiedMemory
    );
    assert_eq!(
        "iphone".parse::<DeviceClass>().unwrap(),
        DeviceClass::Mobile
    );
    assert_eq!(
        "harmonyos".parse::<DeviceClass>().unwrap(),
        DeviceClass::Mobile
    );
    assert_eq!(
        "snapdragon".parse::<DeviceClass>().unwrap(),
        DeviceClass::NpuAccelerator
    );
    assert_eq!(
        "hailo".parse::<DeviceClass>().unwrap(),
        DeviceClass::NpuAccelerator
    );
    assert_eq!(
        "ascend".parse::<DeviceClass>().unwrap(),
        DeviceClass::NpuAccelerator
    );
    assert_eq!(
        "rknn".parse::<DeviceClass>().unwrap(),
        DeviceClass::NpuAccelerator
    );
    assert_eq!(
        "wasm".parse::<DeviceClass>().unwrap(),
        DeviceClass::BrowserWasm
    );
    assert_eq!(
        "webgpu".parse::<DeviceClass>().unwrap(),
        DeviceClass::BrowserWasm
    );
    assert_eq!(
        "microcontroller".parse::<DeviceClass>().unwrap(),
        DeviceClass::Microcontroller
    );
    assert_eq!(
        "no-std".parse::<DeviceClass>().unwrap(),
        DeviceClass::Microcontroller
    );
    assert_eq!(
        "riscv".parse::<DeviceClass>().unwrap(),
        DeviceClass::Embedded
    );
    assert_eq!(
        "wearable".parse::<DeviceClass>().unwrap(),
        DeviceClass::Mobile
    );
    assert_eq!("jetson".parse::<DeviceClass>().unwrap(), DeviceClass::Edge);
    assert_eq!(
        "automotive".parse::<DeviceClass>().unwrap(),
        DeviceClass::Edge
    );
    assert_eq!("nas".parse::<DeviceClass>().unwrap(), DeviceClass::Edge);
    assert_eq!(
        "datacenter".parse::<DeviceClass>().unwrap(),
        DeviceClass::Server
    );
    assert_eq!("epyc".parse::<DeviceClass>().unwrap(), DeviceClass::Server);
    assert_eq!("hpc".parse::<DeviceClass>().unwrap(), DeviceClass::Server);
    assert_eq!(
        "tensor-parallel".parse::<DeviceClass>().unwrap(),
        DeviceClass::MultiGpu
    );
}

#[test]
fn device_profile_descriptors_roundtrip_aliases() {
    for device in DeviceClass::explicit_profiles() {
        let descriptor = device.descriptor();

        assert_eq!(descriptor.device, *device);
        assert_eq!(descriptor.tier, device.tier());
        assert!(!descriptor.scope.is_empty());
        assert!(descriptor.aliases.len() >= 8);

        for alias in descriptor.aliases {
            assert_eq!(
                alias.parse::<DeviceClass>().unwrap(),
                *device,
                "alias {alias} should resolve to {}",
                device.as_str()
            );
        }
    }
}

#[test]
fn every_supported_device_profile_has_a_plan() {
    let allocator = HardwareAllocator::new();
    let base = HierarchyWeights::new(0.30, 0.40, 0.30);

    for device in DeviceClass::supported_profiles() {
        let plan = allocator.plan(
            HardwareSnapshot::new(*device, 0.35, 0.30, 0.45, 0.20),
            TaskProfile::General,
            4096,
            base,
        );
        let hierarchy_total =
            plan.hierarchy.global + plan.hierarchy.local + plan.hierarchy.convolution;

        assert_eq!(plan.device, *device);
        assert_eq!(plan.tier, device.tier());
        assert!(plan.local_kv_token_budget >= 32);
        assert!(plan.global_kv_token_budget >= 32);
        assert!(plan.execution.max_parallel_chunks >= 1);
        assert!(plan.execution.kv_prefetch_blocks >= 1);
        assert!(!plan.execution.adapter_hints.is_empty());
        assert!(matches!(plan.execution.hot_kv_precision_bits, 4 | 8));
        assert_eq!(plan.execution.cold_kv_precision_bits, 4);
        assert!(plan.execution.cold_kv_precision_bits <= plan.execution.hot_kv_precision_bits);
        assert!((hierarchy_total - 1.0).abs() < 0.001);
        assert!(plan.notes.iter().any(|note| note.starts_with("device:")));
        assert!(plan.notes.iter().any(|note| note.starts_with("tier:")));
        assert!(plan.notes.iter().any(|note| note.starts_with("execution:")));
        assert!(
            plan.notes
                .iter()
                .any(|note| note.starts_with("memory_mode:"))
        );
        assert!(
            plan.execution.adapter_hints.iter().any(|adapter| matches!(
                adapter,
                RuntimeAdapterHint::PortableRust | RuntimeAdapterHint::CpuSimd
            )) || matches!(
                plan.execution.fallback_lane,
                ComputeLane::CpuPortable | ComputeLane::CpuVector
            )
        );
    }
}

#[test]
fn runtime_device_capability_catalog_covers_common_backends() {
    let catalog = runtime_device_capability_catalog();

    assert!(
        catalog
            .iter()
            .any(|capability| capability.device == DeviceClass::CpuOnly
                && capability.backend_family == "cpu")
    );
    assert!(
        catalog
            .iter()
            .any(|capability| capability.backend_family.contains("cuda"))
    );
    assert!(
        catalog
            .iter()
            .any(|capability| capability.backend_family.contains("directml"))
    );
    assert!(
        catalog
            .iter()
            .any(|capability| capability.backend_family.contains("metal"))
    );
    assert!(
        catalog
            .iter()
            .any(|capability| capability.backend_family.contains("vulkan"))
    );
}

#[test]
fn runtime_budget_no_device_fails_closed_to_cpu_stub() {
    let plan = HardwareAllocator::new().plan(
        HardwareSnapshot::default(),
        TaskProfile::General,
        2048,
        HierarchyWeights::default(),
    );
    let budget = &plan.runtime_budget;

    assert_eq!(budget.requested_device, DeviceClass::Auto);
    assert_eq!(budget.selected_device, DeviceClass::CpuOnly);
    assert_eq!(budget.selected_adapter, RuntimeAdapterHint::PortableRust);
    assert_eq!(
        budget.quantization_profile,
        RuntimeQuantizationProfile::CpuStub
    );
    assert_eq!(
        budget.fallback_reason,
        RuntimeBudgetFallbackReason::AutoDeviceCpuStub
    );
    assert!(budget.fail_closed_cpu_stub);
    assert!(budget.read_only);
    assert!(!budget.write_allowed);
    assert!(!budget.applied);
    assert_eq!(
        budget.total_required_bytes,
        budget
            .model_weight_bytes
            .saturating_add(budget.kv_cache_bytes)
            .saturating_add(budget.gene_segment_cache_bytes)
            .saturating_add(budget.routing_reflection_overhead_bytes)
    );
}

#[test]
fn runtime_budget_low_memory_prefers_q4_without_state_writes() {
    let allocator = HardwareAllocator::new();
    let input = RuntimeBudgetInput::fixture(2048).with_available_memory_bytes(700 * 1024 * 1024);
    let plan = allocator.plan_with_runtime_budget(
        HardwareSnapshot::new(DeviceClass::Mobile, 0.20, 0.20, 0.80, 0.10),
        TaskProfile::General,
        2048,
        HierarchyWeights::default(),
        input,
    );
    let budget = &plan.runtime_budget;

    assert_eq!(budget.requested_device, DeviceClass::Mobile);
    assert_eq!(budget.selected_device, DeviceClass::Mobile);
    assert_eq!(budget.quantization_profile, RuntimeQuantizationProfile::Q4);
    assert_eq!(
        budget.fallback_reason,
        RuntimeBudgetFallbackReason::MemoryPressureQuantized
    );
    assert!(!budget.fail_closed_cpu_stub);
    assert!(budget.memory_pressure <= 1.0);
    assert!(budget.read_only);
    assert!(!budget.write_allowed);
    assert!(!budget.applied);
}

#[test]
fn runtime_budget_preferred_accelerator_keeps_q8_path() {
    let plan = HardwareAllocator::new().plan(
        HardwareSnapshot::new(DeviceClass::DiscreteGpu, 0.12, 0.18, 0.20, 0.10),
        TaskProfile::Coding,
        4096,
        HierarchyWeights::default(),
    );
    let budget = &plan.runtime_budget;

    assert_eq!(budget.requested_device, DeviceClass::DiscreteGpu);
    assert_eq!(budget.selected_device, DeviceClass::DiscreteGpu);
    assert_eq!(budget.selected_adapter, RuntimeAdapterHint::Cuda);
    assert_eq!(budget.quantization_profile, RuntimeQuantizationProfile::Q8);
    assert_eq!(budget.fallback_reason, RuntimeBudgetFallbackReason::None);
    assert!(!budget.fail_closed_cpu_stub);
    assert!(budget.available_budget_bytes > budget.total_required_bytes);
}

#[test]
fn runtime_budget_overflow_fails_closed_to_cpu_stub() {
    let allocator = HardwareAllocator::new();
    let input = RuntimeBudgetInput::fixture(4096)
        .with_model_parameter_count(4_000_000_000)
        .with_available_memory_bytes(128 * 1024 * 1024);
    let plan = allocator.plan_with_runtime_budget(
        HardwareSnapshot::new(DeviceClass::BrowserWasm, 0.20, 0.20, 0.70, 0.10),
        TaskProfile::LongDocument,
        4096,
        HierarchyWeights::default(),
        input,
    );
    let budget = &plan.runtime_budget;

    assert_eq!(budget.selected_device, DeviceClass::CpuOnly);
    assert_eq!(
        budget.quantization_profile,
        RuntimeQuantizationProfile::CpuStub
    );
    assert_eq!(
        budget.fallback_reason,
        RuntimeBudgetFallbackReason::BudgetExceededCpuStub
    );
    assert!(budget.fail_closed_cpu_stub);
    assert!(budget.memory_pressure > 1.0);
    assert!(!budget.write_allowed);
    assert!(!budget.applied);
}

#[test]
fn device_plan_gate_covers_all_explicit_profiles() {
    let report = DevicePlanGateReport::evaluate();

    assert!(report.passed(), "{:?}", report.rows);
    assert_eq!(report.rows.len(), DeviceClass::explicit_profiles().len());
    assert!(report.alias_count() >= 175);
    assert!(report.summary_line().contains("passed=true"));
    assert!(report.summary_line().contains("kv_precision=("));
    let kv_summary = report.kv_precision_policy_summary();
    assert_eq!(kv_summary.profiles, DeviceClass::explicit_profiles().len());
    assert!(kv_summary.hot_q4_profiles >= 3);
    assert!(kv_summary.hot_q8_profiles >= 1);
    assert_eq!(kv_summary.cold_q4_profiles, kv_summary.profiles);
    assert_eq!(kv_summary.runtime_covered_profiles, kv_summary.profiles);
    assert_eq!(kv_summary.order_valid_profiles, kv_summary.profiles);
    let tiny = report
        .rows
        .iter()
        .find(|row| row.device == DeviceClass::Microcontroller)
        .unwrap();
    let server = report
        .rows
        .iter()
        .find(|row| row.device == DeviceClass::Server)
        .unwrap();

    assert!(tiny.memory_governance.retention_policy.stale_after < 64);
    assert!(tiny.memory_governance.compaction_policy.max_candidates < 512);
    assert!(tiny.runtime_kv_import_enabled);
    assert_eq!(tiny.runtime_hot_kv_precision_bits, 8);
    assert_eq!(tiny.runtime_cold_kv_precision_bits, 4);
    assert!(tiny.kv_prefetch_blocks <= tiny.runtime_max_import_blocks);
    assert!(tiny.hot_kv_precision_bits <= tiny.runtime_hot_kv_precision_bits);
    assert!(tiny.hot_quant_policy_covered);
    assert!(tiny.cold_quant_policy_covered);
    assert!(tiny.runtime_quant_policy_covered);
    assert!(tiny.kv_precision_order_valid);
    assert!(
        tiny.runtime_device_contract
            .contains("device=microcontroller")
    );
    assert!(tiny.runtime_device_contract.contains("tier=tiny"));
    assert!(tiny.runtime_device_contract.contains("primary="));
    assert!(
        tiny.runtime_device_contract
            .contains("fallback=cpu-portable")
    );
    assert!(
        tiny.runtime_device_contract
            .contains("adapters=portable-rust")
    );
    assert!(tiny.runtime_device_contract.contains("kv_prefetch="));
    assert!(tiny.runtime_device_contract.contains("kv_bits="));
    assert!(tiny.runtime_device_contract.contains("local_kv_tokens="));
    assert!(!tiny.runtime_device_contract.contains(','));
    assert!(server.memory_governance.retention_policy.stale_after > 64);
    assert!(server.memory_governance.compaction_policy.max_candidates > 512);
    assert!(server.kv_prefetch_blocks <= server.runtime_max_import_blocks);
    assert!(server.runtime_device_contract.contains("device=server"));
    assert!(
        server
            .runtime_device_contract
            .contains("primary=discrete-gpu")
    );
    assert!(server.runtime_device_contract.contains("adapters="));
}

#[test]
fn runtime_manifest_gate_can_cover_every_device_profile() {
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 65_536, 256);

    let report = DevicePlanGateReport::evaluate_runtime_manifest(&manifest);

    assert!(report.passed(), "{:?}", report.rows);
    assert_eq!(report.rows.len(), DeviceClass::explicit_profiles().len());
    assert!(report.rows.iter().all(|row| row.runtime_adapter.is_some()));
}

#[test]
fn runtime_manifest_all_device_gate_reports_unsupported_profiles() {
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 65_536, 256)
        .with_supported_devices(vec![DeviceClass::CpuOnly])
        .with_adapter_hints(vec![RuntimeAdapterHint::PortableRust]);

    let report = DevicePlanGateReport::evaluate_runtime_manifest(&manifest);

    assert!(!report.passed());
    assert!(report.failure_count() > 0);
    let cpu = report
        .rows
        .iter()
        .find(|row| row.device == DeviceClass::CpuOnly)
        .unwrap();
    let mobile = report
        .rows
        .iter()
        .find(|row| row.device == DeviceClass::Mobile)
        .unwrap();
    assert!(cpu.passed(), "{:?}", cpu.failures);
    assert!(
        mobile
            .failures
            .iter()
            .any(|failure| failure.contains("does not support device mobile")),
        "{:?}",
        mobile.failures
    );
}

#[test]
fn runtime_device_contract_validation_reports_missing_fields() {
    let plan = HardwareAllocator::new().plan(
        HardwareSnapshot::new(DeviceClass::CpuOnly, 0.35, 0.30, 0.45, 0.20),
        TaskProfile::General,
        4096,
        HierarchyWeights::default(),
    );

    let failures = validate_runtime_device_contract(&plan, "device=cpu primary=cpu-vector");

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("tier=constrained"))
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("kv_prefetch"))
    );
    assert!(failures.iter().any(|failure| failure.contains("adapter")));
}

#[test]
fn runtime_manifest_gate_bounds_kv_prefetch_and_precision() {
    let execution = DeviceExecutionPlan {
        primary_lane: ComputeLane::CpuVector,
        fallback_lane: ComputeLane::CpuPortable,
        memory_mode: DeviceMemoryMode::TieredDisk,
        adapter_hints: vec![RuntimeAdapterHint::PortableRust],
        max_parallel_chunks: 1,
        kv_prefetch_blocks: 4,
        hot_kv_precision_bits: 8,
        cold_kv_precision_bits: 4,
        allow_disk_spill: true,
    };
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 4096, 128)
        .with_kv_policy(crate::runtime_manifest::RuntimeKvPolicy {
            import_enabled: true,
            export_enabled: true,
            max_import_blocks: 2,
            max_export_blocks: 1,
        })
        .with_quantization(crate::runtime_manifest::RuntimeQuantizationPolicy {
            hot_kv: crate::kv_quant::QuantizationBits::Four,
            cold_kv: crate::kv_quant::QuantizationBits::Four,
            weights: None,
        });

    let failures = validate_runtime_manifest_for_device(
        &manifest,
        DeviceClass::CpuOnly,
        &execution,
        Some(RuntimeAdapterHint::PortableRust),
    );

    assert!(failures.iter().any(|failure| failure.contains("prefetch")));
    assert!(failures.iter().any(|failure| failure.contains("hot KV")));
}

#[test]
fn device_plan_gate_rejects_cold_precision_above_hot_precision() {
    let mut plan = HardwareAllocator::new().plan(
        HardwareSnapshot::new(DeviceClass::Embedded, 0.35, 0.20, 0.70, 0.20),
        TaskProfile::General,
        2048,
        HierarchyWeights::default(),
    );
    plan.execution.hot_kv_precision_bits = 4;
    plan.execution.cold_kv_precision_bits = 8;

    let failures = validate_device_plan(&plan);

    assert!(failures.iter().any(|failure| {
        failure.contains("cold_kv_precision_bits") && failure.contains("hot_kv_precision_bits")
    }));
}

#[test]
fn runtime_manifest_device_gate_reports_current_device_contract() {
    let plan = HardwareAllocator::new().plan(
        HardwareSnapshot::new(DeviceClass::CpuOnly, 0.35, 0.0, 0.45, 0.20),
        TaskProfile::General,
        4096,
        HierarchyWeights::default(),
    );
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 4096, 128);

    let report = RuntimeManifestDeviceGateReport::evaluate(&manifest, &plan);

    assert!(report.passed(), "{:?}", report.failures);
    assert_eq!(report.device, DeviceClass::CpuOnly);
    assert_eq!(
        report.runtime_adapter,
        Some(RuntimeAdapterHint::PortableRust)
    );
    assert!(report.runtime_device_contract.contains("device=cpu"));
    assert!(report.runtime_device_contract.contains("adapters="));
    assert!(report.summary_line().contains("passed=true"));
}

#[test]
fn runtime_manifest_device_gate_blocks_retired_runtime_adapter() {
    let plan = HardwareAllocator::new().plan(
        HardwareSnapshot::new(DeviceClass::CpuOnly, 0.35, 0.0, 0.45, 0.20),
        TaskProfile::General,
        4096,
        HierarchyWeights::default(),
    );
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 4096, 128)
        .with_adapter_hints(vec![RuntimeAdapterHint::PortableRust])
        .with_retired_adapter_hints(vec![RuntimeAdapterHint::PortableRust]);

    let report = RuntimeManifestDeviceGateReport::evaluate(&manifest, &plan);

    assert!(!report.passed());
    assert_eq!(report.runtime_adapter, None);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("retired_blocked")),
        "{:?}",
        report.failures
    );
}

#[test]
fn runtime_manifest_device_gate_blocks_quarantined_runtime_adapter_with_evidence() {
    let plan = HardwareAllocator::new().plan(
        HardwareSnapshot::new(DeviceClass::CpuOnly, 0.35, 0.0, 0.45, 0.20),
        TaskProfile::General,
        4096,
        HierarchyWeights::default(),
    );
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 4096, 128)
        .with_adapter_hints(vec![RuntimeAdapterHint::PortableRust])
        .with_adapter_lifecycle_records(vec![
            crate::runtime_manifest::RuntimeAdapterLifecycleRecord::new(
                RuntimeAdapterHint::PortableRust,
                crate::runtime_manifest::RuntimeAdapterLifecycleState::Quarantined,
                "failed_lane_quarantine",
                "sha256:portable-runtime-source",
                "lineage:runtime:portable",
                "rollback:adapter:portable",
                "scope:local-runtime",
            ),
        ]);

    let report = RuntimeManifestDeviceGateReport::evaluate(&manifest, &plan);

    assert!(!report.passed());
    assert_eq!(report.runtime_adapter, None);
    assert!(
        report.failures.iter().any(|failure| {
            failure.contains("state=quarantined")
                && failure.contains("reason_code=failed_lane_quarantine")
                && failure.contains("source_digest=sha256:portable-runtime-source")
        }),
        "{:?}",
        report.failures
    );
}

#[test]
fn runtime_manifest_device_gate_rejects_device_and_adapter_mismatch() {
    let plan = HardwareAllocator::new().plan(
        HardwareSnapshot::new(DeviceClass::CpuOnly, 0.35, 0.0, 0.45, 0.20),
        TaskProfile::General,
        4096,
        HierarchyWeights::default(),
    );
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 4096, 128)
        .with_supported_devices(vec![DeviceClass::Server])
        .with_adapter_hints(vec![RuntimeAdapterHint::Cuda]);

    let report = RuntimeManifestDeviceGateReport::evaluate(&manifest, &plan);

    assert!(!report.passed());
    assert_eq!(report.runtime_adapter, None);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("does not support device cpu"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("no adapter intersection"))
    );
}

#[test]
fn execution_profiles_map_devices_to_portable_fallbacks() {
    let allocator = HardwareAllocator::new();
    let base = HierarchyWeights::new(0.30, 0.40, 0.30);

    let embedded = allocator.plan(
        HardwareSnapshot::new(DeviceClass::Embedded, 0.40, 0.0, 0.70, 0.30),
        TaskProfile::General,
        2048,
        base,
    );
    let mobile = allocator.plan(
        HardwareSnapshot::new(DeviceClass::Mobile, 0.30, 0.20, 0.50, 0.10),
        TaskProfile::General,
        2048,
        base,
    );
    let browser = allocator.plan(
        HardwareSnapshot::new(DeviceClass::BrowserWasm, 0.30, 0.20, 0.62, 0.08),
        TaskProfile::General,
        2048,
        base,
    );
    let microcontroller = allocator.plan(
        HardwareSnapshot::new(DeviceClass::Microcontroller, 0.45, 0.0, 0.72, 0.18),
        TaskProfile::LongDocument,
        2048,
        base,
    );
    let multi_gpu = allocator.plan(
        HardwareSnapshot::new(DeviceClass::MultiGpu, 0.12, 0.20, 0.20, 0.10),
        TaskProfile::Coding,
        2048,
        base,
    );
    let uma = allocator.plan(
        HardwareSnapshot::new(DeviceClass::UnifiedMemory, 0.16, 0.20, 0.26, 0.10),
        TaskProfile::LongDocument,
        2048,
        base,
    );

    assert_eq!(
        embedded.execution.primary_lane,
        ComputeLane::DiskBackedStreaming
    );
    assert_eq!(embedded.execution.fallback_lane, ComputeLane::CpuPortable);
    assert_eq!(
        embedded.execution.memory_mode,
        DeviceMemoryMode::MinimalDisk
    );
    assert_eq!(embedded.execution.hot_kv_precision_bits, 4);
    assert!(embedded.execution.allow_disk_spill);

    assert_eq!(browser.execution.primary_lane, ComputeLane::IntegratedGpu);
    assert_eq!(browser.execution.fallback_lane, ComputeLane::CpuPortable);
    assert!(
        browser
            .execution
            .adapter_hints
            .contains(&RuntimeAdapterHint::WebGpu)
    );
    assert_eq!(browser.execution.hot_kv_precision_bits, 4);

    assert_eq!(
        microcontroller.execution.primary_lane,
        ComputeLane::DiskBackedStreaming
    );
    assert_eq!(
        microcontroller.execution.fallback_lane,
        ComputeLane::CpuPortable
    );
    assert_eq!(
        microcontroller.execution.memory_mode,
        DeviceMemoryMode::MinimalDisk
    );
    assert_eq!(microcontroller.execution.adapter_hints.len(), 1);
    assert_eq!(microcontroller.execution.hot_kv_precision_bits, 4);
    assert!(microcontroller.local_kv_token_budget < embedded.local_kv_token_budget);

    assert_eq!(mobile.execution.primary_lane, ComputeLane::IntegratedGpu);
    assert!(
        mobile
            .execution
            .adapter_hints
            .contains(&RuntimeAdapterHint::Nnapi)
    );
    assert!(
        mobile
            .execution
            .adapter_hints
            .contains(&RuntimeAdapterHint::Qnn)
    );

    assert_eq!(
        multi_gpu.execution.primary_lane,
        ComputeLane::MultiAccelerator
    );
    assert_eq!(
        multi_gpu.execution.memory_mode,
        DeviceMemoryMode::DistributedSharded
    );
    assert!(
        multi_gpu
            .execution
            .adapter_hints
            .contains(&RuntimeAdapterHint::MultiDevice)
    );
    assert!(
        multi_gpu
            .execution
            .adapter_hints
            .contains(&RuntimeAdapterHint::PortableRust)
    );
    assert!(!multi_gpu.execution.allow_disk_spill);

    assert_eq!(uma.execution.primary_lane, ComputeLane::UnifiedMemoryGpu);
    assert_eq!(uma.execution.memory_mode, DeviceMemoryMode::UnifiedMemory);
    assert!(
        uma.execution
            .adapter_hints
            .contains(&RuntimeAdapterHint::Metal)
    );
}

#[test]
fn execution_budget_degrades_under_pressure() {
    let allocator = HardwareAllocator::new();
    let base = HierarchyWeights::new(0.30, 0.40, 0.30);

    let calm = allocator.plan(
        HardwareSnapshot::new(DeviceClass::Server, 0.10, 0.15, 0.20, 0.10),
        TaskProfile::Coding,
        1024,
        base,
    );
    let overloaded = allocator.plan(
        HardwareSnapshot::new(DeviceClass::Server, 0.95, 0.95, 0.90, 0.80),
        TaskProfile::Coding,
        1024,
        base,
    );

    assert!(calm.execution.max_parallel_chunks > overloaded.execution.max_parallel_chunks);
    assert!(calm.execution.kv_prefetch_blocks > overloaded.execution.kv_prefetch_blocks);
    assert_eq!(overloaded.execution.max_parallel_chunks, 1);
    assert_eq!(overloaded.execution.kv_prefetch_blocks, 1);
    assert_eq!(overloaded.execution.hot_kv_precision_bits, 4);
}

#[test]
fn load_accepts_percent_values() {
    let snapshot = HardwareSnapshot::new(DeviceClass::Auto, 75.0, 25.0, 50.0, 0.10);

    assert!((snapshot.cpu_load - 0.75).abs() < 0.0001);
    assert!((snapshot.gpu_load - 0.25).abs() < 0.0001);
    assert!((snapshot.ram_load - 0.50).abs() < 0.0001);
    assert!((snapshot.disk_load - 0.10).abs() < 0.0001);
}

#[test]
fn tier_compute_headroom_orders_device_capacity() {
    assert!(DeviceTier::Tiny.compute_headroom() < DeviceTier::Constrained.compute_headroom());
    assert!(DeviceTier::Constrained.compute_headroom() < DeviceTier::Balanced.compute_headroom());
    assert!(DeviceTier::Balanced.compute_headroom() < DeviceTier::Accelerated.compute_headroom());
    assert!(
        DeviceTier::Accelerated.compute_headroom() < DeviceTier::Distributed.compute_headroom()
    );
}

#[test]
fn probe_prefers_explicit_environment_profile() {
    let probe = HardwareProbe::new("windows", "x86_64", 8)
        .with_env("NOIRON_DEVICE_PROFILE", "rtx")
        .with_env("NOIRON_CPU_LOAD", "80");
    let report = probe.report();
    let snapshot = probe.snapshot();

    assert_eq!(report.device, DeviceClass::DiscreteGpu);
    assert_eq!(report.reason, "explicit-profile");
    let report_snapshot = report.snapshot();
    assert_eq!(report_snapshot.device, snapshot.device);
    assert!((report_snapshot.cpu_load - snapshot.cpu_load).abs() < 0.0001);
    assert!((report_snapshot.gpu_load - snapshot.gpu_load).abs() < 0.0001);
    assert!((report_snapshot.ram_load - snapshot.ram_load).abs() < 0.0001);
    assert!((report_snapshot.disk_load - snapshot.disk_load).abs() < 0.0001);
    assert_evidence(&report, "explicit_profile:discrete");
    assert_evidence(&report, "load:NOIRON_CPU_LOAD");
    assert_eq!(snapshot.device, DeviceClass::DiscreteGpu);
    assert!((snapshot.cpu_load - 0.80).abs() < 0.0001);
}

#[test]
fn unknown_environment_profile_falls_back_to_portable_cpu() {
    let report = HardwareProbe::new("windows", "x86_64", 8)
        .with_env("NOIRON_DEVICE_PROFILE", "future-device-sku")
        .with_env("WGPU_ADAPTER_NAME", "NVIDIA GeForce RTX")
        .report();

    assert_eq!(report.device, DeviceClass::CpuOnly);
    assert_eq!(report.reason, "unknown-explicit-profile");
    assert_evidence(&report, "unknown_explicit_profile:future-device-sku");
    assert_evidence(&report, "portable_cpu_fallback");
    assert!(!report.summary().contains("NVIDIA GeForce RTX"));
}

#[test]
fn probe_detects_mobile_arm_and_multi_gpu_targets() {
    let mobile = HardwareProbe::new("ios", "aarch64", 6).report();
    let vision = HardwareProbe::new("visionos", "aarch64", 8).report();
    let multi_gpu = HardwareProbe::new("linux", "x86_64", 32)
        .with_env("CUDA_VISIBLE_DEVICES", "0,1")
        .report();

    assert_eq!(mobile.device, DeviceClass::Mobile);
    assert_eq!(mobile.reason, "mobile-os");
    assert_evidence(&mobile, "mobile_os:ios");
    assert_eq!(vision.device, DeviceClass::Mobile);
    assert_eq!(vision.reason, "mobile-os");
    assert_evidence(&vision, "mobile_os:visionos");
    assert_eq!(multi_gpu.device, DeviceClass::MultiGpu);
    assert_eq!(multi_gpu.reason, "multi-accelerator");
    assert_eq!(multi_gpu.accelerator_count, 2);
    assert_evidence(&multi_gpu, "accelerators:2");
}

#[test]
fn probe_detects_unified_integrated_and_edge_targets() {
    let uma = HardwareProbe::new("macos", "aarch64", 10).report();
    let integrated = HardwareProbe::new("windows", "x86_64", 8)
        .with_env("WGPU_ADAPTER_NAME", "Intel Iris Xe Graphics")
        .report();
    let edge = HardwareProbe::new("linux", "aarch64", 8).report();

    assert_eq!(uma.device, DeviceClass::UnifiedMemory);
    assert_eq!(uma.reason, "unified-memory-default");
    assert_evidence(&uma, "apple_silicon_default");
    assert_eq!(integrated.device, DeviceClass::IntegratedGpu);
    assert_eq!(integrated.reason, "integrated-gpu-hint");
    assert_evidence_prefix(&integrated, "adapter:WGPU_ADAPTER_NAME:");
    assert_eq!(edge.device, DeviceClass::Edge);
    assert_eq!(edge.reason, "linux-arm-edge");
    assert_evidence(&edge, "linux_arm_edge");
}

#[test]
fn probe_detects_discrete_edge_and_tiny_fallback_targets() {
    let discrete = HardwareProbe::new("windows", "x86_64", 16)
        .with_env("WGPU_ADAPTER_NAME", "NVIDIA GeForce RTX 4090")
        .report();
    let jetson = HardwareProbe::new("linux", "aarch64", 8)
        .with_env("JETSON_MODEL_NAME", "Jetson Orin")
        .with_env("CUDA_VISIBLE_DEVICES", "0")
        .report();
    let wasm = HardwareProbe::new("wasi", "wasm32", 1).report();
    let tiny = HardwareProbe::new("espidf", "xtensa", 2).report();
    let npu = HardwareProbe::new("linux", "aarch64", 8)
        .with_env("NOIRON_NPU", "true")
        .report();
    let server = HardwareProbe::new("linux", "x86_64", 64).report();

    assert_eq!(discrete.device, DeviceClass::DiscreteGpu);
    assert_eq!(discrete.reason, "discrete-gpu-hint");
    assert_evidence_prefix(&discrete, "adapter:WGPU_ADAPTER_NAME:");
    assert_eq!(jetson.device, DeviceClass::Edge);
    assert_eq!(jetson.reason, "edge-hint");
    assert_evidence(&jetson, "env:JETSON_MODEL_NAME");
    assert_eq!(wasm.device, DeviceClass::BrowserWasm);
    assert_eq!(wasm.reason, "wasm-target");
    assert_evidence(&wasm, "wasm_target");
    assert_eq!(tiny.device, DeviceClass::Microcontroller);
    assert_eq!(tiny.reason, "microcontroller-target");
    assert_evidence(&tiny, "microcontroller_target");
    assert_eq!(npu.device, DeviceClass::NpuAccelerator);
    assert_eq!(npu.reason, "npu-hint");
    assert_evidence(&npu, "env_flag:NOIRON_NPU");
    assert_eq!(server.device, DeviceClass::Server);
    assert_eq!(server.reason, "high-cpu-count");
    assert_evidence(&server, "high_cpu_count");
}
