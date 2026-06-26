use crate::hardware::DeviceClass;
use crate::state_inspect::StateInspectionReport;

use super::super::gate_report::StateInspectionGateReport;
use super::StateInspectionDeviceGateReport;

impl StateInspectionDeviceGateReport {
    pub fn new(device: DeviceClass, report: StateInspectionGateReport) -> Self {
        Self {
            device,
            report,
            runtime_kv_memories: 0,
            runtime_model_experiences: 0,
            runtime_adapter_experiences: 0,
            runtime_adapter_selection_mismatches: 0,
            runtime_forward_energy_experiences: 0,
            runtime_kv_influence_experiences: 0,
            runtime_uncertainty_experiences: 0,
            runtime_uncertainty_tokens: 0,
            runtime_architecture_experiences: 0,
            runtime_kv_precision_experiences: 0,
            runtime_kv_precision_mismatches: 0,
            runtime_device_execution_experiences: 0,
            runtime_layer_mode_experiences: 0,
            runtime_all_layer_mode_experiences: 0,
            runtime_kv_import_experiences: 0,
            runtime_kv_weak_import_skip_experiences: 0,
            weak_runtime_kv_imports_skipped: 0,
            runtime_kv_budget_import_skip_experiences: 0,
            budget_limited_runtime_kv_imports_skipped: 0,
            runtime_kv_budget_pressure_experiences: 0,
            runtime_kv_export_experiences: 0,
            runtime_kv_segment_experiences: 0,
            runtime_kv_segments_included: 0,
            runtime_kv_segments_skipped: 0,
            runtime_kv_segments_rejected: 0,
            runtime_kv_hold_experiences: 0,
            runtime_kv_held_blocks: 0,
            reflection_issue_experiences: 0,
            critical_reflection_issue_experiences: 0,
            revision_action_experiences: 0,
            live_memory_feedback_experiences: 0,
            live_memory_feedback_updates: 0,
            live_memory_feedback_detail_experiences: 0,
            live_memory_feedback_applied: 0,
            live_memory_feedback_removed: 0,
            live_memory_feedback_missing: 0,
            live_memory_feedback_strength_delta: 0.0,
            evolution_live_inference_runs: 0,
            evolution_live_router_threshold_mutations: 0,
            evolution_live_hierarchy_weight_mutations: 0,
            evolution_live_online_reward_feedbacks: 0,
            evolution_live_online_reward_reinforcements: 0,
            evolution_live_online_reward_penalties: 0,
            evolution_live_online_reward_strength: 0.0,
            evolution_live_online_reward_reinforcement_strength: 0.0,
            evolution_live_online_reward_penalty_strength: 0.0,
            evolution_live_memory_updates: 0,
            evolution_live_stored_memory_updates: 0,
            evolution_live_reflection_issues: 0,
            evolution_live_critical_reflection_issues: 0,
            evolution_live_revision_actions: 0,
            evolution_replay_runs: 0,
            evolution_replay_items: 0,
            evolution_router_threshold_mutations: 0,
            evolution_hierarchy_weight_mutations: 0,
            evolution_memory_updates: 0,
            evolution_replay_live_memory_feedback_updates: 0,
            evolution_replay_live_memory_feedback_detail_items: 0,
            evolution_replay_live_memory_feedback_applied: 0,
            evolution_replay_live_memory_feedback_removed: 0,
            evolution_replay_live_memory_feedback_missing: 0,
            evolution_replay_live_memory_feedback_strength_delta: 0.0,
            evolution_replay_live_evolution_items: 0,
            evolution_replay_live_evolution_online_reward_feedbacks: 0,
            evolution_replay_live_evolution_online_reward_reinforcements: 0,
            evolution_replay_live_evolution_online_reward_penalties: 0,
            evolution_replay_live_evolution_online_reward_strength: 0.0,
            evolution_replay_live_evolution_online_reward_reinforcement_strength: 0.0,
            evolution_replay_live_evolution_online_reward_penalty_strength: 0.0,
            evolution_replay_live_evolution_memory_updates: 0,
            evolution_replay_live_evolution_stored_memory_updates: 0,
            evolution_replay_live_evolution_reflection_issues: 0,
            evolution_replay_live_evolution_critical_reflection_issues: 0,
            evolution_replay_live_evolution_revision_actions: 0,
            evolution_recursive_replay_items: 0,
            evolution_recursive_runtime_calls: 0,
        }
    }

    pub fn from_report(
        device: DeviceClass,
        inspection: &StateInspectionReport,
        report: StateInspectionGateReport,
    ) -> Self {
        Self {
            device,
            report,
            runtime_kv_memories: inspection.runtime_kv_memory_count,
            runtime_model_experiences: inspection.runtime_model_experience_count,
            runtime_adapter_experiences: inspection.runtime_adapter_experience_count,
            runtime_adapter_selection_mismatches: inspection
                .runtime_adapter_selection_mismatch_count,
            runtime_forward_energy_experiences: inspection.runtime_forward_energy_experience_count,
            runtime_kv_influence_experiences: inspection.runtime_kv_influence_experience_count,
            runtime_uncertainty_experiences: inspection.runtime_uncertainty_experience_count,
            runtime_uncertainty_tokens: inspection.runtime_uncertainty_token_count,
            runtime_architecture_experiences: inspection.runtime_architecture_experience_count,
            runtime_kv_precision_experiences: inspection.runtime_kv_precision_experience_count,
            runtime_kv_precision_mismatches: inspection.runtime_kv_precision_mismatch_count,
            runtime_device_execution_experiences: inspection
                .runtime_device_execution_experience_count,
            runtime_layer_mode_experiences: inspection.runtime_layer_mode_experience_count,
            runtime_all_layer_mode_experiences: inspection.runtime_all_layer_mode_experience_count,
            runtime_kv_import_experiences: inspection.runtime_kv_import_experience_count,
            runtime_kv_weak_import_skip_experiences: inspection
                .runtime_kv_weak_import_skip_experience_count,
            weak_runtime_kv_imports_skipped: inspection.weak_runtime_kv_imports_skipped,
            runtime_kv_budget_import_skip_experiences: inspection
                .runtime_kv_budget_import_skip_experience_count,
            budget_limited_runtime_kv_imports_skipped: inspection
                .budget_limited_runtime_kv_imports_skipped,
            runtime_kv_budget_pressure_experiences: inspection
                .runtime_kv_budget_pressure_experience_count,
            runtime_kv_export_experiences: inspection.runtime_kv_export_experience_count,
            runtime_kv_segment_experiences: inspection.runtime_kv_segment_experience_count,
            runtime_kv_segments_included: inspection.runtime_kv_segments_included,
            runtime_kv_segments_skipped: inspection.runtime_kv_segments_skipped,
            runtime_kv_segments_rejected: inspection.runtime_kv_segments_rejected,
            runtime_kv_hold_experiences: inspection.runtime_kv_hold_experience_count,
            runtime_kv_held_blocks: inspection.runtime_kv_held_blocks,
            reflection_issue_experiences: inspection.reflection_issue_experience_count,
            critical_reflection_issue_experiences: inspection
                .critical_reflection_issue_experience_count,
            revision_action_experiences: inspection.revision_action_experience_count,
            live_memory_feedback_experiences: inspection.live_memory_feedback_experience_count,
            live_memory_feedback_updates: inspection.live_memory_feedback_update_count,
            live_memory_feedback_detail_experiences: inspection
                .live_memory_feedback_detail_experience_count,
            live_memory_feedback_applied: inspection.live_memory_feedback_applied_count,
            live_memory_feedback_removed: inspection.live_memory_feedback_removed_count,
            live_memory_feedback_missing: inspection.live_memory_feedback_missing_count,
            live_memory_feedback_strength_delta: inspection.live_memory_feedback_strength_delta,
            evolution_live_inference_runs: inspection.evolution_ledger.live_inference_runs,
            evolution_live_router_threshold_mutations: inspection
                .evolution_ledger
                .live_router_threshold_mutations,
            evolution_live_hierarchy_weight_mutations: inspection
                .evolution_ledger
                .live_hierarchy_weight_mutations,
            evolution_live_online_reward_feedbacks: inspection
                .evolution_ledger
                .live_online_reward_feedbacks,
            evolution_live_online_reward_reinforcements: inspection
                .evolution_ledger
                .live_online_reward_reinforcements,
            evolution_live_online_reward_penalties: inspection
                .evolution_ledger
                .live_online_reward_penalties,
            evolution_live_online_reward_strength: inspection
                .evolution_ledger
                .live_online_reward_strength,
            evolution_live_online_reward_reinforcement_strength: inspection
                .evolution_ledger
                .live_online_reward_reinforcement_strength,
            evolution_live_online_reward_penalty_strength: inspection
                .evolution_ledger
                .live_online_reward_penalty_strength,
            evolution_live_memory_updates: inspection.evolution_ledger.live_memory_updates(),
            evolution_live_stored_memory_updates: inspection
                .evolution_ledger
                .live_stored_memory_updates(),
            evolution_live_reflection_issues: inspection.evolution_ledger.live_reflection_issues,
            evolution_live_critical_reflection_issues: inspection
                .evolution_ledger
                .live_critical_reflection_issues,
            evolution_live_revision_actions: inspection.evolution_ledger.live_revision_actions,
            evolution_replay_runs: inspection.evolution_ledger.replay_runs,
            evolution_replay_items: inspection.evolution_ledger.replay_items,
            evolution_router_threshold_mutations: inspection
                .evolution_ledger
                .router_threshold_mutations,
            evolution_hierarchy_weight_mutations: inspection
                .evolution_ledger
                .hierarchy_weight_mutations,
            evolution_memory_updates: inspection.evolution_ledger.memory_updates(),
            evolution_replay_live_memory_feedback_updates: inspection
                .evolution_ledger
                .replay_live_memory_feedback_updates(),
            evolution_replay_live_memory_feedback_detail_items: inspection
                .evolution_ledger
                .replay_live_memory_feedback_detail_items,
            evolution_replay_live_memory_feedback_applied: inspection
                .evolution_ledger
                .replay_live_memory_feedback_applied,
            evolution_replay_live_memory_feedback_removed: inspection
                .evolution_ledger
                .replay_live_memory_feedback_removed,
            evolution_replay_live_memory_feedback_missing: inspection
                .evolution_ledger
                .replay_live_memory_feedback_missing,
            evolution_replay_live_memory_feedback_strength_delta: inspection
                .evolution_ledger
                .replay_live_memory_feedback_strength_delta,
            evolution_replay_live_evolution_items: inspection
                .evolution_ledger
                .replay_live_evolution_items,
            evolution_replay_live_evolution_online_reward_feedbacks: inspection
                .evolution_ledger
                .replay_live_evolution_online_reward_feedbacks,
            evolution_replay_live_evolution_online_reward_reinforcements: inspection
                .evolution_ledger
                .replay_live_evolution_online_reward_reinforcements,
            evolution_replay_live_evolution_online_reward_penalties: inspection
                .evolution_ledger
                .replay_live_evolution_online_reward_penalties,
            evolution_replay_live_evolution_online_reward_strength: inspection
                .evolution_ledger
                .replay_live_evolution_online_reward_strength,
            evolution_replay_live_evolution_online_reward_reinforcement_strength: inspection
                .evolution_ledger
                .replay_live_evolution_online_reward_reinforcement_strength,
            evolution_replay_live_evolution_online_reward_penalty_strength: inspection
                .evolution_ledger
                .replay_live_evolution_online_reward_penalty_strength,
            evolution_replay_live_evolution_memory_updates: inspection
                .evolution_ledger
                .replay_live_evolution_memory_updates,
            evolution_replay_live_evolution_stored_memory_updates: inspection
                .evolution_ledger
                .replay_live_evolution_stored_memory_updates,
            evolution_replay_live_evolution_reflection_issues: inspection
                .evolution_ledger
                .replay_live_evolution_reflection_issues,
            evolution_replay_live_evolution_critical_reflection_issues: inspection
                .evolution_ledger
                .replay_live_evolution_critical_reflection_issues,
            evolution_replay_live_evolution_revision_actions: inspection
                .evolution_ledger
                .replay_live_evolution_revision_actions,
            evolution_recursive_replay_items: inspection.evolution_ledger.recursive_replay_items,
            evolution_recursive_runtime_calls: inspection.evolution_ledger.recursive_runtime_calls,
        }
    }
}
