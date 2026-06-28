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
            let trace_experience_ids = format!(
                "[{}]",
                report
                    .trace_experience_ids
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let json = format!(
                "{{\"passed\":{},\"checked_lines\":{},\"used_experiences\":{},\"imported_kv_blocks\":{},\"runtime_kv_weak_import_pressure_milli\":{},\"trace_experience_ids\":{},\"rust_check_events\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_feedback_updates\":{},\"rust_check_feedback_applied\":{},\"business_contract_events\":{},\"business_contract_event_passed\":{},\"business_contract_event_failed\":{},\"business_contract_event_missing_signals\":{},\"business_contract_event_protocol_leaks\":{},\"business_contract_event_substitutions\":{},\"business_contract_event_evasive_denials\":{},\"business_contract_event_raw_passed\":{},\"business_contract_event_raw_failed\":{},\"business_contract_event_response_normalized\":{},\"business_contract_event_sanitized\":{},\"business_contract_event_canonical_fallbacks\":{},\"runtime_error_events\":{},\"runtime_timeout_events\":{},\"self_evolution_admission_events\":{},\"self_evolution_admission_admitted\":{},\"self_evolution_admission_blocked\":{},\"self_evolution_admission_review_packets\":{},\"self_evolution_admission_evidence_ids\":{},\"self_evolution_admission_missing_review_packet_refs\":{},\"self_evolution_experiment_events\":{},\"self_evolution_experiment_admit\":{},\"self_evolution_experiment_hold\":{},\"self_evolution_experiment_reject\":{},\"self_evolution_experiment_rollback\":{},\"self_evolution_experiment_repeated\":{},\"self_evolution_experiment_conflicts\":{},\"self_evolution_experiment_rollback_replayable\":{},\"self_evolution_experiment_active_candidates\":{},\"self_evolution_experiment_write_allowed\":{},\"self_evolution_experiment_applied\":{},\"self_evolution_rollback_replay_events\":{},\"self_evolution_rollback_replay_items\":{},\"self_evolution_rollback_replay_replayable\":{},\"self_evolution_rollback_replay_blocked\":{},\"self_evolution_rollback_replay_all_replayable\":{},\"self_evolution_rollback_replay_rollback_anchor_ids\":{},\"self_evolution_rollback_replay_evidence_ids\":{},\"self_evolution_rollback_replay_active_candidates\":{},\"self_evolution_rollback_replay_item_write_allowed\":{},\"self_evolution_rollback_replay_item_applied\":{},\"self_evolution_rollback_replay_write_allowed\":{},\"self_evolution_rollback_replay_applied\":{},\"self_evolution_rollback_replay_gate_events\":{},\"self_evolution_rollback_replay_gate_admitted\":{},\"self_evolution_rollback_replay_gate_held\":{},\"self_evolution_rollback_replay_gate_review_packets\":{},\"self_evolution_rollback_replay_gate_review_evidence_ids\":{},\"self_evolution_rollback_replay_gate_missing_review_packet_refs\":{},\"self_evolution_rollback_replay_gate_items\":{},\"self_evolution_rollback_replay_gate_replayable\":{},\"self_evolution_rollback_replay_gate_blocked\":{},\"self_evolution_rollback_replay_gate_all_replayable\":{},\"self_evolution_rollback_replay_gate_rollback_anchor_ids\":{},\"self_evolution_rollback_replay_gate_evidence_ids\":{},\"self_evolution_rollback_replay_gate_active_candidates\":{},\"self_evolution_rollback_replay_gate_item_write_allowed\":{},\"self_evolution_rollback_replay_gate_item_applied\":{},\"self_evolution_rollback_replay_gate_plan_write_allowed\":{},\"self_evolution_rollback_replay_gate_plan_applied\":{},\"self_evolution_rollback_replay_gate_write_allowed\":{},\"self_evolution_rollback_replay_gate_applied\":{},\"self_evolution_operator_approval_events\":{},\"self_evolution_operator_approval_approved\":{},\"self_evolution_operator_approval_held\":{},\"self_evolution_operator_approval_review_packets\":{},\"self_evolution_operator_approval_evidence_ids\":{},\"self_evolution_operator_approval_rollback_anchor_ids\":{},\"self_evolution_operator_approval_content_digests\":{},\"self_evolution_operator_approval_source_report_schemas\":{},\"self_evolution_operator_approval_missing_review_packet_refs\":{},\"self_evolution_operator_approval_write_allowed\":{},\"self_evolution_operator_approval_applied\":{},\"improvement_corpus_events\":{},\"improvement_corpus_episodes\":{},\"improvement_corpus_active_adaptation\":{},\"improvement_corpus_compiler_passed\":{},\"improvement_corpus_test_passed\":{},\"improvement_corpus_benchmark_passed\":{},\"improvement_corpus_privacy_rejected\":{},\"improvement_corpus_secret_leaks\":{},\"adaptive_routing_events\":{},\"adaptive_routing_candidates\":{},\"adaptive_routing_include\":{},\"adaptive_routing_compress\":{},\"adaptive_routing_defer\":{},\"adaptive_routing_skip\":{},\"adaptive_routing_input_tokens\":{},\"adaptive_routing_retained_tokens\":{},\"adaptive_routing_saved_tokens\":{},\"task_hierarchy_events\":{},\"task_hierarchy_mutation_records\":{},\"task_hierarchy_route_pressure_milli\":{},\"task_hierarchy_compute_reduction_milli\":{},\"task_hierarchy_depth_total\":{},\"task_hierarchy_route_fanout_total\":{},\"task_hierarchy_threshold_delta_milli\":{},\"task_hierarchy_selected_lanes\":{},\"task_hierarchy_skipped_lanes\":{},\"task_hierarchy_memory_lanes\":{},\"task_hierarchy_skipped_memory_lanes\":{},\"memory_admission_events\":{},\"memory_admission_candidates\":{},\"memory_admission_ready\":{},\"memory_admission_blocked\":{},\"memory_admission_admitted\":{},\"memory_admission_hold\":{},\"memory_admission_reject\":{},\"memory_admission_quarantine\":{},\"memory_admission_review_packets\":{},\"memory_admission_ledger_records\":{},\"memory_admission_ledger_authorized\":{},\"memory_admission_ledger_applied\":{},\"memory_admission_ledger_preview_only\":{},\"memory_admission_ledger_held\":{},\"memory_admission_ledger_rejected\":{},\"memory_admission_ledger_duplicate\":{},\"memory_admission_ledger_decayed\":{},\"memory_admission_ledger_merged\":{},\"memory_admission_ledger_rollback\":{},\"kv_fusion_events\":{},\"kv_fusion_candidates\":{},\"kv_fusion_fused\":{},\"kv_fusion_compressed\":{},\"kv_fusion_skipped\":{},\"kv_fusion_held\":{},\"kv_fusion_rejected\":{},\"kv_fusion_approval_blocked\":{},\"kv_fusion_input_tokens\":{},\"kv_fusion_retained_tokens\":{},\"kv_fusion_saved_tokens\":{},\"noiron_orchestration_events\":{},\"noiron_orchestration_stages\":{},\"noiron_orchestration_failed_stages\":{},\"noiron_orchestration_writes_gated\":{},\"noiron_orchestration_fht_dke_total_tokens\":{},\"orchestration_audit_events\":{},\"orchestration_audit_checked_fields\":{},\"orchestration_audit_failed_fields\":{},\"orchestration_audit_failed_stages\":{},\"orchestration_audit_integrity_failed_fields\":{},\"summary\":{},\"failures\":{}}}",
                report.passed,
                report.checked_lines,
                report.used_experiences,
                report.imported_kv_blocks,
                report.runtime_kv_weak_import_pressure_milli,
                trace_experience_ids,
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
                report.task_hierarchy_depth_total,
                report.task_hierarchy_route_fanout_total,
                report.task_hierarchy_threshold_delta_milli,
                report.task_hierarchy_selected_lanes,
                report.task_hierarchy_skipped_lanes,
                report.task_hierarchy_memory_lanes,
                report.task_hierarchy_skipped_memory_lanes,
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
                report.noiron_orchestration_events,
                report.noiron_orchestration_stages,
                report.noiron_orchestration_failed_stages,
                report.noiron_orchestration_writes_gated,
                report.noiron_orchestration_fht_dke_total_tokens,
                report.orchestration_audit_events,
                report.orchestration_audit_checked_fields,
                report.orchestration_audit_failed_fields,
                report.orchestration_audit_failed_stages,
                report.orchestration_audit_integrity_failed_fields,
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
            let compute_budget_fields = format!(
                "\"fht_dke_events\":{},\"fht_dke_enabled\":{},\"fht_dke_total_tokens\":{},\"fht_dke_dense_tokens\":{},\"fht_dke_routed_tokens\":{},\"fht_dke_kv_exchange_blocks\":{},\"fht_dke_token_split_invalid\":{},\"fht_dke_attention_threshold_milli\":{},\"fht_dke_route_pressure_milli\":{},\"compute_budget_events\":{},\"compute_budget_threshold_delta_milli\":{},\"compute_budget_runtime_kv_budget_pressure_milli\":{},\"compute_budget_low\":{},\"compute_budget_normal\":{},\"compute_budget_expanded\":{},\"compute_budget_selected_candidates\":{},\"compute_budget_low_value_skipped\":{},\"compute_budget_kv_lookups_skipped\":{},\"compute_budget_validation_cost_tokens\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_self_evolving_memory_fusion_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_fanout_before\":{},\"compute_budget_fanout_after\":{},\"compute_budget_fanout_reduction\":{},\"compute_budget_estimated_budget_tokens\":{},\"compute_budget_estimated_spent_tokens\":{},\"compute_budget_estimated_saved_tokens\":{},\"compute_budget_anchor_count\":{},\"compute_budget_anchors_preserved\":{},\"compute_budget_anchor_preservation_failures\":{},\"compute_budget_fallback_triggered\":{},\"compute_budget_write_allowed\":{},\"compute_budget_applied\":{},\"auto_replay_recursive_runtime_items\":{},\"auto_replay_recursive_runtime_calls\":{},\"auto_replay_avg_recursive_call_pressure_milli\":{},\"auto_replay_max_recursive_call_pressure_milli\":{},\"evolution_recursive_replay_items\":{},\"evolution_recursive_runtime_calls\":{}",
                report.fht_dke_events,
                report.fht_dke_enabled,
                report.fht_dke_total_tokens,
                report.fht_dke_dense_tokens,
                report.fht_dke_routed_tokens,
                report.fht_dke_kv_exchange_blocks,
                report.fht_dke_token_split_invalid,
                report.fht_dke_attention_threshold_milli,
                report.fht_dke_route_pressure_milli,
                report.compute_budget_events,
                report.compute_budget_threshold_delta_milli,
                report.compute_budget_runtime_kv_budget_pressure_milli,
                report.compute_budget_low,
                report.compute_budget_normal,
                report.compute_budget_expanded,
                report.compute_budget_selected_candidates,
                report.compute_budget_low_value_skipped,
                report.compute_budget_kv_lookups_skipped,
                report.compute_budget_validation_cost_tokens,
                report.compute_budget_saved_tokens,
                report.compute_budget_self_evolving_memory_fusion_saved_tokens,
                report.compute_budget_avoided_tokens,
                report.compute_budget_fanout_before,
                report.compute_budget_fanout_after,
                report.compute_budget_fanout_reduction,
                report.compute_budget_estimated_budget_tokens,
                report.compute_budget_estimated_spent_tokens,
                report.compute_budget_estimated_saved_tokens,
                report.compute_budget_anchor_count,
                report.compute_budget_anchors_preserved,
                report.compute_budget_anchor_preservation_failures,
                report.compute_budget_fallback_triggered,
                report.compute_budget_write_allowed,
                report.compute_budget_applied,
                report.auto_replay_recursive_runtime_items,
                report.auto_replay_recursive_runtime_calls,
                report.auto_replay_avg_recursive_call_pressure_milli,
                report.auto_replay_max_recursive_call_pressure_milli,
                report.evolution_recursive_replay_items,
                report.evolution_recursive_runtime_calls,
            );
            let json = json.replacen(
                "\"memory_admission_events\"",
                &format!("{compute_budget_fields},\"memory_admission_events\""),
                1,
            );
            let operator_approval_counters = format!(
                "\"self_evolution_operator_approval_counters\":{}",
                report
                    .self_evolution_operator_approval_service_counters()
                    .json_object()
            );
            let process_reward_counters = format!(
                "\"process_reward_counters\":{{\"events\":{},\"positive\":{},\"reinforce\":{},\"hold\":{},\"penalize\":{},\"total_milli\":{}}}",
                report.process_reward_events,
                report.process_reward_positive,
                report.process_reward_reinforce,
                report.process_reward_hold,
                report.process_reward_penalize,
                report.process_reward_total_milli,
            );
            let live_evolution_counters = format!(
                "\"live_evolution_counters\":{{\"events\":{},\"router_threshold_delta_milli\":{},\"hierarchy_weight_delta_milli\":{},\"online_reward_feedbacks\":{},\"online_reward_reinforcements\":{},\"online_reward_penalties\":{},\"online_reward_strength_milli\":{},\"memory_reinforcements\":{},\"memory_penalties\":{},\"memory_updates\":{},\"stored_memories\":{},\"stored_gist_memories\":{},\"stored_runtime_kv_memories\":{},\"stored_memory_updates\":{},\"reflection_issues\":{},\"critical_reflection_issues\":{},\"revision_actions\":{}}}",
                report.live_evolution_events,
                report.live_router_threshold_delta_milli,
                report.live_hierarchy_weight_delta_milli,
                report.live_online_reward_feedbacks,
                report.live_online_reward_reinforcements,
                report.live_online_reward_penalties,
                report.live_online_reward_strength_milli,
                report.live_memory_reinforcements,
                report.live_memory_penalties,
                report.live_memory_updates,
                report.live_stored_memories,
                report.live_stored_gist_memories,
                report.live_stored_runtime_kv_memories,
                report.live_stored_memory_updates,
                report.live_reflection_issues,
                report.live_critical_reflection_issues,
                report.live_revision_actions,
            );
            let evolution_live_counters = format!(
                "\"evolution_live_counters\":{{\"inference_runs\":{},\"router_threshold_mutations\":{},\"hierarchy_weight_mutations\":{},\"router_threshold_delta_milli\":{},\"hierarchy_weight_delta_milli\":{},\"online_reward_feedbacks\":{},\"online_reward_reinforcements\":{},\"online_reward_penalties\":{},\"online_reward_strength_milli\":{},\"online_reward_reinforcement_strength_milli\":{},\"online_reward_penalty_strength_milli\":{},\"memory_reinforcements\":{},\"memory_penalties\":{},\"memory_updates\":{},\"stored_memories\":{},\"stored_gist_memories\":{},\"stored_runtime_kv_memories\":{},\"stored_memory_updates\":{},\"reflection_issues\":{},\"critical_reflection_issues\":{},\"revision_actions\":{}}}",
                report.evolution_live_inference_runs,
                report.evolution_live_router_threshold_mutations,
                report.evolution_live_hierarchy_weight_mutations,
                report.evolution_live_router_threshold_delta_milli,
                report.evolution_live_hierarchy_weight_delta_milli,
                report.evolution_live_online_reward_feedbacks,
                report.evolution_live_online_reward_reinforcements,
                report.evolution_live_online_reward_penalties,
                report.evolution_live_online_reward_strength_milli,
                report.evolution_live_online_reward_reinforcement_strength_milli,
                report.evolution_live_online_reward_penalty_strength_milli,
                report.evolution_live_memory_reinforcements,
                report.evolution_live_memory_penalties,
                report.evolution_live_memory_updates,
                report.evolution_live_stored_memories,
                report.evolution_live_stored_gist_memories,
                report.evolution_live_stored_runtime_kv_memories,
                report.evolution_live_stored_memory_updates,
                report.evolution_live_reflection_issues,
                report.evolution_live_critical_reflection_issues,
                report.evolution_live_revision_actions,
            );
            let replay_live_evolution_counters = format!(
                "\"replay_live_evolution_counters\":{{\"items\":{},\"router_threshold_mutations\":{},\"hierarchy_weight_mutations\":{},\"router_threshold_delta_milli\":{},\"hierarchy_weight_delta_milli\":{},\"online_reward_feedbacks\":{},\"online_reward_reinforcements\":{},\"online_reward_penalties\":{},\"online_reward_strength_milli\":{},\"online_reward_reinforcement_strength_milli\":{},\"online_reward_penalty_strength_milli\":{},\"memory_updates\":{},\"stored_memory_updates\":{},\"reflection_issues\":{},\"critical_reflection_issues\":{},\"revision_actions\":{}}}",
                report.replay_live_evolution_items,
                report.replay_live_evolution_router_threshold_mutations,
                report.replay_live_evolution_hierarchy_weight_mutations,
                report.replay_live_evolution_router_threshold_delta_milli,
                report.replay_live_evolution_hierarchy_weight_delta_milli,
                report.replay_live_evolution_online_reward_feedbacks,
                report.replay_live_evolution_online_reward_reinforcements,
                report.replay_live_evolution_online_reward_penalties,
                report.replay_live_evolution_online_reward_strength_milli,
                report.replay_live_evolution_online_reward_reinforcement_strength_milli,
                report.replay_live_evolution_online_reward_penalty_strength_milli,
                report.replay_live_evolution_memory_updates,
                report.replay_live_evolution_stored_memory_updates,
                report.replay_live_evolution_reflection_issues,
                report.replay_live_evolution_critical_reflection_issues,
                report.replay_live_evolution_revision_actions,
            );
            let reasoning_genome_counters = format!(
                "\"reasoning_genome_counters\":{{\"events\":{},\"genes\":{},\"active_genes\":{},\"aged_genes\":{},\"malignant_genes\":{},\"relabel_candidates\":{},\"regeneration_candidates\":{},\"gene_scissors_proposals\":{},\"repair_payloads\":{},\"regeneration_payloads\":{},\"lifecycle_records\":{},\"lifecycle_tombstone_candidates\":{},\"lifecycle_pending_validations\":{},\"lifecycle_source_evidence\":{},\"splice_segments\":{},\"splice_exons\":{},\"splice_introns\":{},\"splice_variants\":{},\"splice_quarantined\":{},\"splice_repair_candidates\":{},\"splice_findings\":{},\"splice_proposals\":{},\"write_allowed\":{},\"mutation_applied\":{},\"splice_write_allowed\":{},\"splice_applied\":{}}}",
                report.reasoning_genome_events,
                report.reasoning_genome_genes,
                report.reasoning_genome_active_genes,
                report.reasoning_genome_aged_genes,
                report.reasoning_genome_malignant_genes,
                report.reasoning_genome_relabel_candidates,
                report.reasoning_genome_regeneration_candidates,
                report.reasoning_genome_gene_scissors_proposals,
                report.reasoning_genome_repair_payloads,
                report.reasoning_genome_regeneration_payloads,
                report.reasoning_genome_lifecycle_records,
                report.reasoning_genome_lifecycle_tombstone_candidates,
                report.reasoning_genome_lifecycle_pending_validations,
                report.reasoning_genome_lifecycle_source_evidence,
                report.reasoning_genome_splice_segments,
                report.reasoning_genome_splice_exons,
                report.reasoning_genome_splice_introns,
                report.reasoning_genome_splice_variants,
                report.reasoning_genome_splice_quarantined,
                report.reasoning_genome_splice_repair_candidates,
                report.reasoning_genome_splice_findings,
                report.reasoning_genome_splice_proposals,
                report.reasoning_genome_write_allowed,
                report.reasoning_genome_mutation_applied,
                report.reasoning_genome_splice_write_allowed,
                report.reasoning_genome_splice_applied,
            );
            let self_evolving_memory_counters = format!(
                "\"self_evolving_memory_counters\":{{\"store_events\":{},\"retrieval_events\":{},\"maintenance_events\":{},\"admission_preview_events\":{},\"consolidation_events\":{},\"consolidation_actions\":{},\"merge_previews\":{},\"decay_previews\":{},\"tombstone_previews\":{},\"merge_rejections\":{},\"contexts\":{},\"store_saved_tokens\":{},\"maintenance_actions\":{},\"admission_candidates\":{},\"store_write_allowed\":{},\"store_durable_write_allowed\":{},\"store_applied\":{},\"store_applied_to_disk\":{},\"source_quarantine_events\":{},\"source_quarantine_actions\":{},\"writeback_events\":{},\"writeback_source_case_digests\":{},\"writeback_attempted_records\":{},\"writeback_accepted_records\":{},\"writeback_rejected_records\":{},\"writeback_records_before\":{},\"writeback_records_after\":{},\"writeback_tool_reliability_after\":{},\"writeback_tool_observations_after\":{},\"writeback_maintenance_actions\":{},\"writeback_merged_duplicate_episodes\":{},\"writeback_write_allowed\":{},\"writeback_durable_write_allowed\":{},\"writeback_applied\":{},\"writeback_applied_to_disk\":{},\"writeback_snapshot_changes\":{},\"residency_events\":{},\"residency_decisions\":{},\"residency_hot\":{},\"residency_warm\":{},\"residency_cold\":{},\"residency_quarantined\":{},\"residency_retired\":{},\"residency_protected_rollback_anchors\":{},\"residency_blocked_reasons\":{},\"residency_token_estimate\":{},\"residency_write_allowed\":{},\"residency_durable_write_allowed\":{},\"residency_applied\":{}}}",
                report.self_evolving_memory_store_events,
                report.self_evolving_memory_store_retrieval_events,
                report.self_evolving_memory_store_maintenance_events,
                report.self_evolving_memory_store_admission_preview_events,
                report.self_evolving_memory_store_consolidation_events,
                report.self_evolving_memory_store_consolidation_actions,
                report.self_evolving_memory_store_merge_previews,
                report.self_evolving_memory_store_decay_previews,
                report.self_evolving_memory_store_tombstone_previews,
                report.self_evolving_memory_store_merge_rejections,
                report.self_evolving_memory_store_contexts,
                report.self_evolving_memory_store_saved_tokens,
                report.self_evolving_memory_store_maintenance_actions,
                report.self_evolving_memory_store_admission_candidates,
                report.self_evolving_memory_store_write_allowed,
                report.self_evolving_memory_store_durable_write_allowed,
                report.self_evolving_memory_store_applied,
                report.self_evolving_memory_store_applied_to_disk,
                report.self_evolving_memory_store_source_quarantine_events,
                report.self_evolving_memory_store_source_quarantine_actions,
                report.self_evolving_memory_writeback_events,
                report.self_evolving_memory_writeback_source_case_digests,
                report.self_evolving_memory_writeback_attempted_records,
                report.self_evolving_memory_writeback_accepted_records,
                report.self_evolving_memory_writeback_rejected_records(),
                report.self_evolving_memory_writeback_records_before,
                report.self_evolving_memory_writeback_records_after,
                report.self_evolving_memory_writeback_tool_reliability_after,
                report.self_evolving_memory_writeback_tool_observations_after,
                report.self_evolving_memory_writeback_maintenance_actions,
                report.self_evolving_memory_writeback_merged_duplicate_episodes,
                report.self_evolving_memory_writeback_write_allowed,
                report.self_evolving_memory_writeback_durable_write_allowed,
                report.self_evolving_memory_writeback_applied,
                report.self_evolving_memory_writeback_applied_to_disk,
                report.self_evolving_memory_writeback_snapshot_changes,
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
            );
            let unified_writer_gate_counters = format!(
                "\"unified_writer_gate_counters\":{{\"events\":{},\"records\":{},\"memory_records\":{},\"genome_records\":{},\"experiment_ledger_records\":{},\"evolution_goal_queue_records\":{},\"ready_records\":{},\"held_records\":{},\"rejected_records\":{},\"preview_only_records\":{},\"reason_codes\":{},\"explicit_apply_required\":{},\"write_allowed\":{},\"durable_write_allowed\":{},\"applied\":{}}}",
                report.unified_writer_gate_events,
                report.unified_writer_gate_records,
                report.unified_writer_gate_memory_records,
                report.unified_writer_gate_genome_records,
                report.unified_writer_gate_experiment_ledger_records,
                report.unified_writer_gate_evolution_goal_queue_records,
                report.unified_writer_gate_ready_records,
                report.unified_writer_gate_held_records,
                report.unified_writer_gate_rejected_records,
                report.unified_writer_gate_preview_only_records,
                report.unified_writer_gate_reason_codes,
                report.unified_writer_gate_explicit_apply_required,
                report.unified_writer_gate_write_allowed,
                report.unified_writer_gate_durable_write_allowed,
                report.unified_writer_gate_applied,
            );
            let self_goal_queue_counters = format!(
                "\"self_goal_queue_counters\":{{\"apply_events\":{},\"apply_records\":{},\"apply_ready_records\":{},\"apply_held_records\":{},\"apply_rejected_records\":{},\"apply_reason_codes\":{},\"apply_explicit_apply_required\":{},\"apply_write_allowed\":{},\"apply_applied\":{},\"continuation_events\":{},\"continuation_ready\":{},\"continuation_held\":{},\"continuation_current_queue\":{},\"continuation_completion_resulting_queue\":{},\"continuation_goals\":{},\"continuation_required_evidence\":{},\"continuation_reason_codes\":{},\"continuation_budget_attempts\":{},\"continuation_budget_steps\":{},\"continuation_budget_tokens\":{},\"continuation_budget_runtime_ms\":{},\"continuation_write_allowed\":{},\"continuation_applied\":{},\"evidence_plan_events\":{},\"evidence_plan_ready\":{},\"evidence_plan_held\":{},\"evidence_plan_steps\":{},\"evidence_plan_auto_collectible\":{},\"evidence_plan_manual\":{},\"evidence_plan_required_evidence\":{},\"evidence_plan_packet_templates\":{},\"evidence_plan_command_templates\":{},\"evidence_plan_write_allowed\":{},\"evidence_plan_applied\":{},\"evidence_collection_events\":{},\"evidence_collection_ready\":{},\"evidence_collection_complete\":{},\"evidence_collection_steps\":{},\"evidence_collection_collected\":{},\"evidence_collection_passed\":{},\"evidence_collection_failed\":{},\"evidence_collection_missing\":{},\"evidence_collection_manual_missing\":{},\"evidence_collection_write_allowed\":{},\"evidence_collection_applied\":{},\"local_evidence_events\":{},\"local_evidence_enabled\":{},\"local_evidence_dry_run\":{},\"local_evidence_ready\":{},\"local_evidence_steps\":{},\"local_evidence_attempted\":{},\"local_evidence_generated\":{},\"local_evidence_passed\":{},\"local_evidence_failed\":{},\"local_evidence_skipped\":{},\"local_evidence_manual\":{},\"local_evidence_planned_status\":{},\"local_evidence_write_allowed\":{},\"local_evidence_applied\":{}}}",
                report.self_goal_queue_apply_events,
                report.self_goal_queue_apply_records,
                report.self_goal_queue_apply_ready_records,
                report.self_goal_queue_apply_held_records,
                report.self_goal_queue_apply_rejected_records,
                report.self_goal_queue_apply_reason_codes,
                report.self_goal_queue_apply_explicit_apply_required,
                report.self_goal_queue_apply_write_allowed,
                report.self_goal_queue_apply_applied,
                report.self_goal_queue_continuation_events,
                report.self_goal_queue_continuation_ready,
                report.self_goal_queue_continuation_held,
                report.self_goal_queue_continuation_current_queue,
                report.self_goal_queue_continuation_completion_resulting_queue,
                report.self_goal_queue_continuation_goals,
                report.self_goal_queue_continuation_required_evidence,
                report.self_goal_queue_continuation_reason_codes,
                report.self_goal_queue_continuation_budget_attempts,
                report.self_goal_queue_continuation_budget_steps,
                report.self_goal_queue_continuation_budget_tokens,
                report.self_goal_queue_continuation_budget_runtime_ms,
                report.self_goal_queue_continuation_write_allowed,
                report.self_goal_queue_continuation_applied,
                report.self_goal_queue_evidence_plan_events,
                report.self_goal_queue_evidence_plan_ready,
                report.self_goal_queue_evidence_plan_held,
                report.self_goal_queue_evidence_plan_steps,
                report.self_goal_queue_evidence_plan_auto_collectible,
                report.self_goal_queue_evidence_plan_manual,
                report.self_goal_queue_evidence_plan_required_evidence,
                report.self_goal_queue_evidence_plan_packet_templates,
                report.self_goal_queue_evidence_plan_command_templates,
                report.self_goal_queue_evidence_plan_write_allowed,
                report.self_goal_queue_evidence_plan_applied,
                report.self_goal_queue_evidence_collection_events,
                report.self_goal_queue_evidence_collection_ready,
                report.self_goal_queue_evidence_collection_complete,
                report.self_goal_queue_evidence_collection_steps,
                report.self_goal_queue_evidence_collection_collected,
                report.self_goal_queue_evidence_collection_passed,
                report.self_goal_queue_evidence_collection_failed,
                report.self_goal_queue_evidence_collection_missing,
                report.self_goal_queue_evidence_collection_manual_missing,
                report.self_goal_queue_evidence_collection_write_allowed,
                report.self_goal_queue_evidence_collection_applied,
                report.self_goal_local_evidence_events,
                report.self_goal_local_evidence_enabled,
                report.self_goal_local_evidence_dry_run,
                report.self_goal_local_evidence_ready,
                report.self_goal_local_evidence_steps,
                report.self_goal_local_evidence_attempted,
                report.self_goal_local_evidence_generated,
                report.self_goal_local_evidence_passed,
                report.self_goal_local_evidence_failed,
                report.self_goal_local_evidence_skipped,
                report.self_goal_local_evidence_manual,
                report.self_goal_local_evidence_planned_status,
                report.self_goal_local_evidence_write_allowed,
                report.self_goal_local_evidence_applied,
            );
            let coding_service_eval_counters = format!(
                "\"coding_service_eval_counters\":{{\"events\":{},\"readiness_events\":{},\"runner_events\":{},\"passed\":{},\"requests\":{},\"completed\":{},\"evidence_packets\":{},\"rust_validation_checked\":{},\"compile_checked\":{},\"unit_test_checked\":{},\"write_allowed\":{},\"applied\":{}}}",
                report.coding_service_eval_events,
                report.coding_service_eval_readiness_events,
                report.coding_service_eval_runner_events,
                report.coding_service_eval_passed,
                report.coding_service_eval_requests,
                report.coding_service_eval_completed,
                report.coding_service_eval_evidence_packets,
                report.coding_service_eval_rust_validation_checked,
                report.coding_service_eval_compile_checked,
                report.coding_service_eval_unit_test_checked,
                report.coding_service_eval_write_allowed,
                report.coding_service_eval_applied,
            );
            let evolution_goal_queue_store_write_counters = format!(
                "\"evolution_goal_queue_store_write_counters\":{{\"events\":{},\"applied\":{},\"held\":{},\"rejected\":{},\"reason_codes\":{},\"durable_write_allowed\":{},\"applied_to_disk\":{}}}",
                report.evolution_goal_queue_store_write_events,
                report.evolution_goal_queue_store_write_applied,
                report.evolution_goal_queue_store_write_held,
                report.evolution_goal_queue_store_write_rejected,
                report.evolution_goal_queue_store_write_reason_codes,
                report.evolution_goal_queue_store_write_durable_write_allowed,
                report.evolution_goal_queue_store_write_applied_to_disk,
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
                    "{experiment_counters},{rollback_replay_counters},{operator_approval_counters},{process_reward_counters},{live_evolution_counters},{evolution_live_counters},{replay_live_evolution_counters},{reasoning_genome_counters},{self_evolving_memory_counters},{unified_writer_gate_counters},{self_goal_queue_counters},{evolution_goal_queue_store_write_counters},{coding_service_eval_counters},{promotion_preflight_counters},{rollback_replay_apply_counters},\"summary\""
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
            used_experiences: 6,
            imported_kv_blocks: 7,
            runtime_kv_weak_import_pressure_milli: 600,
            trace_experience_ids: vec![21, 22],
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
            task_hierarchy_depth_total: 6,
            task_hierarchy_route_fanout_total: 5,
            task_hierarchy_threshold_delta_milli: 140,
            task_hierarchy_selected_lanes: 4,
            task_hierarchy_skipped_lanes: 2,
            task_hierarchy_memory_lanes: 3,
            task_hierarchy_skipped_memory_lanes: 1,
            fht_dke_events: 2,
            fht_dke_enabled: 2,
            fht_dke_total_tokens: 18,
            fht_dke_dense_tokens: 8,
            fht_dke_routed_tokens: 10,
            fht_dke_kv_exchange_blocks: 3,
            fht_dke_token_split_invalid: 0,
            fht_dke_attention_threshold_milli: 625,
            fht_dke_route_pressure_milli: 410,
            compute_budget_events: 2,
            compute_budget_threshold_delta_milli: 314,
            compute_budget_runtime_kv_budget_pressure_milli: 815,
            compute_budget_low: 1,
            compute_budget_normal: 1,
            compute_budget_expanded: 0,
            compute_budget_selected_candidates: 9,
            compute_budget_low_value_skipped: 3,
            compute_budget_kv_lookups_skipped: 4,
            compute_budget_validation_cost_tokens: 11,
            compute_budget_saved_tokens: 120,
            compute_budget_self_evolving_memory_fusion_saved_tokens: 37,
            compute_budget_avoided_tokens: 90,
            compute_budget_fanout_before: 12,
            compute_budget_fanout_after: 5,
            compute_budget_fanout_reduction: 7,
            compute_budget_estimated_budget_tokens: 256,
            compute_budget_estimated_spent_tokens: 180,
            compute_budget_estimated_saved_tokens: 76,
            compute_budget_anchor_count: 4,
            compute_budget_anchors_preserved: 4,
            compute_budget_anchor_preservation_failures: 0,
            compute_budget_fallback_triggered: 1,
            compute_budget_write_allowed: 0,
            compute_budget_applied: 0,
            auto_replay_recursive_runtime_items: 1,
            auto_replay_recursive_runtime_calls: 4,
            auto_replay_avg_recursive_call_pressure_milli: 400,
            auto_replay_max_recursive_call_pressure_milli: 800,
            evolution_recursive_replay_items: 1,
            evolution_recursive_runtime_calls: 4,
            process_reward_events: 2,
            process_reward_positive: 2,
            process_reward_reinforce: 2,
            process_reward_hold: 0,
            process_reward_penalize: 0,
            process_reward_total_milli: 1762,
            live_evolution_events: 2,
            live_router_threshold_delta_milli: 125,
            live_hierarchy_weight_delta_milli: 250,
            live_online_reward_feedbacks: 2,
            live_online_reward_reinforcements: 2,
            live_online_reward_penalties: 0,
            live_online_reward_strength_milli: 1730,
            live_memory_reinforcements: 3,
            live_memory_penalties: 1,
            live_memory_updates: 4,
            live_stored_memories: 2,
            live_stored_gist_memories: 3,
            live_stored_runtime_kv_memories: 3,
            live_stored_memory_updates: 8,
            live_reflection_issues: 1,
            live_critical_reflection_issues: 0,
            live_revision_actions: 1,
            evolution_live_inference_runs: 3,
            evolution_live_router_threshold_mutations: 2,
            evolution_live_hierarchy_weight_mutations: 1,
            evolution_live_router_threshold_delta_milli: 150,
            evolution_live_hierarchy_weight_delta_milli: 300,
            evolution_live_online_reward_feedbacks: 4,
            evolution_live_online_reward_reinforcements: 3,
            evolution_live_online_reward_penalties: 1,
            evolution_live_online_reward_strength_milli: 2048,
            evolution_live_online_reward_reinforcement_strength_milli: 1800,
            evolution_live_online_reward_penalty_strength_milli: 248,
            evolution_live_memory_reinforcements: 5,
            evolution_live_memory_penalties: 1,
            evolution_live_memory_updates: 6,
            evolution_live_stored_memories: 7,
            evolution_live_stored_gist_memories: 3,
            evolution_live_stored_runtime_kv_memories: 2,
            evolution_live_stored_memory_updates: 9,
            evolution_live_reflection_issues: 2,
            evolution_live_critical_reflection_issues: 1,
            evolution_live_revision_actions: 2,
            replay_live_evolution_items: 2,
            replay_live_evolution_router_threshold_mutations: 1,
            replay_live_evolution_hierarchy_weight_mutations: 1,
            replay_live_evolution_router_threshold_delta_milli: 75,
            replay_live_evolution_hierarchy_weight_delta_milli: 125,
            replay_live_evolution_online_reward_feedbacks: 3,
            replay_live_evolution_online_reward_reinforcements: 2,
            replay_live_evolution_online_reward_penalties: 1,
            replay_live_evolution_online_reward_strength_milli: 1024,
            replay_live_evolution_online_reward_reinforcement_strength_milli: 900,
            replay_live_evolution_online_reward_penalty_strength_milli: 124,
            replay_live_evolution_memory_updates: 4,
            replay_live_evolution_stored_memory_updates: 5,
            replay_live_evolution_reflection_issues: 1,
            replay_live_evolution_critical_reflection_issues: 1,
            replay_live_evolution_revision_actions: 1,
            reasoning_genome_events: 1,
            reasoning_genome_genes: 4,
            reasoning_genome_active_genes: 3,
            reasoning_genome_malignant_genes: 1,
            reasoning_genome_gene_scissors_proposals: 2,
            reasoning_genome_repair_payloads: 1,
            reasoning_genome_regeneration_payloads: 1,
            reasoning_genome_splice_segments: 6,
            reasoning_genome_splice_exons: 5,
            reasoning_genome_splice_repair_candidates: 2,
            reasoning_genome_write_allowed: 0,
            reasoning_genome_mutation_applied: 0,
            reasoning_genome_splice_write_allowed: 0,
            reasoning_genome_splice_applied: 0,
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
            self_evolving_memory_store_contexts: 4,
            self_evolving_memory_store_saved_tokens: 64,
            self_evolving_memory_store_maintenance_actions: 2,
            self_evolving_memory_store_admission_candidates: 5,
            self_evolving_memory_store_write_allowed: 0,
            self_evolving_memory_store_durable_write_allowed: 0,
            self_evolving_memory_store_applied: 0,
            self_evolving_memory_store_applied_to_disk: 0,
            self_evolving_memory_store_source_quarantine_events: 1,
            self_evolving_memory_store_source_quarantine_actions: 2,
            self_evolving_memory_writeback_events: 1,
            self_evolving_memory_writeback_source_case_digests: 1,
            self_evolving_memory_writeback_attempted_records: 3,
            self_evolving_memory_writeback_accepted_records: 3,
            self_evolving_memory_writeback_records_before: 4,
            self_evolving_memory_writeback_records_after: 7,
            self_evolving_memory_writeback_tool_reliability_after: 2,
            self_evolving_memory_writeback_tool_observations_after: 4,
            self_evolving_memory_writeback_maintenance_actions: 1,
            self_evolving_memory_writeback_merged_duplicate_episodes: 1,
            self_evolving_memory_writeback_write_allowed: 1,
            self_evolving_memory_writeback_durable_write_allowed: 1,
            self_evolving_memory_writeback_applied: 1,
            self_evolving_memory_writeback_applied_to_disk: 1,
            self_evolving_memory_writeback_snapshot_changes: 1,
            memory_residency_events: 1,
            memory_residency_decisions: 6,
            memory_residency_hot: 2,
            memory_residency_warm: 2,
            memory_residency_cold: 1,
            memory_residency_quarantined: 1,
            memory_residency_protected_rollback_anchors: 1,
            memory_residency_blocked_reasons: 2,
            memory_residency_token_estimate: 128,
            memory_residency_write_allowed: 0,
            memory_residency_durable_write_allowed: 0,
            memory_residency_applied: 0,
            unified_writer_gate_events: 2,
            unified_writer_gate_records: 2,
            unified_writer_gate_memory_records: 1,
            unified_writer_gate_genome_records: 1,
            unified_writer_gate_ready_records: 1,
            unified_writer_gate_held_records: 1,
            unified_writer_gate_preview_only_records: 1,
            unified_writer_gate_reason_codes: 2,
            unified_writer_gate_explicit_apply_required: 1,
            unified_writer_gate_write_allowed: 0,
            unified_writer_gate_durable_write_allowed: 0,
            unified_writer_gate_applied: 0,
            self_goal_queue_apply_events: 1,
            self_goal_queue_apply_records: 2,
            self_goal_queue_apply_ready_records: 1,
            self_goal_queue_apply_held_records: 1,
            self_goal_queue_apply_reason_codes: 2,
            self_goal_queue_apply_explicit_apply_required: 1,
            self_goal_queue_apply_write_allowed: 0,
            self_goal_queue_apply_applied: 0,
            self_goal_queue_continuation_events: 1,
            self_goal_queue_continuation_ready: 1,
            self_goal_queue_continuation_current_queue: 1,
            self_goal_queue_continuation_completion_resulting_queue: 1,
            self_goal_queue_continuation_goals: 2,
            self_goal_queue_continuation_required_evidence: 3,
            self_goal_queue_continuation_reason_codes: 2,
            self_goal_queue_continuation_budget_attempts: 1,
            self_goal_queue_continuation_budget_steps: 6,
            self_goal_queue_continuation_budget_tokens: 144,
            self_goal_queue_continuation_budget_runtime_ms: 900,
            self_goal_queue_evidence_plan_events: 1,
            self_goal_queue_evidence_plan_ready: 1,
            self_goal_queue_evidence_plan_steps: 4,
            self_goal_queue_evidence_plan_auto_collectible: 3,
            self_goal_queue_evidence_plan_manual: 1,
            self_goal_queue_evidence_plan_required_evidence: 3,
            self_goal_queue_evidence_plan_packet_templates: 4,
            self_goal_queue_evidence_plan_command_templates: 4,
            self_goal_queue_evidence_collection_events: 1,
            self_goal_queue_evidence_collection_ready: 1,
            self_goal_queue_evidence_collection_complete: 0,
            self_goal_queue_evidence_collection_steps: 4,
            self_goal_queue_evidence_collection_collected: 2,
            self_goal_queue_evidence_collection_passed: 1,
            self_goal_queue_evidence_collection_failed: 1,
            self_goal_queue_evidence_collection_missing: 1,
            self_goal_queue_evidence_collection_manual_missing: 1,
            self_goal_local_evidence_events: 1,
            self_goal_local_evidence_enabled: 1,
            self_goal_local_evidence_dry_run: 0,
            self_goal_local_evidence_ready: 1,
            self_goal_local_evidence_steps: 4,
            self_goal_local_evidence_attempted: 3,
            self_goal_local_evidence_generated: 2,
            self_goal_local_evidence_passed: 2,
            self_goal_local_evidence_failed: 1,
            self_goal_local_evidence_skipped: 1,
            self_goal_local_evidence_manual: 1,
            self_goal_local_evidence_planned_status: 4,
            coding_service_eval_events: 1,
            coding_service_eval_runner_events: 1,
            coding_service_eval_passed: 1,
            coding_service_eval_requests: 5,
            coding_service_eval_completed: 5,
            coding_service_eval_evidence_packets: 5,
            coding_service_eval_rust_validation_checked: 2,
            coding_service_eval_compile_checked: 2,
            coding_service_eval_unit_test_checked: 2,
            coding_service_eval_write_allowed: 0,
            coding_service_eval_applied: 0,
            evolution_goal_queue_store_write_events: 1,
            evolution_goal_queue_store_write_applied: 1,
            evolution_goal_queue_store_write_reason_codes: 1,
            evolution_goal_queue_store_write_durable_write_allowed: 1,
            evolution_goal_queue_store_write_applied_to_disk: 1,
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
            noiron_orchestration_events: 1,
            noiron_orchestration_stages: 8,
            noiron_orchestration_failed_stages: 0,
            noiron_orchestration_writes_gated: 1,
            noiron_orchestration_fht_dke_total_tokens: 18,
            orchestration_audit_events: 1,
            orchestration_audit_checked_fields: 9,
            orchestration_audit_failed_fields: 2,
            orchestration_audit_failed_stages: 1,
            orchestration_audit_integrity_failed_fields: 1,
            failures: Vec::new(),
            ..TraceSchemaGateReport::default()
        };

        let json = option_trace_gate_service_json(Some(&report));

        assert!(json.contains("\"used_experiences\":6"));
        assert!(json.contains("\"imported_kv_blocks\":7"));
        assert!(json.contains("\"runtime_kv_weak_import_pressure_milli\":600"));
        assert!(json.contains("\"trace_experience_ids\":[21,22]"));
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
        assert!(json.contains("\"adaptive_routing_events\":2"));
        assert!(json.contains("\"adaptive_routing_candidates\":5"));
        assert!(json.contains("\"adaptive_routing_saved_tokens\":192"));
        assert!(json.contains("\"task_hierarchy_events\":2"));
        assert!(json.contains("\"task_hierarchy_mutation_records\":4"));
        assert!(json.contains("\"task_hierarchy_compute_reduction_milli\":280"));
        assert!(json.contains("\"task_hierarchy_depth_total\":6"));
        assert!(json.contains("\"task_hierarchy_route_fanout_total\":5"));
        assert!(json.contains("\"task_hierarchy_threshold_delta_milli\":140"));
        assert!(json.contains("\"task_hierarchy_selected_lanes\":4"));
        assert!(json.contains("\"task_hierarchy_skipped_lanes\":2"));
        assert!(json.contains("\"task_hierarchy_memory_lanes\":3"));
        assert!(json.contains("\"task_hierarchy_skipped_memory_lanes\":1"));
        assert!(json.contains("\"fht_dke_events\":2"));
        assert!(json.contains("\"fht_dke_enabled\":2"));
        assert!(json.contains("\"fht_dke_total_tokens\":18"));
        assert!(json.contains("\"fht_dke_dense_tokens\":8"));
        assert!(json.contains("\"fht_dke_routed_tokens\":10"));
        assert!(json.contains("\"fht_dke_kv_exchange_blocks\":3"));
        assert!(json.contains("\"fht_dke_token_split_invalid\":0"));
        assert!(json.contains("\"fht_dke_attention_threshold_milli\":625"));
        assert!(json.contains("\"fht_dke_route_pressure_milli\":410"));
        assert!(json.contains("\"compute_budget_events\":2"));
        assert!(json.contains("\"compute_budget_threshold_delta_milli\":314"));
        assert!(json.contains("\"compute_budget_runtime_kv_budget_pressure_milli\":815"));
        assert!(json.contains("\"compute_budget_low\":1"));
        assert!(json.contains("\"compute_budget_selected_candidates\":9"));
        assert!(json.contains("\"compute_budget_kv_lookups_skipped\":4"));
        assert!(json.contains("\"compute_budget_validation_cost_tokens\":11"));
        assert!(json.contains("\"compute_budget_saved_tokens\":120"));
        assert!(json.contains("\"compute_budget_self_evolving_memory_fusion_saved_tokens\":37"));
        assert!(json.contains("\"compute_budget_avoided_tokens\":90"));
        assert!(json.contains("\"compute_budget_fanout_before\":12"));
        assert!(json.contains("\"compute_budget_fanout_after\":5"));
        assert!(json.contains("\"compute_budget_fanout_reduction\":7"));
        assert!(json.contains("\"compute_budget_estimated_budget_tokens\":256"));
        assert!(json.contains("\"compute_budget_estimated_spent_tokens\":180"));
        assert!(json.contains("\"compute_budget_estimated_saved_tokens\":76"));
        assert!(json.contains("\"compute_budget_anchor_count\":4"));
        assert!(json.contains("\"compute_budget_anchors_preserved\":4"));
        assert!(json.contains("\"compute_budget_anchor_preservation_failures\":0"));
        assert!(json.contains("\"compute_budget_fallback_triggered\":1"));
        assert!(json.contains("\"compute_budget_write_allowed\":0"));
        assert!(json.contains("\"compute_budget_applied\":0"));
        assert!(json.contains("\"auto_replay_recursive_runtime_items\":1"));
        assert!(json.contains("\"auto_replay_recursive_runtime_calls\":4"));
        assert!(json.contains("\"auto_replay_avg_recursive_call_pressure_milli\":400"));
        assert!(json.contains("\"auto_replay_max_recursive_call_pressure_milli\":800"));
        assert!(json.contains("\"evolution_recursive_replay_items\":1"));
        assert!(json.contains("\"evolution_recursive_runtime_calls\":4"));
        assert!(json.contains("\"process_reward_counters\":{"));
        assert!(json.contains("\"positive\":2"));
        assert!(json.contains("\"reinforce\":2"));
        assert!(json.contains("\"penalize\":0"));
        assert!(json.contains("\"total_milli\":1762"));
        assert!(json.contains("\"live_evolution_counters\":{"));
        assert!(json.contains("\"router_threshold_delta_milli\":125"));
        assert!(json.contains("\"hierarchy_weight_delta_milli\":250"));
        assert!(json.contains("\"online_reward_feedbacks\":2"));
        assert!(json.contains("\"online_reward_strength_milli\":1730"));
        assert!(json.contains("\"memory_reinforcements\":3"));
        assert!(json.contains("\"memory_penalties\":1"));
        assert!(json.contains("\"memory_updates\":4"));
        assert!(json.contains("\"stored_memories\":2"));
        assert!(json.contains("\"stored_gist_memories\":3"));
        assert!(json.contains("\"stored_runtime_kv_memories\":3"));
        assert!(json.contains("\"stored_memory_updates\":8"));
        assert!(json.contains("\"reflection_issues\":1"));
        assert!(json.contains("\"revision_actions\":1"));
        assert!(json.contains("\"evolution_live_counters\":{\"inference_runs\":3,\"router_threshold_mutations\":2,\"hierarchy_weight_mutations\":1,\"router_threshold_delta_milli\":150,\"hierarchy_weight_delta_milli\":300,\"online_reward_feedbacks\":4,\"online_reward_reinforcements\":3,\"online_reward_penalties\":1,\"online_reward_strength_milli\":2048,\"online_reward_reinforcement_strength_milli\":1800,\"online_reward_penalty_strength_milli\":248,\"memory_reinforcements\":5,\"memory_penalties\":1,\"memory_updates\":6,\"stored_memories\":7,\"stored_gist_memories\":3,\"stored_runtime_kv_memories\":2,\"stored_memory_updates\":9,\"reflection_issues\":2,\"critical_reflection_issues\":1,\"revision_actions\":2}"));
        assert!(json.contains("\"replay_live_evolution_counters\":{\"items\":2,\"router_threshold_mutations\":1,\"hierarchy_weight_mutations\":1,\"router_threshold_delta_milli\":75,\"hierarchy_weight_delta_milli\":125,\"online_reward_feedbacks\":3,\"online_reward_reinforcements\":2,\"online_reward_penalties\":1,\"online_reward_strength_milli\":1024,\"online_reward_reinforcement_strength_milli\":900,\"online_reward_penalty_strength_milli\":124,\"memory_updates\":4,\"stored_memory_updates\":5,\"reflection_issues\":1,\"critical_reflection_issues\":1,\"revision_actions\":1}"));
        assert!(json.contains("\"reasoning_genome_counters\":{"));
        assert!(json.contains("\"genes\":4"));
        assert!(json.contains("\"malignant_genes\":1"));
        assert!(json.contains("\"gene_scissors_proposals\":2"));
        assert!(json.contains("\"splice_segments\":6"));
        assert!(json.contains("\"splice_repair_candidates\":2"));
        assert!(json.contains("\"mutation_applied\":0"));
        assert!(json.contains("\"self_evolving_memory_counters\":{"));
        assert!(json.contains("\"store_events\":4"));
        assert!(json.contains("\"retrieval_events\":1"));
        assert!(json.contains("\"consolidation_events\":1"));
        assert!(json.contains("\"consolidation_actions\":3"));
        assert!(json.contains("\"merge_previews\":1"));
        assert!(json.contains("\"decay_previews\":1"));
        assert!(json.contains("\"tombstone_previews\":1"));
        assert!(json.contains("\"store_saved_tokens\":64"));
        assert!(json.contains("\"admission_candidates\":5"));
        assert!(json.contains("\"store_durable_write_allowed\":0"));
        assert!(json.contains("\"source_quarantine_events\":1"));
        assert!(json.contains("\"source_quarantine_actions\":2"));
        assert!(json.contains("\"writeback_events\":1"));
        assert!(json.contains("\"writeback_source_case_digests\":1"));
        assert!(json.contains("\"writeback_attempted_records\":3"));
        assert!(json.contains("\"writeback_accepted_records\":3"));
        assert!(json.contains("\"writeback_rejected_records\":0"));
        assert!(json.contains("\"writeback_records_before\":4"));
        assert!(json.contains("\"writeback_records_after\":7"));
        assert!(json.contains("\"writeback_tool_reliability_after\":2"));
        assert!(json.contains("\"writeback_tool_observations_after\":4"));
        assert!(json.contains("\"writeback_maintenance_actions\":1"));
        assert!(json.contains("\"writeback_merged_duplicate_episodes\":1"));
        assert!(json.contains("\"writeback_write_allowed\":1"));
        assert!(json.contains("\"writeback_durable_write_allowed\":1"));
        assert!(json.contains("\"writeback_applied\":1"));
        assert!(json.contains("\"writeback_snapshot_changes\":1"));
        assert!(json.contains("\"writeback_applied_to_disk\":1"));
        assert!(json.contains("\"residency_events\":1"));
        assert!(json.contains("\"residency_quarantined\":1"));
        assert!(json.contains("\"residency_token_estimate\":128"));
        assert!(json.contains("\"unified_writer_gate_counters\":{"));
        assert!(json.contains("\"memory_records\":1"));
        assert!(json.contains("\"genome_records\":1"));
        assert!(json.contains("\"ready_records\":1"));
        assert!(json.contains("\"held_records\":1"));
        assert!(json.contains("\"explicit_apply_required\":1"));
        assert!(json.contains("\"durable_write_allowed\":0"));
        assert!(json.contains("\"self_goal_queue_counters\":{"));
        assert!(json.contains("\"apply_events\":1"));
        assert!(json.contains("\"apply_records\":2"));
        assert!(json.contains("\"continuation_current_queue\":1"));
        assert!(json.contains("\"continuation_completion_resulting_queue\":1"));
        assert!(json.contains("\"continuation_reason_codes\":2"));
        assert!(json.contains("\"continuation_budget_attempts\":1"));
        assert!(json.contains("\"continuation_budget_steps\":6"));
        assert!(json.contains("\"continuation_budget_tokens\":144"));
        assert!(json.contains("\"continuation_budget_runtime_ms\":900"));
        assert!(json.contains("\"evidence_plan_steps\":4"));
        assert!(json.contains("\"evidence_plan_required_evidence\":3"));
        assert!(json.contains("\"evidence_plan_packet_templates\":4"));
        assert!(json.contains("\"evidence_plan_command_templates\":4"));
        assert!(json.contains("\"evidence_collection_manual_missing\":1"));
        assert!(json.contains("\"local_evidence_generated\":2"));
        assert!(json.contains("\"local_evidence_planned_status\":4"));
        assert!(json.contains("\"evolution_goal_queue_store_write_counters\":{"));
        assert!(json.contains("\"applied_to_disk\":1"));
        assert!(json.contains("\"coding_service_eval_counters\":{\"events\":1,\"readiness_events\":0,\"runner_events\":1,\"passed\":1,\"requests\":5,\"completed\":5,\"evidence_packets\":5,\"rust_validation_checked\":2,\"compile_checked\":2,\"unit_test_checked\":2,\"write_allowed\":0,\"applied\":0}"));
        assert!(json.contains("\"memory_admission_events\":1"));
        assert!(json.contains("\"memory_admission_candidates\":3"));
        assert!(json.contains("\"memory_admission_ledger_records\":3"));
        assert!(json.contains("\"memory_admission_ledger_preview_only\":1"));
        assert!(json.contains("\"kv_fusion_events\":1"));
        assert!(json.contains("\"kv_fusion_candidates\":3"));
        assert!(json.contains("\"kv_fusion_saved_tokens\":100"));
        assert!(json.contains("\"noiron_orchestration_events\":1"));
        assert!(json.contains("\"noiron_orchestration_stages\":8"));
        assert!(json.contains("\"noiron_orchestration_failed_stages\":0"));
        assert!(json.contains("\"noiron_orchestration_writes_gated\":1"));
        assert!(json.contains("\"noiron_orchestration_fht_dke_total_tokens\":18"));
        assert!(json.contains("\"orchestration_audit_events\":1"));
        assert!(json.contains("\"orchestration_audit_checked_fields\":9"));
        assert!(json.contains("\"orchestration_audit_failed_fields\":2"));
        assert!(json.contains("\"orchestration_audit_failed_stages\":1"));
        assert!(json.contains("\"orchestration_audit_integrity_failed_fields\":1"));
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
        assert!(json.contains("fht_dke_events=2"));
        assert!(json.contains("noiron_orchestration_events=1"));
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
