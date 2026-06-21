use super::*;

#[test]
fn trace_line_contains_core_control_decisions() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace Rust Noiron routing", TaskProfile::Coding),
        &mut backend,
    );

    let line = trace_json_line(
        "trace Rust Noiron routing",
        TaskProfile::Coding,
        12,
        &outcome,
    );

    assert!(line.contains("\"schema\":\"rust-norion-trace-v1\""));
    assert!(line.contains("\"case\":null"));
    assert!(line.contains("\"reflection\":"));
    assert!(line.contains("\"issue_codes\":"));
    assert!(line.contains("\"revision_passes\":"));
    assert!(line.contains("\"route\":"));
    assert!(line.contains("\"runtime_tokens\":"));
    assert!(line.contains("\"embedding\":{"));
    assert!(line.contains("\"query_source\":\"fallback\""));
    assert!(line.contains("\"query_dimensions\":64"));
    assert!(line.contains("\"fallback_embedding_calls\":"));
    assert!(line.contains("\"average_entropy\":"));
    assert!(line.contains("\"average_neg_logprob\":"));
    assert!(line.contains("\"uncertainty_perplexity\":"));
    assert!(line.contains("\"runtime_diagnostics\":"));
    assert!(line.contains("\"hot_kv_precision_bits\":"));
    assert!(line.contains("\"cold_kv_precision_bits\":"));
    assert!(line.contains("\"has_kv_precision_signal\":"));
    assert!(line.contains("\"runtime_adapter_observations\":"));
    assert!(line.contains("\"observation_count\":"));
    assert!(line.contains("\"best_adapter\":"));
    assert!(line.contains("\"selection_mismatch\":"));
    assert!(line.contains("\"best_score\":"));
    assert!(line.contains("\"forward_energy\":"));
    assert!(line.contains("\"kv_influence\":"));
    assert!(line.contains("\"has_runtime_architecture_signal\":"));
    assert!(line.contains("\"has_forward_signal\":"));
    assert!(line.contains("\"hierarchy\":"));
    assert!(line.contains("\"device_profile\":"));
    assert!(line.contains("\"primary_lane\":"));
    assert!(line.contains("\"runtime_device_contract\":"));
    assert!(line.contains("\"adapter_hints\":"));
    assert!(line.contains("\"local_kv_token_budget\":"));
    assert!(line.contains("\"global_kv_token_budget\":"));
    assert!(line.contains("\"execution_waves\":"));
    assert!(line.contains("\"runtime_calls\":"));
    assert!(line.contains("\"max_parallel_chunks\":"));
    assert!(line.contains("\"infini_memory\":"));
    assert!(line.contains("\"local_window\":"));
    assert!(line.contains("\"global_memory\":"));
    assert!(line.contains("\"sparse_skipped\":"));
    assert!(line.contains("\"skipped_tokens\":"));
    assert!(line.contains("\"template\":\"coding_local\""));
    assert!(line.contains("\"toolsmith\":"));
    assert!(line.contains("\"blueprint_summaries\":"));
    assert!(line.contains("\"gate_passed\":"));
    assert!(line.contains("\"agent_team\":"));
    assert!(line.contains("\"collision_free\":"));
    assert!(line.contains("\"reasoning_genome\":"));
    assert!(line.contains("\"genome_id\":\"genome:coding:v1\""));
    assert!(line.contains("\"stable_anchor_id\":\"genome:coding:stable\""));
    assert!(line.contains("\"gene_count\":"));
    assert!(line.contains("\"active_genes\":"));
    assert!(line.contains("\"aged_genes\":"));
    assert!(line.contains("\"malignant_genes\":"));
    assert!(line.contains("\"relabel_candidates\":"));
    assert!(line.contains("\"regeneration_candidates\":"));
    assert!(line.contains("\"gene_scissors_proposals\":"));
    assert!(line.contains("\"mutation_intents\":"));
    assert!(line.contains("\"proposal_ids\":"));
    assert!(line.contains("\"youth_pressure\":"));
    assert!(line.contains("\"drift\":"));
    assert!(line.contains("\"process_reward\":"));
    assert!(line.contains("\"auto_replay\":"));
    assert!(line.contains("\"router_updates\":"));
    assert!(line.contains("\"hierarchy_updates\":"));
    assert!(line.contains("\"router_threshold_mutations\":"));
    assert!(line.contains("\"hierarchy_weight_mutations\":"));
    assert!(line.contains("\"router_threshold_delta\":"));
    assert!(line.contains("\"hierarchy_weight_delta\":"));
    assert!(line.contains("\"memory_reinforcements\":"));
    assert!(line.contains("\"memory_penalties\":"));
    assert!(line.contains("\"live_memory_feedback_items\":"));
    assert!(line.contains("\"live_memory_feedback_updates\":"));
    assert!(line.contains("\"live_memory_feedback_reinforcements\":"));
    assert!(line.contains("\"live_memory_feedback_penalties\":"));
    assert!(line.contains("\"live_memory_feedback_detail_items\":"));
    assert!(line.contains("\"live_memory_feedback_applied\":"));
    assert!(line.contains("\"live_memory_feedback_removed\":"));
    assert!(line.contains("\"live_memory_feedback_missing\":"));
    assert!(line.contains("\"live_memory_feedback_strength_delta\":"));
    assert!(line.contains("\"business_contract_items\":"));
    assert!(line.contains("\"business_contract_passed\":"));
    assert!(line.contains("\"business_contract_failed\":"));
    assert!(line.contains("\"business_contract_raw_passed\":"));
    assert!(line.contains("\"business_contract_raw_failed\":"));
    assert!(line.contains("\"business_contract_response_normalized\":"));
    assert!(line.contains("\"business_contract_sanitized\":"));
    assert!(line.contains("\"business_contract_canonical_fallbacks\":"));
    assert!(line.contains("\"live_evolution_items\":"));
    assert!(line.contains("\"live_evolution_online_reward_feedbacks\":"));
    assert!(line.contains("\"live_evolution_online_reward_reinforcements\":"));
    assert!(line.contains("\"live_evolution_online_reward_penalties\":"));
    assert!(line.contains("\"live_evolution_online_reward_strength\":"));
    assert!(line.contains("\"live_evolution_online_reward_reinforcement_strength\":"));
    assert!(line.contains("\"live_evolution_online_reward_penalty_strength\":"));
    assert!(line.contains("\"live_evolution_memory_updates\":"));
    assert!(line.contains("\"live_evolution_stored_memory_updates\":"));
    assert!(line.contains("\"live_evolution_reflection_issues\":"));
    assert!(line.contains("\"live_evolution_critical_reflection_issues\":"));
    assert!(line.contains("\"live_evolution_revision_actions\":"));
    assert!(line.contains("\"recursive_runtime_items\":"));
    assert!(line.contains("\"recursive_runtime_calls\":"));
    assert!(line.contains("\"avg_recursive_call_pressure\":"));
    assert!(line.contains("\"max_recursive_call_pressure\":"));
    assert!(line.contains("\"live_evolution\":"));
    assert!(line.contains("\"live_inference_recorded\":true"));
    assert!(line.contains("\"live_router_threshold_delta\":"));
    assert!(line.contains("\"live_hierarchy_weight_delta\":"));
    assert!(line.contains("\"live_online_reward_feedbacks\":"));
    assert!(line.contains("\"live_online_reward_reinforcements\":"));
    assert!(line.contains("\"live_online_reward_penalties\":"));
    assert!(line.contains("\"live_online_reward_strength\":"));
    assert!(line.contains("\"live_online_reward_reinforcement_strength\":"));
    assert!(line.contains("\"live_online_reward_penalty_strength\":"));
    assert!(line.contains("\"live_memory_updates\":"));
    assert!(line.contains("\"live_stored_memory_updates\":"));
    assert!(line.contains("\"live_reflection_issues\":"));
    assert!(line.contains("\"live_critical_reflection_issues\":"));
    assert!(line.contains("\"live_revision_actions\":"));
    assert!(line.contains("\"evolution_ledger\":"));
    assert!(line.contains("\"live_inference_runs\":"));
    assert!(line.contains("\"cumulative_live_online_reward_feedbacks\":"));
    assert!(line.contains("\"cumulative_live_online_reward_reinforcements\":"));
    assert!(line.contains("\"cumulative_live_online_reward_penalties\":"));
    assert!(line.contains("\"cumulative_live_online_reward_strength\":"));
    assert!(line.contains("\"cumulative_live_online_reward_reinforcement_strength\":"));
    assert!(line.contains("\"cumulative_live_online_reward_penalty_strength\":"));
    assert!(line.contains("\"cumulative_live_memory_updates\":"));
    assert!(line.contains("\"cumulative_live_stored_memory_updates\":"));
    assert!(line.contains("\"cumulative_router_threshold_mutations\":"));
    assert!(line.contains("\"cumulative_hierarchy_weight_mutations\":"));
    assert!(line.contains("\"cumulative_memory_updates\":"));
    assert!(line.contains("\"cumulative_replay_live_memory_feedback_updates\":"));
    assert!(line.contains("\"cumulative_replay_live_memory_feedback_detail_items\":"));
    assert!(line.contains("\"cumulative_replay_live_memory_feedback_applied\":"));
    assert!(line.contains("\"cumulative_replay_live_memory_feedback_removed\":"));
    assert!(line.contains("\"cumulative_replay_live_memory_feedback_missing\":"));
    assert!(line.contains("\"cumulative_replay_live_memory_feedback_strength_delta\":"));
    assert!(line.contains("\"cumulative_replay_business_contract_items\":"));
    assert!(line.contains("\"cumulative_replay_business_contract_passed\":"));
    assert!(line.contains("\"cumulative_replay_business_contract_failed\":"));
    assert!(line.contains("\"cumulative_replay_business_contract_raw_passed\":"));
    assert!(line.contains("\"cumulative_replay_business_contract_raw_failed\":"));
    assert!(line.contains("\"cumulative_replay_business_contract_response_normalized\":"));
    assert!(line.contains("\"cumulative_replay_business_contract_sanitized\":"));
    assert!(line.contains("\"cumulative_replay_business_contract_canonical_fallbacks\":"));
    assert!(line.contains("\"cumulative_replay_live_evolution_items\":"));
    assert!(line.contains("\"cumulative_replay_live_evolution_online_reward_feedbacks\":"));
    assert!(line.contains("\"cumulative_replay_live_evolution_online_reward_reinforcements\":"));
    assert!(line.contains("\"cumulative_replay_live_evolution_online_reward_penalties\":"));
    assert!(line.contains("\"cumulative_replay_live_evolution_online_reward_strength\":"));
    assert!(
        line.contains("\"cumulative_replay_live_evolution_online_reward_reinforcement_strength\":")
    );
    assert!(line.contains("\"cumulative_replay_live_evolution_online_reward_penalty_strength\":"));
    assert!(line.contains("\"cumulative_replay_live_evolution_memory_updates\":"));
    assert!(line.contains("\"cumulative_replay_live_evolution_stored_memory_updates\":"));
    assert!(line.contains("\"cumulative_replay_live_evolution_reflection_issues\":"));
    assert!(line.contains("\"cumulative_replay_live_evolution_critical_reflection_issues\":"));
    assert!(line.contains("\"cumulative_replay_live_evolution_revision_actions\":"));
    assert!(line.contains("\"cumulative_recursive_runtime_calls\":"));
    assert!(line.contains("\"cumulative_drift_rollbacks\":"));
    assert!(line.contains("\"cumulative_rollback_router_threshold_delta\":"));
    assert!(line.contains("\"cumulative_rollback_hierarchy_weight_delta\":"));
    assert!(line.contains("\"runtime_kv_exported\":"));
    assert!(line.contains("\"runtime_kv_hold\":"));
    assert!(line.contains("\"runtime_kv_held\":"));
    assert!(line.contains("\"feedback_reinforced\":"));
    assert!(line.contains("\"feedback_penalized\":"));
    assert!(line.contains("\"feedback_reinforcement_amount\":"));
    assert!(line.contains("\"feedback_penalty_amount\":"));
    assert!(line.contains("\"feedback_updates\":"));
    assert!(line.contains("\"feedback_applied\":"));
    assert!(line.contains("\"feedback_removed\":"));
    assert!(line.contains("\"feedback_missing\":"));
    assert!(line.contains("\"feedback_strength_delta\":"));
    assert!(line.contains("\"feedback_update_summaries\":"));
    assert!(line.contains("\"stale_after\":"));
    assert!(line.contains("\"decay_rate\":"));
    assert!(line.contains("\"similarity_threshold\":"));
    assert!(line.contains("\"max_merges\":"));
    assert!(line.contains("\"memory_compaction\":"));
    assert!(line.ends_with('}'));
}

#[test]
fn trace_schema_gate_rejects_reasoning_genome_write_enabled() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace genome write gate", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line("trace genome write gate", TaskProfile::Coding, 5, &outcome),
        "reasoning_genome",
        "\"write_allowed\":false",
        "\"write_allowed\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("reasoning_genome write_allowed")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_reasoning_genome_applied_preview() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace genome applied gate", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace genome applied gate",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "reasoning_genome",
        "\"mutation_applied\":false",
        "\"mutation_applied\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("reasoning_genome mutation_applied")),
        "{failures:?}"
    );
}

#[test]
fn json_escape_handles_quotes_and_newlines() {
    assert_eq!(json_escape("a\"b\nc"), "a\\\"b\\nc");
}

#[test]
fn trace_line_can_include_benchmark_case_name() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace benchmark case", TaskProfile::General),
        &mut backend,
    );

    let line = trace_json_line_with_case(
        Some("general_case"),
        "trace benchmark case",
        TaskProfile::General,
        3,
        &outcome,
    );

    assert!(line.contains("\"case\":\"general_case\""));
}

#[test]
fn trace_schema_gate_accepts_generated_trace_line() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace schema gate", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line("trace schema gate", TaskProfile::Coding, 5, &outcome);

    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_schema_jsonl_gate_counts_runtime_error_notes() {
    struct RuntimeErrorBackend;

    impl InferenceBackend for RuntimeErrorBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                "Runtime backend error: runtime command mistralrs timed out after 1000 ms",
                vec![ReasoningStep::new(
                    "runtime_error",
                    "runtime command mistralrs timed out after 1000 ms",
                    0.0,
                )],
            )
        }
    }

    let path = temp_path("runtime-error-trace-schema");
    let mut engine = NoironEngine::new();
    let mut backend = RuntimeErrorBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace runtime timeout", TaskProfile::Coding),
        &mut backend,
    );
    fs::write(
        &path,
        format!(
            "{}\n",
            trace_json_line("trace runtime timeout", TaskProfile::Coding, 5, &outcome)
        ),
    )
    .unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.rust_check_events, 0);
    assert_eq!(report.runtime_error_events, 1);
    assert_eq!(report.runtime_timeout_events, 1);
    assert!(report.summary_line().contains("runtime_error_events=1"));
    assert!(report.summary_line().contains("runtime_timeout_events=1"));
    cleanup(path);
}

#[test]
fn trace_schema_gate_rejects_invalid_embedding_source() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace embedding source", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line("trace embedding source", TaskProfile::Coding, 5, &outcome),
        "embedding",
        "\"query_source\":\"fallback\"",
        "\"query_source\":\"external\"",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("embedding query_source external")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_zero_embedding_dimensions() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace embedding dimensions", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace embedding dimensions",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "embedding",
        "\"query_dimensions\":64",
        "\"query_dimensions\":0",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("embedding query_dimensions")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_embedding_fallback_count_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace embedding fallback mismatch", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace embedding fallback mismatch",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "embedding",
        "\"fallback_used\":true",
        "\"fallback_used\":false",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("embedding fallback_used")),
        "{failures:?}"
    );
}
