use rust_norion::{StateInspectionGateReport, TraceSchemaGateReport};

use super::super::json::{service_json_string, service_json_string_array};

pub(super) fn option_state_gate_service_json(report: Option<&StateInspectionGateReport>) -> String {
    report
        .map(|report| {
            format!(
                "{{\"passed\":{},\"summary\":{},\"failures\":{}}}",
                report.passed,
                service_json_string(&report.summary_line()),
                service_json_string_array(&report.failures)
            )
        })
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn option_trace_gate_service_json(report: Option<&TraceSchemaGateReport>) -> String {
    report
        .map(|report| {
            let json = format!(
                "{{\"passed\":{},\"checked_lines\":{},\"rust_check_events\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_feedback_updates\":{},\"rust_check_feedback_applied\":{},\"business_contract_events\":{},\"business_contract_event_passed\":{},\"business_contract_event_failed\":{},\"business_contract_event_missing_signals\":{},\"business_contract_event_protocol_leaks\":{},\"business_contract_event_substitutions\":{},\"business_contract_event_evasive_denials\":{},\"business_contract_event_raw_passed\":{},\"business_contract_event_raw_failed\":{},\"business_contract_event_response_normalized\":{},\"business_contract_event_sanitized\":{},\"business_contract_event_canonical_fallbacks\":{},\"runtime_error_events\":{},\"runtime_timeout_events\":{},\"self_evolution_admission_events\":{},\"self_evolution_admission_admitted\":{},\"self_evolution_admission_blocked\":{},\"self_evolution_admission_review_packets\":{},\"self_evolution_admission_evidence_ids\":{},\"self_evolution_admission_missing_review_packet_refs\":{},\"self_evolution_experiment_events\":{},\"self_evolution_experiment_admit\":{},\"self_evolution_experiment_hold\":{},\"self_evolution_experiment_reject\":{},\"self_evolution_experiment_rollback\":{},\"self_evolution_experiment_repeated\":{},\"self_evolution_experiment_conflicts\":{},\"self_evolution_experiment_rollback_replayable\":{},\"self_evolution_experiment_active_candidates\":{},\"self_evolution_experiment_write_allowed\":{},\"self_evolution_experiment_applied\":{},\"self_evolution_rollback_replay_events\":{},\"self_evolution_rollback_replay_items\":{},\"self_evolution_rollback_replay_replayable\":{},\"self_evolution_rollback_replay_blocked\":{},\"self_evolution_rollback_replay_all_replayable\":{},\"self_evolution_rollback_replay_rollback_anchor_ids\":{},\"self_evolution_rollback_replay_evidence_ids\":{},\"self_evolution_rollback_replay_active_candidates\":{},\"self_evolution_rollback_replay_item_write_allowed\":{},\"self_evolution_rollback_replay_item_applied\":{},\"self_evolution_rollback_replay_write_allowed\":{},\"self_evolution_rollback_replay_applied\":{},\"self_evolution_rollback_replay_gate_events\":{},\"self_evolution_rollback_replay_gate_admitted\":{},\"self_evolution_rollback_replay_gate_held\":{},\"self_evolution_rollback_replay_gate_review_packets\":{},\"self_evolution_rollback_replay_gate_review_evidence_ids\":{},\"self_evolution_rollback_replay_gate_missing_review_packet_refs\":{},\"self_evolution_rollback_replay_gate_items\":{},\"self_evolution_rollback_replay_gate_replayable\":{},\"self_evolution_rollback_replay_gate_blocked\":{},\"self_evolution_rollback_replay_gate_all_replayable\":{},\"self_evolution_rollback_replay_gate_rollback_anchor_ids\":{},\"self_evolution_rollback_replay_gate_evidence_ids\":{},\"self_evolution_rollback_replay_gate_active_candidates\":{},\"self_evolution_rollback_replay_gate_item_write_allowed\":{},\"self_evolution_rollback_replay_gate_item_applied\":{},\"self_evolution_rollback_replay_gate_plan_write_allowed\":{},\"self_evolution_rollback_replay_gate_plan_applied\":{},\"self_evolution_rollback_replay_gate_write_allowed\":{},\"self_evolution_rollback_replay_gate_applied\":{},\"self_evolution_operator_approval_events\":{},\"self_evolution_operator_approval_approved\":{},\"self_evolution_operator_approval_held\":{},\"self_evolution_operator_approval_review_packets\":{},\"self_evolution_operator_approval_evidence_ids\":{},\"self_evolution_operator_approval_rollback_anchor_ids\":{},\"self_evolution_operator_approval_content_digests\":{},\"self_evolution_operator_approval_source_report_schemas\":{},\"self_evolution_operator_approval_missing_review_packet_refs\":{},\"self_evolution_operator_approval_write_allowed\":{},\"self_evolution_operator_approval_applied\":{},\"improvement_corpus_events\":{},\"improvement_corpus_episodes\":{},\"improvement_corpus_active_adaptation\":{},\"improvement_corpus_compiler_passed\":{},\"improvement_corpus_test_passed\":{},\"improvement_corpus_benchmark_passed\":{},\"improvement_corpus_privacy_rejected\":{},\"improvement_corpus_secret_leaks\":{},\"adaptive_routing_events\":{},\"adaptive_routing_candidates\":{},\"adaptive_routing_include\":{},\"adaptive_routing_compress\":{},\"adaptive_routing_defer\":{},\"adaptive_routing_skip\":{},\"adaptive_routing_input_tokens\":{},\"adaptive_routing_retained_tokens\":{},\"adaptive_routing_saved_tokens\":{},\"task_hierarchy_events\":{},\"task_hierarchy_mutation_records\":{},\"task_hierarchy_route_pressure_milli\":{},\"task_hierarchy_compute_reduction_milli\":{},\"compute_budget_events\":{},\"compute_budget_low\":{},\"compute_budget_normal\":{},\"compute_budget_expanded\":{},\"compute_budget_selected_candidates\":{},\"compute_budget_low_value_skipped\":{},\"compute_budget_kv_lookups_skipped\":{},\"compute_budget_validation_cost_tokens\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_write_allowed\":{},\"compute_budget_applied\":{},\"memory_admission_events\":{},\"memory_admission_candidates\":{},\"memory_admission_ready\":{},\"memory_admission_blocked\":{},\"memory_admission_admitted\":{},\"memory_admission_hold\":{},\"memory_admission_reject\":{},\"memory_admission_quarantine\":{},\"memory_admission_review_packets\":{},\"memory_admission_ledger_records\":{},\"memory_admission_ledger_authorized\":{},\"memory_admission_ledger_applied\":{},\"memory_admission_ledger_preview_only\":{},\"memory_admission_ledger_held\":{},\"memory_admission_ledger_rejected\":{},\"memory_admission_ledger_duplicate\":{},\"memory_admission_ledger_decayed\":{},\"memory_admission_ledger_merged\":{},\"memory_admission_ledger_rollback\":{},\"kv_fusion_events\":{},\"kv_fusion_candidates\":{},\"kv_fusion_fused\":{},\"kv_fusion_compressed\":{},\"kv_fusion_skipped\":{},\"kv_fusion_held\":{},\"kv_fusion_rejected\":{},\"kv_fusion_approval_blocked\":{},\"kv_fusion_input_tokens\":{},\"kv_fusion_retained_tokens\":{},\"kv_fusion_saved_tokens\":{},\"summary\":{},\"failures\":{}}}",
                report.passed,
                report.checked_lines,
                report.rust_check_events,
                report.rust_check_passed,
                report.rust_check_failed,
                report.rust_check_feedback_updates,
                report.rust_check_feedback_applied,
                report.business_contract_events,
                report.business_contract_event_passed,
                report.business_contract_event_failed,
                report.business_contract_event_missing_signals,
                report.business_contract_event_protocol_leaks,
                report.business_contract_event_substitutions,
                report.business_contract_event_evasive_denials,
                report.business_contract_event_raw_passed,
                report.business_contract_event_raw_failed,
                report.business_contract_event_response_normalized,
                report.business_contract_event_sanitized,
                report.business_contract_event_canonical_fallbacks,
                report.runtime_error_events,
                report.runtime_timeout_events,
                report.self_evolution_admission_events,
                report.self_evolution_admission_admitted,
                report.self_evolution_admission_blocked,
                report.self_evolution_admission_review_packets,
                report.self_evolution_admission_evidence_ids,
                report.self_evolution_admission_missing_review_packet_refs,
                report.self_evolution_experiment_events,
                report.self_evolution_experiment_admit,
                report.self_evolution_experiment_hold,
                report.self_evolution_experiment_reject,
                report.self_evolution_experiment_rollback,
                report.self_evolution_experiment_repeated,
                report.self_evolution_experiment_conflicts,
                report.self_evolution_experiment_rollback_replayable,
                report.self_evolution_experiment_active_candidates,
                report.self_evolution_experiment_write_allowed,
                report.self_evolution_experiment_applied,
                report.self_evolution_rollback_replay_events,
                report.self_evolution_rollback_replay_items,
                report.self_evolution_rollback_replay_replayable,
                report.self_evolution_rollback_replay_blocked,
                report.self_evolution_rollback_replay_all_replayable,
                report.self_evolution_rollback_replay_rollback_anchor_ids,
                report.self_evolution_rollback_replay_evidence_ids,
                report.self_evolution_rollback_replay_active_candidates,
                report.self_evolution_rollback_replay_item_write_allowed,
                report.self_evolution_rollback_replay_item_applied,
                report.self_evolution_rollback_replay_write_allowed,
                report.self_evolution_rollback_replay_applied,
                report.self_evolution_rollback_replay_gate_events,
                report.self_evolution_rollback_replay_gate_admitted,
                report.self_evolution_rollback_replay_gate_held,
                report.self_evolution_rollback_replay_gate_review_packets,
                report.self_evolution_rollback_replay_gate_review_evidence_ids,
                report.self_evolution_rollback_replay_gate_missing_review_packet_refs,
                report.self_evolution_rollback_replay_gate_items,
                report.self_evolution_rollback_replay_gate_replayable,
                report.self_evolution_rollback_replay_gate_blocked,
                report.self_evolution_rollback_replay_gate_all_replayable,
                report.self_evolution_rollback_replay_gate_rollback_anchor_ids,
                report.self_evolution_rollback_replay_gate_evidence_ids,
                report.self_evolution_rollback_replay_gate_active_candidates,
                report.self_evolution_rollback_replay_gate_item_write_allowed,
                report.self_evolution_rollback_replay_gate_item_applied,
                report.self_evolution_rollback_replay_gate_plan_write_allowed,
                report.self_evolution_rollback_replay_gate_plan_applied,
                report.self_evolution_rollback_replay_gate_write_allowed,
                report.self_evolution_rollback_replay_gate_applied,
                report.self_evolution_operator_approval_events,
                report.self_evolution_operator_approval_approved,
                report.self_evolution_operator_approval_held,
                report.self_evolution_operator_approval_review_packets,
                report.self_evolution_operator_approval_evidence_ids,
                report.self_evolution_operator_approval_rollback_anchor_ids,
                report.self_evolution_operator_approval_content_digests,
                report.self_evolution_operator_approval_source_report_schemas,
                report.self_evolution_operator_approval_missing_review_packet_refs,
                report.self_evolution_operator_approval_write_allowed,
                report.self_evolution_operator_approval_applied,
                report.improvement_corpus_events,
                report.improvement_corpus_episodes,
                report.improvement_corpus_active_adaptation,
                report.improvement_corpus_compiler_passed,
                report.improvement_corpus_test_passed,
                report.improvement_corpus_benchmark_passed,
                report.improvement_corpus_privacy_rejected,
                report.improvement_corpus_secret_leaks,
                report.adaptive_routing_events,
                report.adaptive_routing_candidates,
                report.adaptive_routing_include,
                report.adaptive_routing_compress,
                report.adaptive_routing_defer,
                report.adaptive_routing_skip,
                report.adaptive_routing_input_tokens,
                report.adaptive_routing_retained_tokens,
                report.adaptive_routing_saved_tokens,
                report.task_hierarchy_events,
                report.task_hierarchy_mutation_records,
                report.task_hierarchy_route_pressure_milli,
                report.task_hierarchy_compute_reduction_milli,
                report.compute_budget_events,
                report.compute_budget_low,
                report.compute_budget_normal,
                report.compute_budget_expanded,
                report.compute_budget_selected_candidates,
                report.compute_budget_low_value_skipped,
                report.compute_budget_kv_lookups_skipped,
                report.compute_budget_validation_cost_tokens,
                report.compute_budget_saved_tokens,
                report.compute_budget_avoided_tokens,
                report.compute_budget_write_allowed,
                report.compute_budget_applied,
                report.memory_admission_events,
                report.memory_admission_candidates,
                report.memory_admission_ready,
                report.memory_admission_blocked,
                report.memory_admission_admitted,
                report.memory_admission_hold,
                report.memory_admission_reject,
                report.memory_admission_quarantine,
                report.memory_admission_review_packets,
                report.memory_admission_ledger_records,
                report.memory_admission_ledger_authorized,
                report.memory_admission_ledger_applied,
                report.memory_admission_ledger_preview_only,
                report.memory_admission_ledger_held,
                report.memory_admission_ledger_rejected,
                report.memory_admission_ledger_duplicate,
                report.memory_admission_ledger_decayed,
                report.memory_admission_ledger_merged,
                report.memory_admission_ledger_rollback,
                report.kv_fusion_events,
                report.kv_fusion_candidates,
                report.kv_fusion_fused,
                report.kv_fusion_compressed,
                report.kv_fusion_skipped,
                report.kv_fusion_held,
                report.kv_fusion_rejected,
                report.kv_fusion_approval_blocked,
                report.kv_fusion_input_tokens,
                report.kv_fusion_retained_tokens,
                report.kv_fusion_saved_tokens,
                service_json_string(&report.summary_line()),
                service_json_string_array(&report.failures)
            );
            let apply_fields = format!(
                "\"self_evolution_rollback_replay_apply_events\":{},\"self_evolution_rollback_replay_apply_ready\":{},\"self_evolution_rollback_replay_apply_held\":{},\"self_evolution_rollback_replay_apply_items\":{},\"self_evolution_rollback_replay_apply_replayable\":{},\"self_evolution_rollback_replay_apply_blocked\":{},\"self_evolution_rollback_replay_apply_review_packets\":{},\"self_evolution_rollback_replay_apply_evidence_ids\":{},\"self_evolution_rollback_replay_apply_rollback_anchor_ids\":{},\"self_evolution_rollback_replay_apply_content_digests\":{},\"self_evolution_rollback_replay_apply_source_report_schemas\":{},\"self_evolution_rollback_replay_apply_missing_refs\":{},\"self_evolution_rollback_replay_apply_blocked_reasons\":{},\"self_evolution_rollback_replay_apply_write_allowed\":{},\"self_evolution_rollback_replay_apply_applied\":{}",
                report.self_evolution_rollback_replay_apply_events,
                report.self_evolution_rollback_replay_apply_ready,
                report.self_evolution_rollback_replay_apply_held,
                report.self_evolution_rollback_replay_apply_items,
                report.self_evolution_rollback_replay_apply_replayable,
                report.self_evolution_rollback_replay_apply_blocked,
                report.self_evolution_rollback_replay_apply_review_packets,
                report.self_evolution_rollback_replay_apply_evidence_ids,
                report.self_evolution_rollback_replay_apply_rollback_anchor_ids,
                report.self_evolution_rollback_replay_apply_content_digests,
                report.self_evolution_rollback_replay_apply_source_report_schemas,
                report.self_evolution_rollback_replay_apply_missing_refs,
                report.self_evolution_rollback_replay_apply_blocked_reasons,
                report.self_evolution_rollback_replay_apply_write_allowed,
                report.self_evolution_rollback_replay_apply_applied,
            );
            let json = json.replacen(
                "\"improvement_corpus_events\"",
                &format!("{apply_fields},\"improvement_corpus_events\""),
                1,
            );
            let promotion_preflight_fields = format!(
                "\"self_evolution_promotion_preflight_events\":{},\"self_evolution_promotion_preflight_ready\":{},\"self_evolution_promotion_preflight_held\":{},\"self_evolution_promotion_preflight_review_packets\":{},\"self_evolution_promotion_preflight_evidence_ids\":{},\"self_evolution_promotion_preflight_rollback_anchor_ids\":{},\"self_evolution_promotion_preflight_content_digests\":{},\"self_evolution_promotion_preflight_source_report_schemas\":{},\"self_evolution_promotion_preflight_missing_refs\":{},\"self_evolution_promotion_preflight_blocked_reasons\":{},\"self_evolution_promotion_preflight_write_allowed\":{},\"self_evolution_promotion_preflight_applied\":{}",
                report.self_evolution_promotion_preflight_events,
                report.self_evolution_promotion_preflight_ready,
                report.self_evolution_promotion_preflight_held,
                report.self_evolution_promotion_preflight_review_packets,
                report.self_evolution_promotion_preflight_evidence_ids,
                report.self_evolution_promotion_preflight_rollback_anchor_ids,
                report.self_evolution_promotion_preflight_content_digests,
                report.self_evolution_promotion_preflight_source_report_schemas,
                report.self_evolution_promotion_preflight_missing_refs,
                report.self_evolution_promotion_preflight_blocked_reasons,
                report.self_evolution_promotion_preflight_write_allowed,
                report.self_evolution_promotion_preflight_applied,
            );
            let json = json.replacen(
                "\"improvement_corpus_events\"",
                &format!("{promotion_preflight_fields},\"improvement_corpus_events\""),
                1,
            );
            let operator_approval_counters = format!(
                "\"self_evolution_operator_approval_counters\":{}",
                report
                    .self_evolution_operator_approval_service_counters()
                    .json_object()
            );
            let runtime_closed_loop_counters = format!(
                "\"runtime_closed_loop_counters\":{{\"adaptive_routing_events\":{},\"adaptive_routing_candidates\":{},\"adaptive_routing_saved_tokens\":{},\"task_hierarchy_events\":{},\"task_hierarchy_mutation_records\":{},\"task_hierarchy_compute_reduction_milli\":{},\"compute_budget_events\":{},\"compute_budget_selected_candidates\":{},\"compute_budget_kv_lookups_skipped\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_write_allowed\":{},\"compute_budget_applied\":{},\"memory_admission_events\":{},\"memory_admission_candidates\":{},\"memory_admission_ledger_records\":{},\"memory_admission_ledger_preview_only\":{},\"memory_admission_ledger_authorized\":{},\"memory_admission_ledger_applied\":{},\"self_evolving_memory_store_events\":{},\"self_evolving_memory_store_retrieval_events\":{},\"self_evolving_memory_store_maintenance_events\":{},\"self_evolving_memory_store_admission_preview_events\":{},\"self_evolving_memory_store_contexts\":{},\"self_evolving_memory_store_maintenance_actions\":{},\"self_evolving_memory_store_admission_candidates\":{},\"self_evolving_memory_store_write_allowed\":{},\"self_evolving_memory_store_durable_write_allowed\":{},\"self_evolving_memory_store_applied\":{},\"self_evolving_memory_store_applied_to_disk\":{},\"memory_residency_events\":{},\"memory_residency_decisions\":{},\"memory_residency_hot\":{},\"memory_residency_warm\":{},\"memory_residency_cold\":{},\"memory_residency_quarantined\":{},\"memory_residency_retired\":{},\"memory_residency_protected_rollback_anchors\":{},\"memory_residency_blocked_reasons\":{},\"memory_residency_token_estimate\":{},\"memory_residency_write_allowed\":{},\"memory_residency_durable_write_allowed\":{},\"memory_residency_applied\":{},\"auto_replay_live_memory_feedback_items\":{},\"auto_replay_live_memory_feedback_updates\":{},\"auto_replay_live_memory_feedback_reinforcements\":{},\"auto_replay_live_memory_feedback_penalties\":{},\"auto_replay_live_memory_feedback_detail_items\":{},\"auto_replay_live_memory_feedback_applied\":{},\"auto_replay_live_memory_feedback_removed\":{},\"auto_replay_live_memory_feedback_missing\":{},\"auto_replay_live_memory_feedback_strength_delta_milli\":{},\"auto_replay_business_contract_items\":{},\"auto_replay_business_contract_passed\":{},\"auto_replay_business_contract_failed\":{},\"auto_replay_business_contract_raw_passed\":{},\"auto_replay_business_contract_raw_failed\":{},\"auto_replay_business_contract_response_normalized\":{},\"auto_replay_business_contract_sanitized\":{},\"auto_replay_business_contract_canonical_fallbacks\":{},\"auto_replay_live_evolution_items\":{},\"auto_replay_live_evolution_router_threshold_mutations\":{},\"auto_replay_live_evolution_hierarchy_weight_mutations\":{},\"auto_replay_live_evolution_router_threshold_delta_milli\":{},\"auto_replay_live_evolution_hierarchy_weight_delta_milli\":{},\"auto_replay_live_evolution_online_reward_feedbacks\":{},\"auto_replay_live_evolution_online_reward_reinforcements\":{},\"auto_replay_live_evolution_online_reward_penalties\":{},\"auto_replay_live_evolution_online_reward_strength_milli\":{},\"auto_replay_live_evolution_online_reward_reinforcement_strength_milli\":{},\"auto_replay_live_evolution_online_reward_penalty_strength_milli\":{},\"auto_replay_live_evolution_memory_updates\":{},\"auto_replay_live_evolution_stored_memory_updates\":{},\"auto_replay_live_evolution_reflection_issues\":{},\"auto_replay_live_evolution_critical_reflection_issues\":{},\"auto_replay_live_evolution_revision_actions\":{},\"auto_replay_recursive_runtime_items\":{},\"auto_replay_recursive_runtime_calls\":{},\"auto_replay_avg_recursive_call_pressure_milli\":{},\"auto_replay_max_recursive_call_pressure_milli\":{},\"auto_replay_runtime_kv_budget_pressure_items\":{},\"auto_replay_avg_runtime_kv_budget_pressure_milli\":{},\"auto_replay_max_runtime_kv_budget_pressure_milli\":{},\"auto_replay_runtime_kv_weak_import_pressure_items\":{},\"auto_replay_avg_runtime_kv_weak_import_pressure_milli\":{},\"auto_replay_max_runtime_kv_weak_import_pressure_milli\":{},\"kv_fusion_events\":{},\"kv_fusion_candidates\":{},\"kv_fusion_saved_tokens\":{},\"self_evolution_experiment_events\":{},\"self_evolution_experiment_rollback\":{},\"self_evolution_rollback_replay_events\":{},\"self_evolution_rollback_replay_items\":{},\"self_evolution_rollback_replay_gate_held\":{},\"self_evolution_rollback_replay_apply_ready\":{},\"self_evolution_promotion_preflight_ready\":{},\"self_evolution_operator_approval_held\":{},\"reasoning_genome_events\":{},\"reasoning_genome_genes\":{},\"reasoning_genome_gene_scissors_proposals\":{},\"reasoning_genome_repair_payloads\":{},\"reasoning_genome_regeneration_payloads\":{},\"reasoning_genome_splice_quarantined\":{},\"reasoning_genome_mutation_applied\":{}}}",
                report.adaptive_routing_events,
                report.adaptive_routing_candidates,
                report.adaptive_routing_saved_tokens,
                report.task_hierarchy_events,
                report.task_hierarchy_mutation_records,
                report.task_hierarchy_compute_reduction_milli,
                report.compute_budget_events,
                report.compute_budget_selected_candidates,
                report.compute_budget_kv_lookups_skipped,
                report.compute_budget_saved_tokens,
                report.compute_budget_avoided_tokens,
                report.compute_budget_write_allowed,
                report.compute_budget_applied,
                report.memory_admission_events,
                report.memory_admission_candidates,
                report.memory_admission_ledger_records,
                report.memory_admission_ledger_preview_only,
                report.memory_admission_ledger_authorized,
                report.memory_admission_ledger_applied,
                report.self_evolving_memory_store_events,
                report.self_evolving_memory_store_retrieval_events,
                report.self_evolving_memory_store_maintenance_events,
                report.self_evolving_memory_store_admission_preview_events,
                report.self_evolving_memory_store_contexts,
                report.self_evolving_memory_store_maintenance_actions,
                report.self_evolving_memory_store_admission_candidates,
                report.self_evolving_memory_store_write_allowed,
                report.self_evolving_memory_store_durable_write_allowed,
                report.self_evolving_memory_store_applied,
                report.self_evolving_memory_store_applied_to_disk,
                report.memory_residency_events,
                report.memory_residency_decisions,
                report.memory_residency_hot,
                report.memory_residency_warm,
                report.memory_residency_cold,
                report.memory_residency_quarantined,
                report.memory_residency_retired,
                report.memory_residency_protected_rollback_anchors,
                report.memory_residency_blocked_reasons,
                report.memory_residency_token_estimate,
                report.memory_residency_write_allowed,
                report.memory_residency_durable_write_allowed,
                report.memory_residency_applied,
                report.auto_replay_live_memory_feedback_items,
                report.auto_replay_live_memory_feedback_updates,
                report.auto_replay_live_memory_feedback_reinforcements,
                report.auto_replay_live_memory_feedback_penalties,
                report.auto_replay_live_memory_feedback_detail_items,
                report.auto_replay_live_memory_feedback_applied,
                report.auto_replay_live_memory_feedback_removed,
                report.auto_replay_live_memory_feedback_missing,
                report.auto_replay_live_memory_feedback_strength_delta_milli,
                report.auto_replay_business_contract_items,
                report.auto_replay_business_contract_passed,
                report.auto_replay_business_contract_failed,
                report.auto_replay_business_contract_raw_passed,
                report.auto_replay_business_contract_raw_failed,
                report.auto_replay_business_contract_response_normalized,
                report.auto_replay_business_contract_sanitized,
                report.auto_replay_business_contract_canonical_fallbacks,
                report.auto_replay_live_evolution_items,
                report.auto_replay_live_evolution_router_threshold_mutations,
                report.auto_replay_live_evolution_hierarchy_weight_mutations,
                report.auto_replay_live_evolution_router_threshold_delta_milli,
                report.auto_replay_live_evolution_hierarchy_weight_delta_milli,
                report.auto_replay_live_evolution_online_reward_feedbacks,
                report.auto_replay_live_evolution_online_reward_reinforcements,
                report.auto_replay_live_evolution_online_reward_penalties,
                report.auto_replay_live_evolution_online_reward_strength_milli,
                report.auto_replay_live_evolution_online_reward_reinforcement_strength_milli,
                report.auto_replay_live_evolution_online_reward_penalty_strength_milli,
                report.auto_replay_live_evolution_memory_updates,
                report.auto_replay_live_evolution_stored_memory_updates,
                report.auto_replay_live_evolution_reflection_issues,
                report.auto_replay_live_evolution_critical_reflection_issues,
                report.auto_replay_live_evolution_revision_actions,
                report.auto_replay_recursive_runtime_items,
                report.auto_replay_recursive_runtime_calls,
                report.auto_replay_avg_recursive_call_pressure_milli,
                report.auto_replay_max_recursive_call_pressure_milli,
                report.auto_replay_runtime_kv_budget_pressure_items,
                report.auto_replay_avg_runtime_kv_budget_pressure_milli,
                report.auto_replay_max_runtime_kv_budget_pressure_milli,
                report.auto_replay_runtime_kv_weak_import_pressure_items,
                report.auto_replay_avg_runtime_kv_weak_import_pressure_milli,
                report.auto_replay_max_runtime_kv_weak_import_pressure_milli,
                report.kv_fusion_events,
                report.kv_fusion_candidates,
                report.kv_fusion_saved_tokens,
                report.self_evolution_experiment_events,
                report.self_evolution_experiment_rollback,
                report.self_evolution_rollback_replay_events,
                report.self_evolution_rollback_replay_items,
                report.self_evolution_rollback_replay_gate_held,
                report.self_evolution_rollback_replay_apply_ready,
                report.self_evolution_promotion_preflight_ready,
                report.self_evolution_operator_approval_held,
                report.reasoning_genome_events,
                report.reasoning_genome_genes,
                report.reasoning_genome_gene_scissors_proposals,
                report.reasoning_genome_repair_payloads,
                report.reasoning_genome_regeneration_payloads,
                report.reasoning_genome_splice_quarantined,
                report.reasoning_genome_mutation_applied,
            );
            let experiment_counters = format!(
                "\"self_evolution_experiment_counters\":{{\"events\":{},\"admit\":{},\"hold\":{},\"reject\":{},\"rollback\":{},\"repeated\":{},\"conflicts\":{},\"rollback_replayable\":{},\"active_candidates\":{},\"write_allowed\":{},\"applied\":{}}}",
                report.self_evolution_experiment_events,
                report.self_evolution_experiment_admit,
                report.self_evolution_experiment_hold,
                report.self_evolution_experiment_reject,
                report.self_evolution_experiment_rollback,
                report.self_evolution_experiment_repeated,
                report.self_evolution_experiment_conflicts,
                report.self_evolution_experiment_rollback_replayable,
                report.self_evolution_experiment_active_candidates,
                report.self_evolution_experiment_write_allowed,
                report.self_evolution_experiment_applied,
            );
            let rollback_replay_counters = format!(
                "\"self_evolution_rollback_replay_counters\":{{\"events\":{},\"items\":{},\"replayable\":{},\"blocked\":{},\"all_replayable\":{},\"rollback_anchor_ids\":{},\"evidence_ids\":{},\"active_candidates\":{},\"item_write_allowed\":{},\"item_applied\":{},\"write_allowed\":{},\"applied\":{},\"gate_events\":{},\"gate_admitted\":{},\"gate_held\":{},\"gate_review_packets\":{},\"gate_review_evidence_ids\":{},\"gate_missing_review_packet_refs\":{},\"gate_item_write_allowed\":{},\"gate_item_applied\":{},\"gate_plan_write_allowed\":{},\"gate_plan_applied\":{},\"gate_write_allowed\":{},\"gate_applied\":{}}}",
                report.self_evolution_rollback_replay_events,
                report.self_evolution_rollback_replay_items,
                report.self_evolution_rollback_replay_replayable,
                report.self_evolution_rollback_replay_blocked,
                report.self_evolution_rollback_replay_all_replayable,
                report.self_evolution_rollback_replay_rollback_anchor_ids,
                report.self_evolution_rollback_replay_evidence_ids,
                report.self_evolution_rollback_replay_active_candidates,
                report.self_evolution_rollback_replay_item_write_allowed,
                report.self_evolution_rollback_replay_item_applied,
                report.self_evolution_rollback_replay_write_allowed,
                report.self_evolution_rollback_replay_applied,
                report.self_evolution_rollback_replay_gate_events,
                report.self_evolution_rollback_replay_gate_admitted,
                report.self_evolution_rollback_replay_gate_held,
                report.self_evolution_rollback_replay_gate_review_packets,
                report.self_evolution_rollback_replay_gate_review_evidence_ids,
                report.self_evolution_rollback_replay_gate_missing_review_packet_refs,
                report.self_evolution_rollback_replay_gate_item_write_allowed,
                report.self_evolution_rollback_replay_gate_item_applied,
                report.self_evolution_rollback_replay_gate_plan_write_allowed,
                report.self_evolution_rollback_replay_gate_plan_applied,
                report.self_evolution_rollback_replay_gate_write_allowed,
                report.self_evolution_rollback_replay_gate_applied,
            );
            let rollback_replay_apply_counters = format!(
                "\"self_evolution_rollback_replay_apply_counters\":{{\"events\":{},\"ready\":{},\"held\":{},\"items\":{},\"replayable\":{},\"blocked\":{},\"review_packets\":{},\"evidence_ids\":{},\"rollback_anchor_ids\":{},\"content_digests\":{},\"source_report_schemas\":{},\"missing_refs\":{},\"blocked_reasons\":{},\"write_allowed\":{},\"applied\":{}}}",
                report.self_evolution_rollback_replay_apply_events,
                report.self_evolution_rollback_replay_apply_ready,
                report.self_evolution_rollback_replay_apply_held,
                report.self_evolution_rollback_replay_apply_items,
                report.self_evolution_rollback_replay_apply_replayable,
                report.self_evolution_rollback_replay_apply_blocked,
                report.self_evolution_rollback_replay_apply_review_packets,
                report.self_evolution_rollback_replay_apply_evidence_ids,
                report.self_evolution_rollback_replay_apply_rollback_anchor_ids,
                report.self_evolution_rollback_replay_apply_content_digests,
                report.self_evolution_rollback_replay_apply_source_report_schemas,
                report.self_evolution_rollback_replay_apply_missing_refs,
                report.self_evolution_rollback_replay_apply_blocked_reasons,
                report.self_evolution_rollback_replay_apply_write_allowed,
                report.self_evolution_rollback_replay_apply_applied,
            );
            let promotion_preflight_counters = format!(
                "\"self_evolution_promotion_preflight_counters\":{{\"events\":{},\"ready\":{},\"held\":{},\"review_packets\":{},\"evidence_ids\":{},\"rollback_anchor_ids\":{},\"content_digests\":{},\"source_report_schemas\":{},\"missing_refs\":{},\"blocked_reasons\":{},\"write_allowed\":{},\"applied\":{}}}",
                report.self_evolution_promotion_preflight_events,
                report.self_evolution_promotion_preflight_ready,
                report.self_evolution_promotion_preflight_held,
                report.self_evolution_promotion_preflight_review_packets,
                report.self_evolution_promotion_preflight_evidence_ids,
                report.self_evolution_promotion_preflight_rollback_anchor_ids,
                report.self_evolution_promotion_preflight_content_digests,
                report.self_evolution_promotion_preflight_source_report_schemas,
                report.self_evolution_promotion_preflight_missing_refs,
                report.self_evolution_promotion_preflight_blocked_reasons,
                report.self_evolution_promotion_preflight_write_allowed,
                report.self_evolution_promotion_preflight_applied,
            );
            json.replacen(
                "\"summary\"",
                &format!(
                    "{runtime_closed_loop_counters},{experiment_counters},{rollback_replay_counters},{operator_approval_counters},{promotion_preflight_counters},{rollback_replay_apply_counters},\"summary\""
                ),
                1,
            )
        })
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::SelfEvolutionOperatorApprovalServiceCounters;

    #[test]
    fn trace_gate_service_json_exposes_self_evolution_admission_counts() {
        let report = TraceSchemaGateReport {
            passed: true,
            checked_lines: 4,
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
            self_evolution_admission_events: 2,
            self_evolution_admission_admitted: 1,
            self_evolution_admission_blocked: 1,
            self_evolution_admission_review_packets: 2,
            self_evolution_admission_evidence_ids: 4,
            self_evolution_admission_missing_review_packet_refs: 0,
            self_evolution_experiment_events: 4,
            self_evolution_experiment_admit: 1,
            self_evolution_experiment_hold: 1,
            self_evolution_experiment_reject: 1,
            self_evolution_experiment_rollback: 1,
            self_evolution_experiment_repeated: 1,
            self_evolution_experiment_conflicts: 1,
            self_evolution_experiment_rollback_replayable: 1,
            self_evolution_experiment_active_candidates: 0,
            self_evolution_experiment_write_allowed: 0,
            self_evolution_experiment_applied: 0,
            self_evolution_rollback_replay_events: 1,
            self_evolution_rollback_replay_items: 2,
            self_evolution_rollback_replay_replayable: 1,
            self_evolution_rollback_replay_blocked: 1,
            self_evolution_rollback_replay_all_replayable: 0,
            self_evolution_rollback_replay_rollback_anchor_ids: 3,
            self_evolution_rollback_replay_evidence_ids: 4,
            self_evolution_rollback_replay_active_candidates: 0,
            self_evolution_rollback_replay_item_write_allowed: 0,
            self_evolution_rollback_replay_item_applied: 0,
            self_evolution_rollback_replay_write_allowed: 0,
            self_evolution_rollback_replay_applied: 0,
            self_evolution_rollback_replay_gate_events: 2,
            self_evolution_rollback_replay_gate_admitted: 1,
            self_evolution_rollback_replay_gate_held: 1,
            self_evolution_rollback_replay_gate_review_packets: 2,
            self_evolution_rollback_replay_gate_review_evidence_ids: 3,
            self_evolution_rollback_replay_gate_missing_review_packet_refs: 0,
            self_evolution_rollback_replay_gate_items: 3,
            self_evolution_rollback_replay_gate_replayable: 2,
            self_evolution_rollback_replay_gate_blocked: 1,
            self_evolution_rollback_replay_gate_all_replayable: 1,
            self_evolution_rollback_replay_gate_rollback_anchor_ids: 4,
            self_evolution_rollback_replay_gate_evidence_ids: 5,
            self_evolution_rollback_replay_gate_active_candidates: 0,
            self_evolution_rollback_replay_gate_item_write_allowed: 0,
            self_evolution_rollback_replay_gate_item_applied: 0,
            self_evolution_rollback_replay_gate_plan_write_allowed: 0,
            self_evolution_rollback_replay_gate_plan_applied: 0,
            self_evolution_rollback_replay_gate_write_allowed: 0,
            self_evolution_rollback_replay_gate_applied: 0,
            self_evolution_operator_approval_events: 2,
            self_evolution_operator_approval_approved: 1,
            self_evolution_operator_approval_held: 1,
            self_evolution_operator_approval_review_packets: 2,
            self_evolution_operator_approval_evidence_ids: 3,
            self_evolution_operator_approval_rollback_anchor_ids: 4,
            self_evolution_operator_approval_content_digests: 5,
            self_evolution_operator_approval_source_report_schemas: 2,
            self_evolution_operator_approval_missing_review_packet_refs: 0,
            self_evolution_operator_approval_write_allowed: 0,
            self_evolution_operator_approval_applied: 0,
            self_evolution_promotion_preflight_events: 2,
            self_evolution_promotion_preflight_ready: 1,
            self_evolution_promotion_preflight_held: 1,
            self_evolution_promotion_preflight_review_packets: 2,
            self_evolution_promotion_preflight_evidence_ids: 4,
            self_evolution_promotion_preflight_rollback_anchor_ids: 4,
            self_evolution_promotion_preflight_content_digests: 6,
            self_evolution_promotion_preflight_source_report_schemas: 4,
            self_evolution_promotion_preflight_missing_refs: 0,
            self_evolution_promotion_preflight_blocked_reasons: 1,
            self_evolution_promotion_preflight_write_allowed: 0,
            self_evolution_promotion_preflight_applied: 0,
            self_evolution_rollback_replay_apply_events: 2,
            self_evolution_rollback_replay_apply_ready: 1,
            self_evolution_rollback_replay_apply_held: 1,
            self_evolution_rollback_replay_apply_items: 2,
            self_evolution_rollback_replay_apply_replayable: 2,
            self_evolution_rollback_replay_apply_blocked: 0,
            self_evolution_rollback_replay_apply_review_packets: 2,
            self_evolution_rollback_replay_apply_evidence_ids: 4,
            self_evolution_rollback_replay_apply_rollback_anchor_ids: 4,
            self_evolution_rollback_replay_apply_content_digests: 6,
            self_evolution_rollback_replay_apply_source_report_schemas: 4,
            self_evolution_rollback_replay_apply_missing_refs: 0,
            self_evolution_rollback_replay_apply_blocked_reasons: 1,
            self_evolution_rollback_replay_apply_write_allowed: 0,
            self_evolution_rollback_replay_apply_applied: 0,
            improvement_corpus_events: 0,
            improvement_corpus_episodes: 0,
            improvement_corpus_active_adaptation: 0,
            improvement_corpus_compiler_passed: 0,
            improvement_corpus_test_passed: 0,
            improvement_corpus_benchmark_passed: 0,
            improvement_corpus_privacy_rejected: 0,
            improvement_corpus_secret_leaks: 0,
            adaptive_routing_events: 2,
            adaptive_routing_candidates: 5,
            adaptive_routing_include: 2,
            adaptive_routing_compress: 1,
            adaptive_routing_defer: 1,
            adaptive_routing_skip: 1,
            adaptive_routing_input_tokens: 512,
            adaptive_routing_retained_tokens: 320,
            adaptive_routing_saved_tokens: 192,
            task_hierarchy_events: 2,
            task_hierarchy_mutation_records: 4,
            task_hierarchy_route_pressure_milli: 730,
            task_hierarchy_compute_reduction_milli: 280,
            compute_budget_events: 2,
            compute_budget_low: 1,
            compute_budget_normal: 0,
            compute_budget_expanded: 1,
            compute_budget_selected_candidates: 3,
            compute_budget_low_value_skipped: 2,
            compute_budget_kv_lookups_skipped: 4,
            compute_budget_validation_cost_tokens: 32,
            compute_budget_saved_tokens: 144,
            compute_budget_avoided_tokens: 233,
            compute_budget_write_allowed: 0,
            compute_budget_applied: 0,
            memory_admission_events: 1,
            memory_admission_candidates: 3,
            memory_admission_ready: 1,
            memory_admission_blocked: 2,
            memory_admission_admitted: 0,
            memory_admission_hold: 1,
            memory_admission_reject: 1,
            memory_admission_quarantine: 0,
            memory_admission_review_packets: 3,
            memory_admission_ledger_records: 3,
            memory_admission_ledger_authorized: 0,
            memory_admission_ledger_applied: 0,
            memory_admission_ledger_preview_only: 1,
            memory_admission_ledger_held: 1,
            memory_admission_ledger_rejected: 1,
            memory_admission_ledger_duplicate: 0,
            memory_admission_ledger_decayed: 0,
            memory_admission_ledger_merged: 0,
            memory_admission_ledger_rollback: 0,
            self_evolving_memory_store_events: 3,
            self_evolving_memory_store_retrieval_events: 1,
            self_evolving_memory_store_maintenance_events: 1,
            self_evolving_memory_store_admission_preview_events: 1,
            self_evolving_memory_store_contexts: 4,
            self_evolving_memory_store_maintenance_actions: 2,
            self_evolving_memory_store_admission_candidates: 2,
            self_evolving_memory_store_write_allowed: 0,
            self_evolving_memory_store_durable_write_allowed: 0,
            self_evolving_memory_store_applied: 0,
            self_evolving_memory_store_applied_to_disk: 0,
            memory_residency_events: 1,
            memory_residency_decisions: 4,
            memory_residency_hot: 1,
            memory_residency_warm: 1,
            memory_residency_cold: 1,
            memory_residency_quarantined: 1,
            memory_residency_retired: 0,
            memory_residency_protected_rollback_anchors: 2,
            memory_residency_blocked_reasons: 1,
            memory_residency_token_estimate: 20,
            memory_residency_write_allowed: 0,
            memory_residency_durable_write_allowed: 0,
            memory_residency_applied: 0,
            auto_replay_live_memory_feedback_items: 1,
            auto_replay_live_memory_feedback_updates: 1,
            auto_replay_live_memory_feedback_reinforcements: 1,
            auto_replay_live_memory_feedback_penalties: 0,
            auto_replay_live_memory_feedback_detail_items: 1,
            auto_replay_live_memory_feedback_applied: 1,
            auto_replay_live_memory_feedback_removed: 0,
            auto_replay_live_memory_feedback_missing: 0,
            auto_replay_live_memory_feedback_strength_delta_milli: 250,
            auto_replay_recursive_runtime_items: 1,
            auto_replay_recursive_runtime_calls: 2,
            auto_replay_avg_recursive_call_pressure_milli: 500,
            auto_replay_max_recursive_call_pressure_milli: 750,
            auto_replay_runtime_kv_budget_pressure_items: 1,
            auto_replay_avg_runtime_kv_budget_pressure_milli: 400,
            auto_replay_max_runtime_kv_budget_pressure_milli: 800,
            auto_replay_runtime_kv_weak_import_pressure_items: 1,
            auto_replay_avg_runtime_kv_weak_import_pressure_milli: 300,
            auto_replay_max_runtime_kv_weak_import_pressure_milli: 600,
            kv_fusion_events: 1,
            kv_fusion_candidates: 3,
            kv_fusion_fused: 1,
            kv_fusion_compressed: 1,
            kv_fusion_skipped: 1,
            kv_fusion_held: 0,
            kv_fusion_rejected: 0,
            kv_fusion_approval_blocked: 0,
            kv_fusion_input_tokens: 240,
            kv_fusion_retained_tokens: 140,
            kv_fusion_saved_tokens: 100,
            reasoning_genome_events: 1,
            reasoning_genome_genes: 8,
            reasoning_genome_gene_scissors_proposals: 2,
            reasoning_genome_repair_payloads: 2,
            reasoning_genome_regeneration_payloads: 1,
            reasoning_genome_splice_quarantined: 1,
            reasoning_genome_mutation_applied: 0,
            failures: Vec::new(),
            ..TraceSchemaGateReport::default()
        };

        let json = option_trace_gate_service_json(Some(&report));

        assert!(json.contains("\"self_evolution_admission_events\":2"));
        assert!(json.contains("\"self_evolution_admission_admitted\":1"));
        assert!(json.contains("\"self_evolution_admission_blocked\":1"));
        assert!(json.contains("\"self_evolution_admission_review_packets\":2"));
        assert!(json.contains("\"self_evolution_admission_evidence_ids\":4"));
        assert!(json.contains("\"self_evolution_admission_missing_review_packet_refs\":0"));
        assert!(json.contains("\"self_evolution_experiment_events\":4"));
        assert!(json.contains("\"self_evolution_experiment_admit\":1"));
        assert!(json.contains("\"self_evolution_experiment_rollback\":1"));
        assert!(json.contains("\"self_evolution_experiment_repeated\":1"));
        assert!(json.contains("\"self_evolution_experiment_conflicts\":1"));
        assert!(json.contains("\"self_evolution_experiment_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_events\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_items\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_replayable\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_blocked\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_all_replayable\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_rollback_anchor_ids\":3"));
        assert!(json.contains("\"self_evolution_rollback_replay_evidence_ids\":4"));
        assert!(json.contains("\"self_evolution_rollback_replay_active_candidates\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_item_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_item_applied\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_applied\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_events\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_admitted\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_held\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_review_packets\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_review_evidence_ids\":3"));
        assert!(
            json.contains("\"self_evolution_rollback_replay_gate_missing_review_packet_refs\":0")
        );
        assert!(json.contains("\"self_evolution_rollback_replay_gate_items\":3"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_replayable\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_blocked\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_all_replayable\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_rollback_anchor_ids\":4"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_evidence_ids\":5"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_active_candidates\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_item_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_item_applied\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_plan_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_plan_applied\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_gate_applied\":0"));
        assert!(json.contains("\"self_evolution_operator_approval_events\":2"));
        assert!(json.contains("\"self_evolution_operator_approval_approved\":1"));
        assert!(json.contains("\"self_evolution_operator_approval_held\":1"));
        assert!(json.contains("\"self_evolution_operator_approval_review_packets\":2"));
        assert!(json.contains("\"self_evolution_operator_approval_evidence_ids\":3"));
        assert!(json.contains("\"self_evolution_operator_approval_rollback_anchor_ids\":4"));
        assert!(json.contains("\"self_evolution_operator_approval_content_digests\":5"));
        assert!(json.contains("\"self_evolution_operator_approval_source_report_schemas\":2"));
        assert!(json.contains("\"self_evolution_operator_approval_missing_review_packet_refs\":0"));
        assert!(json.contains("\"self_evolution_operator_approval_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_operator_approval_applied\":0"));
        assert!(json.contains("\"self_evolution_promotion_preflight_events\":2"));
        assert!(json.contains("\"self_evolution_promotion_preflight_ready\":1"));
        assert!(json.contains("\"self_evolution_promotion_preflight_held\":1"));
        assert!(json.contains("\"self_evolution_promotion_preflight_review_packets\":2"));
        assert!(json.contains("\"self_evolution_promotion_preflight_evidence_ids\":4"));
        assert!(json.contains("\"self_evolution_promotion_preflight_rollback_anchor_ids\":4"));
        assert!(json.contains("\"self_evolution_promotion_preflight_content_digests\":6"));
        assert!(json.contains("\"self_evolution_promotion_preflight_source_report_schemas\":4"));
        assert!(json.contains("\"self_evolution_promotion_preflight_missing_refs\":0"));
        assert!(json.contains("\"self_evolution_promotion_preflight_blocked_reasons\":1"));
        assert!(json.contains("\"self_evolution_promotion_preflight_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_promotion_preflight_applied\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_events\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_ready\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_held\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_items\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_replayable\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_blocked\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_review_packets\":2"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_evidence_ids\":4"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_rollback_anchor_ids\":4"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_content_digests\":6"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_source_report_schemas\":4"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_missing_refs\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_blocked_reasons\":1"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_applied\":0"));
        assert!(json.contains("\"improvement_corpus_events\":0"));
        assert!(json.contains(
            "\"runtime_closed_loop_counters\":{\"adaptive_routing_events\":2,\"adaptive_routing_candidates\":5,\"adaptive_routing_saved_tokens\":192"
        ));
        assert!(json.contains(
            "\"compute_budget_events\":2,\"compute_budget_selected_candidates\":3,\"compute_budget_kv_lookups_skipped\":4,\"compute_budget_saved_tokens\":144,\"compute_budget_avoided_tokens\":233,\"compute_budget_write_allowed\":0,\"compute_budget_applied\":0"
        ));
        assert!(json.contains(
            "\"memory_admission_events\":1,\"memory_admission_candidates\":3,\"memory_admission_ledger_records\":3,\"memory_admission_ledger_preview_only\":1,\"memory_admission_ledger_authorized\":0,\"memory_admission_ledger_applied\":0"
        ));
        assert!(json.contains(
            "\"self_evolving_memory_store_events\":3,\"self_evolving_memory_store_retrieval_events\":1,\"self_evolving_memory_store_maintenance_events\":1,\"self_evolving_memory_store_admission_preview_events\":1,\"self_evolving_memory_store_contexts\":4,\"self_evolving_memory_store_maintenance_actions\":2,\"self_evolving_memory_store_admission_candidates\":2"
        ));
        assert!(json.contains(
            "\"memory_residency_events\":1,\"memory_residency_decisions\":4,\"memory_residency_hot\":1,\"memory_residency_warm\":1,\"memory_residency_cold\":1,\"memory_residency_quarantined\":1,\"memory_residency_retired\":0,\"memory_residency_protected_rollback_anchors\":2,\"memory_residency_blocked_reasons\":1,\"memory_residency_token_estimate\":20"
        ));
        assert!(json.contains(
            "\"auto_replay_live_memory_feedback_items\":1,\"auto_replay_live_memory_feedback_updates\":1,\"auto_replay_live_memory_feedback_reinforcements\":1,\"auto_replay_live_memory_feedback_penalties\":0,\"auto_replay_live_memory_feedback_detail_items\":1,\"auto_replay_live_memory_feedback_applied\":1"
        ));
        assert!(json.contains(
            "\"auto_replay_live_memory_feedback_strength_delta_milli\":250,\"auto_replay_business_contract_items\":0,\"auto_replay_business_contract_passed\":0,\"auto_replay_business_contract_failed\":0"
        ));
        assert!(json.contains(
            "\"auto_replay_live_evolution_items\":0,\"auto_replay_live_evolution_router_threshold_mutations\":0,\"auto_replay_live_evolution_hierarchy_weight_mutations\":0"
        ));
        assert!(json.contains(
            "\"auto_replay_live_evolution_online_reward_feedbacks\":0,\"auto_replay_live_evolution_online_reward_reinforcements\":0,\"auto_replay_live_evolution_online_reward_penalties\":0"
        ));
        assert!(json.contains(
            "\"auto_replay_live_evolution_memory_updates\":0,\"auto_replay_live_evolution_stored_memory_updates\":0,\"auto_replay_live_evolution_reflection_issues\":0"
        ));
        assert!(json.contains(
            "\"auto_replay_recursive_runtime_items\":1,\"auto_replay_recursive_runtime_calls\":2,\"auto_replay_avg_recursive_call_pressure_milli\":500,\"auto_replay_max_recursive_call_pressure_milli\":750"
        ));
        assert!(json.contains(
            "\"auto_replay_runtime_kv_budget_pressure_items\":1,\"auto_replay_avg_runtime_kv_budget_pressure_milli\":400,\"auto_replay_max_runtime_kv_budget_pressure_milli\":800,\"auto_replay_runtime_kv_weak_import_pressure_items\":1,\"auto_replay_avg_runtime_kv_weak_import_pressure_milli\":300,\"auto_replay_max_runtime_kv_weak_import_pressure_milli\":600"
        ));
        assert!(json.contains(
            "\"kv_fusion_events\":1,\"kv_fusion_candidates\":3,\"kv_fusion_saved_tokens\":100"
        ));
        assert!(json.contains(
            "\"self_evolution_experiment_events\":4,\"self_evolution_experiment_rollback\":1,\"self_evolution_rollback_replay_events\":1,\"self_evolution_rollback_replay_items\":2"
        ));
        assert!(json.contains(
            "\"self_evolution_rollback_replay_gate_held\":1,\"self_evolution_rollback_replay_apply_ready\":1,\"self_evolution_promotion_preflight_ready\":1,\"self_evolution_operator_approval_held\":1"
        ));
        assert!(json.contains(
            "\"reasoning_genome_events\":1,\"reasoning_genome_genes\":8,\"reasoning_genome_gene_scissors_proposals\":2,\"reasoning_genome_repair_payloads\":2,\"reasoning_genome_regeneration_payloads\":1,\"reasoning_genome_splice_quarantined\":1,\"reasoning_genome_mutation_applied\":0}"
        ));
        assert!(json.contains("\"adaptive_routing_events\":2"));
        assert!(json.contains("\"adaptive_routing_candidates\":5"));
        assert!(json.contains("\"adaptive_routing_saved_tokens\":192"));
        assert!(json.contains("\"task_hierarchy_events\":2"));
        assert!(json.contains("\"task_hierarchy_mutation_records\":4"));
        assert!(json.contains("\"task_hierarchy_compute_reduction_milli\":280"));
        assert!(json.contains("\"compute_budget_events\":2"));
        assert!(json.contains("\"compute_budget_low\":1"));
        assert!(json.contains("\"compute_budget_expanded\":1"));
        assert!(json.contains("\"compute_budget_selected_candidates\":3"));
        assert!(json.contains("\"compute_budget_low_value_skipped\":2"));
        assert!(json.contains("\"compute_budget_kv_lookups_skipped\":4"));
        assert!(json.contains("\"compute_budget_validation_cost_tokens\":32"));
        assert!(json.contains("\"compute_budget_saved_tokens\":144"));
        assert!(json.contains("\"compute_budget_avoided_tokens\":233"));
        assert!(json.contains("\"compute_budget_write_allowed\":0"));
        assert!(json.contains("\"compute_budget_applied\":0"));
        assert!(json.contains("\"memory_admission_events\":1"));
        assert!(json.contains("\"memory_admission_candidates\":3"));
        assert!(json.contains("\"memory_admission_ledger_records\":3"));
        assert!(json.contains("\"memory_admission_ledger_preview_only\":1"));
        assert!(json.contains("\"kv_fusion_events\":1"));
        assert!(json.contains("\"kv_fusion_candidates\":3"));
        assert!(json.contains("\"kv_fusion_saved_tokens\":100"));
        assert!(json.contains("self_evolution_admission_events=2"));
        assert!(json.contains("self_evolution_admission_review_packets=2"));
        assert!(json.contains("self_evolution_experiment_events=4"));
        assert!(json.contains("self_evolution_experiment_rollback=1"));
        assert!(json.contains("self_evolution_rollback_replay_events=1"));
        assert!(json.contains("self_evolution_rollback_replay_blocked=1"));
        assert!(json.contains("self_evolution_rollback_replay_gate_events=2"));
        assert!(json.contains("self_evolution_rollback_replay_gate_held=1"));
        assert!(json.contains("self_evolution_rollback_replay_gate_review_packets=2"));
        assert!(json.contains("self_evolution_operator_approval_events=2"));
        assert!(json.contains("self_evolution_operator_approval_held=1"));
        assert!(json.contains("self_evolution_operator_approval_review_packets=2"));
        assert!(json.contains("self_evolution_promotion_preflight_events=2"));
        assert!(json.contains("self_evolution_promotion_preflight_ready=1"));
        assert!(json.contains("self_evolution_rollback_replay_apply_events=2"));
        assert!(json.contains("self_evolution_rollback_replay_apply_ready=1"));
        assert!(json.contains("adaptive_routing_candidates=5"));
        assert!(json.contains("task_hierarchy_mutation_records=4"));
        assert!(json.contains("compute_budget_events=2"));
        assert!(json.contains("compute_budget_saved_tokens=144"));
        assert!(json.contains("memory_admission_ledger_records=3"));
        assert!(json.contains("kv_fusion_saved_tokens=100"));
    }

    #[test]
    fn trace_gate_service_json_exposes_operator_approval_counter_object() {
        let report = TraceSchemaGateReport {
            passed: true,
            checked_lines: 2,
            self_evolution_operator_approval_events: 2,
            self_evolution_operator_approval_approved: 1,
            self_evolution_operator_approval_held: 1,
            self_evolution_operator_approval_review_packets: 3,
            self_evolution_operator_approval_evidence_ids: 4,
            self_evolution_operator_approval_rollback_anchor_ids: 5,
            self_evolution_operator_approval_content_digests: 6,
            self_evolution_operator_approval_source_report_schemas: 2,
            self_evolution_operator_approval_missing_review_packet_refs: 1,
            self_evolution_operator_approval_write_allowed: 0,
            self_evolution_operator_approval_applied: 0,
            failures: Vec::new(),
            ..TraceSchemaGateReport::default()
        };

        let json = option_trace_gate_service_json(Some(&report));
        let counters: SelfEvolutionOperatorApprovalServiceCounters =
            report.self_evolution_operator_approval_service_counters();

        assert!(counters.review_required);
        assert!(counters.blocked);
        assert!(!counters.approval_ready);
        assert!(json.contains("\"self_evolution_operator_approval_counters\":{"));
        assert!(json.contains("\"trace_gate_passed\":true"));
        assert!(json.contains("\"data_present\":true"));
        assert!(json.contains("\"approval_ready\":false"));
        assert!(json.contains("\"review_required\":true"));
        assert!(json.contains("\"blocked\":true"));
        assert!(json.contains("\"events\":2"));
        assert!(json.contains("\"approved\":1"));
        assert!(json.contains("\"held\":1"));
        assert!(json.contains("\"review_packets\":3"));
        assert!(json.contains("\"evidence_ids\":4"));
        assert!(json.contains("\"rollback_anchor_ids\":5"));
        assert!(json.contains("\"content_digests\":6"));
        assert!(json.contains("\"source_report_schemas\":2"));
        assert!(json.contains("\"missing_review_packet_refs\":1"));
        assert!(json.contains("\"write_allowed\":0"));
        assert!(json.contains("\"applied\":0"));
        assert!(json.contains("\"activation_allowed\":false"));
        assert!(json.contains("\"memory_write_allowed\":false"));
        assert!(json.contains("\"genome_write_allowed\":false"));
        assert!(json.contains("\"kv_write_allowed\":false"));
        assert!(json.contains("\"validation_failures\":["));
        assert!(json.contains("self_evolution_operator_approval_missing_review_packet_refs"));
        assert!(json.contains("\"summary\":"));
    }

    #[test]
    fn trace_gate_service_json_exposes_self_evolution_gate_counter_objects() {
        let report = TraceSchemaGateReport {
            passed: true,
            checked_lines: 5,
            self_evolution_experiment_events: 4,
            self_evolution_experiment_admit: 1,
            self_evolution_experiment_hold: 1,
            self_evolution_experiment_reject: 1,
            self_evolution_experiment_rollback: 1,
            self_evolution_experiment_repeated: 1,
            self_evolution_experiment_conflicts: 1,
            self_evolution_experiment_rollback_replayable: 1,
            self_evolution_experiment_active_candidates: 0,
            self_evolution_experiment_write_allowed: 0,
            self_evolution_experiment_applied: 0,
            self_evolution_rollback_replay_events: 1,
            self_evolution_rollback_replay_items: 2,
            self_evolution_rollback_replay_replayable: 2,
            self_evolution_rollback_replay_blocked: 0,
            self_evolution_rollback_replay_all_replayable: 1,
            self_evolution_rollback_replay_rollback_anchor_ids: 2,
            self_evolution_rollback_replay_evidence_ids: 3,
            self_evolution_rollback_replay_active_candidates: 0,
            self_evolution_rollback_replay_item_write_allowed: 0,
            self_evolution_rollback_replay_item_applied: 0,
            self_evolution_rollback_replay_write_allowed: 0,
            self_evolution_rollback_replay_applied: 0,
            self_evolution_rollback_replay_gate_events: 1,
            self_evolution_rollback_replay_gate_admitted: 1,
            self_evolution_rollback_replay_gate_held: 0,
            self_evolution_rollback_replay_gate_review_packets: 1,
            self_evolution_rollback_replay_gate_review_evidence_ids: 4,
            self_evolution_rollback_replay_gate_missing_review_packet_refs: 0,
            self_evolution_rollback_replay_gate_item_write_allowed: 0,
            self_evolution_rollback_replay_gate_item_applied: 0,
            self_evolution_rollback_replay_gate_plan_write_allowed: 0,
            self_evolution_rollback_replay_gate_plan_applied: 0,
            self_evolution_rollback_replay_gate_write_allowed: 0,
            self_evolution_rollback_replay_gate_applied: 0,
            self_evolution_rollback_replay_apply_events: 2,
            self_evolution_rollback_replay_apply_ready: 1,
            self_evolution_rollback_replay_apply_held: 1,
            self_evolution_rollback_replay_apply_items: 2,
            self_evolution_rollback_replay_apply_replayable: 2,
            self_evolution_rollback_replay_apply_blocked: 0,
            self_evolution_rollback_replay_apply_review_packets: 1,
            self_evolution_rollback_replay_apply_evidence_ids: 4,
            self_evolution_rollback_replay_apply_rollback_anchor_ids: 2,
            self_evolution_rollback_replay_apply_content_digests: 3,
            self_evolution_rollback_replay_apply_source_report_schemas: 2,
            self_evolution_rollback_replay_apply_missing_refs: 0,
            self_evolution_rollback_replay_apply_blocked_reasons: 1,
            self_evolution_rollback_replay_apply_write_allowed: 0,
            self_evolution_rollback_replay_apply_applied: 0,
            self_evolution_promotion_preflight_events: 2,
            self_evolution_promotion_preflight_ready: 1,
            self_evolution_promotion_preflight_held: 1,
            self_evolution_promotion_preflight_review_packets: 2,
            self_evolution_promotion_preflight_evidence_ids: 4,
            self_evolution_promotion_preflight_rollback_anchor_ids: 2,
            self_evolution_promotion_preflight_content_digests: 3,
            self_evolution_promotion_preflight_source_report_schemas: 2,
            self_evolution_promotion_preflight_missing_refs: 0,
            self_evolution_promotion_preflight_blocked_reasons: 1,
            self_evolution_promotion_preflight_write_allowed: 0,
            self_evolution_promotion_preflight_applied: 0,
            failures: Vec::new(),
            ..TraceSchemaGateReport::default()
        };

        let json = option_trace_gate_service_json(Some(&report));

        assert!(json.contains(
            "\"self_evolution_experiment_counters\":{\"events\":4,\"admit\":1,\"hold\":1,\"reject\":1,\"rollback\":1"
        ));
        assert!(json.contains("\"repeated\":1"));
        assert!(json.contains("\"conflicts\":1"));
        assert!(json.contains("\"rollback_replayable\":1"));
        assert!(json.contains("\"active_candidates\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_counters\":{"));
        assert!(json.contains("\"items\":2"));
        assert!(json.contains("\"all_replayable\":1"));
        assert!(json.contains("\"gate_admitted\":1"));
        assert!(json.contains("\"gate_held\":0"));
        assert!(json.contains("\"gate_plan_write_allowed\":0"));
        assert!(json.contains("\"self_evolution_rollback_replay_apply_counters\":{"));
        assert!(json.contains("\"ready\":1"));
        assert!(json.contains("\"held\":1"));
        assert!(json.contains("\"missing_refs\":0"));
        assert!(json.contains("\"blocked_reasons\":1"));
        assert!(json.contains("\"write_allowed\":0"));
        assert!(json.contains("\"applied\":0"));
        assert!(json.contains("\"self_evolution_promotion_preflight_counters\":{"));
        assert!(json.contains("\"review_packets\":2"));
        assert!(json.contains("\"content_digests\":3"));
    }
}
