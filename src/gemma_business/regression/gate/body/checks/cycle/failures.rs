use crate::gemma_business::response_json::response_empty_array_field;

pub(super) fn require_empty_failure_fields(body: &str, failures: &mut Vec<String>) {
    if !response_empty_array_field(body, "missing_signals") {
        failures.push("missing_signals not empty".to_owned());
    }
    if !response_empty_array_field(body, "failures") {
        failures.push("report failures array not empty".to_owned());
    }
}
