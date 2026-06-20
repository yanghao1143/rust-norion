use super::super::super::shared::{require_f32_at_least, require_usize_at_least};
use super::super::context::AutoReplayTrace;

pub(super) fn require_current_counters(
    failures: &mut Vec<String>,
    trace: &AutoReplayTrace,
    expected_memory_updates: usize,
) {
    let cumulative = &trace.cumulative;

    require_usize_at_least(
        failures,
        "cumulative_router_threshold_mutations",
        cumulative.router_threshold_mutations,
        "auto_replay router_threshold_mutations",
        trace.router_threshold_mutations,
    );
    require_usize_at_least(
        failures,
        "cumulative_hierarchy_weight_mutations",
        cumulative.hierarchy_weight_mutations,
        "auto_replay hierarchy_weight_mutations",
        trace.hierarchy_weight_mutations,
    );
    require_f32_at_least(
        failures,
        "cumulative_router_threshold_delta",
        cumulative.router_threshold_delta,
        "auto_replay router_threshold_delta",
        trace.router_threshold_delta,
    );
    require_f32_at_least(
        failures,
        "cumulative_hierarchy_weight_delta",
        cumulative.hierarchy_weight_delta,
        "auto_replay hierarchy_weight_delta",
        trace.hierarchy_weight_delta,
    );
    require_usize_at_least(
        failures,
        "cumulative_memory_reinforcements",
        cumulative.memory_reinforcements,
        "auto_replay memory_reinforcements",
        trace.memory_reinforcements,
    );
    require_usize_at_least(
        failures,
        "cumulative_memory_penalties",
        cumulative.memory_penalties,
        "auto_replay memory_penalties",
        trace.memory_penalties,
    );
    require_usize_at_least(
        failures,
        "cumulative_memory_updates",
        cumulative.memory_updates,
        "auto_replay memory updates",
        expected_memory_updates,
    );
}
