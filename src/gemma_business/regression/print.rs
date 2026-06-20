use std::path::Path;

use super::GemmaBusinessCycleSmokeReportGate;

pub(super) fn print_gemma_business_report_gate(
    label: &str,
    failure_label: &str,
    path: &Path,
    report: &GemmaBusinessCycleSmokeReportGate,
) {
    println!(
        "{}: passed={} path={} schema={} failures={} cases={}/{} runtime_tokens={} feedback_applied={} rust_check_feedback_applied={} external_feedbacks={} feedback_memory_updates={} replay_rust_check_passed={} replay_live_memory_feedback_applied={} replay_live_evolution_items={} trace_lines={}",
        label,
        report.passed,
        path.display(),
        report.schema.as_deref().unwrap_or("missing"),
        report.failures.len(),
        report.passed_cases,
        report.case_count,
        report.runtime_token_count,
        report.feedback_applied,
        report.rust_check_feedback_applied,
        report.external_feedbacks,
        report.feedback_memory_updates,
        report.replay_rust_check_passed,
        report.replay_live_memory_feedback_applied,
        report.replay_live_evolution_items,
        report.checked_trace_lines
    );
    for failure in &report.failures {
        println!("{failure_label}: {failure}");
    }
}
