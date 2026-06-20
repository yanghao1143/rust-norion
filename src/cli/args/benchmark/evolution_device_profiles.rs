use super::BenchmarkFlagParse;
use crate::cli::args::values::parse_usize;

pub(crate) fn parse(
    parser: &mut BenchmarkFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--benchmark-min-evolution-live-inference-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_inference_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-router-threshold-mutation-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_live_router_threshold_mutation_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-hierarchy-weight-mutation-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_live_hierarchy_weight_mutation_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-online-reward-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_online_reward_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-online-reward-strength-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_live_online_reward_strength_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-memory-update-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_memory_update_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-stored-memory-update-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_live_stored_memory_update_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-reflection-issue-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_live_reflection_issue_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-critical-reflection-issue-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_live_critical_reflection_issue_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-revision-action-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_live_revision_action_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-online-reward-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_online_reward_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-online-reward-strength-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_online_reward_strength_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-memory-update-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_memory_update_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-critical-reflection-issue-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-revision-action-device-profiles"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_revision_action_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        _ => None,
    }
}
