use rust_norion::{StateInspectionGate, StateInspectionMatrixGate};

use crate::cli::args::Args;

impl Args {
    pub(crate) fn state_inspection_gate(&self) -> StateInspectionGate {
        StateInspectionGate {
            min_memories: self.inspect_min_memories,
            min_runtime_kv_memories: self.inspect_min_runtime_kv_memories,
            min_experiences: self.inspect_min_experiences,
            max_experience_hygiene_quarantine_candidates: self
                .inspect_max_experience_hygiene_quarantine_candidates,
            max_experience_repairable_legacy_metadata_lessons: self
                .inspect_max_experience_repairable_legacy_metadata_lessons,
            max_experience_repairable_index_records: self
                .inspect_max_experience_repairable_index_records,
            max_experience_repair_projected_legacy_metadata_lessons: self
                .inspect_max_experience_repair_projected_legacy_metadata_lessons,
            max_experience_repair_skipped_missing_clean_gist: self
                .inspect_max_experience_repair_skipped_missing_clean_gist,
            max_experience_index_overlong_records: self
                .inspect_max_experience_index_overlong_records,
            max_experience_index_overlong_without_clean_gist: self
                .inspect_max_experience_index_overlong_without_clean_gist,
            max_experience_index_record_chars: self.inspect_max_experience_index_record_chars,
            max_experience_index_noisy_records: self.inspect_max_experience_index_noisy_records,
            max_experience_index_noise_penalty: self
                .inspect_max_experience_index_noise_penalty
                .map(|value| value.max(0.0)),
            min_experience_index_quality_score: self
                .inspect_min_experience_index_quality_score
                .map(|value| value.clamp(0.0, 1.0)),
            require_experience_index_retrieval_ready: self
                .inspect_require_experience_index_retrieval_ready,
            min_runtime_model_experiences: self.inspect_min_runtime_model_experiences,
            min_runtime_adapter_experiences: self.inspect_min_runtime_adapter_experiences,
            max_runtime_adapter_selection_mismatches: self
                .inspect_max_runtime_adapter_selection_mismatches,
            min_runtime_forward_energy_experiences: self
                .inspect_min_runtime_forward_energy_experiences,
            min_runtime_kv_influence_experiences: self.inspect_min_runtime_kv_influence_experiences,
            min_runtime_tokens: self.inspect_min_runtime_tokens,
            min_runtime_uncertainty_experiences: self.inspect_min_runtime_uncertainty_experiences,
            min_runtime_uncertainty_tokens: self.inspect_min_runtime_uncertainty_tokens,
            min_runtime_architecture_experiences: self.inspect_min_runtime_architecture_experiences,
            min_runtime_kv_precision_experiences: self.inspect_min_runtime_kv_precision_experiences,
            max_runtime_kv_precision_mismatches: self.inspect_max_runtime_kv_precision_mismatches,
            max_runtime_errors: self.inspect_max_runtime_errors,
            max_runtime_timeouts: self.inspect_max_runtime_timeouts,
            max_runtime_error_message_chars: self.inspect_max_runtime_error_message_chars,
            min_runtime_device_execution_experiences: self
                .inspect_min_runtime_device_execution_experiences,
            min_runtime_layer_mode_experiences: self.inspect_min_runtime_layer_mode_experiences,
            min_runtime_all_layer_mode_experiences: self
                .inspect_min_runtime_all_layer_mode_experiences,
            min_runtime_global_layers: self.inspect_min_runtime_global_layers,
            min_runtime_local_window_layers: self.inspect_min_runtime_local_window_layers,
            min_runtime_convolutional_fusion_layers: self
                .inspect_min_runtime_convolutional_fusion_layers,
            min_runtime_kv_import_experiences: self.inspect_min_runtime_kv_import_experiences,
            min_runtime_imported_kv_blocks: self.inspect_min_runtime_imported_kv_blocks,
            min_self_evolving_memory_writeback_experiences: self
                .inspect_min_self_evolving_memory_writeback_experiences,
            min_self_evolving_memory_writeback_attempted_records: self
                .inspect_min_self_evolving_memory_writeback_attempted_records,
            min_self_evolving_memory_writeback_accepted_records: self
                .inspect_min_self_evolving_memory_writeback_accepted_records,
            max_self_evolving_memory_writeback_rejected_records: self
                .inspect_max_self_evolving_memory_writeback_rejected_records,
            min_self_evolving_memory_writeback_write_allowed: self
                .inspect_min_self_evolving_memory_writeback_write_allowed,
            min_self_evolving_memory_writeback_durable_write_allowed: self
                .inspect_min_self_evolving_memory_writeback_durable_write_allowed,
            min_self_evolving_memory_writeback_applied: self
                .inspect_min_self_evolving_memory_writeback_applied,
            min_self_evolving_memory_writeback_applied_to_disk: self
                .inspect_min_self_evolving_memory_writeback_applied_to_disk,
            min_self_evolving_memory_writeback_snapshot_changes: self
                .inspect_min_self_evolving_memory_writeback_snapshot_changes,
            min_runtime_kv_weak_import_skip_experiences: self
                .inspect_min_runtime_kv_weak_import_skip_experiences,
            min_weak_runtime_kv_imports_skipped: self.inspect_min_weak_runtime_kv_imports_skipped,
            min_runtime_kv_weak_import_pressure_experiences: self
                .inspect_min_runtime_kv_weak_import_pressure_experiences,
            min_runtime_kv_weak_import_pressure: self
                .inspect_min_runtime_kv_weak_import_pressure
                .map(|value| value.clamp(0.0, 1.0)),
            max_runtime_kv_weak_import_pressure: self
                .inspect_max_runtime_kv_weak_import_pressure
                .map(|value| value.clamp(0.0, 1.0)),
            min_runtime_kv_budget_import_skip_experiences: self
                .inspect_min_runtime_kv_budget_import_skip_experiences,
            min_budget_limited_runtime_kv_imports_skipped: self
                .inspect_min_budget_limited_runtime_kv_imports_skipped,
            min_runtime_kv_budget_pressure_experiences: self
                .inspect_min_runtime_kv_budget_pressure_experiences,
            min_runtime_kv_budget_pressure: self
                .inspect_min_runtime_kv_budget_pressure
                .map(|value| value.clamp(0.0, 1.0)),
            max_runtime_kv_budget_pressure: self
                .inspect_max_runtime_kv_budget_pressure
                .map(|value| value.clamp(0.0, 1.0)),
            min_runtime_kv_export_experiences: self.inspect_min_runtime_kv_export_experiences,
            min_runtime_kv_segment_experiences: self.inspect_min_runtime_kv_segment_experiences,
            min_runtime_kv_segments_included: self.inspect_min_runtime_kv_segments_included,
            max_runtime_kv_segments_skipped: self.inspect_max_runtime_kv_segments_skipped,
            max_runtime_kv_segments_rejected: self.inspect_max_runtime_kv_segments_rejected,
            min_runtime_kv_hold_experiences: self.inspect_min_runtime_kv_hold_experiences,
            min_runtime_kv_held_blocks: self.inspect_min_runtime_kv_held_blocks,
            min_fht_dke_budget_experiences: self.inspect_min_fht_dke_budget_experiences,
            min_fht_dke_enabled_experiences: self.inspect_min_fht_dke_enabled_experiences,
            min_fht_dke_routed_tokens: self.inspect_min_fht_dke_routed_tokens,
            max_fht_dke_token_split_invalid: self.inspect_max_fht_dke_token_split_invalid,
            min_fht_dke_attention_threshold: self
                .inspect_min_fht_dke_attention_threshold
                .map(|value| value.clamp(0.0, 1.0)),
            max_fht_dke_attention_threshold: self
                .inspect_max_fht_dke_attention_threshold
                .map(|value| value.clamp(0.0, 1.0)),
            min_fht_dke_route_pressure: self
                .inspect_min_fht_dke_route_pressure
                .map(|value| value.clamp(0.0, 1.0)),
            max_fht_dke_route_pressure: self
                .inspect_max_fht_dke_route_pressure
                .map(|value| value.clamp(0.0, 1.0)),
            min_process_reward_experiences: self.inspect_min_process_reward_experiences,
            min_process_reward_positive: self.inspect_min_process_reward_positive,
            min_process_reward_reinforce: self.inspect_min_process_reward_reinforce,
            min_process_reward_total: self
                .inspect_min_process_reward_total
                .map(|value| value.max(0.0)),
            max_pool_dispatch_clamped: self.inspect_max_pool_dispatch_clamped,
            max_pool_dispatch_low_priority: self.inspect_max_pool_dispatch_low_priority,
            min_external_semantic_context_experiences: self
                .inspect_min_external_semantic_context_experiences,
            min_external_semantic_contexts: self.inspect_min_external_semantic_contexts,
            min_reflection_issue_experiences: self.inspect_min_reflection_issue_experiences,
            min_critical_reflection_issue_experiences: self
                .inspect_min_critical_reflection_issue_experiences,
            min_revision_action_experiences: self.inspect_min_revision_action_experiences,
            min_live_memory_feedback_experiences: self.inspect_min_live_memory_feedback_experiences,
            min_live_memory_feedback_updates: self.inspect_min_live_memory_feedback_updates,
            min_live_memory_feedback_reinforced: self.inspect_min_live_memory_feedback_reinforced,
            min_live_memory_feedback_penalized: self.inspect_min_live_memory_feedback_penalized,
            min_live_memory_feedback_detail_experiences: self
                .inspect_min_live_memory_feedback_detail_experiences,
            min_live_memory_feedback_applied: self.inspect_min_live_memory_feedback_applied,
            max_live_memory_feedback_missing: self.inspect_max_live_memory_feedback_missing,
            min_live_memory_feedback_strength_delta: self
                .inspect_min_live_memory_feedback_strength_delta
                .map(|value| value.max(0.0)),
            min_rust_check_experiences: self.inspect_min_rust_check_experiences,
            min_rust_check_passed: self.inspect_min_rust_check_passed,
            max_rust_check_failed: self.inspect_max_rust_check_failed,
            min_rust_check_diagnostic_chars: self.inspect_min_rust_check_diagnostic_chars,
            min_business_contract_experiences: self.inspect_min_business_contract_experiences,
            min_business_contract_passed: self.inspect_min_business_contract_passed,
            max_business_contract_failed: self.inspect_max_business_contract_failed,
            max_business_contract_missing_signals: self
                .inspect_max_business_contract_missing_signals,
            max_business_contract_protocol_leaks: self.inspect_max_business_contract_protocol_leaks,
            max_business_contract_substitutions: self.inspect_max_business_contract_substitutions,
            max_business_contract_evasive_denials: self
                .inspect_max_business_contract_evasive_denials,
            max_business_contract_missing_handling_signals: self
                .inspect_max_business_contract_missing_handling_signals,
            min_router_observations: self.inspect_min_router_observations,
            min_evolution_live_inference_runs: self.inspect_min_evolution_live_inference_runs,
            min_evolution_live_router_threshold_mutations: self
                .inspect_min_evolution_live_router_threshold_mutations,
            min_evolution_live_hierarchy_weight_mutations: self
                .inspect_min_evolution_live_hierarchy_weight_mutations,
            min_evolution_live_router_threshold_delta: self
                .inspect_min_evolution_live_router_threshold_delta
                .map(|value| value.max(0.0)),
            min_evolution_live_hierarchy_weight_delta: self
                .inspect_min_evolution_live_hierarchy_weight_delta
                .map(|value| value.max(0.0)),
            min_evolution_live_online_reward_feedbacks: self
                .inspect_min_evolution_live_online_reward_feedbacks,
            min_evolution_live_online_reward_reinforcements: self
                .inspect_min_evolution_live_online_reward_reinforcements,
            min_evolution_live_online_reward_penalties: self
                .inspect_min_evolution_live_online_reward_penalties,
            min_evolution_live_online_reward_strength: self
                .inspect_min_evolution_live_online_reward_strength
                .map(|value| value.max(0.0)),
            min_evolution_live_online_reward_reinforcement_strength: self
                .inspect_min_evolution_live_online_reward_reinforcement_strength
                .map(|value| value.max(0.0)),
            min_evolution_live_online_reward_penalty_strength: self
                .inspect_min_evolution_live_online_reward_penalty_strength
                .map(|value| value.max(0.0)),
            min_evolution_live_memory_reinforcements: self
                .inspect_min_evolution_live_memory_reinforcements,
            min_evolution_live_memory_penalties: self.inspect_min_evolution_live_memory_penalties,
            min_evolution_live_stored_memories: self.inspect_min_evolution_live_stored_memories,
            min_evolution_live_stored_gist_memories: self
                .inspect_min_evolution_live_stored_gist_memories,
            min_evolution_live_stored_runtime_kv_memories: self
                .inspect_min_evolution_live_stored_runtime_kv_memories,
            min_evolution_live_memory_updates: self.inspect_min_evolution_live_memory_updates,
            min_evolution_live_stored_memory_updates: self
                .inspect_min_evolution_live_stored_memory_updates,
            min_evolution_live_reflection_issues: self.inspect_min_evolution_live_reflection_issues,
            min_evolution_live_critical_reflection_issues: self
                .inspect_min_evolution_live_critical_reflection_issues,
            min_evolution_live_revision_actions: self.inspect_min_evolution_live_revision_actions,
            min_evolution_replay_runs: self.inspect_min_evolution_replay_runs,
            min_evolution_replay_items: self.inspect_min_evolution_replay_items,
            min_evolution_router_threshold_mutations: self
                .inspect_min_evolution_router_threshold_mutations,
            min_evolution_hierarchy_weight_mutations: self
                .inspect_min_evolution_hierarchy_weight_mutations,
            min_evolution_router_threshold_delta: self
                .inspect_min_evolution_router_threshold_delta
                .map(|value| value.max(0.0)),
            min_evolution_hierarchy_weight_delta: self
                .inspect_min_evolution_hierarchy_weight_delta
                .map(|value| value.max(0.0)),
            min_evolution_memory_updates: self.inspect_min_evolution_memory_updates,
            min_evolution_external_feedbacks: self.inspect_min_evolution_external_feedbacks,
            min_evolution_external_feedback_reinforcements: self
                .inspect_min_evolution_external_feedback_reinforcements,
            min_evolution_external_feedback_penalties: self
                .inspect_min_evolution_external_feedback_penalties,
            min_evolution_external_feedback_memory_updates: self
                .inspect_min_evolution_external_feedback_memory_updates,
            min_evolution_external_feedback_strength_delta: self
                .inspect_min_evolution_external_feedback_strength_delta
                .map(|value| value.max(0.0)),
            max_evolution_external_feedback_missing: self
                .inspect_max_evolution_external_feedback_missing,
            min_evolution_replay_live_memory_feedback_updates: self
                .inspect_min_evolution_replay_live_memory_feedback_updates,
            min_evolution_replay_live_memory_feedback_reinforcements: self
                .inspect_min_evolution_replay_live_memory_feedback_reinforcements,
            min_evolution_replay_live_memory_feedback_penalties: self
                .inspect_min_evolution_replay_live_memory_feedback_penalties,
            min_evolution_replay_live_memory_feedback_detail_items: self
                .inspect_min_evolution_replay_live_memory_feedback_detail_items,
            min_evolution_replay_live_memory_feedback_applied: self
                .inspect_min_evolution_replay_live_memory_feedback_applied,
            max_evolution_replay_live_memory_feedback_missing: self
                .inspect_max_evolution_replay_live_memory_feedback_missing,
            min_evolution_replay_live_memory_feedback_strength_delta: self
                .inspect_min_evolution_replay_live_memory_feedback_strength_delta
                .map(|value| value.max(0.0)),
            min_evolution_replay_rust_check_items: self
                .inspect_min_evolution_replay_rust_check_items,
            min_evolution_replay_rust_check_passed: self
                .inspect_min_evolution_replay_rust_check_passed,
            max_evolution_replay_rust_check_failed: self
                .inspect_max_evolution_replay_rust_check_failed,
            min_evolution_replay_rust_check_live_memory_feedback_updates: self
                .inspect_min_evolution_replay_rust_check_live_memory_feedback_updates,
            min_evolution_replay_rust_check_live_memory_feedback_applied: self
                .inspect_min_evolution_replay_rust_check_live_memory_feedback_applied,
            min_evolution_replay_rust_check_live_memory_feedback_strength_delta: self
                .inspect_min_evolution_replay_rust_check_live_memory_feedback_strength_delta
                .map(|value| value.max(0.0)),
            min_evolution_replay_business_contract_items: self
                .inspect_min_evolution_replay_business_contract_items,
            min_evolution_replay_business_contract_passed: self
                .inspect_min_evolution_replay_business_contract_passed,
            max_evolution_replay_business_contract_failed: self
                .inspect_max_evolution_replay_business_contract_failed,
            min_evolution_replay_business_contract_raw_audits: self
                .inspect_min_evolution_replay_business_contract_raw_audits,
            min_evolution_replay_live_evolution_items: self
                .inspect_min_evolution_replay_live_evolution_items,
            min_evolution_replay_live_evolution_online_reward_feedbacks: self
                .inspect_min_evolution_replay_live_evolution_online_reward_feedbacks,
            min_evolution_replay_live_evolution_online_reward_reinforcements: self
                .inspect_min_evolution_replay_live_evolution_online_reward_reinforcements,
            min_evolution_replay_live_evolution_online_reward_penalties: self
                .inspect_min_evolution_replay_live_evolution_online_reward_penalties,
            min_evolution_replay_live_evolution_online_reward_strength: self
                .inspect_min_evolution_replay_live_evolution_online_reward_strength
                .map(|value| value.max(0.0)),
            min_evolution_replay_live_evolution_online_reward_reinforcement_strength: self
                .inspect_min_evolution_replay_live_evolution_online_reward_reinforcement_strength
                .map(|value| value.max(0.0)),
            min_evolution_replay_live_evolution_online_reward_penalty_strength: self
                .inspect_min_evolution_replay_live_evolution_online_reward_penalty_strength
                .map(|value| value.max(0.0)),
            min_evolution_replay_live_evolution_memory_updates: self
                .inspect_min_evolution_replay_live_evolution_memory_updates,
            min_evolution_replay_live_evolution_stored_memory_updates: self
                .inspect_min_evolution_replay_live_evolution_stored_memory_updates,
            min_evolution_replay_live_evolution_reflection_issues: self
                .inspect_min_evolution_replay_live_evolution_reflection_issues,
            min_evolution_replay_live_evolution_critical_reflection_issues: self
                .inspect_min_evolution_replay_live_evolution_critical_reflection_issues,
            min_evolution_replay_live_evolution_revision_actions: self
                .inspect_min_evolution_replay_live_evolution_revision_actions,
            min_evolution_recursive_replay_items: self.inspect_min_evolution_recursive_replay_items,
            min_evolution_recursive_runtime_calls: self
                .inspect_min_evolution_recursive_runtime_calls,
            max_evolution_drift_rollbacks: self.inspect_max_evolution_drift_rollbacks,
            max_evolution_rollback_router_threshold_delta: self
                .inspect_max_evolution_rollback_router_threshold_delta
                .map(|value| value.max(0.0)),
            max_evolution_rollback_hierarchy_weight_delta: self
                .inspect_max_evolution_rollback_hierarchy_weight_delta
                .map(|value| value.max(0.0)),
            require_runtime_kv_dimensions: self.inspect_require_runtime_kv_dimensions,
        }
    }

    pub(crate) fn state_inspection_matrix_gate(&self) -> StateInspectionMatrixGate {
        StateInspectionMatrixGate {
            min_runtime_kv_memory_device_profiles: self
                .inspect_min_runtime_kv_memory_device_profiles,
            min_runtime_model_device_profiles: self.inspect_min_runtime_model_device_profiles,
            min_runtime_adapter_device_profiles: self.inspect_min_runtime_adapter_device_profiles,
            max_runtime_adapter_selection_mismatches: self
                .inspect_max_runtime_adapter_selection_mismatches,
            min_runtime_forward_energy_device_profiles: self
                .inspect_min_runtime_forward_energy_device_profiles,
            min_runtime_kv_influence_device_profiles: self
                .inspect_min_runtime_kv_influence_device_profiles,
            min_runtime_uncertainty_device_profiles: self
                .inspect_min_runtime_uncertainty_device_profiles,
            min_runtime_uncertainty_token_device_profiles: self
                .inspect_min_runtime_uncertainty_token_device_profiles,
            min_runtime_kv_precision_device_profiles: self
                .inspect_min_runtime_kv_precision_device_profiles,
            max_runtime_kv_precision_mismatches: self.inspect_max_runtime_kv_precision_mismatches,
            min_runtime_device_execution_device_profiles: self
                .inspect_min_runtime_device_execution_device_profiles,
            min_runtime_layer_mode_device_profiles: self
                .inspect_min_runtime_layer_mode_device_profiles,
            min_runtime_all_layer_mode_device_profiles: self
                .inspect_min_runtime_all_layer_mode_device_profiles,
            min_runtime_kv_import_device_profiles: self
                .inspect_min_runtime_kv_import_device_profiles,
            min_runtime_kv_weak_import_skip_device_profiles: self
                .inspect_min_runtime_kv_weak_import_skip_device_profiles,
            min_runtime_kv_weak_import_pressure_device_profiles: self
                .inspect_min_runtime_kv_weak_import_pressure_device_profiles,
            min_runtime_kv_budget_import_skip_device_profiles: self
                .inspect_min_runtime_kv_budget_import_skip_device_profiles,
            min_runtime_kv_budget_pressure_device_profiles: self
                .inspect_min_runtime_kv_budget_pressure_device_profiles,
            min_runtime_kv_export_device_profiles: self
                .inspect_min_runtime_kv_export_device_profiles,
            min_runtime_kv_segment_device_profiles: self
                .inspect_min_runtime_kv_segment_device_profiles,
            min_runtime_kv_hold_device_profiles: self
                .inspect_min_runtime_kv_hold_device_profiles,
            min_reflection_issue_device_profiles: self.inspect_min_reflection_issue_device_profiles,
            min_critical_reflection_issue_device_profiles: self
                .inspect_min_critical_reflection_issue_device_profiles,
            min_revision_action_device_profiles: self.inspect_min_revision_action_device_profiles,
            min_live_memory_feedback_device_profiles: self
                .inspect_min_live_memory_feedback_device_profiles,
            min_evolution_live_inference_device_profiles: self
                .inspect_min_evolution_live_inference_device_profiles,
            min_evolution_live_router_threshold_mutation_device_profiles: self
                .inspect_min_evolution_live_router_threshold_mutation_device_profiles,
            min_evolution_live_hierarchy_weight_mutation_device_profiles: self
                .inspect_min_evolution_live_hierarchy_weight_mutation_device_profiles,
            min_evolution_live_online_reward_device_profiles: self
                .inspect_min_evolution_live_online_reward_device_profiles,
            min_evolution_live_online_reward_strength_device_profiles: self
                .inspect_min_evolution_live_online_reward_strength_device_profiles,
            min_evolution_live_memory_update_device_profiles: self
                .inspect_min_evolution_live_memory_update_device_profiles,
            min_evolution_live_stored_memory_update_device_profiles: self
                .inspect_min_evolution_live_stored_memory_update_device_profiles,
            min_evolution_live_reflection_issue_device_profiles: self
                .inspect_min_evolution_live_reflection_issue_device_profiles,
            min_evolution_live_critical_reflection_issue_device_profiles: self
                .inspect_min_evolution_live_critical_reflection_issue_device_profiles,
            min_evolution_live_revision_action_device_profiles: self
                .inspect_min_evolution_live_revision_action_device_profiles,
            min_evolution_replay_run_device_profiles: self
                .inspect_min_evolution_replay_run_device_profiles,
            min_evolution_replay_item_device_profiles: self
                .inspect_min_evolution_replay_item_device_profiles,
            min_evolution_router_threshold_mutation_device_profiles: self
                .inspect_min_evolution_router_threshold_mutation_device_profiles,
            min_evolution_hierarchy_weight_mutation_device_profiles: self
                .inspect_min_evolution_hierarchy_weight_mutation_device_profiles,
            min_evolution_memory_update_device_profiles: self
                .inspect_min_evolution_memory_update_device_profiles,
            min_evolution_replay_live_memory_feedback_device_profiles: self
                .inspect_min_evolution_replay_live_memory_feedback_device_profiles,
            min_evolution_replay_live_memory_feedback_detail_device_profiles: self
                .inspect_min_evolution_replay_live_memory_feedback_detail_device_profiles,
            min_evolution_replay_live_evolution_device_profiles: self
                .inspect_min_evolution_replay_live_evolution_device_profiles,
            min_evolution_replay_live_evolution_online_reward_device_profiles: self
                .inspect_min_evolution_replay_live_evolution_online_reward_device_profiles,
            min_evolution_replay_live_evolution_online_reward_strength_device_profiles: self
                .inspect_min_evolution_replay_live_evolution_online_reward_strength_device_profiles,
            min_evolution_replay_live_evolution_memory_update_device_profiles: self
                .inspect_min_evolution_replay_live_evolution_memory_update_device_profiles,
            min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles: self
                .inspect_min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles,
            min_evolution_replay_live_evolution_revision_action_device_profiles: self
                .inspect_min_evolution_replay_live_evolution_revision_action_device_profiles,
            min_evolution_recursive_replay_device_profiles: self
                .inspect_min_evolution_recursive_replay_device_profiles,
            min_evolution_recursive_runtime_call_device_profiles: self
                .inspect_min_evolution_recursive_runtime_call_device_profiles,
        }
    }
}
