use rust_norion::{
    StateExperienceHygieneFinding, StateExperienceIndexFinding, StateExperienceSummary,
    StateInspectionGateReport, StateInspectionReport, StateMemorySummary,
    StateMemoryVectorDimensions, TaskProfile, TraceSchemaGateReport,
};

use super::super::json::{
    option_f32_service_json, option_str_service_json, option_usize_service_json,
    service_json_string, service_json_string_array,
};
use super::gates::{option_state_gate_service_json, option_trace_gate_service_json};

pub(crate) fn model_service_state_response_json(
    request_id: usize,
    report: &StateInspectionReport,
    state_gate_report: Option<&StateInspectionGateReport>,
    trace_gate_report: Option<&TraceSchemaGateReport>,
) -> String {
    format!(
        "{{\"ok\":true,\"request_id\":{},\"state\":{},\"state_gate\":{},\"trace_gate\":{}}}",
        request_id,
        model_service_state_json(report),
        option_state_gate_service_json(state_gate_report),
        option_trace_gate_service_json(trace_gate_report)
    )
}

pub(super) fn model_service_state_json(report: &StateInspectionReport) -> String {
    let mut body = format!(
        "{{\"summary\":{},\"memories\":{},\"runtime_kv_memories\":{},\"experiences\":{},\"experience_hygiene_findings\":{},\"experience_hygiene_watch\":{},\"experience_hygiene_quarantine_candidates\":{},\"experience_hygiene_legacy_metadata_lessons\":{},\"experience_hygiene_legacy_metadata_without_clean_gist\":{},\"experience_repairable_legacy_metadata_lessons\":{},\"experience_repairable_index_records\":{},\"experience_repair_projected_findings\":{},\"experience_repair_projected_watch\":{},\"experience_repair_projected_quarantine_candidates\":{},\"experience_repair_projected_legacy_metadata_lessons\":{},\"experience_repair_projected_legacy_metadata_without_clean_gist\":{},\"experience_repair_skipped_quarantine_candidates\":{},\"experience_repair_skipped_missing_clean_gist\":{},\"experience_hygiene_clean\":{},\"experience_hygiene_samples\":{},\"experience_index_compacted_records\":{},\"experience_index_overlong_records\":{},\"experience_index_overlong_without_clean_gist\":{},\"experience_index_max_record_chars\":{},\"experience_index_noisy_records\":{},\"experience_index_duplicate_outputs\":{},\"experience_index_max_noise_penalty\":{:.6},\"experience_index_quality_score\":{:.6},\"experience_index_retrieval_ready\":{},\"experience_index_risk_level\":{},\"experience_index_samples\":{},\"runtime_model_experiences\":{},\"runtime_tokens\":{},\"runtime_architecture_experiences\":{},\"runtime_kv_precision_experiences\":{},\"runtime_device_execution_experiences\":{},\"runtime_error_experiences\":{},\"runtime_errors\":{},\"runtime_timeout_experiences\":{},\"runtime_timeouts\":{},\"runtime_error_message_chars\":{},\"rust_check_experiences\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_diagnostic_chars\":{},\"business_contract_experiences\":{},\"business_contract_passed\":{},\"business_contract_failed\":{},\"business_contract_required_signals\":{},\"business_contract_matched_signals\":{},\"business_contract_missing_signals\":{},\"business_contract_protocol_leaks\":{},\"business_contract_substitutions\":{},\"business_contract_evasive_denials\":{},\"business_contract_missing_handling_signals\":{},\"business_contract_raw_passed\":{},\"business_contract_raw_failed\":{},\"business_contract_response_normalized\":{},\"business_contract_sanitized\":{},\"business_contract_canonical_fallbacks\":{},\"pool_dispatch_experiences\":{},\"pool_dispatch_items\":{},\"pool_dispatch_forwarded\":{},\"pool_dispatch_clamped\":{},\"pool_dispatch_low_priority\":{},\"evolution_live_inference_runs\":{},\"evolution_replay_runs\":{},\"evolution_replay_items\":{},\"evolution_external_feedbacks\":{},\"evolution_external_feedback_memory_updates\":{},\"evolution_external_feedback_strength_delta\":{:.6},\"evolution_replay_rust_check_items\":{},\"evolution_replay_rust_check_passed\":{},\"evolution_replay_rust_check_failed\":{},\"evolution_replay_rust_check_live_memory_feedback_updates\":{},\"evolution_replay_rust_check_live_memory_feedback_applied\":{},\"evolution_replay_rust_check_live_memory_feedback_strength_delta\":{:.6},\"evolution_replay_business_contract_items\":{},\"evolution_replay_business_contract_passed\":{},\"evolution_replay_business_contract_failed\":{},\"evolution_replay_business_contract_raw_passed\":{},\"evolution_replay_business_contract_raw_failed\":{},\"evolution_replay_business_contract_response_normalized\":{},\"evolution_replay_business_contract_sanitized\":{},\"evolution_replay_business_contract_canonical_fallbacks\":{},\"router_threshold\":{:.6}",
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
        report.runtime_model_experience_count,
        report.runtime_token_count,
        report.runtime_architecture_experience_count,
        report.runtime_kv_precision_experience_count,
        report.runtime_device_execution_experience_count,
        report.runtime_error_experience_count,
        report.runtime_error_count,
        report.runtime_timeout_experience_count,
        report.runtime_timeout_count,
        report.runtime_error_message_chars,
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
            .replay_business_contract_response_normalized,
        report.evolution_ledger.replay_business_contract_sanitized,
        report
            .evolution_ledger
            .replay_business_contract_canonical_fallbacks,
        report.router_threshold
    );
    body.push_str(&runtime_kv_state_fields_json(report));
    body.push_str(&memory_vector_dimension_fields_json(report));
    body.push_str(&top_memory_state_fields_json(report));
    body.push_str(&top_experience_state_fields_json(report));
    body.push_str(&reflection_feedback_state_fields_json(report));
    body.push_str(&profile_tier_state_fields_json(report));
    body.push_str(&memory_policy_state_fields_json(report));
    body.push_str(&adaptive_loop_state_fields_json(report));
    body.push('}');
    body
}

fn top_experience_state_fields_json(report: &StateInspectionReport) -> String {
    format!(
        ",\"top_experiences\":{}",
        experience_summaries_json(&report.top_experiences)
    )
}

fn experience_summaries_json(summaries: &[StateExperienceSummary]) -> String {
    let items = summaries
        .iter()
        .map(experience_summary_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn experience_summary_json(summary: &StateExperienceSummary) -> String {
    format!(
        "{{\"id\":{},\"profile\":\"{}\",\"quality\":{:.6},\"process_reward\":{:.6},\"reward_action\":\"{}\",\"runtime_model\":{},\"runtime_adapter\":{},\"runtime_device\":{},\"runtime_primary_lane\":{},\"runtime_fallback_lane\":{},\"runtime_memory_mode\":{},\"runtime_layer_count\":{},\"runtime_global_layers\":{},\"runtime_local_window_layers\":{},\"runtime_convolutional_fusion_layers\":{},\"runtime_hidden_size\":{},\"runtime_local_window_tokens\":{},\"runtime_forward_energy\":{},\"runtime_kv_influence\":{},\"runtime_token_count\":{},\"runtime_uncertainty_token_count\":{},\"runtime_uncertainty_perplexity\":{},\"runtime_hot_kv_precision_bits\":{},\"runtime_cold_kv_precision_bits\":{},\"runtime_imported_kv_blocks\":{},\"runtime_weak_kv_imports_skipped\":{},\"runtime_budget_limited_kv_imports_skipped\":{},\"runtime_kv_budget_pressure\":{:.6},\"runtime_exported_kv_blocks\":{},\"runtime_kv_segments_included\":{},\"runtime_kv_segments_skipped\":{},\"runtime_kv_segments_rejected\":{},\"runtime_kv_segment_yield\":{},\"recursive_runtime_calls\":{},\"live_online_reward_feedbacks\":{},\"live_online_reward_reinforcements\":{},\"live_online_reward_penalties\":{},\"live_memory_feedback_updates\":{},\"live_memory_feedback_reinforced\":{},\"live_memory_feedback_penalized\":{},\"live_memory_feedback_applied\":{},\"live_memory_feedback_removed\":{},\"live_memory_feedback_missing\":{},\"live_memory_feedback_strength_delta\":{:.6},\"live_memory_feedback_detail\":{},\"runtime_errors\":{},\"runtime_timeouts\":{},\"runtime_error_message_chars\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_diagnostic_chars\":{},\"business_contract_passed\":{},\"business_contract_failed\":{},\"business_contract_missing_signals\":{},\"business_contract_protocol_leaks\":{},\"business_contract_substitutions\":{},\"business_contract_evasive_denials\":{},\"business_contract_missing_handling_signals\":{},\"business_contract_raw_passed\":{},\"business_contract_raw_failed\":{},\"business_contract_response_normalized\":{},\"business_contract_sanitized\":{},\"business_contract_canonical_fallbacks\":{},\"pool_dispatch_items\":{},\"pool_dispatch_selected_roles\":{},\"pool_dispatch_forwarded\":{},\"pool_dispatch_clamped\":{},\"pool_dispatch_low_priority\":{},\"reflection_issues\":{},\"critical_reflection_issues\":{},\"revision_actions\":{}}}",
        summary.id,
        task_profile_name(summary.profile),
        summary.quality,
        summary.process_reward,
        summary.reward_action.as_str(),
        option_str_service_json(summary.runtime_model_id.as_deref()),
        option_str_service_json(summary.runtime_selected_adapter.as_deref()),
        option_str_service_json(summary.runtime_device_profile.as_deref()),
        option_str_service_json(summary.runtime_primary_lane.as_deref()),
        option_str_service_json(summary.runtime_fallback_lane.as_deref()),
        option_str_service_json(summary.runtime_memory_mode.as_deref()),
        summary.runtime_layer_count,
        summary.runtime_global_layers,
        summary.runtime_local_window_layers,
        summary.runtime_convolutional_fusion_layers,
        summary.runtime_hidden_size,
        summary.runtime_local_window_tokens,
        option_f32_service_json(summary.runtime_forward_energy),
        option_f32_service_json(summary.runtime_kv_influence),
        summary.runtime_token_count,
        summary.runtime_uncertainty_token_count,
        option_f32_service_json(summary.runtime_uncertainty_perplexity),
        option_usize_service_json(summary.runtime_hot_kv_precision_bits.map(usize::from)),
        option_usize_service_json(summary.runtime_cold_kv_precision_bits.map(usize::from)),
        summary.runtime_imported_kv_blocks,
        summary.runtime_weak_kv_imports_skipped,
        summary.runtime_budget_limited_kv_imports_skipped,
        runtime_kv_budget_pressure(summary),
        summary.runtime_exported_kv_blocks,
        summary.runtime_kv_segments_included,
        summary.runtime_kv_segments_skipped,
        summary.runtime_kv_segments_rejected,
        option_f32_service_json(runtime_kv_segment_yield(summary)),
        option_usize_service_json(summary.recursive_runtime_calls),
        summary.live_online_reward_feedbacks,
        summary.live_online_reward_reinforcements,
        summary.live_online_reward_penalties,
        summary.live_memory_feedback_updates,
        summary.live_memory_feedback_reinforced,
        summary.live_memory_feedback_penalized,
        summary.live_memory_feedback_applied,
        summary.live_memory_feedback_removed,
        summary.live_memory_feedback_missing,
        summary.live_memory_feedback_strength_delta,
        summary.live_memory_feedback_detail,
        summary.runtime_errors,
        summary.runtime_timeouts,
        summary.runtime_error_message_chars,
        summary.rust_check_passed,
        summary.rust_check_failed,
        summary.rust_check_diagnostic_chars,
        summary.business_contract_passed,
        summary.business_contract_failed,
        summary.business_contract_missing_signals,
        summary.business_contract_protocol_leaks,
        summary.business_contract_substitutions,
        summary.business_contract_evasive_denials,
        summary.business_contract_missing_handling_signals,
        summary.business_contract_raw_passed,
        summary.business_contract_raw_failed,
        summary.business_contract_response_normalized,
        summary.business_contract_sanitized,
        summary.business_contract_canonical_fallbacks,
        summary.pool_dispatch_items,
        service_json_string_array(&summary.pool_dispatch_selected_roles),
        summary.pool_dispatch_forwarded,
        summary.pool_dispatch_clamped,
        summary.pool_dispatch_low_priority,
        summary.reflection_issues,
        summary.critical_reflection_issues,
        summary.revision_actions
    )
}

fn runtime_kv_budget_pressure(summary: &StateExperienceSummary) -> f32 {
    let skipped = summary.runtime_budget_limited_kv_imports_skipped;
    let total = summary.runtime_exported_kv_blocks.saturating_add(skipped);
    if total == 0 {
        return 0.0;
    }

    (skipped as f32 / total as f32).clamp(0.0, 1.0)
}

fn runtime_kv_segment_yield(summary: &StateExperienceSummary) -> Option<f32> {
    let total = summary
        .runtime_kv_segments_included
        .saturating_add(summary.runtime_kv_segments_skipped)
        .saturating_add(summary.runtime_kv_segments_rejected);
    if total == 0 {
        return None;
    }

    let total = total as f32;
    let included = summary.runtime_kv_segments_included as f32 / total;
    let skipped = summary.runtime_kv_segments_skipped as f32 / total;
    let rejected = summary.runtime_kv_segments_rejected as f32 / total;
    Some((included - skipped * 0.25 - rejected * 0.75).clamp(0.0, 1.0))
}

fn task_profile_name(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long",
    }
}

fn top_memory_state_fields_json(report: &StateInspectionReport) -> String {
    format!(
        ",\"top_memories\":{},\"top_runtime_kv_memories\":{}",
        memory_summaries_json(&report.top_memories),
        memory_summaries_json(&report.top_runtime_kv_memories)
    )
}

fn memory_summaries_json(summaries: &[StateMemorySummary]) -> String {
    let items = summaries
        .iter()
        .map(memory_summary_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn memory_summary_json(summary: &StateMemorySummary) -> String {
    format!(
        "{{\"id\":{},\"key\":{},\"vector_dimensions\":{},\"strength\":{:.6},\"hits\":{},\"failures\":{},\"last_score\":{:.6}}}",
        summary.id,
        service_json_string(&summary.key),
        summary.vector_dimensions,
        summary.strength,
        summary.hits,
        summary.failures,
        summary.last_score
    )
}

fn runtime_kv_state_fields_json(report: &StateInspectionReport) -> String {
    format!(
        ",\"runtime_adapter_experiences\":{},\"runtime_adapter_selection_mismatches\":{},\"runtime_forward_energy_experiences\":{},\"runtime_kv_influence_experiences\":{},\"runtime_uncertainty_experiences\":{},\"runtime_uncertainty_tokens\":{},\"runtime_kv_precision_mismatches\":{},\"runtime_layer_mode_experiences\":{},\"runtime_all_layer_mode_experiences\":{},\"runtime_global_layers\":{},\"runtime_local_window_layers\":{},\"runtime_convolutional_fusion_layers\":{},\"runtime_kv_import_experiences\":{},\"runtime_kv_weak_import_skip_experiences\":{},\"weak_runtime_kv_imports_skipped\":{},\"runtime_kv_weak_import_pressure_experiences\":{},\"runtime_kv_weak_import_pressure_avg\":{:.6},\"runtime_kv_weak_import_pressure_max\":{:.6},\"runtime_kv_budget_import_skip_experiences\":{},\"budget_limited_runtime_kv_imports_skipped\":{},\"runtime_kv_budget_pressure_experiences\":{},\"runtime_kv_budget_pressure_avg\":{:.6},\"runtime_kv_budget_pressure_max\":{:.6},\"runtime_kv_export_experiences\":{},\"runtime_kv_segment_experiences\":{},\"runtime_kv_segments_included\":{},\"runtime_kv_segments_skipped\":{},\"runtime_kv_segments_rejected\":{},\"runtime_kv_hold_experiences\":{},\"runtime_kv_held_blocks\":{}",
        report.runtime_adapter_experience_count,
        report.runtime_adapter_selection_mismatch_count,
        report.runtime_forward_energy_experience_count,
        report.runtime_kv_influence_experience_count,
        report.runtime_uncertainty_experience_count,
        report.runtime_uncertainty_token_count,
        report.runtime_kv_precision_mismatch_count,
        report.runtime_layer_mode_experience_count,
        report.runtime_all_layer_mode_experience_count,
        report.runtime_global_layers,
        report.runtime_local_window_layers,
        report.runtime_convolutional_fusion_layers,
        report.runtime_kv_import_experience_count,
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
        report.runtime_kv_export_experience_count,
        report.runtime_kv_segment_experience_count,
        report.runtime_kv_segments_included,
        report.runtime_kv_segments_skipped,
        report.runtime_kv_segments_rejected,
        report.runtime_kv_hold_experience_count,
        report.runtime_kv_held_blocks
    )
}

fn memory_vector_dimension_fields_json(report: &StateInspectionReport) -> String {
    format!(
        ",\"memory_vector_dimensions\":{},\"runtime_kv_vector_dimensions\":{}",
        memory_vector_dimensions_json(&report.memory_vector_dimensions),
        memory_vector_dimensions_json(&report.runtime_kv_vector_dimensions)
    )
}

fn memory_vector_dimensions_json(buckets: &[StateMemoryVectorDimensions]) -> String {
    let items = buckets
        .iter()
        .map(|bucket| {
            format!(
                "{{\"dimensions\":{},\"count\":{}}}",
                bucket.dimensions, bucket.count
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn reflection_feedback_state_fields_json(report: &StateInspectionReport) -> String {
    format!(
        ",\"reflection_issue_experiences\":{},\"critical_reflection_issue_experiences\":{},\"revision_action_experiences\":{},\"live_memory_feedback_experiences\":{},\"live_memory_feedback_updates\":{},\"live_memory_feedback_detail_experiences\":{},\"live_memory_feedback_applied\":{},\"live_memory_feedback_removed\":{},\"live_memory_feedback_missing\":{},\"live_memory_feedback_strength_delta\":{:.6}",
        report.reflection_issue_experience_count,
        report.critical_reflection_issue_experience_count,
        report.revision_action_experience_count,
        report.live_memory_feedback_experience_count,
        report.live_memory_feedback_update_count,
        report.live_memory_feedback_detail_experience_count,
        report.live_memory_feedback_applied_count,
        report.live_memory_feedback_removed_count,
        report.live_memory_feedback_missing_count,
        report.live_memory_feedback_strength_delta
    )
}

fn profile_tier_state_fields_json(report: &StateInspectionReport) -> String {
    format!(
        ",\"profile_observations_general\":{},\"profile_observations_coding\":{},\"profile_observations_writing\":{},\"profile_observations_long_document\":{},\"profile_hierarchy_observations_general\":{},\"profile_hierarchy_observations_coding\":{},\"profile_hierarchy_observations_writing\":{},\"profile_hierarchy_observations_long_document\":{},\"tier_hot_gpu\":{},\"tier_warm_ram\":{},\"tier_cold_disk\":{}",
        report.profile_observations.general,
        report.profile_observations.coding,
        report.profile_observations.writing,
        report.profile_observations.long_document,
        report.profile_hierarchy_observations.general,
        report.profile_hierarchy_observations.coding,
        report.profile_hierarchy_observations.writing,
        report.profile_hierarchy_observations.long_document,
        report.tier_counts.hot_gpu,
        report.tier_counts.warm_ram,
        report.tier_counts.cold_disk
    )
}

fn memory_policy_state_fields_json(report: &StateInspectionReport) -> String {
    format!(
        ",\"memory_retention_stale_after\":{},\"memory_retention_decay_rate\":{:.6},\"memory_retention_remove_below_strength\":{:.6},\"memory_retention_remove_after_failures\":{},\"memory_compaction_similarity_threshold\":{:.6},\"memory_compaction_max_candidates\":{},\"memory_compaction_max_merges\":{}",
        report.memory_retention_policy.stale_after,
        report.memory_retention_policy.decay_rate,
        report.memory_retention_policy.remove_below_strength,
        report.memory_retention_policy.remove_after_failures,
        report.memory_compaction_policy.similarity_threshold,
        report.memory_compaction_policy.max_candidates,
        report.memory_compaction_policy.max_merges
    )
}

fn adaptive_loop_state_fields_json(report: &StateInspectionReport) -> String {
    format!(
        ",\"router_observations\":{},\"profile_threshold_general\":{:.6},\"profile_threshold_coding\":{:.6},\"profile_threshold_writing\":{:.6},\"profile_threshold_long_document\":{:.6},\"hierarchy_global\":{:.6},\"hierarchy_local\":{:.6},\"hierarchy_convolution\":{:.6},\"profile_hierarchy_global_general\":{:.6},\"profile_hierarchy_local_general\":{:.6},\"profile_hierarchy_convolution_general\":{:.6},\"profile_hierarchy_global_coding\":{:.6},\"profile_hierarchy_local_coding\":{:.6},\"profile_hierarchy_convolution_coding\":{:.6},\"profile_hierarchy_global_writing\":{:.6},\"profile_hierarchy_local_writing\":{:.6},\"profile_hierarchy_convolution_writing\":{:.6},\"profile_hierarchy_global_long_document\":{:.6},\"profile_hierarchy_local_long_document\":{:.6},\"profile_hierarchy_convolution_long_document\":{:.6},\"evolution_live_router_threshold_mutations\":{},\"evolution_live_hierarchy_weight_mutations\":{},\"evolution_live_router_threshold_delta\":{:.6},\"evolution_live_hierarchy_weight_delta\":{:.6},\"evolution_live_reflection_issues\":{},\"evolution_live_critical_reflection_issues\":{},\"evolution_live_revision_actions\":{},\"evolution_router_threshold_mutations\":{},\"evolution_hierarchy_weight_mutations\":{},\"evolution_router_threshold_delta\":{:.6},\"evolution_hierarchy_weight_delta\":{:.6},\"evolution_replay_live_evolution_items\":{},\"evolution_replay_live_evolution_router_threshold_mutations\":{},\"evolution_replay_live_evolution_hierarchy_weight_mutations\":{},\"evolution_replay_live_evolution_router_threshold_delta\":{:.6},\"evolution_replay_live_evolution_hierarchy_weight_delta\":{:.6},\"evolution_drift_rollbacks\":{},\"evolution_rollback_router_threshold_delta\":{:.6},\"evolution_rollback_hierarchy_weight_delta\":{:.6},\"evolution_recursive_runtime_calls\":{}",
        report.router_observations,
        report.profile_thresholds.general,
        report.profile_thresholds.coding,
        report.profile_thresholds.writing,
        report.profile_thresholds.long_document,
        report.hierarchy.global,
        report.hierarchy.local,
        report.hierarchy.convolution,
        report.profile_hierarchy_weights.general.global,
        report.profile_hierarchy_weights.general.local,
        report.profile_hierarchy_weights.general.convolution,
        report.profile_hierarchy_weights.coding.global,
        report.profile_hierarchy_weights.coding.local,
        report.profile_hierarchy_weights.coding.convolution,
        report.profile_hierarchy_weights.writing.global,
        report.profile_hierarchy_weights.writing.local,
        report.profile_hierarchy_weights.writing.convolution,
        report.profile_hierarchy_weights.long_document.global,
        report.profile_hierarchy_weights.long_document.local,
        report.profile_hierarchy_weights.long_document.convolution,
        report.evolution_ledger.live_router_threshold_mutations,
        report.evolution_ledger.live_hierarchy_weight_mutations,
        report.evolution_ledger.live_router_threshold_delta,
        report.evolution_ledger.live_hierarchy_weight_delta,
        report.evolution_ledger.live_reflection_issues,
        report.evolution_ledger.live_critical_reflection_issues,
        report.evolution_ledger.live_revision_actions,
        report.evolution_ledger.router_threshold_mutations,
        report.evolution_ledger.hierarchy_weight_mutations,
        report.evolution_ledger.router_threshold_delta,
        report.evolution_ledger.hierarchy_weight_delta,
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
        report.evolution_ledger.drift_rollbacks,
        report.evolution_ledger.rollback_router_threshold_delta,
        report.evolution_ledger.rollback_hierarchy_weight_delta,
        report.evolution_ledger.recursive_runtime_calls
    )
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
        "{{\"experience_id\":{},\"severity\":\"{}\",\"reason\":{},\"markers\":{},\"prompt_preview\":{},\"lesson_preview\":{}}}",
        finding.experience_id,
        finding.severity.as_str(),
        service_json_string(&finding.reason),
        string_array_json(&finding.markers),
        service_json_string(&finding.prompt_preview),
        service_json_string(&finding.lesson_preview)
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
        "{{\"experience_id\":{},\"reason\":{},\"compacted\":{},\"noise_penalty\":{:.6},\"prompt_chars\":{},\"lesson_chars\":{},\"prompt_preview\":{},\"lesson_preview\":{}}}",
        finding.experience_id,
        service_json_string(&finding.reason),
        finding.compacted,
        finding.noise_penalty,
        finding.prompt_chars,
        finding.lesson_chars,
        service_json_string(&finding.prompt_preview),
        service_json_string(&finding.lesson_preview)
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
