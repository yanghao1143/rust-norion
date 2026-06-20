use super::super::super::shared::{require_f32_at_least, require_usize_at_least};
use super::super::context::AutoReplayTrace;

pub(super) fn require_live_feedback(failures: &mut Vec<String>, trace: &AutoReplayTrace) {
    let current = &trace.live_memory_feedback;
    let cumulative = &trace.cumulative.replay_live_memory_feedback;

    require_usize_at_least(
        failures,
        "cumulative_replay_live_memory_feedback_items",
        cumulative.items,
        "auto_replay live_memory_feedback_items",
        current.items,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_memory_feedback_updates",
        cumulative.updates,
        "auto_replay live_memory_feedback_updates",
        current.updates,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_memory_feedback_reinforcements",
        cumulative.reinforcements,
        "auto_replay live_memory_feedback_reinforcements",
        current.reinforcements,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_memory_feedback_penalties",
        cumulative.penalties,
        "auto_replay live_memory_feedback_penalties",
        current.penalties,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_memory_feedback_detail_items",
        cumulative.detail_items,
        "auto_replay live_memory_feedback_detail_items",
        current.detail_items,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_memory_feedback_applied",
        cumulative.applied,
        "auto_replay live_memory_feedback_applied",
        current.applied,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_memory_feedback_removed",
        cumulative.removed,
        "auto_replay live_memory_feedback_removed",
        current.removed,
    );
    require_usize_at_least(
        failures,
        "cumulative_replay_live_memory_feedback_missing",
        cumulative.missing,
        "auto_replay live_memory_feedback_missing",
        current.missing,
    );
    require_f32_at_least(
        failures,
        "cumulative_replay_live_memory_feedback_strength_delta",
        cumulative.strength_delta,
        "auto_replay live_memory_feedback_strength_delta",
        current.strength_delta,
    );
}
