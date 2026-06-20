use crate::gemma_business::response_metrics::{
    cycle_external_feedbacks, cycle_feedback_memory_updates, cycle_replay_rust_check_passed,
    live_evolution_items as cycle_live_evolution_items,
    live_memory_feedback_applied as cycle_live_memory_feedback_applied,
    runtime_tokens as cycle_runtime_tokens,
};

pub(super) fn runtime_tokens(body: &str) -> u64 {
    cycle_runtime_tokens(body)
}

pub(super) fn external_feedbacks(body: &str) -> u64 {
    cycle_external_feedbacks(body)
}

pub(super) fn feedback_memory_updates(body: &str) -> u64 {
    cycle_feedback_memory_updates(body)
}

pub(super) fn replay_rust_check_passed(body: &str) -> u64 {
    cycle_replay_rust_check_passed(body)
}

pub(super) fn live_memory_feedback_applied(body: &str) -> u64 {
    cycle_live_memory_feedback_applied(body)
}

pub(super) fn live_evolution_items(body: &str) -> u64 {
    cycle_live_evolution_items(body)
}
