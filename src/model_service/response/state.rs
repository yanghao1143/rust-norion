use rust_norion::{
    StateExperienceHygieneFinding, StateExperienceIndexFinding, StateExperienceSummary,
    StateInspectionGateReport, StateInspectionReport, StateMemorySummary,
    StateMemoryVectorDimensions, TaskProfile, TraceSchemaGateReport,
};

use super::super::json::service_json_string;
use super::gates::{option_state_gate_service_json, option_trace_gate_service_json};
use crate::cli::state::{runtime_state_bucket, RuntimeStateBucketSummary};
use crate::Args;

pub(crate) fn model_service_state_response_json(
    request_id: usize,
    args: &Args,
    report: &StateInspectionReport,
    state_gate_report: Option<&StateInspectionGateReport>,
    trace_gate_report: Option<&TraceSchemaGateReport>,
) -> String {
    let runtime_state_bucket = runtime_state_bucket(args);
    format!(
        "{{\"ok\":true,\"request_id\":{},\"runtime_state_bucket\":{},\"state\":{},\"state_gate\":{},\"trace_gate\":{}}}",
        request_id,
        runtime_state_bucket_service_json(&runtime_state_bucket),
        model_service_state_json(report),
        option_state_gate_service_json(state_gate_report),
        option_trace_gate_service_json(trace_gate_report)
    )
}

pub(crate) fn runtime_state_bucket_service_json(summary: &RuntimeStateBucketSummary) -> String {
    format!(
        "{{\"current\":{},\"memory_file\":{},\"experience_file\":{},\"adaptive_file\":{},\"in_current_bucket\":{},\"legacy_root_artifacts\":{},\"stale_version_buckets\":{}}}",
        service_json_string(&summary.current.display().to_string()),
        service_json_string(&summary.memory_path.display().to_string()),
        service_json_string(&summary.experience_path.display().to_string()),
        service_json_string(&summary.adaptive_path.display().to_string()),
        summary.in_current_bucket,
        summary.legacy_root_artifacts,
        summary.stale_version_buckets
    )
}

pub(super) fn model_service_state_json(report: &StateInspectionReport) -> String {
    let json = format!(
        "{{\"summary\":{},\"memories\":{},\"runtime_kv_memories\":{},\"experiences\":{},\"experience_hygiene_findings\":{},\"experience_hygiene_watch\":{},\"experience_hygiene_quarantine_candidates\":{},\"experience_hygiene_legacy_metadata_lessons\":{},\"experience_hygiene_legacy_metadata_without_clean_gist\":{},\"experience_repairable_legacy_metadata_lessons\":{},\"experience_repairable_index_records\":{},\"experience_repair_projected_findings\":{},\"experience_repair_projected_watch\":{},\"experience_repair_projected_quarantine_candidates\":{},\"experience_repair_projected_legacy_metadata_lessons\":{},\"experience_repair_projected_legacy_metadata_without_clean_gist\":{},\"experience_repair_skipped_quarantine_candidates\":{},\"experience_repair_skipped_missing_clean_gist\":{},\"experience_hygiene_clean\":{},\"experience_hygiene_samples\":{},\"experience_index_compacted_records\":{},\"experience_index_overlong_records\":{},\"experience_index_overlong_without_clean_gist\":{},\"experience_index_max_record_chars\":{},\"experience_index_noisy_records\":{},\"experience_index_duplicate_outputs\":{},\"experience_index_max_noise_penalty\":{:.6},\"experience_index_quality_score\":{:.6},\"experience_index_retrieval_ready\":{},\"experience_index_risk_level\":{},\"experience_index_samples\":{},\"process_reward_experiences\":{},\"process_reward_positive\":{},\"process_reward_reinforce\":{},\"process_reward_hold\":{},\"process_reward_penalize\":{},\"process_reward_total\":{:.6},\"runtime_model_experiences\":{},\"runtime_tokens\":{},\"runtime_imported_kv_blocks\":{},\"runtime_architecture_experiences\":{},\"runtime_kv_precision_experiences\":{},\"runtime_device_execution_experiences\":{},\"runtime_error_experiences\":{},\"runtime_errors\":{},\"runtime_timeout_experiences\":{},\"runtime_timeouts\":{},\"runtime_error_message_chars\":{},\"reflection_issue_experiences\":{},\"critical_reflection_issue_experiences\":{},\"revision_action_experiences\":{},\"live_memory_feedback_experiences\":{},\"live_memory_feedback_updates\":{},\"live_memory_feedback_reinforced\":{},\"live_memory_feedback_penalized\":{},\"live_memory_feedback_detail_experiences\":{},\"live_memory_feedback_applied\":{},\"live_memory_feedback_removed\":{},\"live_memory_feedback_missing\":{},\"live_memory_feedback_strength_delta\":{:.6},\"rust_check_experiences\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_diagnostic_chars\":{},\"business_contract_experiences\":{},\"business_contract_passed\":{},\"business_contract_failed\":{},\"business_contract_required_signals\":{},\"business_contract_matched_signals\":{},\"business_contract_missing_signals\":{},\"business_contract_protocol_leaks\":{},\"business_contract_substitutions\":{},\"business_contract_evasive_denials\":{},\"business_contract_missing_handling_signals\":{},\"business_contract_raw_passed\":{},\"business_contract_raw_failed\":{},\"business_contract_response_normalized\":{},\"business_contract_sanitized\":{},\"business_contract_canonical_fallbacks\":{},\"pool_dispatch_experiences\":{},\"pool_dispatch_items\":{},\"pool_dispatch_forwarded\":{},\"pool_dispatch_clamped\":{},\"pool_dispatch_low_priority\":{},\"evolution_live_inference_runs\":{},\"evolution_live_router_threshold_mutations\":{},\"evolution_live_hierarchy_weight_mutations\":{},\"evolution_live_router_threshold_delta\":{:.6},\"evolution_live_hierarchy_weight_delta\":{:.6},\"evolution_live_online_reward_feedbacks\":{},\"evolution_live_online_reward_reinforcements\":{},\"evolution_live_online_reward_penalties\":{},\"evolution_live_online_reward_strength\":{:.6},\"evolution_live_online_reward_reinforcement_strength\":{:.6},\"evolution_live_online_reward_penalty_strength\":{:.6},\"evolution_live_memory_reinforcements\":{},\"evolution_live_memory_penalties\":{},\"evolution_live_stored_memories\":{},\"evolution_live_stored_gist_memories\":{},\"evolution_live_stored_runtime_kv_memories\":{},\"evolution_live_memory_updates\":{},\"evolution_live_stored_memory_updates\":{},\"evolution_live_reflection_issues\":{},\"evolution_live_critical_reflection_issues\":{},\"evolution_live_revision_actions\":{},\"evolution_replay_runs\":{},\"evolution_replay_items\":{},\"evolution_external_feedbacks\":{},\"evolution_external_feedback_memory_updates\":{},\"evolution_external_feedback_strength_delta\":{:.6},\"evolution_replay_rust_check_items\":{},\"evolution_replay_rust_check_passed\":{},\"evolution_replay_rust_check_failed\":{},\"evolution_replay_rust_check_live_memory_feedback_updates\":{},\"evolution_replay_rust_check_live_memory_feedback_applied\":{},\"evolution_replay_rust_check_live_memory_feedback_strength_delta\":{:.6},\"evolution_replay_business_contract_items\":{},\"evolution_replay_business_contract_passed\":{},\"evolution_replay_business_contract_failed\":{},\"evolution_replay_business_contract_raw_passed\":{},\"evolution_replay_business_contract_raw_failed\":{},\"evolution_replay_business_contract_raw_audits\":{},\"evolution_replay_business_contract_response_normalized\":{},\"evolution_replay_business_contract_sanitized\":{},\"evolution_replay_business_contract_canonical_fallbacks\":{},\"router_threshold\":{:.6}}}",
        service_json_string(&report.summary_line()),
        report.memory_count,
        report.runtime_kv_memory_count,
        report.experience_count,
        report.experience_hygiene_finding_count,
        report.experience_hygiene_watch_count,
        report.experience_hygiene_quarantine_candidate_count,
        report.experience_hygiene_legacy_metadata_lesson_count,
        report.experience_hygiene_legacy_metadata_without_clean_gist_count,
        report.experience_repairable_legacy_metadata_lesson_count,
        report.experience_repairable_index_record_count,
        report.experience_repair_projected_hygiene_finding_count,
        report.experience_repair_projected_hygiene_watch_count,
        report.experience_repair_projected_hygiene_quarantine_candidate_count,
        report.experience_repair_projected_legacy_metadata_lesson_count,
        report.experience_repair_projected_legacy_metadata_without_clean_gist_count,
        report.experience_repair_skipped_quarantine_candidate_count,
        report.experience_repair_skipped_missing_clean_gist_count,
        report.experience_hygiene_quarantine_candidate_count == 0,
        experience_hygiene_samples_json(&report.experience_hygiene_findings),
        report.experience_index_compacted_record_count,
        report.experience_index_overlong_record_count,
        report.experience_index_overlong_without_clean_gist_count,
        report.experience_index_max_record_chars,
        report.experience_index_noisy_record_count,
        report.experience_index_duplicate_output_count,
        report.experience_index_max_noise_penalty,
        report.experience_index_quality_score,
        report.experience_index_retrieval_ready,
        service_json_string(&report.experience_index_risk_level),
        experience_index_samples_json(&report.experience_index_findings),
        report.process_reward_experience_count,
        report.process_reward_positive_count,
        report.process_reward_reinforce_count,
        report.process_reward_hold_count,
        report.process_reward_penalize_count,
        report.process_reward_total,
        report.runtime_model_experience_count,
        report.runtime_token_count,
        report.runtime_imported_kv_blocks,
        report.runtime_architecture_experience_count,
        report.runtime_kv_precision_experience_count,
        report.runtime_device_execution_experience_count,
        report.runtime_error_experience_count,
        report.runtime_error_count,
        report.runtime_timeout_experience_count,
        report.runtime_timeout_count,
        report.runtime_error_message_chars,
        report.reflection_issue_experience_count,
        report.critical_reflection_issue_experience_count,
        report.revision_action_experience_count,
        report.live_memory_feedback_experience_count,
        report.live_memory_feedback_update_count,
        report.live_memory_feedback_reinforced_count,
        report.live_memory_feedback_penalized_count,
        report.live_memory_feedback_detail_experience_count,
        report.live_memory_feedback_applied_count,
        report.live_memory_feedback_removed_count,
        report.live_memory_feedback_missing_count,
        report.live_memory_feedback_strength_delta,
        report.rust_check_experience_count,
        report.rust_check_passed_count,
        report.rust_check_failed_count,
        report.rust_check_diagnostic_chars,
        report.business_contract_experience_count,
        report.business_contract_passed_count,
        report.business_contract_failed_count,
        report.business_contract_required_signals,
        report.business_contract_matched_signals,
        report.business_contract_missing_signals,
        report.business_contract_protocol_leaks,
        report.business_contract_substitutions,
        report.business_contract_evasive_denials,
        report.business_contract_missing_handling_signals,
        report.business_contract_raw_passed_count,
        report.business_contract_raw_failed_count,
        report.business_contract_response_normalized_count,
        report.business_contract_sanitized_count,
        report.business_contract_canonical_fallback_count,
        report.pool_dispatch_experience_count,
        report.pool_dispatch_item_count,
        report.pool_dispatch_forwarded_count,
        report.pool_dispatch_clamped_count,
        report.pool_dispatch_low_priority_count,
        report.evolution_ledger.live_inference_runs,
        report.evolution_ledger.live_router_threshold_mutations,
        report.evolution_ledger.live_hierarchy_weight_mutations,
        report.evolution_ledger.live_router_threshold_delta,
        report.evolution_ledger.live_hierarchy_weight_delta,
        report.evolution_ledger.live_online_reward_feedbacks,
        report.evolution_ledger.live_online_reward_reinforcements,
        report.evolution_ledger.live_online_reward_penalties,
        report.evolution_ledger.live_online_reward_strength,
        report
            .evolution_ledger
            .live_online_reward_reinforcement_strength,
        report.evolution_ledger.live_online_reward_penalty_strength,
        report.evolution_ledger.live_memory_reinforcements,
        report.evolution_ledger.live_memory_penalties,
        report.evolution_ledger.live_stored_memories,
        report.evolution_ledger.live_stored_gist_memories,
        report.evolution_ledger.live_stored_runtime_kv_memories,
        report.evolution_ledger.live_memory_updates(),
        report.evolution_ledger.live_stored_memory_updates(),
        report.evolution_ledger.live_reflection_issues,
        report.evolution_ledger.live_critical_reflection_issues,
        report.evolution_ledger.live_revision_actions,
        report.evolution_ledger.replay_runs,
        report.evolution_ledger.replay_items,
        report.evolution_ledger.external_feedbacks,
        report.evolution_ledger.external_feedback_memory_updates,
        report.evolution_ledger.external_feedback_strength_delta,
        report.evolution_ledger.replay_rust_check_items,
        report.evolution_ledger.replay_rust_check_passed,
        report.evolution_ledger.replay_rust_check_failed,
        report
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_updates,
        report
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_applied,
        report
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_strength_delta,
        report.evolution_ledger.replay_business_contract_items,
        report.evolution_ledger.replay_business_contract_passed,
        report.evolution_ledger.replay_business_contract_failed,
        report.evolution_ledger.replay_business_contract_raw_passed,
        report.evolution_ledger.replay_business_contract_raw_failed,
        report
            .evolution_ledger
            .replay_business_contract_raw_audits(),
        report
            .evolution_ledger
            .replay_business_contract_response_normalized,
        report.evolution_ledger.replay_business_contract_sanitized,
        report
            .evolution_ledger
            .replay_business_contract_canonical_fallbacks,
        report.router_threshold
    );
    let runtime_observability_fields = format!(
        "\"runtime_adapter_experiences\":{},\"runtime_adapter_selection_mismatches\":{},\"runtime_forward_energy_experiences\":{},\"runtime_kv_influence_experiences\":{},\"runtime_kv_precision_mismatches\":{},\"runtime_uncertainty_experiences\":{},\"runtime_uncertainty_tokens\":{},\"runtime_layer_mode_experiences\":{},\"runtime_all_layer_mode_experiences\":{},\"runtime_global_layers\":{},\"runtime_local_window_layers\":{},\"runtime_convolutional_fusion_layers\":{},\"runtime_kv_import_experiences\":{},\"runtime_kv_hold_experiences\":{},\"runtime_kv_held_blocks\":{}",
        report.runtime_adapter_experience_count,
        report.runtime_adapter_selection_mismatch_count,
        report.runtime_forward_energy_experience_count,
        report.runtime_kv_influence_experience_count,
        report.runtime_kv_precision_mismatch_count,
        report.runtime_uncertainty_experience_count,
        report.runtime_uncertainty_token_count,
        report.runtime_layer_mode_experience_count,
        report.runtime_all_layer_mode_experience_count,
        report.runtime_global_layers,
        report.runtime_local_window_layers,
        report.runtime_convolutional_fusion_layers,
        report.runtime_kv_import_experience_count,
        report.runtime_kv_hold_experience_count,
        report.runtime_kv_held_blocks,
    );
    let runtime_kv_fields = format!(
        "\"runtime_kv_export_experiences\":{},\"runtime_kv_weak_import_skip_experiences\":{},\"weak_runtime_kv_imports_skipped\":{},\"runtime_kv_weak_import_pressure_experiences\":{},\"runtime_kv_weak_import_pressure_avg\":{:.6},\"runtime_kv_weak_import_pressure_max\":{:.6},\"runtime_kv_budget_import_skip_experiences\":{},\"budget_limited_runtime_kv_imports_skipped\":{},\"runtime_kv_budget_pressure_experiences\":{},\"runtime_kv_budget_pressure_avg\":{:.6},\"runtime_kv_budget_pressure_max\":{:.6},\"runtime_kv_segment_experiences\":{},\"runtime_kv_segments_included\":{},\"runtime_kv_segments_skipped\":{},\"runtime_kv_segments_rejected\":{}",
        report.runtime_kv_export_experience_count,
        report.runtime_kv_weak_import_skip_experience_count,
        report.weak_runtime_kv_imports_skipped,
        report.runtime_kv_weak_import_pressure_experience_count,
        report.runtime_kv_weak_import_pressure_avg,
        report.runtime_kv_weak_import_pressure_max,
        report.runtime_kv_budget_import_skip_experience_count,
        report.budget_limited_runtime_kv_imports_skipped,
        report.runtime_kv_budget_pressure_experience_count,
        report.runtime_kv_budget_pressure_avg,
        report.runtime_kv_budget_pressure_max,
        report.runtime_kv_segment_experience_count,
        report.runtime_kv_segments_included,
        report.runtime_kv_segments_skipped,
        report.runtime_kv_segments_rejected,
    );
    let fht_dke_fields = format!(
        "\"fht_dke_budget_experiences\":{},\"fht_dke_enabled_experiences\":{},\"fht_dke_total_tokens\":{},\"fht_dke_dense_tokens\":{},\"fht_dke_routed_tokens\":{},\"fht_dke_kv_exchange_blocks\":{},\"fht_dke_token_split_valid\":{},\"fht_dke_token_split_invalid\":{},\"fht_dke_attention_threshold_experiences\":{},\"fht_dke_attention_threshold_avg\":{:.6},\"fht_dke_attention_threshold_max\":{:.6},\"fht_dke_route_pressure_experiences\":{},\"fht_dke_route_pressure_avg\":{:.6},\"fht_dke_route_pressure_max\":{:.6}",
        report.fht_dke_budget_experience_count,
        report.fht_dke_enabled_experience_count,
        report.fht_dke_total_tokens,
        report.fht_dke_dense_tokens,
        report.fht_dke_routed_tokens,
        report.fht_dke_kv_exchange_blocks,
        report.fht_dke_token_split_valid_count,
        report.fht_dke_token_split_invalid_count,
        report.fht_dke_attention_threshold_experience_count,
        report.fht_dke_attention_threshold_avg,
        report.fht_dke_attention_threshold_max,
        report.fht_dke_route_pressure_experience_count,
        report.fht_dke_route_pressure_avg,
        report.fht_dke_route_pressure_max,
    );
    let adaptive_routing_fields = format!(
        "\"router_observations\":{},\"profile_observations_general\":{},\"profile_observations_coding\":{},\"profile_observations_writing\":{},\"profile_observations_long_document\":{},\"profile_threshold_general\":{:.6},\"profile_threshold_coding\":{:.6},\"profile_threshold_writing\":{:.6},\"profile_threshold_long_document\":{:.6}",
        report.router_observations,
        report.profile_observations.general,
        report.profile_observations.coding,
        report.profile_observations.writing,
        report.profile_observations.long_document,
        report.profile_thresholds.general,
        report.profile_thresholds.coding,
        report.profile_thresholds.writing,
        report.profile_thresholds.long_document,
    );
    let hierarchy_fields = format!(
        "\"hierarchy_global\":{:.6},\"hierarchy_local\":{:.6},\"hierarchy_convolution\":{:.6},\"profile_hierarchy_observations_general\":{},\"profile_hierarchy_observations_coding\":{},\"profile_hierarchy_observations_writing\":{},\"profile_hierarchy_observations_long_document\":{},\"profile_hierarchy_local_general\":{:.6},\"profile_hierarchy_local_coding\":{:.6},\"profile_hierarchy_local_writing\":{:.6},\"profile_hierarchy_local_long_document\":{:.6}",
        report.hierarchy.global,
        report.hierarchy.local,
        report.hierarchy.convolution,
        report.profile_hierarchy_observations.general,
        report.profile_hierarchy_observations.coding,
        report.profile_hierarchy_observations.writing,
        report.profile_hierarchy_observations.long_document,
        report.profile_hierarchy_weights.general.local,
        report.profile_hierarchy_weights.coding.local,
        report.profile_hierarchy_weights.writing.local,
        report.profile_hierarchy_weights.long_document.local,
    );
    let tier_fields = format!(
        "\"memory_tier_hot_gpu\":{},\"memory_tier_warm_ram\":{},\"memory_tier_cold_disk\":{}",
        report.tier_counts.hot_gpu, report.tier_counts.warm_ram, report.tier_counts.cold_disk,
    );
    let vector_dimension_fields = format!(
        "\"memory_vector_dimensions\":{},\"runtime_kv_vector_dimensions\":{}",
        memory_vector_dimensions_json(&report.memory_vector_dimensions),
        memory_vector_dimensions_json(&report.runtime_kv_vector_dimensions),
    );
    let memory_metric_fields = format!(
        "\"top_memory_metrics\":{},\"top_runtime_kv_memory_metrics\":{},\"top_experience_metrics\":{}",
        memory_summaries_json(&report.top_memories),
        memory_summaries_json(&report.top_runtime_kv_memories),
        experience_summaries_json(&report.top_experiences),
    );
    let memory_policy_fields = format!(
        "\"memory_retention_policy\":{{\"stale_after\":{},\"decay_rate\":{:.6},\"remove_below_strength\":{:.6},\"remove_after_failures\":{}}},\"memory_compaction_policy\":{{\"similarity_threshold\":{:.6},\"max_candidates\":{},\"max_merges\":{}}}",
        report.memory_retention_policy.stale_after,
        report.memory_retention_policy.decay_rate,
        report.memory_retention_policy.remove_below_strength,
        report.memory_retention_policy.remove_after_failures,
        report.memory_compaction_policy.similarity_threshold,
        report.memory_compaction_policy.max_candidates,
        report.memory_compaction_policy.max_merges,
    );
    let replay_adjustment_fields = format!(
        "\"evolution_router_threshold_mutations\":{},\"evolution_hierarchy_weight_mutations\":{},\"evolution_router_threshold_delta\":{:.6},\"evolution_hierarchy_weight_delta\":{:.6},\"evolution_memory_updates\":{},\"evolution_replay_rust_check_diagnostic_chars\":{},\"evolution_replay_rust_check_live_memory_feedback_items\":{}",
        report.evolution_ledger.router_threshold_mutations,
        report.evolution_ledger.hierarchy_weight_mutations,
        report.evolution_ledger.router_threshold_delta,
        report.evolution_ledger.hierarchy_weight_delta,
        report.evolution_ledger.memory_updates(),
        report.evolution_ledger.replay_rust_check_diagnostic_chars,
        report
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_items,
    );
    let feedback_loop_fields = format!(
        "\"evolution_external_feedback_reinforcements\":{},\"evolution_external_feedback_penalties\":{},\"evolution_external_feedback_removed\":{},\"evolution_external_feedback_missing\":{},\"evolution_replay_live_memory_feedback_items\":{},\"evolution_replay_live_memory_feedback_updates\":{},\"evolution_replay_live_memory_feedback_reinforcements\":{},\"evolution_replay_live_memory_feedback_penalties\":{},\"evolution_replay_live_memory_feedback_detail_items\":{},\"evolution_replay_live_memory_feedback_applied\":{},\"evolution_replay_live_memory_feedback_removed\":{},\"evolution_replay_live_memory_feedback_missing\":{},\"evolution_replay_live_memory_feedback_strength_delta\":{:.6}",
        report.evolution_ledger.external_feedback_reinforcements,
        report.evolution_ledger.external_feedback_penalties,
        report.evolution_ledger.external_feedback_removed,
        report.evolution_ledger.external_feedback_missing,
        report.evolution_ledger.replay_live_memory_feedback_items,
        report
            .evolution_ledger
            .replay_live_memory_feedback_updates(),
        report
            .evolution_ledger
            .replay_live_memory_feedback_reinforcements,
        report
            .evolution_ledger
            .replay_live_memory_feedback_penalties,
        report
            .evolution_ledger
            .replay_live_memory_feedback_detail_items,
        report.evolution_ledger.replay_live_memory_feedback_applied,
        report.evolution_ledger.replay_live_memory_feedback_removed,
        report.evolution_ledger.replay_live_memory_feedback_missing,
        report
            .evolution_ledger
            .replay_live_memory_feedback_strength_delta,
    );
    let replay_evolution_fields = format!(
        "\"evolution_replay_live_evolution_items\":{},\"evolution_replay_live_evolution_router_threshold_mutations\":{},\"evolution_replay_live_evolution_hierarchy_weight_mutations\":{},\"evolution_replay_live_evolution_router_threshold_delta\":{:.6},\"evolution_replay_live_evolution_hierarchy_weight_delta\":{:.6},\"evolution_replay_live_evolution_online_reward_feedbacks\":{},\"evolution_replay_live_evolution_online_reward_reinforcements\":{},\"evolution_replay_live_evolution_online_reward_penalties\":{},\"evolution_replay_live_evolution_online_reward_strength\":{:.6},\"evolution_replay_live_evolution_online_reward_reinforcement_strength\":{:.6},\"evolution_replay_live_evolution_online_reward_penalty_strength\":{:.6},\"evolution_replay_live_evolution_memory_updates\":{},\"evolution_replay_live_evolution_stored_memory_updates\":{},\"evolution_replay_live_evolution_reflection_issues\":{},\"evolution_replay_live_evolution_critical_reflection_issues\":{},\"evolution_replay_live_evolution_revision_actions\":{},\"evolution_recursive_replay_items\":{},\"evolution_recursive_runtime_calls\":{},\"evolution_drift_rollbacks\":{},\"evolution_rollback_router_threshold_delta\":{:.6},\"evolution_rollback_hierarchy_weight_delta\":{:.6}",
        report.evolution_ledger.replay_live_evolution_items,
        report
            .evolution_ledger
            .replay_live_evolution_router_threshold_mutations,
        report
            .evolution_ledger
            .replay_live_evolution_hierarchy_weight_mutations,
        report
            .evolution_ledger
            .replay_live_evolution_router_threshold_delta,
        report
            .evolution_ledger
            .replay_live_evolution_hierarchy_weight_delta,
        report
            .evolution_ledger
            .replay_live_evolution_online_reward_feedbacks,
        report
            .evolution_ledger
            .replay_live_evolution_online_reward_reinforcements,
        report
            .evolution_ledger
            .replay_live_evolution_online_reward_penalties,
        report
            .evolution_ledger
            .replay_live_evolution_online_reward_strength,
        report
            .evolution_ledger
            .replay_live_evolution_online_reward_reinforcement_strength,
        report
            .evolution_ledger
            .replay_live_evolution_online_reward_penalty_strength,
        report.evolution_ledger.replay_live_evolution_memory_updates,
        report
            .evolution_ledger
            .replay_live_evolution_stored_memory_updates,
        report
            .evolution_ledger
            .replay_live_evolution_reflection_issues,
        report
            .evolution_ledger
            .replay_live_evolution_critical_reflection_issues,
        report
            .evolution_ledger
            .replay_live_evolution_revision_actions,
        report.evolution_ledger.recursive_replay_items,
        report.evolution_ledger.recursive_runtime_calls,
        report.evolution_ledger.drift_rollbacks,
        report.evolution_ledger.rollback_router_threshold_delta,
        report.evolution_ledger.rollback_hierarchy_weight_delta,
    );
    let external_semantic_context_fields = format!(
        "\"external_semantic_context_experiences\":{},\"external_semantic_contexts\":{}",
        report.external_semantic_context_experience_count, report.external_semantic_context_count,
    );
    let self_evolving_memory_writeback_fields = format!(
        "\"self_evolving_memory_writeback_experiences\":{},\"self_evolving_memory_writeback_attempted_records\":{},\"self_evolving_memory_writeback_accepted_records\":{},\"self_evolving_memory_writeback_rejected_records\":{},\"self_evolving_memory_writeback_records_before\":{},\"self_evolving_memory_writeback_records_after\":{},\"self_evolving_memory_writeback_tool_reliability_after\":{},\"self_evolving_memory_writeback_tool_observations_after\":{},\"self_evolving_memory_writeback_maintenance_actions\":{},\"self_evolving_memory_writeback_merged_duplicate_episodes\":{},\"self_evolving_memory_writeback_write_allowed\":{},\"self_evolving_memory_writeback_durable_write_allowed\":{},\"self_evolving_memory_writeback_applied\":{},\"self_evolving_memory_writeback_applied_to_disk\":{},\"self_evolving_memory_writeback_snapshot_changes\":{}",
        report.self_evolving_memory_writeback_experience_count,
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
    );
    json.replacen(
        "\"router_threshold\"",
        &format!(
            "{runtime_observability_fields},{runtime_kv_fields},{fht_dke_fields},{adaptive_routing_fields},{hierarchy_fields},{tier_fields},{vector_dimension_fields},{memory_metric_fields},{memory_policy_fields},{replay_adjustment_fields},{feedback_loop_fields},{replay_evolution_fields},{external_semantic_context_fields},{self_evolving_memory_writeback_fields},\"router_threshold\""
        ),
        1,
    )
}

fn experience_summaries_json(experiences: &[StateExperienceSummary]) -> String {
    let experiences = experiences
        .iter()
        .map(|experience| {
            format!(
                "{{\"id\":{},\"profile\":\"{}\",\"quality\":{:.6},\"process_reward\":{:.6},\"reward_action\":\"{}\",\"runtime_layer_count\":{},\"runtime_global_layers\":{},\"runtime_local_window_layers\":{},\"runtime_convolutional_fusion_layers\":{},\"runtime_hidden_size\":{},\"runtime_local_window_tokens\":{},\"runtime_forward_energy\":{},\"runtime_kv_influence\":{},\"runtime_tokens\":{},\"runtime_uncertainty_tokens\":{},\"runtime_uncertainty_perplexity\":{},\"runtime_hot_kv_precision_bits\":{},\"runtime_cold_kv_precision_bits\":{},\"runtime_imported_kv_blocks\":{},\"runtime_weak_kv_imports_skipped\":{},\"runtime_budget_limited_kv_imports_skipped\":{},\"runtime_exported_kv_blocks\":{},\"runtime_kv_segments_included\":{},\"runtime_kv_segments_skipped\":{},\"runtime_kv_segments_rejected\":{},\"recursive_runtime_calls\":{},\"external_semantic_contexts\":{},\"self_evolving_memory_writeback_attempted_records\":{},\"self_evolving_memory_writeback_accepted_records\":{},\"self_evolving_memory_writeback_rejected_records\":{},\"self_evolving_memory_writeback_records_before\":{},\"self_evolving_memory_writeback_records_after\":{},\"self_evolving_memory_writeback_tool_reliability_after\":{},\"self_evolving_memory_writeback_tool_observations_after\":{},\"self_evolving_memory_writeback_maintenance_actions\":{},\"self_evolving_memory_writeback_merged_duplicate_episodes\":{},\"self_evolving_memory_writeback_write_allowed\":{},\"self_evolving_memory_writeback_durable_write_allowed\":{},\"self_evolving_memory_writeback_applied\":{},\"self_evolving_memory_writeback_applied_to_disk\":{},\"self_evolving_memory_writeback_snapshot_changes\":{},\"live_online_reward_feedbacks\":{},\"live_online_reward_reinforcements\":{},\"live_online_reward_penalties\":{},\"live_memory_feedback_updates\":{},\"live_memory_feedback_reinforced\":{},\"live_memory_feedback_penalized\":{},\"live_memory_feedback_applied\":{},\"live_memory_feedback_removed\":{},\"live_memory_feedback_missing\":{},\"live_memory_feedback_strength_delta\":{:.6},\"live_memory_feedback_detail\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_diagnostic_chars\":{},\"business_contract_passed\":{},\"business_contract_failed\":{},\"business_contract_missing_signals\":{},\"business_contract_protocol_leaks\":{},\"business_contract_substitutions\":{},\"business_contract_evasive_denials\":{},\"business_contract_missing_handling_signals\":{},\"business_contract_raw_passed\":{},\"business_contract_raw_failed\":{},\"business_contract_response_normalized\":{},\"business_contract_sanitized\":{},\"business_contract_canonical_fallbacks\":{},\"pool_dispatch_items\":{},\"pool_dispatch_selected_roles\":{},\"pool_dispatch_forwarded\":{},\"pool_dispatch_clamped\":{},\"pool_dispatch_low_priority\":{},\"reflection_issues\":{},\"critical_reflection_issues\":{},\"revision_actions\":{},\"runtime_errors\":{},\"runtime_timeouts\":{},\"runtime_error_message_chars\":{}}}",
                experience.id,
                task_profile_json_label(experience.profile),
                experience.quality,
                experience.process_reward,
                experience.reward_action.as_str(),
                experience.runtime_layer_count,
                experience.runtime_global_layers,
                experience.runtime_local_window_layers,
                experience.runtime_convolutional_fusion_layers,
                experience.runtime_hidden_size,
                experience.runtime_local_window_tokens,
                option_f32_json(experience.runtime_forward_energy),
                option_f32_json(experience.runtime_kv_influence),
                experience.runtime_token_count,
                experience.runtime_uncertainty_token_count,
                option_f32_json(experience.runtime_uncertainty_perplexity),
                option_u8_json(experience.runtime_hot_kv_precision_bits),
                option_u8_json(experience.runtime_cold_kv_precision_bits),
                experience.runtime_imported_kv_blocks,
                experience.runtime_weak_kv_imports_skipped,
                experience.runtime_budget_limited_kv_imports_skipped,
                experience.runtime_exported_kv_blocks,
                experience.runtime_kv_segments_included,
                experience.runtime_kv_segments_skipped,
                experience.runtime_kv_segments_rejected,
                option_usize_json(experience.recursive_runtime_calls),
                experience.external_semantic_contexts,
                experience.self_evolving_memory_writeback_attempted_records,
                experience.self_evolving_memory_writeback_accepted_records,
                experience.self_evolving_memory_writeback_rejected_records(),
                experience.self_evolving_memory_writeback_records_before,
                experience.self_evolving_memory_writeback_records_after,
                experience.self_evolving_memory_writeback_tool_reliability_after,
                experience.self_evolving_memory_writeback_tool_observations_after,
                experience.self_evolving_memory_writeback_maintenance_actions,
                experience.self_evolving_memory_writeback_merged_duplicate_episodes,
                experience.self_evolving_memory_writeback_write_allowed,
                experience.self_evolving_memory_writeback_durable_write_allowed,
                experience.self_evolving_memory_writeback_applied,
                experience.self_evolving_memory_writeback_applied_to_disk,
                experience.self_evolving_memory_writeback_snapshot_changes,
                experience.live_online_reward_feedbacks,
                experience.live_online_reward_reinforcements,
                experience.live_online_reward_penalties,
                experience.live_memory_feedback_updates,
                experience.live_memory_feedback_reinforced,
                experience.live_memory_feedback_penalized,
                experience.live_memory_feedback_applied,
                experience.live_memory_feedback_removed,
                experience.live_memory_feedback_missing,
                experience.live_memory_feedback_strength_delta,
                experience.live_memory_feedback_detail,
                experience.rust_check_passed,
                experience.rust_check_failed,
                experience.rust_check_diagnostic_chars,
                experience.business_contract_passed,
                experience.business_contract_failed,
                experience.business_contract_missing_signals,
                experience.business_contract_protocol_leaks,
                experience.business_contract_substitutions,
                experience.business_contract_evasive_denials,
                experience.business_contract_missing_handling_signals,
                experience.business_contract_raw_passed,
                experience.business_contract_raw_failed,
                experience.business_contract_response_normalized,
                experience.business_contract_sanitized,
                experience.business_contract_canonical_fallbacks,
                experience.pool_dispatch_items,
                string_array_json(&experience.pool_dispatch_selected_roles),
                experience.pool_dispatch_forwarded,
                experience.pool_dispatch_clamped,
                experience.pool_dispatch_low_priority,
                experience.reflection_issues,
                experience.critical_reflection_issues,
                experience.revision_actions,
                experience.runtime_errors,
                experience.runtime_timeouts,
                experience.runtime_error_message_chars,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{experiences}]")
}

fn option_usize_json(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_u8_json(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_f32_json(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "null".to_owned())
}

fn task_profile_json_label(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn memory_summaries_json(memories: &[StateMemorySummary]) -> String {
    let memories = memories
        .iter()
        .map(|memory| {
            format!(
                "{{\"id\":{},\"vector_dimensions\":{},\"strength\":{:.6},\"hits\":{},\"failures\":{},\"last_score\":{:.6}}}",
                memory.id,
                memory.vector_dimensions,
                memory.strength,
                memory.hits,
                memory.failures,
                memory.last_score,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{memories}]")
}

fn memory_vector_dimensions_json(buckets: &[StateMemoryVectorDimensions]) -> String {
    let buckets = buckets
        .iter()
        .map(|bucket| {
            format!(
                "{{\"dimensions\":{},\"count\":{}}}",
                bucket.dimensions, bucket.count
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{buckets}]")
}

fn experience_hygiene_samples_json(findings: &[StateExperienceHygieneFinding]) -> String {
    let samples = findings
        .iter()
        .map(experience_hygiene_finding_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{samples}]")
}

fn experience_hygiene_finding_json(finding: &StateExperienceHygieneFinding) -> String {
    format!(
        "{{\"experience_id\":{},\"severity\":\"{}\",\"reason\":{},\"markers\":{}}}",
        finding.experience_id,
        finding.severity.as_str(),
        service_json_string(&finding.reason),
        string_array_json(&finding.markers)
    )
}

fn experience_index_samples_json(findings: &[StateExperienceIndexFinding]) -> String {
    let samples = findings
        .iter()
        .map(experience_index_finding_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{samples}]")
}

fn experience_index_finding_json(finding: &StateExperienceIndexFinding) -> String {
    format!(
        "{{\"experience_id\":{},\"reason\":{},\"compacted\":{},\"noise_penalty\":{:.6},\"duplicate_of\":{},\"prompt_chars\":{},\"lesson_chars\":{}}}",
        finding.experience_id,
        service_json_string(&finding.reason),
        finding.compacted,
        finding.noise_penalty,
        option_u64_json(finding.duplicate_of),
        finding.prompt_chars,
        finding.lesson_chars
    )
}

fn string_array_json(items: &[String]) -> String {
    let items = items
        .iter()
        .map(|item| service_json_string(item))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::{EvolutionLedger, RewardAction, StateExperienceHygieneFinding};

    #[test]
    fn state_sample_json_exposes_runtime_and_preview_fields() {
        let experience = StateExperienceSummary {
            id: 7,
            profile: TaskProfile::Coding,
            quality: 0.91,
            process_reward: 0.82,
            reward_action: RewardAction::Reinforce,
            runtime_model_id: Some("noiron-runtime-v2".to_owned()),
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_device_profile: Some("cpu".to_owned()),
            runtime_primary_lane: Some("cpu-primary".to_owned()),
            runtime_fallback_lane: Some("cpu-fallback".to_owned()),
            runtime_memory_mode: Some("hybrid-kv".to_owned()),
            runtime_layer_count: 24,
            runtime_global_layers: 4,
            runtime_local_window_layers: 18,
            runtime_convolutional_fusion_layers: 2,
            runtime_hidden_size: 4096,
            runtime_local_window_tokens: 2048,
            runtime_forward_energy: Some(1.25),
            runtime_kv_influence: Some(0.75),
            runtime_token_count: 512,
            runtime_uncertainty_token_count: 9,
            runtime_uncertainty_perplexity: Some(2.5),
            runtime_hot_kv_precision_bits: Some(8),
            runtime_cold_kv_precision_bits: Some(4),
            runtime_imported_kv_blocks: 3,
            runtime_weak_kv_imports_skipped: 1,
            runtime_budget_limited_kv_imports_skipped: 2,
            runtime_exported_kv_blocks: 4,
            runtime_kv_segments_included: 5,
            runtime_kv_segments_skipped: 6,
            runtime_kv_segments_rejected: 7,
            recursive_runtime_calls: Some(2),
            external_semantic_contexts: 3,
            self_evolving_memory_writeback_attempted_records: 4,
            self_evolving_memory_writeback_accepted_records: 3,
            self_evolving_memory_writeback_records_before: 5,
            self_evolving_memory_writeback_records_after: 9,
            self_evolving_memory_writeback_tool_reliability_after: 2,
            self_evolving_memory_writeback_tool_observations_after: 4,
            self_evolving_memory_writeback_maintenance_actions: 1,
            self_evolving_memory_writeback_merged_duplicate_episodes: 1,
            self_evolving_memory_writeback_write_allowed: 1,
            self_evolving_memory_writeback_durable_write_allowed: 1,
            self_evolving_memory_writeback_applied: 1,
            self_evolving_memory_writeback_applied_to_disk: 1,
            self_evolving_memory_writeback_snapshot_changes: 1,
            runtime_errors: 0,
            runtime_timeouts: 1,
            runtime_error_message_chars: 12,
            live_online_reward_feedbacks: 2,
            live_online_reward_reinforcements: 2,
            live_online_reward_penalties: 0,
            live_memory_feedback_updates: 3,
            live_memory_feedback_reinforced: 2,
            live_memory_feedback_penalized: 1,
            live_memory_feedback_applied: 2,
            live_memory_feedback_removed: 0,
            live_memory_feedback_missing: 1,
            live_memory_feedback_strength_delta: 0.6,
            live_memory_feedback_detail: true,
            rust_check_passed: 1,
            rust_check_failed: 0,
            rust_check_diagnostic_chars: 42,
            business_contract_passed: 1,
            business_contract_failed: 0,
            business_contract_missing_signals: 1,
            business_contract_protocol_leaks: 0,
            business_contract_substitutions: 0,
            business_contract_evasive_denials: 0,
            business_contract_missing_handling_signals: 1,
            business_contract_raw_passed: 1,
            business_contract_raw_failed: 0,
            business_contract_response_normalized: 1,
            business_contract_sanitized: 1,
            business_contract_canonical_fallbacks: 0,
            pool_dispatch_items: 2,
            pool_dispatch_selected_roles: vec!["coder".to_owned(), "reviewer".to_owned()],
            pool_dispatch_forwarded: 1,
            pool_dispatch_clamped: 0,
            pool_dispatch_low_priority: 1,
            reflection_issues: 1,
            critical_reflection_issues: 0,
            revision_actions: 1,
        };
        let memory = StateMemorySummary {
            id: 1,
            key: "kv:route".to_owned(),
            vector_dimensions: 16,
            strength: 0.5,
            hits: 2,
            failures: 0,
            last_score: 0.8,
        };
        let hygiene = StateExperienceHygieneFinding {
            experience_id: 7,
            severity: rust_norion::ExperienceHygieneSeverity::Watch,
            reason: "needs preview".to_owned(),
            markers: vec!["runtime".to_owned()],
            prompt_preview: "prompt".to_owned(),
            lesson_preview: "lesson".to_owned(),
        };
        let index = StateExperienceIndexFinding {
            experience_id: 7,
            reason: "duplicate".to_owned(),
            compacted: true,
            noise_penalty: 0.2,
            duplicate_of: Some(3),
            prompt_chars: 10,
            lesson_chars: 20,
            prompt_preview: "prompt index".to_owned(),
            lesson_preview: "lesson index".to_owned(),
        };

        let experiences_json = experience_summaries_json(&[experience]);
        let memories_json = memory_summaries_json(&[memory]);
        let hygiene_json = experience_hygiene_finding_json(&hygiene);
        let index_json = experience_index_finding_json(&index);

        assert!(!experiences_json.contains("noiron-runtime-v2"));
        assert!(!experiences_json.contains("portable-rust"));
        assert!(experiences_json.contains("\"runtime_hidden_size\":4096"));
        assert!(experiences_json.contains("\"runtime_uncertainty_perplexity\":2.500000"));
        assert!(
            experiences_json.contains("\"pool_dispatch_selected_roles\":[\"coder\",\"reviewer\"]")
        );
        assert!(experiences_json.contains("\"self_evolving_memory_writeback_records_before\":5"));
        assert!(experiences_json
            .contains("\"self_evolving_memory_writeback_tool_reliability_after\":2"));
        assert!(experiences_json
            .contains("\"self_evolving_memory_writeback_tool_observations_after\":4"));
        assert!(
            experiences_json.contains("\"self_evolving_memory_writeback_maintenance_actions\":1")
        );
        assert!(experiences_json
            .contains("\"self_evolving_memory_writeback_merged_duplicate_episodes\":1"));
        assert!(experiences_json.contains("\"business_contract_missing_signals\":1"));
        assert!(
            model_service_state_json(&StateInspectionReport::from_engine(
                &rust_norion::NoironEngine::new(),
                1
            ))
            .contains("\"evolution_replay_business_contract_raw_audits\":0")
        );
        let mut engine = rust_norion::NoironEngine::new();
        engine.evolution_ledger = EvolutionLedger {
            live_memory_reinforcements: 4,
            live_memory_penalties: 1,
            live_stored_memories: 2,
            live_stored_gist_memories: 3,
            live_stored_runtime_kv_memories: 1,
            ..EvolutionLedger::default()
        };
        let state_json = model_service_state_json(&StateInspectionReport::from_engine(&engine, 1));
        assert!(state_json.contains("\"evolution_live_memory_reinforcements\":4"));
        assert!(state_json.contains("\"evolution_live_memory_penalties\":1"));
        assert!(state_json.contains("\"evolution_live_stored_memories\":2"));
        assert!(state_json.contains("\"evolution_live_stored_gist_memories\":3"));
        assert!(state_json.contains("\"evolution_live_stored_runtime_kv_memories\":1"));
        assert!(!experiences_json.contains("keep useful KV"));
        assert!(!memories_json.contains("kv:route"));
        assert!(!hygiene_json.contains("prompt_preview"));
        assert!(!hygiene_json.contains("lesson_preview"));
        assert!(index_json.contains("\"duplicate_of\":3"));
        assert!(!index_json.contains("prompt_preview"));
        assert!(!index_json.contains("lesson_preview"));
    }
}
