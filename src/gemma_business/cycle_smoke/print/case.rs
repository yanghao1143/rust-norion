use super::BusinessCycleSmokePrintReport;
use crate::gemma_business::smoke_report::compact_business_answer_preview;
use crate::model_service::json::service_json_string;

pub(super) fn print_case_summaries(report: &BusinessCycleSmokePrintReport<'_>) {
    for result in report.case_results {
        println!(
            "gemma_business_cycle_smoke_case: name={} passed={} runtime_tokens={} feedback_applied={} rust_check_feedback_applied={} trace_lines={} answer_preview={}",
            result.name,
            result.passed,
            result.runtime_token_count,
            result.feedback_applied,
            result.rust_check_feedback_applied,
            result.checked_trace_lines,
            service_json_string(&compact_business_answer_preview(&result.answer, 180))
        );
    }
}
