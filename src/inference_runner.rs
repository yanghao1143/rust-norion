use std::path::PathBuf;
use std::time::Instant;

use rust_norion::{
    DraftToken, InferenceBackend, InferenceRequest, NoironEngine, RuntimeError, TaskProfile,
    TenantScope, append_trace_jsonl, append_trace_jsonl_with_case,
};

use crate::model_service::types::TimedOutcome;

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
    let started = Instant::now();
    let request = inference_request_with_options(prompt.clone(), profile, max_tokens, tenant_scope);
    let outcome = engine.infer(request, backend);
    let elapsed_ms = started.elapsed().as_millis();

    if let Some(trace_path) = trace_path {
        if let Some(case_name) = case_name {
            append_trace_jsonl_with_case(
                trace_path, case_name, &prompt, profile, elapsed_ms, &outcome,
            )?;
        } else {
            append_trace_jsonl(trace_path, &prompt, profile, elapsed_ms, &outcome)?;
        }
    }

    Ok(TimedOutcome {
        outcome,
        elapsed_ms,
    })
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
    let started = Instant::now();
    let request = inference_request_with_options(prompt.clone(), profile, max_tokens, tenant_scope);
    let mut observer_error = None;
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
        engine.infer_stream_checked(request, backend, &mut checked)
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
    let request = InferenceRequest::new(prompt, profile).with_max_tokens(max_tokens);
    match tenant_scope {
        Some(scope) => request.with_tenant_scope(scope),
        None => request,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
