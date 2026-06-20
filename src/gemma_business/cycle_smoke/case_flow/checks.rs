mod assertions;
mod audit;
mod cycle;
mod gates;
mod rust_feedback;
mod self_improve;

use super::response::BusinessCycleCaseResponse;
use crate::gemma_business::GemmaModelServiceBusinessCase;
use audit::require_runtime_and_answer_audit;
use cycle::require_cycle_response;
use gates::require_state_and_trace_gates;
use rust_feedback::require_rust_feedback;
use self_improve::require_self_improve;

pub(super) fn case_failure_passed(
    business_case: &GemmaModelServiceBusinessCase,
    cycle_response: &BusinessCycleCaseResponse,
    failures: &mut Vec<String>,
) -> bool {
    let mut case_passed = true;
    push_case_failures(business_case, cycle_response, failures, &mut case_passed);
    case_passed
}

fn push_case_failures(
    business_case: &GemmaModelServiceBusinessCase,
    cycle_response: &BusinessCycleCaseResponse,
    failures: &mut Vec<String>,
    case_passed: &mut bool,
) {
    require_cycle_response(business_case, cycle_response, failures, case_passed);
    require_rust_feedback(business_case, cycle_response, failures, case_passed);
    require_self_improve(business_case, cycle_response, failures, case_passed);
    require_state_and_trace_gates(business_case, cycle_response, failures, case_passed);
    require_runtime_and_answer_audit(business_case, cycle_response, failures, case_passed);
}
