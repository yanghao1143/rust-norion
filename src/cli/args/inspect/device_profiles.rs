use super::InspectFlagParse;
use crate::cli::args::values::parse_usize;

pub(crate) fn parse(
    parser: &mut InspectFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--inspect-min-runtime-kv-memory-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_memory_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-model-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_model_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-adapter-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_adapter_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-forward-energy-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_forward_energy_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-influence-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_influence_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-uncertainty-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_uncertainty_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-uncertainty-token-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_uncertainty_token_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-precision-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_precision_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-device-execution-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_device_execution_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-layer-mode-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_layer_mode_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-all-layer-mode-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_all_layer_mode_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-import-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_import_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-export-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_export_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-hold-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_hold_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-reflection-issue-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_reflection_issue_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-critical-reflection-issue-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_critical_reflection_issue_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-revision-action-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_revision_action_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-live-memory-feedback-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_live_memory_feedback_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-live-inference-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_evolution_live_inference_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-live-router-threshold-mutation-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_live_router_threshold_mutation_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-live-hierarchy-weight-mutation-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_live_hierarchy_weight_mutation_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-live-online-reward-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_evolution_live_online_reward_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-live-online-reward-strength-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_live_online_reward_strength_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-live-memory-update-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_evolution_live_memory_update_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-live-stored-memory-update-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_live_stored_memory_update_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-live-reflection-issue-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_live_reflection_issue_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-live-critical-reflection-issue-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_live_critical_reflection_issue_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-live-revision-action-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_evolution_live_revision_action_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-replay-run-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_evolution_replay_run_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-replay-item-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_evolution_replay_item_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-router-threshold-mutation-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_router_threshold_mutation_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-hierarchy-weight-mutation-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_hierarchy_weight_mutation_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-memory-update-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_evolution_memory_update_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-replay-live-memory-feedback-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_replay_live_memory_feedback_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-replay-live-memory-feedback-detail-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_replay_live_memory_feedback_detail_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-replay-live-evolution-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_replay_live_evolution_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-replay-live-evolution-online-reward-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_replay_live_evolution_online_reward_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-replay-live-evolution-online-reward-strength-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_replay_live_evolution_online_reward_strength_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-replay-live-evolution-memory-update-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_replay_live_evolution_memory_update_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-replay-live-evolution-critical-reflection-issue-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-replay-live-evolution-revision-action-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_replay_live_evolution_revision_action_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-recursive-replay-device-profiles" if index + 1 < raw.len() => {
            *parser.inspect_min_evolution_recursive_replay_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--inspect-min-evolution-recursive-runtime-call-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_min_evolution_recursive_runtime_call_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        _ => None,
    }
}
