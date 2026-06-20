use super::ModelServiceSmokeReport;
use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use crate::gemma_business::health_status::{SmokeHealthStatus, optional_bool_label};
use crate::gemma_business::response_json::{response_object_bool_field, response_ok};
use crate::option_path_display;

pub(super) fn print_http_summary(report: &ModelServiceSmokeReport<'_>) {
    let health = SmokeHealthStatus::from_body(report.health_body);
    println!(
        "gemma_model_service_smoke_http: passed={} bind={} health_ok={} readiness_ok={} readiness_failures={} safe_device_ok={} safe_device_failures={} generate_ok={}/{} feedback_ok={}/{} rust_check_ok={}/{} self_improve_ok={} inspect_ok={}",
        report.failures.is_empty(),
        report.bind,
        health.ok,
        optional_bool_label(health.readiness_ok),
        health.readiness_failures.len(),
        optional_bool_label(health.safe_device_ok),
        health.safe_device_failures.len(),
        report.case_run.generate_ok_count,
        GEMMA_MODEL_SERVICE_BUSINESS_CASES.len(),
        report.case_run.feedback_ok_count,
        GEMMA_MODEL_SERVICE_BUSINESS_CASES.len(),
        report.case_run.rust_check_ok_count,
        report.case_run.rust_check_expected_count,
        response_object_bool_field(report.self_improve_body, "self_improve", "passed"),
        response_ok(report.inspect_body)
    );
}

pub(super) fn print_failures(report: &ModelServiceSmokeReport<'_>) {
    for failure in report.failures {
        println!("gemma_model_service_smoke_failure: {failure}");
    }
}

pub(super) fn print_gate_summary(report: &ModelServiceSmokeReport<'_>) {
    println!(
        "gemma_model_service_smoke_gate: passed={} trace_file={} memory_file={} experience_file={} adaptive_file={}",
        report.failures.is_empty(),
        option_path_display(report.service_args.trace_path.as_ref()),
        report.service_args.memory_path.display(),
        report.service_args.experience_path.display(),
        report.service_args.adaptive_path.display()
    );
}
