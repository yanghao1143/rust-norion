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
use self::endpoint_info::{handle_endpoint_info, handle_model_capabilities};
use self::evolution::{handle_feedback, handle_replay, handle_rust_check, handle_self_improve};
use self::experience_cleanup_audit::handle_experience_cleanup_audit;
use self::experience_hygiene::{handle_experience_hygiene, handle_experience_hygiene_quarantine};
use self::experience_repair::handle_experience_repair;
use self::experience_retrieval::handle_experience_retrieval;
use self::generation::{
    GenerationHandlerContext, handle_chat, handle_chat_stream, handle_generate,
    handle_generate_stream, handle_openai_chat_completions, handle_openai_chat_completions_stream,
    handle_openai_completions,
};
use self::inspection::{handle_inspect, handle_state};
use self::model_pool::{
    handle_model_pool_call, handle_model_pool_manifest, handle_model_pool_route,
    handle_model_pool_status,
};
use super::super::http::read_http_request;
use super::super::json::{
    option_str_service_json, service_error_json, service_json_string, write_http_json,
};
use super::super::request::{
    ModelServiceChatRequest, ModelServiceHttpRequest, ModelServiceRequestCancelRequest,
    parse_model_service_http_request,
};
use super::health::model_service_health_json;
use super::state::{
    ModelServiceBackpressureRejection, ModelServiceLastInferenceTelemetry, ModelServiceServerState,
};
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
            handle_model_pool_route(args, stream, request_id, state, request)
        }
        ModelServiceHttpRequest::ModelPoolCall(request) => {
            handle_model_pool_call(args, stream, request_id, request)
        }
        ModelServiceHttpRequest::ModelCapabilities => {
            handle_model_capabilities(stream, request_id, args)
        }
        ModelServiceHttpRequest::Info(endpoint) => {
            handle_endpoint_info(stream, request_id, endpoint)
        }
        ModelServiceHttpRequest::RequestCancel(request) => {
            handle_request_cancel(stream, request_id, state, request)
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
            let _active = match state.try_begin_stream_engine_request(
                request_id,
                "generate-stream",
                &request.prompt,
            ) {
                Ok(active) => active,
                Err(rejection) => return handle_backpressure_rejection(stream, state, rejection),
            };
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
        ModelServiceHttpRequest::OpenAiChatCompletions(request) => {
            let prompt_preview = chat_prompt_preview(&request);
            let _active =
                state.begin_engine_request(request_id, "chat-completions", &prompt_preview);
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            let mut backend = backend
                .lock()
                .map_err(|_| std::io::Error::other("model service backend lock poisoned"))?;
            handle_openai_chat_completions(
                &mut engine,
                &mut **backend,
                state,
                args,
                stream,
                request_id,
                request,
            )
        }
        ModelServiceHttpRequest::OpenAiCompletions(request) => {
            let _active =
                state.begin_engine_request(request_id, "completions", &request.generate.prompt);
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            let mut backend = backend
                .lock()
                .map_err(|_| std::io::Error::other("model service backend lock poisoned"))?;
            handle_openai_completions(
                &mut engine,
                &mut **backend,
                state,
                args,
                stream,
                request_id,
                request,
            )
        }
        ModelServiceHttpRequest::OpenAiChatCompletionsStream(request) => {
            let prompt_preview = chat_prompt_preview(&request);
            let _active = match state.try_begin_stream_engine_request(
                request_id,
                "chat-completions-stream",
                &prompt_preview,
            ) {
                Ok(active) => active,
                Err(rejection) => return handle_backpressure_rejection(stream, state, rejection),
            };
            let mut engine = engine
                .lock()
                .map_err(|_| std::io::Error::other("model service engine lock poisoned"))?;
            let mut backend = backend
                .lock()
                .map_err(|_| std::io::Error::other("model service backend lock poisoned"))?;
            handle_openai_chat_completions_stream(
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
            let _active = match state.try_begin_stream_engine_request(
                request_id,
                "chat-stream",
                &prompt_preview,
            ) {
                Ok(active) => active,
                Err(rejection) => return handle_backpressure_rejection(stream, state, rejection),
            };
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
            let _active = match state.try_begin_stream_engine_request(
                request_id,
                "business-cycle-stream",
                &request.prompt,
            ) {
                Ok(active) => active,
                Err(rejection) => return handle_backpressure_rejection(stream, state, rejection),
            };
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

fn handle_backpressure_rejection(
    stream: &mut TcpStream,
    state: &ModelServiceServerState,
    rejection: ModelServiceBackpressureRejection,
) -> std::io::Result<()> {
    let message = rejection.message();
    state.record_inference(ModelServiceLastInferenceTelemetry::error(
        rejection.request_id,
        rejection.endpoint.clone(),
        message.clone(),
    ));
    let body = format!(
        "{{\"ok\":false,\"error\":{},\"request_id\":{},\"endpoint\":{},\"active_engine_requests\":{},\"max_active_engine_requests\":{},\"retryable\":true,\"persistent_writes\":false}}",
        service_json_string(&message),
        rejection.request_id,
        service_json_string(&rejection.endpoint),
        rejection.active_engine_requests,
        rejection.max_active_engine_requests
    );
    write_http_json(stream, 429, "Too Many Requests", &body)
}

fn handle_request_cancel(
    stream: &mut TcpStream,
    request_id: usize,
    state: &ModelServiceServerState,
    request: ModelServiceRequestCancelRequest,
) -> std::io::Result<()> {
    let cancellation =
        state.request_cancel(request.request_id, request.reason, request.retag_label);
    let body = format!(
        "{{\"ok\":true,\"request_id\":{},\"target_request_id\":{},\"target_active\":{},\"target_endpoint\":{},\"repair_factor_released\":{},\"repair_factor\":{},\"retag_applied\":{},\"retag_label\":{},\"reason\":{},\"cooperative_only\":true,\"persistent_writes\":false,\"next_step\":{}}}",
        request_id,
        cancellation.request_id,
        cancellation.target_active,
        option_str_service_json(cancellation.endpoint.as_deref()),
        cancellation.target_active,
        service_json_string(&cancellation.repair_factor),
        cancellation.target_active,
        service_json_string(&cancellation.retag_label),
        service_json_string(&cancellation.reason),
        service_json_string(if cancellation.target_active {
            "active request will observe the repair factor at stream and stage boundaries"
        } else {
            "target request is not active; no repair factor was held"
        })
    );
    write_http_json(stream, 200, "OK", &body)
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
