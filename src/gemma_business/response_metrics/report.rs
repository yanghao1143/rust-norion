use crate::gemma_business::response_json::response_u64_field;

pub(in crate::gemma_business) fn report_external_feedbacks(body: &str) -> u64 {
    response_u64_field(body, "external_feedbacks")
}

pub(in crate::gemma_business) fn report_feedback_memory_updates(body: &str) -> u64 {
    response_u64_field(body, "feedback_memory_updates")
}

pub(in crate::gemma_business) fn report_replay_rust_check_passed(body: &str) -> u64 {
    response_u64_field(body, "replay_rust_check_passed")
}
