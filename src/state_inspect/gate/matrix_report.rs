use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct StateInspectionMatrixGateReport {
    pub passed: bool,
    pub device_reports: Vec<StateInspectionDeviceGateReport>,
    pub failures: Vec<String>,
}

impl StateInspectionMatrixGateReport {
    pub fn evaluate(device_reports: Vec<StateInspectionDeviceGateReport>) -> Self {
        Self::evaluate_with_gate(device_reports, &StateInspectionMatrixGate::default())
    }

    pub fn evaluate_with_gate(
        device_reports: Vec<StateInspectionDeviceGateReport>,
        gate: &StateInspectionMatrixGate,
    ) -> Self {
        let mut failures = Vec::new();

        if device_reports.is_empty() {
            failures.push("no state inspection device reports were recorded".to_owned());
        }

        let missing = missing_state_inspection_devices(&device_reports);
        if !missing.is_empty() {
            failures.push(format!(
                "state_inspection_devices {} below expected {} missing={}",
                explicit_state_inspection_devices(&device_reports),
                DeviceClass::explicit_profiles().len(),
                missing
                    .iter()
                    .map(|device| device.as_str())
                    .collect::<Vec<_>>()
                    .join("+")
            ));
        }

        for device_report in &device_reports {
            if !device_report.report.passed() {
                failures.push(format!(
                    "device {} state inspection failed with {} failures",
                    device_report.device.as_str(),
                    device_report.report.failures.len()
                ));
            }
        }

        require_min_device_profiles(
            &mut failures,
            "runtime_kv_memory_device_profiles",
            runtime_kv_memory_device_profiles(&device_reports),
            gate.min_runtime_kv_memory_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_model_device_profiles",
            runtime_model_device_profiles(&device_reports),
            gate.min_runtime_model_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_adapter_device_profiles",
            runtime_adapter_device_profiles(&device_reports),
            gate.min_runtime_adapter_device_profiles,
        );
        require_max_usize(
            &mut failures,
            "runtime_adapter_selection_mismatches",
            runtime_adapter_selection_mismatches(&device_reports),
            gate.max_runtime_adapter_selection_mismatches,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_forward_energy_device_profiles",
            runtime_forward_energy_device_profiles(&device_reports),
            gate.min_runtime_forward_energy_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_influence_device_profiles",
            runtime_kv_influence_device_profiles(&device_reports),
            gate.min_runtime_kv_influence_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_uncertainty_device_profiles",
            runtime_uncertainty_device_profiles(&device_reports),
            gate.min_runtime_uncertainty_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_uncertainty_token_device_profiles",
            runtime_uncertainty_token_device_profiles(&device_reports),
            gate.min_runtime_uncertainty_token_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_precision_device_profiles",
            runtime_kv_precision_device_profiles(&device_reports),
            gate.min_runtime_kv_precision_device_profiles,
        );
        require_max_usize(
            &mut failures,
            "runtime_kv_precision_mismatches",
            runtime_kv_precision_mismatches(&device_reports),
            gate.max_runtime_kv_precision_mismatches,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_device_execution_device_profiles",
            runtime_device_execution_device_profiles(&device_reports),
            gate.min_runtime_device_execution_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_layer_mode_device_profiles",
            runtime_layer_mode_device_profiles(&device_reports),
            gate.min_runtime_layer_mode_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_all_layer_mode_device_profiles",
            runtime_all_layer_mode_device_profiles(&device_reports),
            gate.min_runtime_all_layer_mode_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_import_device_profiles",
            runtime_kv_import_device_profiles(&device_reports),
            gate.min_runtime_kv_import_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_weak_import_skip_device_profiles",
            runtime_kv_weak_import_skip_device_profiles(&device_reports),
            gate.min_runtime_kv_weak_import_skip_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_budget_import_skip_device_profiles",
            runtime_kv_budget_import_skip_device_profiles(&device_reports),
            gate.min_runtime_kv_budget_import_skip_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_export_device_profiles",
            runtime_kv_export_device_profiles(&device_reports),
            gate.min_runtime_kv_export_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_segment_device_profiles",
            runtime_kv_segment_device_profiles(&device_reports),
            gate.min_runtime_kv_segment_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_hold_device_profiles",
            runtime_kv_hold_device_profiles(&device_reports),
            gate.min_runtime_kv_hold_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "reflection_issue_device_profiles",
            reflection_issue_device_profiles(&device_reports),
            gate.min_reflection_issue_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "critical_reflection_issue_device_profiles",
            critical_reflection_issue_device_profiles(&device_reports),
            gate.min_critical_reflection_issue_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "revision_action_device_profiles",
            revision_action_device_profiles(&device_reports),
            gate.min_revision_action_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "live_memory_feedback_device_profiles",
            live_memory_feedback_device_profiles(&device_reports),
            gate.min_live_memory_feedback_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_inference_device_profiles",
            evolution_live_inference_device_profiles(&device_reports),
            gate.min_evolution_live_inference_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_router_threshold_mutation_device_profiles",
            evolution_live_router_threshold_mutation_device_profiles(&device_reports),
            gate.min_evolution_live_router_threshold_mutation_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_hierarchy_weight_mutation_device_profiles",
            evolution_live_hierarchy_weight_mutation_device_profiles(&device_reports),
            gate.min_evolution_live_hierarchy_weight_mutation_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_online_reward_device_profiles",
            evolution_live_online_reward_device_profiles(&device_reports),
            gate.min_evolution_live_online_reward_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_online_reward_strength_device_profiles",
            evolution_live_online_reward_strength_device_profiles(&device_reports),
            gate.min_evolution_live_online_reward_strength_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_memory_update_device_profiles",
            evolution_live_memory_update_device_profiles(&device_reports),
            gate.min_evolution_live_memory_update_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_stored_memory_update_device_profiles",
            evolution_live_stored_memory_update_device_profiles(&device_reports),
            gate.min_evolution_live_stored_memory_update_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_reflection_issue_device_profiles",
            evolution_live_reflection_issue_device_profiles(&device_reports),
            gate.min_evolution_live_reflection_issue_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_critical_reflection_issue_device_profiles",
            evolution_live_critical_reflection_issue_device_profiles(&device_reports),
            gate.min_evolution_live_critical_reflection_issue_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_revision_action_device_profiles",
            evolution_live_revision_action_device_profiles(&device_reports),
            gate.min_evolution_live_revision_action_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_run_device_profiles",
            evolution_replay_run_device_profiles(&device_reports),
            gate.min_evolution_replay_run_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_item_device_profiles",
            evolution_replay_item_device_profiles(&device_reports),
            gate.min_evolution_replay_item_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_router_threshold_mutation_device_profiles",
            evolution_router_threshold_mutation_device_profiles(&device_reports),
            gate.min_evolution_router_threshold_mutation_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_hierarchy_weight_mutation_device_profiles",
            evolution_hierarchy_weight_mutation_device_profiles(&device_reports),
            gate.min_evolution_hierarchy_weight_mutation_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_memory_update_device_profiles",
            evolution_memory_update_device_profiles(&device_reports),
            gate.min_evolution_memory_update_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_memory_feedback_device_profiles",
            evolution_replay_live_memory_feedback_device_profiles(&device_reports),
            gate.min_evolution_replay_live_memory_feedback_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_memory_feedback_detail_device_profiles",
            evolution_replay_live_memory_feedback_detail_device_profiles(&device_reports),
            gate.min_evolution_replay_live_memory_feedback_detail_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_evolution_device_profiles",
            evolution_replay_live_evolution_device_profiles(&device_reports),
            gate.min_evolution_replay_live_evolution_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_evolution_online_reward_device_profiles",
            evolution_replay_live_evolution_online_reward_device_profiles(&device_reports),
            gate.min_evolution_replay_live_evolution_online_reward_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_evolution_online_reward_strength_device_profiles",
            evolution_replay_live_evolution_online_reward_strength_device_profiles(&device_reports),
            gate.min_evolution_replay_live_evolution_online_reward_strength_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_evolution_memory_update_device_profiles",
            evolution_replay_live_evolution_memory_update_device_profiles(&device_reports),
            gate.min_evolution_replay_live_evolution_memory_update_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_evolution_critical_reflection_issue_device_profiles",
            evolution_replay_live_evolution_critical_reflection_issue_device_profiles(
                &device_reports,
            ),
            gate.min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_evolution_revision_action_device_profiles",
            evolution_replay_live_evolution_revision_action_device_profiles(&device_reports),
            gate.min_evolution_replay_live_evolution_revision_action_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_recursive_replay_device_profiles",
            evolution_recursive_replay_device_profiles(&device_reports),
            gate.min_evolution_recursive_replay_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_recursive_runtime_call_device_profiles",
            evolution_recursive_runtime_call_device_profiles(&device_reports),
            gate.min_evolution_recursive_runtime_call_device_profiles,
        );

        Self {
            passed: failures.is_empty(),
            device_reports,
            failures,
        }
    }

    pub fn passed(&self) -> bool {
        self.passed
    }

    pub fn covered_devices(&self) -> usize {
        explicit_state_inspection_devices(&self.device_reports)
    }

    pub fn missing_devices(&self) -> Vec<DeviceClass> {
        missing_state_inspection_devices(&self.device_reports)
    }

    pub fn failed_devices(&self) -> Vec<DeviceClass> {
        self.device_reports
            .iter()
            .filter(|device_report| !device_report.report.passed())
            .map(|device_report| device_report.device)
            .collect()
    }
}
