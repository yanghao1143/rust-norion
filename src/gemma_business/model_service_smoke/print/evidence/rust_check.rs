use crate::gemma_business::model_service_smoke::print::ModelServiceSmokeReport;

pub(in crate::gemma_business::model_service_smoke::print) fn print_rust_check_evidence(
    report: &ModelServiceSmokeReport<'_>,
) {
    println!(
        "gemma_model_service_smoke_rust_check: expected={} ok={} inspect_passed={} inspect_experiences={} replay_items={} replay_passed={} replay_feedback_updates={} replay_feedback_applied={}",
        report.case_run.rust_check_expected_count,
        report.case_run.rust_check_ok_count,
        report.inspect.rust_check_passed,
        report.inspect.rust_check_experiences,
        report.replay.rust_check.items,
        report.replay.rust_check.passed,
        report.replay.rust_check.feedback_updates,
        report.replay.rust_check.feedback_applied
    );
}
