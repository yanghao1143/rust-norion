use std::path::PathBuf;

use rust_norion::TaskProfile;

use crate::cli::help::print_help_and_exit;

use super::Args;
use super::benchmark::BenchmarkFlagParse;
use super::device::DeviceFlagParse;
use super::gemma::GemmaFlagParse;
use super::inspect::InspectFlagParse;
use super::runtime::RuntimeFlagParse;
use super::service::ServiceFlagParse;
use super::state::StateFlagParse;
use super::values::{parse_f32, parse_u64, parse_usize};

mod state;
use state::ParseState;

impl Args {
    pub(crate) fn parse(raw: Vec<String>) -> Self {
        let mut state = ParseState::new();
        let mut index = 0;

        while index < raw.len() {
            if let Some(consumed) = (GemmaFlagParse {
                business_smoke: &mut state.gemma_business_smoke,
                business_cycle_smoke: &mut state.gemma_business_cycle_smoke,
                business_regression_gate_path: &mut state.gemma_business_regression_gate_path,
                business_cycle_smoke_report_gate_path: &mut state
                    .gemma_business_cycle_smoke_report_gate_path,
                model_service_smoke: &mut state.gemma_model_service_smoke,
                smoke_check_only: &mut state.gemma_smoke_check_only,
                smoke_keep_runs: &mut state.gemma_smoke_keep_runs,
                runtime: &mut state.gemma_runtime,
                runtime_metadata: &mut state.runtime_metadata,
                runtime_layer_count: &mut state.runtime_layer_count,
                runtime_hidden_size: &mut state.runtime_hidden_size,
                runtime_attention_heads: &mut state.runtime_attention_heads,
                runtime_kv_heads: &mut state.runtime_kv_heads,
                runtime_local_window_tokens: &mut state.runtime_local_window_tokens,
            })
            .parse(&raw, index)
            {
                index += consumed;
                continue;
            }

            if let Some(consumed) = (RuntimeFlagParse {
                local_runtime: &mut state.local_runtime,
                production_runtime: &mut state.production_runtime,
                production_reference_kernel: &mut state.production_reference_kernel,
                production_local_kernel: &mut state.production_local_kernel,
                production_kernel_conformance_gate: &mut state.production_kernel_conformance_gate,
                production_kernel_conformance_all_devices_gate: &mut state
                    .production_kernel_conformance_all_devices_gate,
                runtime_manifest_gate: &mut state.runtime_manifest_gate,
                runtime_manifest_all_devices_gate: &mut state.runtime_manifest_all_devices_gate,
                runtime_weights_path: &mut state.runtime_weights_path,
                runtime_tokenizer_path: &mut state.runtime_tokenizer_path,
                runtime_config_path: &mut state.runtime_config_path,
                runtime_layer_count: &mut state.runtime_layer_count,
                runtime_hidden_size: &mut state.runtime_hidden_size,
                runtime_attention_heads: &mut state.runtime_attention_heads,
                runtime_kv_heads: &mut state.runtime_kv_heads,
                runtime_local_window_tokens: &mut state.runtime_local_window_tokens,
                runtime_command: &mut state.runtime_command,
                runtime_args: &mut state.runtime_args,
                runtime_timeout_ms: &mut state.runtime_timeout_ms,
                runtime_stream_idle_timeout_ms: &mut state.runtime_stream_idle_timeout_ms,
                runtime_prompt_mode: &mut state.runtime_prompt_mode,
                runtime_wire_format: &mut state.runtime_wire_format,
                runtime_metadata: &mut state.runtime_metadata,
            })
            .parse(&raw, index)
            {
                index += consumed;
                continue;
            }

            if let Some(consumed) = (ServiceFlagParse {
                serve: &mut state.serve,
                serve_bind: &mut state.serve_bind,
                serve_max_requests: &mut state.serve_max_requests,
                model_pool_manifest_path: &mut state.model_pool_manifest_path,
            })
            .parse(&raw, index)
            {
                index += consumed;
                continue;
            }

            if let Some(consumed) = (StateFlagParse {
                native_window_tokens: &mut state.native_window_tokens,
                chunk_tokens: &mut state.chunk_tokens,
                chunk_overlap_tokens: &mut state.chunk_overlap_tokens,
                merge_fan_in: &mut state.merge_fan_in,
                replay_limit: &mut state.replay_limit,
                auto_replay_limit: &mut state.auto_replay_limit,
                retention_stale_after: &mut state.retention_stale_after,
                retention_decay_rate: &mut state.retention_decay_rate,
                retention_remove_below: &mut state.retention_remove_below,
                retention_remove_after_failures: &mut state.retention_remove_after_failures,
                compaction_similarity_threshold: &mut state.compaction_similarity_threshold,
                compaction_max_candidates: &mut state.compaction_max_candidates,
                compaction_max_merges: &mut state.compaction_max_merges,
            })
            .parse(&raw, index)
            {
                index += consumed;
                continue;
            }

            if let Some(consumed) = (DeviceFlagParse {
                list_devices: &mut state.list_devices,
                probe_device: &mut state.probe_device,
                device_gate: &mut state.device_gate,
                kv_quant_gate: &mut state.kv_quant_gate,
                kv_quant_max_total_us: &mut state.kv_quant_max_total_us,
                device: &mut state.device,
                device_flag_provided: &mut state.device_flag_provided,
                cpu_load: &mut state.cpu_load,
                gpu_load: &mut state.gpu_load,
                ram_load: &mut state.ram_load,
                disk_load: &mut state.disk_load,
                cpu_load_set: &mut state.cpu_load_set,
                gpu_load_set: &mut state.gpu_load_set,
                ram_load_set: &mut state.ram_load_set,
                disk_load_set: &mut state.disk_load_set,
            })
            .parse(&raw, index)
            {
                index += consumed;
                continue;
            }

            if let Some(consumed) = (BenchmarkFlagParse {
                benchmark_path: &mut state.benchmark_path,
                benchmark_gate_enabled: &mut state.benchmark_gate_enabled,
                benchmark_all_devices: &mut state.benchmark_all_devices,
                benchmark_min_quality: &mut state.benchmark_min_quality,
                benchmark_min_reward: &mut state.benchmark_min_reward,
                benchmark_max_total_ms: &mut state.benchmark_max_total_ms,
                benchmark_max_recursive_chunks: &mut state.benchmark_max_recursive_chunks,
                benchmark_min_recursive_cases: &mut state.benchmark_min_recursive_cases,
                benchmark_min_recursive_runtime_calls: &mut state.benchmark_min_recursive_runtime_calls,
                benchmark_min_auto_replay_router_updates: &mut state.benchmark_min_auto_replay_router_updates,
                benchmark_min_auto_replay_hierarchy_updates: &mut state.benchmark_min_auto_replay_hierarchy_updates,
                benchmark_min_auto_replay_router_threshold_mutations: &mut state.benchmark_min_auto_replay_router_threshold_mutations,
                benchmark_min_auto_replay_hierarchy_weight_mutations: &mut state.benchmark_min_auto_replay_hierarchy_weight_mutations,
                benchmark_min_auto_replay_router_threshold_delta: &mut state.benchmark_min_auto_replay_router_threshold_delta,
                benchmark_min_auto_replay_hierarchy_weight_delta: &mut state.benchmark_min_auto_replay_hierarchy_weight_delta,
                benchmark_min_auto_replay_memory_updates: &mut state.benchmark_min_auto_replay_memory_updates,
                benchmark_min_live_memory_feedback_updates: &mut state.benchmark_min_live_memory_feedback_updates,
                benchmark_min_auto_replay_live_memory_feedback_updates: &mut state.benchmark_min_auto_replay_live_memory_feedback_updates,
                benchmark_min_auto_replay_live_memory_feedback_detail_items: &mut state.benchmark_min_auto_replay_live_memory_feedback_detail_items,
                benchmark_min_auto_replay_live_memory_feedback_applied: &mut state.benchmark_min_auto_replay_live_memory_feedback_applied,
                benchmark_min_auto_replay_live_memory_feedback_strength_delta: &mut state.benchmark_min_auto_replay_live_memory_feedback_strength_delta,
                benchmark_min_auto_replay_recursive_items: &mut state.benchmark_min_auto_replay_recursive_items,
                benchmark_min_auto_replay_recursive_call_pressure: &mut state.benchmark_min_auto_replay_recursive_call_pressure,
                benchmark_max_auto_replay_recursive_call_pressure: &mut state.benchmark_max_auto_replay_recursive_call_pressure,
                benchmark_min_evolution_live_inference_runs: &mut state.benchmark_min_evolution_live_inference_runs,
                benchmark_min_evolution_live_router_threshold_mutations: &mut state.benchmark_min_evolution_live_router_threshold_mutations,
                benchmark_min_evolution_live_hierarchy_weight_mutations: &mut state.benchmark_min_evolution_live_hierarchy_weight_mutations,
                benchmark_min_evolution_live_router_threshold_delta: &mut state.benchmark_min_evolution_live_router_threshold_delta,
                benchmark_min_evolution_live_hierarchy_weight_delta: &mut state.benchmark_min_evolution_live_hierarchy_weight_delta,
                benchmark_min_evolution_live_online_reward_feedbacks: &mut state.benchmark_min_evolution_live_online_reward_feedbacks,
                benchmark_min_evolution_live_online_reward_reinforcements: &mut state.benchmark_min_evolution_live_online_reward_reinforcements,
                benchmark_min_evolution_live_online_reward_penalties: &mut state.benchmark_min_evolution_live_online_reward_penalties,
                benchmark_min_evolution_live_online_reward_strength: &mut state.benchmark_min_evolution_live_online_reward_strength,
                benchmark_min_evolution_live_online_reward_reinforcement_strength: &mut state.benchmark_min_evolution_live_online_reward_reinforcement_strength,
                benchmark_min_evolution_live_online_reward_penalty_strength: &mut state.benchmark_min_evolution_live_online_reward_penalty_strength,
                benchmark_min_evolution_live_memory_updates: &mut state.benchmark_min_evolution_live_memory_updates,
                benchmark_min_evolution_live_stored_memory_updates: &mut state.benchmark_min_evolution_live_stored_memory_updates,
                benchmark_min_evolution_live_reflection_issues: &mut state.benchmark_min_evolution_live_reflection_issues,
                benchmark_min_evolution_live_critical_reflection_issues: &mut state.benchmark_min_evolution_live_critical_reflection_issues,
                benchmark_min_evolution_live_revision_actions: &mut state.benchmark_min_evolution_live_revision_actions,
                benchmark_min_evolution_live_inference_device_profiles: &mut state.benchmark_min_evolution_live_inference_device_profiles,
                benchmark_min_evolution_live_router_threshold_mutation_device_profiles: &mut state.benchmark_min_evolution_live_router_threshold_mutation_device_profiles,
                benchmark_min_evolution_live_hierarchy_weight_mutation_device_profiles: &mut state.benchmark_min_evolution_live_hierarchy_weight_mutation_device_profiles,
                benchmark_min_evolution_live_online_reward_device_profiles: &mut state.benchmark_min_evolution_live_online_reward_device_profiles,
                benchmark_min_evolution_live_online_reward_strength_device_profiles: &mut state.benchmark_min_evolution_live_online_reward_strength_device_profiles,
                benchmark_min_evolution_live_memory_update_device_profiles: &mut state.benchmark_min_evolution_live_memory_update_device_profiles,
                benchmark_min_evolution_live_stored_memory_update_device_profiles: &mut state.benchmark_min_evolution_live_stored_memory_update_device_profiles,
                benchmark_min_evolution_live_reflection_issue_device_profiles: &mut state.benchmark_min_evolution_live_reflection_issue_device_profiles,
                benchmark_min_evolution_live_critical_reflection_issue_device_profiles: &mut state.benchmark_min_evolution_live_critical_reflection_issue_device_profiles,
                benchmark_min_evolution_live_revision_action_device_profiles: &mut state.benchmark_min_evolution_live_revision_action_device_profiles,
                benchmark_min_evolution_replay_runs: &mut state.benchmark_min_evolution_replay_runs,
                benchmark_min_evolution_replay_items: &mut state.benchmark_min_evolution_replay_items,
                benchmark_min_evolution_router_threshold_mutations: &mut state.benchmark_min_evolution_router_threshold_mutations,
                benchmark_min_evolution_hierarchy_weight_mutations: &mut state.benchmark_min_evolution_hierarchy_weight_mutations,
                benchmark_min_evolution_router_threshold_delta: &mut state.benchmark_min_evolution_router_threshold_delta,
                benchmark_min_evolution_hierarchy_weight_delta: &mut state.benchmark_min_evolution_hierarchy_weight_delta,
                benchmark_min_evolution_memory_updates: &mut state.benchmark_min_evolution_memory_updates,
                benchmark_min_evolution_replay_live_memory_feedback_updates: &mut state.benchmark_min_evolution_replay_live_memory_feedback_updates,
                benchmark_min_evolution_replay_live_memory_feedback_detail_items: &mut state.benchmark_min_evolution_replay_live_memory_feedback_detail_items,
                benchmark_min_evolution_replay_live_memory_feedback_applied: &mut state.benchmark_min_evolution_replay_live_memory_feedback_applied,
                benchmark_min_evolution_replay_live_memory_feedback_strength_delta: &mut state.benchmark_min_evolution_replay_live_memory_feedback_strength_delta,
                benchmark_min_evolution_replay_rust_check_items: &mut state.benchmark_min_evolution_replay_rust_check_items,
                benchmark_min_evolution_replay_rust_check_passed: &mut state.benchmark_min_evolution_replay_rust_check_passed,
                benchmark_max_evolution_replay_rust_check_failed: &mut state.benchmark_max_evolution_replay_rust_check_failed,
                benchmark_min_evolution_replay_rust_check_live_memory_feedback_updates: &mut state.benchmark_min_evolution_replay_rust_check_live_memory_feedback_updates,
                benchmark_min_evolution_replay_rust_check_live_memory_feedback_applied: &mut state.benchmark_min_evolution_replay_rust_check_live_memory_feedback_applied,
                benchmark_min_evolution_replay_rust_check_live_memory_feedback_strength_delta: &mut state.benchmark_min_evolution_replay_rust_check_live_memory_feedback_strength_delta,
                benchmark_min_evolution_replay_live_evolution_items: &mut state.benchmark_min_evolution_replay_live_evolution_items,
                benchmark_min_evolution_replay_live_evolution_online_reward_feedbacks: &mut state.benchmark_min_evolution_replay_live_evolution_online_reward_feedbacks,
                benchmark_min_evolution_replay_live_evolution_online_reward_reinforcements: &mut state.benchmark_min_evolution_replay_live_evolution_online_reward_reinforcements,
                benchmark_min_evolution_replay_live_evolution_online_reward_penalties: &mut state.benchmark_min_evolution_replay_live_evolution_online_reward_penalties,
                benchmark_min_evolution_replay_live_evolution_online_reward_strength: &mut state.benchmark_min_evolution_replay_live_evolution_online_reward_strength,
                benchmark_min_evolution_replay_live_evolution_online_reward_reinforcement_strength: &mut state.benchmark_min_evolution_replay_live_evolution_online_reward_reinforcement_strength,
                benchmark_min_evolution_replay_live_evolution_online_reward_penalty_strength: &mut state.benchmark_min_evolution_replay_live_evolution_online_reward_penalty_strength,
                benchmark_min_evolution_replay_live_evolution_memory_updates: &mut state.benchmark_min_evolution_replay_live_evolution_memory_updates,
                benchmark_min_evolution_replay_live_evolution_stored_memory_updates: &mut state.benchmark_min_evolution_replay_live_evolution_stored_memory_updates,
                benchmark_min_evolution_replay_live_evolution_reflection_issues: &mut state.benchmark_min_evolution_replay_live_evolution_reflection_issues,
                benchmark_min_evolution_replay_live_evolution_critical_reflection_issues: &mut state.benchmark_min_evolution_replay_live_evolution_critical_reflection_issues,
                benchmark_min_evolution_replay_live_evolution_revision_actions: &mut state.benchmark_min_evolution_replay_live_evolution_revision_actions,
                benchmark_min_evolution_replay_live_evolution_device_profiles: &mut state.benchmark_min_evolution_replay_live_evolution_device_profiles,
                benchmark_min_evolution_replay_live_evolution_online_reward_device_profiles: &mut state.benchmark_min_evolution_replay_live_evolution_online_reward_device_profiles,
                benchmark_min_evolution_replay_live_evolution_online_reward_strength_device_profiles: &mut state.benchmark_min_evolution_replay_live_evolution_online_reward_strength_device_profiles,
                benchmark_min_evolution_replay_live_evolution_memory_update_device_profiles: &mut state.benchmark_min_evolution_replay_live_evolution_memory_update_device_profiles,
                benchmark_min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles: &mut state.benchmark_min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles,
                benchmark_min_evolution_replay_live_evolution_revision_action_device_profiles: &mut state.benchmark_min_evolution_replay_live_evolution_revision_action_device_profiles,
                benchmark_min_evolution_recursive_replay_items: &mut state.benchmark_min_evolution_recursive_replay_items,
                benchmark_min_evolution_recursive_runtime_calls: &mut state.benchmark_min_evolution_recursive_runtime_calls,
                benchmark_max_evolution_drift_rollbacks: &mut state.benchmark_max_evolution_drift_rollbacks,
                benchmark_max_evolution_rollback_router_threshold_delta: &mut state.benchmark_max_evolution_rollback_router_threshold_delta,
                benchmark_max_evolution_rollback_hierarchy_weight_delta: &mut state.benchmark_max_evolution_rollback_hierarchy_weight_delta,
                benchmark_min_sparse_skipped_cases: &mut state.benchmark_min_sparse_skipped_cases,
                benchmark_min_sparse_skipped_tokens: &mut state.benchmark_min_sparse_skipped_tokens,
                benchmark_min_runtime_forward_cases: &mut state.benchmark_min_runtime_forward_cases,
                benchmark_min_runtime_forward_energy_cases: &mut state.benchmark_min_runtime_forward_energy_cases,
                benchmark_min_runtime_kv_influence_cases: &mut state.benchmark_min_runtime_kv_influence_cases,
                benchmark_min_runtime_architecture_cases: &mut state.benchmark_min_runtime_architecture_cases,
                benchmark_min_runtime_architecture_device_profiles: &mut state.benchmark_min_runtime_architecture_device_profiles,
                benchmark_min_runtime_kv_precision_cases: &mut state.benchmark_min_runtime_kv_precision_cases,
                benchmark_min_runtime_layer_mode_cases: &mut state.benchmark_min_runtime_layer_mode_cases,
                benchmark_min_runtime_all_layer_mode_cases: &mut state.benchmark_min_runtime_all_layer_mode_cases,
                benchmark_min_runtime_global_layers: &mut state.benchmark_min_runtime_global_layers,
                benchmark_min_runtime_local_window_layers: &mut state.benchmark_min_runtime_local_window_layers,
                benchmark_min_runtime_convolutional_fusion_layers: &mut state.benchmark_min_runtime_convolutional_fusion_layers,
                benchmark_min_runtime_uncertainty_cases: &mut state.benchmark_min_runtime_uncertainty_cases,
                benchmark_min_runtime_uncertainty_tokens: &mut state.benchmark_min_runtime_uncertainty_tokens,
                benchmark_min_runtime_uncertainty_device_profiles: &mut state.benchmark_min_runtime_uncertainty_device_profiles,
                benchmark_min_runtime_uncertainty_token_device_profiles: &mut state.benchmark_min_runtime_uncertainty_token_device_profiles,
                benchmark_min_runtime_kv_import_cases: &mut state.benchmark_min_runtime_kv_import_cases,
                benchmark_min_runtime_kv_imported: &mut state.benchmark_min_runtime_kv_imported,
                benchmark_min_runtime_kv_import_device_profiles: &mut state.benchmark_min_runtime_kv_import_device_profiles,
                benchmark_min_runtime_kv_exported: &mut state.benchmark_min_runtime_kv_exported,
                benchmark_min_runtime_kv_export_device_profiles: &mut state.benchmark_min_runtime_kv_export_device_profiles,
                benchmark_min_runtime_kv_stored: &mut state.benchmark_min_runtime_kv_stored,
                benchmark_min_runtime_kv_stored_device_profiles: &mut state.benchmark_min_runtime_kv_stored_device_profiles,
                benchmark_min_runtime_kv_hold_cases: &mut state.benchmark_min_runtime_kv_hold_cases,
                benchmark_min_runtime_kv_held: &mut state.benchmark_min_runtime_kv_held,
                benchmark_min_runtime_kv_hold_device_profiles: &mut state.benchmark_min_runtime_kv_hold_device_profiles,
                benchmark_min_runtime_adapter_contract_cases: &mut state.benchmark_min_runtime_adapter_contract_cases,
                benchmark_min_runtime_adapter_kinds: &mut state.benchmark_min_runtime_adapter_kinds,
                benchmark_min_runtime_adapter_observations: &mut state.benchmark_min_runtime_adapter_observations,
                benchmark_min_runtime_adapter_best_score: &mut state.benchmark_min_runtime_adapter_best_score,
                benchmark_max_runtime_adapter_contract_violations: &mut state.benchmark_max_runtime_adapter_contract_violations,
                benchmark_max_runtime_adapter_selection_mismatches: &mut state.benchmark_max_runtime_adapter_selection_mismatches,
                benchmark_min_runtime_embedding_cases: &mut state.benchmark_min_runtime_embedding_cases,
                benchmark_min_runtime_embedding_device_profiles: &mut state.benchmark_min_runtime_embedding_device_profiles,
                benchmark_max_embedding_fallback_cases: &mut state.benchmark_max_embedding_fallback_cases,
                benchmark_max_embedding_evidence_failures: &mut state.benchmark_max_embedding_evidence_failures,
                benchmark_min_runtime_device_execution_cases: &mut state.benchmark_min_runtime_device_execution_cases,
                benchmark_min_runtime_device_execution_device_profiles: &mut state.benchmark_min_runtime_device_execution_device_profiles,
                benchmark_min_runtime_kv_precision_device_profiles: &mut state.benchmark_min_runtime_kv_precision_device_profiles,
                benchmark_max_runtime_device_execution_violations: &mut state.benchmark_max_runtime_device_execution_violations,
                benchmark_max_memory_governance_failures: &mut state.benchmark_max_memory_governance_failures,
                benchmark_min_reasoning_genome_expression_cases: &mut state.benchmark_min_reasoning_genome_expression_cases,
                benchmark_min_reasoning_genome_expression_device_profiles: &mut state.benchmark_min_reasoning_genome_expression_device_profiles,
                benchmark_min_reasoning_genome_splice_cases: &mut state.benchmark_min_reasoning_genome_splice_cases,
                benchmark_min_reasoning_genome_splice_device_profiles: &mut state.benchmark_min_reasoning_genome_splice_device_profiles,
                benchmark_min_gene_scissors_proposal_cases: &mut state.benchmark_min_gene_scissors_proposal_cases,
                benchmark_min_gene_scissors_proposal_device_profiles: &mut state.benchmark_min_gene_scissors_proposal_device_profiles,
                benchmark_min_reasoning_genome_repair_payloads: &mut state.benchmark_min_reasoning_genome_repair_payloads,
                benchmark_min_reasoning_genome_regeneration_payloads: &mut state.benchmark_min_reasoning_genome_regeneration_payloads,
                benchmark_min_mutation_repair_fixtures: &mut state.benchmark_min_mutation_repair_fixtures,
                benchmark_min_mutation_repair_fixture_kinds: &mut state.benchmark_min_mutation_repair_fixture_kinds,
                benchmark_min_mutation_repair_candidates: &mut state.benchmark_min_mutation_repair_candidates,
                benchmark_min_mutation_repair_review_packets: &mut state.benchmark_min_mutation_repair_review_packets,
                benchmark_min_malignant_gene_recovery_drills: &mut state.benchmark_min_malignant_gene_recovery_drills,
                benchmark_min_malignant_gene_quarantines: &mut state.benchmark_min_malignant_gene_quarantines,
                benchmark_min_malignant_gene_cut_candidates: &mut state.benchmark_min_malignant_gene_cut_candidates,
                benchmark_min_malignant_gene_regeneration_candidates: &mut state.benchmark_min_malignant_gene_regeneration_candidates,
                benchmark_min_dna_evolution_reports: &mut state.benchmark_min_dna_evolution_reports,
                benchmark_min_dna_evolution_candidates: &mut state.benchmark_min_dna_evolution_candidates,
                benchmark_min_dna_evolution_candidate_previews: &mut state.benchmark_min_dna_evolution_candidate_previews,
                benchmark_max_dna_evolution_activation_eligible: &mut state.benchmark_max_dna_evolution_activation_eligible,
                benchmark_min_dna_evolution_transaction_replays: &mut state.benchmark_min_dna_evolution_transaction_replays,
                benchmark_min_dna_evolution_replay_passed: &mut state.benchmark_min_dna_evolution_replay_passed,
                benchmark_min_dna_evolution_validation_passed: &mut state.benchmark_min_dna_evolution_validation_passed,
                benchmark_min_dna_evolution_writer_gate_reports: &mut state.benchmark_min_dna_evolution_writer_gate_reports,
                benchmark_min_dna_evolution_writer_gate_holds: &mut state.benchmark_min_dna_evolution_writer_gate_holds,
                benchmark_min_dna_evolution_writer_gate_explicit_apply_required: &mut state.benchmark_min_dna_evolution_writer_gate_explicit_apply_required,
                benchmark_max_dna_evolution_writer_gate_ready: &mut state.benchmark_max_dna_evolution_writer_gate_ready,
                benchmark_max_dna_evolution_writer_gate_durable_write_allowed: &mut state.benchmark_max_dna_evolution_writer_gate_durable_write_allowed,
                benchmark_min_memory_governance_cases: &mut state.benchmark_min_memory_governance_cases,
                benchmark_min_memory_governance_device_profiles: &mut state.benchmark_min_memory_governance_device_profiles,
                benchmark_min_memory_retention_activity_cases: &mut state.benchmark_min_memory_retention_activity_cases,
                benchmark_min_memory_compaction_activity_cases: &mut state.benchmark_min_memory_compaction_activity_cases,
                benchmark_min_reflection_issue_cases: &mut state.benchmark_min_reflection_issue_cases,
                benchmark_min_reflection_issues: &mut state.benchmark_min_reflection_issues,
                benchmark_min_critical_reflection_issue_cases: &mut state.benchmark_min_critical_reflection_issue_cases,
                benchmark_min_critical_reflection_issues: &mut state.benchmark_min_critical_reflection_issues,
                benchmark_min_revision_action_cases: &mut state.benchmark_min_revision_action_cases,
                benchmark_min_revision_actions: &mut state.benchmark_min_revision_actions,
                benchmark_min_reflection_issue_device_profiles: &mut state.benchmark_min_reflection_issue_device_profiles,
                benchmark_min_critical_reflection_issue_device_profiles: &mut state.benchmark_min_critical_reflection_issue_device_profiles,
                benchmark_min_revision_action_device_profiles: &mut state.benchmark_min_revision_action_device_profiles,
                benchmark_min_device_profiles: &mut state.benchmark_min_device_profiles,
                benchmark_min_recursive_device_profiles: &mut state.benchmark_min_recursive_device_profiles,
                benchmark_max_drift_blocks: &mut state.benchmark_max_drift_blocks,
                benchmark_max_drift_rollbacks: &mut state.benchmark_max_drift_rollbacks,
                benchmark_roundtrip: &mut state.benchmark_roundtrip,
            })
            .parse(&raw, index)
            {
                index += consumed;
                continue;
            }

            if let Some(consumed) = (InspectFlagParse {
                inspect_state: &mut state.inspect_state,
                inspect_gate: &mut state.inspect_gate,
                benchmark_all_devices: &mut state.benchmark_all_devices,
                inspect_limit: &mut state.inspect_limit,
                inspect_min_memories: &mut state.inspect_min_memories,
                inspect_min_runtime_kv_memories: &mut state.inspect_min_runtime_kv_memories,
                inspect_min_experiences: &mut state.inspect_min_experiences,
                inspect_max_experience_hygiene_quarantine_candidates: &mut state
                    .inspect_max_experience_hygiene_quarantine_candidates,
                inspect_max_experience_repairable_legacy_metadata_lessons: &mut state
                    .inspect_max_experience_repairable_legacy_metadata_lessons,
                inspect_max_experience_repairable_index_records: &mut state
                    .inspect_max_experience_repairable_index_records,
                inspect_max_experience_repair_projected_legacy_metadata_lessons: &mut state
                    .inspect_max_experience_repair_projected_legacy_metadata_lessons,
                inspect_max_experience_repair_skipped_missing_clean_gist: &mut state
                    .inspect_max_experience_repair_skipped_missing_clean_gist,
                inspect_max_experience_index_overlong_records: &mut state
                    .inspect_max_experience_index_overlong_records,
                inspect_max_experience_index_overlong_without_clean_gist: &mut state
                    .inspect_max_experience_index_overlong_without_clean_gist,
                inspect_max_experience_index_record_chars: &mut state
                    .inspect_max_experience_index_record_chars,
                inspect_max_experience_index_noisy_records: &mut state
                    .inspect_max_experience_index_noisy_records,
                inspect_max_experience_index_noise_penalty: &mut state
                    .inspect_max_experience_index_noise_penalty,
                inspect_min_experience_index_quality_score: &mut state
                    .inspect_min_experience_index_quality_score,
                inspect_require_experience_index_retrieval_ready: &mut state
                    .inspect_require_experience_index_retrieval_ready,
                inspect_min_runtime_model_experiences: &mut state.inspect_min_runtime_model_experiences,
                inspect_min_runtime_adapter_experiences: &mut state.inspect_min_runtime_adapter_experiences,
                inspect_max_runtime_adapter_selection_mismatches: &mut state.inspect_max_runtime_adapter_selection_mismatches,
                inspect_min_runtime_forward_energy_experiences: &mut state.inspect_min_runtime_forward_energy_experiences,
                inspect_min_runtime_kv_influence_experiences: &mut state.inspect_min_runtime_kv_influence_experiences,
                inspect_min_runtime_tokens: &mut state.inspect_min_runtime_tokens,
                inspect_min_runtime_uncertainty_experiences: &mut state.inspect_min_runtime_uncertainty_experiences,
                inspect_min_runtime_uncertainty_tokens: &mut state.inspect_min_runtime_uncertainty_tokens,
                inspect_min_runtime_architecture_experiences: &mut state.inspect_min_runtime_architecture_experiences,
                inspect_min_runtime_kv_precision_experiences: &mut state.inspect_min_runtime_kv_precision_experiences,
                inspect_max_runtime_kv_precision_mismatches: &mut state.inspect_max_runtime_kv_precision_mismatches,
                inspect_max_runtime_errors: &mut state.inspect_max_runtime_errors,
                inspect_max_runtime_timeouts: &mut state.inspect_max_runtime_timeouts,
                inspect_min_runtime_device_execution_experiences: &mut state.inspect_min_runtime_device_execution_experiences,
                inspect_min_runtime_layer_mode_experiences: &mut state.inspect_min_runtime_layer_mode_experiences,
                inspect_min_runtime_all_layer_mode_experiences: &mut state.inspect_min_runtime_all_layer_mode_experiences,
                inspect_min_runtime_global_layers: &mut state.inspect_min_runtime_global_layers,
                inspect_min_runtime_local_window_layers: &mut state.inspect_min_runtime_local_window_layers,
                inspect_min_runtime_convolutional_fusion_layers: &mut state.inspect_min_runtime_convolutional_fusion_layers,
                inspect_min_runtime_kv_import_experiences: &mut state.inspect_min_runtime_kv_import_experiences,
                inspect_min_runtime_kv_export_experiences: &mut state.inspect_min_runtime_kv_export_experiences,
                inspect_min_runtime_kv_hold_experiences: &mut state.inspect_min_runtime_kv_hold_experiences,
                inspect_min_runtime_kv_held_blocks: &mut state.inspect_min_runtime_kv_held_blocks,
                inspect_min_runtime_kv_memory_device_profiles: &mut state.inspect_min_runtime_kv_memory_device_profiles,
                inspect_min_runtime_model_device_profiles: &mut state.inspect_min_runtime_model_device_profiles,
                inspect_min_runtime_adapter_device_profiles: &mut state.inspect_min_runtime_adapter_device_profiles,
                inspect_min_runtime_forward_energy_device_profiles: &mut state.inspect_min_runtime_forward_energy_device_profiles,
                inspect_min_runtime_kv_influence_device_profiles: &mut state.inspect_min_runtime_kv_influence_device_profiles,
                inspect_min_runtime_uncertainty_device_profiles: &mut state.inspect_min_runtime_uncertainty_device_profiles,
                inspect_min_runtime_uncertainty_token_device_profiles: &mut state.inspect_min_runtime_uncertainty_token_device_profiles,
                inspect_min_runtime_kv_precision_device_profiles: &mut state.inspect_min_runtime_kv_precision_device_profiles,
                inspect_min_runtime_device_execution_device_profiles: &mut state.inspect_min_runtime_device_execution_device_profiles,
                inspect_min_runtime_layer_mode_device_profiles: &mut state.inspect_min_runtime_layer_mode_device_profiles,
                inspect_min_runtime_all_layer_mode_device_profiles: &mut state.inspect_min_runtime_all_layer_mode_device_profiles,
                inspect_min_runtime_kv_import_device_profiles: &mut state.inspect_min_runtime_kv_import_device_profiles,
                inspect_min_runtime_kv_export_device_profiles: &mut state.inspect_min_runtime_kv_export_device_profiles,
                inspect_min_runtime_kv_hold_device_profiles: &mut state.inspect_min_runtime_kv_hold_device_profiles,
                inspect_min_reflection_issue_experiences: &mut state.inspect_min_reflection_issue_experiences,
                inspect_min_critical_reflection_issue_experiences: &mut state.inspect_min_critical_reflection_issue_experiences,
                inspect_min_revision_action_experiences: &mut state.inspect_min_revision_action_experiences,
                inspect_min_live_memory_feedback_experiences: &mut state.inspect_min_live_memory_feedback_experiences,
                inspect_min_live_memory_feedback_updates: &mut state.inspect_min_live_memory_feedback_updates,
                inspect_min_live_memory_feedback_detail_experiences: &mut state.inspect_min_live_memory_feedback_detail_experiences,
                inspect_min_live_memory_feedback_applied: &mut state.inspect_min_live_memory_feedback_applied,
                inspect_min_live_memory_feedback_strength_delta: &mut state.inspect_min_live_memory_feedback_strength_delta,
                inspect_min_rust_check_experiences: &mut state.inspect_min_rust_check_experiences,
                inspect_min_rust_check_passed: &mut state.inspect_min_rust_check_passed,
                inspect_max_rust_check_failed: &mut state.inspect_max_rust_check_failed,
                inspect_min_rust_check_diagnostic_chars: &mut state.inspect_min_rust_check_diagnostic_chars,
                inspect_min_reflection_issue_device_profiles: &mut state.inspect_min_reflection_issue_device_profiles,
                inspect_min_critical_reflection_issue_device_profiles: &mut state.inspect_min_critical_reflection_issue_device_profiles,
                inspect_min_revision_action_device_profiles: &mut state.inspect_min_revision_action_device_profiles,
                inspect_min_live_memory_feedback_device_profiles: &mut state.inspect_min_live_memory_feedback_device_profiles,
                inspect_min_evolution_live_inference_device_profiles: &mut state.inspect_min_evolution_live_inference_device_profiles,
                inspect_min_evolution_live_router_threshold_mutation_device_profiles: &mut state.inspect_min_evolution_live_router_threshold_mutation_device_profiles,
                inspect_min_evolution_live_hierarchy_weight_mutation_device_profiles: &mut state.inspect_min_evolution_live_hierarchy_weight_mutation_device_profiles,
                inspect_min_evolution_live_online_reward_device_profiles: &mut state.inspect_min_evolution_live_online_reward_device_profiles,
                inspect_min_evolution_live_online_reward_strength_device_profiles: &mut state.inspect_min_evolution_live_online_reward_strength_device_profiles,
                inspect_min_evolution_live_memory_update_device_profiles: &mut state.inspect_min_evolution_live_memory_update_device_profiles,
                inspect_min_evolution_live_stored_memory_update_device_profiles: &mut state.inspect_min_evolution_live_stored_memory_update_device_profiles,
                inspect_min_evolution_live_reflection_issue_device_profiles: &mut state.inspect_min_evolution_live_reflection_issue_device_profiles,
                inspect_min_evolution_live_critical_reflection_issue_device_profiles: &mut state.inspect_min_evolution_live_critical_reflection_issue_device_profiles,
                inspect_min_evolution_live_revision_action_device_profiles: &mut state.inspect_min_evolution_live_revision_action_device_profiles,
                inspect_min_evolution_replay_run_device_profiles: &mut state.inspect_min_evolution_replay_run_device_profiles,
                inspect_min_evolution_replay_item_device_profiles: &mut state.inspect_min_evolution_replay_item_device_profiles,
                inspect_min_evolution_router_threshold_mutation_device_profiles: &mut state.inspect_min_evolution_router_threshold_mutation_device_profiles,
                inspect_min_evolution_hierarchy_weight_mutation_device_profiles: &mut state.inspect_min_evolution_hierarchy_weight_mutation_device_profiles,
                inspect_min_evolution_memory_update_device_profiles: &mut state.inspect_min_evolution_memory_update_device_profiles,
                inspect_min_evolution_replay_live_memory_feedback_device_profiles: &mut state.inspect_min_evolution_replay_live_memory_feedback_device_profiles,
                inspect_min_evolution_replay_live_memory_feedback_detail_device_profiles: &mut state.inspect_min_evolution_replay_live_memory_feedback_detail_device_profiles,
                inspect_min_evolution_replay_live_evolution_device_profiles: &mut state.inspect_min_evolution_replay_live_evolution_device_profiles,
                inspect_min_evolution_replay_live_evolution_online_reward_device_profiles: &mut state.inspect_min_evolution_replay_live_evolution_online_reward_device_profiles,
                inspect_min_evolution_replay_live_evolution_online_reward_strength_device_profiles: &mut state.inspect_min_evolution_replay_live_evolution_online_reward_strength_device_profiles,
                inspect_min_evolution_replay_live_evolution_memory_update_device_profiles: &mut state.inspect_min_evolution_replay_live_evolution_memory_update_device_profiles,
                inspect_min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles: &mut state.inspect_min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles,
                inspect_min_evolution_replay_live_evolution_revision_action_device_profiles: &mut state.inspect_min_evolution_replay_live_evolution_revision_action_device_profiles,
                inspect_min_evolution_recursive_replay_device_profiles: &mut state.inspect_min_evolution_recursive_replay_device_profiles,
                inspect_min_evolution_recursive_runtime_call_device_profiles: &mut state.inspect_min_evolution_recursive_runtime_call_device_profiles,
                inspect_min_router_observations: &mut state.inspect_min_router_observations,
                inspect_min_evolution_live_inference_runs: &mut state.inspect_min_evolution_live_inference_runs,
                inspect_min_evolution_live_router_threshold_mutations: &mut state.inspect_min_evolution_live_router_threshold_mutations,
                inspect_min_evolution_live_hierarchy_weight_mutations: &mut state.inspect_min_evolution_live_hierarchy_weight_mutations,
                inspect_min_evolution_live_router_threshold_delta: &mut state.inspect_min_evolution_live_router_threshold_delta,
                inspect_min_evolution_live_hierarchy_weight_delta: &mut state.inspect_min_evolution_live_hierarchy_weight_delta,
                inspect_min_evolution_live_online_reward_feedbacks: &mut state.inspect_min_evolution_live_online_reward_feedbacks,
                inspect_min_evolution_live_online_reward_reinforcements: &mut state.inspect_min_evolution_live_online_reward_reinforcements,
                inspect_min_evolution_live_online_reward_penalties: &mut state.inspect_min_evolution_live_online_reward_penalties,
                inspect_min_evolution_live_online_reward_strength: &mut state.inspect_min_evolution_live_online_reward_strength,
                inspect_min_evolution_live_online_reward_reinforcement_strength: &mut state.inspect_min_evolution_live_online_reward_reinforcement_strength,
                inspect_min_evolution_live_online_reward_penalty_strength: &mut state.inspect_min_evolution_live_online_reward_penalty_strength,
                inspect_min_evolution_live_memory_updates: &mut state.inspect_min_evolution_live_memory_updates,
                inspect_min_evolution_live_stored_memory_updates: &mut state.inspect_min_evolution_live_stored_memory_updates,
                inspect_min_evolution_live_reflection_issues: &mut state.inspect_min_evolution_live_reflection_issues,
                inspect_min_evolution_live_critical_reflection_issues: &mut state.inspect_min_evolution_live_critical_reflection_issues,
                inspect_min_evolution_live_revision_actions: &mut state.inspect_min_evolution_live_revision_actions,
                inspect_min_evolution_replay_runs: &mut state.inspect_min_evolution_replay_runs,
                inspect_min_evolution_replay_items: &mut state.inspect_min_evolution_replay_items,
                inspect_min_evolution_router_threshold_mutations: &mut state.inspect_min_evolution_router_threshold_mutations,
                inspect_min_evolution_hierarchy_weight_mutations: &mut state.inspect_min_evolution_hierarchy_weight_mutations,
                inspect_min_evolution_router_threshold_delta: &mut state.inspect_min_evolution_router_threshold_delta,
                inspect_min_evolution_hierarchy_weight_delta: &mut state.inspect_min_evolution_hierarchy_weight_delta,
                inspect_min_evolution_memory_updates: &mut state.inspect_min_evolution_memory_updates,
                inspect_min_evolution_external_feedbacks: &mut state.inspect_min_evolution_external_feedbacks,
                inspect_min_evolution_external_feedback_memory_updates: &mut state.inspect_min_evolution_external_feedback_memory_updates,
                inspect_min_evolution_external_feedback_strength_delta: &mut state.inspect_min_evolution_external_feedback_strength_delta,
                inspect_min_evolution_replay_live_memory_feedback_updates: &mut state.inspect_min_evolution_replay_live_memory_feedback_updates,
                inspect_min_evolution_replay_live_memory_feedback_detail_items: &mut state.inspect_min_evolution_replay_live_memory_feedback_detail_items,
                inspect_min_evolution_replay_live_memory_feedback_applied: &mut state.inspect_min_evolution_replay_live_memory_feedback_applied,
                inspect_min_evolution_replay_live_memory_feedback_strength_delta: &mut state.inspect_min_evolution_replay_live_memory_feedback_strength_delta,
                inspect_min_evolution_replay_rust_check_items: &mut state.inspect_min_evolution_replay_rust_check_items,
                inspect_min_evolution_replay_rust_check_passed: &mut state.inspect_min_evolution_replay_rust_check_passed,
                inspect_max_evolution_replay_rust_check_failed: &mut state.inspect_max_evolution_replay_rust_check_failed,
                inspect_min_evolution_replay_rust_check_live_memory_feedback_updates: &mut state.inspect_min_evolution_replay_rust_check_live_memory_feedback_updates,
                inspect_min_evolution_replay_rust_check_live_memory_feedback_applied: &mut state.inspect_min_evolution_replay_rust_check_live_memory_feedback_applied,
                inspect_min_evolution_replay_rust_check_live_memory_feedback_strength_delta: &mut state.inspect_min_evolution_replay_rust_check_live_memory_feedback_strength_delta,
                inspect_min_evolution_replay_live_evolution_items: &mut state.inspect_min_evolution_replay_live_evolution_items,
                inspect_min_evolution_replay_live_evolution_online_reward_feedbacks: &mut state.inspect_min_evolution_replay_live_evolution_online_reward_feedbacks,
                inspect_min_evolution_replay_live_evolution_online_reward_reinforcements: &mut state.inspect_min_evolution_replay_live_evolution_online_reward_reinforcements,
                inspect_min_evolution_replay_live_evolution_online_reward_penalties: &mut state.inspect_min_evolution_replay_live_evolution_online_reward_penalties,
                inspect_min_evolution_replay_live_evolution_online_reward_strength: &mut state.inspect_min_evolution_replay_live_evolution_online_reward_strength,
                inspect_min_evolution_replay_live_evolution_online_reward_reinforcement_strength: &mut state.inspect_min_evolution_replay_live_evolution_online_reward_reinforcement_strength,
                inspect_min_evolution_replay_live_evolution_online_reward_penalty_strength: &mut state.inspect_min_evolution_replay_live_evolution_online_reward_penalty_strength,
                inspect_min_evolution_replay_live_evolution_memory_updates: &mut state.inspect_min_evolution_replay_live_evolution_memory_updates,
                inspect_min_evolution_replay_live_evolution_stored_memory_updates: &mut state.inspect_min_evolution_replay_live_evolution_stored_memory_updates,
                inspect_min_evolution_replay_live_evolution_reflection_issues: &mut state.inspect_min_evolution_replay_live_evolution_reflection_issues,
                inspect_min_evolution_replay_live_evolution_critical_reflection_issues: &mut state.inspect_min_evolution_replay_live_evolution_critical_reflection_issues,
                inspect_min_evolution_replay_live_evolution_revision_actions: &mut state.inspect_min_evolution_replay_live_evolution_revision_actions,
                inspect_min_evolution_recursive_replay_items: &mut state.inspect_min_evolution_recursive_replay_items,
                inspect_min_evolution_recursive_runtime_calls: &mut state.inspect_min_evolution_recursive_runtime_calls,
                inspect_max_evolution_drift_rollbacks: &mut state.inspect_max_evolution_drift_rollbacks,
                inspect_max_evolution_rollback_router_threshold_delta: &mut state.inspect_max_evolution_rollback_router_threshold_delta,
                inspect_max_evolution_rollback_hierarchy_weight_delta: &mut state.inspect_max_evolution_rollback_hierarchy_weight_delta,
                inspect_require_runtime_kv_dimensions: &mut state.inspect_require_runtime_kv_dimensions,
            })
            .parse(&raw, index)
            {
                index += consumed;
                continue;
            }

            match raw[index].as_str() {
                "--profile" | "-p" if index + 1 < raw.len() => {
                    state.profile = raw[index + 1].parse::<TaskProfile>().ok();
                    index += 2;
                }
                "--max-tokens" | "--max" if index + 1 < raw.len() => {
                    state.max_tokens = Some(parse_usize(&raw[index + 1], 1).max(1));
                    index += 2;
                }
                "--memory" | "-m" if index + 1 < raw.len() => {
                    state.memory_path = PathBuf::from(&raw[index + 1]);
                    state.memory_path_set = true;
                    index += 2;
                }
                "--experience" | "-e" if index + 1 < raw.len() => {
                    state.experience_path = PathBuf::from(&raw[index + 1]);
                    state.experience_path_set = true;
                    index += 2;
                }
                "--adaptive" | "-a" if index + 1 < raw.len() => {
                    state.adaptive_path = PathBuf::from(&raw[index + 1]);
                    state.adaptive_path_set = true;
                    index += 2;
                }
                "--trace" if index + 1 < raw.len() => {
                    state.trace_path = Some(PathBuf::from(&raw[index + 1]));
                    state.trace_path_set = true;
                    index += 2;
                }
                "--trace-schema-gate" | "--trace-gate" if index + 1 < raw.len() => {
                    state.trace_schema_gate_path = Some(PathBuf::from(&raw[index + 1]));
                    state.trace_schema_gate_path_set = true;
                    index += 2;
                }
                "--self-goal-queue" => {
                    state.self_goal_queue = true;
                    index += 1;
                }
                "--self-goal-queue-store" if index + 1 < raw.len() => {
                    state.self_goal_queue = true;
                    state.self_goal_queue_store_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--self-goal-queue-store-apply" => {
                    state.self_goal_queue = true;
                    state.self_goal_queue_store_apply = true;
                    index += 1;
                }
                "--self-goal-queue-evidence" if index + 1 < raw.len() => {
                    state.self_goal_queue = true;
                    state
                        .self_goal_queue_evidence_packets
                        .push(raw[index + 1].to_owned());
                    index += 2;
                }
                "--self-goal-queue-evidence-file" if index + 1 < raw.len() => {
                    state.self_goal_queue = true;
                    state.self_goal_queue_evidence_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--self-goal-queue-local-evidence" => {
                    state.self_goal_queue = true;
                    state.self_goal_queue_local_evidence = true;
                    index += 1;
                }
                "--self-goal-queue-local-evidence-dry-run" => {
                    state.self_goal_queue = true;
                    state.self_goal_queue_local_evidence = true;
                    state.self_goal_queue_local_evidence_dry_run = true;
                    index += 1;
                }
                "--coding-service-eval-readiness" => {
                    state.coding_service_eval_readiness = true;
                    index += 1;
                }
                "--coding-service-eval-runner" => {
                    state.coding_service_eval_runner = true;
                    index += 1;
                }
                "--self-goal-queue-tenant" if index + 1 < raw.len() => {
                    state.self_goal_queue = true;
                    state.self_goal_queue_tenant = raw[index + 1].to_owned();
                    index += 2;
                }
                "--self-goal-queue-workspace" if index + 1 < raw.len() => {
                    state.self_goal_queue = true;
                    state.self_goal_queue_workspace = raw[index + 1].to_owned();
                    index += 2;
                }
                "--self-goal-queue-session" if index + 1 < raw.len() => {
                    state.self_goal_queue = true;
                    state.self_goal_queue_session = raw[index + 1].to_owned();
                    index += 2;
                }
                "--self-goal-queue-key" if index + 1 < raw.len() => {
                    state.self_goal_queue = true;
                    state.self_goal_queue_key = raw[index + 1].to_owned();
                    index += 2;
                }
                "--self-goal-queue-operator" if index + 1 < raw.len() => {
                    state.self_goal_queue = true;
                    state.self_goal_queue_operator = raw[index + 1].to_owned();
                    index += 2;
                }
                "--self-goal-queue-ticket" if index + 1 < raw.len() => {
                    state.self_goal_queue = true;
                    state.self_goal_queue_ticket = raw[index + 1].to_owned();
                    index += 2;
                }
                "--experience-hygiene" => {
                    state.experience_hygiene = true;
                    index += 1;
                }
                "--experience-hygiene-quarantine" => {
                    state.experience_hygiene = true;
                    state.experience_hygiene_quarantine = true;
                    index += 1;
                }
                "--experience-hygiene-apply" => {
                    state.experience_hygiene = true;
                    state.experience_hygiene_quarantine = true;
                    state.experience_hygiene_apply = true;
                    index += 1;
                }
                "--experience-hygiene-limit" if index + 1 < raw.len() => {
                    state.experience_hygiene = true;
                    state.experience_hygiene_limit =
                        parse_usize(&raw[index + 1], state.experience_hygiene_limit).max(1);
                    index += 2;
                }
                "--experience-hygiene-quarantine-path" if index + 1 < raw.len() => {
                    state.experience_hygiene = true;
                    state.experience_hygiene_quarantine = true;
                    state.experience_hygiene_quarantine_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--experience-hygiene-backup-path" if index + 1 < raw.len() => {
                    state.experience_hygiene = true;
                    state.experience_hygiene_quarantine = true;
                    state.experience_hygiene_backup_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--experience-repair" => {
                    state.experience_repair = true;
                    index += 1;
                }
                "--experience-repair-apply" => {
                    state.experience_repair = true;
                    state.experience_repair_apply = true;
                    index += 1;
                }
                "--experience-repair-limit" if index + 1 < raw.len() => {
                    state.experience_repair = true;
                    state.experience_repair_limit =
                        parse_usize(&raw[index + 1], state.experience_repair_limit).max(1);
                    index += 2;
                }
                "--experience-repair-backup-path" if index + 1 < raw.len() => {
                    state.experience_repair = true;
                    state.experience_repair_apply = true;
                    state.experience_repair_backup_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--experience-retrieval" | "--retrieve-experience" => {
                    state.experience_retrieval = true;
                    index += 1;
                }
                "--experience-retrieval-limit" | "--retrieve-experience-limit"
                    if index + 1 < raw.len() =>
                {
                    state.experience_retrieval = true;
                    state.experience_retrieval_limit =
                        parse_usize(&raw[index + 1], state.experience_retrieval_limit).max(1);
                    index += 2;
                }
                "--experience-cleanup-audit" => {
                    state.experience_cleanup_audit = true;
                    index += 1;
                }
                "--experience-cleanup-audit-limit" if index + 1 < raw.len() => {
                    state.experience_cleanup_audit = true;
                    state.experience_cleanup_audit_limit =
                        parse_usize(&raw[index + 1], state.experience_cleanup_audit_limit).max(1);
                    index += 2;
                }
                "--experience-cleanup-audit-path" if index + 1 < raw.len() => {
                    state.experience_cleanup_audit = true;
                    state.experience_cleanup_audit_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--experience-index-add-clean-gist" => {
                    state.experience_index_add_clean_gist = true;
                    index += 1;
                }
                "--experience-index-record-id" if index + 1 < raw.len() => {
                    state.experience_index_add_clean_gist = true;
                    state.experience_index_record_id = Some(parse_u64(&raw[index + 1], 0).max(1));
                    index += 2;
                }
                "--experience-index-clean-gist" if index + 1 < raw.len() => {
                    state.experience_index_add_clean_gist = true;
                    state.experience_index_clean_gist = Some(raw[index + 1].to_owned());
                    index += 2;
                }
                "--experience-index-clean-gist-title" if index + 1 < raw.len() => {
                    state.experience_index_add_clean_gist = true;
                    state.experience_index_clean_gist_title = Some(raw[index + 1].to_owned());
                    index += 2;
                }
                "--experience-index-clean-gist-importance" if index + 1 < raw.len() => {
                    state.experience_index_add_clean_gist = true;
                    state.experience_index_clean_gist_importance = parse_f32(
                        &raw[index + 1],
                        state.experience_index_clean_gist_importance,
                    )
                    .clamp(0.0, 1.0);
                    index += 2;
                }
                "--experience-index-backup-path" if index + 1 < raw.len() => {
                    state.experience_index_add_clean_gist = true;
                    state.experience_index_backup_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--help" | "-h" => {
                    print_help_and_exit();
                }
                value => {
                    state.prompt_parts.push(value.to_owned());
                    index += 1;
                }
            }
        }

        state.finalize()
    }
}
