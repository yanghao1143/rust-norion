use crate::gemma_business::model_service_smoke::print::ModelServiceSmokeReport;

pub(in crate::gemma_business::model_service_smoke::print) fn print_runtime_audit(
    report: &ModelServiceSmokeReport<'_>,
) {
    let runtime_audit = report.inspect.runtime_audit;
    println!(
        "gemma_model_service_smoke_runtime_audit: passed={} runtime_error_experiences={} runtime_errors={} runtime_timeout_experiences={} runtime_timeouts={} trace_runtime_error_events={} trace_runtime_timeout_events={}",
        runtime_audit.passed(),
        runtime_audit.runtime_error_experiences,
        runtime_audit.runtime_errors,
        runtime_audit.runtime_timeout_experiences,
        runtime_audit.runtime_timeouts,
        runtime_audit.trace_runtime_error_events,
        runtime_audit.trace_runtime_timeout_events
    );
}
