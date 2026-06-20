mod business_cycle;
mod endpoint_info;
mod evolution;
mod experience_cleanup_audit;
mod experience_hygiene;
mod experience_repair;
mod experience_retrieval;
mod generation;
mod inspection;
mod model_pool;

use std::net::TcpStream;
use std::sync::Mutex;

use rust_norion::{InferenceBackend, NoironEngine};

use self::business_cycle::{handle_business_cycle, handle_business_cycle_stream};
use self::endpoint_info::handle_endpoint_info;
use self::evolution::{handle_feedback, handle_replay, handle_rust_check, handle_self_improve};
use self::experience_cleanup_audit::handle_experience_cleanup_audit;
use self::experience_hygiene::{handle_experience_hygiene, handle_experience_hygiene_quarantine};
use self::experience_repair::handle_experience_repair;
use self::experience_retrieval::handle_experience_retrieval;
use self::generation::{
    GenerationHandlerContext, handle_chat, handle_chat_stream, handle_generate,
    handle_generate_stream,
};
use self::inspection::{handle_inspect, handle_state};
use self::model_pool::{
    handle_model_pool_call, handle_model_pool_manifest, handle_model_pool_route,
    handle_model_pool_status,
};
use super::super::http::read_http_request;
use super::super::json::{service_error_json, write_http_json};
use super::super::request::{
    ModelServiceChatRequest, ModelServiceHttpRequest, parse_model_service_http_request,
};
use super::health::model_service_health_json;
use super::state::ModelServiceServerState;
use crate::Args;

pub(super) fn handle_model_service_connection_concurrent<B: InferenceBackend>(
    engine: &Mutex<&mut NoironEngine>,
    backend: &Mutex<&mut B>,
    state: &ModelServiceServerState,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
) -> std::io::Result<()> {
    let raw = read_http_request(stream)?;
    let request = match parse_model_service_http_request(&raw) {
        Ok(request) => request,
        Err(error) => {
            let body = service_error_json(&error);
            return write_http_json(stream, 400, "Bad Request", &body);
        }
    };

    match request {
        ModelServiceHttpRequest::Health => handle_health(stream, request_id, state, args),
        ModelServiceHttpRequest::ExperienceHygiene => {
            handle_experience_hygiene(args, stream, request_id)
        }
        ModelServiceHttpRequest::ExperienceCleanupAudit(request) => {
            handle_experience_cleanup_audit(args, stream, request_id, request)
        }
        ModelServiceHttpRequest::ExperienceHygieneQuarantine(request) => {
            let _active = state.begin_engine_request(
                request_id,
                "experience-hygiene-quarantine",
                "experience hygiene quarantine",
            );
            handle_experience_hygiene_quarantine(args, stream, request_id, request)
        }
        ModelServiceHttpRequest::ExperienceRepair(request) => {
            let _active =
                state.begin_engine_request(request_id, "experience-repair", "experience repair");
            handle_experience_repair(args, stream, request_id, request)
        }
        ModelServiceHttpRequest::ExperienceRetrieval(request) => {
            let _active =
                state.begin_engine_request(request_id, "experience-retrieval", &request.prompt);
            let engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            handle_experience_retrieval(&engine, args, stream, request_id, request)
        }
        ModelServiceHttpRequest::ModelPoolManifest => {
            handle_model_pool_manifest(args, stream, request_id)
        }
        ModelServiceHttpRequest::ModelPoolStatus => {
            handle_model_pool_status(args, stream, request_id)
        }
        ModelServiceHttpRequest::ModelPoolRoute(request) => {
            handle_model_pool_route(args, stream, request_id, request)
        }
        ModelServiceHttpRequest::ModelPoolCall(request) => {
            handle_model_pool_call(args, stream, request_id, request)
        }
        ModelServiceHttpRequest::Info(endpoint) => {
            handle_endpoint_info(stream, request_id, endpoint)
        }
        ModelServiceHttpRequest::State => {
            let _active = state.begin_engine_request(request_id, "state", "state inspection");
            let engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            handle_state(&engine, args, stream, request_id)
        }
        ModelServiceHttpRequest::Generate(request) => {
            let _active = state.begin_engine_request(request_id, "generate", &request.prompt);
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            let mut backend = backend
                .lock()
                .map_err(|_| std::io::Error::other("model service backend lock poisoned"))?;
            handle_generate(
                &mut engine,
                &mut **backend,
                GenerationHandlerContext {
                    state,
                    args,
                    stream,
                    request_id,
                    endpoint: "generate",
                },
                request,
            )
        }
        ModelServiceHttpRequest::GenerateStream(request) => {
            let _active =
                state.begin_engine_request(request_id, "generate-stream", &request.prompt);
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            let mut backend = backend
                .lock()
                .map_err(|_| std::io::Error::other("model service backend lock poisoned"))?;
            handle_generate_stream(
                &mut engine,
                &mut **backend,
                GenerationHandlerContext {
                    state,
                    args,
                    stream,
                    request_id,
                    endpoint: "generate-stream",
                },
                request,
            )
        }
        ModelServiceHttpRequest::Chat(request) => {
            let prompt_preview = chat_prompt_preview(&request);
            let _active = state.begin_engine_request(request_id, "chat", &prompt_preview);
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            let mut backend = backend
                .lock()
                .map_err(|_| std::io::Error::other("model service backend lock poisoned"))?;
            handle_chat(
                &mut engine,
                &mut **backend,
                state,
                args,
                stream,
                request_id,
                request,
            )
        }
        ModelServiceHttpRequest::ChatStream(request) => {
            let prompt_preview = chat_prompt_preview(&request);
            let _active = state.begin_engine_request(request_id, "chat-stream", &prompt_preview);
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            let mut backend = backend
                .lock()
                .map_err(|_| std::io::Error::other("model service backend lock poisoned"))?;
            handle_chat_stream(
                &mut engine,
                &mut **backend,
                state,
                args,
                stream,
                request_id,
                request,
            )
        }
        ModelServiceHttpRequest::BusinessCycle(request) => {
            let _active = state.begin_engine_request(request_id, "business-cycle", &request.prompt);
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            let mut backend = backend
                .lock()
                .map_err(|_| std::io::Error::other("model service backend lock poisoned"))?;
            handle_business_cycle(
                &mut engine,
                &mut **backend,
                state,
                args,
                stream,
                request_id,
                request,
            )
        }
        ModelServiceHttpRequest::BusinessCycleStream(request) => {
            let _active =
                state.begin_engine_request(request_id, "business-cycle-stream", &request.prompt);
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            let mut backend = backend
                .lock()
                .map_err(|_| std::io::Error::other("model service backend lock poisoned"))?;
            handle_business_cycle_stream(
                &mut engine,
                &mut **backend,
                state,
                args,
                stream,
                request_id,
                request,
            )
        }
        ModelServiceHttpRequest::Replay(request) => {
            let _active = state.begin_engine_request(
                request_id,
                "replay",
                &format!("experience replay limit={}", request.limit),
            );
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            handle_replay(&mut engine, args, stream, request_id, request)
        }
        ModelServiceHttpRequest::SelfImprove(request) => {
            let _active = state.begin_engine_request(
                request_id,
                "self-improve",
                &format!("self-improve limit={}", request.limit),
            );
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            handle_self_improve(&mut engine, args, stream, request_id, request)
        }
        ModelServiceHttpRequest::Feedback(request) => {
            let _active = state.begin_engine_request(
                request_id,
                "feedback",
                &format!(
                    "feedback action={} experience_id={:?} memory_id={:?}",
                    request.action.as_str(),
                    request.experience_id,
                    request.memory_id
                ),
            );
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            handle_feedback(&mut engine, args, stream, request_id, request)
        }
        ModelServiceHttpRequest::RustCheck(request) => {
            let _active = state.begin_engine_request(request_id, "rust-check", &request.code);
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            handle_rust_check(&mut engine, args, stream, request_id, request)
        }
        ModelServiceHttpRequest::Inspect(request) => {
            let _active = state.begin_engine_request(request_id, "inspect", "state inspection");
            let engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            handle_inspect(&engine, args, stream, request_id, request)
        }
    }
}

fn chat_prompt_preview(request: &ModelServiceChatRequest) -> String {
    request
        .messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .or_else(|| request.messages.last())
        .map(|message| message.content.clone())
        .unwrap_or_else(|| "chat request".to_owned())
}

fn handle_health(
    stream: &mut TcpStream,
    request_id: usize,
    state: &ModelServiceServerState,
    args: &Args,
) -> std::io::Result<()> {
    write_http_json(
        stream,
        200,
        "OK",
        &model_service_health_json(request_id, state, args),
    )
}
