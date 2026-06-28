use super::*;

#[test]
fn gemma_business_cycle_smoke_report_json_records_gate_evidence() {
    let asset_dir = target_asset_dir("gemma-business-cycle-report-json");
    fs::create_dir_all(&asset_dir).unwrap();
    let memory = asset_dir.join("memory.ndkv");
    let experience = asset_dir.join("experience.ndkv");
    let adaptive = asset_dir.join("adaptive.ndkv");
    let trace = asset_dir.join("trace.jsonl");
    let response = asset_dir.join("business-cycle.response.json");
    let args = Args::parse(vec![
        "--memory".to_owned(),
        memory.display().to_string(),
        "--experience".to_owned(),
        experience.display().to_string(),
        "--adaptive".to_owned(),
        adaptive.display().to_string(),
        "--trace".to_owned(),
        trace.display().to_string(),
    ]);
    let business_case = GEMMA_MODEL_SERVICE_BUSINESS_CASES
        .iter()
        .find(|business_case| business_case.name == "gemma-service-rust-feedback")
        .unwrap();
    let answer = "runtime_model_experiences=audit telemetry; apply_user_feedback=interface; feedback=applied to memory.";
    let answer_audit = GemmaModelServiceAnswerAudit::from_case(business_case, answer);
    let cycle_body = format!(
        "{{\"ok\":true,\"business_cycle\":{{\"passed\":true,\"feedback_applied\":1,\"rust_check_checked\":true,\"rust_check_passed\":true,\"rust_check_feedback_applied\":1,\"self_improve_checked\":true,\"self_improve_passed\":true,\"state_gate_passed\":true,\"trace_gate_passed\":true}},\"generate\":{{\"answer\":{},\"runtime_model\":\"D:\\\\hf-cache\\\\gemma\",\"runtime_token_count\":24,\"runtime_uncertainty_signal\":false}},\"replay\":{{\"live_memory_feedback_applied\":1,\"live_evolution_items\":1}},\"state\":{{\"runtime_tokens\":24,\"evolution_external_feedbacks\":2,\"evolution_external_feedback_memory_updates\":2,\"evolution_replay_rust_check_passed\":1}},\"state_gate\":{{\"passed\":true}},\"trace_gate\":{{\"passed\":true,\"checked_lines\":3}}}}",
        service_json_string(answer)
    );
    let report = gemma_business_cycle_smoke_report_json(
        true,
        "127.0.0.1:7878",
        business_case,
        &args,
        Some(&response),
        "{\"ok\":true}",
        &cycle_body,
        &answer_audit,
        &[],
        24,
        1,
        1,
        3,
    );

    assert_eq!(
        json_string_field(&report, "schema").as_deref(),
        Some("rust-norion-gemma-business-cycle-smoke-v1")
    );
    assert_eq!(json_bool_field(&report, "passed"), Some(true));
    assert_eq!(
        json_bool_field(&report, "business_cycle_passed"),
        Some(true)
    );
    assert_eq!(
        json_string_field(&report, "business_case").as_deref(),
        Some("gemma-service-rust-feedback")
    );
    assert_eq!(json_u64_field(&report, "runtime_token_count"), Some(24));
    assert_eq!(json_u64_field(&report, "applied"), Some(1));
    assert_eq!(
        json_u64_field(&report, "rust_check_feedback_applied"),
        Some(1)
    );
    assert_eq!(
        json_u64_field(&report, "live_memory_feedback_applied"),
        Some(1)
    );
    assert_eq!(json_u64_field(&report, "live_evolution_items"), Some(1));
    assert_eq!(json_u64_field(&report, "checked_lines"), Some(3));
    assert!(report.contains("\"response\":"));
    assert!(report.contains("business-cycle.response.json"));
    assert!(report.contains("\"failures\":[]"));
    let gate = evaluate_gemma_business_cycle_smoke_report_gate_body(&report);
    assert!(gate.passed, "{:?}", gate.failures);
    assert_eq!(gate.runtime_token_count, 24);
    assert_eq!(gate.feedback_applied, 1);
    assert_eq!(gate.rust_check_feedback_applied, 1);
    assert_eq!(gate.external_feedbacks, 2);
    assert_eq!(gate.feedback_memory_updates, 2);
    assert_eq!(gate.replay_rust_check_passed, 1);
    assert_eq!(gate.replay_live_memory_feedback_applied, 1);
    assert_eq!(gate.replay_live_evolution_items, 1);
    assert_eq!(gate.checked_trace_lines, 3);

    let bad_report = report.replace("\"runtime_token_count\":24", "\"runtime_token_count\":0");
    let bad_gate = evaluate_gemma_business_cycle_smoke_report_gate_body(&bad_report);
    assert!(!bad_gate.passed);
    assert!(
        bad_gate
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_token_count 0")),
        "{:?}",
        bad_gate.failures
    );

    let report_path = asset_dir.join("gemma-business-cycle-smoke-report.json");
    fs::write(&report_path, &report).unwrap();
    let artifact_gate = evaluate_gemma_business_cycle_smoke_report_gate(&report_path).unwrap();
    assert!(!artifact_gate.passed);
    assert!(
        artifact_gate
            .failures
            .iter()
            .any(|failure| failure.contains("artifact path trace missing")),
        "{:?}",
        artifact_gate.failures
    );

    fs::write(&memory, "broken-memory-state\n").unwrap();
    fs::write(&experience, "broken-experience-state\n").unwrap();
    fs::write(&adaptive, "broken-adaptive-state\n").unwrap();
    fs::write(&trace, "{}\n").unwrap();
    fs::write(&response, &cycle_body).unwrap();
    let bad_state_artifact_gate =
        evaluate_gemma_business_cycle_smoke_report_gate(&report_path).unwrap();
    assert!(!bad_state_artifact_gate.passed);
    assert!(
        bad_state_artifact_gate
            .failures
            .iter()
            .any(|failure| failure.contains("state artifact gate failure")
                || failure.contains("state artifacts could not be loaded")),
        "{:?}",
        bad_state_artifact_gate.failures
    );

    let gate_args = Args::parse(vec![
        "--gemma-business-cycle-smoke-report-gate".to_owned(),
        response.display().to_string(),
    ]);
    assert_eq!(
        gate_args
            .gemma_business_cycle_smoke_report_gate_path
            .as_ref(),
        Some(&response)
    );
    let regression_gate_args = Args::parse(vec![
        "--gemma-business-regression-gate".to_owned(),
        response.display().to_string(),
    ]);
    assert_eq!(
        regression_gate_args
            .gemma_business_regression_gate_path
            .as_ref(),
        Some(&response)
    );
    let short_regression_gate_args = Args::parse(vec![
        "--business-regression-gate".to_owned(),
        response.display().to_string(),
    ]);
    assert_eq!(
        short_regression_gate_args
            .gemma_business_regression_gate_path
            .as_ref(),
        Some(&response)
    );
    assert_eq!(
        gemma_business_regression_report_path(&asset_dir),
        asset_dir.join(GEMMA_BUSINESS_CYCLE_SMOKE_REPORT_FILE)
    );
    assert_eq!(
        gemma_business_regression_report_path(&report_path),
        report_path
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn gemma_business_cycle_smoke_matrix_report_records_all_business_cases() {
    let asset_dir = target_asset_dir("gemma-business-cycle-matrix-report-json");
    fs::create_dir_all(&asset_dir).unwrap();
    let memory = asset_dir.join("memory.ndkv");
    let experience = asset_dir.join("experience.ndkv");
    let adaptive = asset_dir.join("adaptive.ndkv");
    let trace = asset_dir.join("trace.jsonl");
    let response = asset_dir.join("business-cycle.response.json");
    let args = Args::parse(vec![
        "--memory".to_owned(),
        memory.display().to_string(),
        "--experience".to_owned(),
        experience.display().to_string(),
        "--adaptive".to_owned(),
        adaptive.display().to_string(),
        "--trace".to_owned(),
        trace.display().to_string(),
    ]);
    let mut case_results = Vec::new();
    for (index, business_case) in GEMMA_MODEL_SERVICE_BUSINESS_CASES.iter().enumerate() {
        let runtime_tokens = 10 + index as u64;
        let checked_lines = ((index + 1) * 3) as u64;
        let state_external_feedbacks = ((index + 1) * 2) as u64;
        let answer = business_case.contract_line.to_owned();
        let body = format!(
            "{{\"ok\":true,\"business_cycle\":{{\"passed\":true,\"feedback_applied\":1,\"rust_check_checked\":true,\"rust_check_passed\":true,\"rust_check_feedback_applied\":1,\"self_improve_checked\":true,\"self_improve_passed\":true,\"state_gate_passed\":true,\"trace_gate_passed\":true}},\"generate\":{{\"answer\":{},\"runtime_model\":\"D:\\\\hf-cache\\\\gemma\",\"runtime_token_count\":{},\"runtime_uncertainty_signal\":false}},\"replay\":{{\"live_memory_feedback_applied\":1,\"live_evolution_items\":1}},\"state\":{{\"runtime_tokens\":{},\"evolution_external_feedbacks\":{},\"evolution_external_feedback_memory_updates\":{},\"evolution_replay_rust_check_passed\":{}}},\"state_gate\":{{\"passed\":true}},\"trace_gate\":{{\"passed\":true,\"checked_lines\":{}}}}}",
            service_json_string(&answer),
            runtime_tokens,
            runtime_tokens,
            state_external_feedbacks,
            state_external_feedbacks,
            index as u64 + 1,
            checked_lines
        );
        case_results.push(GemmaBusinessCycleCaseResult {
            name: business_case.name,
            body,
            answer: answer.clone(),
            answer_audit: GemmaModelServiceAnswerAudit::from_case(business_case, &answer),
            runtime_token_count: runtime_tokens,
            feedback_applied: 1,
            rust_check_feedback_applied: 1,
            checked_trace_lines: checked_lines,
            passed: true,
        });
    }
    let total_runtime_tokens = case_results
        .iter()
        .map(|result| result.runtime_token_count)
        .sum::<u64>();
    let checked_trace_lines = case_results
        .iter()
        .map(|result| result.checked_trace_lines)
        .max()
        .unwrap();
    let report = gemma_business_cycle_smoke_matrix_report_json(
        true,
        "127.0.0.1:7878",
        &args,
        Some(&response),
        "{\"ok\":true}",
        &case_results.last().unwrap().body,
        &case_results,
        &[],
        total_runtime_tokens,
        case_results.len() as u64,
        case_results.len() as u64,
        checked_trace_lines,
    );

    assert_eq!(
        json_string_field(&report, "business_case").as_deref(),
        Some("gemma-business-cycle-matrix")
    );
    assert_eq!(
        json_u64_field(&report, "case_count"),
        Some(GEMMA_MODEL_SERVICE_BUSINESS_CASES.len() as u64)
    );
    assert_eq!(
        json_u64_field(&report, "passed_cases"),
        Some(GEMMA_MODEL_SERVICE_BUSINESS_CASES.len() as u64)
    );
    assert_eq!(
        json_u64_field(&report, "runtime_token_count"),
        Some(total_runtime_tokens)
    );
    for business_case in &GEMMA_MODEL_SERVICE_BUSINESS_CASES {
        assert!(report.contains(business_case.name), "{report}");
    }
    let gate = evaluate_gemma_business_cycle_smoke_report_gate_body(&report);
    assert!(gate.passed, "{:?}", gate.failures);
    assert_eq!(
        gate.case_count,
        GEMMA_MODEL_SERVICE_BUSINESS_CASES.len() as u64
    );
    assert_eq!(gate.runtime_token_count, total_runtime_tokens);
    assert_eq!(gate.feedback_applied, case_results.len() as u64);
    assert_eq!(gate.rust_check_feedback_applied, case_results.len() as u64);
    assert_eq!(gate.external_feedbacks, (case_results.len() * 2) as u64);
    assert_eq!(
        gate.feedback_memory_updates,
        (case_results.len() * 2) as u64
    );
    assert_eq!(gate.checked_trace_lines, checked_trace_lines);

    let bad_report = report.replacen(
        &format!(
            "\"passed_cases\":{}",
            GEMMA_MODEL_SERVICE_BUSINESS_CASES.len()
        ),
        "\"passed_cases\":1",
        1,
    );
    let bad_gate = evaluate_gemma_business_cycle_smoke_report_gate_body(&bad_report);
    assert!(!bad_gate.passed);
    assert!(
        bad_gate
            .failures
            .iter()
            .any(|failure| failure.contains("passed_cases 1")),
        "{:?}",
        bad_gate.failures
    );

    NoironEngine::new()
        .save_full_state(&memory, &experience, &adaptive)
        .unwrap();
    fs::write(
        &response,
        gemma_business_cycle_smoke_aggregate_response_json(
            true,
            &case_results,
            total_runtime_tokens,
            case_results.len() as u64,
            case_results.len() as u64,
            checked_trace_lines,
        ),
    )
    .unwrap();
    fs::write(&trace, "{}\n").unwrap();
    let report_path = asset_dir.join("gemma-business-cycle-smoke-report.json");
    fs::write(&report_path, &report).unwrap();
    let artifact_gate = evaluate_gemma_business_cycle_smoke_report_gate(&report_path).unwrap();
    assert!(!artifact_gate.passed);
    assert!(
        artifact_gate.failures.iter().any(|failure| failure
            .contains("runtime_model_experience_count 0 below required 3")
            || failure.contains("state artifact runtime_model_experience_count 0 below report 3")),
        "{:?}",
        artifact_gate.failures
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_state_json_includes_gate_evidence() {
    let engine = NoironEngine::new();
    let mut inspection = StateInspectionReport::from_engine(&engine, 1);
    inspection.pool_dispatch_experience_count = 1;
    inspection.pool_dispatch_item_count = 2;
    inspection.pool_dispatch_forwarded_count = 1;
    inspection.pool_dispatch_clamped_count = 1;
    inspection.pool_dispatch_low_priority_count = 1;
    inspection.reflection_issue_experience_count = 2;
    inspection.critical_reflection_issue_experience_count = 1;
    inspection.revision_action_experience_count = 3;
    inspection.live_memory_feedback_experience_count = 2;
    inspection.live_memory_feedback_update_count = 4;
    inspection.live_memory_feedback_detail_experience_count = 1;
    inspection.live_memory_feedback_applied_count = 3;
    inspection.live_memory_feedback_removed_count = 1;
    inspection.live_memory_feedback_missing_count = 2;
    inspection.live_memory_feedback_strength_delta = 1.25;
    inspection.runtime_adapter_experience_count = 2;
    inspection.runtime_adapter_selection_mismatch_count = 1;
    inspection.runtime_forward_energy_experience_count = 3;
    inspection.runtime_kv_influence_experience_count = 4;
    inspection.runtime_kv_precision_mismatch_count = 1;
    inspection.runtime_uncertainty_experience_count = 2;
    inspection.runtime_uncertainty_token_count = 9;
    inspection.runtime_layer_mode_experience_count = 2;
    inspection.runtime_all_layer_mode_experience_count = 1;
    inspection.runtime_global_layers = 5;
    inspection.runtime_local_window_layers = 7;
    inspection.runtime_convolutional_fusion_layers = 11;
    inspection.runtime_kv_import_experience_count = 1;
    inspection.runtime_kv_weak_import_skip_experience_count = 2;
    inspection.weak_runtime_kv_imports_skipped = 6;
    inspection.runtime_kv_weak_import_pressure_experience_count = 2;
    inspection.runtime_kv_weak_import_pressure_avg = 0.5;
    inspection.runtime_kv_weak_import_pressure_max = 0.75;
    inspection.runtime_kv_budget_import_skip_experience_count = 2;
    inspection.budget_limited_runtime_kv_imports_skipped = 5;
    inspection.runtime_kv_budget_pressure_experience_count = 2;
    inspection.runtime_kv_budget_pressure_avg = 0.625;
    inspection.runtime_kv_budget_pressure_max = 1.0;
    inspection.runtime_kv_export_experience_count = 1;
    inspection.runtime_kv_segment_experience_count = 1;
    inspection.runtime_kv_segments_included = 3;
    inspection.runtime_kv_segments_skipped = 2;
    inspection.runtime_kv_segments_rejected = 1;
    inspection.runtime_kv_hold_experience_count = 1;
    inspection.runtime_kv_held_blocks = 4;
    inspection.fht_dke_budget_experience_count = 2;
    inspection.fht_dke_enabled_experience_count = 1;
    inspection.fht_dke_total_tokens = 64;
    inspection.fht_dke_dense_tokens = 20;
    inspection.fht_dke_routed_tokens = 44;
    inspection.fht_dke_kv_exchange_blocks = 5;
    inspection.fht_dke_token_split_valid_count = 2;
    inspection.fht_dke_token_split_invalid_count = 1;
    inspection.fht_dke_attention_threshold_experience_count = 2;
    inspection.fht_dke_attention_threshold_avg = 0.625;
    inspection.fht_dke_attention_threshold_max = 0.75;
    inspection.fht_dke_route_pressure_experience_count = 2;
    inspection.fht_dke_route_pressure_avg = 0.6875;
    inspection.fht_dke_route_pressure_max = 0.875;
    inspection.experience_index_overlong_record_count = 2;
    inspection.experience_index_overlong_without_clean_gist_count = 1;
    inspection.experience_index_max_record_chars = 4096;
    inspection.experience_index_noisy_record_count = 3;
    inspection.experience_index_duplicate_output_count = 1;
    inspection.experience_index_max_noise_penalty = 0.375;
    inspection.experience_index_quality_score = 0.8125;
    inspection.experience_index_retrieval_ready = true;
    inspection.experience_index_risk_level = "watch".to_owned();
    inspection.process_reward_experience_count = 3;
    inspection.process_reward_positive_count = 2;
    inspection.process_reward_reinforce_count = 1;
    inspection.process_reward_hold_count = 1;
    inspection.process_reward_penalize_count = 1;
    inspection.process_reward_total = 1.75;
    inspection.live_memory_feedback_reinforced_count = 2;
    inspection.live_memory_feedback_penalized_count = 1;
    inspection.external_semantic_context_experience_count = 2;
    inspection.external_semantic_context_count = 5;
    inspection.self_evolving_memory_writeback_experience_count = 1;
    inspection.self_evolving_memory_writeback_attempted_records = 3;
    inspection.self_evolving_memory_writeback_accepted_records = 3;
    inspection.self_evolving_memory_writeback_records_before = 2;
    inspection.self_evolving_memory_writeback_records_after = 6;
    inspection.self_evolving_memory_writeback_tool_reliability_after = 1;
    inspection.self_evolving_memory_writeback_tool_observations_after = 2;
    inspection.self_evolving_memory_writeback_maintenance_actions = 1;
    inspection.self_evolving_memory_writeback_merged_duplicate_episodes = 1;
    inspection.self_evolving_memory_writeback_write_allowed = 1;
    inspection.self_evolving_memory_writeback_durable_write_allowed = 1;
    inspection.self_evolving_memory_writeback_applied = 1;
    inspection.self_evolving_memory_writeback_applied_to_disk = 1;
    inspection.self_evolving_memory_writeback_snapshot_changes = 1;
    inspection.router_observations = 42;
    inspection.profile_thresholds = rust_norion::ProfileThresholds {
        general: 0.31,
        coding: 0.41,
        writing: 0.51,
        long_document: 0.61,
    };
    inspection.profile_observations = rust_norion::ProfileObservations {
        general: 4,
        coding: 3,
        writing: 2,
        long_document: 1,
    };
    inspection.hierarchy = rust_norion::HierarchyWeights::new(0.2, 0.3, 0.5);
    inspection.profile_hierarchy_weights = rust_norion::ProfileHierarchyWeights {
        general: rust_norion::HierarchyWeights::new(0.1, 0.7, 0.2),
        coding: rust_norion::HierarchyWeights::new(0.2, 0.6, 0.2),
        writing: rust_norion::HierarchyWeights::new(0.3, 0.5, 0.2),
        long_document: rust_norion::HierarchyWeights::new(0.4, 0.4, 0.2),
    };
    inspection.profile_hierarchy_observations = rust_norion::ProfileHierarchyObservations {
        general: 8,
        coding: 6,
        writing: 4,
        long_document: 2,
    };
    inspection.tier_counts = rust_norion::TierCounts {
        hot_gpu: 1,
        warm_ram: 2,
        cold_disk: 3,
    };
    inspection.memory_vector_dimensions = vec![rust_norion::StateMemoryVectorDimensions {
        dimensions: 384,
        count: 2,
    }];
    inspection.runtime_kv_vector_dimensions = vec![rust_norion::StateMemoryVectorDimensions {
        dimensions: 128,
        count: 3,
    }];
    inspection.top_memories = vec![rust_norion::StateMemorySummary {
        id: 11,
        key: "do-not-leak-memory-key".to_owned(),
        vector_dimensions: 384,
        strength: 0.9,
        hits: 7,
        failures: 1,
        last_score: 0.8,
    }];
    inspection.top_runtime_kv_memories = vec![rust_norion::StateMemorySummary {
        id: 12,
        key: "do-not-leak-runtime-kv-key".to_owned(),
        vector_dimensions: 128,
        strength: 0.7,
        hits: 5,
        failures: 2,
        last_score: 0.6,
    }];
    inspection.top_experiences = vec![rust_norion::StateExperienceSummary {
        id: 21,
        profile: rust_norion::TaskProfile::Coding,
        quality: 0.93,
        process_reward: 1.25,
        reward_action: rust_norion::RewardAction::Reinforce,
        runtime_model_id: Some("do-not-leak-model-id".to_owned()),
        runtime_selected_adapter: Some("do-not-leak-adapter".to_owned()),
        runtime_device_profile: None,
        runtime_primary_lane: None,
        runtime_fallback_lane: None,
        runtime_memory_mode: None,
        runtime_layer_count: 0,
        runtime_global_layers: 0,
        runtime_local_window_layers: 0,
        runtime_convolutional_fusion_layers: 0,
        runtime_hidden_size: 0,
        runtime_local_window_tokens: 0,
        runtime_forward_energy: None,
        runtime_kv_influence: None,
        runtime_token_count: 44,
        runtime_uncertainty_token_count: 0,
        runtime_uncertainty_perplexity: None,
        runtime_hot_kv_precision_bits: None,
        runtime_cold_kv_precision_bits: None,
        runtime_imported_kv_blocks: 2,
        runtime_weak_kv_imports_skipped: 1,
        runtime_budget_limited_kv_imports_skipped: 2,
        runtime_exported_kv_blocks: 3,
        runtime_kv_segments_included: 4,
        runtime_kv_segments_skipped: 5,
        runtime_kv_segments_rejected: 6,
        recursive_runtime_calls: Some(2),
        external_semantic_contexts: 4,
        self_evolving_memory_writeback_attempted_records: 3,
        self_evolving_memory_writeback_accepted_records: 3,
        self_evolving_memory_writeback_records_before: 3,
        self_evolving_memory_writeback_records_after: 6,
        self_evolving_memory_writeback_tool_reliability_after: 1,
        self_evolving_memory_writeback_tool_observations_after: 2,
        self_evolving_memory_writeback_maintenance_actions: 1,
        self_evolving_memory_writeback_merged_duplicate_episodes: 1,
        self_evolving_memory_writeback_write_allowed: 1,
        self_evolving_memory_writeback_durable_write_allowed: 1,
        self_evolving_memory_writeback_applied: 1,
        self_evolving_memory_writeback_applied_to_disk: 1,
        self_evolving_memory_writeback_snapshot_changes: 1,
        runtime_errors: 1,
        runtime_timeouts: 0,
        runtime_error_message_chars: 9,
        live_online_reward_feedbacks: 0,
        live_online_reward_reinforcements: 0,
        live_online_reward_penalties: 0,
        live_memory_feedback_updates: 0,
        live_memory_feedback_reinforced: 1,
        live_memory_feedback_penalized: 1,
        live_memory_feedback_applied: 0,
        live_memory_feedback_removed: 0,
        live_memory_feedback_missing: 1,
        live_memory_feedback_strength_delta: 0.0,
        live_memory_feedback_detail: false,
        rust_check_passed: 0,
        rust_check_failed: 0,
        rust_check_diagnostic_chars: 0,
        business_contract_passed: 0,
        business_contract_failed: 0,
        business_contract_missing_signals: 0,
        business_contract_protocol_leaks: 0,
        business_contract_substitutions: 0,
        business_contract_evasive_denials: 0,
        business_contract_missing_handling_signals: 0,
        business_contract_raw_passed: 0,
        business_contract_raw_failed: 0,
        business_contract_response_normalized: 0,
        business_contract_sanitized: 0,
        business_contract_canonical_fallbacks: 0,
        pool_dispatch_items: 0,
        pool_dispatch_selected_roles: Vec::new(),
        pool_dispatch_forwarded: 0,
        pool_dispatch_clamped: 0,
        pool_dispatch_low_priority: 0,
        reflection_issues: 2,
        critical_reflection_issues: 1,
        revision_actions: 2,
    }];
    inspection.memory_retention_policy = rust_norion::MemoryRetentionPolicy {
        stale_after: 16,
        decay_rate: 0.125,
        remove_below_strength: 0.0625,
        remove_after_failures: 3,
    };
    inspection.memory_compaction_policy = rust_norion::MemoryCompactionPolicy {
        similarity_threshold: 0.875,
        max_candidates: 24,
        max_merges: 5,
    };
    inspection.evolution_ledger = rust_norion::EvolutionLedger {
        live_inference_runs: 2,
        live_router_threshold_mutations: 1,
        live_hierarchy_weight_mutations: 1,
        live_router_threshold_delta: 0.125,
        live_hierarchy_weight_delta: 0.25,
        live_online_reward_feedbacks: 5,
        live_online_reward_reinforcements: 4,
        live_online_reward_penalties: 1,
        live_online_reward_strength: 0.75,
        live_online_reward_reinforcement_strength: 0.6,
        live_online_reward_penalty_strength: 0.15,
        live_memory_reinforcements: 3,
        live_memory_penalties: 2,
        live_stored_memories: 1,
        live_stored_gist_memories: 2,
        live_stored_runtime_kv_memories: 1,
        live_reflection_issues: 6,
        live_critical_reflection_issues: 1,
        live_revision_actions: 7,
        router_threshold_mutations: 8,
        hierarchy_weight_mutations: 6,
        router_threshold_delta: 0.42,
        hierarchy_weight_delta: 0.21,
        memory_reinforcements: 4,
        memory_penalties: 3,
        external_feedbacks: 3,
        external_feedback_reinforcements: 2,
        external_feedback_penalties: 1,
        external_feedback_memory_updates: 5,
        external_feedback_removed: 1,
        external_feedback_missing: 2,
        external_feedback_strength_delta: 0.5,
        replay_live_memory_feedback_items: 4,
        replay_live_memory_feedback_reinforcements: 3,
        replay_live_memory_feedback_penalties: 2,
        replay_live_memory_feedback_detail_items: 2,
        replay_live_memory_feedback_applied: 4,
        replay_live_memory_feedback_removed: 1,
        replay_live_memory_feedback_missing: 1,
        replay_live_memory_feedback_strength_delta: 0.875,
        replay_rust_check_diagnostic_chars: 13,
        replay_rust_check_live_memory_feedback_items: 2,
        replay_live_evolution_items: 6,
        replay_live_evolution_router_threshold_mutations: 2,
        replay_live_evolution_hierarchy_weight_mutations: 3,
        replay_live_evolution_router_threshold_delta: 0.375,
        replay_live_evolution_hierarchy_weight_delta: 0.625,
        replay_live_evolution_online_reward_feedbacks: 7,
        replay_live_evolution_online_reward_reinforcements: 5,
        replay_live_evolution_online_reward_penalties: 2,
        replay_live_evolution_online_reward_strength: 1.125,
        replay_live_evolution_online_reward_reinforcement_strength: 0.75,
        replay_live_evolution_online_reward_penalty_strength: 0.25,
        replay_live_evolution_memory_updates: 9,
        replay_live_evolution_stored_memory_updates: 4,
        replay_live_evolution_reflection_issues: 3,
        replay_live_evolution_critical_reflection_issues: 1,
        replay_live_evolution_revision_actions: 2,
        recursive_replay_items: 2,
        recursive_runtime_calls: 8,
        drift_rollbacks: 1,
        rollback_router_threshold_delta: 0.05,
        rollback_hierarchy_weight_delta: 0.07,
        ..rust_norion::EvolutionLedger::default()
    };
    let state_gate = StateInspectionGateReport {
        passed: false,
        failures: vec!["memory_count below minimum".to_owned()],
    };
    let trace_gate = TraceSchemaGateReport {
        passed: true,
        checked_lines: 1,
        rust_check_events: 0,
        rust_check_passed: 0,
        rust_check_failed: 0,
        rust_check_feedback_updates: 0,
        rust_check_feedback_applied: 0,
        business_contract_events: 0,
        business_contract_event_passed: 0,
        business_contract_event_failed: 0,
        business_contract_event_missing_signals: 0,
        business_contract_event_protocol_leaks: 0,
        business_contract_event_substitutions: 0,
        business_contract_event_evasive_denials: 0,
        business_contract_event_raw_passed: 0,
        business_contract_event_raw_failed: 0,
        business_contract_event_response_normalized: 0,
        business_contract_event_sanitized: 0,
        business_contract_event_canonical_fallbacks: 0,
        runtime_error_events: 0,
        runtime_timeout_events: 0,
        self_evolving_memory_store_events: 4,
        self_evolving_memory_store_retrieval_events: 1,
        self_evolving_memory_store_maintenance_events: 1,
        self_evolving_memory_store_admission_preview_events: 1,
        self_evolving_memory_store_consolidation_events: 1,
        self_evolving_memory_store_consolidation_actions: 3,
        self_evolving_memory_store_merge_previews: 1,
        self_evolving_memory_store_decay_previews: 1,
        self_evolving_memory_store_tombstone_previews: 1,
        self_evolving_memory_store_merge_rejections: 0,
        self_evolving_memory_store_contexts: 3,
        self_evolving_memory_store_saved_tokens: 96,
        self_evolving_memory_store_maintenance_actions: 2,
        self_evolving_memory_store_admission_candidates: 4,
        self_evolving_memory_store_write_allowed: 0,
        self_evolving_memory_store_applied: 0,
        self_evolving_memory_store_applied_to_disk: 0,
        self_evolving_memory_writeback_events: 1,
        self_evolving_memory_writeback_source_case_digests: 1,
        self_evolving_memory_writeback_attempted_records: 3,
        self_evolving_memory_writeback_accepted_records: 3,
        self_evolving_memory_writeback_records_before: 3,
        self_evolving_memory_writeback_records_after: 6,
        self_evolving_memory_writeback_tool_reliability_after: 1,
        self_evolving_memory_writeback_tool_observations_after: 2,
        self_evolving_memory_writeback_maintenance_actions: 1,
        self_evolving_memory_writeback_merged_duplicate_episodes: 1,
        self_evolving_memory_writeback_write_allowed: 1,
        self_evolving_memory_writeback_durable_write_allowed: 1,
        self_evolving_memory_writeback_applied: 1,
        self_evolving_memory_writeback_applied_to_disk: 1,
        self_evolving_memory_writeback_snapshot_changes: 1,
        self_evolution_admission_events: 0,
        self_evolution_admission_admitted: 0,
        self_evolution_admission_blocked: 0,
        self_evolution_admission_review_packets: 0,
        self_evolution_admission_evidence_ids: 0,
        self_evolution_admission_missing_review_packet_refs: 0,
        improvement_corpus_events: 0,
        improvement_corpus_episodes: 0,
        improvement_corpus_active_adaptation: 0,
        improvement_corpus_compiler_passed: 0,
        improvement_corpus_test_passed: 0,
        improvement_corpus_benchmark_passed: 0,
        improvement_corpus_privacy_rejected: 0,
        improvement_corpus_secret_leaks: 0,
        failures: Vec::new(),
        ..TraceSchemaGateReport::default()
    };

    let args = Args::parse(Vec::new());
    let body = model_service_state_response_json(
        7,
        &args,
        &inspection,
        Some(&state_gate),
        Some(&trace_gate),
    );

    assert!(body.contains("\"request_id\":7"));
    assert!(body.contains("\"runtime_state_bucket\":{"));
    assert!(body.contains("\"current\":\"state\\\\rust-norion-v"));
    assert!(body.contains("\"runtime_model_experiences\":0"));
    assert!(body.contains("\"experience_hygiene_findings\":0"));
    assert!(body.contains("\"experience_hygiene_quarantine_candidates\":0"));
    assert!(body.contains("\"experience_repairable_legacy_metadata_lessons\":0"));
    assert!(body.contains("\"experience_repairable_index_records\":0"));
    assert!(body.contains("\"experience_repair_projected_findings\":0"));
    assert!(body.contains("\"experience_repair_projected_quarantine_candidates\":0"));
    assert!(body.contains("\"experience_repair_projected_legacy_metadata_lessons\":0"));
    assert!(body.contains("\"experience_repair_skipped_quarantine_candidates\":0"));
    assert!(body.contains("\"experience_repair_skipped_missing_clean_gist\":0"));
    assert!(body.contains("\"experience_hygiene_clean\":true"));
    assert!(body.contains("\"experience_hygiene_samples\":[]"));
    assert!(body.contains("\"experience_index_compacted_records\":0"));
    assert!(body.contains("\"experience_index_overlong_records\":2"));
    assert!(body.contains("\"experience_index_overlong_without_clean_gist\":1"));
    assert!(body.contains("\"experience_index_max_record_chars\":4096"));
    assert!(body.contains("\"experience_index_noisy_records\":3"));
    assert!(body.contains("\"experience_index_duplicate_outputs\":1"));
    assert!(body.contains("\"experience_index_max_noise_penalty\":0.375000"));
    assert!(body.contains("\"experience_index_quality_score\":0.812500"));
    assert!(body.contains("\"experience_index_retrieval_ready\":true"));
    assert!(body.contains("\"experience_index_risk_level\":\"watch\""));
    assert!(body.contains("\"experience_index_samples\":[]"));
    assert!(body.contains("\"process_reward_experiences\":3"));
    assert!(body.contains("\"process_reward_positive\":2"));
    assert!(body.contains("\"process_reward_reinforce\":1"));
    assert!(body.contains("\"process_reward_hold\":1"));
    assert!(body.contains("\"process_reward_penalize\":1"));
    assert!(body.contains("\"process_reward_total\":1.750000"));
    assert!(body.contains("\"live_memory_feedback_reinforced\":2"));
    assert!(body.contains("\"live_memory_feedback_penalized\":1"));
    assert!(body.contains("\"external_semantic_context_experiences\":2"));
    assert!(body.contains("\"external_semantic_contexts\":5"));
    assert!(body.contains("\"self_evolving_memory_writeback_experiences\":1"));
    assert!(body.contains("\"self_evolving_memory_writeback_attempted_records\":3"));
    assert!(body.contains("\"self_evolving_memory_writeback_accepted_records\":3"));
    assert!(body.contains("\"self_evolving_memory_writeback_rejected_records\":0"));
    assert!(body.contains("\"self_evolving_memory_writeback_records_before\":2"));
    assert!(body.contains("\"self_evolving_memory_writeback_records_after\":6"));
    assert!(body.contains("\"self_evolving_memory_writeback_tool_reliability_after\":1"));
    assert!(body.contains("\"self_evolving_memory_writeback_tool_observations_after\":2"));
    assert!(body.contains("\"self_evolving_memory_writeback_maintenance_actions\":1"));
    assert!(body.contains("\"self_evolving_memory_writeback_merged_duplicate_episodes\":1"));
    assert!(body.contains("\"self_evolving_memory_writeback_write_allowed\":1"));
    assert!(body.contains("\"self_evolving_memory_writeback_durable_write_allowed\":1"));
    assert!(body.contains("\"self_evolving_memory_writeback_applied\":1"));
    assert!(body.contains("\"self_evolving_memory_writeback_applied_to_disk\":1"));
    assert!(body.contains("\"self_evolving_memory_writeback_snapshot_changes\":1"));
    assert!(body.contains("\"runtime_adapter_experiences\":2"));
    assert!(body.contains("\"runtime_adapter_selection_mismatches\":1"));
    assert!(body.contains("\"runtime_forward_energy_experiences\":3"));
    assert!(body.contains("\"runtime_kv_influence_experiences\":4"));
    assert!(body.contains("\"runtime_kv_precision_mismatches\":1"));
    assert!(body.contains("\"runtime_tokens\":0"));
    assert!(body.contains("\"runtime_uncertainty_experiences\":2"));
    assert!(body.contains("\"runtime_uncertainty_tokens\":9"));
    assert!(body.contains("\"runtime_layer_mode_experiences\":2"));
    assert!(body.contains("\"runtime_all_layer_mode_experiences\":1"));
    assert!(body.contains("\"runtime_global_layers\":5"));
    assert!(body.contains("\"runtime_local_window_layers\":7"));
    assert!(body.contains("\"runtime_convolutional_fusion_layers\":11"));
    assert!(body.contains("\"runtime_kv_import_experiences\":1"));
    assert!(body.contains("\"runtime_imported_kv_blocks\":0"));
    assert!(body.contains("\"runtime_kv_export_experiences\":1"));
    assert!(body.contains("\"runtime_kv_weak_import_skip_experiences\":2"));
    assert!(body.contains("\"weak_runtime_kv_imports_skipped\":6"));
    assert!(body.contains("\"runtime_kv_weak_import_pressure_experiences\":2"));
    assert!(body.contains("\"runtime_kv_weak_import_pressure_avg\":0.500000"));
    assert!(body.contains("\"runtime_kv_weak_import_pressure_max\":0.750000"));
    assert!(body.contains("\"runtime_kv_budget_import_skip_experiences\":2"));
    assert!(body.contains("\"budget_limited_runtime_kv_imports_skipped\":5"));
    assert!(body.contains("\"runtime_kv_budget_pressure_experiences\":2"));
    assert!(body.contains("\"runtime_kv_budget_pressure_avg\":0.625000"));
    assert!(body.contains("\"runtime_kv_budget_pressure_max\":1.000000"));
    assert!(body.contains("\"runtime_kv_segment_experiences\":1"));
    assert!(body.contains("\"runtime_kv_segments_included\":3"));
    assert!(body.contains("\"runtime_kv_segments_skipped\":2"));
    assert!(body.contains("\"runtime_kv_segments_rejected\":1"));
    assert!(body.contains("\"runtime_kv_hold_experiences\":1"));
    assert!(body.contains("\"runtime_kv_held_blocks\":4"));
    assert!(body.contains("\"fht_dke_budget_experiences\":2"));
    assert!(body.contains("\"fht_dke_enabled_experiences\":1"));
    assert!(body.contains("\"fht_dke_total_tokens\":64"));
    assert!(body.contains("\"fht_dke_dense_tokens\":20"));
    assert!(body.contains("\"fht_dke_routed_tokens\":44"));
    assert!(body.contains("\"fht_dke_kv_exchange_blocks\":5"));
    assert!(body.contains("\"fht_dke_token_split_valid\":2"));
    assert!(body.contains("\"fht_dke_token_split_invalid\":1"));
    assert!(body.contains("\"fht_dke_attention_threshold_experiences\":2"));
    assert!(body.contains("\"fht_dke_attention_threshold_avg\":0.625000"));
    assert!(body.contains("\"fht_dke_attention_threshold_max\":0.750000"));
    assert!(body.contains("\"fht_dke_route_pressure_experiences\":2"));
    assert!(body.contains("\"fht_dke_route_pressure_avg\":0.687500"));
    assert!(body.contains("\"fht_dke_route_pressure_max\":0.875000"));
    assert!(body.contains("\"router_observations\":42"));
    assert!(body.contains("\"profile_observations_general\":4"));
    assert!(body.contains("\"profile_observations_coding\":3"));
    assert!(body.contains("\"profile_observations_writing\":2"));
    assert!(body.contains("\"profile_observations_long_document\":1"));
    assert!(body.contains("\"profile_threshold_general\":0.310000"));
    assert!(body.contains("\"profile_threshold_coding\":0.410000"));
    assert!(body.contains("\"profile_threshold_writing\":0.510000"));
    assert!(body.contains("\"profile_threshold_long_document\":0.610000"));
    assert!(body.contains("\"hierarchy_global\":0.200000"));
    assert!(body.contains("\"hierarchy_local\":0.300000"));
    assert!(body.contains("\"hierarchy_convolution\":0.500000"));
    assert!(body.contains("\"profile_hierarchy_observations_general\":8"));
    assert!(body.contains("\"profile_hierarchy_observations_coding\":6"));
    assert!(body.contains("\"profile_hierarchy_observations_writing\":4"));
    assert!(body.contains("\"profile_hierarchy_observations_long_document\":2"));
    assert!(body.contains("\"profile_hierarchy_local_general\":0.700000"));
    assert!(body.contains("\"profile_hierarchy_local_coding\":0.600000"));
    assert!(body.contains("\"profile_hierarchy_local_writing\":0.500000"));
    assert!(body.contains("\"profile_hierarchy_local_long_document\":0.400000"));
    assert!(body.contains("\"memory_tier_hot_gpu\":1"));
    assert!(body.contains("\"memory_tier_warm_ram\":2"));
    assert!(body.contains("\"memory_tier_cold_disk\":3"));
    assert!(body.contains("\"memory_vector_dimensions\":[{\"dimensions\":384,\"count\":2}]"));
    assert!(body.contains("\"runtime_kv_vector_dimensions\":[{\"dimensions\":128,\"count\":3}]"));
    assert!(body.contains("\"top_memory_metrics\":[{\"id\":11,\"vector_dimensions\":384,\"strength\":0.900000,\"hits\":7,\"failures\":1,\"last_score\":0.800000}]"));
    assert!(body.contains("\"top_runtime_kv_memory_metrics\":[{\"id\":12,\"vector_dimensions\":128,\"strength\":0.700000,\"hits\":5,\"failures\":2,\"last_score\":0.600000}]"));
    assert!(!body.contains("do-not-leak-memory-key"));
    assert!(!body.contains("do-not-leak-runtime-kv-key"));
    assert!(body.contains("\"top_experience_metrics\":[{\"id\":21,\"profile\":\"coding\""));
    assert!(body.contains("\"runtime_imported_kv_blocks\":2"));
    assert!(body.contains("\"runtime_weak_kv_imports_skipped\":1"));
    assert!(body.contains("\"runtime_budget_limited_kv_imports_skipped\":2"));
    assert!(body.contains("\"runtime_exported_kv_blocks\":3"));
    assert!(body.contains("\"runtime_kv_segments_included\":4"));
    assert!(body.contains("\"runtime_kv_segments_skipped\":5"));
    assert!(body.contains("\"runtime_kv_segments_rejected\":6"));
    assert!(body.contains("\"recursive_runtime_calls\":2"));
    assert!(body.contains("\"live_memory_feedback_reinforced\":1"));
    assert!(body.contains("\"live_memory_feedback_penalized\":1"));
    assert!(body.contains("\"live_memory_feedback_missing\":1"));
    assert!(body.contains("\"reflection_issues\":2"));
    assert!(body.contains("\"critical_reflection_issues\":1"));
    assert!(body.contains("\"revision_actions\":2"));
    assert!(body.contains("\"runtime_error_message_chars\":9"));
    assert!(!body.contains("do-not-leak-memory-key"));
    assert!(!body.contains("do-not-leak-runtime-kv-key"));
    assert!(!body.contains("do-not-leak-model-id"));
    assert!(!body.contains("do-not-leak-adapter"));
    assert!(!body.contains("do-not-leak-lesson"));
    assert!(body.contains("\"memory_retention_policy\":{\"stale_after\":16,\"decay_rate\":0.125000,\"remove_below_strength\":0.062500,\"remove_after_failures\":3}"));
    assert!(body.contains("\"memory_compaction_policy\":{\"similarity_threshold\":0.875000,\"max_candidates\":24,\"max_merges\":5}"));
    assert!(body.contains("\"evolution_router_threshold_mutations\":8"));
    assert!(body.contains("\"evolution_hierarchy_weight_mutations\":6"));
    assert!(body.contains("\"evolution_router_threshold_delta\":0.420000"));
    assert!(body.contains("\"evolution_hierarchy_weight_delta\":0.210000"));
    assert!(body.contains("\"evolution_memory_updates\":7"));
    assert!(body.contains("\"evolution_replay_rust_check_diagnostic_chars\":13"));
    assert!(body.contains("\"evolution_replay_rust_check_live_memory_feedback_items\":2"));
    assert!(body.contains("\"evolution_external_feedbacks\":3"));
    assert!(body.contains("\"evolution_external_feedback_reinforcements\":2"));
    assert!(body.contains("\"evolution_external_feedback_penalties\":1"));
    assert!(body.contains("\"evolution_external_feedback_memory_updates\":5"));
    assert!(body.contains("\"evolution_external_feedback_removed\":1"));
    assert!(body.contains("\"evolution_external_feedback_missing\":2"));
    assert!(body.contains("\"evolution_external_feedback_strength_delta\":0.500000"));
    assert!(body.contains("\"evolution_replay_live_memory_feedback_items\":4"));
    assert!(body.contains("\"evolution_replay_live_memory_feedback_updates\":5"));
    assert!(body.contains("\"evolution_replay_live_memory_feedback_reinforcements\":3"));
    assert!(body.contains("\"evolution_replay_live_memory_feedback_penalties\":2"));
    assert!(body.contains("\"evolution_replay_live_memory_feedback_detail_items\":2"));
    assert!(body.contains("\"evolution_replay_live_memory_feedback_applied\":4"));
    assert!(body.contains("\"evolution_replay_live_memory_feedback_removed\":1"));
    assert!(body.contains("\"evolution_replay_live_memory_feedback_missing\":1"));
    assert!(body.contains("\"evolution_replay_live_memory_feedback_strength_delta\":0.875000"));
    assert!(body.contains("\"evolution_replay_live_evolution_items\":6"));
    assert!(body.contains("\"evolution_replay_live_evolution_router_threshold_mutations\":2"));
    assert!(body.contains("\"evolution_replay_live_evolution_hierarchy_weight_mutations\":3"));
    assert!(body.contains("\"evolution_replay_live_evolution_router_threshold_delta\":0.375000"));
    assert!(body.contains("\"evolution_replay_live_evolution_hierarchy_weight_delta\":0.625000"));
    assert!(body.contains("\"evolution_replay_live_evolution_online_reward_feedbacks\":7"));
    assert!(body.contains("\"evolution_replay_live_evolution_online_reward_reinforcements\":5"));
    assert!(body.contains("\"evolution_replay_live_evolution_online_reward_penalties\":2"));
    assert!(body.contains("\"evolution_replay_live_evolution_online_reward_strength\":1.125000"));
    assert!(body.contains(
        "\"evolution_replay_live_evolution_online_reward_reinforcement_strength\":0.750000"
    ));
    assert!(body
        .contains("\"evolution_replay_live_evolution_online_reward_penalty_strength\":0.250000"));
    assert!(body.contains("\"evolution_replay_live_evolution_memory_updates\":9"));
    assert!(body.contains("\"evolution_replay_live_evolution_stored_memory_updates\":4"));
    assert!(body.contains("\"evolution_replay_live_evolution_reflection_issues\":3"));
    assert!(body.contains("\"evolution_replay_live_evolution_critical_reflection_issues\":1"));
    assert!(body.contains("\"evolution_replay_live_evolution_revision_actions\":2"));
    assert!(body.contains("\"evolution_recursive_replay_items\":2"));
    assert!(body.contains("\"evolution_recursive_runtime_calls\":8"));
    assert!(body.contains("\"evolution_drift_rollbacks\":1"));
    assert!(body.contains("\"evolution_rollback_router_threshold_delta\":0.050000"));
    assert!(body.contains("\"evolution_rollback_hierarchy_weight_delta\":0.070000"));
    assert!(body.contains("\"runtime_error_experiences\":0"));
    assert!(body.contains("\"runtime_errors\":0"));
    assert!(body.contains("\"runtime_timeout_experiences\":0"));
    assert!(body.contains("\"runtime_timeouts\":0"));
    assert!(body.contains("\"runtime_error_message_chars\":0"));
    assert!(body.contains("\"rust_check_experiences\":0"));
    assert!(body.contains("\"rust_check_passed\":0"));
    assert!(body.contains("\"business_contract_experiences\":0"));
    assert!(body.contains("\"business_contract_passed\":0"));
    assert!(body.contains("\"business_contract_failed\":0"));
    assert!(body.contains("\"business_contract_missing_signals\":0"));
    assert!(body.contains("\"business_contract_raw_passed\":0"));
    assert!(body.contains("\"business_contract_raw_failed\":0"));
    assert!(body.contains("\"business_contract_response_normalized\":0"));
    assert!(body.contains("\"business_contract_canonical_fallbacks\":0"));
    assert!(body.contains("\"pool_dispatch_experiences\":1"));
    assert!(body.contains("\"pool_dispatch_items\":2"));
    assert!(body.contains("\"pool_dispatch_forwarded\":1"));
    assert!(body.contains("\"pool_dispatch_clamped\":1"));
    assert!(body.contains("\"pool_dispatch_low_priority\":1"));
    assert!(body.contains("\"self_evolving_memory_counters\":{"));
    assert!(body.contains("\"store_events\":4"));
    assert!(body.contains("\"retrieval_events\":1"));
    assert!(body.contains("\"maintenance_events\":1"));
    assert!(body.contains("\"admission_preview_events\":1"));
    assert!(body.contains("\"consolidation_events\":1"));
    assert!(body.contains("\"consolidation_actions\":3"));
    assert!(body.contains("\"merge_previews\":1"));
    assert!(body.contains("\"decay_previews\":1"));
    assert!(body.contains("\"tombstone_previews\":1"));
    assert!(body.contains("\"contexts\":3"));
    assert!(body.contains("\"store_saved_tokens\":96"));
    assert!(body.contains("\"maintenance_actions\":2"));
    assert!(body.contains("\"admission_candidates\":4"));
    assert!(body.contains("\"store_write_allowed\":0"));
    assert!(body.contains("\"store_applied\":0"));
    assert!(body.contains("\"store_applied_to_disk\":0"));
    assert!(body.contains("\"writeback_events\":1"));
    assert!(body.contains("\"writeback_source_case_digests\":1"));
    assert!(body.contains("\"writeback_attempted_records\":3"));
    assert!(body.contains("\"writeback_accepted_records\":3"));
    assert!(body.contains("\"writeback_records_before\":3"));
    assert!(body.contains("\"writeback_records_after\":6"));
    assert!(body.contains("\"writeback_tool_reliability_after\":1"));
    assert!(body.contains("\"writeback_tool_observations_after\":2"));
    assert!(body.contains("\"writeback_maintenance_actions\":1"));
    assert!(body.contains("\"writeback_merged_duplicate_episodes\":1"));
    assert!(body.contains("\"writeback_write_allowed\":1"));
    assert!(body.contains("\"writeback_durable_write_allowed\":1"));
    assert!(body.contains("\"writeback_applied\":1"));
    assert!(body.contains("\"writeback_applied_to_disk\":1"));
    assert!(body.contains("\"writeback_snapshot_changes\":1"));
    assert!(body.contains("\"reflection_issue_experiences\":2"));
    assert!(body.contains("\"critical_reflection_issue_experiences\":1"));
    assert!(body.contains("\"revision_action_experiences\":3"));
    assert!(body.contains("\"live_memory_feedback_experiences\":2"));
    assert!(body.contains("\"live_memory_feedback_updates\":4"));
    assert!(body.contains("\"live_memory_feedback_detail_experiences\":1"));
    assert!(body.contains("\"live_memory_feedback_applied\":3"));
    assert!(body.contains("\"live_memory_feedback_removed\":1"));
    assert!(body.contains("\"live_memory_feedback_missing\":2"));
    assert!(body.contains("\"live_memory_feedback_strength_delta\":1.250000"));
    assert!(body.contains("\"evolution_live_inference_runs\":2"));
    assert!(body.contains("\"evolution_live_router_threshold_mutations\":1"));
    assert!(body.contains("\"evolution_live_hierarchy_weight_mutations\":1"));
    assert!(body.contains("\"evolution_live_router_threshold_delta\":0.125000"));
    assert!(body.contains("\"evolution_live_hierarchy_weight_delta\":0.250000"));
    assert!(body.contains("\"evolution_live_online_reward_feedbacks\":5"));
    assert!(body.contains("\"evolution_live_online_reward_reinforcements\":4"));
    assert!(body.contains("\"evolution_live_online_reward_penalties\":1"));
    assert!(body.contains("\"evolution_live_online_reward_strength\":0.750000"));
    assert!(body.contains("\"evolution_live_memory_updates\":5"));
    assert!(body.contains("\"evolution_live_stored_memory_updates\":4"));
    assert!(body.contains("\"evolution_live_reflection_issues\":6"));
    assert!(body.contains("\"evolution_live_critical_reflection_issues\":1"));
    assert!(body.contains("\"evolution_live_revision_actions\":7"));
    assert!(body.contains("\"evolution_replay_business_contract_items\":0"));
    assert!(body.contains("\"evolution_replay_business_contract_passed\":0"));
    assert!(body.contains("\"evolution_replay_business_contract_raw_failed\":0"));
    assert!(body.contains("\"evolution_replay_business_contract_canonical_fallbacks\":0"));
    assert!(body.contains("\"state_gate\":{\"passed\":false"));
    assert!(body.contains("\"trace_gate\":{\"passed\":true"));
    assert!(body.contains("\"rust_check_events\":0"));
    assert!(body.contains("\"business_contract_events\":0"));
    assert!(body.contains("\"business_contract_event_raw_passed\":0"));
    assert!(body.contains("\"business_contract_event_response_normalized\":0"));
    assert!(body.contains("\"runtime_error_events\":0"));
    assert!(body.contains("\"runtime_timeout_events\":0"));
    assert!(body.contains("memory_count below minimum"));
}

#[test]
fn model_service_state_json_exposes_experience_hygiene_samples() {
    let mut engine = NoironEngine::new();
    engine.experience.record(rust_norion::ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant: ok"
            .to_owned(),
        profile: TaskProfile::Coding,
        lesson: "ssh -o ConnectTimeout=8 gitlab.local merge_requests bash command".to_owned(),
        quality: 0.94,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.5,
        stream_windows: 1,
        route_budget: rust_norion::RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: rust_norion::HierarchyWeights::new(0.33, 0.34, 0.33),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: rust_norion::RuntimeDiagnostics::default(),
        runtime_token_metrics: rust_norion::ExperienceRuntimeTokenMetrics::default(),
        process_reward: rust_norion::ProcessRewardReport::default(),
        live_evolution: rust_norion::LiveInferenceEvolution::default(),
    });
    let inspection = StateInspectionReport::from_engine(&engine, 3);

    let args = Args::parse(Vec::new());
    let body = model_service_state_response_json(9, &args, &inspection, None, None);

    assert!(body.contains("\"experience_hygiene_findings\":1"));
    assert!(body.contains("\"experience_hygiene_quarantine_candidates\":1"));
    assert!(body.contains("\"experience_hygiene_clean\":false"));
    assert!(body.contains("\"experience_hygiene_samples\":[{"));
    assert!(body.contains("\"reason\":\"cross_task_shell_transcript\""));
    assert!(body.contains("\"markers\":[\"ssh_connect_timeout\""));
    assert!(body.contains("\"gitlab_local\""));
    assert!(!body.contains("prompt_preview"));
    assert!(!body.contains("lesson_preview"));
    assert!(!body.contains("帮我用rust"));
    assert!(!body.contains("gitlab.local"));
}

#[test]
fn model_service_state_json_exposes_experience_index_samples() {
    let mut engine = NoironEngine::new();
    engine.experience.record(rust_norion::ExperienceInput {
        prompt: "Conversation transcript:\nuser: long rust task\nassistant: ".repeat(140),
        profile: TaskProfile::Coding,
        lesson: "lesson ".repeat(260),
        quality: 0.82,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.5,
        stream_windows: 1,
        route_budget: rust_norion::RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: rust_norion::HierarchyWeights::new(0.33, 0.34, 0.33),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: rust_norion::RuntimeDiagnostics::default(),
        runtime_token_metrics: rust_norion::ExperienceRuntimeTokenMetrics::default(),
        process_reward: rust_norion::ProcessRewardReport::default(),
        live_evolution: rust_norion::LiveInferenceEvolution::default(),
    });
    let inspection = StateInspectionReport::from_engine(&engine, 3);

    let args = Args::parse(Vec::new());
    let body = model_service_state_response_json(10, &args, &inspection, None, None);

    assert!(body.contains("\"experience_index_compacted_records\":1"));
    assert!(body.contains("\"experience_index_noisy_records\":1"));
    assert!(body.contains("\"experience_index_samples\":[{"));
    assert!(body.contains(
        "\"reason\":\"unstructured_long_transcript+transcript_prompt_without_clean_lesson\""
    ));
    assert!(body.contains("\"noise_penalty\":"));
    assert!(body.contains("\"prompt_chars\":"));
    assert!(body.contains("\"lesson_chars\":"));
    assert!(!body.contains("prompt_preview"));
    assert!(!body.contains("lesson_preview"));
    assert!(!body.contains("long rust task"));
    assert!(!body.contains("lesson lesson"));
}
