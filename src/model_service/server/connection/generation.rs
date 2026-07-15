use std::io::Write;
use std::net::TcpStream;

use rust_norion::{DraftToken, InferenceBackend, NoironEngine, TaskProfile, TenantScope};

use super::super::super::json::{
    option_str_service_json, option_usize_service_json, service_json_string, write_http_json,
    write_http_sse_headers, write_sse_event,
};
use super::super::super::newapi_fallback::newapi_behavior_task_kind;
use super::super::super::profile::detect_profile;
use super::super::super::request::{
    ModelServiceChatRequest, ModelServiceOpenAiCompletionRequest, ModelServiceOutputMode,
    ModelServiceRequest,
};
use super::super::super::response::{
    ModelServiceTaskIntentMetadata, model_service_model_fallback_json, model_service_response_json,
    model_service_runtime_closed_loop_counters_json, model_service_task_intent_metadata,
    model_service_task_metadata_json, openai_chat_completion_response_json,
    openai_completion_response_json, openai_norion_runtime_metadata_json,
};
use super::super::state::{
    ModelServiceBehaviorFeedbackReceipt, ModelServiceEvolutionCandidateReceipt,
    ModelServiceLastInferenceTelemetry, ModelServiceServerState,
};
use crate::Args;
use crate::gemma_business::contract::annotate_model_service_business_case_for_timed;
use crate::inference_runner::{
    run_timed_inference_stream_checked_with_scope_options_cancelable,
    run_timed_inference_with_scope_options_cancelable,
};
use crate::model_service::types::TimedOutcome;

#[cfg(test)]
thread_local! {
    static FORCE_POSTPROCESS_ERROR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(super) struct GenerationHandlerContext<'a> {
    pub(super) state: &'a ModelServiceServerState,
    pub(super) args: &'a Args,
    pub(super) stream: &'a mut TcpStream,
    pub(super) request_id: usize,
    pub(super) endpoint: &'static str,
}

fn annotate_generation_business_case(
    engine: &mut NoironEngine,
    timed: &mut TimedOutcome,
    case_name: Option<&str>,
    trace_path: Option<&std::path::PathBuf>,
) -> std::io::Result<()> {
    #[cfg(test)]
    if case_name.is_some() && FORCE_POSTPROCESS_ERROR.with(std::cell::Cell::get) {
        return Err(std::io::Error::other(
            "injected generation postprocess failure",
        ));
    }
    annotate_model_service_business_case_for_timed(engine, timed, case_name, trace_path)
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
    let transaction = engine.begin_generation_state_transaction();
    let profile = request
        .profile
        .unwrap_or_else(|| detect_profile(&request.prompt));
    let task_intent = model_service_task_intent_metadata(&request.prompt, profile);
    let behavior_task_kind = newapi_behavior_task_kind(&request.prompt);
    let case_name = request.case_name.clone();
    let evolution_prompt = request.evolution_preview.then(|| request.prompt.clone());
    let evolution_scope = request.tenant_scope.clone();
    let tenant_scope = request.tenant_scope;
    let max_tokens = request.max_tokens;
    let mut should_cancel = || state.is_cancel_requested(request_id);
    let mut timed = match run_timed_inference_with_scope_options_cancelable(
        engine,
        backend,
        request.prompt,
        profile,
        max_tokens,
        tenant_scope,
        args.trace_path.as_ref(),
        case_name.as_deref(),
        &mut should_cancel,
    ) {
        Ok(timed) => timed,
        Err(error) => {
            engine.rollback_generation_state_transaction(transaction);
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
    if state.is_cancel_requested(request_id) {
        engine.rollback_generation_state_transaction(transaction);
        let message = cancellation_message(state, request_id);
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_timed_state(
            request_id, endpoint, message, &timed, true, false, false, None,
        ));
        let body = generation_cancelled_after_inference_json(
            &response_format,
            request_id,
            endpoint,
            &timed,
        );
        return write_http_json(stream, 409, "Conflict", &body);
    }
    if let Some(note) = runtime_error_note(&timed) {
        engine.rollback_generation_state_transaction(transaction);
        let message = runtime_error_message(&timed);
        let timeout = runtime_error_note_is_timeout(note);
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_timed_state(
            request_id,
            endpoint,
            message.clone(),
            &timed,
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
    if let Err(error) = annotate_generation_business_case(
        engine,
        &mut timed,
        case_name.as_deref(),
        args.trace_path.as_ref(),
    ) {
        engine.rollback_generation_state_transaction(transaction);
        let message = error.to_string();
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_timed_state(
            request_id,
            endpoint,
            message.clone(),
            &timed,
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
    if let Err(save_error) = engine.save_full_state_in_generation_transaction(
        &transaction,
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    ) {
        let committed = save_error.committed();
        let error = save_error.into_inner();
        if committed {
            engine.commit_generation_state_transaction(transaction);
        } else {
            engine.rollback_generation_state_transaction(transaction);
        }
        let message = error.to_string();
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_timed_state(
            request_id,
            endpoint,
            message.clone(),
            &timed,
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
    engine.commit_generation_state_transaction(transaction);
    state.record_inference(ModelServiceLastInferenceTelemetry::from_timed(
        request_id, endpoint, &timed,
    ));
    let (evolution_candidate, behavior_feedback) = register_generation_capabilities(
        state,
        request_id,
        evolution_prompt.as_deref(),
        evolution_scope.as_ref(),
        behavior_task_kind,
        max_tokens,
        &timed,
    );
    let body = match &response_format {
        GenerationResponseFormat::ModelService => model_service_success_json(
            &model_service_response_json(
                request_id,
                profile,
                args.trace_path.is_some(),
                request.output_mode,
                max_tokens,
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
            max_tokens,
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
                max_tokens,
                task_intent,
                &timed,
            )
        }
    };
    let body =
        append_evolution_candidate_json(&body, &response_format, evolution_candidate.as_ref());
    let body = append_behavior_feedback_json(&body, &response_format, behavior_feedback.as_ref());
    write_http_json(stream, 200, "OK", &body)
}

fn register_generation_capabilities(
    state: &ModelServiceServerState,
    request_id: usize,
    evolution_prompt: Option<&str>,
    scope: Option<&TenantScope>,
    behavior_task_kind: &str,
    max_tokens: Option<usize>,
    timed: &TimedOutcome,
) -> (
    Option<ModelServiceEvolutionCandidateReceipt>,
    Option<ModelServiceBehaviorFeedbackReceipt>,
) {
    let evolution_candidate = evolution_prompt.and_then(|prompt| {
        scope.map(|scope| {
            state.register_evolution_candidate(request_id, prompt, scope, max_tokens, timed)
        })
    });
    let behavior_feedback = timed
        .outcome
        .report
        .issues
        .iter()
        .any(|issue| issue.code == "generated_code_behavior_unverified")
        .then(|| {
            scope.map(|scope| {
                state.register_behavior_feedback(
                    request_id,
                    timed.outcome.experience_id,
                    scope,
                    timed.outcome.runtime_diagnostics.model_id.as_deref(),
                    behavior_task_kind,
                )
            })
        })
        .flatten();
    (evolution_candidate, behavior_feedback)
}

fn append_behavior_feedback_json(
    response_json: &str,
    response_format: &GenerationResponseFormat,
    receipt: Option<&ModelServiceBehaviorFeedbackReceipt>,
) -> String {
    let Some(receipt) = receipt else {
        return response_json.to_owned();
    };
    let runtime_model = receipt
        .runtime_model
        .as_deref()
        .map(service_json_string)
        .unwrap_or_else(|| "null".to_owned());
    let field = format!(
        "\"behavior_feedback\":{{\"eligible\":true,\"token\":{},\"experience_id\":{},\"expires_in_seconds\":{},\"runtime_model\":{},\"task_kind\":{}}}",
        service_json_string(&receipt.token),
        receipt.experience_id,
        receipt.expires_in_seconds,
        runtime_model,
        service_json_string(&receipt.task_kind),
    );
    match response_format {
        GenerationResponseFormat::ModelService => response_json
            .strip_suffix('}')
            .map(|prefix| format!("{prefix},{field}}}"))
            .unwrap_or_else(|| response_json.to_owned()),
        GenerationResponseFormat::OpenAiCompletion { .. }
        | GenerationResponseFormat::OpenAiChatCompletion { .. } => response_json
            .strip_suffix("}}")
            .map(|prefix| format!("{prefix},{field}}}}}"))
            .unwrap_or_else(|| response_json.to_owned()),
    }
}

fn append_evolution_candidate_json(
    response_json: &str,
    response_format: &GenerationResponseFormat,
    receipt: Option<&ModelServiceEvolutionCandidateReceipt>,
) -> String {
    let Some(receipt) = receipt else {
        return response_json.to_owned();
    };
    let token = receipt
        .token
        .as_deref()
        .map(service_json_string)
        .unwrap_or_else(|| "null".to_owned());
    let field = format!(
        "\"evolution_candidate\":{{\"eligible\":{},\"token\":{},\"prompt_digest\":{},\"candidate_digest\":{},\"generation_before\":{},\"residency_revision_before\":{},\"candidate_count\":{},\"expires_in_seconds\":{},\"reason\":{}}}",
        receipt.eligible,
        token,
        service_json_string(&receipt.prompt_digest),
        service_json_string(&receipt.candidate_digest),
        receipt.generation_before,
        receipt.residency_revision_before,
        receipt.candidate_count,
        receipt.expires_in_seconds,
        service_json_string(&receipt.reason),
    );
    match response_format {
        GenerationResponseFormat::ModelService => response_json
            .strip_suffix('}')
            .map(|prefix| format!("{prefix},{field}}}"))
            .unwrap_or_else(|| response_json.to_owned()),
        GenerationResponseFormat::OpenAiCompletion { .. }
        | GenerationResponseFormat::OpenAiChatCompletion { .. } => response_json
            .strip_suffix("}}")
            .map(|prefix| format!("{prefix},{field}}}}}"))
            .unwrap_or_else(|| response_json.to_owned()),
    }
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
    let model_fallback = model_fallback_metadata_json(timed);
    let runtime_closed_loop_counters = runtime_closed_loop_counters_metadata_json(timed);
    match response_format {
        GenerationResponseFormat::ModelService => format!(
            "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"error\":{},\"error_type\":\"{}\",\"cancelled\":false,\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,{},{},{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
            request_id,
            service_json_string(endpoint),
            service_json_string(message),
            error_type,
            timeout,
            retryable,
            note_json,
            route_metadata,
            model_fallback,
            runtime_closed_loop_counters
        ),
        GenerationResponseFormat::OpenAiCompletion { model }
        | GenerationResponseFormat::OpenAiChatCompletion { model } => {
            let model = openai_model_name(model.as_deref());
            format!(
                "{{\"ok\":false,\"error\":{{\"message\":{},\"type\":\"{}\",\"param\":null,\"code\":null}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"cancelled\":false,\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,{},{},{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}}}",
                service_json_string(message),
                error_type,
                request_id,
                service_json_string(endpoint),
                service_json_string(&model),
                timeout,
                retryable,
                note_json,
                route_metadata,
                model_fallback,
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
    let model_fallback = model_fallback_metadata_json(Some(timed));
    let runtime_closed_loop_counters =
        model_service_runtime_closed_loop_counters_json(&timed.outcome);
    let fanout_reduction = compute_budget
        .route_fanout_before
        .saturating_sub(compute_budget.route_fanout_after);
    match response_format {
        GenerationResponseFormat::ModelService => format!(
            "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"error\":\"request cancelled by runtime_request_splice\",\"error_type\":\"cancelled\",\"cancelled\":true,\"timeout\":false,\"retryable\":false,\"compute_budget\":{},\"compute_budget_summary\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_kv_lookups_skipped\":{},\"compute_budget_fanout_reduction\":{},\"compute_budget_read_only\":{},\"compute_budget_write_allowed\":{},\"compute_budget_applied\":{},\"used_memory_count\":{},\"route_threshold\":{:.6},\"route_attention_tokens\":{},\"route_fast_tokens\":{},\"route_attention_fraction\":{:.6},{},{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
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
            model_fallback,
            runtime_closed_loop_counters
        ),
        GenerationResponseFormat::OpenAiCompletion { model }
        | GenerationResponseFormat::OpenAiChatCompletion { model } => {
            let model = openai_model_name(model.as_deref());
            format!(
                "{{\"ok\":false,\"error\":{{\"message\":\"request cancelled by runtime_request_splice\",\"type\":\"cancelled\",\"param\":null,\"code\":null}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"cancelled\":true,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"compute_budget\":{},\"compute_budget_summary\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_kv_lookups_skipped\":{},\"compute_budget_fanout_reduction\":{},\"compute_budget_read_only\":{},\"compute_budget_write_allowed\":{},\"compute_budget_applied\":{},\"used_memory_count\":{},\"route_threshold\":{:.6},\"route_attention_tokens\":{},\"route_fast_tokens\":{},\"route_attention_fraction\":{:.6},{},{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}}}",
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
                model_fallback,
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

pub(super) fn runtime_error_note(timed: &TimedOutcome) -> Option<&str> {
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
    let behavior_task_kind = newapi_behavior_task_kind(&request.prompt);
    let case_name = request.case_name.clone();
    let output_mode = request.output_mode;
    let evolution_prompt = request.evolution_preview.then(|| request.prompt.clone());
    let evolution_scope = request.tenant_scope.clone();
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

    let transaction = engine.begin_generation_state_transaction();

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
        let mut should_cancel = || state.is_cancel_requested(request_id);
        run_timed_inference_stream_checked_with_scope_options_cancelable(
            engine,
            backend,
            request.prompt,
            profile,
            max_tokens,
            tenant_scope,
            args.trace_path.as_ref(),
            case_name.as_deref(),
            &mut on_token,
            &mut should_cancel,
        )
    };
    if cancel_requested || state.is_cancel_requested(request_id) {
        engine.rollback_generation_state_transaction(transaction);
        let message = cancellation_message(state, request_id);
        let timed = timed_result.as_ref().ok();
        state.record_inference(match timed {
            Some(timed) => ModelServiceLastInferenceTelemetry::error_with_timed_state(
                request_id,
                endpoint,
                message.clone(),
                timed,
                true,
                false,
                false,
                None,
            ),
            None => ModelServiceLastInferenceTelemetry::error_with_state(
                request_id,
                endpoint,
                message.clone(),
                true,
                false,
                false,
                None,
            ),
        });
        match &response_format {
            StreamResponseFormat::ModelService => {
                write_sse_event(stream, "error", &message)?;
                let final_body =
                    stream_cancel_final_json(request_id, endpoint, token_count, &message, timed);
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
                    timed,
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
            engine.rollback_generation_state_transaction(transaction);
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
        engine.rollback_generation_state_transaction(transaction);
        let message = runtime_error_message(&timed);
        let timeout = runtime_error_note_is_timeout(note);
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_timed_state(
            request_id,
            endpoint,
            message.clone(),
            &timed,
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

    let finalize_result = match annotate_generation_business_case(
        engine,
        &mut timed,
        case_name.as_deref(),
        args.trace_path.as_ref(),
    ) {
        Ok(()) => engine
            .save_full_state_in_generation_transaction(
                &transaction,
                &args.memory_path,
                &args.experience_path,
                &args.adaptive_path,
            )
            .map_err(|save_error| {
                let committed = save_error.committed();
                (save_error.into_inner(), committed)
            }),
        Err(error) => Err((error, false)),
    };
    if let Err((error, committed)) = finalize_result {
        if committed {
            engine.commit_generation_state_transaction(transaction);
        } else {
            engine.rollback_generation_state_transaction(transaction);
        }
        let message = error.to_string();
        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_timed_state(
            request_id,
            endpoint,
            message.clone(),
            &timed,
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

    engine.commit_generation_state_transaction(transaction);

    state.record_inference(ModelServiceLastInferenceTelemetry::from_timed(
        request_id, endpoint, &timed,
    ));
    let (evolution_candidate, behavior_feedback) = register_generation_capabilities(
        state,
        request_id,
        evolution_prompt.as_deref(),
        evolution_scope.as_ref(),
        behavior_task_kind,
        max_tokens,
        &timed,
    );
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
                max_tokens,
                task_intent,
                &timed,
            );
            let body = stream_success_final_json(&body, endpoint, token_count, &timed);
            let body = append_evolution_candidate_json(
                &body,
                &GenerationResponseFormat::ModelService,
                evolution_candidate.as_ref(),
            );
            let body = append_behavior_feedback_json(
                &body,
                &GenerationResponseFormat::ModelService,
                behavior_feedback.as_ref(),
            );
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
                output_mode,
                max_tokens,
                task_intent,
                &timed,
            );
            let capability_format = GenerationResponseFormat::OpenAiChatCompletion {
                model: Some(model.clone()),
            };
            let body = append_evolution_candidate_json(
                &body,
                &capability_format,
                evolution_candidate.as_ref(),
            );
            let body = append_behavior_feedback_json(
                &body,
                &capability_format,
                behavior_feedback.as_ref(),
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
    timed: Option<&TimedOutcome>,
) -> String {
    let route_metadata = memory_route_metadata_json(timed);
    let model_fallback = model_fallback_metadata_json(timed);
    let runtime_closed_loop_counters = runtime_closed_loop_counters_metadata_json(timed);
    format!(
        "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"stream_state\":\"interrupted\",\"cancelled\":true,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"partial_result\":{},\"partial_finalized\":true,\"streamed_tokens\":{},\"queue_time_ms\":0,\"cancellation_reason\":{},\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_interrupted_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,{},{},{},\"error\":{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
        request_id,
        service_json_string(endpoint),
        streamed_tokens > 0,
        streamed_tokens,
        service_json_string(message),
        route_metadata,
        model_fallback,
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
    let model_fallback = model_fallback_metadata_json(timed);
    let runtime_closed_loop_counters = runtime_closed_loop_counters_metadata_json(timed);
    format!(
        "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"stream_state\":\"failed\",\"cancelled\":false,\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"partial_result\":{},\"partial_finalized\":true,\"streamed_tokens\":{},\"queue_time_ms\":0,\"cancellation_reason\":null,\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,{},{},{},\"error\":{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
        request_id,
        service_json_string(endpoint),
        timeout,
        retryable,
        service_json_string(&message),
        streamed_tokens > 0,
        streamed_tokens,
        route_metadata,
        model_fallback,
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
    let model_fallback = model_fallback_metadata_json(timed);
    let runtime_closed_loop_counters = runtime_closed_loop_counters_metadata_json(timed);
    format!(
        "{{\"ok\":false,\"request_id\":{},\"endpoint\":{},\"stream_state\":\"failed\",\"cancelled\":false,\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"partial_result\":{},\"partial_finalized\":true,\"streamed_tokens\":{},\"queue_time_ms\":0,\"cancellation_reason\":null,\"compute_budget\":null,\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\",\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,{},{},{},\"error\":{},\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}",
        request_id,
        service_json_string(endpoint),
        timeout,
        retryable,
        service_json_string(runtime_error_note),
        streamed_tokens > 0,
        streamed_tokens,
        route_metadata,
        model_fallback,
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
    output_mode: ModelServiceOutputMode,
    requested_max_tokens: Option<usize>,
    task_intent: ModelServiceTaskIntentMetadata,
    timed: &TimedOutcome,
) -> String {
    let answer = (output_mode == ModelServiceOutputMode::Enhanced
        && timed.outcome.answer != timed.outcome.raw_answer)
        .then_some(timed.outcome.answer.as_str());
    let runtime_metadata = openai_norion_runtime_metadata_json(&timed.outcome);
    let task_metadata = model_service_task_metadata_json(&timed.outcome, task_intent);
    format!(
        "{{\"id\":\"chatcmpl-norion-{}\",\"object\":\"chat.completion.chunk\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}],\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"profile\":\"{}\",{},\"requested_max_tokens\":{},\"output_mode\":\"{}\",\"answer\":{},\"stream_state\":\"completed\",\"cancelled\":false,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"streamed_tokens\":{}, {},\"elapsed_ms\":{},\"persistent_writes\":true,\"memory_write_allowed\":true,\"genome_write_allowed\":true,\"self_evolution_write_allowed\":true}}}}",
        request_id,
        created,
        service_json_string(model),
        request_id,
        service_json_string(endpoint),
        service_json_string(model),
        profile_name_for_sse(profile),
        task_metadata,
        option_usize_service_json(requested_max_tokens),
        output_mode.as_str(),
        option_str_service_json(answer),
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
    let model_fallback = model_fallback_metadata_json(context.timed);
    let runtime_closed_loop_counters = runtime_closed_loop_counters_metadata_json(context.timed);
    format!(
        "{{\"id\":\"chatcmpl-norion-{}\",\"object\":\"chat.completion.chunk\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}],\"error\":{{\"message\":{},\"type\":\"{}\"}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"stream_state\":\"failed\",\"cancelled\":{},\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{},\"streamed_tokens\":{},\"stored_runtime_kv_memory_ids\":[],{},{},{},\"compute_budget\":null,\"compute_budget_summary\":{},\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_fanout_reduction\":0,\"compute_budget_read_only\":true,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,\"persistent_writes\":false,\"memory_write_allowed\":false,\"genome_write_allowed\":false,\"self_evolution_write_allowed\":false}}}}",
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
        model_fallback,
        runtime_closed_loop_counters,
        service_json_string(compute_budget_summary)
    )
}

fn neutral_memory_route_metadata_json() -> &'static str {
    "\"used_memory_count\":0,\"route_threshold\":0.000000,\"route_attention_tokens\":0,\"route_fast_tokens\":0,\"route_attention_fraction\":0.000000"
}

fn neutral_runtime_closed_loop_counters_json() -> &'static str {
    "\"runtime_closed_loop_counters\":{\"adaptive_routing_candidates\":0,\"adaptive_routing_saved_tokens\":0,\"adaptive_routing_threshold_delta_milli\":0,\"task_hierarchy_mutation_records\":0,\"task_hierarchy_compute_reduction_milli\":0,\"task_hierarchy_weight_delta_milli\":0,\"compute_budget_selected_candidates\":0,\"compute_budget_kv_lookups_skipped\":0,\"compute_budget_saved_tokens\":0,\"compute_budget_avoided_tokens\":0,\"compute_budget_write_allowed\":false,\"compute_budget_applied\":false,\"memory_admission_candidates\":0,\"memory_admission_ready\":0,\"memory_admission_blocked\":0,\"memory_admission_ledger_records\":0,\"memory_admission_ledger_preview_only\":0,\"memory_admission_ledger_authorized\":0,\"memory_admission_ledger_applied\":0,\"memory_admission_write_allowed\":false,\"memory_admission_applied\":false,\"kv_fusion_candidates\":0,\"kv_fusion_fused\":0,\"kv_fusion_compressed\":0,\"kv_fusion_skipped\":0,\"kv_fusion_held\":0,\"kv_fusion_rejected\":0,\"kv_fusion_approval_blocked\":0,\"kv_fusion_input_tokens\":0,\"kv_fusion_retained_tokens\":0,\"kv_fusion_saved_tokens\":0,\"kv_fusion_write_allowed\":false,\"kv_fusion_applied\":false,\"self_evolving_memory_store_updates\":0,\"self_evolving_memory_store_primary_applied\":false,\"self_evolving_memory_store_gist_applied\":0,\"self_evolving_memory_store_runtime_kv_applied\":0,\"memory_residency_retention_decayed\":0,\"memory_residency_retention_removed\":0,\"memory_residency_compaction_merged\":0,\"memory_residency_compaction_removed\":0,\"reflection_issues\":0,\"reflection_critical_issues\":0,\"reflection_revision_actions\":0,\"online_reward_feedbacks\":0,\"online_reward_reinforcements\":0,\"online_reward_penalties\":0,\"online_reward_strength_milli\":0,\"online_reward_reinforcement_strength_milli\":0,\"online_reward_penalty_strength_milli\":0,\"memory_feedback_updates\":0,\"memory_feedback_reinforcements\":0,\"memory_feedback_penalties\":0,\"noiron_orchestration_stages\":0,\"noiron_orchestration_completed_stages\":0,\"noiron_orchestration_failed_stages\":0,\"noiron_orchestration_preview_only_stages\":0,\"noiron_orchestration_gated_stages\":0,\"noiron_orchestration_rolled_back_stages\":0,\"noiron_orchestration_rollback_records\":0,\"noiron_orchestration_writes_gated\":true,\"noiron_orchestration_durable_memory_ledger_authorized\":0,\"noiron_orchestration_durable_memory_ledger_applied\":0,\"control_expression_profile_selected\":0,\"control_expression_context_anchor_promoted\":0,\"control_expression_suppression_gate_triggered\":0,\"control_expression_checkpoint_repair_requested\":0,\"control_expression_checkpoint_rejected\":0,\"control_expression_memory_refresh_candidate\":0,\"control_expression_memory_tombstone_candidate\":0,\"control_expression_preview_admission\":0,\"control_expression_write_allowed\":false,\"control_expression_applied\":false,\"control_expression_operator_approval_required\":true,\"control_expression_ready\":false}"
}

fn runtime_closed_loop_counters_metadata_json(timed: Option<&TimedOutcome>) -> String {
    timed.map_or_else(
        || neutral_runtime_closed_loop_counters_json().to_owned(),
        |timed| model_service_runtime_closed_loop_counters_json(&timed.outcome),
    )
}

fn model_fallback_metadata_json(timed: Option<&TimedOutcome>) -> String {
    timed.map_or_else(
        || "\"model_fallback\":null".to_owned(),
        |timed| model_service_model_fallback_json(&timed.outcome),
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
    use std::fs;
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};
    use std::path::PathBuf;
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::*;
    use rust_norion::{HeuristicBackend, InferenceRequest};

    struct PeakMemoryProbeBackend;

    impl InferenceBackend for PeakMemoryProbeBackend {
        fn generate(
            &mut self,
            _context: rust_norion::GenerationContext<'_>,
        ) -> rust_norion::reflection::InferenceDraft {
            rust_norion::reflection::InferenceDraft::new(
                "Rust borrowing keeps the generation transaction bounded while committed state remains stable and auditable.",
                vec![rust_norion::reflection::ReasoningStep::new(
                    "draft",
                    "bounded transaction result",
                    0.94,
                )],
            )
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum GenerationTestPath {
        Ordinary,
        Sse,
    }

    #[test]
    fn generation_save_failure_restores_full_engine_state() {
        let mut engine = NoironEngine::new();
        let before = engine.clone();
        let mut backend = HeuristicBackend;
        let state = ModelServiceServerState::default();
        let (mut args, root) = generation_test_args("save-rollback");
        let blocked_parent = root.join("blocked");
        fs::write(&blocked_parent, b"not a directory").unwrap();
        args.memory_path = blocked_parent.join("memory.ndkv");
        args.experience_path = blocked_parent.join("experience.ndkv");
        args.adaptive_path = blocked_parent.join("adaptive.ndkv");

        let (result, response) = capture_response(|stream| {
            handle_generate(
                &mut engine,
                &mut backend,
                GenerationHandlerContext {
                    state: &state,
                    args: &args,
                    stream,
                    request_id: 1,
                    endpoint: "generate-test",
                },
                generation_test_request(),
            )
        });

        result.unwrap();
        assert!(response.contains("\"error_type\":\"persistence_error\""));
        assert_eq!(engine.cache.len(), before.cache.len());
        assert_eq!(
            engine.experience.records().len(),
            before.experience.records().len()
        );
        assert_eq!(engine.router.observations(), before.router.observations());
        assert_eq!(
            engine.evolution_ledger.live_inference_runs,
            before.evolution_ledger.live_inference_runs
        );
        assert_eq!(
            engine.genome_runtime_state.profile(TaskProfile::General),
            before.genome_runtime_state.profile(TaskProfile::General)
        );
        let committed_memory = root.join("committed-memory.ndkv");
        let committed_experience = root.join("committed-experience.ndkv");
        let committed_adaptive = root.join("committed-adaptive.ndkv");
        engine
            .save_full_state(
                &committed_memory,
                &committed_experience,
                &committed_adaptive,
            )
            .unwrap();
        let restarted = NoironEngine::load_full_state(
            &committed_memory,
            &committed_experience,
            &committed_adaptive,
        )
        .unwrap();
        assert_eq!(restarted.cache.len(), before.cache.len());
        assert_eq!(restarted.experience.len(), before.experience.len());
        assert_eq!(
            restarted.router.observations(),
            before.router.observations()
        );
        assert_eq!(
            restarted.evolution_ledger.live_inference_runs,
            before.evolution_ledger.live_inference_runs
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn generation_stream_save_failure_restores_full_engine_state() {
        let mut engine = NoironEngine::new();
        let before = engine.clone();
        let mut backend = HeuristicBackend;
        let state = ModelServiceServerState::default();
        let (mut args, root) = generation_test_args("stream-save-rollback");
        let blocked_parent = root.join("blocked");
        fs::write(&blocked_parent, b"not a directory").unwrap();
        args.memory_path = blocked_parent.join("memory.ndkv");
        args.experience_path = blocked_parent.join("experience.ndkv");
        args.adaptive_path = blocked_parent.join("adaptive.ndkv");

        let response = capture_generation_test_response(
            GenerationTestPath::Sse,
            &mut engine,
            &mut backend,
            &state,
            &args,
            11,
        );

        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(
            response.contains("\"persistent_writes\":false"),
            "{response}"
        );
        assert_engine_request_state(&engine, &before);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn generation_runtime_io_error_rolls_back_ordinary_and_sse() {
        for (index, path) in [GenerationTestPath::Ordinary, GenerationTestPath::Sse]
            .into_iter()
            .enumerate()
        {
            let mut engine = NoironEngine::new();
            let before = engine.clone();
            let mut backend = PeakMemoryProbeBackend;
            let state = ModelServiceServerState::default();
            let (mut args, root) = generation_test_args(&format!("runtime-rollback-{path:?}"));
            let blocked_parent = root.join("blocked-trace");
            fs::write(&blocked_parent, b"not a directory").unwrap();
            args.trace_path = Some(blocked_parent.join("trace.jsonl"));

            let response = capture_generation_test_response(
                path,
                &mut engine,
                &mut backend,
                &state,
                &args,
                20 + index,
            );

            assert!(response.contains("runtime"), "{response}");
            assert_engine_request_state(&engine, &before);
            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn generation_postprocess_error_rolls_back_ordinary_and_sse() {
        for (index, path) in [GenerationTestPath::Ordinary, GenerationTestPath::Sse]
            .into_iter()
            .enumerate()
        {
            let mut engine = NoironEngine::new();
            let before = engine.clone();
            let mut backend = PeakMemoryProbeBackend;
            let state = ModelServiceServerState::default();
            let (args, root) = generation_test_args(&format!("postprocess-rollback-{path:?}"));
            let mut request = generation_test_request();
            request.case_name = Some("gemma-service-en-routing".to_owned());

            let response = FORCE_POSTPROCESS_ERROR.with(|force| {
                force.set(true);
                let response = capture_generation_test_response_with_request(
                    path,
                    &mut engine,
                    &mut backend,
                    &state,
                    &args,
                    25 + index,
                    request,
                );
                force.set(false);
                response
            });

            assert!(response.contains("postprocess"), "{response}");
            assert_engine_request_state(&engine, &before);
            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn generation_cancellation_rolls_back_ordinary_and_sse() {
        for (index, path) in [GenerationTestPath::Ordinary, GenerationTestPath::Sse]
            .into_iter()
            .enumerate()
        {
            let mut engine = NoironEngine::new();
            let before = engine.clone();
            let mut backend = PeakMemoryProbeBackend;
            let state = ModelServiceServerState::default();
            let (args, root) = generation_test_args(&format!("cancel-rollback-{path:?}"));
            let request_id = 30 + index;
            let _active = state.begin_engine_request(request_id, "generate-test", "cancel me");
            let cancellation = state.request_cancel(
                request_id,
                "test cancellation",
                "repair_factor:runtime_splice",
            );
            assert!(cancellation.target_active);

            let response = capture_generation_test_response(
                path,
                &mut engine,
                &mut backend,
                &state,
                &args,
                request_id,
            );

            assert!(response.contains("cancel"), "{response}");
            assert_engine_request_state(&engine, &before);
            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn generation_conflict_is_not_reported_as_a_timeout() {
        for (index, path) in [GenerationTestPath::Ordinary, GenerationTestPath::Sse]
            .into_iter()
            .enumerate()
        {
            let (args, root) = generation_test_args(&format!("generation-conflict-{path:?}"));
            let mut base = NoironEngine::new();
            base.save_full_state(
                &args.memory_path,
                &args.experience_path,
                &args.adaptive_path,
            )
            .unwrap();
            let mut stale = NoironEngine::load_full_state(
                &args.memory_path,
                &args.experience_path,
                &args.adaptive_path,
            )
            .unwrap();
            let stale_before = stale.clone();
            let mut current = stale.clone();
            let mut backend = HeuristicBackend;
            current.infer(
                InferenceRequest::new("publish current generation", TaskProfile::Coding),
                &mut backend,
            );
            current
                .save_full_state(
                    &args.memory_path,
                    &args.experience_path,
                    &args.adaptive_path,
                )
                .unwrap();
            let state = ModelServiceServerState::default();

            let response = capture_generation_test_response(
                path,
                &mut stale,
                &mut backend,
                &state,
                &args,
                40 + index,
            );

            assert!(response.contains("full-state generation conflict"));
            assert!(response.contains("\"timeout\":false"));
            assert!(response.contains("\"retryable\":false"));
            assert_engine_request_state(&stale, &stale_before);
            let retry_error = stale
                .save_full_state(
                    &args.memory_path,
                    &args.experience_path,
                    &args.adaptive_path,
                )
                .unwrap_err();
            assert!(retry_error.to_string().contains("engine=1 current=2"));
            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn generation_success_keeps_full_engine_state_changes() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let state = ModelServiceServerState::default();
        let (args, root) = generation_test_args("success-keeps-state");

        let (result, response) = capture_response(|stream| {
            handle_generate(
                &mut engine,
                &mut backend,
                GenerationHandlerContext {
                    state: &state,
                    args: &args,
                    stream,
                    request_id: 2,
                    endpoint: "generate-test",
                },
                generation_test_request(),
            )
        });

        result.unwrap();
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(!engine.cache.is_empty());
        assert!(!engine.experience.records().is_empty());
        assert!(engine.router.observations() > 0);
        assert!(engine.evolution_ledger.live_inference_runs > 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[ignore = "manual same-fixture peak-working-set evidence"]
    fn generation_hot_path_peak_memory_probe() {
        const LARGE_EXPERIENCE_IDS: usize = 16 * 1024 * 1024;

        let mode = std::env::var("RUST_NORION_GENERATION_MEMORY_PROBE")
            .unwrap_or_else(|_| "baseline".to_owned());
        let mut engine = NoironEngine::new();
        let mut backend = PeakMemoryProbeBackend;
        let fixture = engine.infer(
            InferenceRequest::new("seed rollback memory fixture", TaskProfile::General),
            &mut backend,
        );
        let fixture_record = engine
            .experience
            .record_mut(fixture.experience_id)
            .expect("fixture experience");
        fixture_record.prompt =
            "Conversation transcript:\nuser: explain bounded Rust borrowing\nassistant: done"
                .to_owned();
        fixture_record.lesson =
            "unrelated shell transcript: ssh -o ConnectTimeout=8 host; merge_requests".to_owned();
        fixture_record.stored_memory_id = None;
        fixture_record.gist_memory_ids.clear();
        fixture_record.stored_runtime_kv_memory_ids.clear();
        fixture_record.used_memory_ids = vec![u64::MAX; LARGE_EXPERIENCE_IDS];
        engine.set_auto_replay_limit(0);

        let (args, root) = generation_test_args(&format!("memory-probe-{mode}"));
        let state = ModelServiceServerState::default();
        let rollback_clone = match mode.as_str() {
            "baseline" => None,
            "clone-baseline" => Some(engine.clone()),
            "ordinary" => {
                let (result, response) = capture_response(|stream| {
                    handle_generate(
                        &mut engine,
                        &mut backend,
                        GenerationHandlerContext {
                            state: &state,
                            args: &args,
                            stream,
                            request_id: 41,
                            endpoint: "generate-memory-probe",
                        },
                        generation_test_request(),
                    )
                });
                result.unwrap();
                assert!(response.starts_with("HTTP/1.1 200"), "{response}");
                None
            }
            "sse" => {
                let (result, response) = capture_response(|stream| {
                    handle_generate_stream(
                        &mut engine,
                        &mut backend,
                        GenerationHandlerContext {
                            state: &state,
                            args: &args,
                            stream,
                            request_id: 42,
                            endpoint: "generate-stream-memory-probe",
                        },
                        generation_test_request(),
                    )
                });
                result.unwrap();
                assert!(response.starts_with("HTTP/1.1 200"), "{response}");
                None
            }
            other => panic!("unsupported memory probe mode: {other}"),
        };

        assert_eq!(
            engine.experience.records()[0].used_memory_ids.len(),
            LARGE_EXPERIENCE_IDS
        );
        if matches!(mode.as_str(), "ordinary" | "sse") {
            let (_, experience_path, _) = NoironEngine::full_state_read_paths(
                &args.memory_path,
                &args.experience_path,
                &args.adaptive_path,
            )
            .unwrap();
            assert!(
                fs::metadata(experience_path).unwrap().len() < 1024 * 1024,
                "quarantined fixture must not enter persistence"
            );
        }
        println!(
            "generation_peak_memory_probe mode={mode} fixture_bytes={} experience_records={}",
            LARGE_EXPERIENCE_IDS * std::mem::size_of::<u64>(),
            engine.experience.len()
        );
        std::hint::black_box(&rollback_clone);
        thread::sleep(Duration::from_millis(300));
        let _ = fs::remove_dir_all(root);
    }

    fn capture_response(
        run: impl FnOnce(&mut TcpStream) -> std::io::Result<()>,
    ) -> (std::io::Result<()>, String) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let reader = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            response
        });
        let mut stream = TcpStream::connect(address).unwrap();
        let result = run(&mut stream);
        drop(stream);
        (result, reader.join().unwrap())
    }

    fn capture_generation_test_response<B: InferenceBackend>(
        path: GenerationTestPath,
        engine: &mut NoironEngine,
        backend: &mut B,
        state: &ModelServiceServerState,
        args: &Args,
        request_id: usize,
    ) -> String {
        capture_generation_test_response_with_request(
            path,
            engine,
            backend,
            state,
            args,
            request_id,
            generation_test_request(),
        )
    }

    fn capture_generation_test_response_with_request<B: InferenceBackend>(
        path: GenerationTestPath,
        engine: &mut NoironEngine,
        backend: &mut B,
        state: &ModelServiceServerState,
        args: &Args,
        request_id: usize,
        request: ModelServiceRequest,
    ) -> String {
        let (result, response) = capture_response(|stream| match path {
            GenerationTestPath::Ordinary => handle_generate(
                engine,
                backend,
                GenerationHandlerContext {
                    state,
                    args,
                    stream,
                    request_id,
                    endpoint: "generate-test",
                },
                request,
            ),
            GenerationTestPath::Sse => handle_generate_stream(
                engine,
                backend,
                GenerationHandlerContext {
                    state,
                    args,
                    stream,
                    request_id,
                    endpoint: "generate-stream-test",
                },
                request,
            ),
        });
        result.unwrap();
        response
    }

    fn assert_engine_request_state(actual: &NoironEngine, expected: &NoironEngine) {
        assert_eq!(
            format!("{:?}", actual.cache.entries()),
            format!("{:?}", expected.cache.entries())
        );
        assert_eq!(actual.cache.clock(), expected.cache.clock());
        assert_eq!(
            format!("{:?}", actual.experience.records()),
            format!("{:?}", expected.experience.records())
        );
        assert_eq!(
            format!("{:?}", actual.adaptive_state()),
            format!("{:?}", expected.adaptive_state())
        );
    }

    fn generation_test_args(case_name: &str) -> (Args, PathBuf) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "rust-norion-generation-{case_name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut args = Args::parse(Vec::new());
        args.memory_path = root.join("memory.ndkv");
        args.experience_path = root.join("experience.ndkv");
        args.adaptive_path = root.join("adaptive.ndkv");
        (args, root)
    }

    fn generation_test_request() -> ModelServiceRequest {
        ModelServiceRequest {
            prompt: "explain bounded local memory routing with stable evidence".to_owned(),
            profile: Some(TaskProfile::General),
            case_name: None,
            output_mode: ModelServiceOutputMode::Enhanced,
            max_tokens: Some(128),
            tenant_scope: Some(TenantScope::local_single_user()),
            evolution_preview: false,
        }
    }

    fn evolution_candidate_receipt() -> ModelServiceEvolutionCandidateReceipt {
        ModelServiceEvolutionCandidateReceipt {
            eligible: true,
            token: Some("candidate-token".to_owned()),
            prompt_digest: "redaction-digest:prompt".to_owned(),
            candidate_digest: "redaction-digest:candidate".to_owned(),
            generation_before: 3,
            residency_revision_before: 9,
            candidate_count: 2,
            expires_in_seconds: 300,
            reason: "ready_for_explicit_apply".to_owned(),
        }
    }

    fn behavior_feedback_receipt() -> ModelServiceBehaviorFeedbackReceipt {
        ModelServiceBehaviorFeedbackReceipt {
            token: "behavior-token".to_owned(),
            experience_id: 42,
            expires_in_seconds: 300,
            runtime_model: Some("model-a".to_owned()),
            task_kind: "gomoku".to_owned(),
        }
    }

    #[test]
    fn evolution_candidate_metadata_is_inserted_inside_openai_norion_object() {
        let body = append_evolution_candidate_json(
            "{\"id\":\"chat\",\"norion\":{\"quality\":0.9}}",
            &GenerationResponseFormat::OpenAiChatCompletion { model: None },
            Some(&evolution_candidate_receipt()),
        );

        assert!(body.starts_with("{\"id\":\"chat\",\"norion\":{"));
        assert!(body.contains("\"evolution_candidate\":{\"eligible\":true"));
        assert!(body.contains("\"token\":\"candidate-token\""));
        assert!(body.contains("\"residency_revision_before\":9"));
        assert!(body.ends_with("}}"));
    }

    #[test]
    fn behavior_feedback_metadata_is_inserted_inside_openai_norion_object() {
        let body = append_behavior_feedback_json(
            "{\"id\":\"chat\",\"norion\":{\"quality\":0.9}}",
            &GenerationResponseFormat::OpenAiChatCompletion { model: None },
            Some(&behavior_feedback_receipt()),
        );

        assert!(body.starts_with("{\"id\":\"chat\",\"norion\":{"));
        assert!(body.contains("\"behavior_feedback\":{\"eligible\":true"));
        assert!(body.contains("\"token\":\"behavior-token\""));
        assert!(body.contains("\"experience_id\":42"));
        assert!(body.contains("\"runtime_model\":\"model-a\""));
        assert!(body.contains("\"task_kind\":\"gomoku\""));
        assert!(body.ends_with("}}"));
    }

    #[test]
    fn evolution_candidate_metadata_is_inserted_at_model_service_root() {
        let body = append_evolution_candidate_json(
            "{\"ok\":true}",
            &GenerationResponseFormat::ModelService,
            Some(&evolution_candidate_receipt()),
        );

        assert!(body.starts_with("{\"ok\":true,\"evolution_candidate\":{"));
        assert!(body.ends_with('}'));
    }

    #[test]
    fn stream_cancel_final_json_preserves_partial_count_and_blocks_writes() {
        let body = stream_cancel_final_json(
            7,
            "generate-stream",
            3,
            "request cancelled by runtime_request_splice",
            None,
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
        assert!(body.contains("\"model_fallback\":null"));
        assert!(body.contains("\"persistent_writes\":false"));
        assert!(body.contains("\"memory_write_allowed\":false"));
        assert!(body.contains("\"genome_write_allowed\":false"));
        assert!(body.contains("\"self_evolution_write_allowed\":false"));
    }

    #[test]
    fn stream_cancellation_preserves_timed_fallback_receipt() {
        let timed = timed_failed_fallback_fixture();
        let model_service = stream_cancel_final_json(
            8,
            "generate-stream",
            0,
            "request cancelled after fallback",
            Some(&timed),
        );
        let openai = openai_chat_completion_stream_error_json(OpenAiStreamErrorContext {
            request_id: 8,
            model: "rust-norion-local",
            created: 13,
            endpoint: "chat-completions-stream",
            streamed_tokens: 0,
            message: "request cancelled after fallback",
            runtime_error_note: None,
            cancelled: true,
            timeout: false,
            retryable: false,
            timed: Some(&timed),
        });

        for body in [model_service, openai] {
            assert!(body.contains("\"cancelled\":true"));
            assert!(body.contains("\"model_fallback\":{"));
            assert!(body.contains("\"attempts\":3"));
            assert!(body.contains("\"all_failed\":true"));
        }
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
        assert!(body.contains("\"adaptive_routing_candidates\":0"));
        assert!(body.contains("\"task_hierarchy_compute_reduction_milli\":0"));
        assert!(body.contains("\"compute_budget_saved_tokens\":0"));
        assert!(body.contains("\"compute_budget_avoided_tokens\":0"));
        assert!(body.contains("\"memory_admission_ledger_authorized\":0"));
        assert!(body.contains("\"memory_admission_ledger_applied\":0"));
        assert!(body.contains("\"kv_fusion_saved_tokens\":0"));
        assert!(body.contains("\"kv_fusion_applied\":false"));
        assert!(body.contains("\"self_evolving_memory_store_updates\":0"));
        assert!(body.contains("\"memory_residency_retention_removed\":0"));
        assert!(body.contains("\"reflection_issues\":0"));
        assert!(body.contains("\"online_reward_feedbacks\":0"));
    }

    fn assert_runtime_closed_loop_counter_fields(body: &str) {
        assert!(body.contains("\"runtime_closed_loop_counters\":{"));
        assert!(body.contains("\"adaptive_routing_candidates\":"));
        assert!(body.contains("\"adaptive_routing_saved_tokens\":"));
        assert!(body.contains("\"adaptive_routing_threshold_delta_milli\":"));
        assert!(body.contains("\"task_hierarchy_mutation_records\":"));
        assert!(body.contains("\"task_hierarchy_compute_reduction_milli\":"));
        assert!(body.contains("\"task_hierarchy_weight_delta_milli\":"));
        assert!(body.contains("\"compute_budget_saved_tokens\":"));
        assert!(body.contains("\"compute_budget_avoided_tokens\":"));
        assert!(body.contains("\"memory_admission_ledger_authorized\":"));
        assert!(body.contains("\"memory_admission_ledger_applied\":"));
        assert!(body.contains("\"kv_fusion_saved_tokens\":"));
        assert!(body.contains("\"self_evolving_memory_store_updates\":"));
        assert!(body.contains("\"memory_residency_retention_removed\":"));
        assert!(body.contains("\"reflection_issues\":"));
        assert!(body.contains("\"reflection_revision_actions\":"));
        assert!(body.contains("\"online_reward_feedbacks\":"));
        assert!(body.contains("\"online_reward_strength_milli\":"));
        assert!(body.contains("\"memory_feedback_updates\":"));
        assert!(body.contains("\"noiron_orchestration_stages\":"));
        assert!(body.contains("\"noiron_orchestration_failed_stages\":"));
        assert!(body.contains("\"noiron_orchestration_writes_gated\":"));
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

    #[test]
    fn runtime_closed_loop_counters_report_reflection_and_reward_values() {
        let mut timed = timed_route_fixture();
        timed.outcome.live_evolution.router_threshold_delta = 0.125;
        timed.outcome.live_evolution.hierarchy_weight_delta = 0.25;
        timed.outcome.live_evolution.reflection_issues = 3;
        timed.outcome.live_evolution.critical_reflection_issues = 1;
        timed.outcome.live_evolution.revision_actions = 2;
        timed.outcome.live_evolution.online_reward_feedbacks = 4;
        timed.outcome.live_evolution.online_reward_reinforcements = 3;
        timed.outcome.live_evolution.online_reward_penalties = 1;
        timed.outcome.live_evolution.online_reward_strength = 1.5;
        timed
            .outcome
            .live_evolution
            .online_reward_reinforcement_strength = 1.25;
        timed.outcome.live_evolution.online_reward_penalty_strength = 0.25;
        timed.outcome.live_evolution.memory_reinforcements = 5;
        timed.outcome.live_evolution.memory_penalties = 2;
        let orchestration = timed.outcome.orchestration_trace();
        let control_expression = &orchestration.control_expression;

        let body = model_service_runtime_closed_loop_counters_json(&timed.outcome);

        assert!(body.contains("\"adaptive_routing_candidates\":"));
        assert!(body.contains("\"adaptive_routing_threshold_delta_milli\":125"));
        assert!(body.contains("\"task_hierarchy_mutation_records\":"));
        assert!(body.contains("\"task_hierarchy_weight_delta_milli\":250"));
        assert!(body.contains("\"reflection_issues\":3"));
        assert!(body.contains("\"reflection_critical_issues\":1"));
        assert!(body.contains("\"reflection_revision_actions\":2"));
        assert!(body.contains("\"online_reward_feedbacks\":4"));
        assert!(body.contains("\"online_reward_reinforcements\":3"));
        assert!(body.contains("\"online_reward_penalties\":1"));
        assert!(body.contains("\"online_reward_strength_milli\":1500"));
        assert!(body.contains("\"online_reward_reinforcement_strength_milli\":1250"));
        assert!(body.contains("\"online_reward_penalty_strength_milli\":250"));
        assert!(body.contains("\"memory_feedback_updates\":7"));
        assert!(body.contains("\"memory_feedback_reinforcements\":5"));
        assert!(body.contains("\"memory_feedback_penalties\":2"));
        assert!(body.contains(&format!(
            "\"noiron_orchestration_stages\":{}",
            orchestration.stages.len()
        )));
        assert!(body.contains(&format!(
            "\"noiron_orchestration_failed_stages\":{}",
            orchestration.failed_stages().len()
        )));
        assert!(body.contains(&format!(
            "\"noiron_orchestration_writes_gated\":{}",
            orchestration.all_writes_gated()
        )));
        assert!(body.contains(&format!(
            "\"noiron_orchestration_live_feedback_closed\":{}",
            orchestration.live_feedback_closed()
        )));
        assert!(body.contains(&format!(
            "\"noiron_orchestration_durable_memory_ledger_authorized\":{}",
            orchestration.gates.durable_memory_ledger_authorized
        )));
        assert!(body.contains(&format!(
            "\"noiron_orchestration_durable_memory_ledger_applied\":{}",
            orchestration.gates.durable_memory_ledger_applied
        )));
        assert!(body.contains(&format!(
            "\"control_expression_profile_selected\":{}",
            control_expression.control_expression_profile_selected
        )));
        assert!(body.contains(&format!(
            "\"control_expression_context_anchor_promoted\":{}",
            control_expression.context_anchor_promoted
        )));
        assert!(body.contains(&format!(
            "\"control_expression_suppression_gate_triggered\":{}",
            control_expression.suppression_gate_triggered
        )));
        assert!(body.contains(&format!(
            "\"control_expression_checkpoint_repair_requested\":{}",
            control_expression.checkpoint_repair_requested
        )));
        assert!(body.contains(&format!(
            "\"control_expression_checkpoint_rejected\":{}",
            control_expression.checkpoint_rejected
        )));
        assert!(body.contains(&format!(
            "\"control_expression_memory_refresh_candidate\":{}",
            control_expression.memory_refresh_candidate
        )));
        assert!(body.contains(&format!(
            "\"control_expression_memory_tombstone_candidate\":{}",
            control_expression.memory_tombstone_candidate
        )));
        assert!(body.contains(&format!(
            "\"control_expression_preview_admission\":{}",
            control_expression.control_expression_preview_admission
        )));
        assert!(body.contains(&format!(
            "\"control_expression_write_allowed\":{}",
            control_expression.write_allowed
        )));
        assert!(body.contains(&format!(
            "\"control_expression_applied\":{}",
            control_expression.applied
        )));
        assert!(body.contains(&format!(
            "\"control_expression_operator_approval_required\":{}",
            control_expression.operator_approval_required
        )));
        assert!(body.contains(&format!(
            "\"control_expression_ready\":{}",
            control_expression.ready()
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

    fn timed_failed_fallback_fixture() -> TimedOutcome {
        let mut timed = timed_route_fixture();
        let diagnostics = &mut timed.outcome.runtime_diagnostics;
        diagnostics.model_fallback_configured = true;
        diagnostics.model_fallback_primary_failed = true;
        diagnostics.model_fallback_attempts = 3;
        diagnostics.model_fallback_failures = 3;
        diagnostics.model_fallback_quarantined = 2;
        diagnostics.model_fallback_cooldown_skipped = 1;
        diagnostics.model_fallback_all_failed = true;
        timed
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
        assert!(body.contains("\"model_fallback\":null"));
        assert_failed_memory_route_budget_fields(&body);
        assert_failed_runtime_closed_loop_counter_fields(&body);
    }

    #[test]
    fn generation_error_json_preserves_timed_fallback_receipt() {
        let timed = timed_failed_fallback_fixture();
        let body = generation_error_json(
            &GenerationResponseFormat::ModelService,
            18,
            "generate",
            "runtime failed after fallback",
            "runtime_error",
            false,
            true,
            Some("runtime_error:label=runtime_error:timeout=false"),
            Some(&timed),
        );

        assert!(body.contains("\"model_fallback\":{"));
        assert!(body.contains("\"configured\":true"));
        assert!(body.contains("\"primary_failed\":true"));
        assert!(body.contains("\"attempts\":3"));
        assert!(body.contains("\"failures\":3"));
        assert!(body.contains("\"quarantined\":2"));
        assert!(body.contains("\"cooldown_skipped\":1"));
        assert!(body.contains("\"all_failed\":true"));
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
    fn openai_stream_final_reports_requested_max_tokens() {
        let timed = timed_route_fixture();
        let intent =
            model_service_task_intent_metadata("Write Rust ownership notes", TaskProfile::Coding);
        let body = openai_chat_completion_stream_final_json(
            5,
            "rust-norion-local",
            10,
            "chat-completions-stream",
            3,
            TaskProfile::Coding,
            ModelServiceOutputMode::Enhanced,
            Some(64),
            intent,
            &timed,
        );

        assert!(body.contains("\"stream_state\":\"completed\""));
        assert!(body.contains("\"requested_max_tokens\":64"));
        assert!(body.contains("\"output_mode\":\"enhanced\""));
        assert!(body.contains("\"coding_language\":\"rust\""));
        assert!(body.contains("\"runtime_closed_loop_counters\":{"));
    }

    #[test]
    fn openai_stream_final_reconciles_output_mode_answer() {
        let mut timed = timed_route_fixture();
        timed.outcome.raw_answer = "raw streamed draft".to_owned();
        timed.outcome.answer = "reflected final answer".to_owned();
        let intent = model_service_task_intent_metadata("Explain Rust", TaskProfile::Coding);

        let enhanced = openai_chat_completion_stream_final_json(
            6,
            "rust-norion-local",
            11,
            "chat-completions-stream",
            3,
            TaskProfile::Coding,
            ModelServiceOutputMode::Enhanced,
            Some(64),
            intent,
            &timed,
        );
        let raw = openai_chat_completion_stream_final_json(
            6,
            "rust-norion-local",
            11,
            "chat-completions-stream",
            3,
            TaskProfile::Coding,
            ModelServiceOutputMode::Raw,
            Some(64),
            intent,
            &timed,
        );

        assert!(enhanced.contains("\"output_mode\":\"enhanced\""));
        assert!(enhanced.contains("\"answer\":\"reflected final answer\""));
        assert!(raw.contains("\"output_mode\":\"raw\""));
        assert!(raw.contains("\"answer\":null"));

        timed.outcome.answer = timed.outcome.raw_answer.clone();
        let unchanged = openai_chat_completion_stream_final_json(
            6,
            "rust-norion-local",
            11,
            "chat-completions-stream",
            3,
            TaskProfile::Coding,
            ModelServiceOutputMode::Enhanced,
            Some(64),
            intent,
            &timed,
        );
        assert!(unchanged.contains("\"answer\":null"));
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
        let timed = timed_failed_fallback_fixture();
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
        assert!(body.contains("\"model_fallback\":{"));
        assert!(body.contains("\"attempts\":3"));
        assert!(body.contains("\"all_failed\":true"));
        assert_failed_stream_compute_budget_fields(&body);
        assert_actual_memory_route_budget_fields(&body, &timed);
        assert_runtime_closed_loop_counter_fields(&body);
        assert!(body.contains("\"persistent_writes\":false"));
    }
}
