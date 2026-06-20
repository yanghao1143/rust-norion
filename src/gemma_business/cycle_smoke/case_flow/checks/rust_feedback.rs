use super::assertions::{require_positive_feedback, require_response_bool_field};
use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::cycle_smoke::case_flow::response::BusinessCycleCaseResponse;

pub(super) fn require_rust_feedback(
    business_case: &GemmaModelServiceBusinessCase,
    cycle_response: &BusinessCycleCaseResponse,
    failures: &mut Vec<String>,
    case_passed: &mut bool,
) {
    require_response_bool_field(
        business_case,
        &cycle_response.body,
        "rust_check_checked",
        "business-cycle did not run rust-check",
        failures,
        case_passed,
    );
    require_response_bool_field(
        business_case,
        &cycle_response.body,
        "rust_check_passed",
        "business-cycle rust-check did not pass",
        failures,
        case_passed,
    );
    require_positive_feedback(
        business_case,
        cycle_response.rust_check_feedback_applied,
        "business-cycle rust-check feedback did not update memory",
        failures,
        case_passed,
    );
}
