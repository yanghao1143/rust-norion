use super::*;

#[test]
fn runtime_manifest_gate_builds_production_manifest_from_cli_assets() {
    let asset_dir = temp_asset_dir("runtime-manifest-cli-assets");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    let config = asset_dir.join("config.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    File::create(&config).unwrap();
    let args = Args::parse(vec![
        "--runtime-manifest-gate".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "65536".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "256".to_owned(),
        "--runtime-layers".to_owned(),
        "32".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "256".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "8".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "4".to_owned(),
        "--runtime-local-window".to_owned(),
        "8192".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
        "--runtime-config".to_owned(),
        config.display().to_string(),
        "--device".to_owned(),
        "cpu".to_owned(),
    ]);

    let manifest = args.runtime_manifest();
    let validation = manifest.validate_for_production();
    let device_gate =
        RuntimeManifestDeviceGateReport::evaluate(&manifest, &args.runtime_manifest_device_plan());
    let all_devices_gate = DevicePlanGateReport::evaluate_runtime_manifest(&manifest);

    assert!(args.runtime_manifest_gate);
    assert!(!args.runtime_manifest_all_devices_gate);
    assert_eq!(manifest.metadata.model_id, "self-owned-transformer");
    assert_eq!(manifest.metadata.tokenizer, "self-bpe");
    assert_eq!(manifest.metadata.native_context_window, 65_536);
    assert_eq!(manifest.metadata.embedding_dimensions, 256);
    assert_eq!(manifest.architecture.layer_count, 32);
    assert_eq!(manifest.architecture.hidden_size, 256);
    assert_eq!(manifest.architecture.attention_heads, 8);
    assert_eq!(manifest.architecture.kv_heads, 4);
    assert_eq!(manifest.architecture.local_window_tokens, 8_192);
    assert!(manifest.metadata.supports_kv_import);
    assert!(manifest.metadata.supports_kv_export);
    assert!(validation.passed(), "{validation:?}");
    assert!(device_gate.passed(), "{device_gate:?}");
    assert!(all_devices_gate.passed(), "{all_devices_gate:?}");
    assert_eq!(
        all_devices_gate.rows.len(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(device_gate.device, DeviceClass::CpuOnly);
    assert_eq!(device_gate.runtime_adapter_name(), "portable-rust");
    assert!(device_gate.runtime_device_contract.contains("device=cpu"));
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn runtime_manifest_all_devices_gate_cli_flag_enables_manifest_gate() {
    let args = Args::parse(vec![
        "--runtime-manifest-all-devices-gate".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-model-id".to_owned(),
        "all-device-transformer".to_owned(),
        "--runtime-native-window".to_owned(),
        "65536".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "256".to_owned(),
    ]);

    let report = DevicePlanGateReport::evaluate_runtime_manifest(&args.runtime_manifest());

    assert!(args.runtime_manifest_gate);
    assert!(args.runtime_manifest_all_devices_gate);
    assert!(report.passed(), "{report:?}");
    assert_eq!(report.rows.len(), DeviceClass::explicit_profiles().len());
}

#[test]
fn production_runtime_cli_builds_manifest_backed_boundary() {
    let asset_dir = temp_asset_dir("production-runtime-cli-assets");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let args = Args::parse(vec![
        "--production-runtime".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "4096".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "64".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "1024".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
        "--device".to_owned(),
        "cpu".to_owned(),
        "connect production runtime".to_owned(),
    ]);

    let runtime = args.production_runtime().unwrap();

    assert!(args.production_runtime);
    assert_eq!(runtime.metadata().model_id, "self-owned-transformer");
    assert_eq!(runtime.architecture().layer_count, 6);
    assert!(runtime.device_gate().passed());
    assert_eq!(
        runtime.device_gate().runtime_adapter_name(),
        "portable-rust"
    );
    assert!(runtime.assets().summary_line().contains("weights_bytes=0"));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_gate_cli_passes_reference_kernel() {
    let asset_dir = temp_asset_dir("production-runtime-cli-conformance-reference");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let args = Args::parse(vec![
        "--production-reference-kernel".to_owned(),
        "--production-kernel-conformance-gate".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "4096".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "64".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "1024".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
        "--device".to_owned(),
        "cpu".to_owned(),
    ]);

    let runtime = args.production_runtime().unwrap();
    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(args.production_runtime);
    assert!(args.production_reference_kernel);
    assert!(args.production_kernel_conformance_gate);
    assert!(report.passed, "{report:?}");
    assert_eq!(report.model_id, "self-owned-transformer");
    assert_eq!(report.selected_adapter, "portable-rust");
    assert!(report.exported_kv_blocks > 0);

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_local_kernel_cli_passes_conformance_gate() {
    let asset_dir = temp_asset_dir("production-runtime-cli-local-kernel");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let args = Args::parse(vec![
        "--production-local-kernel".to_owned(),
        "--production-kernel-conformance-gate".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "4096".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "64".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "1024".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
        "--device".to_owned(),
        "cpu".to_owned(),
    ]);

    let runtime = args.production_runtime().unwrap();
    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(args.production_runtime);
    assert!(args.production_local_kernel);
    assert!(args.production_kernel_conformance_gate);
    assert!(report.passed, "{report:?}");
    assert_eq!(report.model_id, "self-owned-transformer");
    assert!(report.exported_kv_blocks > 0);

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_command_runtime_cli_passes_conformance_gate() {
    let asset_dir = temp_asset_dir("production-runtime-cli-command-kernel");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let (program, runtime_args) = command_runtime_json_import_probe_args();
    let mut raw = vec![
        "--production-runtime".to_owned(),
        "--production-kernel-conformance-gate".to_owned(),
        "--runtime-command".to_owned(),
        program,
        "--runtime-json".to_owned(),
        "--runtime-prompt-mode".to_owned(),
        "stdin".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "4096".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "64".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "1024".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
        "--device".to_owned(),
        "cpu".to_owned(),
    ];
    for arg in runtime_args {
        raw.push("--runtime-arg".to_owned());
        raw.push(arg);
    }
    let args = Args::parse(raw);

    let runtime = args.production_runtime().unwrap();
    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(args.production_runtime);
    assert!(!args.production_reference_kernel);
    assert!(!args.production_local_kernel);
    assert!(args.runtime_command.is_some());
    assert_eq!(args.runtime_wire_format, CommandWireFormat::Json);
    assert!(runtime.kernel_connected());
    assert!(report.passed, "{report:?}");
    assert_eq!(report.model_id, "self-owned-transformer");
    assert_eq!(report.selected_adapter, "portable-rust");
    assert_eq!(report.token_count, 1);
    assert!(report.trace_steps >= 2);
    assert_eq!(report.imported_kv_blocks, 1);
    assert_eq!(report.exported_kv_blocks, 1);
    assert_eq!(report.forward_energy, Some(0.42));
    assert_eq!(report.kv_influence, Some(0.25));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_gate_cli_fails_without_kernel() {
    let asset_dir = temp_asset_dir("production-runtime-cli-conformance-missing");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let args = Args::parse(vec![
        "--production-kernel-conformance-gate".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "4096".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "64".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "1024".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
        "--device".to_owned(),
        "cpu".to_owned(),
    ]);

    let runtime = args.production_runtime().unwrap();
    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(args.production_runtime);
    assert!(!args.production_reference_kernel);
    assert!(args.production_kernel_conformance_gate);
    assert!(!report.passed);
    assert!(!report.kernel_connected);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("production forward kernel is not connected"))
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_reference_kernel_conformance_all_devices_cli_passes() {
    assert_production_kernel_conformance_all_devices_cli_passes(
        "--production-reference-kernel",
        "production-runtime-cli-conformance-reference-matrix",
    );
}

#[test]
fn production_local_kernel_conformance_all_devices_cli_passes() {
    assert_production_kernel_conformance_all_devices_cli_passes(
        "--production-local-kernel",
        "production-runtime-cli-conformance-local-matrix",
    );
}

#[test]
fn production_kernel_conformance_all_devices_cli_fails_without_kernel() {
    let asset_dir = temp_asset_dir("production-runtime-cli-conformance-missing-matrix");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let args = Args::parse(production_conformance_all_devices_args(
        "--production-kernel-conformance-all-devices-gate",
        &weights,
        &tokenizer,
    ));

    let report = run_production_kernel_conformance_all_devices(&args);

    assert!(args.production_runtime);
    assert!(args.production_kernel_conformance_gate);
    assert!(args.production_kernel_conformance_all_devices_gate);
    assert!(!report.passed);
    assert_eq!(
        report.covered_devices(),
        DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        report.failed_devices().len(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(report.device_reports.iter().all(|device_report| {
        !device_report.report.kernel_connected
            && device_report
                .report
                .failures
                .iter()
                .any(|failure| failure.contains("production forward kernel is not connected"))
    }));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_cli_generate_reports_kernel_boundary_error() {
    let asset_dir = temp_asset_dir("production-runtime-cli-generate");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let args = Args::parse(vec![
        "--production-runtime".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "4096".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "64".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "1024".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
        "--device".to_owned(),
        "cpu".to_owned(),
        "connect production runtime".to_owned(),
    ]);
    let mut engine = NoironEngine::new();
    configure_engine(&mut engine, &args);
    let mut backend = RuntimeBackend::new(args.production_runtime().unwrap());

    let outcome = engine.infer(
        InferenceRequest::new(args.prompt.clone(), args.profile),
        &mut backend,
    );

    assert!(outcome.answer.contains("kernel is not connected"));
    assert!(
        backend
            .last_error()
            .unwrap()
            .message()
            .contains("kernel is not connected")
    );
    assert!(outcome.report.quality < 0.5);

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_cli_can_run_reference_kernel_end_to_end() {
    let asset_dir = temp_asset_dir("production-runtime-cli-reference");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let args = Args::parse(vec![
        "--production-reference-kernel".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "4096".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "64".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "1024".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
        "--device".to_owned(),
        "cpu".to_owned(),
        "connect production reference kernel with Noiron memory".to_owned(),
    ]);
    let mut engine = NoironEngine::new();
    configure_engine(&mut engine, &args);
    let mut backend = RuntimeBackend::new(args.production_runtime().unwrap());

    let outcome = engine.infer(
        InferenceRequest::new(args.prompt.clone(), args.profile),
        &mut backend,
    );

    assert!(args.production_runtime);
    assert!(args.production_reference_kernel);
    assert!(backend.last_error().is_none());
    assert!(
        outcome
            .answer
            .contains("Reference production Transformer kernel result")
    );
    assert!(outcome.runtime_diagnostics.has_forward_signal());
    assert_eq!(
        outcome.runtime_diagnostics.model_id.as_deref(),
        Some("self-owned-transformer")
    );
    assert_eq!(
        outcome.runtime_diagnostics.selected_adapter.as_deref(),
        Some("portable-rust")
    );
    assert_eq!(outcome.runtime_diagnostics.layer_count, 6);
    assert!(outcome.exported_runtime_kv_blocks > 0);
    assert!(outcome.report.quality > 0.46);

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_reference_kernel_benchmark_passes_runtime_and_trace_gates() {
    let asset_dir = temp_asset_dir("production-reference-benchmark");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let trace_path = asset_dir.join("benchmark.jsonl");
    let args = Args::parse(vec![
        "--production-reference-kernel".to_owned(),
        "--benchmark".to_owned(),
        trace_path.display().to_string(),
        "--benchmark-gate".to_owned(),
        "--benchmark-min-quality".to_owned(),
        "0.45".to_owned(),
        "--benchmark-min-reward".to_owned(),
        "0.35".to_owned(),
        "--benchmark-min-runtime-forward-cases".to_owned(),
        "4".to_owned(),
        "--benchmark-min-runtime-kv-exported".to_owned(),
        "4".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "4096".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "64".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "1024".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
        "--device".to_owned(),
        "cpu".to_owned(),
    ]);
    let mut engine = NoironEngine::new();
    configure_engine(&mut engine, &args);
    let runtime = args.production_runtime().unwrap();
    let mut backend = RuntimeBackend::new(runtime);

    let summary = run_benchmark(&mut engine, &mut backend, &trace_path).unwrap();
    let gate_report = summary.evaluate(&args.benchmark_gate());
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

    assert_eq!(summary.len(), 4);
    assert_eq!(summary.runtime_forward_cases(), 4);
    assert!(summary.total_runtime_kv_exported() >= 4);
    assert!(summary.summary_line().contains("runtime_forward_cases=4"));
    assert!(summary.summary_line().contains("runtime_kv_exported="));
    assert!(gate_report.passed, "{:?}", gate_report.failures);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 4);
    assert!(backend.last_error().is_none());

    fs::remove_dir_all(asset_dir).unwrap();
}
