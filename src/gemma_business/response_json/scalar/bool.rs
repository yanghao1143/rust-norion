use crate::model_service::json::json_bool_field;

pub(in crate::gemma_business) fn response_bool_field(body: &str, field: &str) -> bool {
    response_optional_bool_field(body, field).unwrap_or(false)
}

pub(in crate::gemma_business) fn response_optional_bool_field(
    body: &str,
    field: &str,
) -> Option<bool> {
    json_bool_field(body, field)
}

pub(in crate::gemma_business) fn response_ok(body: &str) -> bool {
    response_bool_field(body, "ok")
}
