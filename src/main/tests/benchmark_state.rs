use super::*;

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
    assert!(
        report
            .telemetry
            .iter()
            .any(|line| { line == "self_evolution_admission_adaptive_preview_source_count=2" })
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
    assert!(report.device_reports.iter().all(|device_report| {
        device_report.report.first_runtime_kv_namespace_preserved
            && device_report.report.second_used_runtime_kv_memory
            && device_report
                .report
                .second_imported_runtime_kv_from_namespace
            && device_report.report.second_runtime_adapter_best_adapter
                == device_report.report.second_runtime_selected_adapter
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
    assert!(gate.passed(), "{:?}", gate.failures);
    assert!(args.memory_path.exists());
    assert!(args.experience_path.exists());
    assert!(args.adaptive_path.exists());

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
