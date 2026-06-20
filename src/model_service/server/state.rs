use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crate::model_service::types::TimedOutcome;

#[derive(Default)]
pub(super) struct ModelServiceServerState {
    active_engine_requests: AtomicUsize,
    active_requests: Mutex<Vec<ModelServiceActiveRequestTelemetry>>,
    last_inference: Mutex<Option<ModelServiceLastInferenceTelemetry>>,
}

#[derive(Debug, Clone)]
pub(super) struct ModelServiceActiveRequestTelemetry {
    pub(super) request_id: usize,
    pub(super) endpoint: String,
    pub(super) prompt_preview: String,
    started: Instant,
}

impl ModelServiceActiveRequestTelemetry {
    fn new(request_id: usize, endpoint: impl Into<String>, prompt: &str) -> Self {
        Self {
            request_id,
            endpoint: endpoint.into(),
            prompt_preview: prompt_preview(prompt, 160),
            started: Instant::now(),
        }
    }

    pub(super) fn elapsed_ms(&self) -> u128 {
        self.started.elapsed().as_millis()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ModelServiceLastInferenceTelemetry {
    pub(super) request_id: usize,
    pub(super) endpoint: String,
    pub(super) elapsed_ms: u128,
    pub(super) runtime_model: Option<String>,
    pub(super) runtime_token_count: usize,
    pub(super) quality: f32,
    pub(super) process_reward: f32,
    pub(super) action: String,
    pub(super) error: Option<String>,
}

impl ModelServiceLastInferenceTelemetry {
    pub(super) fn from_timed(
        request_id: usize,
        endpoint: impl Into<String>,
        timed: &TimedOutcome,
    ) -> Self {
        Self {
            request_id,
            endpoint: endpoint.into(),
            elapsed_ms: timed.elapsed_ms,
            runtime_model: timed.outcome.runtime_diagnostics.model_id.clone(),
            runtime_token_count: timed.outcome.runtime_token_metrics.token_count,
            quality: timed.outcome.report.quality,
            process_reward: timed.outcome.process_reward.total,
            action: timed.outcome.process_reward.action.as_str().to_owned(),
            error: None,
        }
    }

    pub(super) fn error(
        request_id: usize,
        endpoint: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            request_id,
            endpoint: endpoint.into(),
            elapsed_ms: 0,
            runtime_model: None,
            runtime_token_count: 0,
            quality: 0.0,
            process_reward: 0.0,
            action: "error".to_owned(),
            error: Some(error.into()),
        }
    }
}

impl ModelServiceServerState {
    pub(super) fn begin_engine_request(
        &self,
        request_id: usize,
        endpoint: impl Into<String>,
        prompt: &str,
    ) -> ModelServiceEngineRequestGuard<'_> {
        let endpoint = endpoint.into();
        self.active_engine_requests.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut active_requests) = self.active_requests.lock() {
            active_requests.push(ModelServiceActiveRequestTelemetry::new(
                request_id,
                endpoint.clone(),
                prompt,
            ));
        }
        ModelServiceEngineRequestGuard {
            state: self,
            request_id,
            endpoint,
        }
    }

    pub(super) fn active_engine_requests(&self) -> usize {
        self.active_engine_requests.load(Ordering::SeqCst)
    }

    pub(super) fn active_requests(&self) -> Vec<ModelServiceActiveRequestTelemetry> {
        self.active_requests
            .lock()
            .map(|active_requests| active_requests.clone())
            .unwrap_or_default()
    }

    pub(super) fn record_inference(&self, telemetry: ModelServiceLastInferenceTelemetry) {
        if let Ok(mut last_inference) = self.last_inference.lock() {
            *last_inference = Some(telemetry);
        }
    }

    pub(super) fn last_inference(&self) -> Option<ModelServiceLastInferenceTelemetry> {
        self.last_inference
            .lock()
            .ok()
            .and_then(|last_inference| last_inference.clone())
    }

    fn finish_engine_request(&self, request_id: usize, endpoint: &str) {
        self.active_engine_requests.fetch_sub(1, Ordering::SeqCst);
        if let Ok(mut active_requests) = self.active_requests.lock()
            && let Some(index) = active_requests.iter().position(|request| {
                request.request_id == request_id && request.endpoint == endpoint
            })
        {
            active_requests.remove(index);
        }
    }
}

pub(super) struct ModelServiceEngineRequestGuard<'a> {
    state: &'a ModelServiceServerState,
    request_id: usize,
    endpoint: String,
}

impl Drop for ModelServiceEngineRequestGuard<'_> {
    fn drop(&mut self) {
        self.state
            .finish_engine_request(self.request_id, &self.endpoint);
    }
}

fn prompt_preview(prompt: &str, max_chars: usize) -> String {
    let normalized = prompt
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let text = if normalized.is_empty() {
        prompt.trim()
    } else {
        normalized.as_str()
    };
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let keep_chars = max_chars.saturating_sub(3);
    let mut preview = text.chars().take(keep_chars).collect::<String>();
    preview.push_str("...");
    preview
}
