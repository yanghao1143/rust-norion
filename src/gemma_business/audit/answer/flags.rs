use crate::gemma_business::audit::signals::{
    business_answer_contains_evasive_denial, business_answer_contains_handling_signal,
    business_answer_contains_protocol_leak,
};

pub(super) struct BusinessAnswerFlags {
    pub(super) has_runtime_model_experiences: bool,
    pub(super) protocol_leak: bool,
    pub(super) substituted_runtime_model_experiences: bool,
    pub(super) evasive_denial: bool,
    pub(super) handling_signal: bool,
}

impl BusinessAnswerFlags {
    pub(super) fn from_answer(answer: &str, lower: &str) -> Self {
        Self {
            has_runtime_model_experiences: answer.contains("runtime_model_experiences"),
            protocol_leak: business_answer_contains_protocol_leak(answer, lower),
            substituted_runtime_model_experiences: lower.contains("memory_experiences"),
            evasive_denial: business_answer_contains_evasive_denial(answer, lower),
            handling_signal: business_answer_contains_handling_signal(answer, lower),
        }
    }
}
