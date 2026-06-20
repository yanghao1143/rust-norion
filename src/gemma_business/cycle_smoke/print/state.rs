use crate::gemma_business::response_metrics::{
    cycle_external_feedbacks, cycle_feedback_memory_updates, cycle_replay_rust_check_passed,
    cycle_rust_check_passed, live_evolution_items, live_memory_feedback_applied, runtime_tokens,
};

use super::BusinessCycleSmokePrintReport;

pub(super) fn print_state_summary(report: &BusinessCycleSmokePrintReport<'_>) {
    println!(
        "gemma_business_cycle_smoke_state: cases={} runtime_tokens={} external_feedbacks={} feedback_memory_updates={} rust_check_passed={} replay_rust_check_passed={} replay_live_memory_feedback_applied={} replay_live_evolution_items={}",
        report.case_results.len(),
        runtime_tokens(report.final_cycle_body),
        cycle_external_feedbacks(report.final_cycle_body),
        cycle_feedback_memory_updates(report.final_cycle_body),
        cycle_rust_check_passed(report.final_cycle_body),
        cycle_replay_rust_check_passed(report.final_cycle_body),
        live_memory_feedback_applied(report.final_cycle_body),
        live_evolution_items(report.final_cycle_body)
    );
}
