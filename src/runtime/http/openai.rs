use crate::reflection::ReasoningStep;
use crate::runtime::wire::split_json_objects;
use crate::runtime::wire::{extract_json_array_field, extract_json_object_field};
use crate::runtime::wire::{extract_json_string_field, extract_json_usize_field, json_string};
use crate::runtime::{RuntimeError, RuntimeRequest, RuntimeResponse, RuntimeToken};

pub(in crate::runtime) const CHAT_COMPLETIONS_PATH: &str = "/v1/chat/completions";

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::runtime) struct ChatCompletionOptions {
    max_tokens: usize,
    temperature: f32,
}

impl ChatCompletionOptions {
    pub(in crate::runtime) fn default_for(request: &RuntimeRequest) -> Self {
        Self {
            max_tokens: request.max_tokens.max(1),
            temperature: 0.2,
        }
    }

    pub(in crate::runtime) fn stable_retry_for(request: &RuntimeRequest) -> Self {
        Self {
            max_tokens: request.max_tokens.max(1),
            temperature: 0.0,
        }
    }
}

pub(in crate::runtime) fn chat_completion_payload_with_options(
    request: &RuntimeRequest,
    options: ChatCompletionOptions,
) -> String {
    chat_completion_payload_with_stream(request, options, false)
}

pub(in crate::runtime) fn chat_completion_stream_payload_with_options(
    request: &RuntimeRequest,
    options: ChatCompletionOptions,
) -> String {
    chat_completion_payload_with_stream(request, options, true)
}

fn chat_completion_payload_with_stream(
    request: &RuntimeRequest,
    options: ChatCompletionOptions,
    stream: bool,
) -> String {
    format!(
        "{{\
         \"model\":{},\
         \"messages\":[\
         {{\"role\":\"system\",\"content\":{}}},\
         {{\"role\":\"user\",\"content\":{}}}\
         ],\
         \"max_tokens\":{},\
         \"stream\":{},\
         \"temperature\":{:.3},\
         \"top_p\":1.0,\
         \"enable_thinking\":false\
         }}",
        json_string(&request.runtime_metadata.model_id),
        json_string(runtime_system_prompt()),
        json_string(&request.prompt),
        options.max_tokens,
        stream,
        options.temperature
    )
}

fn runtime_system_prompt() -> &'static str {
    "你是本地 Gemma，请直接回答用户；用户用中文就用中文。"
}

pub(in crate::runtime) fn response_from_chat_completion(
    payload: &str,
) -> Result<RuntimeResponse, RuntimeError> {
    let answer = sanitize_chat_answer(&parse_chat_completion_answer(payload)?);
    let mut response = RuntimeResponse::new(answer.clone());
    response.diagnostics.model_id = chat_completion_model_id(payload);
    if let Some(usage) = parse_chat_completion_usage(payload) {
        response.tokens = runtime_tokens_from_usage(&answer, usage.completion_tokens);
        response.trace.push(ReasoningStep::new(
            "mistralrs_http_usage",
            usage.summary(),
            0.72,
        ));
    }
    response.trace.push(ReasoningStep::new(
        "mistralrs_http_runtime",
        "OpenAI chat completion",
        0.78,
    ));
    Ok(response)
}

pub(in crate::runtime) fn response_from_chat_completion_stream(
    payload: &str,
) -> Result<RuntimeResponse, RuntimeError> {
    let mut accumulator = ChatCompletionStreamAccumulator::default();
    accumulator.push_bytes(payload.as_bytes(), &mut |_| Ok(()))?;
    accumulator.finish()
}

pub(in crate::runtime) fn parse_chat_completion_answer(
    payload: &str,
) -> Result<String, RuntimeError> {
    let choices = extract_json_array_field(payload, "choices")
        .ok_or_else(|| RuntimeError::new("mistralrs response missing choices array"))?;
    let first_choice = split_json_objects(choices)
        .into_iter()
        .next()
        .ok_or_else(|| RuntimeError::new("mistralrs response choices array was empty"))?;
    let message = extract_json_object_field(first_choice, "message")
        .ok_or_else(|| RuntimeError::new("mistralrs response choice missing message object"))?;
    let content = extract_json_string_field(message, "content")
        .ok_or_else(|| RuntimeError::new("mistralrs response message missing content string"))?;
    if content.trim().is_empty() {
        return Err(RuntimeError::new(
            "mistralrs response message content was empty",
        ));
    }
    Ok(content)
}

#[cfg(test)]
pub(in crate::runtime) fn parse_chat_completion_stream_deltas(
    payload: &str,
) -> Result<Vec<String>, RuntimeError> {
    let mut accumulator = ChatCompletionStreamAccumulator::default();
    accumulator.push_bytes(payload.as_bytes(), &mut |_| Ok(()))?;
    accumulator.deltas()
}

#[derive(Debug, Clone, Default)]
pub(in crate::runtime) struct ChatCompletionStreamAccumulator {
    pending: Vec<u8>,
    deltas: Vec<String>,
    model_id: Option<String>,
    done: bool,
}

impl ChatCompletionStreamAccumulator {
    pub(in crate::runtime) fn push_bytes(
        &mut self,
        bytes: &[u8],
        on_delta: &mut dyn FnMut(&str) -> Result<(), RuntimeError>,
    ) -> Result<(), RuntimeError> {
        if self.done {
            return Ok(());
        }
        self.pending.extend_from_slice(bytes);
        while let Some((event_end, boundary_len)) = sse_event_boundary(&self.pending) {
            let event = self.pending[..event_end].to_vec();
            self.pending.drain(..event_end + boundary_len);
            self.handle_event(&event, on_delta)?;
            if self.done {
                break;
            }
        }
        Ok(())
    }

    pub(in crate::runtime) fn delta_count(&self) -> usize {
        self.deltas.len()
    }

    #[cfg(test)]
    pub(in crate::runtime) fn deltas(&self) -> Result<Vec<String>, RuntimeError> {
        if self.deltas.is_empty() {
            return Err(RuntimeError::new(
                "mistralrs stream response had no content deltas",
            ));
        }
        Ok(self.deltas.clone())
    }

    pub(in crate::runtime) fn finish(self) -> Result<RuntimeResponse, RuntimeError> {
        let mut response = response_from_stream_deltas(&self.deltas)?;
        response.diagnostics.model_id = self.model_id;
        Ok(response)
    }

    fn handle_event(
        &mut self,
        event: &[u8],
        on_delta: &mut dyn FnMut(&str) -> Result<(), RuntimeError>,
    ) -> Result<(), RuntimeError> {
        let event = std::str::from_utf8(event).map_err(|error| {
            RuntimeError::new(format!("mistralrs stream event was not UTF-8: {error}"))
        })?;
        let event_data = sse_event_data(event);
        if event_data.trim().is_empty() {
            return Ok(());
        }
        if event_data.trim() == "[DONE]" {
            self.done = true;
            return Ok(());
        }

        if self.model_id.is_none() {
            self.model_id = chat_completion_model_id(&event_data);
        }

        for delta in chat_completion_deltas_from_event(&event_data) {
            on_delta(&delta)?;
            self.deltas.push(delta);
        }
        Ok(())
    }
}

fn sse_event_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
    let lf = bytes.windows(2).position(|window| window == b"\n\n");
    let crlf = bytes.windows(4).position(|window| window == b"\r\n\r\n");
    match (lf, crlf) {
        (Some(lf), Some(crlf)) if crlf < lf => Some((crlf, 4)),
        (Some(lf), _) => Some((lf, 2)),
        (None, Some(crlf)) => Some((crlf, 4)),
        (None, None) => None,
    }
}

fn sse_event_data(event: &str) -> String {
    let mut data_lines = Vec::new();
    for line in event.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with(':') {
            continue;
        }
        if let Some(data) = line.strip_prefix("data:") {
            data_lines.push(data.trim_start());
        }
    }
    data_lines.join("\n")
}

fn chat_completion_deltas_from_event(event_data: &str) -> Vec<String> {
    let Some(choices) = extract_json_array_field(event_data, "choices") else {
        return Vec::new();
    };
    let mut deltas = Vec::new();
    for choice in split_json_objects(choices) {
        let Some(delta) = extract_json_object_field(choice, "delta") else {
            continue;
        };
        if let Some(content) = extract_json_string_field(delta, "content")
            && !content.is_empty()
        {
            deltas.push(content);
        }
    }
    deltas
}

fn response_from_stream_deltas(deltas: &[String]) -> Result<RuntimeResponse, RuntimeError> {
    if deltas.is_empty() {
        return Err(RuntimeError::new(
            "mistralrs stream response had no content deltas",
        ));
    }
    let raw_answer = deltas.join("");
    let answer = sanitize_chat_answer(&raw_answer);
    if answer.trim().is_empty() {
        return Err(RuntimeError::new(
            "mistralrs stream response content was empty",
        ));
    }

    let mut response = RuntimeResponse::new(answer);
    response.tokens = runtime_tokens_from_stream_deltas(deltas);
    response.trace.push(ReasoningStep::new(
        "mistralrs_http_stream",
        format!("OpenAI chat completion stream deltas={}", deltas.len()),
        0.8,
    ));
    Ok(response)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChatCompletionUsage {
    prompt_tokens: Option<usize>,
    completion_tokens: usize,
    total_tokens: Option<usize>,
}

impl ChatCompletionUsage {
    fn summary(self) -> String {
        format!(
            "usage prompt_tokens={} completion_tokens={} total_tokens={}",
            option_usize(self.prompt_tokens),
            self.completion_tokens,
            option_usize(self.total_tokens)
        )
    }
}

fn parse_chat_completion_usage(payload: &str) -> Option<ChatCompletionUsage> {
    let usage = extract_json_object_field(payload, "usage")?;
    let completion_tokens = extract_json_usize_field(usage, "completion_tokens")?;
    Some(ChatCompletionUsage {
        prompt_tokens: extract_json_usize_field(usage, "prompt_tokens"),
        completion_tokens,
        total_tokens: extract_json_usize_field(usage, "total_tokens"),
    })
}

fn chat_completion_model_id(payload: &str) -> Option<String> {
    extract_json_string_field(payload, "model").and_then(non_empty_string)
}

fn option_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn runtime_tokens_from_usage(answer: &str, completion_tokens: usize) -> Vec<RuntimeToken> {
    if completion_tokens == 0 {
        return Vec::new();
    }
    let chars = answer.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Vec::new();
    }

    let target_tokens = completion_tokens.min(chars.len());
    let mut tokens = Vec::with_capacity(target_tokens);
    let mut start = 0_usize;
    for index in 0..target_tokens {
        let remaining_chars = chars.len().saturating_sub(start);
        let remaining_tokens = target_tokens.saturating_sub(index).max(1);
        let take = remaining_chars.div_ceil(remaining_tokens).max(1);
        let end = (start + take).min(chars.len());
        let text = chars[start..end].iter().collect::<String>();
        tokens.push(RuntimeToken::new(text));
        start = end;
    }
    tokens
}

fn runtime_tokens_from_stream_deltas(deltas: &[String]) -> Vec<RuntimeToken> {
    deltas
        .iter()
        .filter(|delta| !delta.is_empty())
        .map(|delta| RuntimeToken::new(delta.clone()))
        .collect()
}

fn sanitize_chat_answer(answer: &str) -> String {
    let cleaned = answer
        .replace("<audio|>", "")
        .replace("<image|>", "")
        .replace("<video|>", "")
        .replace("<channel|>", "")
        .replace("<start_of_turn>", "")
        .replace("<end_of_turn>", "");
    strip_leading_thought_marker(&cleaned).trim().to_owned()
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn strip_leading_thought_marker(answer: &str) -> &str {
    let trimmed = answer.trim_start();
    let Some(rest) = trimmed.strip_prefix("thought") else {
        return trimmed;
    };
    if rest
        .chars()
        .next()
        .map(|character| character.is_whitespace() || character == ':' || character == '|')
        .unwrap_or(true)
    {
        rest.trim_start_matches(|character: char| {
            character.is_whitespace() || character == ':' || character == '|'
        })
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_request(prompt: &str) -> RuntimeRequest {
        RuntimeRequest {
            prompt: prompt.to_owned(),
            profile: crate::hierarchy::TaskProfile::General,
            runtime_metadata: crate::runtime::RuntimeMetadata::new(
                "gemma-test",
                "gemma-test-tokenizer",
                8192,
                4096,
            ),
            runtime_architecture: crate::runtime_manifest::TransformerRuntimeArchitecture::new(
                12, 128, 8, 4, 8192,
            ),
            memory_hints: Vec::new(),
            infini_memory_hints: Vec::new(),
            experience_hints: Vec::new(),
            runtime_adapter_observations: Vec::new(),
            toolsmith_plan: crate::toolsmith::ToolsmithPlan::default(),
            agent_team_plan: crate::agent_team::AgentTeamPlan::default(),
            route_budget: crate::router::RouteBudget {
                threshold: 0.5,
                attention_tokens: 2,
                fast_tokens: 1,
                attention_fraction: 0.66,
            },
            hierarchy: crate::hierarchy::HierarchyWeights::default(),
            transformer_plan: crate::transformer::TransformerRefactorPlan::default(),
            recursive_schedule: crate::recursive_scheduler::RecursiveSchedule::default(),
            hardware_plan: crate::hardware::HardwarePlan::default(),
            imported_kv_blocks: Vec::new(),
            max_tokens: 64,
        }
    }

    #[test]
    fn parses_openai_chat_completion_answer() {
        let payload = r#"{
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "你好，Gemma 已经接入。"
                    },
                    "finish_reason": "stop"
                }
            ]
        }"#;

        assert_eq!(
            parse_chat_completion_answer(payload).unwrap(),
            "你好，Gemma 已经接入。"
        );
    }

    #[test]
    fn rejects_empty_choices() {
        let error = parse_chat_completion_answer(r#"{"choices":[]}"#).unwrap_err();

        assert!(error.message().contains("empty"));
    }

    #[test]
    fn parses_openai_usage_into_runtime_tokens() {
        let payload = r#"{
            "model": "Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf",
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "你好，Gemma 已经接入。"
                    }
                }
            ],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 4,
                "total_tokens": 16
            }
        }"#;

        let response = response_from_chat_completion(payload).unwrap();

        assert_eq!(response.tokens.len(), 4);
        assert_eq!(
            response.diagnostics.model_id.as_deref(),
            Some("Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf")
        );
        assert!(response.tokens.iter().all(|token| !token.text.is_empty()));
        assert!(response.trace.iter().any(|step| {
            step.label == "mistralrs_http_usage" && step.content.contains("completion_tokens=4")
        }));
    }

    #[test]
    fn stream_payload_requests_sse() {
        let request = runtime_request("用中文回答");
        let payload = chat_completion_stream_payload_with_options(
            &request,
            ChatCompletionOptions::default_for(&request),
        );

        assert!(payload.contains("\"stream\":true"));
        assert!(payload.contains("\"enable_thinking\":false"));
    }

    #[test]
    fn parses_openai_chat_completion_stream_deltas() {
        let payload = concat!(
            "data: {\"model\":\"Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf\",\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"你好\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"，Gemma\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" 已接入。\"}}]}\n\n",
            "data: [DONE]\n\n",
        );

        let response = response_from_chat_completion_stream(payload).unwrap();

        assert_eq!(response.answer, "你好，Gemma 已接入。");
        assert_eq!(response.tokens.len(), 3);
        assert_eq!(response.tokens[0].text, "你好");
        assert_eq!(
            response.diagnostics.model_id.as_deref(),
            Some("Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf")
        );
        assert!(
            response
                .trace
                .iter()
                .any(|step| step.label == "mistralrs_http_stream"
                    && step.content.contains("deltas=3"))
        );
    }

    #[test]
    fn stream_parser_ignores_comments_and_usage_only_events() {
        let payload = concat!(
            ": keepalive\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"A\"}}]}\n\n",
            "data: {\"choices\":[],\"usage\":{\"completion_tokens\":1}}\n\n",
            "data: [DONE]\n\n",
        );

        let deltas = parse_chat_completion_stream_deltas(payload).unwrap();

        assert_eq!(deltas, vec!["A".to_owned()]);
    }

    #[test]
    fn sanitizes_modality_tokens() {
        assert_eq!(
            sanitize_chat_answer("我是<audio|> Gemma<end_of_turn>。"),
            "我是 Gemma。"
        );
    }

    #[test]
    fn sanitizes_thought_channel_prefix() {
        assert_eq!(
            sanitize_chat_answer("thought\n<channel|>我是 Gemma。"),
            "我是 Gemma。"
        );
    }

    #[test]
    fn default_payload_uses_requested_generation_budget_for_local_http_runtime() {
        let mut request = runtime_request("用中文回答");
        request.max_tokens = 4096;

        let payload = chat_completion_payload_with_options(
            &request,
            ChatCompletionOptions::default_for(&request),
        );

        assert!(payload.contains("\"max_tokens\":4096"));
        assert!(payload.contains("\"temperature\":0.200"));
        assert!(payload.contains("\"top_p\":1.0"));
        assert!(payload.contains("\"enable_thinking\":false"));
    }

    #[test]
    fn stable_retry_payload_uses_greedy_sampling_without_shrinking_budget() {
        let mut request = runtime_request("用中文回答");
        request.max_tokens = 4096;

        let payload = chat_completion_payload_with_options(
            &request,
            ChatCompletionOptions::stable_retry_for(&request),
        );

        assert!(payload.contains("\"max_tokens\":4096"));
        assert!(payload.contains("\"temperature\":0.000"));
    }
}
