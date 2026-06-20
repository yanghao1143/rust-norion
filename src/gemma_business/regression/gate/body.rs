mod checks;
mod evidence;

use checks::require_report_body;
use evidence::ReportBodyEvidence;

use super::types::GemmaBusinessCycleSmokeReportGate;

pub fn evaluate_gemma_business_cycle_smoke_report_gate_body(
    body: &str,
) -> GemmaBusinessCycleSmokeReportGate {
    let evidence = ReportBodyEvidence::from_body(body);
    let mut failures = Vec::new();
    require_report_body(body, &evidence, &mut failures);

    GemmaBusinessCycleSmokeReportGate {
        passed: failures.is_empty(),
        schema: evidence.schema,
        case_count: evidence.case_count,
        passed_cases: evidence.passed_cases,
        runtime_token_count: evidence.runtime_token_count,
        feedback_applied: evidence.feedback_applied,
        rust_check_feedback_applied: evidence.rust_check_feedback_applied,
        external_feedbacks: evidence.external_feedbacks,
        feedback_memory_updates: evidence.feedback_memory_updates,
        replay_rust_check_passed: evidence.replay_rust_check_passed,
        replay_live_memory_feedback_applied: evidence.replay_live_memory_feedback_applied,
        replay_live_evolution_items: evidence.replay_live_evolution_items,
        checked_trace_lines: evidence.checked_trace_lines,
        failures,
    }
}
