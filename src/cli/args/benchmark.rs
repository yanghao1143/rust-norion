use std::path::PathBuf;

mod auto_replay;
mod core;
mod evolution_device_profiles;
mod evolution_live;
mod evolution_replay;
mod genome;
mod misc;
mod reflection;
mod runtime;

pub(crate) struct BenchmarkFlagParse<'a> {
    pub(crate) benchmark_path: &'a mut Option<PathBuf>,
    pub(crate) benchmark_gate_enabled: &'a mut bool,
    pub(crate) benchmark_all_devices: &'a mut bool,
    pub(crate) benchmark_min_quality: &'a mut Option<f32>,
    pub(crate) benchmark_min_reward: &'a mut Option<f32>,
    pub(crate) benchmark_max_total_ms: &'a mut Option<u128>,
    pub(crate) benchmark_max_recursive_chunks: &'a mut Option<usize>,
    pub(crate) benchmark_min_recursive_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_recursive_runtime_calls: &'a mut Option<usize>,
    pub(crate) benchmark_min_auto_replay_router_updates: &'a mut Option<usize>,
    pub(crate) benchmark_min_auto_replay_hierarchy_updates: &'a mut Option<usize>,
    pub(crate) benchmark_min_auto_replay_router_threshold_mutations: &'a mut Option<usize>,
    pub(crate) benchmark_min_auto_replay_hierarchy_weight_mutations: &'a mut Option<usize>,
    pub(crate) benchmark_min_auto_replay_router_threshold_delta: &'a mut Option<f32>,
    pub(crate) benchmark_min_auto_replay_hierarchy_weight_delta: &'a mut Option<f32>,
    pub(crate) benchmark_min_auto_replay_memory_updates: &'a mut Option<usize>,
    pub(crate) benchmark_min_live_memory_feedback_updates: &'a mut Option<usize>,
    pub(crate) benchmark_min_auto_replay_live_memory_feedback_updates: &'a mut Option<usize>,
    pub(crate) benchmark_min_auto_replay_live_memory_feedback_detail_items: &'a mut Option<usize>,
    pub(crate) benchmark_min_auto_replay_live_memory_feedback_applied: &'a mut Option<usize>,
    pub(crate) benchmark_min_auto_replay_live_memory_feedback_strength_delta: &'a mut Option<f32>,
    pub(crate) benchmark_min_auto_replay_recursive_items: &'a mut Option<usize>,
    pub(crate) benchmark_min_auto_replay_recursive_call_pressure: &'a mut Option<f32>,
    pub(crate) benchmark_max_auto_replay_recursive_call_pressure: &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_live_inference_runs: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_live_router_threshold_mutations: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_live_hierarchy_weight_mutations: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_live_router_threshold_delta: &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_live_hierarchy_weight_delta: &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_live_online_reward_feedbacks: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_live_online_reward_reinforcements: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_live_online_reward_penalties: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_live_online_reward_strength: &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_live_online_reward_reinforcement_strength:
        &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_live_online_reward_penalty_strength: &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_live_memory_updates: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_live_stored_memory_updates: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_live_reflection_issues: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_live_critical_reflection_issues: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_live_revision_actions: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_live_inference_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_live_router_threshold_mutation_device_profiles:
        &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_live_hierarchy_weight_mutation_device_profiles:
        &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_live_online_reward_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_live_online_reward_strength_device_profiles:
        &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_live_memory_update_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_live_stored_memory_update_device_profiles:
        &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_live_reflection_issue_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_live_critical_reflection_issue_device_profiles:
        &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_live_revision_action_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_replay_runs: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_items: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_router_threshold_mutations: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_hierarchy_weight_mutations: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_router_threshold_delta: &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_hierarchy_weight_delta: &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_memory_updates: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_memory_feedback_updates: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_memory_feedback_detail_items:
        &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_memory_feedback_applied: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_memory_feedback_strength_delta:
        &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_replay_rust_check_items: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_rust_check_passed: &'a mut Option<u64>,
    pub(crate) benchmark_max_evolution_replay_rust_check_failed: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_rust_check_live_memory_feedback_updates:
        &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_rust_check_live_memory_feedback_applied:
        &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_rust_check_live_memory_feedback_strength_delta:
        &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_items: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_online_reward_feedbacks:
        &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_online_reward_reinforcements:
        &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_online_reward_penalties:
        &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_online_reward_strength:
        &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_online_reward_reinforcement_strength:
        &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_online_reward_penalty_strength:
        &'a mut Option<f32>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_memory_updates: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_stored_memory_updates:
        &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_reflection_issues: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_critical_reflection_issues:
        &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_revision_actions: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_online_reward_device_profiles:
        &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_online_reward_strength_device_profiles:
        &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_memory_update_device_profiles:
        &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles:
        &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_replay_live_evolution_revision_action_device_profiles:
        &'a mut Option<usize>,
    pub(crate) benchmark_min_evolution_recursive_replay_items: &'a mut Option<u64>,
    pub(crate) benchmark_min_evolution_recursive_runtime_calls: &'a mut Option<u64>,
    pub(crate) benchmark_max_evolution_drift_rollbacks: &'a mut Option<u64>,
    pub(crate) benchmark_max_evolution_rollback_router_threshold_delta: &'a mut Option<f32>,
    pub(crate) benchmark_max_evolution_rollback_hierarchy_weight_delta: &'a mut Option<f32>,
    pub(crate) benchmark_min_sparse_skipped_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_sparse_skipped_tokens: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_forward_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_forward_energy_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_influence_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_architecture_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_architecture_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_precision_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_layer_mode_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_all_layer_mode_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_global_layers: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_local_window_layers: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_convolutional_fusion_layers: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_uncertainty_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_uncertainty_tokens: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_uncertainty_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_uncertainty_token_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_import_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_weak_import_skip_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_weak_runtime_kv_imports_skipped: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_weak_import_skip_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_budget_import_skip_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_budget_limited_runtime_kv_imports_skipped: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_budget_import_skip_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_segment_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_segments_included: &'a mut Option<usize>,
    pub(crate) benchmark_max_runtime_kv_segments_rejected: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_segment_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_imported: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_import_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_exported: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_export_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_stored: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_stored_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_hold_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_held: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_hold_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_adapter_contract_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_adapter_kinds: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_adapter_cache_modes: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_adapter_stream_trace_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_adapter_stream_gate_summary_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_adapter_observations: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_adapter_current_signals: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_adapter_best_score: &'a mut Option<f32>,
    pub(crate) benchmark_max_runtime_adapter_contract_violations: &'a mut Option<usize>,
    pub(crate) benchmark_max_runtime_adapter_selection_mismatches: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_embedding_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_embedding_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_max_embedding_fallback_cases: &'a mut Option<usize>,
    pub(crate) benchmark_max_embedding_evidence_failures: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_device_execution_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_device_execution_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_runtime_kv_precision_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_max_runtime_device_execution_violations: &'a mut Option<usize>,
    pub(crate) benchmark_max_memory_governance_failures: &'a mut Option<usize>,
    pub(crate) benchmark_min_reasoning_genome_expression_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_reasoning_genome_expression_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_reasoning_genome_splice_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_reasoning_genome_splice_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_gene_scissors_proposal_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_gene_scissors_proposal_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_reasoning_genome_repair_payloads: &'a mut Option<usize>,
    pub(crate) benchmark_min_reasoning_genome_regeneration_payloads: &'a mut Option<usize>,
    pub(crate) benchmark_min_mutation_repair_fixtures: &'a mut Option<usize>,
    pub(crate) benchmark_min_mutation_repair_fixture_kinds: &'a mut Option<usize>,
    pub(crate) benchmark_min_mutation_repair_candidates: &'a mut Option<usize>,
    pub(crate) benchmark_min_mutation_repair_review_packets: &'a mut Option<usize>,
    pub(crate) benchmark_min_malignant_gene_recovery_drills: &'a mut Option<usize>,
    pub(crate) benchmark_min_malignant_gene_quarantines: &'a mut Option<usize>,
    pub(crate) benchmark_min_malignant_gene_cut_candidates: &'a mut Option<usize>,
    pub(crate) benchmark_min_malignant_gene_regeneration_candidates: &'a mut Option<usize>,
    pub(crate) benchmark_min_dna_evolution_reports: &'a mut Option<usize>,
    pub(crate) benchmark_min_dna_evolution_candidates: &'a mut Option<usize>,
    pub(crate) benchmark_min_dna_evolution_candidate_previews: &'a mut Option<usize>,
    pub(crate) benchmark_max_dna_evolution_activation_eligible: &'a mut Option<usize>,
    pub(crate) benchmark_min_dna_evolution_transaction_replays: &'a mut Option<usize>,
    pub(crate) benchmark_min_dna_evolution_replay_passed: &'a mut Option<usize>,
    pub(crate) benchmark_min_dna_evolution_validation_passed: &'a mut Option<usize>,
    pub(crate) benchmark_min_dna_evolution_writer_gate_reports: &'a mut Option<usize>,
    pub(crate) benchmark_min_dna_evolution_writer_gate_holds: &'a mut Option<usize>,
    pub(crate) benchmark_min_dna_evolution_writer_gate_explicit_apply_required:
        &'a mut Option<usize>,
    pub(crate) benchmark_max_dna_evolution_writer_gate_ready: &'a mut Option<usize>,
    pub(crate) benchmark_max_dna_evolution_writer_gate_durable_write_allowed: &'a mut Option<usize>,
    pub(crate) benchmark_min_memory_governance_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_memory_governance_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_memory_retention_activity_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_memory_compaction_activity_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_reflection_issue_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_reflection_issues: &'a mut Option<usize>,
    pub(crate) benchmark_min_critical_reflection_issue_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_critical_reflection_issues: &'a mut Option<usize>,
    pub(crate) benchmark_min_revision_action_cases: &'a mut Option<usize>,
    pub(crate) benchmark_min_revision_actions: &'a mut Option<usize>,
    pub(crate) benchmark_min_reflection_issue_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_critical_reflection_issue_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_revision_action_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_min_recursive_device_profiles: &'a mut Option<usize>,
    pub(crate) benchmark_max_drift_blocks: &'a mut Option<usize>,
    pub(crate) benchmark_max_drift_rollbacks: &'a mut Option<usize>,
    pub(crate) benchmark_roundtrip: &'a mut bool,
}

impl BenchmarkFlagParse<'_> {
    pub(crate) fn parse(&mut self, raw: &[String], index: usize) -> Option<usize> {
        if let Some(consumed) = core::parse(self, raw, index) {
            return Some(consumed);
        }
        if let Some(consumed) = auto_replay::parse(self, raw, index) {
            return Some(consumed);
        }
        if let Some(consumed) = evolution_live::parse(self, raw, index) {
            return Some(consumed);
        }
        if let Some(consumed) = evolution_device_profiles::parse(self, raw, index) {
            return Some(consumed);
        }
        if let Some(consumed) = evolution_replay::parse(self, raw, index) {
            return Some(consumed);
        }
        if let Some(consumed) = runtime::parse(self, raw, index) {
            return Some(consumed);
        }
        if let Some(consumed) = genome::parse(self, raw, index) {
            return Some(consumed);
        }
        if let Some(consumed) = reflection::parse(self, raw, index) {
            return Some(consumed);
        }
        misc::parse(self, raw, index)
    }
}
