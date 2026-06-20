use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::audit::gemma_business_smoke_runtime_failure_parts;
use crate::gemma_business::cycle_smoke::case_flow::response::BusinessCycleCaseResponse;

use super::assertions::fail_case;

pub(super) fn require_runtime_and_answer_audit(
    business_case: &GemmaModelServiceBusinessCase,
    cycle_response: &BusinessCycleCaseResponse,
    failures: &mut Vec<String>,
    case_passed: &mut bool,
) {
    if let Some(failure) = gemma_business_smoke_runtime_failure_parts(
        &cycle_response.answer,
        cycle_response.runtime_token_count as usize,
    ) {
        fail_case(
            business_case,
            &format!("business-cycle runtime failed: {failure}"),
            failures,
            case_passed,
        );
    }
    if let Some(failure) = cycle_response.answer_audit.failure() {
        fail_case(
            business_case,
            &format!("business-cycle answer failed: {failure}"),
            failures,
            case_passed,
        );
    }
}
