use super::super::super::shared::require_usize_at_least;
use super::super::context::AutoReplayTrace;

pub(super) fn require_recursive(failures: &mut Vec<String>, trace: &AutoReplayTrace) {
    require_usize_at_least(
        failures,
        "cumulative_recursive_replay_items",
        trace.cumulative.recursive_replay_items,
        "auto_replay recursive_runtime_items",
        trace.recursive_runtime.items,
    );
    require_usize_at_least(
        failures,
        "cumulative_recursive_runtime_calls",
        trace.cumulative.recursive_runtime_calls,
        "auto_replay recursive_runtime_calls",
        trace.recursive_runtime.calls,
    );
}
