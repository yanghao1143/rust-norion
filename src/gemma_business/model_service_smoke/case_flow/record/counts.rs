use crate::gemma_business::model_service_smoke::case_flow::generate::GenerateEvidence;
use crate::gemma_business::model_service_smoke::case_flow::rust_check::RustCheckFeedback;
use crate::gemma_business::model_service_smoke::case_flow::types::ModelServiceCaseRun;

pub(super) fn record_case_counts(
    run: &mut ModelServiceCaseRun,
    generate: &GenerateEvidence,
    feedback_ok: bool,
    rust_check: &RustCheckFeedback,
) {
    if generate.ok {
        run.generate_ok_count = run.generate_ok_count.saturating_add(1);
    }
    run.total_runtime_token_count = run
        .total_runtime_token_count
        .saturating_add(generate.runtime_token_count);
    run.total_feedback_memory_ids = run
        .total_feedback_memory_ids
        .saturating_add(generate.feedback_memory_ids.len() as u64);
    if feedback_ok {
        run.feedback_ok_count = run.feedback_ok_count.saturating_add(1);
    }
    if rust_check.checked {
        run.rust_check_expected_count = run.rust_check_expected_count.saturating_add(1);
    }
    if rust_check.ok == Some(true) {
        run.rust_check_ok_count = run.rust_check_ok_count.saturating_add(1);
    }
}
