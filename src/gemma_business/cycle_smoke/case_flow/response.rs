use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::audit::GemmaModelServiceAnswerAudit;
use crate::gemma_business::response_json::{response_string_field, response_u64_field};

pub(super) struct BusinessCycleCaseResponse {
    pub(super) body: String,
    pub(super) answer: String,
    pub(super) answer_audit: GemmaModelServiceAnswerAudit,
    pub(super) runtime_token_count: u64,
    pub(super) feedback_applied: u64,
    pub(super) rust_check_feedback_applied: u64,
    pub(super) checked_trace_lines: u64,
}

impl BusinessCycleCaseResponse {
    pub(super) fn from_body(
        business_case: &GemmaModelServiceBusinessCase,
        cycle_body: &str,
    ) -> Self {
        let body = cycle_body.to_owned();
        let answer = response_string_field(&body, "answer");
        let answer_audit = GemmaModelServiceAnswerAudit::from_case(business_case, &answer);
        Self {
            runtime_token_count: response_u64_field(&body, "runtime_token_count"),
            feedback_applied: response_u64_field(&body, "feedback_applied"),
            rust_check_feedback_applied: response_u64_field(&body, "rust_check_feedback_applied"),
            checked_trace_lines: response_u64_field(&body, "checked_lines"),
            body,
            answer,
            answer_audit,
        }
    }
}
