use std::net::TcpStream;

use rust_norion::{DraftToken, InferenceBackend, NoironEngine, TaskProfile};

use super::super::super::json::{write_http_json, write_http_sse_headers, write_sse_event};
use super::super::super::profile::detect_profile;
use super::super::super::request::{ModelServiceChatRequest, ModelServiceRequest};
use super::super::super::response::model_service_response_json;
use super::super::state::{ModelServiceLastInferenceTelemetry, ModelServiceServerState};
use crate::Args;
use crate::gemma_business::contract::annotate_model_service_business_case_for_timed;
use crate::inference_runner::{
    run_timed_inference_stream_with_options, run_timed_inference_with_options,
};

pub(super) struct GenerationHandlerContext<'a> {
    pub(super) state: &'a ModelServiceServerState,
    pub(super) args: &'a Args,
    pub(super) stream: &'a mut TcpStream,
    pub(super) request_id: usize,
    pub(super) endpoint: &'static str,
}

pub(super) fn handle_chat<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    state: &ModelServiceServerState,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceChatRequest,
) -> std::io::Result<()> {
    handle_generate(
        engine,
        backend,
        GenerationHandlerContext {
            state,
            args,
            stream,
            request_id,
            endpoint: "chat",
        },
        request.into_generate_request(),
    )
}

pub(super) fn handle_chat_stream<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    state: &ModelServiceServerState,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceChatRequest,
) -> std::io::Result<()> {
    handle_generate_stream(
        engine,
        backend,
        GenerationHandlerContext {
            state,
            args,
            stream,
            request_id,
            endpoint: "chat-stream",
        },
        request.into_generate_request(),
    )
}

pub(super) fn handle_generate<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    context: GenerationHandlerContext<'_>,
    request: ModelServiceRequest,
) -> std::io::Result<()> {
    let GenerationHandlerContext {
        state,
        args,
        stream,
        request_id,
        endpoint,
    } = context;
    let profile = request
        .profile
        .unwrap_or_else(|| detect_profile(&request.prompt));
    let case_name = request.case_name.clone();
    let max_tokens = request.max_tokens;
    let mut timed = match run_timed_inference_with_options(
        engine,
        backend,
        request.prompt,
        profile,
        max_tokens,
        args.trace_path.as_ref(),
        case_name.as_deref(),
    ) {
        Ok(timed) => timed,
        Err(error) => {
            state.record_inference(ModelServiceLastInferenceTelemetry::error(
                request_id,
                endpoint,
                error.to_string(),
            ));
            return Err(error);
        }
    };
    if state.is_cancel_requested(request_id) {
        let message = cancellation_message(state, request_id);
        state.record_inference(ModelServiceLastInferenceTelemetry::error(
            request_id, endpoint, message,
        ));
        let body = "{\"ok\":false,\"error\":\"request cancelled by runtime_request_splice\",\"persistent_writes\":false}";
        return write_http_json(stream, 409, "Conflict", body);
    }
    annotate_model_service_business_case_for_timed(
        engine,
        &mut timed,
        case_name.as_deref(),
        args.trace_path.as_ref(),
    )?;
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    state.record_inference(ModelServiceLastInferenceTelemetry::from_timed(
        request_id, endpoint, &timed,
    ));
    let body = model_service_response_json(
        request_id,
        profile,
        args.trace_path.is_some(),
        request.output_mode,
        &timed,
    );
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_generate_stream<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    context: GenerationHandlerContext<'_>,
    request: ModelServiceRequest,
) -> std::io::Result<()> {
    let GenerationHandlerContext {
        state,
        args,
        stream,
        request_id,
        endpoint,
    } = context;
    let profile = request
        .profile
        .unwrap_or_else(|| detect_profile(&request.prompt));
    let case_name = request.case_name.clone();
    let output_mode = request.output_mode;
    let max_tokens = request.max_tokens;

    write_http_sse_headers(stream)?;
    write_sse_event(stream, "status", "rust-norion stream connected")?;
    write_sse_event(
        stream,
        "status",
        &format!(
            "running {endpoint} with profile={} max_tokens={}",
            profile_name_for_sse(profile),
            option_usize_for_sse(max_tokens)
        ),
    )?;

    let mut token_count = 0_usize;
    let mut token_write_error = None;
    let mut cancel_requested = false;
    let timed_result = {
        let mut on_token = |token: &DraftToken| {
            if cancel_requested {
                return;
            }
            if state.is_cancel_requested(request_id) {
                cancel_requested = true;
                return;
            }
            token_count += 1;
            if token_write_error.is_none()
                && let Err(error) = write_sse_event(stream, "delta", &token.text)
            {
                token_write_error = Some(error);
            }
        };
        run_timed_inference_stream_with_options(
            engine,
            backend,
            request.prompt,
            profile,
            max_tokens,
            args.trace_path.as_ref(),
            case_name.as_deref(),
            &mut on_token,
        )
    };
    if let Some(error) = token_write_error {
        return Err(error);
    }
    if cancel_requested || state.is_cancel_requested(request_id) {
        let message = cancellation_message(state, request_id);
        state.record_inference(ModelServiceLastInferenceTelemetry::error(
            request_id,
            endpoint,
            message.clone(),
        ));
        write_sse_event(stream, "error", &message)?;
        write_sse_event(stream, "done", "[DONE]")?;
        return Ok(());
    }

    let mut timed = match timed_result {
        Ok(timed) => timed,
        Err(error) => {
            state.record_inference(ModelServiceLastInferenceTelemetry::error(
                request_id,
                endpoint,
                error.to_string(),
            ));
            write_sse_event(stream, "error", &error.to_string())?;
            write_sse_event(stream, "done", "[DONE]")?;
            return Ok(());
        }
    };

    if let Err(error) = annotate_model_service_business_case_for_timed(
        engine,
        &mut timed,
        case_name.as_deref(),
        args.trace_path.as_ref(),
    )
    .and_then(|_| {
        engine.save_full_state(
            &args.memory_path,
            &args.experience_path,
            &args.adaptive_path,
        )
    }) {
        state.record_inference(ModelServiceLastInferenceTelemetry::error(
            request_id,
            endpoint,
            error.to_string(),
        ));
        write_sse_event(stream, "error", &error.to_string())?;
        write_sse_event(stream, "done", "[DONE]")?;
        return Ok(());
    }

    state.record_inference(ModelServiceLastInferenceTelemetry::from_timed(
        request_id, endpoint, &timed,
    ));
    write_sse_event(
        stream,
        "meta",
        &format!(
            "streamed_tokens={} runtime_tokens={} elapsed_ms={}",
            token_count, timed.outcome.runtime_token_metrics.token_count, timed.elapsed_ms
        ),
    )?;
    let body = model_service_response_json(
        request_id,
        profile,
        args.trace_path.is_some(),
        output_mode,
        &timed,
    );
    write_sse_event(stream, "final", &body)?;
    write_sse_event(stream, "done", "[DONE]")
}

fn profile_name_for_sse(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long",
    }
}

fn option_usize_for_sse(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "backend-default".to_owned())
}

fn cancellation_message(state: &ModelServiceServerState, request_id: usize) -> String {
    state
        .cancellation_intent(request_id)
        .map(|cancellation| {
            format!(
                "request cancelled by {}; repair_factor={} retag_label={} reason={}",
                cancellation.repair_factor,
                cancellation.repair_factor,
                cancellation.retag_label,
                cancellation.reason
            )
        })
        .unwrap_or_else(|| "request cancelled by runtime_request_splice".to_owned())
}
