use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use crate::model_service::http::try_model_service_http_request;

pub(super) fn require_business_cycle_case_coverage(
    bind: &str,
    case_count: usize,
    failures: &mut Vec<String>,
) {
    let expected_case_count = GEMMA_MODEL_SERVICE_BUSINESS_CASES.len();
    if case_count >= expected_case_count {
        return;
    }
    failures.push(format!(
        "business-cycle matrix ran {case_count}/{expected_case_count} cases"
    ));
    for _ in case_count..expected_case_count {
        let _ = try_model_service_http_request(bind, "GET", "/health", None);
    }
}
