mod evidence;
mod failures;
mod request;
mod response;

use std::io;

use failures::push_generate_failures;
use request::generate_request_body;
use response::generate_evidence_from_body;

use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::model_service::http::{model_service_http_body, model_service_http_request};

pub(super) use evidence::GenerateEvidence;

pub(super) fn run_generate(
    bind: &str,
    business_case: &GemmaModelServiceBusinessCase,
    failures: &mut Vec<String>,
) -> io::Result<GenerateEvidence> {
    let generate_request = generate_request_body(business_case);
    let generate =
        model_service_http_request(bind, "POST", "/v1/generate", Some(&generate_request))?;
    let generate_body = model_service_http_body(&generate).to_owned();
    let evidence = generate_evidence_from_body(&generate_body);
    push_generate_failures(business_case, &evidence, failures);
    Ok(evidence)
}
