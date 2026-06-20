use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::request_json::experience_id_field;
use crate::model_service::json::service_json_string;

pub(super) const RUST_FEEDBACK_CASE: &str = "gemma-service-rust-feedback";

const RUST_CHECK_CODE: &str = "pub fn apply_user_feedback(memory_id: u64, amount: f32) -> bool { memory_id > 0 && amount.is_finite() && amount >= 0.0 }";

pub(super) fn rust_check_request_body(
    business_case: &GemmaModelServiceBusinessCase,
    experience_id: Option<u64>,
) -> String {
    format!(
        "{{{},\"edition\":\"2021\",\"case\":\"{}-compiler-feedback\",\"code\":{}}}",
        experience_id_field(experience_id),
        business_case.name,
        service_json_string(RUST_CHECK_CODE)
    )
}
