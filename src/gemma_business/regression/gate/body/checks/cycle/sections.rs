use crate::gemma_business::regression::report_checks::{
    report_contains_object_bool, require_report_bool,
};

pub(super) fn require_cycle_sections(body: &str, failures: &mut Vec<String>) {
    require_report_bool(
        failures,
        "rust_check.checked",
        report_contains_object_bool(body, "rust_check", "checked", true),
    );
    require_report_bool(
        failures,
        "rust_check.passed",
        report_contains_object_bool(body, "rust_check", "passed", true),
    );
    require_report_bool(
        failures,
        "self_improve.checked",
        report_contains_object_bool(body, "self_improve", "checked", true),
    );
    require_report_bool(
        failures,
        "self_improve.passed",
        report_contains_object_bool(body, "self_improve", "passed", true),
    );
}
