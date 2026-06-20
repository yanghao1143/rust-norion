use crate::gemma_business::model_service_smoke::print::ModelServiceSmokeReport;
use crate::model_service::json::service_json_string;
use crate::{option_bool_display, option_u64_display};

pub(super) fn print_case_summaries(report: &ModelServiceSmokeReport<'_>) {
    for result in &report.case_run.case_results {
        println!(
            "gemma_model_service_smoke_case: name={} experience_id={} feedback_memory_ids={} runtime_tokens={} answer_chars={} generate_ok={} feedback_ok={} rust_check_ok={} answer_preview={}",
            result.name,
            option_u64_display(result.experience_id),
            result.feedback_memory_ids.len(),
            result.runtime_token_count,
            result.answer_chars,
            result.generate_ok,
            result.feedback_ok,
            option_bool_display(result.rust_check_ok),
            service_json_string(&result.answer_preview)
        );
    }
}
