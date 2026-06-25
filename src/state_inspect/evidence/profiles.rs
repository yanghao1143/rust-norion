use super::super::StateInspectionDeviceGateReport;
use super::coverage::explicit_state_inspection_evidence_devices;
use super::reward::online_reward_strength_is_consistent;

pub(in crate::state_inspect) fn runtime_kv_memory_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_memories > 0
    })
}

pub(in crate::state_inspect) fn runtime_model_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_model_experiences > 0
    })
}

pub(in crate::state_inspect) fn runtime_adapter_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_adapter_experiences > 0
    })
}

pub(in crate::state_inspect) fn runtime_adapter_selection_mismatches(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    device_reports
        .iter()
        .map(|device_report| device_report.runtime_adapter_selection_mismatches)
        .sum()
}

pub(in crate::state_inspect) fn runtime_forward_energy_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_forward_energy_experiences > 0
    })
}

pub(in crate::state_inspect) fn runtime_kv_influence_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_influence_experiences > 0
    })
}

pub(in crate::state_inspect) fn runtime_uncertainty_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_uncertainty_experiences > 0
    })
}

pub(in crate::state_inspect) fn runtime_uncertainty_token_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_uncertainty_tokens > 0
    })
}

pub(in crate::state_inspect) fn runtime_kv_precision_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_precision_experiences > 0
    })
}

pub(in crate::state_inspect) fn runtime_kv_precision_mismatches(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    device_reports
        .iter()
        .map(|device_report| device_report.runtime_kv_precision_mismatches)
        .sum()
}

pub(in crate::state_inspect) fn runtime_device_execution_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_device_execution_experiences > 0
    })
}

pub(in crate::state_inspect) fn runtime_layer_mode_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_layer_mode_experiences > 0
    })
}

pub(in crate::state_inspect) fn runtime_all_layer_mode_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_all_layer_mode_experiences > 0
    })
}

pub(in crate::state_inspect) fn runtime_kv_import_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_import_experiences > 0
    })
}

pub(in crate::state_inspect) fn runtime_kv_weak_import_skip_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_weak_import_skip_experiences > 0
            || device_report.weak_runtime_kv_imports_skipped > 0
    })
}

pub(in crate::state_inspect) fn runtime_kv_export_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_export_experiences > 0
    })
}

pub(in crate::state_inspect) fn runtime_kv_segment_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_segment_experiences > 0
            || device_report
                .runtime_kv_segments_included
                .saturating_add(device_report.runtime_kv_segments_skipped)
                .saturating_add(device_report.runtime_kv_segments_rejected)
                > 0
    })
}

pub(in crate::state_inspect) fn runtime_kv_hold_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_hold_experiences > 0 || device_report.runtime_kv_held_blocks > 0
    })
}

pub(in crate::state_inspect) fn reflection_issue_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.reflection_issue_experiences > 0
    })
}

pub(in crate::state_inspect) fn critical_reflection_issue_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.critical_reflection_issue_experiences > 0
    })
}

pub(in crate::state_inspect) fn revision_action_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.revision_action_experiences > 0
    })
}

pub(in crate::state_inspect) fn live_memory_feedback_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.live_memory_feedback_experiences > 0
            && device_report.live_memory_feedback_updates > 0
            && device_report.live_memory_feedback_detail_experiences > 0
            && device_report
                .live_memory_feedback_applied
                .saturating_add(device_report.live_memory_feedback_missing)
                == device_report.live_memory_feedback_updates
            && device_report.live_memory_feedback_removed
                <= device_report.live_memory_feedback_applied
            && device_report
                .live_memory_feedback_strength_delta
                .is_finite()
            && device_report.live_memory_feedback_strength_delta >= 0.0
    })
}

pub(in crate::state_inspect) fn evolution_live_inference_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_inference_runs > 0
    })
}

pub(in crate::state_inspect) fn evolution_live_router_threshold_mutation_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_router_threshold_mutations > 0
    })
}

pub(in crate::state_inspect) fn evolution_live_hierarchy_weight_mutation_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_hierarchy_weight_mutations > 0
    })
}

pub(in crate::state_inspect) fn evolution_live_online_reward_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_online_reward_feedbacks > 0
            && device_report.evolution_live_online_reward_feedbacks
                == device_report
                    .evolution_live_online_reward_reinforcements
                    .saturating_add(device_report.evolution_live_online_reward_penalties)
    })
}

pub(in crate::state_inspect) fn evolution_live_online_reward_strength_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        online_reward_strength_is_consistent(
            device_report.evolution_live_online_reward_feedbacks,
            device_report.evolution_live_online_reward_reinforcements,
            device_report.evolution_live_online_reward_penalties,
            device_report.evolution_live_online_reward_strength,
            device_report.evolution_live_online_reward_reinforcement_strength,
            device_report.evolution_live_online_reward_penalty_strength,
        )
    })
}

pub(in crate::state_inspect) fn evolution_live_memory_update_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_memory_updates > 0
    })
}

pub(in crate::state_inspect) fn evolution_live_stored_memory_update_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_stored_memory_updates > 0
    })
}

pub(in crate::state_inspect) fn evolution_live_reflection_issue_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_reflection_issues > 0
    })
}

pub(in crate::state_inspect) fn evolution_live_critical_reflection_issue_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_critical_reflection_issues > 0
    })
}

pub(in crate::state_inspect) fn evolution_live_revision_action_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_revision_actions > 0
    })
}

pub(in crate::state_inspect) fn evolution_replay_run_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_runs > 0
    })
}

pub(in crate::state_inspect) fn evolution_replay_item_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_items > 0
    })
}

pub(in crate::state_inspect) fn evolution_router_threshold_mutation_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_router_threshold_mutations > 0
    })
}

pub(in crate::state_inspect) fn evolution_hierarchy_weight_mutation_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_hierarchy_weight_mutations > 0
    })
}

pub(in crate::state_inspect) fn evolution_memory_update_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_memory_updates > 0
    })
}

pub(in crate::state_inspect) fn evolution_replay_live_memory_feedback_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_memory_feedback_updates > 0
    })
}

pub(in crate::state_inspect) fn evolution_replay_live_memory_feedback_detail_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_memory_feedback_detail_items > 0
            && device_report
                .evolution_replay_live_memory_feedback_applied
                .saturating_add(device_report.evolution_replay_live_memory_feedback_missing)
                <= device_report.evolution_replay_live_memory_feedback_updates
            && device_report.evolution_replay_live_memory_feedback_removed
                <= device_report.evolution_replay_live_memory_feedback_applied
            && device_report
                .evolution_replay_live_memory_feedback_strength_delta
                .is_finite()
            && device_report.evolution_replay_live_memory_feedback_strength_delta >= 0.0
    })
}

pub(in crate::state_inspect) fn evolution_replay_live_evolution_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_evolution_items > 0
    })
}

pub(in crate::state_inspect) fn evolution_replay_live_evolution_online_reward_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_evolution_online_reward_feedbacks > 0
            && device_report.evolution_replay_live_evolution_online_reward_feedbacks
                == device_report
                    .evolution_replay_live_evolution_online_reward_reinforcements
                    .saturating_add(
                        device_report.evolution_replay_live_evolution_online_reward_penalties,
                    )
    })
}

pub(in crate::state_inspect) fn evolution_replay_live_evolution_online_reward_strength_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        online_reward_strength_is_consistent(
            device_report.evolution_replay_live_evolution_online_reward_feedbacks,
            device_report.evolution_replay_live_evolution_online_reward_reinforcements,
            device_report.evolution_replay_live_evolution_online_reward_penalties,
            device_report.evolution_replay_live_evolution_online_reward_strength,
            device_report.evolution_replay_live_evolution_online_reward_reinforcement_strength,
            device_report.evolution_replay_live_evolution_online_reward_penalty_strength,
        )
    })
}

pub(in crate::state_inspect) fn evolution_replay_live_evolution_memory_update_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_evolution_memory_updates > 0
    })
}

pub(in crate::state_inspect) fn evolution_replay_live_evolution_critical_reflection_issue_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_evolution_critical_reflection_issues > 0
    })
}

pub(in crate::state_inspect) fn evolution_replay_live_evolution_revision_action_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_evolution_revision_actions > 0
    })
}

pub(in crate::state_inspect) fn evolution_recursive_replay_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_recursive_replay_items > 0
    })
}

pub(in crate::state_inspect) fn evolution_recursive_runtime_call_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_recursive_runtime_calls > 0
    })
}
