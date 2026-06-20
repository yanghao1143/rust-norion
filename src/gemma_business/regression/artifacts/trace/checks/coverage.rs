use std::path::Path;

use rust_norion::TraceSchemaGateReport;

use crate::gemma_business::response_json::response_u64_field;

mod minimum;

use minimum::require_trace_min_u64;

pub(super) fn require_checked_lines_cover_report(
    trace_path: &Path,
    trace_report: &TraceSchemaGateReport,
    report_body: &str,
    failures: &mut Vec<String>,
) {
    let expected_lines = response_u64_field(report_body, "checked_lines");
    require_trace_min_u64(
        trace_path,
        "checked_lines",
        trace_report.checked_lines as u64,
        "report",
        expected_lines,
        failures,
    );
}

pub(super) fn require_business_contract_events_cover_cases(
    trace_path: &Path,
    trace_report: &TraceSchemaGateReport,
    expected_case_count: u64,
    failures: &mut Vec<String>,
) {
    require_trace_min_u64(
        trace_path,
        "passed business contract events",
        trace_report.business_contract_event_passed as u64,
        "report case_count",
        expected_case_count,
        failures,
    );
}

pub(super) fn require_rust_check_events_cover_cases(
    trace_path: &Path,
    trace_report: &TraceSchemaGateReport,
    expected_case_count: u64,
    failures: &mut Vec<String>,
) {
    require_trace_min_u64(
        trace_path,
        "passed rust-check events",
        trace_report.rust_check_passed as u64,
        "report case_count",
        expected_case_count,
        failures,
    );
}
