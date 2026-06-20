use crate::gemma_business::model_service_smoke::evidence::field;

pub(super) fn applied(body: &str) -> u64 {
    field(body, "applied")
}

pub(super) fn live_memory_feedback_updates(body: &str) -> u64 {
    field(body, "live_memory_feedback_updates")
}

pub(super) fn live_memory_feedback_applied(body: &str) -> u64 {
    field(body, "live_memory_feedback_applied")
}

pub(super) fn live_evolution_items(body: &str) -> u64 {
    field(body, "live_evolution_items")
}

pub(super) fn live_evolution_online_reward_feedbacks(body: &str) -> u64 {
    field(body, "live_evolution_online_reward_feedbacks")
}

pub(super) fn rust_check_items(body: &str) -> u64 {
    field(body, "rust_check_items")
}

pub(super) fn rust_check_passed(body: &str) -> u64 {
    field(body, "rust_check_passed")
}

pub(super) fn rust_check_feedback_updates(body: &str) -> u64 {
    field(body, "rust_check_live_memory_feedback_updates")
}

pub(super) fn rust_check_feedback_applied(body: &str) -> u64 {
    field(body, "rust_check_live_memory_feedback_applied")
}
