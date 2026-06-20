use std::net::TcpStream;

use rust_norion::NoironEngine;

use super::super::super::json::write_http_json;
use super::super::super::request::ModelServiceExperienceRetrievalRequest;
use super::super::super::response::model_service_experience_retrieval_response_json;
use crate::Args;

pub(super) fn handle_experience_retrieval(
    engine: &NoironEngine,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceExperienceRetrievalRequest,
) -> std::io::Result<()> {
    let profile = request.profile.unwrap_or(args.profile);
    let limit = request
        .limit
        .unwrap_or(args.experience_retrieval_limit)
        .max(1);
    let mut report =
        engine
            .experience
            .retrieval_report(&request.effective_retrieval_prompt(), profile, limit);
    report.prompt = request.prompt.clone();
    let body = model_service_experience_retrieval_response_json(
        request_id,
        &report,
        request.index_context_used(),
        request.index_context_chars(),
    );
    write_http_json(stream, 200, "OK", &body)
}
