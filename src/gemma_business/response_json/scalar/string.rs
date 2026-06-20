use crate::model_service::json::json_string_field;

pub(in crate::gemma_business) fn response_string_field(body: &str, field: &str) -> String {
    response_optional_string_field(body, field).unwrap_or_default()
}

pub(in crate::gemma_business) fn response_optional_string_field(
    body: &str,
    field: &str,
) -> Option<String> {
    json_string_field(body, field)
}
