use super::*;

#[test]
fn gemma_model_service_smoke_cli_uses_local_snapshot_defaults_and_gates() {
    let cache_dir = temp_asset_dir("gemma-model-service-smoke-cache");
    let snapshot_dir = cache_dir
        .join("hub")
        .join("models--google--gemma-4-12B-it")
        .join("snapshots")
        .join("5926caa");
    write_minimal_gemma_snapshot(&snapshot_dir);
    let snapshot = snapshot_dir.display().to_string();
    let args = Args::parse(vec![
        "--gemma-model-service-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot.clone(),
    ]);

    assert!(args.gemma_model_service_smoke);
    assert!(!args.serve);
    assert!(args.gemma_12b_runtime);
    assert_eq!(args.profile, TaskProfile::Coding);
    assert_eq!(args.prompt, GEMMA_BUSINESS_SMOKE_PROMPT);
    assert_eq!(args.gemma_smoke_keep_runs, GEMMA_SMOKE_DEFAULT_KEEP_RUNS);
    assert_gemma_smoke_paths_are_isolated(&args, GEMMA_MODEL_SERVICE_SMOKE_DIR);
    assert_eq!(
        args.trace_schema_gate_path.as_ref(),
        args.trace_path.as_ref()
    );
    assert_eq!(args.runtime_metadata.model_id, snapshot);
    assert_eq!(args.gemma_runtime_token_source.as_deref(), Some("none"));
    assert_eq!(args.gemma_runtime_hf_cache, Some(cache_dir.clone()));
    assert_eq!(
        args.gemma_runtime_quantization_mode,
        GemmaRuntimeQuantizationMode::Isq
    );
    assert_eq!(args.gemma_runtime_quantization, "4");
    assert_eq!(args.runtime_args, vec!["--seed".to_owned(), "7".to_owned()]);
    assert_eq!(
        args.runtime_timeout_ms,
        Some(GEMMA4_12B_SMOKE_COLD_START_TIMEOUT_MS)
    );
    assert_eq!(
        args.command_runtime().unwrap().timeout_ms(),
        Some(GEMMA4_12B_SMOKE_COLD_START_TIMEOUT_MS)
    );
    assert!(gemma_business_smoke_preflight_failures(&args).is_empty());

    let gate = gemma_model_service_smoke_state_gate(&args);
    let case_count = GEMMA_MODEL_SERVICE_BUSINESS_CASES.len();
    assert_eq!(case_count, 3);
    assert_eq!(gate.min_memories, Some(1));
    assert_eq!(gate.min_experiences, Some(case_count));
    assert_eq!(gate.min_runtime_model_experiences, Some(case_count));
    assert_eq!(gate.min_runtime_tokens, Some(case_count));
    assert_eq!(gate.min_runtime_architecture_experiences, Some(case_count));
    assert_eq!(gate.min_runtime_kv_precision_experiences, Some(case_count));
    assert_eq!(gate.max_runtime_kv_precision_mismatches, Some(0));
    assert_eq!(gate.max_runtime_errors, Some(0));
    assert_eq!(gate.max_runtime_timeouts, Some(0));
    assert_eq!(
        gate.min_evolution_live_inference_runs,
        Some(case_count as u64)
    );
    assert_eq!(
        gate.min_evolution_external_feedbacks,
        Some(case_count as u64)
    );
    assert_eq!(
        gate.min_evolution_external_feedback_memory_updates,
        Some(case_count as u64)
    );
    assert!(
        (gate
            .min_evolution_external_feedback_strength_delta
            .unwrap_or_default()
            - (0.01 * case_count as f32))
            .abs()
            < 0.0001
    );
    assert_eq!(gate.min_business_contract_experiences, Some(case_count));
    assert_eq!(gate.min_business_contract_passed, Some(case_count));
    assert_eq!(gate.max_business_contract_failed, Some(0));
    assert_eq!(gate.max_business_contract_missing_signals, Some(0));
    assert_eq!(gate.max_business_contract_protocol_leaks, Some(0));
    assert_eq!(gate.max_business_contract_substitutions, Some(0));
    assert_eq!(gate.max_business_contract_evasive_denials, Some(0));
    assert_eq!(gate.max_business_contract_missing_handling_signals, Some(0));
    assert_eq!(gate.min_evolution_replay_runs, Some(1));
    assert_eq!(gate.min_evolution_replay_items, Some(case_count as u64));
    assert_eq!(
        gate.min_evolution_replay_business_contract_items,
        Some(case_count as u64)
    );
    assert_eq!(
        gate.min_evolution_replay_business_contract_passed,
        Some(case_count as u64)
    );
    assert_eq!(
        gate.min_evolution_replay_business_contract_raw_audits,
        Some(case_count as u64)
    );
    assert_eq!(gate.max_evolution_replay_business_contract_failed, Some(0));
    assert_eq!(gate.min_rust_check_experiences, Some(1));
    assert_eq!(gate.min_rust_check_passed, Some(1));
    assert_eq!(gate.max_rust_check_failed, Some(0));
    assert_eq!(gate.min_evolution_replay_rust_check_items, Some(1));
    assert_eq!(gate.min_evolution_replay_rust_check_passed, Some(1));
    assert_eq!(gate.max_evolution_replay_rust_check_failed, Some(0));
    assert_eq!(
        gate.min_evolution_replay_rust_check_live_memory_feedback_updates,
        Some(1)
    );
    assert_eq!(
        gate.min_evolution_replay_rust_check_live_memory_feedback_applied,
        Some(1)
    );

    fs::remove_dir_all(cache_dir).unwrap();
}

#[test]
fn gemma_smoke_preserves_explicit_state_paths() {
    let cache_dir = temp_asset_dir("gemma-smoke-explicit-path-cache");
    let snapshot_dir = cache_dir
        .join("hub")
        .join("models--google--gemma-4-12B-it")
        .join("snapshots")
        .join("5926caa");
    fs::create_dir_all(&snapshot_dir).unwrap();
    let asset_dir = temp_asset_dir("gemma-smoke-explicit-paths");
    let memory = asset_dir.join("memory.ndkv");
    let experience = asset_dir.join("experience.ndkv");
    let adaptive = asset_dir.join("adaptive.ndkv");
    let trace = asset_dir.join("trace.jsonl");
    let args = Args::parse(vec![
        "--gemma-model-service-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot_dir.display().to_string(),
        "--memory".to_owned(),
        memory.display().to_string(),
        "--experience".to_owned(),
        experience.display().to_string(),
        "--adaptive".to_owned(),
        adaptive.display().to_string(),
        "--trace".to_owned(),
        trace.display().to_string(),
    ]);

    assert_eq!(args.memory_path, memory);
    assert_eq!(args.experience_path, experience);
    assert_eq!(args.adaptive_path, adaptive);
    assert_eq!(args.trace_path.as_ref(), Some(&trace));
    assert_eq!(args.trace_schema_gate_path.as_ref(), Some(&trace));

    fs::remove_dir_all(cache_dir).unwrap();
}

#[test]
fn gemma_smoke_respects_explicit_runtime_timeout() {
    let cache_dir = temp_asset_dir("gemma-smoke-timeout-cache");
    let snapshot_dir = cache_dir
        .join("hub")
        .join("models--google--gemma-4-12B-it")
        .join("snapshots")
        .join("5926caa");
    fs::create_dir_all(&snapshot_dir).unwrap();
    let snapshot = snapshot_dir.display().to_string();
    let args = Args::parse(vec![
        "--gemma-model-service-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot,
        "--runtime-timeout-ms".to_owned(),
        "1500".to_owned(),
    ]);

    assert_eq!(args.runtime_timeout_ms, Some(1500));
    assert_eq!(args.command_runtime().unwrap().timeout_ms(), Some(1500));

    fs::remove_dir_all(cache_dir).unwrap();
}

#[test]
fn gemma_smoke_keep_runs_flag_overrides_default() {
    let cache_dir = temp_asset_dir("gemma-smoke-keep-runs-cache");
    let snapshot_dir = cache_dir
        .join("hub")
        .join("models--google--gemma-4-12B-it")
        .join("snapshots")
        .join("5926caa");
    fs::create_dir_all(&snapshot_dir).unwrap();
    let args = Args::parse(vec![
        "--gemma-model-service-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot_dir.display().to_string(),
        "--gemma-smoke-keep-runs".to_owned(),
        "2".to_owned(),
    ]);

    assert_eq!(args.gemma_smoke_keep_runs, 2);

    fs::remove_dir_all(cache_dir).unwrap();
}

#[test]
fn gemma_smoke_retention_prunes_only_generated_run_dirs() {
    let parent = temp_asset_dir("gemma-smoke-retention");
    fs::create_dir_all(&parent).unwrap();
    let base = parent.join("gemma-model-service-smoke");
    let generated_names = [
        "gemma-model-service-smoke-1",
        "gemma-model-service-smoke-20260611-014722",
        "gemma-model-service-smoke-3",
        "gemma-model-service-smoke-4",
    ];
    for name in generated_names {
        let nested = parent.join(name).join("nested");
        fs::create_dir_all(&nested).unwrap();
        File::create(nested.join("trace.jsonl")).unwrap();
        thread::sleep(Duration::from_millis(5));
    }
    let tagged_evidence = parent.join("gemma-model-service-smoke-20260611-tracefix-pass");
    let other_smoke = parent.join("gemma-business-smoke-9");
    fs::create_dir_all(&tagged_evidence).unwrap();
    fs::create_dir_all(&other_smoke).unwrap();

    let report = prune_gemma_smoke_run_dirs(&base, 2).unwrap();
    let remaining_generated = generated_names
        .iter()
        .filter(|name| parent.join(name).exists())
        .count();

    assert_eq!(report.before, 4);
    assert_eq!(report.removed, 2);
    assert_eq!(report.kept, 2);
    assert_eq!(remaining_generated, 2);
    assert!(tagged_evidence.exists());
    assert!(other_smoke.exists());

    fs::remove_dir_all(parent).unwrap();
}
