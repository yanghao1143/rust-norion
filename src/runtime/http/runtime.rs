use std::time::Duration;

use crate::development_pollution::{
    DefenseSpacer, DefenseSpacerActivationGate, DefenseSpacerCandidate, DevelopmentPollutionEvent,
    classify_development_pollution_event, development_evidence_payload_reason,
    gate_defense_spacer_activation,
};
use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::ReasoningStep;
use crate::runtime::command::populate_static_runtime_diagnostics;
use crate::runtime::http::client::{HttpEndpoint, HttpStreamTimeouts};
use crate::runtime::http::openai::{
    CHAT_COMPLETIONS_PATH, ChatCompletionOptions, ChatCompletionStreamAccumulator,
    chat_completion_payload_with_options, chat_completion_stream_payload_with_options,
    response_from_chat_completion, response_from_chat_completion_stream,
};
use crate::runtime::{
    ModelRuntime, RuntimeError, RuntimeMetadata, RuntimeRequest, RuntimeResponse, RuntimeToken,
};
use crate::runtime_manifest::{
    TransformerRuntimeArchitecture, default_transformer_runtime_architecture,
};

#[derive(Debug, Clone)]
pub struct MistralRsHttpRuntime {
    endpoint: HttpEndpoint,
    timeout_ms: Option<u64>,
    stream_idle_timeout_ms: Option<u64>,
    metadata: RuntimeMetadata,
    architecture: Option<TransformerRuntimeArchitecture>,
    imported_kv_blocks: Vec<RuntimeKvBlock>,
    exported_kv_blocks: Vec<RuntimeKvBlock>,
    activation_gate: Option<DefenseSpacerActivationGate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatCompletionWire {
    Json,
    Stream,
}

#[derive(Debug, Clone)]
struct ChatCompletionPayload {
    body: String,
    wire: ChatCompletionWire,
    used_stable_retry: bool,
    used_non_stream_fallback: bool,
}

impl MistralRsHttpRuntime {
    pub fn new(base_url: impl AsRef<str>) -> Result<Self, RuntimeError> {
        let base_url = base_url.as_ref();
        Ok(Self {
            endpoint: HttpEndpoint::parse(base_url)?,
            timeout_ms: None,
            stream_idle_timeout_ms: None,
            metadata: RuntimeMetadata::default(),
            architecture: None,
            imported_kv_blocks: Vec::new(),
            exported_kv_blocks: Vec::new(),
            activation_gate: None,
        }
        .with_development_pollution_activation_gate(
            "mistralrs_http_runtime",
            "http_endpoint",
            base_url,
        ))
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms.max(1));
        self
    }

    pub fn with_stream_idle_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.stream_idle_timeout_ms = Some(timeout_ms.max(1));
        self
    }

    pub fn with_metadata(mut self, metadata: RuntimeMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_architecture(mut self, architecture: TransformerRuntimeArchitecture) -> Self {
        self.architecture = Some(architecture);
        self
    }

    pub fn with_activation_gate(mut self, gate: DefenseSpacerActivationGate) -> Self {
        self.activation_gate = Some(gate);
        self
    }

    pub fn with_development_pollution_activation_gate(
        mut self,
        source_kind: impl Into<String>,
        matched_scope: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        let source_kind = source_kind.into();
        let matched_scope = matched_scope.into();
        let payload = payload.into();
        let reason = development_evidence_payload_reason(&payload);
        let mut event = DevelopmentPollutionEvent::new(
            format!("{source_kind}-activation"),
            source_kind,
            payload,
            reason,
        );
        if reason == "current_validated_path" {
            event = event.with_current_proof(true);
        }
        let finding = classify_development_pollution_event(&event);
        let spacer = DefenseSpacer::from_finding(
            &finding,
            matched_scope.clone(),
            "mistralrs-http-runtime",
            "live_validation_before_http_runtime_activation",
        );
        let candidate = DefenseSpacerCandidate::from_finding(&finding, matched_scope);
        self.activation_gate = Some(gate_defense_spacer_activation(&[spacer], &candidate));
        self
    }

    pub fn activation_gate(&self) -> Option<&DefenseSpacerActivationGate> {
        self.activation_gate.as_ref()
    }

    pub fn timeout_ms(&self) -> Option<u64> {
        self.timeout_ms
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout_ms.map(Duration::from_millis)
    }

    pub fn stream_idle_timeout_ms(&self) -> Option<u64> {
        self.stream_idle_timeout_ms
    }

    fn stream_timeouts(&self) -> HttpStreamTimeouts {
        HttpStreamTimeouts::new(
            self.timeout_ms,
            self.stream_idle_timeout_ms.or(self.timeout_ms),
        )
    }

    fn ensure_activation_allowed(&self) -> Result<(), RuntimeError> {
        if let Some(gate) = self.activation_gate.as_ref().filter(|gate| !gate.allowed) {
            return Err(RuntimeError::new(format!(
                "mistralrs HTTP runtime activation blocked: {}",
                gate.summary_line()
            )));
        }
        Ok(())
    }

    fn post_chat_completion_json(
        &self,
        request: &RuntimeRequest,
    ) -> Result<(String, bool), RuntimeError> {
        let body = chat_completion_payload_with_options(
            request,
            ChatCompletionOptions::default_for(request),
        );
        match self
            .endpoint
            .post_json(CHAT_COMPLETIONS_PATH, &body, self.timeout_ms)
        {
            Ok(payload) => Ok((payload, false)),
            Err(error) if should_retry_with_stable_sampling(&error) => {
                let stable_body = chat_completion_payload_with_options(
                    request,
                    ChatCompletionOptions::stable_retry_for(request),
                );
                self.endpoint
                    .post_json(CHAT_COMPLETIONS_PATH, &stable_body, self.timeout_ms)
                    .map(|payload| (payload, true))
                    .map_err(|retry_error| {
                        RuntimeError::new(format!(
                            "mistralrs HTTP runtime failed after stable retry: first error: {}; retry error: {}",
                            error.message(),
                            retry_error.message()
                        ))
                    })
            }
            Err(error) => Err(error),
        }
    }

    fn post_chat_completion_stream(
        &self,
        request: &RuntimeRequest,
    ) -> Result<(String, bool), RuntimeError> {
        let body = chat_completion_stream_payload_with_options(
            request,
            ChatCompletionOptions::default_for(request),
        );
        match self
            .endpoint
            .post_json(CHAT_COMPLETIONS_PATH, &body, self.timeout_ms)
        {
            Ok(payload) => Ok((payload, false)),
            Err(error) if should_retry_with_stable_sampling(&error) => {
                let stable_body = chat_completion_stream_payload_with_options(
                    request,
                    ChatCompletionOptions::stable_retry_for(request),
                );
                self.endpoint
                    .post_json(CHAT_COMPLETIONS_PATH, &stable_body, self.timeout_ms)
                    .map(|payload| (payload, true))
                    .map_err(|retry_error| {
                        RuntimeError::new(format!(
                            "mistralrs HTTP runtime stream failed after stable retry: first error: {}; retry error: {}",
                            error.message(),
                            retry_error.message()
                        ))
                    })
            }
            Err(error) => Err(error),
        }
    }

    fn post_chat_completion(
        &self,
        request: &RuntimeRequest,
    ) -> Result<ChatCompletionPayload, RuntimeError> {
        match self.post_chat_completion_stream(request) {
            Ok((body, used_stable_retry)) => Ok(ChatCompletionPayload {
                body,
                wire: ChatCompletionWire::Stream,
                used_stable_retry,
                used_non_stream_fallback: false,
            }),
            Err(stream_error) if should_fallback_to_non_streaming(&stream_error) => {
                let (body, used_stable_retry) =
                    self.post_chat_completion_json(request).map_err(|json_error| {
                        RuntimeError::new(format!(
                            "mistralrs HTTP runtime stream failed and non-stream fallback failed: stream error: {}; fallback error: {}",
                            stream_error.message(),
                            json_error.message()
                        ))
                    })?;
                Ok(ChatCompletionPayload {
                    body,
                    wire: ChatCompletionWire::Json,
                    used_stable_retry,
                    used_non_stream_fallback: true,
                })
            }
            Err(error) => Err(error),
        }
    }

    fn post_chat_completion_stream_observed(
        &self,
        request: &RuntimeRequest,
        on_token: &mut dyn FnMut(&RuntimeToken) -> Result<(), RuntimeError>,
    ) -> Result<(RuntimeResponse, bool, bool), RuntimeError> {
        let body = chat_completion_stream_payload_with_options(
            request,
            ChatCompletionOptions::default_for(request),
        );
        let mut accumulator = ChatCompletionStreamAccumulator::default();
        let mut used_stable_retry = false;
        let raw = match self.endpoint.post_json_stream(
            CHAT_COMPLETIONS_PATH,
            &body,
            self.stream_timeouts(),
            &mut |chunk| {
                accumulator.push_bytes(chunk, &mut |delta| on_token(&RuntimeToken::new(delta)))
            },
        ) {
            Ok(raw) => raw,
            Err(error) if should_fallback_to_non_streaming(&error) => {
                let (payload, used_stable_retry) = self.post_chat_completion_json(request)?;
                let response = response_from_chat_completion(&payload)?;
                for token in &response.tokens {
                    on_token(token)?;
                }
                return Ok((response, used_stable_retry, true));
            }
            Err(error) if should_retry_with_stable_sampling(&error) => {
                used_stable_retry = true;
                let stable_body = chat_completion_stream_payload_with_options(
                    request,
                    ChatCompletionOptions::stable_retry_for(request),
                );
                accumulator = ChatCompletionStreamAccumulator::default();
                self.endpoint.post_json_stream(
                    CHAT_COMPLETIONS_PATH,
                    &stable_body,
                    self.stream_timeouts(),
                    &mut |chunk| {
                        accumulator
                            .push_bytes(chunk, &mut |delta| on_token(&RuntimeToken::new(delta)))
                    },
                )?
            }
            Err(error) => return Err(error),
        };

        let delta_count = accumulator.delta_count();
        match accumulator.finish() {
            Ok(response) => Ok((response, used_stable_retry, false)),
            Err(stream_error) if delta_count == 0 => {
                let raw = String::from_utf8(raw).map_err(|error| {
                    RuntimeError::new(format!("mistralrs HTTP response was not UTF-8: {error}"))
                })?;
                let response = response_from_chat_completion(&raw).map_err(|json_error| {
                    RuntimeError::new(format!(
                        "mistralrs HTTP runtime stream produced no deltas and JSON fallback parse failed: stream error: {}; JSON parse error: {}",
                        stream_error.message(),
                        json_error.message()
                    ))
                })?;
                for token in &response.tokens {
                    on_token(token)?;
                }
                Ok((response, used_stable_retry, false))
            }
            Err(error) => Err(error),
        }
    }
}

fn should_retry_with_stable_sampling(error: &RuntimeError) -> bool {
    let message = error.message().to_ascii_lowercase();
    message.contains("invalid sampling probability") || message.contains("nan")
}

fn should_fallback_to_non_streaming(error: &RuntimeError) -> bool {
    let message = error.message().to_ascii_lowercase();
    message.contains("returned status 400")
        || message.contains("returned status 404")
        || message.contains("returned status 405")
        || message.contains("returned status 422")
}

impl ModelRuntime for MistralRsHttpRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        self.metadata.clone()
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        self.architecture.unwrap_or_else(|| {
            default_transformer_runtime_architecture(
                self.metadata.native_context_window,
                self.metadata.embedding_dimensions,
            )
        })
    }

    fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        self.imported_kv_blocks.clear();
        self.imported_kv_blocks.extend(blocks.iter().cloned());
        Ok(self.imported_kv_blocks.len())
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        Ok(self.exported_kv_blocks.clone())
    }

    fn supports_endpoint_override(&self) -> bool {
        true
    }

    fn clone_for_endpoint_override(&self, base_url: &str) -> Result<Option<Self>, RuntimeError> {
        Ok(Some(
            Self {
                endpoint: HttpEndpoint::parse(base_url)?,
                timeout_ms: self.timeout_ms,
                stream_idle_timeout_ms: self.stream_idle_timeout_ms,
                metadata: self.metadata.clone(),
                architecture: self.architecture,
                imported_kv_blocks: Vec::new(),
                exported_kv_blocks: Vec::new(),
                activation_gate: None,
            }
            .with_development_pollution_activation_gate(
                "mistralrs_http_endpoint_override",
                "http_endpoint_override",
                base_url,
            ),
        ))
    }

    fn generate(&mut self, mut request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        self.exported_kv_blocks.clear();
        self.ensure_activation_allowed()?;
        if request.imported_kv_blocks.is_empty() && !self.imported_kv_blocks.is_empty() {
            request.imported_kv_blocks = std::mem::take(&mut self.imported_kv_blocks);
        } else {
            self.imported_kv_blocks.clear();
        }

        let payload = self.post_chat_completion(&request)?;
        let mut response = match payload.wire {
            ChatCompletionWire::Stream => {
                match response_from_chat_completion_stream(&payload.body) {
                    Ok(response) => response,
                    Err(stream_error) => {
                        let mut response = response_from_chat_completion(&payload.body).map_err(
                            |json_error| {
                                RuntimeError::new(format!(
                                    "mistralrs HTTP runtime could not parse response as stream or JSON: stream parse error: {}; JSON parse error: {}",
                                    stream_error.message(),
                                    json_error.message()
                                ))
                            },
                        )?;
                        response.trace.push(ReasoningStep::new(
                            "mistralrs_http_stream_parse_fallback",
                            "stream request returned non-SSE JSON; parsed as regular chat completion",
                            0.66,
                        ));
                        response
                    }
                }
            }
            ChatCompletionWire::Json => response_from_chat_completion(&payload.body)?,
        };
        let architecture = self.architecture();
        populate_static_runtime_diagnostics(
            &mut response.diagnostics,
            &self.metadata,
            architecture,
        );
        if payload.used_stable_retry {
            response.trace.push(ReasoningStep::new(
                "mistralrs_http_runtime_stable_retry",
                "retried with greedy sampling after unstable sampling while preserving requested max_tokens",
                0.74,
            ));
        }
        if payload.used_non_stream_fallback {
            response.trace.push(ReasoningStep::new(
                "mistralrs_http_runtime_non_stream_fallback",
                "streaming chat completion was unavailable; used non-streaming JSON",
                0.68,
            ));
        }
        response.trace.push(ReasoningStep::new(
            "mistralrs_http_runtime_endpoint",
            "called persistent mistralrs serve",
            0.82,
        ));
        Ok(response)
    }

    fn generate_stream(
        &mut self,
        request: RuntimeRequest,
        on_token: &mut dyn FnMut(&RuntimeToken) -> Result<(), RuntimeError>,
    ) -> Result<RuntimeResponse, RuntimeError> {
        self.exported_kv_blocks.clear();
        self.ensure_activation_allowed()?;
        let mut request = request;
        if request.imported_kv_blocks.is_empty() && !self.imported_kv_blocks.is_empty() {
            request.imported_kv_blocks = std::mem::take(&mut self.imported_kv_blocks);
        } else {
            self.imported_kv_blocks.clear();
        }

        let (mut response, used_stable_retry, used_non_stream_fallback) =
            self.post_chat_completion_stream_observed(&request, on_token)?;
        let architecture = self.architecture();
        populate_static_runtime_diagnostics(
            &mut response.diagnostics,
            &self.metadata,
            architecture,
        );
        if used_stable_retry {
            response.trace.push(ReasoningStep::new(
                "mistralrs_http_runtime_stable_retry",
                "retried with greedy sampling after unstable sampling while preserving requested max_tokens",
                0.74,
            ));
        }
        if used_non_stream_fallback {
            response.trace.push(ReasoningStep::new(
                "mistralrs_http_runtime_non_stream_fallback",
                "streaming chat completion was unavailable; used non-streaming JSON",
                0.68,
            ));
        }
        response.trace.push(ReasoningStep::new(
            "mistralrs_http_runtime_endpoint",
            "called persistent mistralrs serve with streaming observer",
            0.83,
        ));
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_sampling_nan_errors() {
        let error = RuntimeError::new(
            "mistralrs HTTP runtime returned status 500: Invalid sampling probability at index 0: NaN",
        );

        assert!(should_retry_with_stable_sampling(&error));
    }

    #[test]
    fn falls_back_to_non_streaming_for_unsupported_stream_routes() {
        let error =
            RuntimeError::new("mistralrs HTTP runtime returned status 422: stream unsupported");

        assert!(should_fallback_to_non_streaming(&error));
    }

    #[test]
    fn does_not_fallback_to_non_streaming_for_timeout_or_connect_errors() {
        let error = RuntimeError::new("failed to read mistralrs HTTP response: timed out");

        assert!(!should_fallback_to_non_streaming(&error));
    }

    #[test]
    fn endpoint_override_preserves_stream_idle_timeout() {
        let runtime = MistralRsHttpRuntime::new("http://127.0.0.1:8686")
            .unwrap()
            .with_timeout_ms(1_000)
            .with_stream_idle_timeout_ms(50);

        let override_runtime = runtime
            .clone_for_endpoint_override("http://127.0.0.1:8687")
            .unwrap()
            .unwrap();

        assert_eq!(override_runtime.timeout_ms(), Some(1_000));
        assert_eq!(override_runtime.stream_idle_timeout_ms(), Some(50));
    }

    #[test]
    fn does_not_retry_unrelated_http_errors() {
        let error = RuntimeError::new("mistralrs HTTP runtime returned status 404: missing route");

        assert!(!should_retry_with_stable_sampling(&error));
    }
}
