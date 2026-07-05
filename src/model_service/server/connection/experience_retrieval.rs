use std::net::TcpStream;
use std::time::Instant;

use rust_norion::{ExperienceRecord, NoironEngine, TenantScope};

use super::super::super::json::{service_error_json, write_http_json};
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
    let started = Instant::now();
    let profile = request.profile.unwrap_or(args.profile);
    let limit = request
        .limit
        .unwrap_or(args.experience_retrieval_limit)
        .max(1);
    let Some(scope) = request.tenant_scope.as_ref() else {
        let body = service_error_json(
            "experience-retrieval requires tenant_id, workspace_id, and session_id",
        );
        return write_http_json(stream, 400, "Bad Request", &body);
    };
    let retrieval_prompt = request.effective_retrieval_prompt();
    let visible_memory_ids = scoped_memory_ids(engine, scope);
    let mut report =
        engine
            .experience
            .retrieval_report_matching(&retrieval_prompt, profile, limit, |record| {
                record_has_visible_memory(record, &visible_memory_ids)
            });
    report.prompt = request.prompt.clone();
    let body = model_service_experience_retrieval_response_json(
        request_id,
        &report,
        started.elapsed().as_millis(),
        request.index_context_used(),
        request.index_context_chars(),
    );
    write_http_json(stream, 200, "OK", &body)
}

fn scoped_memory_ids(engine: &NoironEngine, scope: &TenantScope) -> Vec<u64> {
    engine
        .cache
        .entries_scoped(scope)
        .into_iter()
        .map(|entry| entry.id)
        .collect()
}

fn record_has_visible_memory(record: &ExperienceRecord, visible_memory_ids: &[u64]) -> bool {
    record
        .stored_memory_id
        .is_some_and(|id| visible_memory_ids.contains(&id))
        || record
            .used_memory_ids
            .iter()
            .chain(record.gist_memory_ids.iter())
            .chain(record.stored_runtime_kv_memory_ids.iter())
            .any(|id| visible_memory_ids.contains(id))
}
