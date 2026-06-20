use crate::gemma_business::response_json::response_u64_field;

pub(in crate::gemma_business) fn cycle_external_feedbacks(body: &str) -> u64 {
    response_u64_field(body, "evolution_external_feedbacks")
}

pub(in crate::gemma_business) fn cycle_feedback_memory_updates(body: &str) -> u64 {
    response_u64_field(body, "evolution_external_feedback_memory_updates")
}

pub(in crate::gemma_business) fn cycle_replay_rust_check_passed(body: &str) -> u64 {
    response_u64_field(body, "evolution_replay_rust_check_passed")
}

pub(in crate::gemma_business) fn cycle_rust_check_passed(body: &str) -> u64 {
    response_u64_field(body, "rust_check_passed")
}

pub(in crate::gemma_business) fn live_memory_feedback_applied(body: &str) -> u64 {
    response_u64_field(body, "live_memory_feedback_applied")
}

pub(in crate::gemma_business) fn live_evolution_items(body: &str) -> u64 {
    response_u64_field(body, "live_evolution_items")
}
