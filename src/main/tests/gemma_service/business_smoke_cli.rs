use super::*;

#[test]
fn gemma_business_smoke_cli_uses_local_snapshot_defaults_and_gates() {
    let cache_dir = temp_asset_dir("gemma-business-smoke-cache");
    let snapshot_dir = cache_dir
        .join("hub")
        .join("models--google--gemma-4-12B-it")
        .join("snapshots")
        .join("5926caa");
    write_minimal_gemma_snapshot(&snapshot_dir);
    let snapshot = snapshot_dir.display().to_string();
    let args = Args::parse(vec![
        "--gemma-business-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot.clone(),
    ]);

    assert!(args.gemma_business_smoke);
    assert!(args.gemma_12b_runtime);
    assert_eq!(args.profile, TaskProfile::Coding);
    assert_eq!(args.prompt, GEMMA_BUSINESS_SMOKE_PROMPT);
    assert_eq!(args.gemma_smoke_keep_runs, GEMMA_SMOKE_DEFAULT_KEEP_RUNS);
    assert_gemma_smoke_paths_are_isolated(&args, GEMMA_BUSINESS_SMOKE_DIR);
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

    let gate = gemma_business_smoke_state_gate(&args);
    assert_eq!(gate.min_memories, Some(1));
    assert_eq!(gate.min_experiences, Some(1));
    assert_eq!(gate.min_runtime_model_experiences, Some(1));
    assert_eq!(gate.min_runtime_tokens, Some(1));
    assert_eq!(gate.min_runtime_architecture_experiences, Some(1));
    assert_eq!(gate.min_runtime_kv_precision_experiences, Some(1));
    assert_eq!(gate.max_runtime_kv_precision_mismatches, Some(0));
    assert_eq!(gate.max_runtime_errors, Some(0));
    assert_eq!(gate.max_runtime_timeouts, Some(0));
    assert_eq!(gate.min_business_contract_experiences, Some(1));
    assert_eq!(gate.min_business_contract_passed, Some(1));
    assert_eq!(gate.max_business_contract_failed, Some(0));
    assert_eq!(gate.max_business_contract_missing_signals, Some(0));
    assert_eq!(gate.max_business_contract_protocol_leaks, Some(0));
    assert_eq!(gate.max_business_contract_substitutions, Some(0));
    assert_eq!(gate.max_business_contract_evasive_denials, Some(0));
    assert_eq!(gate.max_business_contract_missing_handling_signals, Some(0));
    assert_eq!(gate.min_evolution_live_inference_runs, Some(1));
    assert_eq!(gate.min_evolution_replay_runs, Some(1));
    assert_eq!(gate.min_evolution_replay_items, Some(1));
    assert_eq!(gate.min_evolution_replay_business_contract_items, Some(1));
    assert_eq!(gate.min_evolution_replay_business_contract_passed, Some(1));
    assert_eq!(
        gate.min_evolution_replay_business_contract_raw_audits,
        Some(1)
    );
    assert_eq!(gate.max_evolution_replay_business_contract_failed, Some(0));

    fs::remove_dir_all(cache_dir).unwrap();
}

#[test]
fn gemma_business_cycle_smoke_cli_uses_strict_business_cycle_gate() {
    let cache_dir = temp_asset_dir("gemma-business-cycle-smoke-cache");
    let snapshot_dir = cache_dir
        .join("hub")
        .join("models--google--gemma-4-12B-it")
        .join("snapshots")
        .join("5926caa");
    write_minimal_gemma_snapshot(&snapshot_dir);
    let snapshot = snapshot_dir.display().to_string();
    let args = Args::parse(vec![
        "--gemma-business-cycle-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot.clone(),
    ]);

    assert!(args.gemma_business_cycle_smoke);
    assert!(args.gemma_12b_runtime);
    assert_eq!(args.profile, TaskProfile::Coding);
    assert_eq!(args.prompt, GEMMA_BUSINESS_SMOKE_PROMPT);
    assert_gemma_smoke_paths_are_isolated(&args, GEMMA_BUSINESS_CYCLE_SMOKE_DIR);
    assert_eq!(
        args.trace_schema_gate_path.as_ref(),
        args.trace_path.as_ref()
    );
    assert_eq!(args.runtime_metadata.model_id, snapshot);
    assert_eq!(args.gemma_runtime_token_source.as_deref(), Some("none"));
    assert_eq!(args.gemma_runtime_hf_cache, Some(cache_dir.clone()));
    assert_eq!(args.runtime_args, vec!["--seed".to_owned(), "7".to_owned()]);
    assert_eq!(
        args.runtime_timeout_ms,
        Some(GEMMA4_12B_SMOKE_COLD_START_TIMEOUT_MS)
    );
    assert!(gemma_business_smoke_preflight_failures(&args).is_empty());
    assert_eq!(
        gemma_smoke_base_dir(&args),
        Some(GEMMA_BUSINESS_CYCLE_SMOKE_DIR)
    );

    let gate = gemma_business_cycle_state_gate(&args);
    assert_eq!(gate.min_runtime_model_experiences, Some(1));
    assert_eq!(gate.min_runtime_tokens, Some(1));
    assert_eq!(gate.min_runtime_architecture_experiences, Some(1));
    assert_eq!(gate.min_runtime_kv_precision_experiences, Some(1));
    assert_eq!(gate.max_runtime_errors, Some(0));
    assert_eq!(gate.max_runtime_timeouts, Some(0));
    assert_eq!(gate.min_business_contract_experiences, Some(1));
    assert_eq!(gate.min_business_contract_passed, Some(1));
    assert_eq!(gate.min_evolution_external_feedbacks, Some(2));
    assert_eq!(gate.min_evolution_external_feedback_memory_updates, Some(2));
    assert_eq!(
        gate.min_evolution_external_feedback_strength_delta,
        Some(0.01)
    );
    assert_eq!(gate.min_rust_check_experiences, Some(1));
    assert_eq!(gate.min_rust_check_passed, Some(1));
    assert_eq!(gate.max_rust_check_failed, Some(0));
    assert_eq!(gate.min_evolution_replay_rust_check_items, Some(1));
    assert_eq!(gate.min_evolution_replay_rust_check_passed, Some(1));
    assert_eq!(gate.max_evolution_replay_rust_check_failed, Some(0));
    assert_eq!(
        gate.min_evolution_replay_live_memory_feedback_updates,
        Some(1)
    );
    assert_eq!(
        gate.min_evolution_replay_live_memory_feedback_applied,
        Some(1)
    );
    assert_eq!(
        gate.min_evolution_replay_live_memory_feedback_strength_delta,
        Some(0.01)
    );
    assert_eq!(gate.min_evolution_replay_live_evolution_items, Some(1));
    assert_eq!(
        gate.min_evolution_replay_live_evolution_online_reward_feedbacks,
        Some(1)
    );
    assert_eq!(
        gate.min_evolution_replay_live_evolution_online_reward_reinforcements,
        Some(1)
    );

    fs::remove_dir_all(cache_dir).unwrap();
}

#[test]
fn gemma_business_smoke_records_contract_audit_trace_and_state() {
    struct PassingBusinessBackend;

    impl InferenceBackend for PassingBusinessBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            let mut token = DraftToken::new("runtime_model_experiences");
            token.logprob = Some(-0.05);
            token.entropy = Some(0.10);
            InferenceDraft::new(
                gemma_business_smoke_case().contract_line,
                vec![ReasoningStep::new(
                    "business_contract",
                    "returned the required local Gemma business receipt signals",
                    0.95,
                )],
            )
            .with_tokens(vec![token])
        }
    }

    let asset_dir = target_asset_dir("gemma-business-contract-audit");
    fs::create_dir_all(&asset_dir).unwrap();
    let trace = asset_dir.join("trace.jsonl");
    let gate = asset_dir.join("trace-gate.jsonl");
    let mut engine = NoironEngine::new();
    let mut backend = PassingBusinessBackend;
    let timed = run_timed_inference(
        &mut engine,
        &mut backend,
        GEMMA_BUSINESS_SMOKE_PROMPT.to_owned(),
        TaskProfile::Coding,
        Some(&trace),
        Some("gemma-business-runtime"),
    )
    .unwrap();
    let audit = crate::gemma_business::contract::record_gemma_business_smoke_contract_to_paths(
        &mut engine,
        &timed.outcome,
        [Some(&trace), Some(&gate)],
    )
    .unwrap();
    let inspection = StateInspectionReport::from_engine(&engine, 5);
    let trace_report = evaluate_trace_schema_jsonl(&trace).unwrap();
    let gate_report = evaluate_trace_schema_jsonl(&gate).unwrap();

    assert!(audit.passed(), "{audit:?}");
    assert_eq!(inspection.business_contract_experience_count, 1);
    assert_eq!(inspection.business_contract_passed_count, 1);
    assert_eq!(inspection.business_contract_failed_count, 0);
    assert_eq!(inspection.business_contract_raw_passed_count, 1);
    assert_eq!(inspection.business_contract_response_normalized_count, 0);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert!(gate_report.passed, "{:?}", gate_report.failures);
    assert_eq!(trace_report.business_contract_events, 1);
    assert_eq!(gate_report.business_contract_events, 1);
    assert_eq!(trace_report.business_contract_event_passed, 1);
    assert_eq!(gate_report.business_contract_event_passed, 1);
    assert_eq!(trace_report.business_contract_event_raw_passed, 1);
    assert_eq!(gate_report.business_contract_event_raw_passed, 1);
    assert_eq!(trace_report.business_contract_event_response_normalized, 0);
    assert_eq!(gate_report.business_contract_event_response_normalized, 0);

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn gemma_business_smoke_normalizes_partial_real_model_contract_answer() {
    struct PartialBusinessBackend;

    impl InferenceBackend for PartialBusinessBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            let mut token = DraftToken::new("runtime_model_experiences");
            token.logprob = Some(-0.11);
            token.entropy = Some(0.18);
            InferenceDraft::new(
                    "请将 runtime_model_experiences 视为审计遥测字段，而非 Rust API，并包含该特定的字段名称。",
                    vec![ReasoningStep::new(
                        "business_contract",
                        "answered with the audit field but omitted the canonical receipt signals",
                        0.72,
                    )],
                )
                .with_tokens(vec![token])
        }
    }

    let asset_dir = target_asset_dir("gemma-business-contract-normalized-audit");
    fs::create_dir_all(&asset_dir).unwrap();
    let trace = asset_dir.join("trace.jsonl");
    let mut engine = NoironEngine::new();
    let mut backend = PartialBusinessBackend;
    let timed = run_timed_inference(
        &mut engine,
        &mut backend,
        GEMMA_BUSINESS_SMOKE_PROMPT.to_owned(),
        TaskProfile::Coding,
        Some(&trace),
        Some("gemma-business-runtime"),
    )
    .unwrap();
    let audit =
        record_gemma_business_smoke_contract(&mut engine, &timed.outcome, Some(&trace)).unwrap();
    let inspection = StateInspectionReport::from_engine(&engine, 5);
    let trace_report = evaluate_trace_schema_jsonl(&trace).unwrap();

    assert!(audit.passed(), "{audit:?}");
    assert_eq!(inspection.business_contract_experience_count, 1);
    assert_eq!(inspection.business_contract_passed_count, 1);
    assert_eq!(inspection.business_contract_failed_count, 0);
    assert_eq!(inspection.business_contract_raw_passed_count, 0);
    assert_eq!(inspection.business_contract_raw_failed_count, 1);
    assert_eq!(inspection.business_contract_missing_signals, 0);
    assert_eq!(inspection.business_contract_response_normalized_count, 1);
    assert_eq!(inspection.business_contract_canonical_fallback_count, 1);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.business_contract_events, 1);
    assert_eq!(trace_report.business_contract_event_passed, 1);
    assert_eq!(trace_report.business_contract_event_failed, 0);
    assert_eq!(trace_report.business_contract_event_missing_signals, 0);
    assert_eq!(trace_report.business_contract_event_raw_passed, 0);
    assert_eq!(trace_report.business_contract_event_raw_failed, 1);
    assert_eq!(trace_report.business_contract_event_response_normalized, 1);
    assert_eq!(trace_report.business_contract_event_canonical_fallbacks, 1);

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn gemma_business_smoke_replays_recorded_business_contract() {
    struct PartialBusinessBackend;

    impl InferenceBackend for PartialBusinessBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                    "请将 runtime_model_experiences 视为审计遥测字段，而非 Rust API，并包含该特定的字段名称。",
                    vec![ReasoningStep::new(
                        "business_contract",
                        "record a raw-failed contract that should replay as canonical evidence",
                        0.72,
                    )],
                )
                .with_tokens(vec![DraftToken::new("runtime_model_experiences")])
        }
    }

    let asset_dir = target_asset_dir("gemma-business-contract-replay");
    let snapshot_dir = asset_dir
        .join("hub")
        .join("models--google--gemma-4-12B-it")
        .join("snapshots")
        .join("5926caa");
    fs::create_dir_all(&snapshot_dir).unwrap();
    let snapshot = snapshot_dir.display().to_string();
    let mut args = Args::parse(vec![
        "--gemma-business-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot,
    ]);
    args.memory_path = asset_dir.join("memory.ndkv");
    args.experience_path = asset_dir.join("experience.ndkv");
    args.adaptive_path = asset_dir.join("adaptive.ndkv");
    args.trace_path = Some(asset_dir.join("trace.jsonl"));
    args.trace_schema_gate_path = args.trace_path.clone();

    let mut engine = NoironEngine::new();
    let mut backend = PartialBusinessBackend;
    let timed = run_timed_inference(
        &mut engine,
        &mut backend,
        GEMMA_BUSINESS_SMOKE_PROMPT.to_owned(),
        TaskProfile::Coding,
        args.trace_path.as_ref(),
        Some("gemma-business-runtime"),
    )
    .unwrap();
    record_gemma_business_smoke_contract(&mut engine, &timed.outcome, args.trace_path.as_ref())
        .unwrap();
    engine
        .save_full_state(
            &args.memory_path,
            &args.experience_path,
            &args.adaptive_path,
        )
        .unwrap();

    let replay = run_gemma_business_smoke_replay(&args).unwrap();
    let inspection = run_state_inspection(&args).unwrap();

    assert_eq!(replay.applied, 1);
    assert_eq!(replay.business_contract_items, 1);
    assert_eq!(replay.business_contract_passed, 1);
    assert_eq!(replay.business_contract_failed, 0);
    assert_eq!(replay.business_contract_raw_passed, 0);
    assert_eq!(replay.business_contract_raw_failed, 1);
    assert_eq!(replay.business_contract_response_normalized, 1);
    assert_eq!(replay.business_contract_canonical_fallbacks, 1);
    assert_eq!(inspection.evolution_ledger.replay_runs, 1);
    assert_eq!(inspection.evolution_ledger.replay_items, 1);
    assert_eq!(
        inspection.evolution_ledger.replay_business_contract_items,
        1
    );
    assert_eq!(
        inspection.evolution_ledger.replay_business_contract_passed,
        1
    );
    assert_eq!(
        inspection.evolution_ledger.replay_business_contract_failed,
        0
    );
    assert_eq!(
        inspection
            .evolution_ledger
            .replay_business_contract_raw_failed,
        1
    );
    assert_eq!(
        inspection
            .evolution_ledger
            .replay_business_contract_canonical_fallbacks,
        1
    );

    fs::remove_dir_all(asset_dir).unwrap();
}
