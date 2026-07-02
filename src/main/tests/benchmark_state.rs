use super::*;
use norion_cli::{parse_evidence_packet_args, run_evidence_packet};

#[test]
fn benchmark_self_evolution_admission_report_projects_preview_evidence() {
    let asset_dir = temp_asset_dir("self-evolution-admission-benchmark");
    fs::create_dir_all(&asset_dir).unwrap();
    let trace_path = asset_dir.join("benchmark.jsonl");
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;

    let summary = run_benchmark(&mut engine, &mut backend, &trace_path).unwrap();
    let gate_report = rust_norion::BenchmarkGateReport {
        passed: true,
        failures: Vec::new(),
    };
    let report = benchmark_self_evolution_admission_report(
        "benchmark:test",
        &engine,
        &summary,
        &gate_report,
        TaskProfile::General,
    );

    assert!(summary.len() > 0);
    assert_eq!(report.candidate_id, "benchmark:test");
    assert!(report.read_only);
    assert!(report.report_only);
    assert!(report.preview_only);
    assert!(report.benchmark_gate_passed);
    assert!(report.adaptive_preview_evidence_present);
    assert!(report.router_threshold_preview_ready);
    assert!(report.hierarchy_adjustment_preview_ready);
    assert!(!report.kv_fusion_policy_observation_preview_ready);
    assert_eq!(report.adaptive_preview_source_count, 2);
    assert!(report.adaptive_preview_read_only);
    assert!(report.adaptive_preview_report_only);
    assert!(report.adaptive_preview_preview_only);
    assert!(!report.adaptive_preview_write_allowed);
    assert!(!report.adaptive_preview_applied);
    assert!(!report.memory_store_write_allowed);
    assert!(!report.ndkv_write_allowed);
    assert!(!report.model_weight_write_allowed);
    assert!(!report.git_write_allowed);
    assert!(report.summary_line().contains("self_evolution_admission"));
    let json_line = report.json_line();
    assert!(json_line.contains("\"schema\":\"rust-norion-self-evolution-admission-v1\""));
    assert!(json_line.contains("\"candidate_id\":\"benchmark:test\""));
    assert!(json_line.contains("\"read_only\":true"));
    assert!(json_line.contains("\"report_only\":true"));
    assert!(json_line.contains("\"preview_only\":true"));
    assert!(json_line.contains("\"review_packet\":{"));
    assert!(json_line.contains("\"approval_review_packet_ids\":[\"tenant=local;workspace=default;session=interactive;lane=approval_packet;key=approval-review:benchmark:test\"]"));
    assert!(json_line.contains("\"approval_tokens_included\":false"));
    assert!(json_line.contains("\"evidence_count\":4"));
    assert!(json_line.contains("\"benchmark_gate\":{\"passed\":true"));
    assert!(json_line.contains("\"adaptive_preview\":{\"evidence_present\":true"));
    assert!(json_line.contains("\"source_count\":2"));
    assert!(json_line.contains("\"kv_fusion_policy_observation_ready\":false"));
    assert!(json_line.contains(
        "\"writes\":{\"mutation_allowed\":false,\"memory_store_allowed\":false,\"ndkv_allowed\":false,\"model_weight_allowed\":false,\"git_allowed\":false}"
    ));
    assert!(json_line.contains("\"blocked_reasons\":[]"));
    assert!(
        report
            .telemetry
            .iter()
            .any(|line| { line == "self_evolution_admission_adaptive_preview_source_count=2" })
    );
    assert!(
        report
            .telemetry
            .iter()
            .any(|line| line == "self_evolution_admission_review_packet_evidence_ids=4")
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn benchmark_records_rust_native_adapter_stream_evidence_from_real_run() {
    let asset_dir = temp_asset_dir("rust-native-adapter-stream-benchmark");
    fs::create_dir_all(&asset_dir).unwrap();
    let trace_path = asset_dir.join("benchmark.jsonl");
    let case_count = default_benchmark_cases().len();
    let mut engine = NoironEngine::new();
    let runtime =
        rust_norion::RustNativeModelRuntime::new(rust_norion::MockRustNativeAdapter::new())
            .with_cache_mode(rust_norion::ChunkedKvCacheMode::ChunkedCache);
    let mut backend = RuntimeBackend::new(runtime).with_max_tokens(32);

    let summary = run_benchmark(&mut engine, &mut backend, &trace_path).unwrap();
    let gate_report = summary.evaluate(&rust_norion::BenchmarkGate {
        min_average_quality: 0.0,
        min_average_reward: 0.0,
        min_runtime_adapter_cache_modes: Some(1),
        min_runtime_adapter_stream_trace_cases: Some(case_count),
        min_runtime_adapter_stream_gate_summary_cases: Some(case_count),
        min_runtime_adapter_stream_write_gate_cases: Some(case_count),
        min_runtime_adapter_stream_complete_cases: Some(case_count),
        max_runtime_adapter_contract_violations: Some(0),
        max_runtime_device_execution_violations: Some(0),
        ..rust_norion::BenchmarkGate::default()
    });
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

    assert_eq!(summary.len(), case_count);
    assert_eq!(summary.runtime_adapter_cache_mode_cases(), case_count);
    assert_eq!(summary.runtime_adapter_stream_trace_cases(), case_count);
    assert_eq!(
        summary.runtime_adapter_stream_gate_summary_cases(),
        case_count
    );
    assert_eq!(
        summary.runtime_adapter_stream_write_gate_cases(),
        case_count
    );
    assert_eq!(summary.runtime_adapter_stream_complete_cases(), case_count);
    assert!(
        summary
            .summary_line()
            .contains(&format!("runtime_adapter_stream_trace_cases={case_count}"))
    );
    assert!(summary.summary_line().contains(&format!(
        "runtime_adapter_stream_gate_summary_cases={case_count}"
    )));
    assert!(summary.summary_line().contains(&format!(
        "runtime_adapter_stream_write_gate_cases={case_count}"
    )));
    assert!(summary.summary_line().contains(&format!(
        "runtime_adapter_stream_complete_cases={case_count}"
    )));
    assert!(gate_report.passed, "{:?}", gate_report.failures);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, case_count);

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn benchmark_compares_rust_native_adapter_cache_modes_from_real_runs() {
    let asset_dir = temp_asset_dir("rust-native-adapter-cache-mode-benchmark");
    fs::create_dir_all(&asset_dir).unwrap();
    let case_count = default_benchmark_cases().len();
    let modes = [
        (rust_norion::ChunkedKvCacheMode::NoCache, "no_cache"),
        (
            rust_norion::ChunkedKvCacheMode::ChunkedCache,
            "chunked_cache",
        ),
        (
            rust_norion::ChunkedKvCacheMode::GenomeFiltered,
            "genome_filtered",
        ),
    ];
    let mut observed = Vec::new();

    for (mode, label) in modes {
        let trace_path = asset_dir.join(format!("{label}.jsonl"));
        let mut engine = NoironEngine::new();
        let runtime =
            rust_norion::RustNativeModelRuntime::new(rust_norion::MockRustNativeAdapter::new())
                .with_cache_mode(mode);
        let mut backend = RuntimeBackend::new(runtime).with_max_tokens(32);

        let summary = run_benchmark(&mut engine, &mut backend, &trace_path).unwrap();
        let gate_report = summary.evaluate(&rust_norion::BenchmarkGate {
            min_average_quality: 0.0,
            min_average_reward: 0.0,
            min_runtime_adapter_cache_modes: Some(1),
            min_runtime_adapter_stream_trace_cases: Some(case_count),
            min_runtime_adapter_stream_gate_summary_cases: Some(case_count),
            min_runtime_adapter_stream_write_gate_cases: Some(case_count),
            min_runtime_adapter_stream_complete_cases: Some(case_count),
            max_runtime_adapter_contract_violations: Some(0),
            max_runtime_device_execution_violations: Some(0),
            ..rust_norion::BenchmarkGate::default()
        });
        let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

        assert_eq!(summary.len(), case_count);
        assert_eq!(summary.runtime_adapter_cache_modes_csv(), label);
        assert_eq!(summary.runtime_adapter_cache_mode_cases(), case_count);
        assert!(gate_report.passed, "{:?}", gate_report.failures);
        assert!(trace_report.passed, "{:?}", trace_report.failures);
        assert_eq!(trace_report.checked_lines, case_count);
        observed.push((
            label,
            summary.total_runtime_kv_imported(),
            summary.total_runtime_kv_exported(),
            summary.total_runtime_kv_segments_included(),
            summary.total_runtime_kv_segments_skipped(),
        ));
    }

    assert_eq!(
        observed
            .iter()
            .map(|(mode, _, _, _, _)| *mode)
            .collect::<Vec<_>>(),
        vec!["no_cache", "chunked_cache", "genome_filtered"]
    );
    assert_eq!(observed[0].1, 0);
    assert_eq!(observed[0].2, 0);
    assert_eq!(observed[0].3, 0);
    assert!(observed[0].4 > 0);
    assert!(observed[1].1 > observed[0].1);
    assert!(observed[1].2 > observed[0].2);
    assert!(observed[1].3 > observed[0].3);
    assert!(observed[2].1 > observed[0].1);
    assert!(observed[2].2 > observed[0].2);
    assert!(observed[2].3 > observed[0].3);

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn benchmark_dispatch_appends_self_evolution_admission_trace_packet() {
    let asset_dir = temp_asset_dir("benchmark-dispatch-admission-trace");
    fs::create_dir_all(&asset_dir).unwrap();
    let memory = asset_dir.join("memory.ndkv");
    let experience = asset_dir.join("experience.ndkv");
    let adaptive = asset_dir.join("adaptive.ndkv");
    let trace_path = asset_dir.join("benchmark.jsonl");
    let args = Args::parse(vec![
        "--benchmark".to_owned(),
        trace_path.display().to_string(),
        "--benchmark-gate".to_owned(),
        "--trace-schema-gate".to_owned(),
        trace_path.display().to_string(),
        "--memory".to_owned(),
        memory.display().to_string(),
        "--experience".to_owned(),
        experience.display().to_string(),
        "--adaptive".to_owned(),
        adaptive.display().to_string(),
    ]);

    dispatch::run(args).unwrap();
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(
        trace_report.checked_lines,
        default_benchmark_cases().len() + 1
    );
    assert_eq!(trace_report.self_evolution_admission_events, 1);
    assert_eq!(
        trace_report.self_evolution_admission_admitted
            + trace_report.self_evolution_admission_blocked,
        1
    );
    assert_eq!(trace_report.self_evolution_admission_review_packets, 1);
    assert!(trace_report.self_evolution_admission_evidence_ids >= 3);
    assert_eq!(
        trace_report.self_evolution_admission_missing_review_packet_refs,
        0
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn benchmark_all_devices_runs_every_explicit_profile() {
    let asset_dir = temp_asset_dir("all-device-benchmark");
    fs::create_dir_all(&asset_dir).unwrap();
    let trace_path = asset_dir.join("benchmark.jsonl");
    let args = Args::parse(vec![
        "--benchmark".to_owned(),
        trace_path.display().to_string(),
        "--benchmark-all-devices".to_owned(),
        "--benchmark-gate".to_owned(),
        "--benchmark-min-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--benchmark-max-drift-blocks".to_owned(),
        "0".to_owned(),
        "--benchmark-max-drift-rollbacks".to_owned(),
        "0".to_owned(),
        "--device".to_owned(),
        "cpu".to_owned(),
    ]);
    let mut engine = NoironEngine::new();
    configure_engine(&mut engine, &args);
    let mut backend = HeuristicBackend;

    let summary = run_benchmark_for_args(&mut engine, &mut backend, &args, &trace_path).unwrap();
    let gate_report = summary.evaluate(&args.benchmark_gate());
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

    assert!(args.benchmark_all_devices);
    assert_eq!(
        summary.len(),
        DeviceClass::explicit_profiles().len() * default_benchmark_cases().len()
    );
    assert_eq!(
        summary.explicit_device_profiles_covered(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(summary.missing_explicit_device_profiles().is_empty());
    assert!(summary.results().iter().any(|result| {
        result.device == DeviceClass::Microcontroller && result.name.starts_with("microcontroller_")
    }));
    assert!(summary.summary_line().contains("device_profiles=12"));
    assert!(gate_report.passed, "{:?}", gate_report.failures);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, summary.len());

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn benchmark_all_devices_can_gate_recursive_coverage_per_profile() {
    let asset_dir = temp_asset_dir("all-device-recursive-benchmark");
    fs::create_dir_all(&asset_dir).unwrap();
    let trace_path = asset_dir.join("benchmark.jsonl");
    let args = Args::parse(vec![
        "--benchmark".to_owned(),
        trace_path.display().to_string(),
        "--benchmark-all-devices".to_owned(),
        "--benchmark-gate".to_owned(),
        "--benchmark-min-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--benchmark-min-recursive-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--benchmark-min-recursive-cases".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--benchmark-max-drift-blocks".to_owned(),
        "0".to_owned(),
        "--benchmark-max-drift-rollbacks".to_owned(),
        "0".to_owned(),
        "--native-window".to_owned(),
        "64".to_owned(),
        "--chunk-tokens".to_owned(),
        "32".to_owned(),
        "--chunk-overlap".to_owned(),
        "8".to_owned(),
        "--device".to_owned(),
        "cpu".to_owned(),
    ]);
    let mut engine = NoironEngine::new();
    configure_engine(&mut engine, &args);
    let mut backend = HeuristicBackend;

    let summary = run_benchmark_for_args(&mut engine, &mut backend, &args, &trace_path).unwrap();
    let gate_report = summary.evaluate(&args.benchmark_gate());
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

    assert_eq!(
        summary.explicit_device_profiles_covered(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        summary.recursive_device_profiles_covered(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        summary.recursive_cases(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(
        summary
            .summary_line()
            .contains("recursive_device_profiles=12")
    );
    assert!(gate_report.passed, "{:?}", gate_report.failures);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, summary.len());

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_reference_kernel_all_devices_gates_recursive_runtime_coverage() {
    assert_production_kernel_all_devices_gate_recursive_runtime_coverage(
        "--production-reference-kernel",
        "production-reference-all-device-recursive",
    );
}

#[test]
fn production_local_kernel_all_devices_gates_recursive_runtime_coverage() {
    assert_production_kernel_all_devices_gate_recursive_runtime_coverage(
        "--production-local-kernel",
        "production-local-all-device-recursive",
    );
}

#[test]
fn persistent_roundtrip_all_devices_verifies_runtime_kv_namespace_reuse() {
    let asset_dir = temp_asset_dir("roundtrip-all-devices");
    fs::create_dir_all(&asset_dir).unwrap();
    let args = Args::parse(vec![
        "--benchmark-roundtrip".to_owned(),
        "--benchmark-all-devices".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "--profile".to_owned(),
        "coding".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "32".to_owned(),
        "Verify persistent runtime KV reuse across every supported device".to_owned(),
    ]);

    let report = run_persistent_roundtrip_all_devices(&args).unwrap();

    assert!(args.benchmark_roundtrip);
    assert!(args.benchmark_all_devices);
    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(
        report.covered_devices(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.device_reports.len(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(report.missing_devices().is_empty());
    assert!(report.failed_devices().is_empty());
    assert!(report.summary_line().contains("devices=12"));
    assert!(report.second_compute_budget_saved_tokens() > 0);
    assert!(report.second_compute_budget_avoided_tokens() > 0);
    assert!(report.second_compute_budget_kv_lookups_skipped() > 0);
    assert!(
        report
            .summary_line()
            .contains("second_compute_budget_saved_tokens=")
    );
    assert!(
        report
            .summary_line()
            .contains("second_compute_budget_avoided_tokens=")
    );
    assert!(
        report
            .summary_line()
            .contains("second_compute_budget_kv_lookups_skipped=")
    );
    assert!(report.device_reports.iter().all(|device_report| {
        device_report.report.first_runtime_kv_namespace_preserved
            && device_report.report.second_used_runtime_kv_memory
            && device_report
                .report
                .second_imported_runtime_kv_from_namespace
            && device_report.report.second_runtime_adapter_best_adapter
                == device_report.report.second_runtime_selected_adapter
            && device_report.report.second_compute_budget_saved_tokens > 0
            && device_report.report.second_compute_budget_avoided_tokens > 0
            && device_report
                .report
                .second_compute_budget_kv_lookups_skipped
                > 0
            && device_report.report.negative_gate_evidence.passed()
            && device_report
                .report
                .summary_line()
                .contains("negative_digest_only=true")
    }));
    assert!(
        device_scoped_path(&args.memory_path, DeviceClass::CpuOnly)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("memory.cpu.ndkv")
    );
    assert!(device_scoped_path(&args.memory_path, DeviceClass::CpuOnly).exists());
    assert!(device_scoped_path(&args.memory_path, DeviceClass::Mobile).exists());

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn state_inspection_all_devices_gates_roundtrip_state_files() {
    let asset_dir = temp_asset_dir("inspect-all-devices");
    fs::create_dir_all(&asset_dir).unwrap();
    let roundtrip_args = Args::parse(vec![
        "--benchmark-roundtrip".to_owned(),
        "--benchmark-all-devices".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "--profile".to_owned(),
        "coding".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "32".to_owned(),
        "Create inspectable runtime KV state for every supported device".to_owned(),
    ]);
    let roundtrip = run_persistent_roundtrip_all_devices(&roundtrip_args).unwrap();
    assert!(roundtrip.passed, "{:?}", roundtrip.failures);

    let inspect_args = Args::parse(vec![
        "--inspect-state".to_owned(),
        "--benchmark-all-devices".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "--inspect-min-memories".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-memories".to_owned(),
        "1".to_owned(),
        "--inspect-min-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-model-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-adapter-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-max-runtime-adapter-selection-mismatches".to_owned(),
        "0".to_owned(),
        "--inspect-min-runtime-forward-energy-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-influence-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-precision-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-max-runtime-kv-precision-mismatches".to_owned(),
        "0".to_owned(),
        "--inspect-min-runtime-device-execution-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-layer-mode-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-all-layer-mode-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-global-layers".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-local-window-layers".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-convolutional-fusion-layers".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-import-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-export-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-memory-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-model-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-adapter-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-forward-energy-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-kv-influence-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-kv-precision-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-device-execution-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-layer-mode-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-all-layer-mode-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-kv-import-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-kv-export-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-reflection-issue-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-revision-action-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-live-memory-feedback-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-evolution-replay-run-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-evolution-replay-item-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-evolution-memory-update-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-evolution-memory-updates".to_owned(),
        "1".to_owned(),
        "--inspect-require-runtime-kv-dimensions".to_owned(),
    ]);
    let report = run_state_inspection_all_devices(&inspect_args).unwrap();

    assert!(inspect_args.inspect_state);
    assert!(inspect_args.inspect_gate);
    assert!(inspect_args.benchmark_all_devices);
    assert!(report.passed(), "{:?}", report.failures);
    assert_eq!(
        report.covered_devices(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.runtime_kv_memory_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.runtime_model_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.runtime_adapter_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.runtime_forward_energy_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.runtime_kv_influence_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.runtime_device_execution_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.runtime_layer_mode_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.runtime_all_layer_mode_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.runtime_kv_import_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.runtime_kv_export_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.reflection_issue_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.revision_action_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.live_memory_feedback_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.evolution_replay_run_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.evolution_replay_item_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.evolution_memory_update_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(report.missing_devices().is_empty());
    assert!(report.failed_devices().is_empty());
    assert!(
        report
            .summary_line()
            .contains("state_inspection_matrix_gate: passed=true")
    );
    assert!(report.device_reports.iter().all(|device_report| {
        device_report.report.passed() && device_report.report.summary_line().contains("passed=true")
    }));
    assert!(device_scoped_path(&inspect_args.memory_path, DeviceClass::Server).exists());

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn roundtrip_and_inspect_state_can_chain_single_device_gate() {
    let asset_dir = temp_asset_dir("roundtrip-inspect-single");
    fs::create_dir_all(&asset_dir).unwrap();
    let args = Args::parse(vec![
        "--benchmark-roundtrip".to_owned(),
        "--inspect-state".to_owned(),
        "--inspect-gate".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "--profile".to_owned(),
        "coding".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "32".to_owned(),
        "--inspect-min-runtime-kv-memories".to_owned(),
        "1".to_owned(),
        "--inspect-min-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-model-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-adapter-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-max-runtime-adapter-selection-mismatches".to_owned(),
        "0".to_owned(),
        "--inspect-min-runtime-forward-energy-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-influence-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-precision-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-max-runtime-kv-precision-mismatches".to_owned(),
        "0".to_owned(),
        "--inspect-min-runtime-device-execution-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-import-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-export-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-live-memory-feedback-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-live-memory-feedback-updates".to_owned(),
        "1".to_owned(),
        "--inspect-min-reflection-issue-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-revision-action-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-live-memory-feedback-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-evolution-memory-updates".to_owned(),
        "1".to_owned(),
        "--inspect-require-runtime-kv-dimensions".to_owned(),
        "Chain roundtrip into inspect gate for self-owned runtime state".to_owned(),
    ]);

    let roundtrip = run_persistent_roundtrip(&args).unwrap();
    let inspect = run_state_inspection(&args).unwrap();
    let gate = inspect.evaluate(&args.state_inspection_gate());

    assert!(args.benchmark_roundtrip);
    assert!(args.inspect_state);
    assert!(args.inspect_gate);
    assert!(roundtrip.passed, "{:?}", roundtrip.failures);
    assert!(roundtrip.second_compute_budget_saved_tokens > 0);
    assert!(roundtrip.second_compute_budget_avoided_tokens > 0);
    assert!(roundtrip.second_compute_budget_kv_lookups_skipped > 0);
    assert!(roundtrip.negative_gate_evidence.passed());
    assert!(
        roundtrip
            .summary_line()
            .contains("second_compute_budget_saved_tokens=")
    );
    assert!(
        roundtrip
            .summary_line()
            .contains("second_compute_budget_avoided_tokens=")
    );
    assert!(
        roundtrip
            .summary_line()
            .contains("second_compute_budget_kv_lookups_skipped=")
    );
    assert!(
        roundtrip
            .summary_line()
            .contains("negative_tenant_scope_write_denied=true")
    );
    assert!(
        roundtrip
            .summary_line()
            .contains("negative_tenant_scope_mode=local_single_user_preview")
    );
    assert!(
        roundtrip
            .summary_line()
            .contains("negative_tenant_scope_denial_reason=cross_tenant_scope_rejected")
    );
    assert!(gate.passed(), "{:?}", gate.failures);
    assert!(args.memory_path.exists());
    assert!(args.experience_path.exists());
    assert!(args.adaptive_path.exists());

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn issue30_clean_checkout_demo_writes_digest_only_evidence_packet() {
    let asset_dir = temp_asset_dir("issue30-clean-checkout-demo");
    fs::create_dir_all(&asset_dir).unwrap();
    let clean_git_worktree = asset_dir.join("clean-git-worktree");
    fs::create_dir_all(&clean_git_worktree).unwrap();
    let git_init = std::process::Command::new("git")
        .arg("-C")
        .arg(&clean_git_worktree)
        .args(["init", "--quiet"])
        .output()
        .expect("git init should run for the issue 30 clean-checkout evidence test");
    assert!(git_init.status.success());
    let git_branch = std::process::Command::new("git")
        .arg("-C")
        .arg(&clean_git_worktree)
        .args([
            "checkout",
            "-b",
            "codex/issue-30-roundtrip-compute-budget-evidence",
        ])
        .output()
        .expect("git branch should run for the issue 30 clean-checkout evidence test");
    assert!(git_branch.status.success());
    fs::write(clean_git_worktree.join("fixture.txt"), "issue30 fixture\n").unwrap();
    for args in [
        ["config", "user.email", "issue30@example.invalid"].as_slice(),
        ["config", "user.name", "Issue 30 Fixture"].as_slice(),
        ["add", "fixture.txt"].as_slice(),
        ["commit", "--quiet", "-m", "fixture"].as_slice(),
    ] {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&clean_git_worktree)
            .args(args)
            .output()
            .expect("git fixture command should run for the issue 30 clean-checkout evidence test");
        assert!(
            output.status.success(),
            "git fixture command failed: {args:?}"
        );
    }
    let args = Args::parse(vec![
        "--benchmark-roundtrip".to_owned(),
        "--inspect-state".to_owned(),
        "--inspect-gate".to_owned(),
        "--trace".to_owned(),
        asset_dir.join("issue30-trace.jsonl").display().to_string(),
        "--trace-schema-gate".to_owned(),
        asset_dir.join("issue30-trace.jsonl").display().to_string(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "--profile".to_owned(),
        "coding".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "32".to_owned(),
        "--inspect-min-runtime-kv-memories".to_owned(),
        "1".to_owned(),
        "--inspect-min-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-model-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-adapter-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-max-runtime-adapter-selection-mismatches".to_owned(),
        "0".to_owned(),
        "--inspect-min-runtime-forward-energy-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-influence-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-precision-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-max-runtime-kv-precision-mismatches".to_owned(),
        "0".to_owned(),
        "--inspect-min-runtime-device-execution-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-import-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-export-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-live-memory-feedback-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-live-memory-feedback-updates".to_owned(),
        "1".to_owned(),
        "--inspect-require-runtime-kv-dimensions".to_owned(),
    ]);

    let roundtrip = run_persistent_roundtrip(&args).unwrap();
    let inspect = run_state_inspection(&args).unwrap();
    let gate = inspect.evaluate(&args.state_inspection_gate());
    let trace_report =
        evaluate_trace_schema_jsonl(args.trace_schema_gate_path.as_ref().unwrap()).unwrap();
    assert!(roundtrip.passed, "{:?}", roundtrip.failures);
    assert!(roundtrip.second_compute_budget_saved_tokens > 0);
    assert!(roundtrip.second_compute_budget_avoided_tokens > 0);
    assert!(roundtrip.second_compute_budget_kv_lookups_skipped > 0);
    assert!(roundtrip.second_compute_budget_anchor_count > 0);
    assert!(roundtrip.second_compute_budget_anchors_preserved);
    assert_eq!(
        roundtrip.second_compute_budget_anchors_preserved_count,
        roundtrip.second_compute_budget_anchor_count
    );
    assert!(roundtrip.second_quality >= 0.50);
    assert!(gate.passed(), "{:?}", gate.failures);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert!(trace_report.reasoning_genome_events >= 2);
    assert_eq!(trace_report.self_evolution_admission_events, 1);
    assert_eq!(trace_report.self_evolution_admission_review_packets, 1);
    assert!(trace_report.self_evolution_admission_evidence_ids >= 3);
    assert_eq!(
        trace_report.self_evolution_admission_missing_review_packet_refs,
        0
    );
    assert_eq!(trace_report.reasoning_genome_write_allowed, 0);
    assert_eq!(trace_report.reasoning_genome_splice_write_allowed, 0);

    let rc_sha_output = std::process::Command::new("git")
        .arg("-C")
        .arg(&clean_git_worktree)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse HEAD should run for the issue 30 clean-checkout evidence test");
    assert!(rc_sha_output.status.success());
    let rc_sha = String::from_utf8(rc_sha_output.stdout)
        .expect("git rev-parse HEAD should emit utf-8")
        .trim()
        .to_owned();
    let rc_sha_field = format!("rc_sha={rc_sha}");
    let release_review_path = asset_dir.join("issue30-release-review.txt");
    fs::write(
        &release_review_path,
        "pr=428 review=REVIEW_REQUIRED checks=passed branch_protection=present\npr=429 review=REVIEW_REQUIRED checks=passed branch_protection=present\n",
    )
    .unwrap();
    let issue_state_path = asset_dir.join("issue30-issue-state.txt");
    fs::write(
        &issue_state_path,
        "issue=31 state=open final_signoff=false\nissue=19 state=open runtime_surface_closed=false runtime_surface_merged_prs=#290,#291,#292,#293,#296,#307,#308,#309 runtime_counters_pr=#429 runtime_counters_ready=false runtime_counters_state=head_6f049dd_checks_green_review_required_unmerged runtime_surface_blocker=#429:REVIEW_REQUIRED\nissue=30 state=open close_allowed=false\n",
    )
    .unwrap();
    let demo_proof_path = asset_dir.join("issue30-demo-proof.txt");
    fs::write(
        &demo_proof_path,
        "integration_test=issue30_clean_checkout_demo_writes_digest_only_evidence_packet dispatch_test=issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate dispatch_path=dispatch::run trace_schema_gate_executed=true\n",
    )
    .unwrap();
    let roundtrip_proof_path = asset_dir.join("issue30-roundtrip-proof.txt");
    fs::write(
        &roundtrip_proof_path,
        format!("{}\n", roundtrip.summary_line()),
    )
    .unwrap();
    let entry_chain_evidence = rust_norion::issue30_entry_chain_evidence_line();
    let issue377_evidence = rust_norion::issue30_problem_hypothesis_evidence_line();
    let issue30_context_path = asset_dir.join("issue30-context-proof.txt");
    fs::write(
        &issue30_context_path,
        format!("{entry_chain_evidence}\n{issue377_evidence}\n"),
    )
    .unwrap();
    let state_files_path = asset_dir.join("issue30-state-files.txt");
    fs::write(
        &state_files_path,
        format!(
            "memory={} experience={} adaptive={}\n",
            args.memory_path.display(),
            args.experience_path.display(),
            args.adaptive_path.display()
        ),
    )
    .unwrap();
    let raw_evidence = format!(
        "issue30_clean_checkout_demo clean_checkout=true live_model_required=false private_state_required=false prompt_digest_ref=redaction-digest:issue30-default-prompt\nhidden_cot=private chain-of-thought\n{}\n{}\nreasoning_genome_events={} reasoning_genome_write_allowed={} reasoning_genome_splice_write_allowed={} self_evolution_admission_events={} self_evolution_admission_review_packets={} self_evolution_admission_evidence_ids={}\n",
        gate.summary_line(),
        trace_report.summary_line(),
        trace_report.reasoning_genome_events,
        trace_report.reasoning_genome_write_allowed,
        trace_report.reasoning_genome_splice_write_allowed,
        trace_report.self_evolution_admission_events,
        trace_report.self_evolution_admission_review_packets,
        trace_report.self_evolution_admission_evidence_ids,
    );
    let raw_path = asset_dir.join("issue30-evidence.raw.txt");
    fs::write(&raw_path, raw_evidence).unwrap();
    let command = "cargo run -- --benchmark-roundtrip --inspect-state --inspect-gate --trace \"$STATE_DIR/issue30-trace.jsonl\" --trace-schema-gate \"$STATE_DIR/issue30-trace.jsonl\" --memory \"$STATE_DIR/memory.ndkv\" --experience \"$STATE_DIR/experience.ndkv\" --adaptive \"$STATE_DIR/adaptive.ndkv\" --profile coding --runtime-kv-exchange --runtime-layers 6 --runtime-hidden-size 64 --runtime-attention-heads 4 --runtime-kv-heads 2 --runtime-local-window 32 --inspect-min-runtime-kv-memories 1 --inspect-min-experiences 1 --inspect-min-runtime-model-experiences 1 --inspect-min-runtime-adapter-experiences 1 --inspect-max-runtime-adapter-selection-mismatches 0 --inspect-min-runtime-forward-energy-experiences 1 --inspect-min-runtime-kv-influence-experiences 1 --inspect-min-runtime-kv-precision-experiences 1 --inspect-max-runtime-kv-precision-mismatches 0 --inspect-min-runtime-device-execution-experiences 1 --inspect-min-runtime-kv-import-experiences 1 --inspect-min-runtime-kv-export-experiences 1 --inspect-min-live-memory-feedback-experiences 1 --inspect-min-live-memory-feedback-updates 1 --inspect-require-runtime-kv-dimensions";
    let raw_path_arg = raw_path.display().to_string();
    let clean_git_worktree_arg = clean_git_worktree.display().to_string();
    let release_review_path_arg = release_review_path.display().to_string();
    let issue_state_path_arg = issue_state_path.display().to_string();
    let demo_proof_path_arg = demo_proof_path.display().to_string();
    let roundtrip_proof_path_arg = roundtrip_proof_path.display().to_string();
    let issue30_context_path_arg = issue30_context_path.display().to_string();
    let state_files_path_arg = state_files_path.display().to_string();
    let memory_path_reject = args.memory_path.display().to_string();
    let experience_path_reject = args.experience_path.display().to_string();
    let adaptive_path_reject = args.adaptive_path.display().to_string();
    let config = parse_evidence_packet_args(
        [
            "evidence-packet",
            "--issue",
            "30",
            "--commit",
            rc_sha.as_str(),
            "--command",
            command,
            "--gate",
            "passed",
            "--input",
            raw_path_arg.as_str(),
            "--git-worktree",
            clean_git_worktree_arg.as_str(),
            "--release-review-input",
            release_review_path_arg.as_str(),
            "--issue-state-input",
            issue_state_path_arg.as_str(),
            "--demo-proof-input",
            demo_proof_path_arg.as_str(),
            "--roundtrip-proof-input",
            roundtrip_proof_path_arg.as_str(),
            "--issue30-context-input",
            issue30_context_path_arg.as_str(),
            "--state-files-input",
            state_files_path_arg.as_str(),
            "--require",
            "clean_checkout=true",
            "--require",
            "live_model_required=false",
            "--require",
            "private_state_required=false",
            "--require",
            rc_sha_field.as_str(),
            "--require",
            "rc_sha_source=git_rev_parse",
            "--require",
            "rc_branch=codex/issue-30-roundtrip-compute-budget-evidence",
            "--require",
            "rc_branch_source=git_branch",
            "--require",
            "rc_prs=#428,#429",
            "--require",
            "rc_prs_source=release_review_input",
            "--require",
            "dirty_worktree=false",
            "--require",
            "dirty_worktree_source=git_status",
            "--require",
            "release_review_ready=false",
            "--require",
            "release_relevant_prs=#428,#429",
            "--require",
            "release_review_blockers=#428:REVIEW_REQUIRED,#429:REVIEW_REQUIRED",
            "--require",
            "release_review_source=release_review_input",
            "--require",
            "issue31_final_signoff_present=false",
            "--require",
            "issue31_final_signoff_source=issue_state_input",
            "--require",
            "issue19_runtime_surface_closed=false",
            "--require",
            "issue19_runtime_surface_merged_prs=#290,#291,#292,#293,#296,#307,#308,#309",
            "--require",
            "issue19_runtime_counters_pr=#429",
            "--require",
            "issue19_runtime_counters_ready=false",
            "--require",
            "issue19_runtime_counters_state=head_6f049dd_checks_green_review_required_unmerged",
            "--require",
            "issue19_runtime_surface_blocker=#429:REVIEW_REQUIRED",
            "--require",
            "issue19_runtime_surface_source=issue_state_input",
            "--require",
            "issue30_close_allowed=false",
            "--require",
            "issue30_close_allowed_source=issue_state_input",
            "--require",
            "issue30_demo_integration_test=issue30_clean_checkout_demo_writes_digest_only_evidence_packet",
            "--require",
            "issue30_demo_dispatch_test=issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate",
            "--require",
            "issue30_demo_dispatch_path=dispatch::run",
            "--require",
            "issue30_demo_trace_schema_gate_executed=true",
            "--require",
            "issue30_demo_source=demo_proof_input",
            "--require",
            "prompt_digest_ref=redaction-digest:issue30-default-prompt",
            "--require=--trace-schema-gate",
            "--require",
            "persistent_roundtrip: passed=true",
            "--require",
            "state_inspection_gate: passed=true",
            "--require",
            "trace_schema_gate: passed=true",
            "--require",
            "reasoning_genome_events=",
            "--require",
            "reasoning_genome_write_allowed=0",
            "--require",
            "reasoning_genome_splice_write_allowed=0",
            "--require",
            "self_evolution_admission_events=1",
            "--require",
            "self_evolution_admission_review_packets=1",
            "--require",
            "self_evolution_admission_evidence_ids=",
            "--require",
            "self_evolution_admission_missing_review_packet_refs=0",
            "--require",
            "issue30_environment_pressure_present=true",
            "--require",
            "issue30_pollution_event_id=redaction-digest:",
            "--require",
            "issue385_self_ontology_body_present=true",
            "--require",
            "issue385_body_state_id=redaction-digest:",
            "--require",
            "issue375_pre_reasoning_genome_isa_present=true",
            "--require",
            "issue375_reasoning_frame_id=redaction-digest:",
            "--require",
            "issue30_backend_action=deterministic_runtime_kv_roundtrip",
            "--require",
            "issue379_control_candidate_preview_only=true",
            "--require",
            "issue379_action_vocab_mask_preview=true",
            "--require",
            "issue379_signal_saliency_bias_preview=true",
            "--require",
            "issue377_problem_finding_present=true",
            "--require",
            "issue377_problem_finding_id=redaction-digest:",
            "--require",
            "issue377_hypothesis_candidate_present=true",
            "--require",
            "issue377_hypothesis_candidate_id=redaction-digest:",
            "--require",
            "issue377_problem_hypothesis_link=redaction-digest:",
            "--require",
            "issue377_admission_decision=preview_only",
            "--require",
            "issue30_context_source=issue30_context_input",
            "--require",
            "second_compute_budget_saved_tokens=",
            "--require",
            "second_compute_budget_avoided_tokens=",
            "--require",
            "second_compute_budget_kv_lookups_skipped=",
            "--require",
            "second_approved_experience_reuse_digest=redaction-digest:",
            "--require",
            "second_compute_budget_anchor_count=",
            "--require",
            "second_compute_budget_anchors_preserved=true",
            "--require",
            "second_compute_budget_anchors_preserved_count=",
            "--require",
            "second_quality=",
            "--require",
            "first_drift=watch",
            "--require",
            "second_drift=watch",
            "--require",
            "failures=0",
            "--require",
            "negative_unauthorized_write_allowed=false",
            "--require",
            "negative_durable_write_allowed=false",
            "--require",
            "negative_memory_write_allowed=false",
            "--require",
            "negative_genome_write_allowed=false",
            "--require",
            "negative_self_evolution_write_allowed=false",
            "--require",
            "negative_polluted_evidence_blocked=true",
            "--require",
            "negative_polluted_evidence_quarantined=true",
            "--require",
            "negative_bad_candidate_held_or_rolled_back=true",
            "--require",
            "negative_bad_candidate_digest=redaction-digest:",
            "--require",
            "negative_bad_candidate_decision=hold_then_rollback",
            "--require",
            "negative_rollback_anchor_present=true",
            "--require",
            "negative_rollback_anchor_evidence_id=issue-30-roundtrip-negative-gate-hold",
            "--require",
            "negative_rollback_anchor_digest=redaction-digest:",
            "--require",
            "negative_tenant_scope_write_denied=true",
            "--require",
            "negative_tenant_scope_mode=local_single_user_preview",
            "--require",
            "negative_tenant_scope_actor=fnv64:",
            "--require",
            "negative_tenant_scope_target=fnv64:",
            "--require",
            "negative_tenant_scope_denial_lane=self_evolving_memory",
            "--require",
            "negative_tenant_scope_denial_reason=cross_tenant_scope_rejected",
            "--require",
            "negative_single_tenant_preview=true",
            "--require",
            "negative_provenance_license_redaction_passed=true",
            "--require",
            "negative_digest_only=true",
            "--require",
            "issue30_roundtrip_source=roundtrip_proof_input",
            "--require",
            "memory_file_exists=true",
            "--require",
            "experience_file_exists=true",
            "--require",
            "adaptive_file_exists=true",
            "--require",
            "state_files_source=state_files_input",
            "--reject",
            memory_path_reject.as_str(),
            "--reject",
            experience_path_reject.as_str(),
            "--reject",
            adaptive_path_reject.as_str(),
            "--reject",
            args.prompt.as_str(),
            "--reject",
            "raw prompt",
            "--reject",
            "raw answer",
            "--reject",
            "chain-of-thought",
            "--reject",
            "C:\\Users",
            "--reject",
            "AppData",
            "--reject",
            "Design a Rust Noiron prototype",
            "--reject",
            "reuse_response",
            "--reject",
            "sk-secret",
            "--reject",
            "ghp_",
        ]
        .into_iter()
        .skip(1),
    )
    .unwrap();
    let packet = run_evidence_packet(&config).unwrap();

    assert!(packet.contains("## Evidence packet for #30"));
    assert!(packet.contains(&format!("- commit: {rc_sha}")));
    assert!(packet.contains("clean_checkout=true"));
    assert!(packet.contains("live_model_required=false"));
    assert!(packet.contains("private_state_required=false"));
    assert!(packet.contains(&rc_sha_field));
    assert!(packet.contains("rc_sha_source=git_rev_parse"));
    assert!(packet.contains("rc_branch=codex/issue-30-roundtrip-compute-budget-evidence"));
    assert!(packet.contains("rc_branch_source=git_branch"));
    assert!(packet.contains("rc_prs=#428,#429"));
    assert!(packet.contains("rc_prs_source=release_review_input"));
    assert!(packet.contains("dirty_worktree=false dirty_worktree_source=git_status"));
    assert!(packet.contains("release_review_ready=false"));
    assert!(packet.contains("release_relevant_prs=#428,#429"));
    assert!(packet.contains("release_review_blockers=#428:REVIEW_REQUIRED,#429:REVIEW_REQUIRED"));
    assert!(packet.contains("release_review_source=release_review_input"));
    assert!(packet.contains("issue31_final_signoff_present=false"));
    assert!(packet.contains("issue31_final_signoff_source=issue_state_input"));
    assert!(packet.contains("issue19_runtime_surface_closed=false"));
    assert!(
        packet
            .contains("issue19_runtime_surface_merged_prs=#290,#291,#292,#293,#296,#307,#308,#309")
    );
    assert!(packet.contains("issue19_runtime_counters_pr=#429"));
    assert!(packet.contains("issue19_runtime_counters_ready=false"));
    assert!(packet.contains(
        "issue19_runtime_counters_state=head_6f049dd_checks_green_review_required_unmerged"
    ));
    assert!(packet.contains("issue19_runtime_surface_blocker=#429:REVIEW_REQUIRED"));
    assert!(packet.contains("issue19_runtime_surface_source=issue_state_input"));
    assert!(packet.contains("issue30_close_allowed=false"));
    assert!(packet.contains("issue30_close_allowed_source=issue_state_input"));
    assert!(packet.contains(
        "issue30_demo_integration_test=issue30_clean_checkout_demo_writes_digest_only_evidence_packet"
    ));
    assert!(packet.contains(
        "issue30_demo_dispatch_test=issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate"
    ));
    assert!(packet.contains("issue30_demo_dispatch_path=dispatch::run"));
    assert!(packet.contains("issue30_demo_trace_schema_gate_executed=true"));
    assert!(packet.contains("issue30_demo_source=demo_proof_input"));
    assert!(packet.contains("redaction-digest:issue30-default-prompt"));
    assert!(packet.contains("hidden_cot=<redacted-payload>"));
    assert!(packet.contains("persistent_roundtrip: passed=true"));
    assert!(packet.contains("state_inspection_gate: passed=true"));
    assert!(packet.contains("--trace-schema-gate"));
    assert!(packet.contains("trace_schema_gate: passed=true"));
    assert!(packet.contains("reasoning_genome_events="));
    assert!(packet.contains("reasoning_genome_write_allowed=0"));
    assert!(packet.contains("reasoning_genome_splice_write_allowed=0"));
    assert!(packet.contains("self_evolution_admission_events=1"));
    assert!(packet.contains("self_evolution_admission_review_packets=1"));
    assert!(packet.contains("self_evolution_admission_evidence_ids="));
    assert!(packet.contains("self_evolution_admission_missing_review_packet_refs=0"));
    assert!(packet.contains("issue30_environment_pressure_present=true"));
    assert!(packet.contains("issue30_pollution_event_id=redaction-digest:"));
    assert!(packet.contains("issue385_self_ontology_body_present=true"));
    assert!(packet.contains("issue385_body_state_id=redaction-digest:"));
    assert!(packet.contains("issue375_pre_reasoning_genome_isa_present=true"));
    assert!(packet.contains("issue375_reasoning_frame_id=redaction-digest:"));
    assert!(packet.contains("issue30_backend_action=deterministic_runtime_kv_roundtrip"));
    assert!(packet.contains("issue379_control_candidate_preview_only=true"));
    assert!(packet.contains("issue379_action_vocab_mask_preview=true"));
    assert!(packet.contains("issue379_signal_saliency_bias_preview=true"));
    assert!(packet.contains("issue377_problem_finding_present=true"));
    assert!(packet.contains("issue377_problem_finding_id=redaction-digest:"));
    assert!(packet.contains("issue377_hypothesis_candidate_present=true"));
    assert!(packet.contains("issue377_hypothesis_candidate_id=redaction-digest:"));
    assert!(packet.contains("issue377_problem_hypothesis_link=redaction-digest:"));
    assert!(packet.contains("issue377_admission_decision=preview_only"));
    assert!(packet.contains("issue30_context_source=issue30_context_input"));
    assert!(packet.contains("second_compute_budget_saved_tokens="));
    assert!(packet.contains("second_compute_budget_avoided_tokens="));
    assert!(packet.contains("second_compute_budget_kv_lookups_skipped="));
    assert!(packet.contains("second_approved_experience_reuse_digest=redaction-digest:"));
    assert!(packet.contains("second_compute_budget_anchor_count="));
    assert!(packet.contains("second_compute_budget_anchors_preserved=true"));
    assert!(packet.contains("second_compute_budget_anchors_preserved_count="));
    assert!(packet.contains("second_quality="));
    assert!(packet.contains("first_drift=watch"));
    assert!(packet.contains("second_drift=watch"));
    assert!(packet.contains("failures=0"));
    assert!(packet.contains("negative_unauthorized_write_allowed=false"));
    assert!(packet.contains("negative_durable_write_allowed=false"));
    assert!(packet.contains("negative_memory_write_allowed=false"));
    assert!(packet.contains("negative_genome_write_allowed=false"));
    assert!(packet.contains("negative_self_evolution_write_allowed=false"));
    assert!(packet.contains("negative_polluted_evidence_blocked=true"));
    assert!(packet.contains("negative_polluted_evidence_quarantined=true"));
    assert!(packet.contains("negative_bad_candidate_held_or_rolled_back=true"));
    assert!(packet.contains("negative_bad_candidate_digest=redaction-digest:"));
    assert!(packet.contains("negative_bad_candidate_decision=hold_then_rollback"));
    assert!(packet.contains("negative_rollback_anchor_present=true"));
    assert!(
        packet
            .contains("negative_rollback_anchor_evidence_id=issue-30-roundtrip-negative-gate-hold")
    );
    assert!(packet.contains("negative_rollback_anchor_digest=redaction-digest:"));
    assert!(packet.contains("negative_tenant_scope_write_denied=true"));
    assert!(packet.contains("negative_tenant_scope_mode=local_single_user_preview"));
    assert!(packet.contains("negative_tenant_scope_actor=fnv64:"));
    assert!(packet.contains("negative_tenant_scope_target=fnv64:"));
    assert!(packet.contains("negative_tenant_scope_denial_lane=self_evolving_memory"));
    assert!(packet.contains("negative_tenant_scope_denial_reason=cross_tenant_scope_rejected"));
    assert!(packet.contains("negative_single_tenant_preview=true"));
    assert!(packet.contains("negative_provenance_license_redaction_passed=true"));
    assert!(packet.contains("negative_digest_only=true"));
    assert!(packet.contains("issue30_roundtrip_source=roundtrip_proof_input"));
    assert!(packet.contains("memory_file_exists=true"));
    assert!(packet.contains("experience_file_exists=true"));
    assert!(packet.contains("adaptive_file_exists=true"));
    assert!(packet.contains("state_files_source=state_files_input"));
    assert!(!packet.contains(&args.memory_path.display().to_string()));
    assert!(!packet.contains(&args.experience_path.display().to_string()));
    assert!(!packet.contains(&args.adaptive_path.display().to_string()));
    assert!(!packet.contains(&args.prompt));
    assert!(!packet.contains("raw prompt"));
    assert!(!packet.contains("raw answer"));
    assert!(!packet.contains("chain-of-thought"));
    assert!(!packet.contains("C:\\Users"));
    assert!(!packet.contains("AppData"));
    assert!(!packet.contains("Design a Rust Noiron prototype"));
    assert!(!packet.contains("reuse_response"));
    assert!(!packet.contains("sk-secret"));
    assert!(!packet.contains("ghp_"));
    assert!(!rust_norion::contains_private_or_executable_marker(&packet));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate() {
    let asset_dir = temp_asset_dir("issue30-dispatch-roundtrip-trace");
    fs::create_dir_all(&asset_dir).unwrap();
    let trace_path = asset_dir.join("issue30-trace.jsonl");
    let args = Args::parse(vec![
        "--benchmark-roundtrip".to_owned(),
        "--inspect-state".to_owned(),
        "--inspect-gate".to_owned(),
        "--trace".to_owned(),
        trace_path.display().to_string(),
        "--trace-schema-gate".to_owned(),
        trace_path.display().to_string(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "--profile".to_owned(),
        "coding".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "32".to_owned(),
        "--inspect-min-runtime-kv-memories".to_owned(),
        "1".to_owned(),
        "--inspect-min-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-require-runtime-kv-dimensions".to_owned(),
    ]);

    dispatch::run(args).unwrap();
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert!(trace_report.reasoning_genome_events >= 2);
    assert_eq!(trace_report.self_evolution_admission_events, 1);
    assert_eq!(
        trace_report.self_evolution_admission_missing_review_packet_refs,
        0
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn roundtrip_and_inspect_state_can_chain_all_device_gate() {
    let asset_dir = temp_asset_dir("roundtrip-inspect-all-devices");
    fs::create_dir_all(&asset_dir).unwrap();
    let args = Args::parse(vec![
        "--benchmark-roundtrip".to_owned(),
        "--inspect-state".to_owned(),
        "--benchmark-all-devices".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "--profile".to_owned(),
        "coding".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "32".to_owned(),
        "--inspect-min-runtime-kv-memories".to_owned(),
        "1".to_owned(),
        "--inspect-min-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-model-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-adapter-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-max-runtime-adapter-selection-mismatches".to_owned(),
        "0".to_owned(),
        "--inspect-min-runtime-forward-energy-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-influence-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-precision-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-max-runtime-kv-precision-mismatches".to_owned(),
        "0".to_owned(),
        "--inspect-min-runtime-device-execution-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-import-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-export-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-runtime-kv-memory-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-model-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-adapter-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-forward-energy-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-kv-influence-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-kv-precision-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-device-execution-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-kv-import-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-runtime-kv-export-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-live-memory-feedback-device-profiles".to_owned(),
        DeviceClass::explicit_profiles().len().to_string(),
        "--inspect-min-evolution-memory-updates".to_owned(),
        "1".to_owned(),
        "--inspect-require-runtime-kv-dimensions".to_owned(),
        "Chain all-device roundtrip into inspect gate".to_owned(),
    ]);

    let roundtrip = run_persistent_roundtrip_all_devices(&args).unwrap();
    let inspect = run_state_inspection_all_devices(&args).unwrap();

    assert!(args.benchmark_roundtrip);
    assert!(args.inspect_state);
    assert!(args.inspect_gate);
    assert!(args.benchmark_all_devices);
    assert!(roundtrip.passed, "{:?}", roundtrip.failures);
    assert!(inspect.passed(), "{:?}", inspect.failures);
    assert_eq!(
        inspect.covered_devices(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        inspect.runtime_kv_memory_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        inspect.runtime_model_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        inspect.runtime_adapter_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        inspect.runtime_forward_energy_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        inspect.runtime_kv_influence_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        inspect.runtime_device_execution_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        inspect.runtime_kv_import_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        inspect.runtime_kv_export_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        inspect.reflection_issue_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        inspect.revision_action_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        inspect.live_memory_feedback_device_profiles(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(device_scoped_path(&args.memory_path, DeviceClass::CpuOnly).exists());
    assert!(device_scoped_path(&args.memory_path, DeviceClass::Server).exists());

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn state_inspection_all_devices_fails_missing_scoped_state_files() {
    let asset_dir = temp_asset_dir("inspect-missing-all-devices");
    fs::create_dir_all(&asset_dir).unwrap();
    let inspect_args = Args::parse(vec![
        "--inspect-state".to_owned(),
        "--benchmark-all-devices".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
    ]);

    let report = run_state_inspection_all_devices(&inspect_args).unwrap();

    assert!(!report.passed());
    assert_eq!(
        report.covered_devices(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.failed_devices().len(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(report.device_reports.iter().all(|device_report| {
        device_report
            .report
            .failures
            .iter()
            .any(|failure| failure.contains("memory file missing"))
    }));
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("device cpu state inspection failed"))
    );

    fs::remove_dir_all(asset_dir).unwrap();
}
