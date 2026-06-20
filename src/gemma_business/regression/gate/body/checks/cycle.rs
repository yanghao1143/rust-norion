mod failures;
mod http;
mod sections;

use failures::require_empty_failure_fields as require_empty_failure_fields_impl;
use http::require_http_and_gate_fields as require_http_and_gate_fields_impl;
use sections::require_cycle_sections as require_cycle_sections_impl;

pub(super) fn require_http_and_gate_fields(body: &str, failures: &mut Vec<String>) {
    require_http_and_gate_fields_impl(body, failures);
}

pub(super) fn require_cycle_sections(body: &str, failures: &mut Vec<String>) {
    require_cycle_sections_impl(body, failures);
}

pub(super) fn require_empty_failure_fields(body: &str, failures: &mut Vec<String>) {
    require_empty_failure_fields_impl(body, failures);
}
