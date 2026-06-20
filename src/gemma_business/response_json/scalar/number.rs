use crate::model_service::json::{json_u64_array_field, json_u64_field};

pub(in crate::gemma_business) fn response_u64_field(body: &str, field: &str) -> u64 {
    response_optional_u64_field(body, field).unwrap_or_default()
}

pub(in crate::gemma_business) fn response_optional_u64_field(
    body: &str,
    field: &str,
) -> Option<u64> {
    json_u64_field(body, field)
}

pub(in crate::gemma_business) fn response_u64_array_field(body: &str, field: &str) -> Vec<u64> {
    json_u64_array_field(body, field).unwrap_or_default()
}
