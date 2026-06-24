#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::http;
use crate::json::{json_string, json_string_field, json_u64_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InferenceRequest {
    pub(crate) prompt: String,
    pub(crate) system_prompt: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) max_tokens: usize,
    pub(crate) temperature_milli: u16,
}

impl InferenceRequest {
    pub(crate) fn new(prompt: impl Into<String>, max_tokens: usize) -> Self {
        Self {
            prompt: prompt.into(),
            system_prompt: None,
            model: None,
            max_tokens,
            temperature_milli: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackendCapabilities {
    pub(crate) ctx_window: usize,
    pub(crate) streaming: bool,
    pub(crate) cancel: bool,
    pub(crate) kv_export: bool,
    pub(crate) local: bool,
    pub(crate) openai_compatible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Usage {
    pub(crate) prompt_tokens: usize,
    pub(crate) completion_tokens: usize,
    pub(crate) total_tokens: usize,
    pub(crate) elapsed_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StreamChunk {
    Token { text: String, token_index: usize },
    Final { usage: Usage },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackendErrorKind {
    Canceled,
    InvalidRequest,
    Unavailable,
    Protocol,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackendError {
    pub(crate) kind: BackendErrorKind,
    pub(crate) message: String,
}

impl BackendError {
    fn new(kind: BackendErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn canceled() -> Self {
        Self::new(BackendErrorKind::Canceled, "inference request canceled")
    }
}

pub(crate) type BackendResult<T> = Result<T, BackendError>;
pub(crate) type InferenceStream = Box<dyn Iterator<Item = BackendResult<StreamChunk>> + Send>;

#[derive(Debug, Clone, Default)]
pub(crate) struct CancellationToken {
    canceled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn cancel(&self) {
        self.canceled.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_canceled(&self) -> bool {
        self.canceled.load(Ordering::SeqCst)
    }
}

pub(crate) trait InferenceBackend: Send + Sync {
    fn id(&self) -> &str;
    fn capabilities(&self) -> BackendCapabilities;
    fn stream(
        &self,
        request: InferenceRequest,
        cancellation: CancellationToken,
    ) -> BackendResult<InferenceStream>;
}

#[derive(Debug, Clone)]
pub(crate) struct DeterministicBackend {
    id: String,
    response_prefix: String,
    ctx_window: usize,
}

impl DeterministicBackend {
    pub(crate) fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            response_prefix: "deterministic".to_owned(),
            ctx_window: 8192,
        }
    }

    pub(crate) fn with_response_prefix(mut self, response_prefix: impl Into<String>) -> Self {
        self.response_prefix = response_prefix.into();
        self
    }
}

impl InferenceBackend for DeterministicBackend {
    fn id(&self) -> &str {
        &self.id
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            ctx_window: self.ctx_window,
            streaming: true,
            cancel: true,
            kv_export: false,
            local: true,
            openai_compatible: false,
        }
    }

    fn stream(
        &self,
        request: InferenceRequest,
        cancellation: CancellationToken,
    ) -> BackendResult<InferenceStream> {
        if request.prompt.trim().is_empty() {
            return Err(BackendError::new(
                BackendErrorKind::InvalidRequest,
                "prompt must not be empty",
            ));
        }
        let started = Instant::now();
        let prompt_tokens = count_tokens(&request.prompt)
            + request
                .system_prompt
                .as_deref()
                .map(count_tokens)
                .unwrap_or(0);
        let model = request.model.as_deref().unwrap_or(self.id());
        let text = format!(
            "{}:{}:{}",
            self.response_prefix,
            model,
            request.prompt.trim()
        );
        let tokens = text
            .split_whitespace()
            .take(request.max_tokens)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        Ok(Box::new(DeterministicStream {
            tokens,
            next_index: 0,
            prompt_tokens,
            started,
            cancellation,
            emitted_final: false,
        }))
    }
}

struct DeterministicStream {
    tokens: Vec<String>,
    next_index: usize,
    prompt_tokens: usize,
    started: Instant,
    cancellation: CancellationToken,
    emitted_final: bool,
}

impl Iterator for DeterministicStream {
    type Item = BackendResult<StreamChunk>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cancellation.is_canceled() {
            return Some(Err(BackendError::canceled()));
        }
        if let Some(token) = self.tokens.get(self.next_index) {
            let token_index = self.next_index;
            self.next_index += 1;
            return Some(Ok(StreamChunk::Token {
                text: token.clone(),
                token_index,
            }));
        }
        if self.emitted_final {
            return None;
        }
        self.emitted_final = true;
        let completion_tokens = self.next_index;
        Some(Ok(StreamChunk::Final {
            usage: Usage {
                prompt_tokens: self.prompt_tokens,
                completion_tokens,
                total_tokens: self.prompt_tokens + completion_tokens,
                elapsed_ms: self.started.elapsed().as_millis(),
            },
        }))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OpenAiCompatibleBackend {
    id: String,
    backend_addr: String,
    path: String,
    model: String,
    timeout_secs: u64,
    ctx_window: usize,
    local: bool,
}

impl OpenAiCompatibleBackend {
    pub(crate) fn local(
        id: impl Into<String>,
        backend_addr: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            backend_addr: backend_addr.into(),
            path: "/v1/chat/completions".to_owned(),
            model: model.into(),
            timeout_secs: 120,
            ctx_window: 8192,
            local: true,
        }
    }

    pub(crate) fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub(crate) fn with_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }
}

impl InferenceBackend for OpenAiCompatibleBackend {
    fn id(&self) -> &str {
        &self.id
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            ctx_window: self.ctx_window,
            streaming: true,
            cancel: true,
            kv_export: false,
            local: self.local,
            openai_compatible: true,
        }
    }

    fn stream(
        &self,
        request: InferenceRequest,
        cancellation: CancellationToken,
    ) -> BackendResult<InferenceStream> {
        if request.prompt.trim().is_empty() {
            return Err(BackendError::new(
                BackendErrorKind::InvalidRequest,
                "prompt must not be empty",
            ));
        }
        if cancellation.is_canceled() {
            return Err(BackendError::canceled());
        }

        let started = Instant::now();
        let prompt_tokens = count_tokens(&request.prompt)
            + request
                .system_prompt
                .as_deref()
                .map(count_tokens)
                .unwrap_or(0);
        let body = openai_chat_body(&self.model, &request);
        let mut chunks = Vec::new();
        let mut completion_tokens = 0usize;
        let mut usage = None;
        http::post_event_stream(
            &self.backend_addr,
            &self.path,
            &body,
            self.timeout_secs,
            &mut |event, data| {
                if cancellation.is_canceled() {
                    return Err("inference request canceled".to_owned());
                }
                match event {
                    "error" => Err(format!("backend returned error event: {data}")),
                    "done" => {
                        usage = Some(usage_from_event(
                            data,
                            prompt_tokens,
                            completion_tokens,
                            started,
                        ));
                        Ok(())
                    }
                    _ if data.trim() == "[DONE]" => {
                        usage = Some(usage_from_event(
                            data,
                            prompt_tokens,
                            completion_tokens,
                            started,
                        ));
                        Ok(())
                    }
                    _ => {
                        if let Some(text) = openai_delta_text(data) {
                            chunks.push(Ok(StreamChunk::Token {
                                text,
                                token_index: completion_tokens,
                            }));
                            completion_tokens += 1;
                        }
                        if data.contains("\"usage\"") {
                            usage = Some(usage_from_event(
                                data,
                                prompt_tokens,
                                completion_tokens,
                                started,
                            ));
                        }
                        Ok(())
                    }
                }
            },
        )
        .map_err(|message| classify_http_error(&message))?;

        chunks.push(Ok(StreamChunk::Final {
            usage: usage.unwrap_or_else(|| Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
                elapsed_ms: started.elapsed().as_millis(),
            }),
        }));
        Ok(Box::new(chunks.into_iter()))
    }
}

fn openai_chat_body(model: &str, request: &InferenceRequest) -> String {
    let mut messages = Vec::new();
    if let Some(system_prompt) = request.system_prompt.as_deref() {
        messages.push(format!(
            "{{\"role\":\"system\",\"content\":{}}}",
            json_string(system_prompt)
        ));
    }
    messages.push(format!(
        "{{\"role\":\"user\",\"content\":{}}}",
        json_string(&request.prompt)
    ));
    format!(
        "{{\"model\":{},\"messages\":[{}],\"max_tokens\":{},\"temperature\":{},\"stream\":true,\"stream_options\":{{\"include_usage\":true}}}}",
        json_string(request.model.as_deref().unwrap_or(model)),
        messages.join(","),
        request.max_tokens,
        f64::from(request.temperature_milli) / 1000.0
    )
}

fn openai_delta_text(data: &str) -> Option<String> {
    json_string_field(data, "content")
        .or_else(|| json_string_field(data, "text"))
        .filter(|text| !text.is_empty())
}

fn usage_from_event(
    data: &str,
    fallback_prompt_tokens: usize,
    fallback_completion_tokens: usize,
    started: Instant,
) -> Usage {
    let prompt_tokens = json_u64_field(data, "prompt_tokens")
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(fallback_prompt_tokens);
    let completion_tokens = json_u64_field(data, "completion_tokens")
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(fallback_completion_tokens);
    let total_tokens = json_u64_field(data, "total_tokens")
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(prompt_tokens + completion_tokens);
    Usage {
        prompt_tokens,
        completion_tokens,
        total_tokens,
        elapsed_ms: started.elapsed().as_millis(),
    }
}

fn classify_http_error(message: &str) -> BackendError {
    let lower = message.to_ascii_lowercase();
    let kind = if lower.contains("canceled") {
        BackendErrorKind::Canceled
    } else if lower.contains("timed out") || lower.contains("timeout") {
        BackendErrorKind::Timeout
    } else if lower.contains("connect") {
        BackendErrorKind::Unavailable
    } else {
        BackendErrorKind::Protocol
    };
    BackendError::new(kind, message)
}

fn count_tokens(text: &str) -> usize {
    text.split_whitespace()
        .filter(|token| !token.is_empty())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_chunks(backend: &dyn InferenceBackend, prompt: &str) -> Vec<StreamChunk> {
        backend
            .stream(InferenceRequest::new(prompt, 16), CancellationToken::new())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn deterministic_backend_streams_tokens_and_final_usage() {
        let backend = DeterministicBackend::new("det-ref");
        let chunks = collect_chunks(&backend, "summarize this");

        assert!(matches!(
            chunks.first(),
            Some(StreamChunk::Token { token_index: 0, .. })
        ));
        let usage = match chunks.last().unwrap() {
            StreamChunk::Final { usage } => usage,
            other => panic!("expected final usage chunk, got {other:?}"),
        };
        assert_eq!(usage.prompt_tokens, 2);
        assert_eq!(
            usage.total_tokens,
            usage.prompt_tokens + usage.completion_tokens
        );
    }

    #[test]
    fn deterministic_backend_honors_cancellation_between_tokens() {
        let backend = DeterministicBackend::new("det-ref").with_response_prefix("one two three");
        let cancellation = CancellationToken::new();
        let mut stream = backend
            .stream(InferenceRequest::new("input", 16), cancellation.clone())
            .unwrap();

        assert!(matches!(stream.next(), Some(Ok(StreamChunk::Token { .. }))));
        cancellation.cancel();
        let error = stream.next().unwrap().unwrap_err();

        assert_eq!(error.kind, BackendErrorKind::Canceled);
    }

    #[test]
    fn same_control_path_can_call_multiple_backend_implementations() {
        let deterministic = DeterministicBackend::new("det-ref");
        let openai = OpenAiCompatibleBackend::local("local-http", "127.0.0.1:7979", "local-model")
            .with_path("/v1/chat/completions");
        let backends: Vec<&dyn InferenceBackend> = vec![&deterministic, &openai];

        assert_eq!(backends[0].id(), "det-ref");
        assert_eq!(backends[1].id(), "local-http");
        assert!(backends[0].capabilities().streaming);
        assert!(backends[1].capabilities().openai_compatible);
        assert!(backends[1].capabilities().cancel);
    }

    #[test]
    fn openai_body_is_secret_free_and_requests_usage_streaming() {
        let request = InferenceRequest {
            prompt: "hello \"model\"".to_owned(),
            system_prompt: Some("be terse".to_owned()),
            model: None,
            max_tokens: 7,
            temperature_milli: 250,
        };
        let body = openai_chat_body("qwen-local", &request);

        assert!(body.contains("\"model\":\"qwen-local\""));
        assert!(body.contains("\"role\":\"system\""));
        assert!(body.contains("\"stream\":true"));
        assert!(body.contains("\"include_usage\":true"));
        assert!(!body.to_ascii_lowercase().contains("api_key"));
    }

    #[test]
    fn openai_sse_delta_and_usage_helpers_parse_compatible_shapes() {
        let delta = r#"{"choices":[{"delta":{"content":"hello"}}]}"#;
        let usage =
            r#"{"choices":[],"usage":{"prompt_tokens":3,"completion_tokens":5,"total_tokens":8}}"#;

        assert_eq!(openai_delta_text(delta).as_deref(), Some("hello"));
        let parsed_usage = usage_from_event(usage, 1, 1, Instant::now());
        assert_eq!(parsed_usage.prompt_tokens, 3);
        assert_eq!(parsed_usage.completion_tokens, 5);
        assert_eq!(parsed_usage.total_tokens, 8);
    }

    #[test]
    fn canceled_http_request_fails_before_touching_network() {
        let backend = OpenAiCompatibleBackend::local("local-http", "127.0.0.1:1", "local-model")
            .with_timeout_secs(1);
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        let error = match backend.stream(InferenceRequest::new("prompt", 4), cancellation) {
            Ok(_) => panic!("expected canceled request to fail before network access"),
            Err(error) => error,
        };

        assert_eq!(error.kind, BackendErrorKind::Canceled);
    }

    #[test]
    fn classifies_transport_errors() {
        assert_eq!(
            classify_http_error("connect 127.0.0.1 failed").kind,
            BackendErrorKind::Unavailable
        );
        assert_eq!(
            classify_http_error("read body timed out").kind,
            BackendErrorKind::Timeout
        );
        assert_eq!(
            classify_http_error("stream frame broke").kind,
            BackendErrorKind::Protocol
        );
    }

    #[test]
    fn capabilities_cover_m2_contract_fields() {
        let capabilities = DeterministicBackend::new("det-ref").capabilities();

        assert_eq!(capabilities.ctx_window, 8192);
        assert!(capabilities.streaming);
        assert!(capabilities.cancel);
        assert!(!capabilities.kv_export);
        assert!(capabilities.local);
        assert!(!capabilities.openai_compatible);
    }

    #[test]
    fn count_tokens_ignores_extra_whitespace() {
        assert_eq!(count_tokens(" one\n two\tthree "), 3);
    }
}
