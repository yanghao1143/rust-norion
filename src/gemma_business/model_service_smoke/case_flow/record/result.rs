use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::audit::GemmaModelServiceAnswerAudit;
use crate::gemma_business::model_service_smoke::case_flow::generate::GenerateEvidence;
use crate::gemma_business::model_service_smoke::case_flow::rust_check::RustCheckFeedback;
use crate::gemma_business::smoke_report::{
    GemmaModelServiceCaseResult, compact_business_answer_preview,
};

pub(super) fn model_service_case_result(
    business_case: &GemmaModelServiceBusinessCase,
    generate: GenerateEvidence,
    feedback_ok: bool,
    rust_check: RustCheckFeedback,
) -> GemmaModelServiceCaseResult {
    let answer_audit = GemmaModelServiceAnswerAudit::from_case(business_case, &generate.answer);
    GemmaModelServiceCaseResult {
        name: business_case.name,
        experience_id: generate.experience_id,
        feedback_memory_ids: generate.feedback_memory_ids,
        runtime_token_count: generate.runtime_token_count,
        answer_chars: generate.answer.chars().count(),
        answer_preview: compact_business_answer_preview(&generate.answer, 180),
        answer_audit,
        generate_ok: generate.ok,
        feedback_ok,
        rust_check_ok: rust_check.ok,
    }
}
