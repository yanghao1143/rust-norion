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
    assert!(line.contains("\"adaptive_routing\":"));
    assert!(line.contains("\"include\":"));
    assert!(line.contains("\"compress\":"));
    assert!(line.contains("\"defer\":"));
    assert!(line.contains("\"skip\":"));
    assert!(line.contains("\"input_tokens\":"));
    assert!(line.contains("\"retained_tokens\":"));
    assert!(line.contains("\"saved_tokens\":"));
    assert!(line.contains("\"selected_routes\":"));
    assert!(line.contains("\"score_summaries\":"));
    assert!(line.contains("\"task_hierarchy\":"));
    assert!(line.contains("\"mode\":\"rust_coding\""));
    assert!(line.contains("\"hierarchy_depth\":"));
    assert!(line.contains("\"route_fanout\":"));
    assert!(line.contains("\"route_pressure\":"));
    assert!(line.contains("\"compute_reduction\":"));
    assert!(line.contains("\"selected_lanes\":"));
    assert!(line.contains("\"memory_lanes\":"));
    assert!(line.contains("\"mutation_records\":"));
    assert!(line.contains("\"mutation_summaries\":"));
    assert!(line.contains("\"rollback_anchor_id\":\"task_hierarchy:"));
    assert!(line.contains("\"runtime_applied\":true"));
    assert!(line.contains("\"runtime_tokens\":"));
    assert!(line.contains("\"embedding\":{"));
    assert!(line.contains("\"query_source\":\"fallback\""));
    assert!(line.contains("\"query_dimensions\":64"));
    assert!(line.contains("\"fallback_embedding_calls\":"));
    assert!(line.contains("\"average_entropy\":"));
    assert!(line.contains("\"average_neg_logprob\":"));
    assert!(line.contains("\"uncertainty_perplexity\":"));
    assert!(line.contains("\"runtime_diagnostics\":"));
    assert!(line.contains("\"adapter_cache_mode\":"));
    assert!(line.contains("\"adapter_stream_trace_id\":"));
    assert!(line.contains("\"adapter_stream_gate_summary_digest\":"));
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
    assert!(line.contains("\"weak_runtime_kv_imports_skipped\":"));
    assert!(line.contains("\"budget_limited_runtime_kv_imports_skipped\":"));
    assert!(line.contains("\"runtime_kv_segments_included\":"));
    assert!(line.contains("\"runtime_kv_segments_skipped\":"));
    assert!(line.contains("\"runtime_kv_segments_rejected\":"));
    assert!(line.contains("\"runtime_kv_segment_count\":"));
    assert!(line.contains("\"runtime_kv_segment_yield\":"));
    assert!(line.contains("\"has_runtime_kv_activity_signal\":"));
    assert!(line.contains("\"has_runtime_kv_segment_signal\":"));
    assert!(line.contains("\"has_runtime_architecture_signal\":"));
    assert!(line.contains("\"has_forward_signal\":"));
    assert!(line.contains("\"hierarchy\":"));
    assert!(line.contains("\"device_profile\":"));
    assert!(line.contains("\"primary_lane\":"));
    assert!(line.contains("\"runtime_device_contract\":"));
    assert!(line.contains("\"runtime_budget\":"));
    assert!(line.contains("\"selected_device\":"));
    assert!(line.contains("\"quantization_profile\":"));
    assert!(line.contains("\"fallback_reason\":"));
    assert!(line.contains("\"fail_closed_cpu_stub\":"));
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
    assert!(line.contains("\"aggregation\":"));
    assert!(line.contains("\"budget_scope\":"));
    assert!(line.contains("\"main_thread_writer\":"));
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
    assert!(line.contains("\"repair_payloads\":"));
    assert!(line.contains("\"regeneration_payloads\":"));
    assert!(line.contains("\"mutation_intents\":"));
    assert!(line.contains("\"proposal_ids\":"));
    assert!(line.contains("\"youth_pressure\":"));
    assert!(line.contains("\"lifecycle_records\":"));
    assert!(line.contains("\"lifecycle_actions\":"));
    assert!(line.contains("\"lifecycle_summaries\":"));
    assert!(line.contains("\"lifecycle_tombstone_candidates\":"));
    assert!(line.contains("\"lifecycle_pending_validations\":"));
    assert!(line.contains("\"lifecycle_source_evidence\":"));
    assert!(line.contains("\"splice_segments\":"));
    assert!(line.contains("\"splice_exons\":"));
    assert!(line.contains("\"splice_introns\":"));
    assert!(line.contains("\"splice_variants\":"));
    assert!(line.contains("\"splice_retained\":"));
    assert!(line.contains("\"splice_skipped\":"));
    assert!(line.contains("\"splice_quarantined\":"));
    assert!(line.contains("\"splice_repair_candidates\":"));
    assert!(line.contains("\"splice_dispositions\":"));
    assert!(line.contains("\"splice_reason_summaries\":"));
    assert!(line.contains("\"splice_lifecycle_records\":"));
    assert!(line.contains("\"splice_lifecycle_states\":"));
    assert!(line.contains("\"splice_lifecycle_summaries\":"));
    assert!(line.contains("\"splice_findings\":"));
    assert!(line.contains("\"splice_finding_kinds\":"));
    assert!(line.contains("\"splice_mutation_intents\":"));
    assert!(line.contains("\"splice_proposals\":"));
    assert!(line.contains("\"splice_proposal_ids\":"));
    assert!(line.contains("\"splice_read_only\":"));
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
    assert!(line.contains("\"memory_admission\":"));
    assert!(line.contains("\"blocked\":"));
    assert!(line.contains("\"admitted\":"));
    assert!(line.contains("\"candidate_summaries\":"));
    assert!(line.contains("\"review_packets\":"));
    assert!(line.contains("\"review_packet_summaries\":"));
    assert!(line.contains("\"ledger_records\":"));
    assert!(line.contains("\"ledger_authorized\":"));
    assert!(line.contains("\"ledger_applied\":"));
    assert!(line.contains("\"ledger_preview_only\":"));
    assert!(line.contains("\"ledger_held\":"));
    assert!(line.contains("\"ledger_rejected\":"));
    assert!(line.contains("\"ledger_duplicate\":"));
    assert!(line.contains("\"ledger_decayed\":"));
    assert!(line.contains("\"ledger_merged\":"));
    assert!(line.contains("\"ledger_rollback\":"));
    assert!(line.contains("\"ledger_summaries\":"));
    assert!(line.contains("\"kv_fusion\":"));
    assert!(line.contains("\"approval_blocked\":"));
    assert!(line.contains("\"average_score\":"));
    assert!(line.contains("\"score_summaries\":"));
    assert!(line.contains("\"read_only\":true"));
    assert!(line.contains("\"write_allowed\":false"));
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
fn trace_schema_gate_rejects_malignant_genome_without_regeneration_payload() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace genome repair payload gate", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace genome repair payload gate",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "reasoning_genome",
        "\"malignant_genes\":0",
        "\"malignant_genes\":1",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("regeneration_payloads")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_reasoning_genome_splice_write_enabled() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace genome splice write gate", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace genome splice write gate",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "reasoning_genome",
        "\"splice_write_allowed\":false",
        "\"splice_write_allowed\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("reasoning_genome splice_write_allowed")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_reasoning_genome_splice_disposition_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace genome splice disposition gate", TaskProfile::Coding),
        &mut backend,
    );
    let line = increment_trace_object_usize(
        &trace_json_line(
            "trace genome splice disposition gate",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "reasoning_genome",
        "splice_retained",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("disposition counts")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_raw_payload_markers_in_splice_reasons() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace genome splice sanitized reasons", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace genome splice sanitized reasons",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "reasoning_genome",
        "\"splice_reason_summaries\":[",
        "\"splice_reason_summaries\":[\"label=private prompt\",",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("splice_reason_summaries")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_missing_splice_lifecycle_for_findings() {
    let line = replace_trace_object_usize(
        &replace_trace_object_usize(
            &rollback_trace_line(),
            "reasoning_genome",
            "splice_findings",
            1,
        ),
        "reasoning_genome",
        "splice_lifecycle_records",
        0,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("lifecycle records")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_write_enabled_splice_lifecycle_summary() {
    let line = replace_in_trace_object(
        &rollback_trace_line(),
        "reasoning_genome",
        "write_allowed=false applied=false",
        "write_allowed=true applied=false",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("splice_lifecycle_summaries")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_agent_team_writer_drift() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace agent team coordination", TaskProfile::Coding),
        &mut backend,
    );
    assert!(outcome.agent_team_plan.enabled);
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace agent team coordination",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "agent_team",
        "\"main_thread_writer\":\"main_thread\"",
        "\"main_thread_writer\":\"reviewer\"",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("main_thread_writer")),
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
fn trace_schema_gate_rejects_adaptive_routing_count_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace adaptive routing mismatch", TaskProfile::Coding),
        &mut backend,
    );
    let line = increment_trace_object_usize(
        &trace_json_line(
            "trace adaptive routing mismatch",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "adaptive_routing",
        "include",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("adaptive_routing decisions")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_adaptive_routing_write_enabled() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace adaptive routing write gate", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace adaptive routing write gate",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "adaptive_routing",
        "\"write_allowed\":false",
        "\"write_allowed\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("adaptive_routing write_allowed")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_task_hierarchy_mutation_count_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace task hierarchy mismatch", TaskProfile::Coding),
        &mut backend,
    );
    let line = increment_trace_object_usize(
        &trace_json_line(
            "trace task hierarchy mismatch",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "task_hierarchy",
        "mutation_records",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("task_hierarchy mutation_summaries")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_task_hierarchy_state_write_enabled() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace task hierarchy write gate", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace task hierarchy write gate",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "task_hierarchy",
        "\"state_write_allowed\":false",
        "\"state_write_allowed\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("task_hierarchy state_write_allowed")),
        "{failures:?}"
    );
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
