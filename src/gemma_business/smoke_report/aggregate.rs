use crate::model_service::json::service_json_string;

use super::types::GemmaBusinessCycleCaseResult;

pub(crate) fn gemma_business_cycle_smoke_aggregate_response_json(
    passed: bool,
    case_results: &[GemmaBusinessCycleCaseResult],
    runtime_token_count: u64,
    feedback_applied: u64,
    rust_check_feedback_applied: u64,
    checked_trace_lines: u64,
) -> String {
    let cases = case_results
        .iter()
        .map(|result| {
            format!(
                "{{\"name\":{},\"passed\":{},\"runtime_token_count\":{},\"feedback_applied\":{},\"rust_check_feedback_applied\":{},\"checked_trace_lines\":{},\"response\":{}}}",
                service_json_string(result.name),
                result.passed,
                result.runtime_token_count,
                result.feedback_applied,
                result.rust_check_feedback_applied,
                result.checked_trace_lines,
                if result.body.trim().is_empty() {
                    "null".to_owned()
                } else {
                    result.body.clone()
                }
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"ok\":true,\"business_cycle\":{{\"passed\":{},\"case_count\":{},\"passed_cases\":{}}},\"runtime_token_count\":{},\"feedback_applied\":{},\"rust_check_feedback_applied\":{},\"checked_lines\":{},\"cases\":[{}]}}",
        passed,
        case_results.len(),
        case_results.iter().filter(|result| result.passed).count(),
        runtime_token_count,
        feedback_applied,
        rust_check_feedback_applied,
        checked_trace_lines,
        cases
    )
}
