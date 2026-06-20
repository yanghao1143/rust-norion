mod counts;
mod result;

use counts::record_case_counts;
use result::model_service_case_result;

use super::generate::GenerateEvidence;
use super::rust_check::RustCheckFeedback;
use super::types::ModelServiceCaseRun;
use crate::gemma_business::GemmaModelServiceBusinessCase;

pub(super) fn record_model_service_case_result(
    business_case: &GemmaModelServiceBusinessCase,
    generate: GenerateEvidence,
    feedback_ok: bool,
    rust_check: RustCheckFeedback,
    failures: &mut Vec<String>,
    run: &mut ModelServiceCaseRun,
) {
    record_case_counts(run, &generate, feedback_ok, &rust_check);

    let case_result = model_service_case_result(business_case, generate, feedback_ok, rust_check);
    let answer_audit = &case_result.answer_audit;
    if let Some(failure) = answer_audit.failure() {
        failures.push(format!(
            "{} generate answer failed: {failure}",
            business_case.name
        ));
    }
    run.case_results.push(case_result);
}
