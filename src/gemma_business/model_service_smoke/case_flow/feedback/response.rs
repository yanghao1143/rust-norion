use super::evidence::FeedbackEvidence;
use crate::gemma_business::response_json::{response_ok, response_u64_field};

pub(super) fn feedback_evidence_from_body(body: &str) -> FeedbackEvidence {
    FeedbackEvidence {
        ok: response_ok(body),
        applied: response_u64_field(body, "applied"),
    }
}
