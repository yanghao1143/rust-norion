use crate::gemma_business::response_json::response_u64_field;

pub(in crate::gemma_business) fn feedback_applied(body: &str) -> u64 {
    response_u64_field(body, "applied")
}

pub(in crate::gemma_business) fn rust_check_feedback_applied(body: &str) -> u64 {
    response_u64_field(body, "rust_check_feedback_applied")
}
