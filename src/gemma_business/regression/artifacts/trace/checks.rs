use std::path::Path;

use rust_norion::TraceSchemaGateReport;

mod coverage;
mod schema;

use coverage::{
    require_business_contract_events_cover_cases, require_checked_lines_cover_report,
    require_rust_check_events_cover_cases,
};
use schema::require_schema_gate_passed;

pub(super) fn require_trace_report_checks(
    trace_path: &Path,
    trace_report: &TraceSchemaGateReport,
    report_body: &str,
    expected_case_count: u64,
    failures: &mut Vec<String>,
) {
    require_schema_gate_passed(trace_path, trace_report, failures);
    require_checked_lines_cover_report(trace_path, trace_report, report_body, failures);
    require_business_contract_events_cover_cases(
        trace_path,
        trace_report,
        expected_case_count,
        failures,
    );
    require_rust_check_events_cover_cases(trace_path, trace_report, expected_case_count, failures);
}
