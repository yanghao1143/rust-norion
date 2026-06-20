use crate::gemma_business::response_json::response_u64_field;

pub(in crate::gemma_business) fn checked_trace_lines(body: &str) -> u64 {
    response_u64_field(body, "checked_lines")
}
