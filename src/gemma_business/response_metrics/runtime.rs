use crate::gemma_business::response_json::response_u64_field;

pub(in crate::gemma_business) fn runtime_token_count(body: &str) -> u64 {
    response_u64_field(body, "runtime_token_count")
}

pub(in crate::gemma_business) fn runtime_tokens(body: &str) -> u64 {
    response_u64_field(body, "runtime_tokens")
}
