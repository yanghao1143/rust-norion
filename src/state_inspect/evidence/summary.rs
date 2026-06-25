use crate::hardware::DeviceClass;

use super::super::StateInspectionMatrixGateReport;
use super::profiles::*;

impl StateInspectionMatrixGateReport {
    pub fn runtime_kv_memory_device_profiles(&self) -> usize {
        runtime_kv_memory_device_profiles(&self.device_reports)
    }

    pub fn runtime_model_device_profiles(&self) -> usize {
        runtime_model_device_profiles(&self.device_reports)
    }

    pub fn runtime_adapter_device_profiles(&self) -> usize {
        runtime_adapter_device_profiles(&self.device_reports)
    }

    pub fn runtime_adapter_selection_mismatches(&self) -> usize {
        runtime_adapter_selection_mismatches(&self.device_reports)
    }

    pub fn runtime_forward_energy_device_profiles(&self) -> usize {
        runtime_forward_energy_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_influence_device_profiles(&self) -> usize {
        runtime_kv_influence_device_profiles(&self.device_reports)
    }

    pub fn runtime_uncertainty_device_profiles(&self) -> usize {
        runtime_uncertainty_device_profiles(&self.device_reports)
    }

    pub fn runtime_uncertainty_token_device_profiles(&self) -> usize {
        runtime_uncertainty_token_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_precision_device_profiles(&self) -> usize {
        runtime_kv_precision_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_precision_mismatches(&self) -> usize {
        runtime_kv_precision_mismatches(&self.device_reports)
    }

    pub fn runtime_device_execution_device_profiles(&self) -> usize {
        runtime_device_execution_device_profiles(&self.device_reports)
    }

    pub fn runtime_layer_mode_device_profiles(&self) -> usize {
        runtime_layer_mode_device_profiles(&self.device_reports)
    }

    pub fn runtime_all_layer_mode_device_profiles(&self) -> usize {
        runtime_all_layer_mode_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_import_device_profiles(&self) -> usize {
        runtime_kv_import_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_weak_import_skip_device_profiles(&self) -> usize {
        runtime_kv_weak_import_skip_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_export_device_profiles(&self) -> usize {
        runtime_kv_export_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_segment_device_profiles(&self) -> usize {
        runtime_kv_segment_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_hold_device_profiles(&self) -> usize {
        runtime_kv_hold_device_profiles(&self.device_reports)
    }

    pub fn reflection_issue_device_profiles(&self) -> usize {
        reflection_issue_device_profiles(&self.device_reports)
    }

    pub fn critical_reflection_issue_device_profiles(&self) -> usize {
        critical_reflection_issue_device_profiles(&self.device_reports)
    }

    pub fn revision_action_device_profiles(&self) -> usize {
        revision_action_device_profiles(&self.device_reports)
    }

    pub fn live_memory_feedback_device_profiles(&self) -> usize {
        live_memory_feedback_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_inference_device_profiles(&self) -> usize {
        evolution_live_inference_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_router_threshold_mutation_device_profiles(&self) -> usize {
        evolution_live_router_threshold_mutation_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_hierarchy_weight_mutation_device_profiles(&self) -> usize {
        evolution_live_hierarchy_weight_mutation_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_online_reward_device_profiles(&self) -> usize {
        evolution_live_online_reward_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_online_reward_strength_device_profiles(&self) -> usize {
        evolution_live_online_reward_strength_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_memory_update_device_profiles(&self) -> usize {
        evolution_live_memory_update_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_stored_memory_update_device_profiles(&self) -> usize {
        evolution_live_stored_memory_update_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_reflection_issue_device_profiles(&self) -> usize {
        evolution_live_reflection_issue_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_critical_reflection_issue_device_profiles(&self) -> usize {
        evolution_live_critical_reflection_issue_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_revision_action_device_profiles(&self) -> usize {
        evolution_live_revision_action_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_run_device_profiles(&self) -> usize {
        evolution_replay_run_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_item_device_profiles(&self) -> usize {
        evolution_replay_item_device_profiles(&self.device_reports)
    }

    pub fn evolution_router_threshold_mutation_device_profiles(&self) -> usize {
        evolution_router_threshold_mutation_device_profiles(&self.device_reports)
    }

    pub fn evolution_hierarchy_weight_mutation_device_profiles(&self) -> usize {
        evolution_hierarchy_weight_mutation_device_profiles(&self.device_reports)
    }

    pub fn evolution_memory_update_device_profiles(&self) -> usize {
        evolution_memory_update_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_memory_feedback_device_profiles(&self) -> usize {
        evolution_replay_live_memory_feedback_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_memory_feedback_detail_device_profiles(&self) -> usize {
        evolution_replay_live_memory_feedback_detail_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_evolution_device_profiles(&self) -> usize {
        evolution_replay_live_evolution_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_evolution_online_reward_device_profiles(&self) -> usize {
        evolution_replay_live_evolution_online_reward_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_evolution_online_reward_strength_device_profiles(&self) -> usize {
        evolution_replay_live_evolution_online_reward_strength_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_evolution_memory_update_device_profiles(&self) -> usize {
        evolution_replay_live_evolution_memory_update_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_evolution_critical_reflection_issue_device_profiles(
        &self,
    ) -> usize {
        evolution_replay_live_evolution_critical_reflection_issue_device_profiles(
            &self.device_reports,
        )
    }

    pub fn evolution_replay_live_evolution_revision_action_device_profiles(&self) -> usize {
        evolution_replay_live_evolution_revision_action_device_profiles(&self.device_reports)
    }

    pub fn evolution_recursive_replay_device_profiles(&self) -> usize {
        evolution_recursive_replay_device_profiles(&self.device_reports)
    }

    pub fn evolution_recursive_runtime_call_device_profiles(&self) -> usize {
        evolution_recursive_runtime_call_device_profiles(&self.device_reports)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "state_inspection_matrix_gate: passed={} devices={} expected_devices={} failed_devices={} runtime_kv_memory_device_profiles={} runtime_model_device_profiles={} runtime_adapter_device_profiles={} runtime_adapter_selection_mismatches={} runtime_forward_energy_device_profiles={} runtime_kv_influence_device_profiles={} runtime_uncertainty_device_profiles={} runtime_uncertainty_token_device_profiles={} runtime_kv_precision_device_profiles={} runtime_kv_precision_mismatches={} runtime_device_execution_device_profiles={} runtime_layer_mode_device_profiles={} runtime_all_layer_mode_device_profiles={} runtime_kv_import_device_profiles={} runtime_kv_weak_import_skip_device_profiles={} runtime_kv_export_device_profiles={} runtime_kv_segment_device_profiles={} runtime_kv_hold_device_profiles={} reflection_issue_device_profiles={} critical_reflection_issue_device_profiles={} revision_action_device_profiles={} live_memory_feedback_device_profiles={} evolution_live_inference_device_profiles={} evolution_live_router_threshold_mutation_device_profiles={} evolution_live_hierarchy_weight_mutation_device_profiles={} evolution_live_online_reward_device_profiles={} evolution_live_online_reward_strength_device_profiles={} evolution_live_memory_update_device_profiles={} evolution_live_stored_memory_update_device_profiles={} evolution_live_reflection_issue_device_profiles={} evolution_live_critical_reflection_issue_device_profiles={} evolution_live_revision_action_device_profiles={} evolution_replay_run_device_profiles={} evolution_replay_item_device_profiles={} evolution_router_threshold_mutation_device_profiles={} evolution_hierarchy_weight_mutation_device_profiles={} evolution_memory_update_device_profiles={} evolution_replay_live_memory_feedback_device_profiles={} evolution_replay_live_memory_feedback_detail_device_profiles={} evolution_replay_live_evolution_device_profiles={} evolution_replay_live_evolution_online_reward_device_profiles={} evolution_replay_live_evolution_online_reward_strength_device_profiles={} evolution_replay_live_evolution_memory_update_device_profiles={} evolution_replay_live_evolution_critical_reflection_issue_device_profiles={} evolution_replay_live_evolution_revision_action_device_profiles={} evolution_recursive_replay_device_profiles={} evolution_recursive_runtime_call_device_profiles={} failures={}",
            self.passed,
            self.covered_devices(),
            DeviceClass::explicit_profiles().len(),
            self.failed_devices().len(),
            self.runtime_kv_memory_device_profiles(),
            self.runtime_model_device_profiles(),
            self.runtime_adapter_device_profiles(),
            self.runtime_adapter_selection_mismatches(),
            self.runtime_forward_energy_device_profiles(),
            self.runtime_kv_influence_device_profiles(),
            self.runtime_uncertainty_device_profiles(),
            self.runtime_uncertainty_token_device_profiles(),
            self.runtime_kv_precision_device_profiles(),
            self.runtime_kv_precision_mismatches(),
            self.runtime_device_execution_device_profiles(),
            self.runtime_layer_mode_device_profiles(),
            self.runtime_all_layer_mode_device_profiles(),
            self.runtime_kv_import_device_profiles(),
            self.runtime_kv_weak_import_skip_device_profiles(),
            self.runtime_kv_export_device_profiles(),
            self.runtime_kv_segment_device_profiles(),
            self.runtime_kv_hold_device_profiles(),
            self.reflection_issue_device_profiles(),
            self.critical_reflection_issue_device_profiles(),
            self.revision_action_device_profiles(),
            self.live_memory_feedback_device_profiles(),
            self.evolution_live_inference_device_profiles(),
            self.evolution_live_router_threshold_mutation_device_profiles(),
            self.evolution_live_hierarchy_weight_mutation_device_profiles(),
            self.evolution_live_online_reward_device_profiles(),
            self.evolution_live_online_reward_strength_device_profiles(),
            self.evolution_live_memory_update_device_profiles(),
            self.evolution_live_stored_memory_update_device_profiles(),
            self.evolution_live_reflection_issue_device_profiles(),
            self.evolution_live_critical_reflection_issue_device_profiles(),
            self.evolution_live_revision_action_device_profiles(),
            self.evolution_replay_run_device_profiles(),
            self.evolution_replay_item_device_profiles(),
            self.evolution_router_threshold_mutation_device_profiles(),
            self.evolution_hierarchy_weight_mutation_device_profiles(),
            self.evolution_memory_update_device_profiles(),
            self.evolution_replay_live_memory_feedback_device_profiles(),
            self.evolution_replay_live_memory_feedback_detail_device_profiles(),
            self.evolution_replay_live_evolution_device_profiles(),
            self.evolution_replay_live_evolution_online_reward_device_profiles(),
            self.evolution_replay_live_evolution_online_reward_strength_device_profiles(),
            self.evolution_replay_live_evolution_memory_update_device_profiles(),
            self.evolution_replay_live_evolution_critical_reflection_issue_device_profiles(),
            self.evolution_replay_live_evolution_revision_action_device_profiles(),
            self.evolution_recursive_replay_device_profiles(),
            self.evolution_recursive_runtime_call_device_profiles(),
            self.failures.len()
        )
    }
}
