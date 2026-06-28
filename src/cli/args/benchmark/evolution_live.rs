use super::BenchmarkFlagParse;
use crate::cli::args::values::{parse_f32, parse_u64};

pub(crate) fn parse(
    parser: &mut BenchmarkFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--benchmark-min-evolution-live-inference-runs" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_inference_runs =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-router-threshold-mutations" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_router_threshold_mutations =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-hierarchy-weight-mutations" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_hierarchy_weight_mutations =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-router-threshold-delta" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_router_threshold_delta =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-hierarchy-weight-delta" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_hierarchy_weight_delta =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-online-reward-feedbacks" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_online_reward_feedbacks =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-online-reward-reinforcements" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_online_reward_reinforcements =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-online-reward-penalties" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_online_reward_penalties =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-online-reward-strength" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_online_reward_strength =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-online-reward-reinforcement-strength"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_live_online_reward_reinforcement_strength =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-online-reward-penalty-strength"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_live_online_reward_penalty_strength =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-memory-reinforcements" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_memory_reinforcements =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-memory-penalties" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_memory_penalties =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-stored-memories" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_stored_memories =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-stored-gist-memories" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_stored_gist_memories =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-stored-runtime-kv-memories" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_stored_runtime_kv_memories =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-memory-updates" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_memory_updates =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-stored-memory-updates" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_stored_memory_updates =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-reflection-issues" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_reflection_issues =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-critical-reflection-issues" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_critical_reflection_issues =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-live-revision-actions" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_live_revision_actions =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        _ => None,
    }
}
