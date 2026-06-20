use super::GenerateEvidence;
use crate::gemma_business::response_json::{
    response_ok, response_optional_u64_field, response_string_field, response_u64_array_field,
    response_u64_field,
};

pub(super) fn generate_evidence_from_body(body: &str) -> GenerateEvidence {
    GenerateEvidence {
        experience_id: response_optional_u64_field(body, "experience_id"),
        feedback_memory_ids: response_u64_array_field(body, "feedback_memory_ids"),
        runtime_token_count: response_u64_field(body, "runtime_token_count"),
        answer: response_string_field(body, "answer"),
        ok: response_ok(body),
    }
}
