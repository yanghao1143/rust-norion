mod evidence;
mod failures;
mod request;
mod response;

use std::io;

use failures::push_rust_check_failures;
use request::{RUST_FEEDBACK_CASE, rust_check_request_body};
use response::rust_check_feedback_from_body;

use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::model_service::http::{model_service_http_body, model_service_http_request};

pub(super) use evidence::RustCheckFeedback;

pub(super) fn run_rust_check_feedback(
    bind: &str,
    business_case: &GemmaModelServiceBusinessCase,
    experience_id: Option<u64>,
    feedback_memory_count: usize,
    failures: &mut Vec<String>,
) -> io::Result<RustCheckFeedback> {
    if business_case.name != RUST_FEEDBACK_CASE {
        return Ok(RustCheckFeedback {
            checked: false,
            ok: None,
            applied: 0,
        });
    }

    let rust_check_request = rust_check_request_body(business_case, experience_id);
    let rust_check =
        model_service_http_request(bind, "POST", "/v1/rust-check", Some(&rust_check_request))?;
    let rust_check_body = model_service_http_body(&rust_check);
    let rust_check = rust_check_feedback_from_body(rust_check_body);
    push_rust_check_failures(business_case, &rust_check, feedback_memory_count, failures);
    Ok(rust_check)
}
