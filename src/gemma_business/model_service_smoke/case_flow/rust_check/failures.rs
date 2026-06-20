use super::RustCheckFeedback;
use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::model_service_smoke::case_flow::checks::{
    require_case_condition, require_case_u64_at_least,
};

pub(super) fn push_rust_check_failures(
    business_case: &GemmaModelServiceBusinessCase,
    rust_check: &RustCheckFeedback,
    feedback_memory_count: usize,
    failures: &mut Vec<String>,
) {
    require_case_condition(
        business_case,
        rust_check.ok == Some(true),
        "rust-check endpoint did not return compiler-backed reinforcement",
        failures,
    );
    require_case_u64_at_least(
        business_case,
        rust_check.applied,
        feedback_memory_count as u64,
        "rust-check endpoint did not apply every generated feedback memory",
        failures,
    );
}
