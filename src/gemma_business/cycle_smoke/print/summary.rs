use crate::option_path_display;

use super::BusinessCycleSmokePrintReport;
use crate::gemma_business::health_status::{SmokeHealthStatus, optional_bool_label};

pub(super) fn print_http_summary(report: &BusinessCycleSmokePrintReport<'_>) {
    let health = SmokeHealthStatus::from_body(report.health_body);
    println!(
        "gemma_business_cycle_smoke_http: passed={} bind={} health_ok={} readiness_ok={} readiness_failures={} safe_device_ok={} safe_device_failures={} cycle_ok={}/{} runtime_tokens={} feedback_applied={} rust_check_feedback_applied={} trace_lines={}",
        report.passed,
        report.bind,
        health.ok,
        optional_bool_label(health.readiness_ok),
        health.readiness_failures.len(),
        optional_bool_label(health.safe_device_ok),
        health.safe_device_failures.len(),
        report.passed_cases,
        report.expected_case_count,
        report.runtime_token_count,
        report.feedback_applied,
        report.rust_check_feedback_applied,
        report.checked_trace_lines
    );
}

pub(super) fn print_failures(report: &BusinessCycleSmokePrintReport<'_>) {
    for failure in report.failures {
        println!("gemma_business_cycle_smoke_failure: {failure}");
    }
}

pub(super) fn print_gate_summary(report: &BusinessCycleSmokePrintReport<'_>) {
    println!(
        "gemma_business_cycle_smoke_gate: passed={} trace_file={} memory_file={} experience_file={} adaptive_file={} report_file={}",
        report.passed,
        option_path_display(report.service_args.trace_path.as_ref()),
        report.service_args.memory_path.display(),
        report.service_args.experience_path.display(),
        report.service_args.adaptive_path.display(),
        option_path_display(report.report_path)
    );
}
