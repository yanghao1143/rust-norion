use super::response::BusinessCycleCaseResponse;
use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::smoke_report::GemmaBusinessCycleCaseResult;

pub(super) fn business_cycle_case_result(
    business_case: &GemmaModelServiceBusinessCase,
    cycle_response: BusinessCycleCaseResponse,
    case_passed: bool,
) -> GemmaBusinessCycleCaseResult {
    GemmaBusinessCycleCaseResult {
        name: business_case.name,
        body: cycle_response.body,
        answer: cycle_response.answer,
        answer_audit: cycle_response.answer_audit,
        runtime_token_count: cycle_response.runtime_token_count,
        feedback_applied: cycle_response.feedback_applied,
        rust_check_feedback_applied: cycle_response.rust_check_feedback_applied,
        checked_trace_lines: cycle_response.checked_trace_lines,
        passed: case_passed,
    }
}
