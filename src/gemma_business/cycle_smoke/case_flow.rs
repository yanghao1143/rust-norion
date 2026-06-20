mod checks;
mod coverage;
mod request;
mod response;
mod result;

use crate::gemma_business::smoke_report::GemmaBusinessCycleCaseResult;
use crate::gemma_business::{GEMMA_MODEL_SERVICE_BUSINESS_CASES, GemmaModelServiceBusinessCase};
use crate::model_service::http::{model_service_http_body, model_service_http_request};
use checks::case_failure_passed;
use coverage::require_business_cycle_case_coverage;
use request::business_cycle_request_json;
use response::BusinessCycleCaseResponse;
use result::business_cycle_case_result;

pub(super) fn run_business_cycle_cases(
    bind: &str,
    failures: &mut Vec<String>,
) -> Vec<GemmaBusinessCycleCaseResult> {
    let mut case_results = Vec::new();
    if failures.is_empty() {
        for business_case in &GEMMA_MODEL_SERVICE_BUSINESS_CASES {
            case_results.push(run_business_cycle_case(bind, business_case, failures));
        }
    }
    require_business_cycle_case_coverage(bind, case_results.len(), failures);
    case_results
}

fn run_business_cycle_case(
    bind: &str,
    business_case: &GemmaModelServiceBusinessCase,
    failures: &mut Vec<String>,
) -> GemmaBusinessCycleCaseResult {
    let business_cycle_request = business_cycle_request_json(business_case);
    let response = match model_service_http_request(
        bind,
        "POST",
        "/v1/business-cycle",
        Some(&business_cycle_request),
    ) {
        Ok(response) => response,
        Err(error) => {
            failures.push(format!(
                "{} business-cycle endpoint failed: {error}",
                business_case.name
            ));
            String::new()
        }
    };
    let cycle_response =
        BusinessCycleCaseResponse::from_body(business_case, model_service_http_body(&response));
    let case_passed = case_failure_passed(business_case, &cycle_response, failures);
    business_cycle_case_result(business_case, cycle_response, case_passed)
}
