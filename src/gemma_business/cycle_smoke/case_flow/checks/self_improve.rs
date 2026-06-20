use super::assertions::require_response_bool_field;
use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::cycle_smoke::case_flow::response::BusinessCycleCaseResponse;

pub(super) fn require_self_improve(
    business_case: &GemmaModelServiceBusinessCase,
    cycle_response: &BusinessCycleCaseResponse,
    failures: &mut Vec<String>,
    case_passed: &mut bool,
) {
    require_response_bool_field(
        business_case,
        &cycle_response.body,
        "self_improve_checked",
        "business-cycle did not run self-improve",
        failures,
        case_passed,
    );
    require_response_bool_field(
        business_case,
        &cycle_response.body,
        "self_improve_passed",
        "business-cycle self-improve did not pass",
        failures,
        case_passed,
    );
}
