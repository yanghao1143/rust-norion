use super::*;

#[test]
fn runtime_manifest_gate_rejects_invalid_cli_architecture() {
    let asset_dir = temp_asset_dir("runtime-manifest-invalid-architecture");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let args = Args::parse(vec![
        "--runtime-manifest-gate".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "4096".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "130".to_owned(),
        "--runtime-layers".to_owned(),
        "12".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "130".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "8".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "16".to_owned(),
        "--runtime-local-window".to_owned(),
        "8192".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
    ]);

    let validation = args.runtime_manifest().validate_for_production();

    assert!(!validation.passed());
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("hidden_size must be divisible"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("kv_heads must not exceed"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("local_window_tokens must not exceed"))
    );
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn configure_engine_applies_memory_policy_flags() {
    let args = Args::parse(vec![
        "--device".to_owned(),
        "microcontroller".to_owned(),
        "--retention-stale-after".to_owned(),
        "9".to_owned(),
        "--retention-decay-rate".to_owned(),
        "1.5".to_owned(),
        "--retention-remove-below".to_owned(),
        "0.12".to_owned(),
        "--retention-remove-after-failures".to_owned(),
        "0".to_owned(),
        "--compaction-threshold".to_owned(),
        "0.05".to_owned(),
        "--compaction-max-candidates".to_owned(),
        "1".to_owned(),
        "--compaction-max-merges".to_owned(),
        "0".to_owned(),
    ]);
    let mut engine = NoironEngine::new();

    configure_engine(&mut engine, &args);

    assert_eq!(engine.memory_retention_policy.stale_after, 9);
    assert_eq!(engine.memory_retention_policy.decay_rate, 0.95);
    assert_eq!(engine.memory_retention_policy.remove_below_strength, 0.12);
    assert_eq!(engine.memory_retention_policy.remove_after_failures, 1);
    assert_eq!(engine.memory_compaction_policy.similarity_threshold, 0.10);
    assert_eq!(engine.memory_compaction_policy.max_candidates, 2);
    assert_eq!(engine.memory_compaction_policy.max_merges, 0);
}

#[test]
fn configure_engine_applies_device_memory_governance_defaults() {
    let tiny_args = Args::parse(vec![
        "--device".to_owned(),
        "microcontroller".to_owned(),
        "--cpu-load".to_owned(),
        "30".to_owned(),
        "--ram-load".to_owned(),
        "35".to_owned(),
    ]);
    let server_args = Args::parse(vec![
        "--device".to_owned(),
        "server".to_owned(),
        "--cpu-load".to_owned(),
        "10".to_owned(),
        "--gpu-load".to_owned(),
        "15".to_owned(),
        "--ram-load".to_owned(),
        "20".to_owned(),
    ]);
    let mut tiny_engine = NoironEngine::new();
    let mut server_engine = NoironEngine::new();

    configure_engine(&mut tiny_engine, &tiny_args);
    configure_engine(&mut server_engine, &server_args);

    assert!(tiny_engine.memory_retention_policy.stale_after < 64);
    assert!(tiny_engine.memory_retention_policy.decay_rate > 0.04);
    assert!(tiny_engine.memory_compaction_policy.max_candidates < 512);
    assert!(tiny_engine.memory_compaction_policy.similarity_threshold > 0.92);
    assert!(server_engine.memory_retention_policy.stale_after > 64);
    assert!(server_engine.memory_retention_policy.decay_rate < 0.04);
    assert!(server_engine.memory_compaction_policy.max_candidates > 512);
}

#[test]
fn auto_device_preserves_manual_load_overrides() {
    let args = Args::parse(vec![
        "--device".to_owned(),
        "auto".to_owned(),
        "--cpu-load".to_owned(),
        "91".to_owned(),
        "--gpu-load".to_owned(),
        "12".to_owned(),
        "--ram-load".to_owned(),
        "61".to_owned(),
        "--disk-load".to_owned(),
        "7".to_owned(),
        "probe defaults should not replace explicit loads".to_owned(),
    ]);

    assert_eq!(args.cpu_load, 91.0);
    assert_eq!(args.gpu_load, 12.0);
    assert_eq!(args.ram_load, 61.0);
    assert_eq!(args.disk_load, 7.0);
    assert!(args.auto_device_probe.is_some());
    let report = args.effective_probe_report();
    assert_eq!(report.device, args.device);
    assert!((report.cpu_load - 0.91).abs() < 0.0001);
    assert!((report.gpu_load - 0.12).abs() < 0.0001);
    assert!((report.ram_load - 0.61).abs() < 0.0001);
    assert!((report.disk_load - 0.07).abs() < 0.0001);
    assert!(
        report
            .evidence
            .iter()
            .any(|item| item == "cli_override:NOIRON_CPU_LOAD")
    );
    assert!(
        report
            .evidence
            .iter()
            .any(|item| item == "cli_override:NOIRON_GPU_LOAD")
    );
}

#[test]
fn unknown_manual_device_uses_portable_cpu_fallback() {
    let args = Args::parse(vec![
        "--device".to_owned(),
        "future-device-sku".to_owned(),
        "unknown devices should still get a portable execution plan".to_owned(),
    ]);

    assert_eq!(args.device, DeviceClass::CpuOnly);
}

#[test]
fn probe_device_flag_exposes_manual_device_plan() {
    let args = Args::parse(vec![
        "--probe-device".to_owned(),
        "--device".to_owned(),
        "server".to_owned(),
        "--cpu-load".to_owned(),
        "20".to_owned(),
        "--ram-load".to_owned(),
        "30".to_owned(),
        "audit the selected server device".to_owned(),
    ]);

    assert!(args.probe_device);
    assert!(args.device_flag_provided);
    assert!(args.auto_device_probe.is_none());
    let report = args.effective_probe_report();
    let plan = args.runtime_manifest_device_plan();

    assert_eq!(report.device, DeviceClass::Server);
    assert_eq!(report.reason, "manual-device");
    assert_eq!(plan.device, DeviceClass::Server);
    assert!(plan.summary().contains("device=server"));
    assert!(
        report
            .evidence
            .iter()
            .any(|item| item == "selected_profile:server")
    );
}
