mod inspect;
mod kv_quant;

use rust_norion::BenchmarkGate;

use super::Args;

impl Args {
    pub(crate) fn benchmark_gate(&self) -> BenchmarkGate {
        let mut gate = BenchmarkGate::default();

        if let Some(value) = self.benchmark_min_quality {
            gate.min_average_quality = value;
        }
        if let Some(value) = self.benchmark_min_reward {
            gate.min_average_reward = value;
        }
        if let Some(value) = self.benchmark_max_total_ms {
            gate.max_total_elapsed_ms = Some(value);
        }
        if let Some(value) = self.benchmark_max_recursive_chunks {
            gate.max_case_recursive_chunks = Some(value);
        }
        if let Some(value) = self.benchmark_min_recursive_cases {
            gate.min_recursive_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_recursive_runtime_calls {
            gate.min_recursive_runtime_calls = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_router_updates {
            gate.min_auto_replay_router_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_hierarchy_updates {
            gate.min_auto_replay_hierarchy_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_router_threshold_mutations {
            gate.min_auto_replay_router_threshold_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_hierarchy_weight_mutations {
            gate.min_auto_replay_hierarchy_weight_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_router_threshold_delta {
            gate.min_auto_replay_router_threshold_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_auto_replay_hierarchy_weight_delta {
            gate.min_auto_replay_hierarchy_weight_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_auto_replay_memory_updates {
            gate.min_auto_replay_memory_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_live_memory_feedback_updates {
            gate.min_live_memory_feedback_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_live_memory_feedback_updates {
            gate.min_auto_replay_live_memory_feedback_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_live_memory_feedback_detail_items {
            gate.min_auto_replay_live_memory_feedback_detail_items = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_live_memory_feedback_applied {
            gate.min_auto_replay_live_memory_feedback_applied = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_live_memory_feedback_strength_delta {
            gate.min_auto_replay_live_memory_feedback_strength_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_auto_replay_recursive_items {
            gate.min_auto_replay_recursive_items = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_recursive_call_pressure {
            gate.min_auto_replay_recursive_call_pressure = Some(value.clamp(0.0, 1.0));
        }
        if let Some(value) = self.benchmark_max_auto_replay_recursive_call_pressure {
            gate.max_auto_replay_recursive_call_pressure = Some(value.clamp(0.0, 1.0));
        }
        if let Some(value) = self.benchmark_min_evolution_live_inference_runs {
            gate.min_evolution_live_inference_runs = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_router_threshold_mutations {
            gate.min_evolution_live_router_threshold_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_hierarchy_weight_mutations {
            gate.min_evolution_live_hierarchy_weight_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_router_threshold_delta {
            gate.min_evolution_live_router_threshold_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_live_hierarchy_weight_delta {
            gate.min_evolution_live_hierarchy_weight_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_live_online_reward_feedbacks {
            gate.min_evolution_live_online_reward_feedbacks = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_online_reward_reinforcements {
            gate.min_evolution_live_online_reward_reinforcements = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_online_reward_penalties {
            gate.min_evolution_live_online_reward_penalties = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_online_reward_strength {
            gate.min_evolution_live_online_reward_strength = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_live_online_reward_reinforcement_strength
        {
            gate.min_evolution_live_online_reward_reinforcement_strength = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_live_online_reward_penalty_strength {
            gate.min_evolution_live_online_reward_penalty_strength = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_live_memory_updates {
            gate.min_evolution_live_memory_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_stored_memory_updates {
            gate.min_evolution_live_stored_memory_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_reflection_issues {
            gate.min_evolution_live_reflection_issues = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_critical_reflection_issues {
            gate.min_evolution_live_critical_reflection_issues = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_revision_actions {
            gate.min_evolution_live_revision_actions = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_inference_device_profiles {
            gate.min_evolution_live_inference_device_profiles = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_live_router_threshold_mutation_device_profiles
        {
            gate.min_evolution_live_router_threshold_mutation_device_profiles = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_live_hierarchy_weight_mutation_device_profiles
        {
            gate.min_evolution_live_hierarchy_weight_mutation_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_online_reward_device_profiles {
            gate.min_evolution_live_online_reward_device_profiles = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_live_online_reward_strength_device_profiles
        {
            gate.min_evolution_live_online_reward_strength_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_memory_update_device_profiles {
            gate.min_evolution_live_memory_update_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_stored_memory_update_device_profiles
        {
            gate.min_evolution_live_stored_memory_update_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_reflection_issue_device_profiles {
            gate.min_evolution_live_reflection_issue_device_profiles = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_live_critical_reflection_issue_device_profiles
        {
            gate.min_evolution_live_critical_reflection_issue_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_revision_action_device_profiles {
            gate.min_evolution_live_revision_action_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_runs {
            gate.min_evolution_replay_runs = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_items {
            gate.min_evolution_replay_items = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_router_threshold_mutations {
            gate.min_evolution_router_threshold_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_hierarchy_weight_mutations {
            gate.min_evolution_hierarchy_weight_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_router_threshold_delta {
            gate.min_evolution_router_threshold_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_hierarchy_weight_delta {
            gate.min_evolution_hierarchy_weight_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_memory_updates {
            gate.min_evolution_memory_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_live_memory_feedback_updates {
            gate.min_evolution_replay_live_memory_feedback_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_live_memory_feedback_detail_items {
            gate.min_evolution_replay_live_memory_feedback_detail_items = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_live_memory_feedback_applied {
            gate.min_evolution_replay_live_memory_feedback_applied = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_live_memory_feedback_strength_delta
        {
            gate.min_evolution_replay_live_memory_feedback_strength_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_replay_rust_check_items {
            gate.min_evolution_replay_rust_check_items = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_rust_check_passed {
            gate.min_evolution_replay_rust_check_passed = Some(value);
        }
        if let Some(value) = self.benchmark_max_evolution_replay_rust_check_failed {
            gate.max_evolution_replay_rust_check_failed = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_rust_check_live_memory_feedback_updates
        {
            gate.min_evolution_replay_rust_check_live_memory_feedback_updates = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_rust_check_live_memory_feedback_applied
        {
            gate.min_evolution_replay_rust_check_live_memory_feedback_applied = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_rust_check_live_memory_feedback_strength_delta
        {
            gate.min_evolution_replay_rust_check_live_memory_feedback_strength_delta =
                Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_replay_live_evolution_items {
            gate.min_evolution_replay_live_evolution_items = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_live_evolution_online_reward_feedbacks
        {
            gate.min_evolution_replay_live_evolution_online_reward_feedbacks = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_live_evolution_online_reward_reinforcements
        {
            gate.min_evolution_replay_live_evolution_online_reward_reinforcements = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_live_evolution_online_reward_penalties
        {
            gate.min_evolution_replay_live_evolution_online_reward_penalties = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_live_evolution_online_reward_strength
        {
            gate.min_evolution_replay_live_evolution_online_reward_strength = Some(value.max(0.0));
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_live_evolution_online_reward_reinforcement_strength
        {
            gate.min_evolution_replay_live_evolution_online_reward_reinforcement_strength =
                Some(value.max(0.0));
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_live_evolution_online_reward_penalty_strength
        {
            gate.min_evolution_replay_live_evolution_online_reward_penalty_strength =
                Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_replay_live_evolution_memory_updates {
            gate.min_evolution_replay_live_evolution_memory_updates = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_live_evolution_stored_memory_updates
        {
            gate.min_evolution_replay_live_evolution_stored_memory_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_live_evolution_reflection_issues {
            gate.min_evolution_replay_live_evolution_reflection_issues = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_live_evolution_critical_reflection_issues
        {
            gate.min_evolution_replay_live_evolution_critical_reflection_issues = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_live_evolution_revision_actions {
            gate.min_evolution_replay_live_evolution_revision_actions = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_live_evolution_device_profiles {
            gate.min_evolution_replay_live_evolution_device_profiles = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_live_evolution_online_reward_device_profiles
        {
            gate.min_evolution_replay_live_evolution_online_reward_device_profiles = Some(value);
        }
        if let Some(value) = self
            .benchmark_min_evolution_replay_live_evolution_online_reward_strength_device_profiles
        {
            gate.min_evolution_replay_live_evolution_online_reward_strength_device_profiles =
                Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_live_evolution_memory_update_device_profiles
        {
            gate.min_evolution_replay_live_evolution_memory_update_device_profiles = Some(value);
        }
        if let Some(value) = self
            .benchmark_min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles
        {
            gate.min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles =
                Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_replay_live_evolution_revision_action_device_profiles
        {
            gate.min_evolution_replay_live_evolution_revision_action_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_recursive_replay_items {
            gate.min_evolution_recursive_replay_items = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_recursive_runtime_calls {
            gate.min_evolution_recursive_runtime_calls = Some(value);
        }
        if let Some(value) = self.benchmark_max_evolution_drift_rollbacks {
            gate.max_evolution_drift_rollbacks = Some(value);
        }
        if let Some(value) = self.benchmark_max_evolution_rollback_router_threshold_delta {
            gate.max_evolution_rollback_router_threshold_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_max_evolution_rollback_hierarchy_weight_delta {
            gate.max_evolution_rollback_hierarchy_weight_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_sparse_skipped_cases {
            gate.min_sparse_skipped_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_sparse_skipped_tokens {
            gate.min_sparse_skipped_tokens = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_forward_cases {
            gate.min_runtime_forward_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_forward_energy_cases {
            gate.min_runtime_forward_energy_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_influence_cases {
            gate.min_runtime_kv_influence_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_architecture_cases {
            gate.min_runtime_architecture_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_architecture_device_profiles {
            gate.min_runtime_architecture_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_precision_cases {
            gate.min_runtime_kv_precision_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_layer_mode_cases {
            gate.min_runtime_layer_mode_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_all_layer_mode_cases {
            gate.min_runtime_all_layer_mode_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_global_layers {
            gate.min_runtime_global_layers = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_local_window_layers {
            gate.min_runtime_local_window_layers = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_convolutional_fusion_layers {
            gate.min_runtime_convolutional_fusion_layers = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_uncertainty_cases {
            gate.min_runtime_uncertainty_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_uncertainty_tokens {
            gate.min_runtime_uncertainty_tokens = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_uncertainty_device_profiles {
            gate.min_runtime_uncertainty_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_uncertainty_token_device_profiles {
            gate.min_runtime_uncertainty_token_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_import_cases {
            gate.min_runtime_kv_import_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_imported {
            gate.min_runtime_kv_imported = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_import_device_profiles {
            gate.min_runtime_kv_import_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_exported {
            gate.min_runtime_kv_exported = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_export_device_profiles {
            gate.min_runtime_kv_export_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_stored {
            gate.min_runtime_kv_stored = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_stored_device_profiles {
            gate.min_runtime_kv_stored_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_hold_cases {
            gate.min_runtime_kv_hold_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_held {
            gate.min_runtime_kv_held = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_hold_device_profiles {
            gate.min_runtime_kv_hold_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_contract_cases {
            gate.min_runtime_adapter_contract_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_kinds {
            gate.min_runtime_adapter_kinds = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_observations {
            gate.min_runtime_adapter_observations = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_best_score {
            gate.min_runtime_adapter_best_score = Some(value.clamp(0.0, 1.0));
        }
        if let Some(value) = self.benchmark_max_runtime_adapter_contract_violations {
            gate.max_runtime_adapter_contract_violations = Some(value);
        }
        if let Some(value) = self.benchmark_max_runtime_adapter_selection_mismatches {
            gate.max_runtime_adapter_selection_mismatches = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_embedding_cases {
            gate.min_runtime_embedding_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_embedding_device_profiles {
            gate.min_runtime_embedding_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_max_embedding_fallback_cases {
            gate.max_embedding_fallback_cases = Some(value);
        }
        if let Some(value) = self.benchmark_max_embedding_evidence_failures {
            gate.max_embedding_evidence_failures = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_device_execution_cases {
            gate.min_runtime_device_execution_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_device_execution_device_profiles {
            gate.min_runtime_device_execution_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_precision_device_profiles {
            gate.min_runtime_kv_precision_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_max_runtime_device_execution_violations {
            gate.max_runtime_device_execution_violations = Some(value);
        }
        if let Some(value) = self.benchmark_max_memory_governance_failures {
            gate.max_memory_governance_failures = Some(value);
        }
        if let Some(value) = self.benchmark_min_memory_governance_cases {
            gate.min_memory_governance_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_memory_governance_device_profiles {
            gate.min_memory_governance_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_memory_retention_activity_cases {
            gate.min_memory_retention_activity_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_memory_compaction_activity_cases {
            gate.min_memory_compaction_activity_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_reflection_issue_cases {
            gate.min_reflection_issue_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_reflection_issues {
            gate.min_reflection_issues = Some(value);
        }
        if let Some(value) = self.benchmark_min_critical_reflection_issue_cases {
            gate.min_critical_reflection_issue_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_critical_reflection_issues {
            gate.min_critical_reflection_issues = Some(value);
        }
        if let Some(value) = self.benchmark_min_revision_action_cases {
            gate.min_revision_action_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_revision_actions {
            gate.min_revision_actions = Some(value);
        }
        if let Some(value) = self.benchmark_min_reflection_issue_device_profiles {
            gate.min_reflection_issue_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_critical_reflection_issue_device_profiles {
            gate.min_critical_reflection_issue_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_revision_action_device_profiles {
            gate.min_revision_action_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_device_profiles {
            gate.min_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_recursive_device_profiles {
            gate.min_recursive_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_max_drift_blocks {
            gate.max_drift_blocks = Some(value);
        }
        if let Some(value) = self.benchmark_max_drift_rollbacks {
            gate.max_drift_rollbacks = Some(value);
        }

        gate
    }
}
