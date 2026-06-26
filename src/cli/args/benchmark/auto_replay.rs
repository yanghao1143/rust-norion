use super::BenchmarkFlagParse;
use crate::cli::args::values::{parse_f32, parse_usize};

pub(crate) fn parse(
    parser: &mut BenchmarkFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--benchmark-min-auto-replay-router-updates" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_router_updates =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-hierarchy-updates" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_hierarchy_updates =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-router-threshold-mutations" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_router_threshold_mutations =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-hierarchy-weight-mutations" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_hierarchy_weight_mutations =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-router-threshold-delta" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_router_threshold_delta =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-hierarchy-weight-delta" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_hierarchy_weight_delta =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-memory-updates" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_memory_updates =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-live-memory-feedback-updates" if index + 1 < raw.len() => {
            *parser.benchmark_min_live_memory_feedback_updates =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-live-memory-feedback-updates" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_live_memory_feedback_updates =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-live-memory-feedback-detail-items"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_auto_replay_live_memory_feedback_detail_items =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-live-memory-feedback-applied" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_live_memory_feedback_applied =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-live-memory-feedback-strength-delta"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_auto_replay_live_memory_feedback_strength_delta =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-recursive-items" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_recursive_items =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-recursive-call-pressure" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_recursive_call_pressure =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-auto-replay-recursive-call-pressure" if index + 1 < raw.len() => {
            *parser.benchmark_max_auto_replay_recursive_call_pressure =
                Some(parse_f32(&raw[index + 1], 1.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-runtime-kv-budget-pressure-items" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_runtime_kv_budget_pressure_items =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-auto-replay-runtime-kv-budget-pressure" if index + 1 < raw.len() => {
            *parser.benchmark_min_auto_replay_runtime_kv_budget_pressure =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-auto-replay-runtime-kv-budget-pressure" if index + 1 < raw.len() => {
            *parser.benchmark_max_auto_replay_runtime_kv_budget_pressure =
                Some(parse_f32(&raw[index + 1], 1.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        _ => None,
    }
}
