use super::assertions::{
    require_positive_feedback, require_response_object_bool_field, require_response_ok,
};
use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::cycle_smoke::case_flow::response::BusinessCycleCaseResponse;

pub(super) fn require_cycle_response(
    business_case: &GemmaModelServiceBusinessCase,
    cycle_response: &BusinessCycleCaseResponse,
    failures: &mut Vec<String>,
    case_passed: &mut bool,
) {
    require_response_ok(
        business_case,
        &cycle_response.body,
        "business-cycle endpoint did not return ok=true",
        failures,
        case_passed,
    );
    require_response_object_bool_field(
        business_case,
        &cycle_response.body,
        "business_cycle",
        "passed",
        "business-cycle report did not pass",
        failures,
        case_passed,
    );
    require_positive_feedback(
        business_case,
        cycle_response.feedback_applied,
        "business-cycle feedback did not apply to generated memory",
        failures,
        case_passed,
    );
}
