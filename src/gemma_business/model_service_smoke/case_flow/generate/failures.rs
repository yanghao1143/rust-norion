use super::GenerateEvidence;
use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::audit::gemma_business_smoke_runtime_failure_parts;
use crate::gemma_business::model_service_smoke::case_flow::checks::{
    push_case_failure, require_case_condition,
};

pub(super) fn push_generate_failures(
    business_case: &GemmaModelServiceBusinessCase,
    evidence: &GenerateEvidence,
    failures: &mut Vec<String>,
) {
    require_case_condition(
        business_case,
        evidence.ok,
        "generate endpoint did not return ok=true",
        failures,
    );
    if let Some(failure) = gemma_business_smoke_runtime_failure_parts(
        &evidence.answer,
        evidence.runtime_token_count as usize,
    ) {
        push_case_failure(
            business_case,
            &format!("generate runtime failed: {failure}"),
            failures,
        );
    }
    require_case_condition(
        business_case,
        evidence.experience_id.is_some(),
        "generate response did not expose experience_id",
        failures,
    );
    require_case_condition(
        business_case,
        !evidence.feedback_memory_ids.is_empty(),
        "generate response did not expose feedback_memory_ids",
        failures,
    );
}
