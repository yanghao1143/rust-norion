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
    model_service_runtime_closed_loop_counters_json, model_service_task_intent_metadata,
    model_service_task_metadata_json, openai_chat_completion_response_json,
    openai_completion_response_json, openai_norion_runtime_metadata_json,
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
            state.record_inference(ModelServiceLastInferenceTelemetry::error_with_state(
                request_id,
                endpoint,
                message.clone(),
                false,
                io_error_is_timeout(&error),
                true,
                None,
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
                None,
            );
        }
    };
    if let Some(note) = runtime_error_note(&timed) {
        let message = runtime_error_message(&timed);
        let timeout = runtime_error_note_is_timeout(note);
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_state(
            request_id,
            endpoint,
            message.clone(),
            false,
            timeout,
            true,
            Some(note),
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
            Some(&timed),
        );
    }
    if state.is_cancel_requested(request_id) {
        let message = cancellation_message(state, request_id);
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_state(
            request_id, endpoint, message, true, false, false, None,
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
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_state(
            request_id,
            endpoint,
            message.clone(),
            false,
            io_error_is_timeout(&error),
            false,
            None,
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
            Some(&timed),
        );
    }
    if let Err(error) = engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    ) {
        let message = error.to_string();
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_state(
            request_id,
            endpoint,
            message.clone(),
            false,
            io_error_is_timeout(&error),
            false,
            None,
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
            Some(&timed),
        );
    }
    state.record_inference(ModelServiceLastInferenceTelemetry::from_timed(
        request_id, endpoint, &timed,
    ));
    let body = match response_format {
        GenerationResponseFormat::ModelService => model_service_success_json(
            &model_service_response_json(
                request_id,
                profile,
                args.trace_path.is_some(),
                request.output_mode,
                task_intent,
                &timed,
            ),
            endpoint,
        ),
        GenerationResponseFormat::OpenAiCompletion { model } => openai_completion_response_json(
            request_id,
            endpoint,
            profile,
            model.as_deref(),
            request.output_mode,
            task_intent,
            &timed,
        ),
        GenerationResponseFormat::OpenAiChatCompletion { model } => {
            openai_chat_completion_response_json(
                request_id,
                endpoint,
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

fn model_service_success_json(response_json: &str, endpoint: &str) -> String {
    let Some(response_prefix) = response_json.strip_suffix('}') else {
        return response_json.to_owned();
    };
    format!(
        "{},\"endpoint\":{},\"error\":null,\"error_type\":null,\"cancelled\":false,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"persistent_writes\":true,\"memory_write_allowed\":true,\"genome_write_allowed\":true,\"self_evolution_write_allowed\":true}}",
        response_prefix,
        service_json_string(endpoint)
    )
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
    timed: Option<&TimedOutcome>,
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
        timed,
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
    timed: Option<&TimedOutcome>,
) -> String {
    let note_json = runtime_error_note
        .map(service_json_string)
        .unwrap_or_else(|| "null".to_owned());
    let route_metadata = memory_route_metadata_json(timed);
    let runtime_closed_loop_counters = runtime_closed_loop_counters_metadata_json(timed);
    match response_format {
        GenerationResponseFormat::ModelService => format!(
            "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"error\":{},\"error_type\":\"{}\",\"cancelled\":false,\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,{},{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
            request_id,
            service_json_string(endpoint),
            service_json_string(message),
            error_type,
            timeout,
            retryable,
            note_json,
            route_metadata,
            runtime_closed_loop_counters
        ),
        GenerationResponseFormat::OpenAiCompletion { model }
        | GenerationResponseFormat::OpenAiChatCompletion { model } => {
            let model = openai_model_name(model.as_deref());
            format!(
                "{{\"ok\":false,\"error\":{{\"message\":{},\"type\":\"{}\",\"param\":null,\"code\":null}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"cancelled\":false,\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,{},{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}}}",
                service_json_string(message),
                error_type,
                request_id,
                service_json_string(endpoint),
                service_json_string(&model),
                timeout,
                retryable,
                note_json,
                route_metadata,
                runtime_closed_loop_counters
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
    let runtime_closed_loop_counters =
        model_service_runtime_closed_loop_counters_json(&timed.outcome);
    let fanout_reduction = compute_budget
        .route_fanout_before
        .saturating_sub(compute_budget.route_fanout_after);
    match response_format {
        GenerationResponseFormat::ModelService => format!(
            "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"error\":\"request cancelled by runtime_request_splice\",\"error_type\":\"cancelled\",\"cancelled\":true,\"timeout\":false,\"retryable\":false,\"compute_budget\":{},\"compute_budget_summary\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_kv_lookups_skipped\":{},\"compute_budget_fanout_reduction\":{},\"compute_budget_read_only\":{},\"compute_budget_write_allowed\":{},\"compute_budget_applied\":{},\"used_memory_count\":{},\"route_threshold\":{:.6},\"route_attention_tokens\":{},\"route_fast_tokens\":{},\"route_attention_fraction\":{:.6},{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
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
            compute_budget.applied,
            timed.outcome.used_memories.len(),
            timed.outcome.route_budget.threshold,
            timed.outcome.route_budget.attention_tokens,
            timed.outcome.route_budget.fast_tokens,
            timed.outcome.route_budget.attention_fraction,
            runtime_closed_loop_counters
        ),
        GenerationResponseFormat::OpenAiCompletion { model }
        | GenerationResponseFormat::OpenAiChatCompletion { model } => {
            let model = openai_model_name(model.as_deref());
            format!(
                "{{\"ok\":false,\"error\":{{\"message\":\"request cancelled by runtime_request_splice\",\"type\":\"cancelled\",\"param\":null,\"code\":null}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"cancelled\":true,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"compute_budget\":{},\"compute_budget_summary\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_kv_lookups_skipped\":{},\"compute_budget_fanout_reduction\":{},\"compute_budget_read_only\":{},\"compute_budget_write_allowed\":{},\"compute_budget_applied\":{},\"used_memory_count\":{},\"route_threshold\":{:.6},\"route_attention_tokens\":{},\"route_fast_tokens\":{},\"route_attention_fraction\":{:.6},{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}}}",
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
                compute_budget.applied,
                timed.outcome.used_memories.len(),
                timed.outcome.route_budget.threshold,
                timed.outcome.route_budget.attention_tokens,
                timed.outcome.route_budget.fast_tokens,
                timed.outcome.route_budget.attention_fraction,
                runtime_closed_loop_counters
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
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_state(
            request_id,
            endpoint,
            message.clone(),
            true,
            false,
            false,
            None,
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
                    runtime_error_note: None,
                    cancelled: true,
                    timeout: false,
                    retryable: false,
                    timed: None,
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
            let message = error.to_string();
            state.record_inference(ModelServiceLastInferenceTelemetry::error_with_state(
                request_id,
                endpoint,
                message.clone(),
                false,
                stream_error_is_timeout(&error),
                true,
                Some(&message),
            ));
            if stream_write_failed {
                return Err(error);
            }
            match &response_format {
                StreamResponseFormat::ModelService => {
                    write_sse_event(stream, "error", &message)?;
                    let final_body = stream_error_final_json(
                        request_id,
                        endpoint,
                        token_count,
                        &error,
                        true,
                        None,
                    );
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
                        runtime_error_note: Some(&message),
                        cancelled: false,
                        timeout: stream_error_is_timeout(&error),
                        retryable: true,
                        timed: None,
                    });
                    write_openai_sse_data(stream, &body)?;
                    write_openai_sse_data(stream, "[DONE]")?;
                }
            }
            return Ok(());
        }
    };

    if let Some(note) = runtime_error_note(&timed) {
        let message = runtime_error_message(&timed);
        let timeout = runtime_error_note_is_timeout(note);
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_state(
            request_id,
            endpoint,
            message.clone(),
            false,
            timeout,
            true,
            Some(note),
        ));
        match &response_format {
            StreamResponseFormat::ModelService => {
                write_sse_event(stream, "error", &message)?;
                let final_body = stream_runtime_error_final_json(
                    request_id,
                    endpoint,
                    token_count,
                    &message,
                    note,
                    timeout,
                    true,
                    Some(&timed),
                );
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
                    runtime_error_note: Some(note),
                    cancelled: false,
                    timeout,
                    retryable: true,
                    timed: Some(&timed),
                });
                write_openai_sse_data(stream, &body)?;
                write_openai_sse_data(stream, "[DONE]")?;
            }
        }
        return Ok(());
    }

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
        let message = error.to_string();
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_state(
            request_id,
            endpoint,
            message.clone(),
            false,
            stream_error_is_timeout(&error),
            false,
            Some(&message),
        ));
        match &response_format {
            StreamResponseFormat::ModelService => {
                write_sse_event(stream, "error", &message)?;
                let final_body = stream_error_final_json(
                    request_id,
                    endpoint,
                    token_count,
                    &error,
                    false,
                    Some(&timed),
                );
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
                    runtime_error_note: Some(&message),
                    cancelled: false,
                    timeout: stream_error_is_timeout(&error),
                    retryable: false,
                    timed: Some(&timed),
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
    let runtime_closed_loop_counters = neutral_runtime_closed_loop_counters_json();
    format!(
        "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"stream_state\":\"interrupted\",\"cancelled\":true,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"partial_result\":{},\"partial_finalized\":true,\"streamed_tokens\":{},\"queue_time_ms\":0,\"cancellation_reason\":{},\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_interrupted_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,{},{},\"error\":{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
        request_id,
        service_json_string(endpoint),
        streamed_tokens > 0,
        streamed_tokens,
        service_json_string(message),
        neutral_memory_route_metadata_json(),
        runtime_closed_loop_counters,
        service_json_string(message)
    )
}

fn stream_error_final_json(
    request_id: usize,
    endpoint: &str,
    streamed_tokens: usize,
    error: &std::io::Error,
    retryable: bool,
    timed: Option<&TimedOutcome>,
) -> String {
    let message = error.to_string();
    let timeout = matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) || message.to_ascii_lowercase().contains("timeout");
    let route_metadata = memory_route_metadata_json(timed);
    let runtime_closed_loop_counters = runtime_closed_loop_counters_metadata_json(timed);
    format!(
        "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"stream_state\":\"failed\",\"cancelled\":false,\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"partial_result\":{},\"partial_finalized\":true,\"streamed_tokens\":{},\"queue_time_ms\":0,\"cancellation_reason\":null,\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,{},{},\"error\":{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
        request_id,
        service_json_string(endpoint),
        timeout,
        retryable,
        service_json_string(&message),
        streamed_tokens > 0,
        streamed_tokens,
        route_metadata,
        runtime_closed_loop_counters,
        service_json_string(&message)
    )
}

fn stream_runtime_error_final_json(
    request_id: usize,
    endpoint: &str,
    streamed_tokens: usize,
    message: &str,
    runtime_error_note: &str,
    timeout: bool,
    retryable: bool,
    timed: Option<&TimedOutcome>,
) -> String {
    let route_metadata = memory_route_metadata_json(timed);
    let runtime_closed_loop_counters = runtime_closed_loop_counters_metadata_json(timed);
    format!(
        "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"stream_state\":\"failed\",\"cancelled\":false,\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"partial_result\":{},\"partial_finalized\":true,\"streamed_tokens\":{},\"queue_time_ms\":0,\"cancellation_reason\":null,\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,{},{},\"error\":{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
        request_id,
        service_json_string(endpoint),
        timeout,
        retryable,
        service_json_string(runtime_error_note),
        streamed_tokens > 0,
        streamed_tokens,
        route_metadata,
        runtime_closed_loop_counters,
        service_json_string(message)
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
        "{},\"endpoint\":{},\"stream_state\":\"completed\",\"cancelled\":false,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"partial_result\":false,\"partial_finalized\":false,\"streamed_tokens\":{},\"queue_time_ms\":0,\"cancellation_reason\":null,\"compute_budget_summary\":{},\"persistent_writes\":true,\"memory_write_allowed\":true,\"genome_write_allowed\":true,\"self_evolution_write_allowed\":true}}",
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
        "{{\"id\":\"chatcmpl-norion-{}\",\"object\":\"chat.completion.chunk\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}],\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"profile\":\"{}\",{},\"stream_state\":\"completed\",\"cancelled\":false,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"streamed_tokens\":{}, {},\"elapsed_ms\":{},\"persistent_writes\":true,\"memory_write_allowed\":true,\"genome_write_allowed\":true,\"self_evolution_write_allowed\":true}}}}",
        request_id,
        created,
        service_json_string(model),
        request_id,
        service_json_string(endpoint),
        service_json_string(model),
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
    runtime_error_note: Option<&'a str>,
    cancelled: bool,
    timeout: bool,
    retryable: bool,
    timed: Option<&'a TimedOutcome>,
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
    let route_metadata = memory_route_metadata_json(context.timed);
    let runtime_closed_loop_counters = runtime_closed_loop_counters_metadata_json(context.timed);
    format!(
        "{{\"id\":\"chatcmpl-norion-{}\",\"object\":\"chat.completion.chunk\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}],\"error\":{{\"message\":{},\"type\":\"{}\"}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"stream_state\":\"failed\",\"cancelled\":{},\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"streamed_tokens\":{},\"stored_runtime_kv_memory_ids\":[],{},{},\"compute_budget\":null,\"compute_budget_summary\":{},\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}}}",
        context.request_id,
        context.created,
        service_json_string(context.model),
        service_json_string(context.message),
        error_type,
        context.request_id,
        service_json_string(context.endpoint),
        service_json_string(context.model),
        context.cancelled,
        context.timeout,
        context.retryable,
        context
            .runtime_error_note
            .map(service_json_string)
            .unwrap_or_else(|| "null".to_owned()),
        context.streamed_tokens,
        route_metadata,
        runtime_closed_loop_counters,
        service_json_string(compute_budget_summary)
    )
}

fn neutral_memory_route_metadata_json() -> &'static str {
    "\"used_memory_count\":0,\"route_threshold\":0.000000,\"route_attention_tokens\":0,\"route_fast_tokens\":0,\"route_attention_fraction\":0.000000"
}

fn neutral_runtime_closed_loop_counters_json() -> &'static str {
    "\"runtime_closed_loop_counters\":{\"compute_budget_selected_candidates\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,\"memory_admission_candidates\":0,\"memory_admission_ready\":0,\"memory_admission_blocked\":0,\"memory_admission_ledger_records\":0,\"memory_admission_ledger_preview_only\":0,\"memory_admission_ledger_authorized\":0,\"memory_admission_ledger_applied\":0,\"memory_admission_write_allowed\":false,\"memory_admission_applied\":false,\"kv_fusion_candidates\":0,\"kv_fusion_fused\":0,\"kv_fusion_compressed\":0,\"kv_fusion_skipped\":0,\"kv_fusion_held\":0,\"kv_fusion_rejected\":0,\"kv_fusion_approval_blocked\":0,\"kv_fusion_input_tokens\":0,\"kv_fusion_retained_tokens\":0,\"kv_fusion_saved_tokens\":0,\"kv_fusion_write_allowed\":false,\"kv_fusion_applied\":false}"
}

fn runtime_closed_loop_counters_metadata_json(timed: Option<&TimedOutcome>) -> String {
    timed.map_or_else(
        || neutral_runtime_closed_loop_counters_json().to_owned(),
        |timed| model_service_runtime_closed_loop_counters_json(&timed.outcome),
    )
}

fn memory_route_metadata_json(timed: Option<&TimedOutcome>) -> String {
    timed.map_or_else(
        || neutral_memory_route_metadata_json().to_owned(),
        |timed| {
            format!(
                "\"used_memory_count\":{},\"route_threshold\":{:.6},\"route_attention_tokens\":{},\"route_fast_tokens\":{},\"route_attention_fraction\":{:.6}",
                timed.outcome.used_memories.len(),
                timed.outcome.route_budget.threshold,
                timed.outcome.route_budget.attention_tokens,
                timed.outcome.route_budget.fast_tokens,
                timed.outcome.route_budget.attention_fraction
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::{HeuristicBackend, InferenceRequest};

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
        assert!(body.contains("\"retryable\":false"));
        assert!(body.contains("\"runtime_error_note\":null"));
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
        assert_failed_memory_route_budget_fields(&body);
        assert_failed_runtime_closed_loop_counter_fields(&body);
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
            true,
            None,
        );

        assert!(body.contains("\"request_id\":8"));
        assert!(body.contains("\"stream_state\":\"failed\""));
        assert!(body.contains("\"cancelled\":false"));
        assert!(body.contains("\"timeout\":true"));
        assert!(body.contains("\"retryable\":true"));
        assert!(body.contains("\"runtime_error_note\":\"runtime timeout\""));
        assert!(body.contains("\"partial_result\":true"));
        assert!(body.contains("\"partial_finalized\":true"));
        assert!(body.contains("\"streamed_tokens\":2"));
        assert!(body.contains("\"queue_time_ms\":0"));
        assert!(body.contains("\"cancellation_reason\":null"));
        assert!(
            body.contains("\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\"")
        );
        assert_failed_stream_compute_budget_fields(&body);
        assert_failed_memory_route_budget_fields(&body);
        assert_failed_runtime_closed_loop_counter_fields(&body);
        assert!(body.contains("\"persistent_writes\":false"));
        assert!(body.contains("\"memory_write_allowed\":false"));
        assert!(body.contains("\"genome_write_allowed\":false"));
        assert!(body.contains("\"self_evolution_write_allowed\":false"));
    }

    #[test]
    fn stream_error_final_json_marks_post_inference_failures_without_timeout() {
        let timed = timed_route_fixture();
        let body = stream_error_final_json(
            9,
            "chat-stream",
            4,
            &std::io::Error::new(std::io::ErrorKind::Other, "state save failed"),
            false,
            Some(&timed),
        );

        assert!(body.contains("\"request_id\":9"));
        assert!(body.contains("\"endpoint\":\"chat-stream\""));
        assert!(body.contains("\"stream_state\":\"failed\""));
        assert!(body.contains("\"timeout\":false"));
        assert!(body.contains("\"retryable\":false"));
        assert!(body.contains("\"runtime_error_note\":\"state save failed\""));
        assert!(body.contains("\"partial_result\":true"));
        assert!(body.contains("\"partial_finalized\":true"));
        assert!(body.contains("\"streamed_tokens\":4"));
        assert_actual_memory_route_budget_fields(&body, &timed);
        assert_runtime_closed_loop_counter_fields(&body);
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

    fn assert_failed_memory_route_budget_fields(body: &str) {
        assert!(body.contains("\"used_memory_count\":0"));
        assert!(body.contains("\"route_threshold\":0.000000"));
        assert!(body.contains("\"route_attention_tokens\":0"));
        assert!(body.contains("\"route_fast_tokens\":0"));
        assert!(body.contains("\"route_attention_fraction\":0.000000"));
    }

    fn assert_failed_runtime_closed_loop_counter_fields(body: &str) {
        assert_runtime_closed_loop_counter_fields(body);
        assert!(body.contains("\"compute_budget_saved_tokens\":0"));
        assert!(body.contains("\"compute_budget_avoided_tokens\":0"));
        assert!(body.contains("\"memory_admission_ledger_authorized\":0"));
        assert!(body.contains("\"memory_admission_ledger_applied\":0"));
        assert!(body.contains("\"kv_fusion_saved_tokens\":0"));
        assert!(body.contains("\"kv_fusion_applied\":false"));
    }

    fn assert_runtime_closed_loop_counter_fields(body: &str) {
        assert!(body.contains("\"runtime_closed_loop_counters\":{"));
        assert!(body.contains("\"compute_budget_saved_tokens\":"));
        assert!(body.contains("\"compute_budget_avoided_tokens\":"));
        assert!(body.contains("\"memory_admission_ledger_authorized\":"));
        assert!(body.contains("\"memory_admission_ledger_applied\":"));
        assert!(body.contains("\"kv_fusion_saved_tokens\":"));
    }

    fn assert_actual_memory_route_budget_fields(body: &str, timed: &TimedOutcome) {
        assert!(body.contains(&format!(
            "\"used_memory_count\":{}",
            timed.outcome.used_memories.len()
        )));
        assert!(body.contains(&format!(
            "\"route_threshold\":{:.6}",
            timed.outcome.route_budget.threshold
        )));
        assert!(body.contains(&format!(
            "\"route_attention_tokens\":{}",
            timed.outcome.route_budget.attention_tokens
        )));
        assert!(body.contains(&format!(
            "\"route_fast_tokens\":{}",
            timed.outcome.route_budget.fast_tokens
        )));
        assert!(body.contains(&format!(
            "\"route_attention_fraction\":{:.6}",
            timed.outcome.route_budget.attention_fraction
        )));
    }

    fn timed_route_fixture() -> TimedOutcome {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("actual route metadata fixture", TaskProfile::Coding),
            &mut backend,
        );
        TimedOutcome {
            outcome,
            elapsed_ms: 7,
        }
    }

    #[test]
    fn model_service_success_json_reports_state_fields() {
        let body = model_service_success_json("{\"ok\":true,\"request_id\":3}", "generate");

        assert!(body.contains("\"endpoint\":\"generate\""));
        assert!(body.contains("\"error\":null"));
        assert!(body.contains("\"error_type\":null"));
        assert!(body.contains("\"cancelled\":false"));
        assert!(body.contains("\"timeout\":false"));
        assert!(body.contains("\"retryable\":false"));
        assert!(body.contains("\"runtime_error_note\":null"));
        assert!(body.contains("\"persistent_writes\":true"));
        assert!(body.contains("\"memory_write_allowed\":true"));
        assert!(body.contains("\"genome_write_allowed\":true"));
        assert!(body.contains("\"self_evolution_write_allowed\":true"));
    }

    #[test]
    fn openai_generation_error_json_reports_cancelled_false() {
        let body = generation_error_json(
            &GenerationResponseFormat::OpenAiCompletion {
                model: Some("rust-norion-local".to_owned()),
            },
            17,
            "completions",
            "runtime failed",
            "runtime_error",
            false,
            true,
            None,
            None,
        );

        assert!(body.contains("\"norion\":{"));
        assert!(body.contains("\"cancelled\":false"));
        assert!(body.contains("\"timeout\":false"));
        assert!(body.contains("\"retryable\":true"));
        assert_failed_memory_route_budget_fields(&body);
        assert_failed_runtime_closed_loop_counter_fields(&body);
    }

    #[test]
    fn model_service_generation_error_json_reports_neutral_route_budget() {
        let body = generation_error_json(
            &GenerationResponseFormat::ModelService,
            18,
            "generate",
            "runtime failed",
            "runtime_error",
            false,
            true,
            None,
            None,
        );

        assert!(body.contains("\"cancelled\":false"));
        assert_failed_memory_route_budget_fields(&body);
        assert_failed_runtime_closed_loop_counter_fields(&body);
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
            runtime_error_note: None,
            cancelled: true,
            timeout: false,
            retryable: false,
            timed: None,
        });

        assert!(body.contains("\"object\":\"chat.completion.chunk\""));
        assert!(body.contains("\"type\":\"cancelled\""));
        assert!(body.contains("\"model\":\"rust-norion-local\""));
        assert!(body.contains("\"stream_state\":\"failed\""));
        assert!(body.contains("\"cancelled\":true"));
        assert!(body.contains("\"retryable\":false"));
        assert!(body.contains("\"runtime_error_note\":null"));
        assert!(body.contains("\"stored_runtime_kv_memory_ids\":[]"));
        assert!(body.contains(
            "\"compute_budget_summary\":\"unavailable_interrupted_before_final_outcome\""
        ));
        assert_failed_stream_compute_budget_fields(&body);
        assert_failed_memory_route_budget_fields(&body);
        assert_failed_runtime_closed_loop_counter_fields(&body);
        assert!(body.contains("\"persistent_writes\":false"));
        assert!(body.contains("\"memory_write_allowed\":false"));
        assert!(body.contains("\"genome_write_allowed\":false"));
        assert!(body.contains("\"self_evolution_write_allowed\":false"));
    }

    #[test]
    fn openai_stream_runtime_error_preserves_note() {
        let timed = timed_route_fixture();
        let body = openai_chat_completion_stream_error_json(OpenAiStreamErrorContext {
            request_id: 7,
            model: "rust-norion-local",
            created: 12,
            endpoint: "chat-completions-stream",
            streamed_tokens: 0,
            message: "Runtime backend error: runtime command timed out",
            runtime_error_note: Some("runtime_error:label=runtime_error:timeout=true"),
            cancelled: false,
            timeout: true,
            retryable: true,
            timed: Some(&timed),
        });

        assert!(body.contains("\"type\":\"timeout\""));
        assert!(body.contains("\"timeout\":true"));
        assert!(body.contains("\"retryable\":true"));
        assert!(
            body.contains(
                "\"runtime_error_note\":\"runtime_error:label=runtime_error:timeout=true\""
            )
        );
        assert!(body.contains("\"stored_runtime_kv_memory_ids\":[]"));
        assert_failed_stream_compute_budget_fields(&body);
        assert_actual_memory_route_budget_fields(&body, &timed);
        assert_runtime_closed_loop_counter_fields(&body);
        assert!(body.contains("\"persistent_writes\":false"));
    }
}
