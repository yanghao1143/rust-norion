use crate::gemma_business::response_json::response_optional_object_bool_field;

pub(in crate::gemma_business::regression) fn report_contains_contract_pass(
    body: &str,
) -> Option<bool> {
    report_contains_object_bool(body, "contract", "passed", true)
}

pub(in crate::gemma_business::regression) fn report_contains_object_bool(
    body: &str,
    object: &str,
    field: &str,
    expected: bool,
) -> Option<bool> {
    response_optional_object_bool_field(body, object, field).map(|actual| actual == expected)
}
