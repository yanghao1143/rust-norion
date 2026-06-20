mod evidence;
mod failures;
mod request;
mod response;

use std::io;

use failures::push_feedback_failures;
use request::feedback_request_body;
use response::feedback_evidence_from_body;

use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::model_service::http::{model_service_http_body, model_service_http_request};

pub(super) fn apply_generated_feedback(
    bind: &str,
    business_case: &GemmaModelServiceBusinessCase,
    experience_id: Option<u64>,
    feedback_memory_count: usize,
    failures: &mut Vec<String>,
) -> io::Result<bool> {
    let feedback_request = feedback_request_body(experience_id);
    let feedback =
        model_service_http_request(bind, "POST", "/v1/feedback", Some(&feedback_request))?;
    let feedback_body = model_service_http_body(&feedback);
    let feedback = feedback_evidence_from_body(feedback_body);
    push_feedback_failures(business_case, &feedback, feedback_memory_count, failures);
    Ok(feedback.ok)
}
