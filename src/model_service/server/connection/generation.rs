use std::io::Write;
use std::net::TcpStream;

use rust_norion::{DraftToken, InferenceBackend, NoironEngine, TaskProfile};

use super::super::super::json::{
    service_json_string, write_http_json, write_http_sse_headers, write_sse_event,
};
use super::super::super::profile::detect_profile;
use super::super::super::request::{
    ModelServiceChatRequest, ModelServiceOpenAiCompletionRequest, ModelServiceRequest,
};
use super::super::super::response::{
    ModelServiceTaskIntentMetadata, model_service_response_json,
    model_service_task_intent_metadata, model_service_task_metadata_json,
    openai_chat_completion_response_json, openai_completion_response_json,
    openai_norion_runtime_metadata_json,
};
use super::super::state::{ModelServiceLastInferenceTelemetry, ModelServiceServerState};
use crate::Args;
use crate::gemma_business::contract::annotate_model_service_business_case_for_timed;
use crate::inference_runner::{
    run_timed_inference_stream_checked_with_scope_options, run_timed_inference_with_scope_options,
};
use crate::model_service::types::TimedOutcome;

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

pub(super) fn handle_openai_chat_completions<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    state: &ModelServiceServerState,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceChatRequest,
) -> std::io::Result<()> {
    let model = request.model.clone();
    handle_generate_with_response(
        engine,
        backend,
        GenerationHandlerContext {
            state,
            args,
            stream,
            request_id,
            endpoint: "chat-completions",
        },
        request.into_generate_request(),
        GenerationResponseFormat::OpenAiChatCompletion { model },
    )
}

pub(super) fn handle_openai_completions<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    state: &ModelServiceServerState,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceOpenAiCompletionRequest,
) -> std::io::Result<()> {
    handle_generate_with_response(
        engine,
        backend,
        GenerationHandlerContext {
            state,
            args,
            stream,
            request_id,
            endpoint: "completions",
        },
        request.generate,
        GenerationResponseFormat::OpenAiCompletion {
            model: request.model,
        },
    )
}

pub(super) fn handle_openai_chat_completions_stream<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    state: &ModelServiceServerState,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceChatRequest,
) -> std::io::Result<()> {
    let model = request.model.clone();
    handle_generate_stream_with_response(
        engine,
        backend,
        GenerationHandlerContext {
            state,
            args,
            stream,
            request_id,
            endpoint: "chat-completions-stream",
        },
        request.into_generate_request(),
        StreamResponseFormat::OpenAiChatCompletion {
            model: openai_model_name(model.as_deref()),
            created: unix_timestamp_seconds(),
        },
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
    handle_generate_with_response(
        engine,
        backend,
        context,
        request,
        GenerationResponseFormat::ModelService,
    )
}

enum GenerationResponseFormat {
    ModelService,
    OpenAiCompletion { model: Option<String> },
    OpenAiChatCompletion { model: Option<String> },
}

fn handle_generate_with_response<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    context: GenerationHandlerContext<'_>,
    request: ModelServiceRequest,
    response_format: GenerationResponseFormat,
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
    let task_intent = model_service_task_intent_metadata(&request.prompt, profile);
    let case_name = request.case_name.clone();
    let tenant_scope = request.tenant_scope;
    let max_tokens = request.max_tokens;
    let mut timed = match run_timed_inference_with_scope_options(
        engine,
        backend,
        request.prompt,
        profile,
        max_tokens,
        tenant_scope,
        args.trace_path.as_ref(),
        case_name.as_deref(),
    ) {
        Ok(timed) => timed,
        Err(error) => {
            let message = error.to_string();
            state.record_inference(ModelServiceLastInferenceTelemetry::error(
                request_id,
                endpoint,
                message.clone(),
            ));
            return write_generation_error_json(
                stream,
                &response_format,
                request_id,
                endpoint,
                &message,
                "runtime_error",
                io_error_is_timeout(&error),
                true,
                None,
            );
        }
    };
    if let Some(note) = runtime_error_note(&timed) {
        let message = runtime_error_message(&timed);
        let timeout = runtime_error_note_is_timeout(note);
        state.record_inference(ModelServiceLastInferenceTelemetry::error(
            request_id,
            endpoint,
            message.clone(),
        ));
        return write_generation_error_json(
            stream,
            &response_format,
            request_id,
            endpoint,
            &message,
            "runtime_error",
            timeout,
            true,
            Some(note),
        );
    }
    if state.is_cancel_requested(request_id) {
        let message = cancellation_message(state, request_id);
        state.record_inference(ModelServiceLastInferenceTelemetry::error(
            request_id, endpoint, message,
        ));
        let body = generation_cancelled_after_inference_json(
            &response_format,
            request_id,
            endpoint,
            &timed,
        );
        return write_http_json(stream, 409, "Conflict", &body);
    }
    if let Err(error) = annotate_model_service_business_case_for_timed(
        engine,
        &mut timed,
        case_name.as_deref(),
        args.trace_path.as_ref(),
    ) {
        let message = error.to_string();
        state.record_inference(ModelServiceLastInferenceTelemetry::error(
            request_id,
            endpoint,
            message.clone(),
        ));
        return write_generation_error_json(
            stream,
            &response_format,
            request_id,
            endpoint,
            &message,
            "post_inference_error",
            io_error_is_timeout(&error),
            false,
            None,
        );
    }
    if let Err(error) = engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    ) {
        let message = error.to_string();
        state.record_inference(ModelServiceLastInferenceTelemetry::error(
            request_id,
            endpoint,
            message.clone(),
        ));
        return write_generation_error_json(
            stream,
            &response_format,
            request_id,
            endpoint,
            &message,
            "persistence_error",
            io_error_is_timeout(&error),
            false,
            None,
        );
    }
    state.record_inference(ModelServiceLastInferenceTelemetry::from_timed(
        request_id, endpoint, &timed,
    ));
    let body = match response_format {
        GenerationResponseFormat::ModelService => model_service_response_json(
            request_id,
            profile,
            args.trace_path.is_some(),
            request.output_mode,
            task_intent,
            &timed,
        ),
        GenerationResponseFormat::OpenAiCompletion { model } => openai_completion_response_json(
            request_id,
            profile,
            model.as_deref(),
            request.output_mode,
            task_intent,
            &timed,
        ),
        GenerationResponseFormat::OpenAiChatCompletion { model } => {
            openai_chat_completion_response_json(
                request_id,
                profile,
                model.as_deref(),
                request.output_mode,
                task_intent,
                &timed,
            )
        }
    };
    write_http_json(stream, 200, "OK", &body)
}

fn write_generation_error_json(
    stream: &mut TcpStream,
    response_format: &GenerationResponseFormat,
    request_id: usize,
    endpoint: &str,
    message: &str,
    error_type: &str,
    timeout: bool,
    retryable: bool,
    runtime_error_note: Option<&str>,
) -> std::io::Result<()> {
    let body = generation_error_json(
        response_format,
        request_id,
        endpoint,
        message,
        error_type,
        timeout,
        retryable,
        runtime_error_note,
    );
    let (status, reason) = generation_error_status(error_type, timeout);
    write_http_json(stream, status, reason, &body)
}

fn generation_error_json(
    response_format: &GenerationResponseFormat,
    request_id: usize,
    endpoint: &str,
    message: &str,
    error_type: &str,
    timeout: bool,
    retryable: bool,
    runtime_error_note: Option<&str>,
) -> String {
    let note_json = runtime_error_note
        .map(service_json_string)
        .unwrap_or_else(|| "null".to_owned());
    match response_format {
        GenerationResponseFormat::ModelService => format!(
            "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"error\":{},\"error_type\":\"{}\",\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
            request_id,
            service_json_string(endpoint),
            service_json_string(message),
            error_type,
            timeout,
            retryable,
            note_json
        ),
        GenerationResponseFormat::OpenAiCompletion { model }
        | GenerationResponseFormat::OpenAiChatCompletion { model } => {
            let model = openai_model_name(model.as_deref());
            format!(
                "{{\"ok\":false,\"error\":{{\"message\":{},\"type\":\"{}\",\"param\":null,\"code\":null}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}}}",
                service_json_string(message),
                error_type,
                request_id,
                service_json_string(endpoint),
                service_json_string(&model),
                timeout,
                retryable,
                note_json
            )
        }
    }
}

fn generation_cancelled_after_inference_json(
    response_format: &GenerationResponseFormat,
    request_id: usize,
    endpoint: &str,
    timed: &TimedOutcome,
) -> String {
    let compute_budget = &timed.outcome.compute_budget_schedule;
    let fanout_reduction = compute_budget
        .route_fanout_before
        .saturating_sub(compute_budget.route_fanout_after);
    match response_format {
        GenerationResponseFormat::ModelService => format!(
            "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"error\":\"request cancelled by runtime_request_splice\",\"error_type\":\"cancelled\",\"cancelled\":true,\"timeout\":false,\"retryable\":false,\"compute_budget\":{},\"compute_budget_summary\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_kv_lookups_skipped\":{},\"compute_budget_fanout_reduction\":{},\"compute_budget_read_only\":{},\"compute_budget_write_allowed\":{},\"compute_budget_applied\":{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
            request_id,
            service_json_string(endpoint),
            service_json_string(compute_budget.compute_budget.as_str()),
            service_json_string(&compute_budget.summary_line()),
            compute_budget.saved_tokens,
            compute_budget.wasted_compute_avoided_tokens,
            compute_budget.kv_lookups_skipped,
            fanout_reduction,
            compute_budget.read_only,
            compute_budget.write_allowed,
            compute_budget.applied
        ),
        GenerationResponseFormat::OpenAiCompletion { model }
        | GenerationResponseFormat::OpenAiChatCompletion { model } => {
            let model = openai_model_name(model.as_deref());
            format!(
                "{{\"ok\":false,\"error\":{{\"message\":\"request cancelled by runtime_request_splice\",\"type\":\"cancelled\",\"param\":null,\"code\":null}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"cancelled\":true,\"timeout\":false,\"retryable\":false,\"compute_budget\":{},\"compute_budget_summary\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_kv_lookups_skipped\":{},\"compute_budget_fanout_reduction\":{},\"compute_budget_read_only\":{},\"compute_budget_write_allowed\":{},\"compute_budget_applied\":{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}}}",
                request_id,
                service_json_string(endpoint),
                service_json_string(&model),
                service_json_string(compute_budget.compute_budget.as_str()),
                service_json_string(&compute_budget.summary_line()),
                compute_budget.saved_tokens,
                compute_budget.wasted_compute_avoided_tokens,
                compute_budget.kv_lookups_skipped,
                fanout_reduction,
                compute_budget.read_only,
                compute_budget.write_allowed,
                compute_budget.applied
            )
        }
    }
}

fn generation_error_status(error_type: &str, timeout: bool) -> (u16, &'static str) {
    if timeout {
        return (504, "Gateway Timeout");
    }
    if error_type == "runtime_error" {
        return (502, "Bad Gateway");
    }
    (500, "Internal Server Error")
}

fn runtime_error_note(timed: &TimedOutcome) -> Option<&str> {
    timed
        .outcome
        .process_reward
        .notes
        .iter()
        .find(|note| note.starts_with("runtime_error:"))
        .map(String::as_str)
}

fn runtime_error_note_is_timeout(note: &str) -> bool {
    note.split(':').any(|field| field == "timeout=true")
}

fn runtime_error_message(timed: &TimedOutcome) -> String {
    let raw_answer = timed.outcome.raw_answer.trim();
    if raw_answer.contains("Runtime backend error:") {
        raw_answer.to_owned()
    } else {
        "runtime adapter failed".to_owned()
    }
}

fn io_error_is_timeout(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) || error.to_string().to_ascii_lowercase().contains("timeout")
}

pub(super) fn handle_generate_stream<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    context: GenerationHandlerContext<'_>,
    request: ModelServiceRequest,
) -> std::io::Result<()> {
    handle_generate_stream_with_response(
        engine,
        backend,
        context,
        request,
        StreamResponseFormat::ModelService,
    )
}

enum StreamResponseFormat {
    ModelService,
    OpenAiChatCompletion { model: String, created: u64 },
}

fn handle_generate_stream_with_response<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    context: GenerationHandlerContext<'_>,
    request: ModelServiceRequest,
    response_format: StreamResponseFormat,
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
    let task_intent = model_service_task_intent_metadata(&request.prompt, profile);
    let case_name = request.case_name.clone();
    let output_mode = request.output_mode;
    let tenant_scope = request.tenant_scope;
    let max_tokens = request.max_tokens;

    write_http_sse_headers(stream)?;
    if matches!(response_format, StreamResponseFormat::ModelService) {
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
    }

    let mut token_count = 0_usize;
    let mut cancel_requested = false;
    let mut stream_write_failed = false;
    let timed_result = {
        let mut on_token = |token: &DraftToken| {
            if cancel_requested {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    cancellation_message(state, request_id),
                ));
            }
            if state.is_cancel_requested(request_id) {
                cancel_requested = true;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    cancellation_message(state, request_id),
                ));
            }
            token_count += 1;
            let write_result = match &response_format {
                StreamResponseFormat::ModelService => write_sse_event(stream, "delta", &token.text),
                StreamResponseFormat::OpenAiChatCompletion { model, created } => {
                    let body = openai_chat_completion_stream_delta_json(
                        request_id,
                        model,
                        *created,
                        &token.text,
                    );
                    write_openai_sse_data(stream, &body)
                }
            };
            write_result.map_err(|error| {
                stream_write_failed = true;
                error
            })
        };
        run_timed_inference_stream_checked_with_scope_options(
            engine,
            backend,
            request.prompt,
            profile,
            max_tokens,
            tenant_scope,
            args.trace_path.as_ref(),
            case_name.as_deref(),
            &mut on_token,
        )
    };
    if cancel_requested || state.is_cancel_requested(request_id) {
        let message = cancellation_message(state, request_id);
        state.record_inference(ModelServiceLastInferenceTelemetry::error(
            request_id,
            endpoint,
            message.clone(),
        ));
        match &response_format {
            StreamResponseFormat::ModelService => {
                write_sse_event(stream, "error", &message)?;
                let final_body =
                    stream_cancel_final_json(request_id, endpoint, token_count, &message);
                write_sse_event(stream, "final", &final_body)?;
                write_sse_event(stream, "done", "[DONE]")?;
            }
            StreamResponseFormat::OpenAiChatCompletion { model, created } => {
                let body = openai_chat_completion_stream_error_json(OpenAiStreamErrorContext {
                    request_id,
                    model,
                    created: *created,
                    endpoint,
                    streamed_tokens: token_count,
                    message: &message,
                    cancelled: true,
                    timeout: false,
                });
                write_openai_sse_data(stream, &body)?;
                write_openai_sse_data(stream, "[DONE]")?;
            }
        }
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
            if stream_write_failed {
                return Err(error);
            }
            match &response_format {
                StreamResponseFormat::ModelService => {
                    write_sse_event(stream, "error", &error.to_string())?;
                    let final_body =
                        stream_error_final_json(request_id, endpoint, token_count, &error);
                    write_sse_event(stream, "final", &final_body)?;
                    write_sse_event(stream, "done", "[DONE]")?;
                }
                StreamResponseFormat::OpenAiChatCompletion { model, created } => {
                    let message = error.to_string();
                    let body = openai_chat_completion_stream_error_json(OpenAiStreamErrorContext {
                        request_id,
                        model,
                        created: *created,
                        endpoint,
                        streamed_tokens: token_count,
                        message: &message,
                        cancelled: false,
                        timeout: stream_error_is_timeout(&error),
                    });
                    write_openai_sse_data(stream, &body)?;
                    write_openai_sse_data(stream, "[DONE]")?;
                }
            }
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
        match &response_format {
            StreamResponseFormat::ModelService => {
                write_sse_event(stream, "error", &error.to_string())?;
                let final_body = stream_error_final_json(request_id, endpoint, token_count, &error);
                write_sse_event(stream, "final", &final_body)?;
                write_sse_event(stream, "done", "[DONE]")?;
            }
            StreamResponseFormat::OpenAiChatCompletion { model, created } => {
                let message = error.to_string();
                let body = openai_chat_completion_stream_error_json(OpenAiStreamErrorContext {
                    request_id,
                    model,
                    created: *created,
                    endpoint,
                    streamed_tokens: token_count,
                    message: &message,
                    cancelled: false,
                    timeout: stream_error_is_timeout(&error),
                });
                write_openai_sse_data(stream, &body)?;
                write_openai_sse_data(stream, "[DONE]")?;
            }
        }
        return Ok(());
    }

    state.record_inference(ModelServiceLastInferenceTelemetry::from_timed(
        request_id, endpoint, &timed,
    ));
    match &response_format {
        StreamResponseFormat::ModelService => {
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
                task_intent,
                &timed,
            );
            let body = stream_success_final_json(&body, endpoint, token_count, &timed);
            write_sse_event(stream, "final", &body)?;
            write_sse_event(stream, "done", "[DONE]")
        }
        StreamResponseFormat::OpenAiChatCompletion { model, created } => {
            let body = openai_chat_completion_stream_final_json(
                request_id,
                model,
                *created,
                endpoint,
                token_count,
                profile,
                task_intent,
                &timed,
            );
            write_openai_sse_data(stream, &body)?;
            write_openai_sse_data(stream, "[DONE]")
        }
    }
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

fn stream_cancel_final_json(
    request_id: usize,
    endpoint: &str,
    streamed_tokens: usize,
    message: &str,
) -> String {
    format!(
        "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"stream_state\":\"interrupted\",\"cancelled\":true,\"timeout\":false,\"partial_result\":{},\"partial_finalized\":true,\"streamed_tokens\":{},\"queue_time_ms\":0,\"cancellation_reason\":{},\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_interrupted_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,\"error\":{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
        request_id,
        service_json_string(endpoint),
        streamed_tokens > 0,
        streamed_tokens,
        service_json_string(message),
        service_json_string(message)
    )
}

fn stream_error_final_json(
    request_id: usize,
    endpoint: &str,
    streamed_tokens: usize,
    error: &std::io::Error,
) -> String {
    let message = error.to_string();
    let timeout = matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) || message.to_ascii_lowercase().contains("timeout");
    format!(
        "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"stream_state\":\"failed\",\"cancelled\":false,\"timeout\":{},\"partial_result\":{},\"partial_finalized\":true,\"streamed_tokens\":{},\"queue_time_ms\":0,\"cancellation_reason\":null,\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,\"error\":{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
        request_id,
        service_json_string(endpoint),
        timeout,
        streamed_tokens > 0,
        streamed_tokens,
        service_json_string(&message)
    )
}

fn stream_error_is_timeout(error: &std::io::Error) -> bool {
    let message = error.to_string();
    matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) || message.to_ascii_lowercase().contains("timeout")
}

fn stream_success_final_json(
    response_json: &str,
    endpoint: &str,
    streamed_tokens: usize,
    timed: &TimedOutcome,
) -> String {
    let Some(response_prefix) = response_json.strip_suffix('}') else {
        return response_json.to_owned();
    };
    format!(
        "{},\"endpoint\":{},\"stream_state\":\"completed\",\"cancelled\":false,\"timeout\":false,\"partial_result\":false,\"partial_finalized\":false,\"streamed_tokens\":{},\"queue_time_ms\":0,\"cancellation_reason\":null,\"compute_budget_summary\":{},\"persistent_writes\":true}}",
        response_prefix,
        service_json_string(endpoint),
        streamed_tokens,
        service_json_string(&timed.outcome.compute_budget_schedule.summary_line())
    )
}

fn openai_model_name(model: Option<&str>) -> String {
    model
        .filter(|model| !model.trim().is_empty())
        .unwrap_or("rust-norion-local")
        .to_owned()
}

fn unix_timestamp_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn write_openai_sse_data(stream: &mut TcpStream, data: &str) -> std::io::Result<()> {
    stream.write_all(format!("data: {data}\n\n").as_bytes())?;
    stream.flush()
}

fn openai_chat_completion_stream_delta_json(
    request_id: usize,
    model: &str,
    created: u64,
    content: &str,
) -> String {
    format!(
        "{{\"id\":\"chatcmpl-norion-{}\",\"object\":\"chat.completion.chunk\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"delta\":{{\"role\":\"assistant\",\"content\":{}}},\"finish_reason\":null}}]}}",
        request_id,
        created,
        service_json_string(model),
        service_json_string(content)
    )
}

fn openai_chat_completion_stream_final_json(
    request_id: usize,
    model: &str,
    created: u64,
    endpoint: &str,
    streamed_tokens: usize,
    profile: TaskProfile,
    task_intent: ModelServiceTaskIntentMetadata,
    timed: &TimedOutcome,
) -> String {
    let runtime_metadata = openai_norion_runtime_metadata_json(&timed.outcome);
    let task_metadata = model_service_task_metadata_json(&timed.outcome, task_intent);
    format!(
        "{{\"id\":\"chatcmpl-norion-{}\",\"object\":\"chat.completion.chunk\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}],\"norion\":{{\"request_id\":{},\"endpoint\":{},\"profile\":\"{}\",{},\"stream_state\":\"completed\",\"streamed_tokens\":{}, {},\"elapsed_ms\":{},\"persistent_writes\":true,\"memory_write_allowed\":true,\"genome_write_allowed\":true,\"self_evolution_write_allowed\":true}}}}",
        request_id,
        created,
        service_json_string(model),
        request_id,
        service_json_string(endpoint),
        profile_name_for_sse(profile),
        task_metadata,
        streamed_tokens,
        runtime_metadata,
        timed.elapsed_ms
    )
}

struct OpenAiStreamErrorContext<'a> {
    request_id: usize,
    model: &'a str,
    created: u64,
    endpoint: &'a str,
    streamed_tokens: usize,
    message: &'a str,
    cancelled: bool,
    timeout: bool,
}

fn openai_chat_completion_stream_error_json(context: OpenAiStreamErrorContext<'_>) -> String {
    let error_type = if context.cancelled {
        "cancelled"
    } else if context.timeout {
        "timeout"
    } else {
        "runtime_error"
    };
    let compute_budget_summary = if context.cancelled {
        "unavailable_interrupted_before_final_outcome"
    } else {
        "unavailable_failed_before_final_outcome"
    };
    format!(
        "{{\"id\":\"chatcmpl-norion-{}\",\"object\":\"chat.completion.chunk\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}],\"error\":{{\"message\":{},\"type\":\"{}\"}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"stream_state\":\"failed\",\"cancelled\":{},\"timeout\":{},\"streamed_tokens\":{},\"compute_budget\":null,\"compute_budget_summary\":{},\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}}}",
        context.request_id,
        context.created,
        service_json_string(context.model),
        service_json_string(context.message),
        error_type,
        context.request_id,
        service_json_string(context.endpoint),
        context.cancelled,
        context.timeout,
        context.streamed_tokens,
        service_json_string(compute_budget_summary)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_cancel_final_json_preserves_partial_count_and_blocks_writes() {
        let body = stream_cancel_final_json(
            7,
            "generate-stream",
            3,
            "request cancelled by runtime_request_splice",
        );

        assert!(body.contains("\"request_id\":7"));
        assert!(body.contains("\"endpoint\":\"generate-stream\""));
        assert!(body.contains("\"stream_state\":\"interrupted\""));
        assert!(body.contains("\"cancelled\":true"));
        assert!(body.contains("\"timeout\":false"));
        assert!(body.contains("\"partial_result\":true"));
        assert!(body.contains("\"partial_finalized\":true"));
        assert!(body.contains("\"streamed_tokens\":3"));
        assert!(body.contains("\"queue_time_ms\":0"));
        assert!(
            body.contains(
                "\"cancellation_reason\":\"request cancelled by runtime_request_splice\""
            )
        );
        assert!(body.contains(
            "\"compute_budget_summary\":\"unavailable_interrupted_before_final_outcome\""
        ));
        assert_failed_stream_compute_budget_fields(&body);
        assert!(body.contains("\"persistent_writes\":false"));
        assert!(body.contains("\"memory_write_allowed\":false"));
        assert!(body.contains("\"genome_write_allowed\":false"));
        assert!(body.contains("\"self_evolution_write_allowed\":false"));
    }

    #[test]
    fn stream_error_final_json_marks_timeout_and_blocks_writes() {
        let body = stream_error_final_json(
            8,
            "generate-stream",
            2,
            &std::io::Error::new(std::io::ErrorKind::TimedOut, "runtime timeout"),
        );

        assert!(body.contains("\"request_id\":8"));
        assert!(body.contains("\"stream_state\":\"failed\""));
        assert!(body.contains("\"cancelled\":false"));
        assert!(body.contains("\"timeout\":true"));
        assert!(body.contains("\"partial_result\":true"));
        assert!(body.contains("\"partial_finalized\":true"));
        assert!(body.contains("\"streamed_tokens\":2"));
        assert!(body.contains("\"queue_time_ms\":0"));
        assert!(body.contains("\"cancellation_reason\":null"));
        assert!(
            body.contains("\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\"")
        );
        assert_failed_stream_compute_budget_fields(&body);
        assert!(body.contains("\"persistent_writes\":false"));
        assert!(body.contains("\"memory_write_allowed\":false"));
        assert!(body.contains("\"genome_write_allowed\":false"));
        assert!(body.contains("\"self_evolution_write_allowed\":false"));
    }

    #[test]
    fn stream_error_final_json_marks_post_inference_failures_without_timeout() {
        let body = stream_error_final_json(
            9,
            "chat-stream",
            4,
            &std::io::Error::new(std::io::ErrorKind::Other, "state save failed"),
        );

        assert!(body.contains("\"request_id\":9"));
        assert!(body.contains("\"endpoint\":\"chat-stream\""));
        assert!(body.contains("\"stream_state\":\"failed\""));
        assert!(body.contains("\"timeout\":false"));
        assert!(body.contains("\"partial_result\":true"));
        assert!(body.contains("\"partial_finalized\":true"));
        assert!(body.contains("\"streamed_tokens\":4"));
        assert!(body.contains("\"persistent_writes\":false"));
    }

    fn assert_failed_stream_compute_budget_fields(body: &str) {
        assert!(body.contains("\"compute_budget\":null"));
        assert!(body.contains("\"compute_budget_saved_tokens\":0"));
        assert!(body.contains("\"compute_budget_avoided_tokens\":0"));
        assert!(body.contains("\"compute_budget_kv_lookups_skipped\":0"));
        assert!(body.contains("\"compute_budget_fanout_reduction\":0"));
        assert!(body.contains("\"compute_budget_read_only\":true"));
        assert!(body.contains("\"compute_budget_write_allowed\":false"));
        assert!(body.contains("\"compute_budget_applied\":false"));
    }

    #[test]
    fn openai_stream_delta_uses_chat_completion_chunk_shape() {
        let body = openai_chat_completion_stream_delta_json(5, "rust-norion-local", 10, "partial");

        assert!(body.contains("\"id\":\"chatcmpl-norion-5\""));
        assert!(body.contains("\"object\":\"chat.completion.chunk\""));
        assert!(body.contains("\"created\":10"));
        assert!(body.contains("\"model\":\"rust-norion-local\""));
        assert!(body.contains("\"delta\":{\"role\":\"assistant\",\"content\":\"partial\"}"));
        assert!(body.contains("\"finish_reason\":null"));
    }

    #[test]
    fn openai_stream_error_blocks_writes() {
        let body = openai_chat_completion_stream_error_json(OpenAiStreamErrorContext {
            request_id: 6,
            model: "rust-norion-local",
            created: 11,
            endpoint: "chat-completions-stream",
            streamed_tokens: 1,
            message: "request cancelled",
            cancelled: true,
            timeout: false,
        });

        assert!(body.contains("\"object\":\"chat.completion.chunk\""));
        assert!(body.contains("\"type\":\"cancelled\""));
        assert!(body.contains("\"stream_state\":\"failed\""));
        assert!(body.contains("\"cancelled\":true"));
        assert!(body.contains(
            "\"compute_budget_summary\":\"unavailable_interrupted_before_final_outcome\""
        ));
        assert_failed_stream_compute_budget_fields(&body);
        assert!(body.contains("\"persistent_writes\":false"));
        assert!(body.contains("\"memory_write_allowed\":false"));
        assert!(body.contains("\"genome_write_allowed\":false"));
        assert!(body.contains("\"self_evolution_write_allowed\":false"));
    }
}
