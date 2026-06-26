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
        if let Some(value) = self.benchmark_min_auto_replay_runtime_kv_budget_pressure_items {
            gate.min_auto_replay_runtime_kv_budget_pressure_items = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_runtime_kv_budget_pressure {
            gate.min_auto_replay_runtime_kv_budget_pressure = Some(value.clamp(0.0, 1.0));
        }
        if let Some(value) = self.benchmark_max_auto_replay_runtime_kv_budget_pressure {
            gate.max_auto_replay_runtime_kv_budget_pressure = Some(value.clamp(0.0, 1.0));
        }
        if let Some(value) = self.benchmark_min_auto_replay_runtime_kv_weak_import_pressure_items {
            gate.min_auto_replay_runtime_kv_weak_import_pressure_items = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_runtime_kv_weak_import_pressure {
            gate.min_auto_replay_runtime_kv_weak_import_pressure = Some(value.clamp(0.0, 1.0));
        }
        if let Some(value) = self.benchmark_max_auto_replay_runtime_kv_weak_import_pressure {
            gate.max_auto_replay_runtime_kv_weak_import_pressure = Some(value.clamp(0.0, 1.0));
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
        if let Some(value) = self.benchmark_min_runtime_kv_weak_import_skip_cases {
            gate.min_runtime_kv_weak_import_skip_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_weak_runtime_kv_imports_skipped {
            gate.min_weak_runtime_kv_imports_skipped = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_weak_import_skip_device_profiles {
            gate.min_runtime_kv_weak_import_skip_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_budget_import_skip_cases {
            gate.min_runtime_kv_budget_import_skip_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_budget_limited_runtime_kv_imports_skipped {
            gate.min_budget_limited_runtime_kv_imports_skipped = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_budget_import_skip_device_profiles {
            gate.min_runtime_kv_budget_import_skip_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_budget_pressure_cases {
            gate.min_runtime_kv_budget_pressure_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_budget_pressure_device_profiles {
            gate.min_runtime_kv_budget_pressure_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_segment_cases {
            gate.min_runtime_kv_segment_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_segments_included {
            gate.min_runtime_kv_segments_included = Some(value);
        }
        if let Some(value) = self.benchmark_max_runtime_kv_segments_rejected {
            gate.max_runtime_kv_segments_rejected = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_segment_device_profiles {
            gate.min_runtime_kv_segment_device_profiles = Some(value);
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
        if let Some(value) = self.benchmark_min_runtime_adapter_cache_modes {
            gate.min_runtime_adapter_cache_modes = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_stream_trace_cases {
            gate.min_runtime_adapter_stream_trace_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_stream_gate_summary_cases {
            gate.min_runtime_adapter_stream_gate_summary_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_stream_write_gate_cases {
            gate.min_runtime_adapter_stream_write_gate_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_observations {
            gate.min_runtime_adapter_observations = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_current_signals {
            gate.min_runtime_adapter_current_signals = Some(value);
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
        if let Some(value) = self.benchmark_min_reasoning_genome_expression_cases {
            gate.min_reasoning_genome_expression_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_reasoning_genome_expression_device_profiles {
            gate.min_reasoning_genome_expression_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_reasoning_genome_splice_cases {
            gate.min_reasoning_genome_splice_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_reasoning_genome_splice_device_profiles {
            gate.min_reasoning_genome_splice_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_gene_scissors_proposal_cases {
            gate.min_gene_scissors_proposal_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_gene_scissors_proposal_device_profiles {
            gate.min_gene_scissors_proposal_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_reasoning_genome_repair_payloads {
            gate.min_reasoning_genome_repair_payloads = Some(value);
        }
        if let Some(value) = self.benchmark_min_reasoning_genome_regeneration_payloads {
            gate.min_reasoning_genome_regeneration_payloads = Some(value);
        }
        if let Some(value) = self.benchmark_min_mutation_repair_fixtures {
            gate.min_mutation_repair_fixtures = Some(value);
        }
        if let Some(value) = self.benchmark_min_mutation_repair_fixture_kinds {
            gate.min_mutation_repair_fixture_kinds = Some(value);
        }
        if let Some(value) = self.benchmark_min_mutation_repair_candidates {
            gate.min_mutation_repair_candidates = Some(value);
        }
        if let Some(value) = self.benchmark_min_mutation_repair_review_packets {
            gate.min_mutation_repair_review_packets = Some(value);
        }
        if let Some(value) = self.benchmark_min_malignant_gene_recovery_drills {
            gate.min_malignant_gene_recovery_drills = Some(value);
        }
        if let Some(value) = self.benchmark_min_malignant_gene_quarantines {
            gate.min_malignant_gene_quarantines = Some(value);
        }
        if let Some(value) = self.benchmark_min_malignant_gene_cut_candidates {
            gate.min_malignant_gene_cut_candidates = Some(value);
        }
        if let Some(value) = self.benchmark_min_malignant_gene_regeneration_candidates {
            gate.min_malignant_gene_regeneration_candidates = Some(value);
        }
        if let Some(value) = self.benchmark_min_dna_evolution_reports {
            gate.min_dna_evolution_reports = Some(value);
        }
        if let Some(value) = self.benchmark_min_dna_evolution_candidates {
            gate.min_dna_evolution_candidates = Some(value);
        }
        if let Some(value) = self.benchmark_min_dna_evolution_candidate_previews {
            gate.min_dna_evolution_candidate_previews = Some(value);
        }
        if let Some(value) = self.benchmark_max_dna_evolution_activation_eligible {
            gate.max_dna_evolution_activation_eligible = Some(value);
        }
        if let Some(value) = self.benchmark_min_dna_evolution_transaction_replays {
            gate.min_dna_evolution_transaction_replays = Some(value);
        }
        if let Some(value) = self.benchmark_min_dna_evolution_replay_passed {
            gate.min_dna_evolution_replay_passed = Some(value);
        }
        if let Some(value) = self.benchmark_min_dna_evolution_validation_passed {
            gate.min_dna_evolution_validation_passed = Some(value);
        }
        if let Some(value) = self.benchmark_min_dna_evolution_writer_gate_reports {
            gate.min_dna_evolution_writer_gate_reports = Some(value);
        }
        if let Some(value) = self.benchmark_min_dna_evolution_writer_gate_holds {
            gate.min_dna_evolution_writer_gate_holds = Some(value);
        }
        if let Some(value) = self.benchmark_min_dna_evolution_writer_gate_explicit_apply_required {
            gate.min_dna_evolution_writer_gate_explicit_apply_required = Some(value);
        }
        if let Some(value) = self.benchmark_max_dna_evolution_writer_gate_ready {
            gate.max_dna_evolution_writer_gate_ready = Some(value);
        }
        if let Some(value) = self.benchmark_max_dna_evolution_writer_gate_durable_write_allowed {
            gate.max_dna_evolution_writer_gate_durable_write_allowed = Some(value);
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
