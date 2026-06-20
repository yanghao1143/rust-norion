use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::request_json::business_case_request_fields;

pub(super) fn generate_request_body(business_case: &GemmaModelServiceBusinessCase) -> String {
    format!("{{{}}}", business_case_request_fields(business_case))
}
