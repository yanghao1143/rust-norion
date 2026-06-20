use crate::gemma_business::regression::report_checks::{
    require_report_bool, require_report_string,
};
use crate::gemma_business::response_json::{
    response_optional_bool_field, response_optional_string_field,
};

pub(super) fn require_http_and_gate_fields(body: &str, failures: &mut Vec<String>) {
    require_report_string(
        failures,
        "gate",
        response_optional_string_field(body, "gate").as_deref(),
        "gemma_business_cycle",
    );
    require_report_bool(
        failures,
        "passed",
        response_optional_bool_field(body, "passed"),
    );
    require_report_bool(
        failures,
        "health_ok",
        response_optional_bool_field(body, "health_ok"),
    );
    require_report_bool(
        failures,
        "readiness_passed",
        response_optional_bool_field(body, "readiness_passed"),
    );
    require_report_bool(
        failures,
        "safe_device_passed",
        response_optional_bool_field(body, "safe_device_passed"),
    );
    require_report_bool(
        failures,
        "business_cycle_ok",
        response_optional_bool_field(body, "business_cycle_ok"),
    );
    require_report_bool(
        failures,
        "business_cycle_passed",
        response_optional_bool_field(body, "business_cycle_passed"),
    );
    require_report_bool(
        failures,
        "state_gate_passed",
        response_optional_bool_field(body, "state_gate_passed"),
    );
    require_report_bool(
        failures,
        "trace_gate_passed",
        response_optional_bool_field(body, "trace_gate_passed"),
    );
}
