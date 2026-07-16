use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use rust_norion::{
    DraftToken, GenerationContext, GenomeEvolutionAuthorization, InferenceBackend, InferenceDraft,
    InferenceRequest, NoironEngine, ReasoningStep, RuntimeError, TaskProfile, TenantScope,
    append_trace_jsonl, append_trace_jsonl_with_case,
};

use crate::model_service::http::{MODEL_POOL_CALL_CANCEL_MARKER, split_http_head_body};
use crate::model_service::json::{
    json_bool_field, json_string_array_field, json_string_field, service_json_string,
};
use crate::model_service::types::TimedOutcome;

const MODEL_POOL_CALL_URL_ENV: &str = "NORION_MODEL_POOL_CALL_URL";
const MODEL_POOL_CALL_DEFAULT_PATH: &str = "/v1/model-pool/call";
const MODEL_POOL_CALL_TIMEOUT: Duration = Duration::from_secs(300);
const MODEL_POOL_HTTP_CONNECT_TIMEOUT: Duration = Duration::from_millis(120);
const MODEL_POOL_HTTP_CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(20);
const MODEL_POOL_ROUTE_PLAN_URL_ENV: &str = "NORION_MODEL_POOL_ROUTE_PLAN_URL";
const MODEL_POOL_ROUTE_PLAN_DEFAULT_PATH: &str = "/v1/model-pool/route-plan";
const MODEL_POOL_ROUTE_PLAN_TIMEOUT: Duration = Duration::from_millis(600);

pub(crate) fn run_timed_inference<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_options(
        engine, backend, prompt, profile, None, trace_path, case_name,
    )
}

pub(crate) fn run_timed_inference_with_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_scope_options(
        engine, backend, prompt, profile, max_tokens, None, trace_path, case_name,
    )
}

pub(crate) fn run_timed_inference_with_scope_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_scope_options_and_genome_authorization(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        tenant_scope,
        trace_path,
        case_name,
        None,
    )
}

pub(crate) fn run_timed_inference_with_scope_options_cancelable<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    should_cancel: &mut dyn FnMut() -> bool,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_scope_and_route_plan_url_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        tenant_scope,
        trace_path,
        case_name,
        None,
        None,
        None,
        Some(should_cancel),
    )
}

pub(crate) fn run_timed_inference_with_scope_options_and_genome_authorization<
    B: InferenceBackend,
>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    genome_authorization: Option<GenomeEvolutionAuthorization>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_scope_and_route_plan_url_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        tenant_scope,
        trace_path,
        case_name,
        None,
        None,
        genome_authorization,
        None,
    )
}

#[cfg(test)]
pub(crate) fn run_timed_inference_with_model_pool_urls<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    route_plan_url: &str,
    call_url: &str,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_scope_and_route_plan_url_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        None,
        trace_path,
        case_name,
        Some(route_plan_url),
        Some(call_url),
        None,
        None,
    )
}

fn run_timed_inference_with_scope_and_route_plan_url_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    route_plan_url: Option<&str>,
    call_url: Option<&str>,
    genome_authorization: Option<GenomeEvolutionAuthorization>,
    mut should_cancel: Option<&mut dyn FnMut() -> bool>,
) -> std::io::Result<TimedOutcome> {
    let started = Instant::now();
    let request = if let Some(route_plan_url) = route_plan_url {
        inference_request_with_options_and_route_plan_url(
            prompt.clone(),
            profile,
            max_tokens,
            tenant_scope,
            Some(route_plan_url),
        )
    } else {
        inference_request_with_options(prompt.clone(), profile, max_tokens, tenant_scope)
    };
    let request = match genome_authorization {
        Some(authorization) => request.with_genome_evolution_authorization(authorization),
        None => request,
    };
    let call_url_env = if call_url.is_none() {
        std::env::var(MODEL_POOL_CALL_URL_ENV).ok()
    } else {
        None
    };
    let call_url = call_url
        .or(call_url_env.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let outcome = if let Some(call_url) = call_url {
        let mut model_pool_backend = ModelPoolCallBackend {
            fallback: backend,
            call_url,
            configured_max_tokens: max_tokens,
        };
        if let Some(should_cancel) = should_cancel.as_mut() {
            engine.infer_cancelable(request, &mut model_pool_backend, *should_cancel)
        } else {
            engine.infer(request, &mut model_pool_backend)
        }
    } else if let Some(should_cancel) = should_cancel.as_mut() {
        engine.infer_cancelable(request, backend, *should_cancel)
    } else {
        engine.infer(request, backend)
    };
    let elapsed_ms = started.elapsed().as_millis();
    let cancelled = should_cancel.as_mut().is_some_and(|cancel| cancel());

    let trace_result = if let Some(trace_path) = trace_path {
        if let Some(case_name) = case_name {
            append_trace_jsonl_with_case(
                trace_path, case_name, &prompt, profile, elapsed_ms, &outcome,
            )
        } else {
            append_trace_jsonl(trace_path, &prompt, profile, elapsed_ms, &outcome)
        }
    } else {
        Ok(())
    };
    if !cancelled {
        trace_result?;
    }

    Ok(TimedOutcome {
        outcome,
        elapsed_ms,
    })
}

struct ModelPoolCallBackend<'a, B: InferenceBackend> {
    fallback: &'a mut B,
    call_url: &'a str,
    configured_max_tokens: Option<usize>,
}

impl<B: InferenceBackend> ModelPoolCallBackend<'_, B> {
    fn generate_fallback_cancelable(
        &mut self,
        context: GenerationContext<'_>,
        should_cancel: &mut dyn FnMut() -> bool,
    ) -> InferenceDraft {
        self.fallback
            .configure_generation(self.configured_max_tokens);
        self.fallback.generate_cancelable(context, should_cancel)
    }

    fn generate_fallback_stream_cancelable(
        &mut self,
        context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
        should_cancel: &mut dyn FnMut() -> bool,
    ) -> InferenceDraft {
        self.fallback
            .configure_generation(self.configured_max_tokens);
        self.fallback
            .generate_stream_checked_cancelable(context, on_token, should_cancel)
    }
}

impl<B: InferenceBackend> InferenceBackend for ModelPoolCallBackend<'_, B> {
    fn defer_auto_replay_until_generation_result(&self) -> bool {
        true
    }

    fn configure_generation(&mut self, max_tokens: Option<usize>) {
        self.configured_max_tokens = max_tokens;
    }

    fn configure_runtime_endpoint_override(
        &mut self,
        base_url: Option<&str>,
    ) -> Result<bool, String> {
        self.fallback.configure_runtime_endpoint_override(base_url)
    }

    fn runtime_endpoint_override_active(&self) -> Option<&str> {
        self.fallback.runtime_endpoint_override_active()
    }

    fn runtime_native_context_window(&self) -> Option<usize> {
        self.fallback.runtime_native_context_window()
    }

    fn embed_text(&mut self, _text: &str) -> Option<Vec<f32>> {
        None
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        let mut never_cancel = || false;
        self.generate_cancelable(context, &mut never_cancel)
    }

    fn generate_cancelable(
        &mut self,
        context: GenerationContext<'_>,
        should_cancel: &mut dyn FnMut() -> bool,
    ) -> InferenceDraft {
        if should_cancel() {
            return self.fallback.generate_cancelable(context, should_cancel);
        }
        match fetch_model_pool_call_answer(
            self.call_url,
            context.prompt,
            self.configured_max_tokens,
            should_cancel,
        ) {
            Ok(_) if should_cancel() => self.fallback.generate_cancelable(context, should_cancel),
            Ok(call) => model_pool_call_draft(call.answer, call.streamed_tokens),
            Err(_) if should_cancel() => self.fallback.generate_cancelable(context, should_cancel),
            Err(error) if error.retryable => {
                self.generate_fallback_cancelable(context, should_cancel)
            }
            Err(error) => model_pool_call_blocked_draft(error),
        }
    }

    fn generate_wave_cancelable(
        &mut self,
        contexts: &[GenerationContext<'_>],
        should_cancel: &mut dyn FnMut() -> bool,
    ) -> (Vec<InferenceDraft>, bool) {
        if contexts.len() <= 1 {
            let drafts = contexts
                .iter()
                .map(|context| self.generate_cancelable((*context).clone(), should_cancel))
                .collect::<Vec<_>>();
            return (drafts, should_cancel());
        }
        if should_cancel() {
            return (
                contexts
                    .iter()
                    .take(1)
                    .map(|_| model_pool_call_cancelled_draft())
                    .collect(),
                true,
            );
        }

        let prompts = contexts
            .iter()
            .map(|context| context.prompt)
            .collect::<Vec<_>>();
        let wave = fetch_model_pool_call_wave(
            self.call_url,
            &prompts,
            self.configured_max_tokens,
            should_cancel,
        );
        if wave.cancelled {
            return (
                contexts
                    .iter()
                    .map(|_| model_pool_call_cancelled_draft())
                    .collect(),
                true,
            );
        }

        let mut drafts = Vec::with_capacity(contexts.len());
        for (context, result) in contexts.iter().zip(wave.results) {
            let draft = match result {
                Ok(call) => model_pool_call_draft(call.answer, call.streamed_tokens),
                Err(error) if error.retryable => {
                    self.generate_fallback_cancelable((*context).clone(), should_cancel)
                }
                Err(error) => model_pool_call_blocked_draft(error),
            };
            let cancelled = should_cancel();
            drafts.push(draft);
            if cancelled {
                drafts.extend(
                    (drafts.len()..contexts.len()).map(|_| model_pool_call_cancelled_draft()),
                );
                return (drafts, true);
            }
        }
        (drafts, false)
    }

    fn generate_stream_checked(
        &mut self,
        context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
    ) -> InferenceDraft {
        let mut never_cancel = || false;
        self.generate_stream_checked_cancelable(context, on_token, &mut never_cancel)
    }

    fn generate_stream_checked_cancelable(
        &mut self,
        context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
        should_cancel: &mut dyn FnMut() -> bool,
    ) -> InferenceDraft {
        if should_cancel() {
            return self.fallback.generate_stream_checked_cancelable(
                context,
                on_token,
                should_cancel,
            );
        }
        match fetch_model_pool_call_answer_with_stream(
            self.call_url,
            context.prompt,
            self.configured_max_tokens,
            true,
            should_cancel,
        ) {
            Ok(call) => {
                let streamed_tokens = if call.streamed_tokens.is_empty() {
                    vec![call.answer.clone()]
                } else {
                    call.streamed_tokens
                };
                let draft = model_pool_call_draft(call.answer, streamed_tokens);
                for token in &draft.tokens {
                    if should_cancel() {
                        return self.fallback.generate_stream_checked_cancelable(
                            context,
                            on_token,
                            should_cancel,
                        );
                    }
                    if let Err(error) = on_token(token) {
                        return InferenceDraft::new(
                            format!("Runtime backend error: {}", error.message()),
                            vec![ReasoningStep::new(
                                "runtime_stream_observer_error",
                                error.message(),
                                0.0,
                            )],
                        );
                    }
                }
                draft
            }
            Err(_) if should_cancel() => {
                self.fallback
                    .generate_stream_checked_cancelable(context, on_token, should_cancel)
            }
            Err(error) if error.retryable => {
                self.generate_fallback_stream_cancelable(context, on_token, should_cancel)
            }
            Err(error) => model_pool_call_blocked_draft(error),
        }
    }
}

struct ModelPoolCallAnswer {
    answer: String,
    streamed_tokens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelPoolHttpError {
    message: String,
    retryable: bool,
}

impl ModelPoolHttpError {
    fn transport(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
        }
    }

    fn blocked(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
        }
    }

    fn response(label: &str, status_code: u16, body: &str) -> Self {
        let detail = json_string_field(body, "error")
            .or_else(|| json_string_field(body, "reason"))
            .or_else(|| json_string_field(body, "route_block_reason"))
            .unwrap_or_else(|| "response did not include a public error".to_owned());
        Self {
            message: format!("{label} returned status {status_code}: {detail}"),
            retryable: json_bool_field(body, "retryable") == Some(true),
        }
    }
}

fn model_pool_call_draft(answer: String, streamed_tokens: Vec<String>) -> InferenceDraft {
    let draft = InferenceDraft::new(
        answer.clone(),
        vec![ReasoningStep::new(
            "model_pool_call",
            "generated draft through model-pool call",
            0.9,
        )],
    );
    if streamed_tokens.is_empty() {
        draft
    } else {
        draft.with_tokens(streamed_tokens.into_iter().map(DraftToken::new).collect())
    }
}

fn model_pool_call_blocked_draft(error: ModelPoolHttpError) -> InferenceDraft {
    let detail = format!("{} retryable=false", error.message);
    InferenceDraft::new(
        format!("Runtime backend error: {detail}"),
        vec![ReasoningStep::new(
            "runtime_model_pool_call_blocked_error",
            detail,
            0.0,
        )],
    )
}

fn model_pool_call_cancelled_draft() -> InferenceDraft {
    InferenceDraft::new(
        "Runtime backend error: generation cancelled",
        vec![ReasoningStep::new(
            "runtime_generation_cancelled_error",
            "generation stopped after cancellation was requested",
            0.0,
        )],
    )
}

#[allow(dead_code)]
pub(crate) fn run_timed_inference_stream<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken),
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_with_options(
        engine, backend, prompt, profile, None, trace_path, case_name, on_token,
    )
}

pub(crate) fn run_timed_inference_stream_with_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken),
) -> std::io::Result<TimedOutcome> {
    let mut checked = |token: &DraftToken| {
        on_token(token);
        Ok(())
    };
    run_timed_inference_stream_checked_with_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        trace_path,
        case_name,
        &mut checked,
    )
}

pub(crate) fn run_timed_inference_stream_checked_with_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_checked_with_scope_options(
        engine, backend, prompt, profile, max_tokens, None, trace_path, case_name, on_token,
    )
}

pub(crate) fn run_timed_inference_stream_checked_with_scope_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_checked_with_scope_options_and_genome_authorization(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        tenant_scope,
        trace_path,
        case_name,
        None,
        on_token,
    )
}

pub(crate) fn run_timed_inference_stream_checked_with_scope_options_cancelable<
    B: InferenceBackend,
>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
    should_cancel: &mut dyn FnMut() -> bool,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_checked_with_scope_and_call_url_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        tenant_scope,
        trace_path,
        case_name,
        on_token,
        None,
        None,
        Some(should_cancel),
    )
}

pub(crate) fn run_timed_inference_stream_checked_with_scope_options_and_genome_authorization<
    B: InferenceBackend,
>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    genome_authorization: Option<GenomeEvolutionAuthorization>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_checked_with_scope_and_call_url_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        tenant_scope,
        trace_path,
        case_name,
        on_token,
        None,
        genome_authorization,
        None,
    )
}

#[cfg(test)]
pub(crate) fn run_timed_inference_stream_checked_with_model_pool_call_url<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
    call_url: &str,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_checked_with_scope_and_call_url_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        None,
        trace_path,
        case_name,
        on_token,
        Some(call_url),
        None,
        None,
    )
}

fn run_timed_inference_stream_checked_with_scope_and_call_url_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
    call_url: Option<&str>,
    genome_authorization: Option<GenomeEvolutionAuthorization>,
    mut should_cancel: Option<&mut dyn FnMut() -> bool>,
) -> std::io::Result<TimedOutcome> {
    let started = Instant::now();
    let request = inference_request_with_options(prompt.clone(), profile, max_tokens, tenant_scope);
    let request = match genome_authorization {
        Some(authorization) => request.with_genome_evolution_authorization(authorization),
        None => request,
    };
    let mut observer_error = None;
    let call_url_env = if call_url.is_none() {
        std::env::var(MODEL_POOL_CALL_URL_ENV).ok()
    } else {
        None
    };
    let call_url = call_url
        .or(call_url_env.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut outcome = {
        let mut checked = |token: &DraftToken| match on_token(token) {
            Ok(()) => Ok(()),
            Err(error) => {
                let message = error.to_string();
                observer_error = Some(error);
                Err(RuntimeError::new(format!(
                    "stream observer failed: {message}"
                )))
            }
        };
        if let Some(call_url) = call_url {
            let mut model_pool_backend = ModelPoolCallBackend {
                fallback: backend,
                call_url,
                configured_max_tokens: max_tokens,
            };
            if let Some(should_cancel) = should_cancel.as_mut() {
                engine.infer_stream_checked_cancelable(
                    request,
                    &mut model_pool_backend,
                    &mut checked,
                    *should_cancel,
                )
            } else {
                engine.infer_stream_checked(request, &mut model_pool_backend, &mut checked)
            }
        } else if let Some(should_cancel) = should_cancel.as_mut() {
            engine.infer_stream_checked_cancelable(request, backend, &mut checked, *should_cancel)
        } else {
            engine.infer_stream_checked(request, backend, &mut checked)
        }
    };
    if let Some(error) = observer_error.as_ref() {
        let message = format!("stream observer failed: {error}");
        let timeout = matches!(
            error.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ) || message.to_ascii_lowercase().contains("timed out")
            || message.to_ascii_lowercase().contains("timeout");
        let note = format!(
            "runtime_error:label=runtime_stream_observer_error:timeout={timeout}:message_chars={}",
            message.chars().count()
        );
        if !outcome
            .process_reward
            .notes
            .iter()
            .any(|item| item == &note)
        {
            outcome.process_reward.notes.push(note);
        }
    }
    let elapsed_ms = started.elapsed().as_millis();

    let trace_result = if let Some(trace_path) = trace_path {
        if let Some(case_name) = case_name {
            append_trace_jsonl_with_case(
                trace_path, case_name, &prompt, profile, elapsed_ms, &outcome,
            )
        } else {
            append_trace_jsonl(trace_path, &prompt, profile, elapsed_ms, &outcome)
        }
    } else {
        Ok(())
    };

    if let Some(error) = observer_error {
        let _ = trace_result;
        return Err(error);
    }
    trace_result?;

    Ok(TimedOutcome {
        outcome,
        elapsed_ms,
    })
}

fn inference_request_with_options(
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
) -> InferenceRequest {
    let route_plan_url = std::env::var(MODEL_POOL_ROUTE_PLAN_URL_ENV).ok();
    inference_request_with_options_and_route_plan_url(
        prompt,
        profile,
        max_tokens,
        tenant_scope,
        route_plan_url.as_deref(),
    )
}

fn inference_request_with_options_and_route_plan_url(
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<TenantScope>,
    route_plan_url: Option<&str>,
) -> InferenceRequest {
    let request = InferenceRequest::new(prompt, profile).with_max_tokens(max_tokens);
    let request =
        request.with_tenant_scope(tenant_scope.unwrap_or_else(TenantScope::local_single_user));
    let Some(route_plan_url) = route_plan_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return request;
    };

    match fetch_model_pool_route_plan_json(route_plan_url, &request.prompt, request.max_tokens) {
        Ok(route_plan_json) => request
            .clone()
            .try_with_agent_team_route_plan_json(&route_plan_json)
            .unwrap_or(request),
        Err(_) => request,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelPoolHttpEndpoint {
    host: String,
    port: u16,
    path: String,
}

fn fetch_model_pool_route_plan_json(
    route_plan_url: &str,
    prompt: &str,
    max_tokens: Option<usize>,
) -> Result<String, String> {
    let body = model_pool_route_plan_request_body(prompt, max_tokens);
    post_model_pool_json(
        route_plan_url,
        MODEL_POOL_ROUTE_PLAN_DEFAULT_PATH,
        MODEL_POOL_ROUTE_PLAN_TIMEOUT,
        "model pool route-plan",
        &body,
        None,
    )
    .map_err(|error| error.message)
}

struct ModelPoolCallWave {
    results: Vec<Result<ModelPoolCallAnswer, ModelPoolHttpError>>,
    cancelled: bool,
}

fn fetch_model_pool_call_wave(
    call_url: &str,
    prompts: &[&str],
    max_tokens: Option<usize>,
    should_cancel: &mut dyn FnMut() -> bool,
) -> ModelPoolCallWave {
    let cancellation = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut results = std::iter::repeat_with(|| None)
        .take(prompts.len())
        .collect::<Vec<_>>();
    let mut cancelled = false;

    std::thread::scope(|scope| {
        for (index, prompt) in prompts.iter().copied().enumerate() {
            let sender = sender.clone();
            let cancellation = std::sync::Arc::clone(&cancellation);
            scope.spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut worker_cancel =
                        || cancellation.load(std::sync::atomic::Ordering::Acquire);
                    fetch_model_pool_call_answer(call_url, prompt, max_tokens, &mut worker_cancel)
                }))
                .unwrap_or_else(|_| {
                    Err(ModelPoolHttpError::transport(
                        "model pool wave worker panicked",
                    ))
                });
                let _ = sender.send((index, result));
            });
        }
        drop(sender);

        let mut received = 0usize;
        while received < prompts.len() {
            match receiver.recv_timeout(MODEL_POOL_HTTP_CANCEL_POLL_INTERVAL) {
                Ok((index, result)) => {
                    results[index] = Some(result);
                    received = received.saturating_add(1);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
            if !cancelled && should_cancel() {
                cancelled = true;
                cancellation.store(true, std::sync::atomic::Ordering::Release);
            }
        }
    });

    if !cancelled && should_cancel() {
        cancelled = true;
    }
    ModelPoolCallWave {
        results: results
            .into_iter()
            .map(|result| {
                result.unwrap_or_else(|| {
                    Err(ModelPoolHttpError::transport(
                        "model pool wave worker ended without a result",
                    ))
                })
            })
            .collect(),
        cancelled,
    }
}

fn fetch_model_pool_call_answer(
    call_url: &str,
    prompt: &str,
    max_tokens: Option<usize>,
    should_cancel: &mut dyn FnMut() -> bool,
) -> Result<ModelPoolCallAnswer, ModelPoolHttpError> {
    fetch_model_pool_call_answer_with_stream(call_url, prompt, max_tokens, false, should_cancel)
}

fn fetch_model_pool_call_answer_with_stream(
    call_url: &str,
    prompt: &str,
    max_tokens: Option<usize>,
    stream: bool,
    should_cancel: &mut dyn FnMut() -> bool,
) -> Result<ModelPoolCallAnswer, ModelPoolHttpError> {
    let body = model_pool_call_request_body(prompt, max_tokens, stream);
    let response_body = post_model_pool_json(
        call_url,
        MODEL_POOL_CALL_DEFAULT_PATH,
        MODEL_POOL_CALL_TIMEOUT,
        "model pool call",
        &body,
        Some(should_cancel),
    )?;
    let answer = json_string_field(&response_body, "answer")
        .filter(|answer| !answer.trim().is_empty())
        .ok_or_else(|| ModelPoolHttpError::blocked("model pool call response missing answer"))?;
    let streamed_tokens = json_string_array_field(&response_body, "worker_streamed_tokens")
        .unwrap_or_default()
        .into_iter()
        .filter(|token| !token.is_empty())
        .collect();
    Ok(ModelPoolCallAnswer {
        answer,
        streamed_tokens,
    })
}

fn post_model_pool_json(
    url: &str,
    default_path: &str,
    timeout: Duration,
    label: &str,
    body: &str,
    should_cancel: Option<&mut dyn FnMut() -> bool>,
) -> Result<String, ModelPoolHttpError> {
    let endpoint = ModelPoolHttpEndpoint::parse(url, default_path, label)
        .map_err(ModelPoolHttpError::blocked)?;
    let response = post_model_pool_http_response(&endpoint, timeout, label, body, should_cancel)
        .map_err(ModelPoolHttpError::transport)?;
    let response = String::from_utf8(response).map_err(|error| {
        ModelPoolHttpError::blocked(format!("{label} response was not UTF-8: {error}"))
    })?;
    model_pool_http_body(&response, label)
}

fn post_model_pool_http_response(
    endpoint: &ModelPoolHttpEndpoint,
    timeout: Duration,
    label: &str,
    body: &str,
    mut should_cancel: Option<&mut dyn FnMut() -> bool>,
) -> Result<Vec<u8>, String> {
    let started = Instant::now();
    // ponytail: std DNS resolution can still block; model-pool URLs normally use local IP literals.
    let addresses = (endpoint.host.as_str(), endpoint.port)
        .to_socket_addrs()
        .map_err(|error| format!("{label} resolve failed: {error}"))?;
    let mut last_connect_error = None;
    let mut connected = None;
    for address in addresses {
        if should_cancel.as_mut().is_some_and(|cancel| (*cancel)()) {
            return Err(format!("{label} cancelled while connecting"));
        }
        let remaining = remaining_model_pool_http_timeout(started, timeout, label, "connect")?;
        match TcpStream::connect_timeout(&address, remaining.min(MODEL_POOL_HTTP_CONNECT_TIMEOUT)) {
            Ok(stream) => {
                connected = Some(stream);
                break;
            }
            Err(error) => last_connect_error = Some(error),
        }
    }
    let mut stream = connected.ok_or_else(|| {
        last_connect_error.map_or_else(
            || format!("{label} resolve returned no address"),
            |error| format!("{label} connect failed: {error}"),
        )
    })?;

    let request = format!(
        "POST {} HTTP/1.1\r\nhost: {}:{}\r\ncontent-type: application/json; charset=utf-8\r\naccept: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        endpoint.path,
        endpoint.host,
        endpoint.port,
        body.len(),
        body
    );
    let mut unwritten = request.as_bytes();
    while !unwritten.is_empty() {
        if should_cancel.as_mut().is_some_and(|cancel| (*cancel)()) {
            let _ = stream.shutdown(Shutdown::Both);
            return Err(format!("{label} cancelled while writing"));
        }
        let remaining = remaining_model_pool_http_timeout(started, timeout, label, "write")?;
        stream
            .set_write_timeout(Some(remaining.min(MODEL_POOL_HTTP_CANCEL_POLL_INTERVAL)))
            .map_err(|error| format!("{label} write timeout setup failed: {error}"))?;
        match stream.write(unwritten) {
            Ok(0) => return Err(format!("{label} write returned zero bytes")),
            Ok(written) => unwritten = &unwritten[written..],
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => return Err(format!("{label} write failed: {error}")),
        }
    }

    let mut response = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        if should_cancel.as_mut().is_some_and(|cancel| (*cancel)()) {
            let _ = stream.write_all(MODEL_POOL_CALL_CANCEL_MARKER);
            let _ = stream.shutdown(Shutdown::Both);
            return Err(format!("{label} cancelled while reading"));
        }
        let remaining = remaining_model_pool_http_timeout(started, timeout, label, "read")?;
        stream
            .set_read_timeout(Some(remaining.min(MODEL_POOL_HTTP_CANCEL_POLL_INTERVAL)))
            .map_err(|error| format!("{label} read timeout setup failed: {error}"))?;
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => response.extend_from_slice(&buffer[..read]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => return Err(format!("{label} read failed: {error}")),
        }
    }
    Ok(response)
}

fn remaining_model_pool_http_timeout(
    started: Instant,
    timeout: Duration,
    label: &str,
    stage: &str,
) -> Result<Duration, String> {
    let remaining = timeout.saturating_sub(started.elapsed());
    if remaining.is_zero() {
        Err(format!(
            "{label} {stage} timed out after {}ms",
            timeout.as_millis()
        ))
    } else {
        Ok(remaining)
    }
}

fn model_pool_route_plan_request_body(prompt: &str, max_tokens: Option<usize>) -> String {
    let max_tokens = max_tokens
        .map(|value| format!(",\"max_tokens\":{}", value.max(1)))
        .unwrap_or_default();
    format!(
        "{{\"task_kind\":\"auto\",\"prompt\":{}{max_tokens}}}",
        service_json_string(prompt)
    )
}

fn model_pool_call_request_body(prompt: &str, max_tokens: Option<usize>, stream: bool) -> String {
    let max_tokens = max_tokens
        .map(|value| format!(",\"max_tokens\":{}", value.max(1)))
        .unwrap_or_default();
    format!(
        "{{\"task_kind\":\"auto\",\"prompt\":{},\"stream\":{stream}{max_tokens}}}",
        service_json_string(prompt)
    )
}

fn model_pool_http_body(response: &str, label: &str) -> Result<String, ModelPoolHttpError> {
    let (head, body) = split_http_head_body(response);
    let status_code = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| {
            ModelPoolHttpError::blocked(format!("{label} response missing HTTP status"))
        })?;
    if !(200..300).contains(&status_code) {
        return Err(ModelPoolHttpError::response(label, status_code, body));
    }
    Ok(body.to_owned())
}

impl ModelPoolHttpEndpoint {
    fn parse(url: &str, default_path: &str, label: &str) -> Result<Self, String> {
        let trimmed = url.trim().trim_end_matches('/');
        let without_scheme = trimmed.strip_prefix("http://").unwrap_or(trimmed);
        if without_scheme.starts_with("https://") {
            return Err(format!("{label} client only supports http://"));
        }
        let (authority, path) = without_scheme
            .split_once('/')
            .map(|(authority, path)| (authority, format!("/{path}")))
            .unwrap_or((without_scheme, default_path.to_owned()));
        let (host, port) = authority
            .rsplit_once(':')
            .ok_or_else(|| format!("{label} URL must include host:port"))?;
        let port = port
            .parse::<u16>()
            .map_err(|_| format!("{label} URL port must be a u16"))?;
        if host.is_empty() {
            return Err(format!("{label} URL host must not be empty"));
        }

        Ok(Self {
            host: host.to_owned(),
            port,
            path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_service::http::read_http_request;
    use std::sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    struct PanicBackend;

    impl InferenceBackend for PanicBackend {
        fn configure_generation(&mut self, _max_tokens: Option<usize>) {
            panic!("fallback backend should not be configured")
        }

        fn embed_text(&mut self, _text: &str) -> Option<Vec<f32>> {
            panic!("fallback backend should not receive embedding input")
        }

        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            panic!("fallback backend should not be called")
        }
    }

    #[derive(Default)]
    struct WaveFallbackBackend {
        prompts: Vec<String>,
    }

    impl InferenceBackend for WaveFallbackBackend {
        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            self.prompts.push(context.prompt.to_owned());
            InferenceDraft::new(
                "fallback-answer",
                vec![ReasoningStep::new(
                    "wave_fallback",
                    "model-pool wave used the serial fallback",
                    0.0,
                )],
            )
        }
    }

    #[derive(Default)]
    struct RecursiveWaveServerState {
        active_chunks: usize,
        peak_chunks: usize,
        chunk_started: usize,
        accepted_requests: usize,
        barrier_timed_out: bool,
        chunk_completions: Vec<usize>,
        merge_requests: Vec<String>,
    }

    struct RecursiveWaveServerControl {
        state: Mutex<RecursiveWaveServerState>,
        changed: Condvar,
        parallel_chunks: usize,
        total_chunks: usize,
    }

    struct RecursiveWaveServerReport {
        peak_chunks: usize,
        accepted_requests: usize,
        barrier_timed_out: bool,
        chunk_completions: Vec<usize>,
        merge_requests: Vec<String>,
    }

    #[derive(Clone, Copy)]
    enum RecursiveWaveResponseMode {
        Success,
        MixedFailures,
    }

    fn recursive_chunk_index(request: &str) -> Option<usize> {
        request
            .split_once("Noiron recursive chunk ")
            .and_then(|(_, suffix)| suffix.split_whitespace().next())
            .and_then(|value| value.parse::<usize>().ok())
    }

    fn write_model_pool_response(stream: &mut TcpStream, status: &str, body: &str) {
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    }

    fn write_model_pool_answer(stream: &mut TcpStream, answer: &str) {
        let body = format!("{{\"ok\":true,\"answer\":{}}}", service_json_string(answer));
        write_model_pool_response(stream, "200 OK", &body);
    }

    fn recursive_wave_model_pool_server(
        total_chunks: usize,
        parallel_chunks: usize,
        response_mode: RecursiveWaveResponseMode,
    ) -> (
        String,
        Arc<AtomicBool>,
        std::thread::JoinHandle<RecursiveWaveServerReport>,
    ) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let done = Arc::new(AtomicBool::new(false));
        let server_done = Arc::clone(&done);
        let control = Arc::new(RecursiveWaveServerControl {
            state: Mutex::new(RecursiveWaveServerState::default()),
            changed: Condvar::new(),
            parallel_chunks: parallel_chunks.max(1),
            total_chunks,
        });
        let server_control = Arc::clone(&control);
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut last_accept = Instant::now();
            let mut workers = Vec::new();
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        last_accept = Instant::now();
                        let worker_control = Arc::clone(&server_control);
                        workers.push(std::thread::spawn(move || {
                            let request = read_http_request(&mut stream).unwrap();
                            if let Some(chunk_index) = recursive_chunk_index(&request) {
                                let mut state = worker_control.state.lock().unwrap();
                                state.accepted_requests += 1;
                                state.active_chunks += 1;
                                state.chunk_started += 1;
                                state.peak_chunks = state.peak_chunks.max(state.active_chunks);
                                let wave_end = ((chunk_index / worker_control.parallel_chunks + 1)
                                    * worker_control.parallel_chunks)
                                    .min(worker_control.total_chunks);
                                worker_control.changed.notify_all();
                                let wait_deadline = Instant::now() + Duration::from_secs(1);
                                while state.chunk_started < wave_end {
                                    let remaining =
                                        wait_deadline.saturating_duration_since(Instant::now());
                                    if remaining.is_zero() {
                                        state.barrier_timed_out = true;
                                        break;
                                    }
                                    let (next, wait) = worker_control
                                        .changed
                                        .wait_timeout(state, remaining)
                                        .unwrap();
                                    state = next;
                                    if wait.timed_out() && state.chunk_started < wave_end {
                                        state.barrier_timed_out = true;
                                        break;
                                    }
                                }
                                drop(state);

                                let high_index = wave_end.saturating_sub(1);
                                if chunk_index != high_index {
                                    let wait_deadline = Instant::now() + Duration::from_secs(1);
                                    let mut state = worker_control.state.lock().unwrap();
                                    while !state.chunk_completions.contains(&high_index) {
                                        let remaining = wait_deadline
                                            .saturating_duration_since(Instant::now());
                                        if remaining.is_zero() {
                                            state.barrier_timed_out = true;
                                            break;
                                        }
                                        let (next, wait) = worker_control
                                            .changed
                                            .wait_timeout(state, remaining)
                                            .unwrap();
                                        state = next;
                                        if wait.timed_out()
                                            && !state.chunk_completions.contains(&high_index)
                                        {
                                            state.barrier_timed_out = true;
                                            break;
                                        }
                                    }
                                }

                                {
                                    let mut state = worker_control.state.lock().unwrap();
                                    state.active_chunks = state.active_chunks.saturating_sub(1);
                                }
                                match (response_mode, chunk_index) {
                                    (RecursiveWaveResponseMode::MixedFailures, 1) => {
                                        write_model_pool_response(
                                            &mut stream,
                                            "503 Unavailable",
                                            r#"{"ok":false,"error":"retryable chunk","retryable":true}"#,
                                        );
                                    }
                                    (RecursiveWaveResponseMode::MixedFailures, 2) => {
                                        write_model_pool_response(
                                            &mut stream,
                                            "409 Conflict",
                                            r#"{"ok":false,"error":"blocked chunk","retryable":false}"#,
                                        );
                                    }
                                    _ => write_model_pool_answer(
                                        &mut stream,
                                        &format!("chunk-answer-{chunk_index}"),
                                    ),
                                }
                                let mut state = worker_control.state.lock().unwrap();
                                state.chunk_completions.push(chunk_index);
                                worker_control.changed.notify_all();
                            } else {
                                let mut state = worker_control.state.lock().unwrap();
                                state.accepted_requests += 1;
                                state.merge_requests.push(request);
                                drop(state);
                                write_model_pool_answer(&mut stream, "merge-answer");
                            }
                        }));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if server_done.load(Ordering::SeqCst)
                            && last_accept.elapsed() >= Duration::from_millis(50)
                        {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("recursive wave server accept failed: {error}"),
                }
            }
            for worker in workers {
                worker.join().unwrap();
            }
            let state = server_control.state.lock().unwrap();
            RecursiveWaveServerReport {
                peak_chunks: state.peak_chunks,
                accepted_requests: state.accepted_requests,
                barrier_timed_out: state.barrier_timed_out,
                chunk_completions: state.chunk_completions.clone(),
                merge_requests: state.merge_requests.clone(),
            }
        });
        (format!("http://{addr}"), done, server)
    }

    fn recursive_wave_engine(second_gene_fitness: f32) -> NoironEngine {
        let mut engine = NoironEngine::new();
        engine.recursive_scheduler = rust_norion::RecursiveScheduler::new(45, 45, 10, 4);
        engine.set_hardware_snapshot(rust_norion::HardwareSnapshot::new(
            rust_norion::DeviceClass::DiscreteGpu,
            0.05,
            0.05,
            0.10,
            0.05,
        ));
        let genes = &mut engine
            .genome_runtime_state
            .profile_mut(TaskProfile::General)
            .active
            .genes;
        for gene in genes.iter_mut() {
            gene.fitness = 0.20;
        }
        genes[0].fitness = 0.95;
        genes[1].fitness = second_gene_fitness;
        engine
    }

    #[test]
    fn model_pool_recursive_wave_budget_one_stays_serial() {
        let (call_url, done, server) =
            recursive_wave_model_pool_server(4, 1, RecursiveWaveResponseMode::Success);
        let mut engine = recursive_wave_engine(0.20);
        let mut backend = WaveFallbackBackend::default();
        let timed = run_timed_inference_with_scope_and_route_plan_url_options(
            &mut engine,
            &mut backend,
            "benchmark DSpark paper throughput and verification scheduling".to_owned(),
            TaskProfile::General,
            None,
            None,
            None,
            None,
            None,
            Some(&call_url),
            None,
            None,
        )
        .unwrap();
        done.store(true, Ordering::SeqCst);
        let report = server.join().unwrap();

        assert!(backend.prompts.is_empty());
        assert_eq!(
            timed
                .outcome
                .reasoning_frame
                .routing_bias
                .confidence_prefix_selected,
            1
        );
        assert_eq!(timed.outcome.recursive_schedule.chunk_count(), 4);
        assert_eq!(timed.outcome.recursive_schedule.execution_wave_count(), 4);
        assert_eq!(timed.outcome.recursive_schedule.max_parallel_chunks, 1);
        assert_eq!(timed.outcome.recursive_runtime_calls, 5);
        assert_eq!(report.accepted_requests, 5);
        assert_eq!(report.peak_chunks, 1);
        assert!(!report.barrier_timed_out);
    }

    #[test]
    fn model_pool_recursive_wave_falls_back_only_for_retryable_results() {
        let (call_url, done, server) =
            recursive_wave_model_pool_server(4, 2, RecursiveWaveResponseMode::MixedFailures);
        let mut engine = recursive_wave_engine(0.80);
        let mut backend = WaveFallbackBackend::default();
        let timed = run_timed_inference_with_scope_and_route_plan_url_options(
            &mut engine,
            &mut backend,
            "benchmark DSpark paper throughput and verification scheduling".to_owned(),
            TaskProfile::General,
            None,
            None,
            None,
            None,
            None,
            Some(&call_url),
            None,
            None,
        )
        .unwrap();
        done.store(true, Ordering::SeqCst);
        let report = server.join().unwrap();

        assert_eq!(
            timed
                .outcome
                .reasoning_frame
                .routing_bias
                .confidence_prefix_selected,
            2
        );
        assert_eq!(timed.outcome.recursive_schedule.chunk_count(), 4);
        assert_eq!(timed.outcome.recursive_schedule.execution_wave_count(), 2);
        assert_eq!(timed.outcome.recursive_schedule.max_parallel_chunks, 2);
        assert_eq!(timed.outcome.recursive_runtime_calls, 5);
        assert_eq!(backend.prompts.len(), 1);
        assert!(backend.prompts[0].contains("Noiron recursive chunk 1"));
        assert_eq!(report.accepted_requests, 5);
        assert_eq!(report.peak_chunks, 2);
        assert!(!report.barrier_timed_out);
        assert_eq!(report.chunk_completions, vec![1, 0, 3, 2]);
        assert_eq!(report.merge_requests.len(), 1);
        let merge_request = &report.merge_requests[0];
        let mut prior_position = 0usize;
        for index in 0..4 {
            let marker = format!("chunk_{index}:");
            let position = merge_request.find(&marker).unwrap();
            assert!(position >= prior_position);
            prior_position = position;
        }
        assert!(merge_request.contains("chunk_1: fallback-answer"));
        assert!(merge_request.contains("chunk_2: Runtime backend error"));
        assert!(merge_request.contains("blocked chunk retryable=false"));
    }

    #[test]
    fn model_pool_cancellation_stops_before_the_next_recursive_wave() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let started = Arc::new(AtomicUsize::new(0));
        let server_started = Arc::clone(&started);
        let done = Arc::new(AtomicBool::new(false));
        let server_done = Arc::clone(&done);
        let server = std::thread::spawn(move || {
            let mut workers = Vec::new();
            let mut accepted = 0usize;
            let mut last_accept = Instant::now();
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        accepted += 1;
                        last_accept = Instant::now();
                        let worker_started = Arc::clone(&server_started);
                        workers.push(std::thread::spawn(move || {
                            let request = read_http_request(&mut stream).unwrap();
                            assert!(recursive_chunk_index(&request).is_some());
                            worker_started.fetch_add(1, Ordering::SeqCst);
                            stream
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .unwrap();
                            let mut buffer = [0_u8; 128];
                            loop {
                                match stream.read(&mut buffer) {
                                    Ok(0) => break,
                                    Ok(_) => {}
                                    Err(error)
                                        if matches!(
                                            error.kind(),
                                            std::io::ErrorKind::TimedOut
                                                | std::io::ErrorKind::WouldBlock
                                        ) =>
                                    {
                                        break;
                                    }
                                    Err(_) => break,
                                }
                            }
                        }));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if server_done.load(Ordering::SeqCst)
                            && last_accept.elapsed() >= Duration::from_millis(50)
                        {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("cancel wave server accept failed: {error}"),
                }
            }
            for worker in workers {
                worker.join().unwrap();
            }
            accepted
        });

        let mut engine = recursive_wave_engine(0.80);
        let mut backend = PanicBackend;
        let mut should_cancel = || started.load(Ordering::SeqCst) >= 2;
        let timed = run_timed_inference_with_scope_and_route_plan_url_options(
            &mut engine,
            &mut backend,
            "benchmark DSpark paper throughput and verification scheduling".to_owned(),
            TaskProfile::General,
            None,
            None,
            None,
            None,
            None,
            Some(&format!("http://{addr}")),
            None,
            Some(&mut should_cancel),
        )
        .unwrap();

        done.store(true, Ordering::SeqCst);
        assert_eq!(server.join().unwrap(), 2);
        assert_eq!(timed.outcome.recursive_runtime_calls, 2);
        assert_eq!(
            timed.outcome.raw_answer,
            "Runtime backend error: generation cancelled"
        );
    }

    #[test]
    fn model_pool_pre_cancel_records_one_attempt_without_dispatching_a_wave() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let mut engine = recursive_wave_engine(0.80);
        let mut backend = PanicBackend;
        let mut should_cancel = || true;

        let timed = run_timed_inference_with_scope_and_route_plan_url_options(
            &mut engine,
            &mut backend,
            "benchmark DSpark paper throughput and verification scheduling".to_owned(),
            TaskProfile::General,
            None,
            None,
            None,
            None,
            None,
            Some(&format!("http://{addr}")),
            None,
            Some(&mut should_cancel),
        )
        .unwrap();

        assert_eq!(timed.outcome.recursive_runtime_calls, 1);
        assert_eq!(
            timed.outcome.raw_answer,
            "Runtime backend error: generation cancelled"
        );
        assert!(matches!(
            listener.accept(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
        ));
    }

    fn stalled_model_pool_call_server() -> (String, Arc<AtomicBool>, std::thread::JoinHandle<bool>)
    {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let request_seen = Arc::new(AtomicBool::new(false));
        let server_request_seen = Arc::clone(&request_seen);
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut buffer = [0_u8; 8192];
            let read = stream.read(&mut buffer).unwrap();
            assert!(
                String::from_utf8_lossy(&buffer[..read])
                    .starts_with("POST /v1/model-pool/call HTTP/1.1")
            );
            server_request_seen.store(true, Ordering::SeqCst);
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) => return true,
                    Ok(_) => {}
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                        ) =>
                    {
                        return false;
                    }
                    Err(_) => return false,
                }
            }
        });
        (format!("http://{addr}"), request_seen, server)
    }

    fn blocked_model_pool_call_server(
        retry_after_invalid_rust: bool,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let request_count = if retry_after_invalid_rust { 2 } else { 1 };
            for request_index in 0..request_count {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0_u8; 8192];
                let read = stream.read(&mut buffer).unwrap();
                assert!(
                    String::from_utf8_lossy(&buffer[..read])
                        .starts_with("POST /v1/model-pool/call HTTP/1.1")
                );
                let (status, body) = if retry_after_invalid_rust && request_index == 0 {
                    (
                        "200 OK",
                        format!(
                            "{{\"ok\":true,\"answer\":{}}}",
                            service_json_string("```rust\nfn broken(\n```")
                        ),
                    )
                } else {
                    let error = "dependency blocked\n```rust\nfn broken(\n```";
                    (
                        "409 Conflict",
                        format!(
                            "{{\"ok\":false,\"error\":{},\"retryable\":false,\"dispatch_attempted\":false,\"sends_prompt\":false,\"persistent_writes\":false}}",
                            service_json_string(error)
                        ),
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (format!("http://{addr}"), server)
    }

    struct DnaFeedbackBackend;

    impl InferenceBackend for DnaFeedbackBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                "Rust DNA routing needs bounded reflection and rollback validation.",
                vec![ReasoningStep::new(
                    "dna_feedback",
                    "trusted promotion feedback",
                    0.2,
                )],
            )
        }
    }

    #[test]
    fn timed_inference_runner_transports_trusted_genome_authorization() {
        let mut engine = NoironEngine::new();
        let mut backend = DnaFeedbackBackend;
        let preflight = rust_norion::SelfEvolutionPromotionPreflightReport {
            decision:
                rust_norion::SelfEvolutionPromotionPreflightDecision::ReadyForExplicitPromotion,
            ready_for_explicit_promotion: true,
            explicit_promotion_required: true,
            candidate_id: "candidate:runner-trusted-authorization".to_owned(),
            admission_admitted_for_human_review: true,
            experiment_admitted_for_human_review: true,
            operator_approved: true,
            rust_validation_passed: true,
            validation_passed: true,
            benchmark_gate_passed: true,
            adaptive_preview_evidence_present: true,
            review_packet_count: 1,
            evidence_id_count: 1,
            rollback_anchor_count: 1,
            content_digest_count: 1,
            source_report_schema_count: 1,
            read_only: true,
            report_only: true,
            preview_only: true,
            activation_write_allowed: false,
            active_candidate: false,
            write_allowed: false,
            applied: false,
            blocked_reasons: Vec::new(),
            content_digest: "fnv64:runner-trusted-authorization".to_owned(),
        };
        let authorization = GenomeEvolutionAuthorization::from_promotion_preflight(
            &preflight,
            rust_norion::TaskSkillGeneEvidence::passing(),
            false,
        )
        .unwrap();

        let timed = run_timed_inference_with_scope_options_and_genome_authorization(
            &mut engine,
            &mut backend,
            "Rust DNA routing feedback".to_owned(),
            TaskProfile::Coding,
            None,
            None,
            None,
            None,
            Some(authorization),
        )
        .unwrap();

        assert!(timed.outcome.dna_apply_receipt.applied);
        assert!(timed.outcome.task_skill_gene.activation_eligible);
    }

    #[test]
    fn inference_request_options_preserve_tenant_scope() {
        let scope = TenantScope::new("tenant-a", "workspace", "session");
        let request = inference_request_with_options(
            "hello".to_owned(),
            TaskProfile::Coding,
            Some(0),
            Some(scope.clone()),
        );

        assert_eq!(request.max_tokens, Some(1));
        assert_eq!(request.tenant_scope, Some(scope));
    }

    #[test]
    fn inference_request_options_default_to_local_single_user_scope() {
        let request =
            inference_request_with_options("hello".to_owned(), TaskProfile::Coding, None, None);

        assert_eq!(request.tenant_scope, Some(TenantScope::local_single_user()));
    }

    #[test]
    fn inference_request_options_fetch_route_plan_proof_when_url_is_set() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let read = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]);
            assert!(request.contains("POST /v1/model-pool/route-plan HTTP/1.1"));
            assert!(request.contains("\"task_kind\":\"auto\""));
            assert!(request.contains("\"prompt\":\"agent team route\""));
            assert!(request.contains("\"max_tokens\":32"));

            let body = r#"{"ok":true,"read_only":true,"launches_process":false,"sends_prompt":false,"route_allowed":true,"reason":"ready","selected_role":"review","agent_model_route_source":{"route_allowed":true,"proof_ready":true,"selected_role":"review","model_registry_id":"registry.review","model_profile_id":"profile.review","inference_backend_id":"backend.review","model_pool_id":"pool.main"}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let request = inference_request_with_options_and_route_plan_url(
            "agent team route".to_owned(),
            TaskProfile::Coding,
            Some(32),
            None,
            Some(&format!("http://{addr}")),
        );

        assert_eq!(
            request
                .agent_team_route_proof
                .as_ref()
                .and_then(|proof| proof.selected_role.as_deref()),
            Some("review")
        );
        server.join().unwrap();
    }

    #[test]
    fn inference_cancellation_interrupts_model_pool_response_wait() {
        let (call_url, request_seen, server) = stalled_model_pool_call_server();
        let mut engine = NoironEngine::new();
        let mut backend = PanicBackend;
        let mut should_cancel = || request_seen.load(Ordering::SeqCst);

        let timed = run_timed_inference_with_scope_and_route_plan_url_options(
            &mut engine,
            &mut backend,
            "cancel model-pool call".to_owned(),
            TaskProfile::Coding,
            Some(12),
            None,
            None,
            None,
            None,
            Some(&call_url),
            None,
            Some(&mut should_cancel),
        )
        .unwrap();

        assert!(timed.elapsed_ms < 1_500);
        assert_eq!(
            timed.outcome.raw_answer,
            "Runtime backend error: generation cancelled"
        );
        assert!(
            server.join().unwrap(),
            "client did not close after cancellation"
        );
    }

    #[test]
    fn stream_cancellation_interrupts_model_pool_response_wait_without_tokens() {
        let (call_url, request_seen, server) = stalled_model_pool_call_server();
        let mut engine = NoironEngine::new();
        let mut backend = PanicBackend;
        let mut tokens = Vec::new();
        let mut on_token = |token: &DraftToken| {
            tokens.push(token.text.clone());
            Ok(())
        };
        let mut should_cancel = || request_seen.load(Ordering::SeqCst);

        let timed = run_timed_inference_stream_checked_with_scope_and_call_url_options(
            &mut engine,
            &mut backend,
            "cancel streaming model-pool call".to_owned(),
            TaskProfile::Coding,
            Some(12),
            None,
            None,
            None,
            &mut on_token,
            Some(&call_url),
            None,
            Some(&mut should_cancel),
        )
        .unwrap();

        assert!(timed.elapsed_ms < 1_500);
        assert!(tokens.is_empty());
        assert_eq!(
            timed.outcome.raw_answer,
            "Runtime backend error: generation cancelled"
        );
        assert!(
            server.join().unwrap(),
            "client did not close after cancellation"
        );
    }

    #[test]
    fn non_retryable_model_pool_block_does_not_call_fallback() {
        let (call_url, server) = blocked_model_pool_call_server(false);
        let mut engine = NoironEngine::new();
        let mut seed_backend = DnaFeedbackBackend;
        engine.infer(
            InferenceRequest::new("seed replay state", TaskProfile::Coding),
            &mut seed_backend,
        );
        let mut backend = PanicBackend;
        let router_before = engine.router.state();
        let hierarchy_before = engine.hierarchy.state();
        let evolution_before = engine.evolution_ledger;
        let memory_count_before = engine.cache.entries().len();
        let experience_count_before = engine.experience.records().len();
        assert!(experience_count_before > 0);

        let timed = run_timed_inference_with_scope_and_route_plan_url_options(
            &mut engine,
            &mut backend,
            "preserve non-retryable model-pool block".to_owned(),
            TaskProfile::Coding,
            Some(12),
            None,
            None,
            None,
            None,
            Some(&call_url),
            None,
            None,
        )
        .unwrap();

        assert!(timed.outcome.raw_answer.contains("dependency blocked"));
        assert!(timed.outcome.raw_answer.contains("retryable=false"));
        assert!(timed.outcome.auto_replay_report.is_none());
        assert!(timed.outcome.stored_memory_id.is_none());
        assert!(timed.outcome.stored_gist_memory_ids.is_empty());
        assert!(timed.outcome.stored_runtime_kv_memory_ids.is_empty());
        assert_eq!(engine.cache.entries().len(), memory_count_before);
        assert_eq!(engine.experience.records().len(), experience_count_before);
        assert_eq!(engine.evolution_ledger, evolution_before);
        let router_after = engine.router.state();
        assert_eq!(router_after.threshold, router_before.threshold);
        assert_eq!(router_after.observations, router_before.observations);
        assert_eq!(
            router_after.profile_thresholds.coding,
            router_before.profile_thresholds.coding
        );
        assert_eq!(
            router_after.profile_observations.coding,
            router_before.profile_observations.coding
        );
        let hierarchy_after = engine.hierarchy.state();
        assert_eq!(hierarchy_after.current, hierarchy_before.current);
        assert_eq!(
            hierarchy_after.profile_weights.coding,
            hierarchy_before.profile_weights.coding
        );
        assert_eq!(
            hierarchy_after.profile_observations.coding,
            hierarchy_before.profile_observations.coding
        );
        assert!(
            timed
                .outcome
                .process_reward
                .notes
                .iter()
                .any(|note| note.contains("runtime_model_pool_call_blocked_error"))
        );
        server.join().unwrap();
    }

    #[test]
    fn streaming_non_retryable_model_pool_block_emits_no_tokens_or_fallback() {
        let (call_url, server) = blocked_model_pool_call_server(false);
        let mut engine = NoironEngine::new();
        let mut backend = PanicBackend;
        let mut tokens = Vec::new();
        let mut on_token = |token: &DraftToken| {
            tokens.push(token.text.clone());
            Ok(())
        };

        let timed = run_timed_inference_stream_checked_with_scope_and_call_url_options(
            &mut engine,
            &mut backend,
            "stream preserve non-retryable model-pool block".to_owned(),
            TaskProfile::Coding,
            Some(12),
            None,
            None,
            None,
            &mut on_token,
            Some(&call_url),
            None,
            None,
        )
        .unwrap();

        assert!(tokens.is_empty());
        assert!(timed.outcome.raw_answer.contains("dependency blocked"));
        assert!(timed.outcome.raw_answer.contains("retryable=false"));
        server.join().unwrap();
    }

    #[test]
    fn rust_validation_retry_preserves_non_retryable_model_pool_block() {
        let (call_url, server) = blocked_model_pool_call_server(true);
        let mut engine = NoironEngine::new();
        let mut seed_backend = DnaFeedbackBackend;
        engine.infer(
            InferenceRequest::new("seed retry block state", TaskProfile::Coding),
            &mut seed_backend,
        );
        let experience_count_before = engine.experience.records().len();
        let evolution_before = engine.evolution_ledger;
        let router_before = engine.router.state();
        let hierarchy_before = engine.hierarchy.state();
        let memory_count_before = engine.cache.entries().len();
        let mut backend = PanicBackend;

        let timed = run_timed_inference_with_scope_and_route_plan_url_options(
            &mut engine,
            &mut backend,
            "retry invalid Rust through model pool".to_owned(),
            TaskProfile::Coding,
            Some(32),
            None,
            None,
            None,
            None,
            Some(&call_url),
            None,
            None,
        )
        .unwrap();

        assert!(timed.outcome.raw_answer.contains("dependency blocked"));
        assert!(timed.outcome.auto_replay_report.is_none());
        assert_eq!(engine.experience.records().len(), experience_count_before);
        assert_eq!(engine.evolution_ledger, evolution_before);
        assert_eq!(
            engine.router.state().observations,
            router_before.observations
        );
        assert_eq!(
            engine.hierarchy.state().profile_weights.coding,
            hierarchy_before.profile_weights.coding
        );
        assert_eq!(engine.cache.entries().len(), memory_count_before);
        server.join().unwrap();
    }

    #[test]
    fn stream_uses_model_pool_call_answer_when_call_url_is_set() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let read = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]);
            assert!(request.contains("POST /v1/model-pool/call HTTP/1.1"));
            assert!(request.contains("\"task_kind\":\"auto\""));
            assert!(request.contains("[noiron-dna"));
            assert!(request.contains("stream through model pool"));
            assert!(request.contains("\"stream\":true"));
            assert!(request.contains("\"max_tokens\":12"));

            let body = r#"{"ok":true,"answer":"stream model-pool answer","worker_streamed_tokens":["stream ","model-pool ","answer"]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let mut engine = NoironEngine::new();
        let mut seed_backend = DnaFeedbackBackend;
        engine.infer(
            InferenceRequest::new("seed deferred replay", TaskProfile::Coding),
            &mut seed_backend,
        );
        let mut backend = PanicBackend;
        let mut tokens = Vec::new();
        let mut on_token = |token: &DraftToken| {
            tokens.push(token.text.clone());
            Ok(())
        };

        let timed = run_timed_inference_stream_checked_with_model_pool_call_url(
            &mut engine,
            &mut backend,
            "stream through model pool".to_owned(),
            TaskProfile::Coding,
            Some(12),
            None,
            None,
            &mut on_token,
            &format!("http://{addr}"),
        )
        .unwrap();

        assert_eq!(tokens, vec!["stream ", "model-pool ", "answer"]);
        assert_eq!(timed.outcome.raw_answer, "stream model-pool answer");
        assert!(timed.outcome.auto_replay_report.is_some());
        server.join().unwrap();
    }

    #[test]
    fn model_pool_http_errors_require_explicit_retryable_response() {
        let blocked = model_pool_http_body(
            "HTTP/1.1 409 Conflict\r\n\r\n{\"retryable\":false}",
            "model pool call",
        )
        .unwrap_err();
        let retryable = model_pool_http_body(
            "HTTP/1.1 503 Unavailable\r\n\r\n{\"retryable\":true}",
            "model pool call",
        )
        .unwrap_err();
        let unspecified = model_pool_http_body(
            "HTTP/1.1 503 Unavailable\r\n\r\n{\"error\":\"unavailable\"}",
            "model pool call",
        )
        .unwrap_err();

        assert!(!blocked.retryable);
        assert!(retryable.retryable);
        assert!(!unspecified.retryable);
    }

    #[test]
    fn route_plan_endpoint_parse_uses_default_path() {
        let endpoint = ModelPoolHttpEndpoint::parse(
            "127.0.0.1:7878",
            MODEL_POOL_ROUTE_PLAN_DEFAULT_PATH,
            "model pool route-plan",
        )
        .unwrap();

        assert_eq!(endpoint.host, "127.0.0.1");
        assert_eq!(endpoint.port, 7878);
        assert_eq!(endpoint.path, "/v1/model-pool/route-plan");
    }
}
