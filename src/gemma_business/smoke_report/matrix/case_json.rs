use crate::model_service::json::{service_json_string, service_json_string_array};

use crate::gemma_business::smoke_report::preview::compact_business_answer_preview;
use crate::gemma_business::smoke_report::types::GemmaBusinessCycleCaseResult;

pub(super) fn gemma_business_cycle_smoke_cases_report_json(
    case_results: &[GemmaBusinessCycleCaseResult],
) -> String {
    case_results
        .iter()
        .map(gemma_business_cycle_smoke_case_report_json)
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn gemma_business_cycle_smoke_case_report_json(
    result: &GemmaBusinessCycleCaseResult,
) -> String {
    format!(
        "{{\"name\":{},\"passed\":{},\"runtime_token_count\":{},\"feedback_applied\":{},\"rust_check_feedback_applied\":{},\"checked_trace_lines\":{},\"contract\":{{\"passed\":{},\"required_signals\":{},\"matched_signals\":{},\"missing_signals\":{},\"runtime_model_experiences\":{},\"protocol_leak\":{},\"substituted_runtime_model_experiences\":{},\"evasive_denial\":{},\"handling_signal\":{}}},\"answer_preview\":{}}}",
        service_json_string(result.name),
        result.passed,
        result.runtime_token_count,
        result.feedback_applied,
        result.rust_check_feedback_applied,
        result.checked_trace_lines,
        result.answer_audit.passed(),
        result.answer_audit.required_signals,
        result.answer_audit.matched_signals,
        service_json_string_array(&result.answer_audit.missing_signals),
        result.answer_audit.has_runtime_model_experiences,
        result.answer_audit.protocol_leak,
        result.answer_audit.substituted_runtime_model_experiences,
        result.answer_audit.evasive_denial,
        result.answer_audit.handling_signal,
        service_json_string(&compact_business_answer_preview(&result.answer, 180))
    )
}
