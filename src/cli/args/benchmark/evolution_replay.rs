use super::BenchmarkFlagParse;
use crate::cli::args::values::{parse_f32, parse_u64};

pub(crate) fn parse(
    parser: &mut BenchmarkFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--benchmark-min-evolution-replay-runs" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_replay_runs = Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-items" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_replay_items = Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-router-threshold-mutations" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_router_threshold_mutations =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-hierarchy-weight-mutations" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_hierarchy_weight_mutations =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-router-threshold-delta" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_router_threshold_delta =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-hierarchy-weight-delta" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_hierarchy_weight_delta =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-memory-updates" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_memory_updates = Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-memory-feedback-updates"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_memory_feedback_updates =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-memory-feedback-detail-items"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_memory_feedback_detail_items =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-memory-feedback-applied"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_memory_feedback_applied =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-memory-feedback-strength-delta"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_memory_feedback_strength_delta =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-rust-check-items" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_replay_rust_check_items =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-rust-check-passed" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_replay_rust_check_passed =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-evolution-replay-rust-check-failed" if index + 1 < raw.len() => {
            *parser.benchmark_max_evolution_replay_rust_check_failed =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-rust-check-live-memory-feedback-updates"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_rust_check_live_memory_feedback_updates =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-rust-check-live-memory-feedback-applied"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_rust_check_live_memory_feedback_applied =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-rust-check-live-memory-feedback-strength-delta"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_rust_check_live_memory_feedback_strength_delta =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-items" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_replay_live_evolution_items =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-online-reward-feedbacks"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_online_reward_feedbacks =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-online-reward-reinforcements"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_online_reward_reinforcements =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-online-reward-penalties"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_online_reward_penalties =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-online-reward-strength"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_online_reward_strength =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-online-reward-reinforcement-strength"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_online_reward_reinforcement_strength =
                        Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-online-reward-penalty-strength"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_online_reward_penalty_strength =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-memory-updates"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_memory_updates =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-stored-memory-updates"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_stored_memory_updates =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-reflection-issues"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_reflection_issues =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-critical-reflection-issues"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_critical_reflection_issues =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-replay-live-evolution-revision-actions"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_evolution_replay_live_evolution_revision_actions =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-recursive-replay-items" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_recursive_replay_items =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-evolution-recursive-runtime-calls" if index + 1 < raw.len() => {
            *parser.benchmark_min_evolution_recursive_runtime_calls =
                Some(parse_u64(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-evolution-drift-rollbacks" if index + 1 < raw.len() => {
            *parser.benchmark_max_evolution_drift_rollbacks =
                Some(parse_u64(&raw[index + 1], u64::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-evolution-rollback-router-threshold-delta" if index + 1 < raw.len() => {
            *parser.benchmark_max_evolution_rollback_router_threshold_delta =
                Some(parse_f32(&raw[index + 1], f32::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-evolution-rollback-hierarchy-weight-delta" if index + 1 < raw.len() => {
            *parser.benchmark_max_evolution_rollback_hierarchy_weight_delta =
                Some(parse_f32(&raw[index + 1], f32::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        _ => None,
    }
}
