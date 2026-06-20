use crate::gemma_business::regression::report_checks::{
    report_contains_contract_pass, require_report_bool, require_report_bool_false,
};
use crate::gemma_business::response_json::response_optional_bool_field;

pub(super) fn require_contract_fields(body: &str, failures: &mut Vec<String>) {
    require_report_bool(
        failures,
        "contract.passed",
        report_contains_contract_pass(body),
    );
    require_report_bool(
        failures,
        "runtime_model_experiences",
        response_optional_bool_field(body, "runtime_model_experiences"),
    );
    require_report_bool_false(
        failures,
        "protocol_leak",
        response_optional_bool_field(body, "protocol_leak"),
    );
    require_report_bool_false(
        failures,
        "substituted_runtime_model_experiences",
        response_optional_bool_field(body, "substituted_runtime_model_experiences"),
    );
    require_report_bool_false(
        failures,
        "evasive_denial",
        response_optional_bool_field(body, "evasive_denial"),
    );
    require_report_bool(
        failures,
        "handling_signal",
        response_optional_bool_field(body, "handling_signal"),
    );
}
