use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::request_json::business_case_request_fields;
use crate::model_service::json::service_json_string;

pub(super) fn business_cycle_request_json(business_case: &GemmaModelServiceBusinessCase) -> String {
    let rust_check_code = "pub fn apply_user_feedback(memory_id: u64) -> bool { memory_id > 0 }";
    let rust_check_case = format!("{}-business-cycle-rust-check", business_case.name);
    format!(
        "{{{},\"feedback_amount\":0.5,\"rust_check_code\":{},\"rust_check_case\":{},\"self_improve\":true,\"self_improve_limit\":1,\"gate\":\"gemma_business_cycle\",\"trace_gate\":true}}",
        business_case_request_fields(business_case),
        service_json_string(rust_check_code),
        service_json_string(&rust_check_case)
    )
}
