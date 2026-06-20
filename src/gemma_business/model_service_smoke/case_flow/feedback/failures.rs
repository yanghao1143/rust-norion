use super::evidence::FeedbackEvidence;
use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::model_service_smoke::case_flow::checks::{
    require_case_condition, require_case_u64_at_least,
};

pub(super) fn push_feedback_failures(
    business_case: &GemmaModelServiceBusinessCase,
    feedback: &FeedbackEvidence,
    feedback_memory_count: usize,
    failures: &mut Vec<String>,
) {
    require_case_condition(
        business_case,
        feedback.ok,
        "feedback endpoint did not return ok=true",
        failures,
    );
    require_case_u64_at_least(
        business_case,
        feedback.applied,
        feedback_memory_count as u64,
        "feedback endpoint did not apply every generated feedback memory",
        failures,
    );
}
