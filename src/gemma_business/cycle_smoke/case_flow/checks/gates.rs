use super::assertions::require_response_object_bool_field;
use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::cycle_smoke::case_flow::response::BusinessCycleCaseResponse;

pub(super) fn require_state_and_trace_gates(
    business_case: &GemmaModelServiceBusinessCase,
    cycle_response: &BusinessCycleCaseResponse,
    failures: &mut Vec<String>,
    case_passed: &mut bool,
) {
    require_response_object_bool_field(
        business_case,
        &cycle_response.body,
        "state_gate",
        "passed",
        "business-cycle state gate did not pass",
        failures,
        case_passed,
    );
    require_response_object_bool_field(
        business_case,
        &cycle_response.body,
        "trace_gate",
        "passed",
        "business-cycle trace gate did not pass",
        failures,
        case_passed,
    );
}
