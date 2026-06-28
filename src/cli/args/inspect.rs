mod core;
mod device_profiles;
mod evolution;
mod reflection;
mod runtime;

pub(crate) struct InspectFlagParse<'a> {
    pub(crate) inspect_state: &'a mut bool,
    pub(crate) inspect_gate: &'a mut bool,
    pub(crate) benchmark_all_devices: &'a mut bool,
    pub(crate) inspect_limit: &'a mut usize,
    pub(crate) inspect_min_memories: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_memories: &'a mut Option<usize>,
    pub(crate) inspect_min_experiences: &'a mut Option<usize>,
    pub(crate) inspect_max_experience_hygiene_quarantine_candidates: &'a mut Option<usize>,
    pub(crate) inspect_max_experience_repairable_legacy_metadata_lessons: &'a mut Option<usize>,
    pub(crate) inspect_max_experience_repairable_index_records: &'a mut Option<usize>,
    pub(crate) inspect_max_experience_repair_projected_legacy_metadata_lessons:
        &'a mut Option<usize>,
    pub(crate) inspect_max_experience_repair_skipped_missing_clean_gist: &'a mut Option<usize>,
    pub(crate) inspect_max_experience_index_overlong_records: &'a mut Option<usize>,
    pub(crate) inspect_max_experience_index_overlong_without_clean_gist: &'a mut Option<usize>,
    pub(crate) inspect_max_experience_index_record_chars: &'a mut Option<usize>,
    pub(crate) inspect_max_experience_index_noisy_records: &'a mut Option<usize>,
    pub(crate) inspect_max_experience_index_noise_penalty: &'a mut Option<f32>,
    pub(crate) inspect_min_experience_index_quality_score: &'a mut Option<f32>,
    pub(crate) inspect_require_experience_index_retrieval_ready: &'a mut bool,
    pub(crate) inspect_min_runtime_model_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_adapter_experiences: &'a mut Option<usize>,
    pub(crate) inspect_max_runtime_adapter_selection_mismatches: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_forward_energy_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_influence_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_tokens: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_uncertainty_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_uncertainty_tokens: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_architecture_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_precision_experiences: &'a mut Option<usize>,
    pub(crate) inspect_max_runtime_kv_precision_mismatches: &'a mut Option<usize>,
    pub(crate) inspect_max_runtime_errors: &'a mut Option<usize>,
    pub(crate) inspect_max_runtime_timeouts: &'a mut Option<usize>,
    pub(crate) inspect_max_runtime_error_message_chars: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_device_execution_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_layer_mode_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_all_layer_mode_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_global_layers: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_local_window_layers: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_convolutional_fusion_layers: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_import_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_imported_kv_blocks: &'a mut Option<usize>,
    pub(crate) inspect_min_self_evolving_memory_writeback_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_self_evolving_memory_writeback_attempted_records: &'a mut Option<usize>,
    pub(crate) inspect_min_self_evolving_memory_writeback_accepted_records: &'a mut Option<usize>,
    pub(crate) inspect_max_self_evolving_memory_writeback_rejected_records: &'a mut Option<usize>,
    pub(crate) inspect_min_self_evolving_memory_writeback_write_allowed: &'a mut Option<usize>,
    pub(crate) inspect_min_self_evolving_memory_writeback_durable_write_allowed:
        &'a mut Option<usize>,
    pub(crate) inspect_min_self_evolving_memory_writeback_applied: &'a mut Option<usize>,
    pub(crate) inspect_min_self_evolving_memory_writeback_applied_to_disk: &'a mut Option<usize>,
    pub(crate) inspect_min_self_evolving_memory_writeback_snapshot_changes: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_weak_import_skip_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_weak_runtime_kv_imports_skipped: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_weak_import_pressure_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_weak_import_pressure: &'a mut Option<f32>,
    pub(crate) inspect_max_runtime_kv_weak_import_pressure: &'a mut Option<f32>,
    pub(crate) inspect_min_runtime_kv_budget_import_skip_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_budget_limited_runtime_kv_imports_skipped: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_budget_pressure_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_budget_pressure: &'a mut Option<f32>,
    pub(crate) inspect_max_runtime_kv_budget_pressure: &'a mut Option<f32>,
    pub(crate) inspect_min_runtime_kv_export_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_segment_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_segments_included: &'a mut Option<usize>,
    pub(crate) inspect_max_runtime_kv_segments_skipped: &'a mut Option<usize>,
    pub(crate) inspect_max_runtime_kv_segments_rejected: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_hold_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_held_blocks: &'a mut Option<usize>,
    pub(crate) inspect_min_fht_dke_budget_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_fht_dke_enabled_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_fht_dke_routed_tokens: &'a mut Option<usize>,
    pub(crate) inspect_max_fht_dke_token_split_invalid: &'a mut Option<usize>,
    pub(crate) inspect_min_fht_dke_attention_threshold: &'a mut Option<f32>,
    pub(crate) inspect_max_fht_dke_attention_threshold: &'a mut Option<f32>,
    pub(crate) inspect_min_fht_dke_route_pressure: &'a mut Option<f32>,
    pub(crate) inspect_max_fht_dke_route_pressure: &'a mut Option<f32>,
    pub(crate) inspect_min_runtime_kv_memory_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_model_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_adapter_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_forward_energy_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_influence_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_uncertainty_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_uncertainty_token_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_precision_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_device_execution_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_layer_mode_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_all_layer_mode_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_import_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_weak_import_skip_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_weak_import_pressure_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_budget_import_skip_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_budget_pressure_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_export_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_segment_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_runtime_kv_hold_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_process_reward_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_process_reward_positive: &'a mut Option<usize>,
    pub(crate) inspect_min_process_reward_reinforce: &'a mut Option<usize>,
    pub(crate) inspect_min_process_reward_total: &'a mut Option<f32>,
    pub(crate) inspect_max_pool_dispatch_clamped: &'a mut Option<usize>,
    pub(crate) inspect_max_pool_dispatch_low_priority: &'a mut Option<usize>,
    pub(crate) inspect_min_external_semantic_context_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_external_semantic_contexts: &'a mut Option<usize>,
    pub(crate) inspect_min_reflection_issue_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_critical_reflection_issue_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_revision_action_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_live_memory_feedback_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_live_memory_feedback_updates: &'a mut Option<usize>,
    pub(crate) inspect_min_live_memory_feedback_reinforced: &'a mut Option<usize>,
    pub(crate) inspect_min_live_memory_feedback_penalized: &'a mut Option<usize>,
    pub(crate) inspect_min_live_memory_feedback_detail_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_live_memory_feedback_applied: &'a mut Option<usize>,
    pub(crate) inspect_max_live_memory_feedback_missing: &'a mut Option<usize>,
    pub(crate) inspect_min_live_memory_feedback_strength_delta: &'a mut Option<f32>,
    pub(crate) inspect_min_rust_check_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_rust_check_passed: &'a mut Option<usize>,
    pub(crate) inspect_max_rust_check_failed: &'a mut Option<usize>,
    pub(crate) inspect_min_rust_check_diagnostic_chars: &'a mut Option<usize>,
    pub(crate) inspect_min_business_contract_experiences: &'a mut Option<usize>,
    pub(crate) inspect_min_business_contract_passed: &'a mut Option<usize>,
    pub(crate) inspect_max_business_contract_failed: &'a mut Option<usize>,
    pub(crate) inspect_max_business_contract_missing_signals: &'a mut Option<usize>,
    pub(crate) inspect_max_business_contract_protocol_leaks: &'a mut Option<usize>,
    pub(crate) inspect_max_business_contract_substitutions: &'a mut Option<usize>,
    pub(crate) inspect_max_business_contract_evasive_denials: &'a mut Option<usize>,
    pub(crate) inspect_max_business_contract_missing_handling_signals: &'a mut Option<usize>,
    pub(crate) inspect_min_reflection_issue_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_critical_reflection_issue_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_revision_action_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_live_memory_feedback_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_live_inference_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_live_router_threshold_mutation_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_live_hierarchy_weight_mutation_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_live_online_reward_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_live_online_reward_strength_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_live_memory_update_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_live_stored_memory_update_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_live_reflection_issue_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_live_critical_reflection_issue_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_live_revision_action_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_replay_run_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_replay_item_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_router_threshold_mutation_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_hierarchy_weight_mutation_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_memory_update_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_replay_live_memory_feedback_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_replay_live_memory_feedback_detail_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_replay_live_evolution_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_replay_live_evolution_online_reward_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_replay_live_evolution_online_reward_strength_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_replay_live_evolution_memory_update_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_replay_live_evolution_revision_action_device_profiles:
        &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_recursive_replay_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_evolution_recursive_runtime_call_device_profiles: &'a mut Option<usize>,
    pub(crate) inspect_min_router_observations: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_inference_runs: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_router_threshold_mutations: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_hierarchy_weight_mutations: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_router_threshold_delta: &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_live_hierarchy_weight_delta: &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_live_online_reward_feedbacks: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_online_reward_reinforcements: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_online_reward_penalties: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_online_reward_strength: &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_live_online_reward_reinforcement_strength: &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_live_online_reward_penalty_strength: &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_live_memory_reinforcements: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_memory_penalties: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_stored_memories: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_stored_gist_memories: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_stored_runtime_kv_memories: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_memory_updates: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_stored_memory_updates: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_reflection_issues: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_critical_reflection_issues: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_live_revision_actions: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_runs: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_items: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_router_threshold_mutations: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_hierarchy_weight_mutations: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_router_threshold_delta: &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_hierarchy_weight_delta: &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_memory_updates: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_external_feedbacks: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_external_feedback_reinforcements: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_external_feedback_penalties: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_external_feedback_memory_updates: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_external_feedback_strength_delta: &'a mut Option<f32>,
    pub(crate) inspect_max_evolution_external_feedback_missing: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_memory_feedback_updates: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_memory_feedback_reinforcements:
        &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_memory_feedback_penalties: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_memory_feedback_detail_items: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_memory_feedback_applied: &'a mut Option<u64>,
    pub(crate) inspect_max_evolution_replay_live_memory_feedback_missing: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_memory_feedback_strength_delta:
        &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_replay_rust_check_items: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_rust_check_passed: &'a mut Option<u64>,
    pub(crate) inspect_max_evolution_replay_rust_check_failed: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_rust_check_live_memory_feedback_updates:
        &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_rust_check_live_memory_feedback_applied:
        &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_rust_check_live_memory_feedback_strength_delta:
        &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_replay_business_contract_items: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_business_contract_passed: &'a mut Option<u64>,
    pub(crate) inspect_max_evolution_replay_business_contract_failed: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_business_contract_raw_audits: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_evolution_items: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_evolution_online_reward_feedbacks:
        &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_evolution_online_reward_reinforcements:
        &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_evolution_online_reward_penalties:
        &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_evolution_online_reward_strength:
        &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_replay_live_evolution_online_reward_reinforcement_strength:
        &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_replay_live_evolution_online_reward_penalty_strength:
        &'a mut Option<f32>,
    pub(crate) inspect_min_evolution_replay_live_evolution_memory_updates: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_evolution_stored_memory_updates:
        &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_evolution_reflection_issues: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_evolution_critical_reflection_issues:
        &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_replay_live_evolution_revision_actions: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_recursive_replay_items: &'a mut Option<u64>,
    pub(crate) inspect_min_evolution_recursive_runtime_calls: &'a mut Option<u64>,
    pub(crate) inspect_max_evolution_drift_rollbacks: &'a mut Option<u64>,
    pub(crate) inspect_max_evolution_rollback_router_threshold_delta: &'a mut Option<f32>,
    pub(crate) inspect_max_evolution_rollback_hierarchy_weight_delta: &'a mut Option<f32>,
    pub(crate) inspect_require_runtime_kv_dimensions: &'a mut bool,
}

impl InspectFlagParse<'_> {
    pub(crate) fn parse(&mut self, raw: &[String], index: usize) -> Option<usize> {
        if let Some(consumed) = core::parse(self, raw, index) {
            return Some(consumed);
        }
        if let Some(consumed) = runtime::parse(self, raw, index) {
            return Some(consumed);
        }
        if let Some(consumed) = reflection::parse(self, raw, index) {
            return Some(consumed);
        }
        if let Some(consumed) = device_profiles::parse(self, raw, index) {
            return Some(consumed);
        }
        evolution::parse(self, raw, index)
    }
}
