use std::path::PathBuf;
use std::time::Instant;

use rust_norion::{
    DraftToken, InferenceBackend, InferenceRequest, NoironEngine, TaskProfile, append_trace_jsonl,
    append_trace_jsonl_with_case,
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
    let started = Instant::now();
    let request = InferenceRequest::new(prompt.clone(), profile).with_max_tokens(max_tokens);
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
    let started = Instant::now();
    let request = InferenceRequest::new(prompt.clone(), profile).with_max_tokens(max_tokens);
    let outcome = engine.infer_stream(request, backend, on_token);
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
