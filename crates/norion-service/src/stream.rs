use crate::protocol::{ChatChunk, ChatChunkKind, ChatRequest, StreamState};
use crate::session::ChatSession;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendSseEvent {
    pub event: String,
    pub data: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SseFrameBuffer {
    pending: String,
}

impl SseFrameBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pending(&self) -> &str {
        &self.pending
    }

    pub fn push(&mut self, chunk: &str) -> Vec<String> {
        self.pending.push_str(chunk);
        let mut frames = Vec::new();
        while let Some((frame, rest)) = split_complete_sse_frame(&self.pending) {
            frames.push(frame.to_owned());
            self.pending = rest.to_owned();
        }
        frames
    }

    pub fn apply_to_session(&mut self, session: &mut ChatSession, chunk: &str) -> Vec<ChatChunk> {
        self.push(chunk)
            .into_iter()
            .filter_map(|frame| apply_sse_frame(session, &frame))
            .collect()
    }

    pub fn finish(self) -> Option<String> {
        let pending = self.pending.trim();
        (!pending.is_empty()).then_some(self.pending)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamFrame {
    Sse(String),
    WebSocketText(String),
    CliText(String),
}

pub trait StreamAdapter {
    fn encode(&self, chunk: &ChatChunk) -> StreamFrame;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SseAdapter;

#[derive(Debug, Clone, Copy, Default)]
pub struct WebSocketAdapter;

#[derive(Debug, Clone, Copy, Default)]
pub struct CliAdapter;

impl StreamAdapter for SseAdapter {
    fn encode(&self, chunk: &ChatChunk) -> StreamFrame {
        let event = chunk.kind.as_str();
        StreamFrame::Sse(format!("event: {event}\ndata: {}\n\n", chunk_json(chunk)))
    }
}

impl StreamAdapter for WebSocketAdapter {
    fn encode(&self, chunk: &ChatChunk) -> StreamFrame {
        StreamFrame::WebSocketText(chunk_json(chunk))
    }
}

impl StreamAdapter for CliAdapter {
    fn encode(&self, chunk: &ChatChunk) -> StreamFrame {
        let display = chunk.display_snapshot();
        let text = match chunk.kind {
            ChatChunkKind::Delta => chunk.content.clone(),
            ChatChunkKind::Done => "\n".to_owned(),
            ChatChunkKind::Start if chunk.content.is_empty() => String::new(),
            _ => format!("[{}] {}\n", display.output_label, chunk.content),
        };
        StreamFrame::CliText(text)
    }
}

pub fn chunk_json(chunk: &ChatChunk) -> String {
    format!(
        "{{\"sequence\":{},\"state\":\"{}\",\"kind\":\"{}\",\"content\":{}}}",
        chunk.sequence,
        chunk.state.as_str(),
        chunk.kind.as_str(),
        json_string(&chunk.content)
    )
}

pub fn request_json(request: &ChatRequest) -> String {
    let wire = request.wire_snapshot();
    let mut fields = vec![
        format!("\"tenant_id\":{}", json_string(&request.tenant_id)),
        format!("\"workspace_id\":{}", json_string(&request.workspace_id)),
        format!("\"session_id\":{}", json_string(&request.session_id)),
        format!("\"messages\":{}", messages_json(request)),
        format!("\"profile\":{}", json_string(&request.profile)),
        format!("\"output\":{}", json_string(&request.output)),
        format!("\"stream\":{}", request.stream),
        format!("\"model_role\":{}", json_string(&wire.model_role_label)),
        format!(
            "\"routing_preference\":{}",
            json_string(&wire.routing_preference_label)
        ),
        format!("\"endpoint_pinned\":{}", wire.endpoint_pinned),
        format!(
            "\"endpoint_kind\":{}",
            json_string(&wire.endpoint_kind_label)
        ),
    ];
    if let Some(max_tokens) = wire.max_tokens {
        fields.push(format!("\"max_tokens\":{max_tokens}"));
    }
    if wire.prefer_fast {
        fields.push("\"prefer_fast\":true".to_owned());
    }
    if wire.prefer_quality {
        fields.push("\"prefer_quality\":true".to_owned());
    }
    if let Some(endpoint) = wire.model_endpoint_label.as_ref() {
        fields.push(format!("\"model_endpoint\":{}", json_string(endpoint)));
    }
    format!("{{{}}}", fields.join(","))
}

pub fn parse_sse_frame(frame: &str) -> Option<BackendSseEvent> {
    let mut event = String::from("message");
    let mut data = Vec::new();
    let mut saw_field = false;

    for line in frame.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        saw_field = true;
        if let Some(value) = line.strip_prefix("event:") {
            event = sse_field_value(value).to_owned();
        } else if let Some(value) = line.strip_prefix("data:") {
            data.push(sse_field_value(value).to_owned());
        }
    }

    saw_field.then(|| BackendSseEvent {
        event,
        data: data.join("\n"),
    })
}

pub fn apply_sse_frame(session: &mut ChatSession, frame: &str) -> Option<ChatChunk> {
    let event = parse_sse_frame(frame)?;
    Some(apply_backend_event(session, &event.event, &event.data))
}

fn sse_field_value(value: &str) -> &str {
    value.strip_prefix(' ').unwrap_or(value)
}

fn split_complete_sse_frame(input: &str) -> Option<(&str, &str)> {
    for delimiter in ["\r\n\r\n", "\n\n", "\r\r"] {
        if let Some(index) = input.find(delimiter) {
            let after = index + delimiter.len();
            return Some((&input[..index], &input[after..]));
        }
    }
    None
}

pub fn apply_backend_event(session: &mut ChatSession, event: &str, data: &str) -> ChatChunk {
    match event.trim() {
        "delta" => session.push_delta(data),
        "status" | "stage" | "heartbeat" => session.push_status(data),
        "meta" | "metadata" => session.push_metadata(data),
        "final" => session.push_final_payload(data),
        "done" => session.finish(),
        "error" => {
            if session.partial_answer().trim().is_empty() {
                session.fail(data)
            } else {
                session.interrupt(data)
            }
        }
        "queued" => session.queued(data),
        "busy" => session.busy(data),
        "backpressure" => session.backpressure(data),
        "" => session.push_status(data),
        other => session.push_status(format!("{other}: {data}")),
    }
}

pub fn apply_backend_final_answer(
    session: &mut ChatSession,
    payload: &str,
    assistant_answer: Option<&str>,
) -> ChatChunk {
    match assistant_answer {
        Some(answer) => session.push_final_payload_with_answer(payload, answer),
        None => session.push_final_payload(payload),
    }
}

pub fn close_incomplete_stream(
    session: &mut ChatSession,
    reason: impl Into<String>,
) -> Option<ChatChunk> {
    if session.state().is_terminal() {
        return None;
    }
    let reason = reason.into();
    Some(
        if let Some(pressure_close) = pressure_close_reason(session, &reason) {
            session.interrupt(pressure_close)
        } else if session.partial_answer().trim().is_empty() {
            session.fail(reason)
        } else {
            session.interrupt(reason)
        },
    )
}

fn pressure_close_reason(session: &ChatSession, reason: &str) -> Option<String> {
    let state = session.state();
    if !matches!(
        state,
        StreamState::Queued | StreamState::Busy | StreamState::Backpressure
    ) {
        return None;
    }
    let pressure = session
        .prompt_blocked_chunk()
        .map(|chunk| chunk.content)
        .filter(|content| !content.trim().is_empty())
        .unwrap_or_else(|| "backend pressure state".to_owned());
    Some(format!("{reason} after {}: {pressure}", state.as_str()))
}

fn messages_json(request: &ChatRequest) -> String {
    let messages = request
        .messages
        .iter()
        .map(|message| {
            format!(
                "{{\"role\":{},\"content\":{}}}",
                json_string(message.role.as_str()),
                json_string(&message.content)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{messages}]")
}

fn json_string(value: &str) -> String {
    let mut encoded = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => encoded.push_str("\\\""),
            '\\' => encoded.push_str("\\\\"),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            ch if ch.is_control() => encoded.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => encoded.push(ch),
        }
    }
    encoded.push('"');
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChatChunk, ChatMessage, ChatRequest, ChatSession, ChatSessionConfig, ModelEndpoint,
        ModelRole, RoutingPreference, StreamState,
    };

    #[test]
    fn adapters_keep_chunk_sequence_and_state() {
        let chunk = ChatChunk::delta(7, "hello");

        let StreamFrame::Sse(sse) = SseAdapter.encode(&chunk) else {
            panic!("expected sse frame");
        };
        let StreamFrame::WebSocketText(websocket) = WebSocketAdapter.encode(&chunk) else {
            panic!("expected websocket frame");
        };
        let StreamFrame::CliText(cli) = CliAdapter.encode(&chunk) else {
            panic!("expected cli frame");
        };

        assert!(sse.contains("event: delta"));
        assert!(sse.contains("\"sequence\":7"));
        assert!(websocket.contains("\"state\":\"streaming\""));
        assert_eq!(cli, "hello");
    }

    #[test]
    fn interrupted_chunk_serializes_as_terminal_state() {
        let chunk = ChatChunk::interrupted(3, "read timeout");

        assert_eq!(chunk.state, StreamState::Interrupted);
        assert!(chunk.state.is_terminal());
        assert!(chunk_json(&chunk).contains("\"state\":\"interrupted\""));
        assert!(chunk_json(&chunk).contains("read timeout"));
    }

    #[test]
    fn wire_frames_distinguish_interrupted_from_failed_by_state() {
        let interrupted = ChatChunk::interrupted(3, "read timeout");
        let failed = ChatChunk::failed(4, "safe-device gate failed");
        let StreamFrame::Sse(interrupted_sse) = SseAdapter.encode(&interrupted) else {
            panic!("expected interrupted sse frame");
        };
        let StreamFrame::WebSocketText(failed_ws) = WebSocketAdapter.encode(&failed) else {
            panic!("expected failed websocket frame");
        };

        assert!(interrupted_sse.contains("event: error"));
        assert!(interrupted_sse.contains("\"kind\":\"error\""));
        assert!(interrupted_sse.contains("\"state\":\"interrupted\""));
        assert!(failed_ws.contains("\"kind\":\"error\""));
        assert!(failed_ws.contains("\"state\":\"failed\""));
    }

    #[test]
    fn final_payload_encodes_as_final_stream_event() {
        let chunk = ChatChunk::final_payload(8, "{\"answer\":\"hello\"}");

        let StreamFrame::Sse(sse) = SseAdapter.encode(&chunk) else {
            panic!("expected sse frame");
        };
        let StreamFrame::CliText(cli) = CliAdapter.encode(&chunk) else {
            panic!("expected cli frame");
        };

        assert!(sse.contains("event: final"));
        assert!(sse.contains("\"kind\":\"final\""));
        assert!(sse.contains("\\\"answer\\\":\\\"hello\\\""));
        assert_eq!(cli, "[final] {\"answer\":\"hello\"}\n");
    }

    #[test]
    fn pressure_states_are_visible_to_all_stream_adapters() {
        let chunk = ChatChunk::backpressure(9, "all workers are saturated");

        let StreamFrame::Sse(sse) = SseAdapter.encode(&chunk) else {
            panic!("expected sse frame");
        };
        let StreamFrame::WebSocketText(websocket) = WebSocketAdapter.encode(&chunk) else {
            panic!("expected websocket frame");
        };
        let StreamFrame::CliText(cli) = CliAdapter.encode(&chunk) else {
            panic!("expected cli frame");
        };

        assert!(sse.contains("\"state\":\"backpressure\""));
        assert!(websocket.contains("\"state\":\"backpressure\""));
        assert_eq!(cli, "[backpressure] all workers are saturated\n");
    }

    #[test]
    fn cli_adapter_labels_interrupted_separately_from_hard_failure() {
        let StreamFrame::CliText(interrupted) =
            CliAdapter.encode(&ChatChunk::interrupted(10, "missing done"))
        else {
            panic!("expected cli frame");
        };
        let StreamFrame::CliText(failed) =
            CliAdapter.encode(&ChatChunk::failed(11, "safe-device gate failed"))
        else {
            panic!("expected cli frame");
        };

        assert_eq!(interrupted, "[interrupted] missing done\n");
        assert_eq!(failed, "[error] safe-device gate failed\n");
    }

    #[test]
    fn request_json_preserves_history_tokens_and_routing_hints() {
        let request = ChatRequest::new(
            "cli-session",
            vec![
                ChatMessage::system("be concise"),
                ChatMessage::user("review this"),
            ],
        )
        .with_max_tokens(Some(8192))
        .with_model_role(ModelRole::Reviewer)
        .with_routing_preference(RoutingPreference::PreferFast)
        .with_model_endpoint(Some(ModelEndpoint::FastReviewer));

        let json = request_json(&request);

        assert!(json.contains("\"tenant_id\":\"local\""));
        assert!(json.contains("\"workspace_id\":\"default\""));
        assert!(json.contains("\"session_id\":\"cli-session\""));
        assert!(json.contains("\"role\":\"system\",\"content\":\"be concise\""));
        assert!(json.contains("\"role\":\"user\",\"content\":\"review this\""));
        assert!(json.contains("\"max_tokens\":8192"));
        assert!(json.contains("\"model_role\":\"reviewer\""));
        assert!(json.contains("\"routing_preference\":\"prefer_fast\""));
        assert!(json.contains("\"prefer_fast\":true"));
        assert!(!json.contains("\"prefer_quality\":true"));
        assert!(json.contains("\"endpoint_pinned\":true"));
        assert!(json.contains("\"endpoint_kind\":\"built_in\""));
        assert!(json.contains("\"model_endpoint\":\"fast-reviewer\""));
        assert!(!json.contains("\"max_tokens\":128"));
    }

    #[test]
    fn request_json_sends_explicit_tenant_workspace_session_scope() {
        let request = ChatRequest::new("scope-session", vec![ChatMessage::user("review")])
            .with_tenant_scope("tenant-a", "workspace-one");

        let json = request_json(&request);

        assert!(json.contains("\"tenant_id\":\"tenant-a\""));
        assert!(json.contains("\"workspace_id\":\"workspace-one\""));
        assert!(json.contains("\"session_id\":\"scope-session\""));
    }

    #[test]
    fn request_json_escapes_history_and_custom_worker_endpoint() {
        let request = ChatRequest::new(
            "cli\nsession",
            vec![
                ChatMessage::system("quote: \"stay safe\""),
                ChatMessage::user("line one\nline two\twith tab"),
            ],
        )
        .with_model_role(ModelRole::Tester)
        .with_routing_preference(RoutingPreference::PreferQuality)
        .with_model_endpoint(Some(ModelEndpoint::Worker("mlx\\tester\"pool".to_owned())));

        let json = request_json(&request);

        assert!(json.contains("\"session_id\":\"cli\\nsession\""));
        assert!(json.contains("\"content\":\"quote: \\\"stay safe\\\"\""));
        assert!(json.contains("\"content\":\"line one\\nline two\\twith tab\""));
        assert!(json.contains("\"model_role\":\"tester\""));
        assert!(json.contains("\"routing_preference\":\"prefer_quality\""));
        assert!(json.contains("\"prefer_quality\":true"));
        assert!(!json.contains("\"prefer_fast\":true"));
        assert!(json.contains("\"endpoint_pinned\":true"));
        assert!(json.contains("\"endpoint_kind\":\"custom\""));
        assert!(json.contains("\"model_endpoint\":\"mlx\\\\tester\\\"pool\""));
    }

    #[test]
    fn request_json_omits_optional_endpoint_and_token_budget_when_absent() {
        let request = ChatRequest::new("auto", vec![ChatMessage::user("hello")]);

        let json = request_json(&request);

        assert!(json.contains("\"routing_preference\":\"balanced\""));
        assert!(json.contains("\"model_role\":\"assistant\""));
        assert!(json.contains("\"endpoint_pinned\":false"));
        assert!(json.contains("\"endpoint_kind\":\"auto\""));
        assert!(!json.contains("\"prefer_fast\""));
        assert!(!json.contains("\"prefer_quality\""));
        assert!(!json.contains("\"model_endpoint\""));
        assert!(!json.contains("\"max_tokens\""));
    }

    #[test]
    fn request_json_keeps_custom_worker_pin_distinct_from_backend_default_tokens() {
        let request = ChatRequest::new("cli-session", vec![ChatMessage::user("review this")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast)
            .with_model_endpoint(Some(ModelEndpoint::Worker("mlx-reviewer-8b".to_owned())))
            .with_max_tokens(None);

        let json = request_json(&request);

        assert!(json.contains("\"model_role\":\"reviewer\""));
        assert!(json.contains("\"routing_preference\":\"prefer_fast\""));
        assert!(json.contains("\"prefer_fast\":true"));
        assert!(!json.contains("\"prefer_quality\":true"));
        assert!(json.contains("\"endpoint_pinned\":true"));
        assert!(json.contains("\"endpoint_kind\":\"custom\""));
        assert!(json.contains("\"model_endpoint\":\"mlx-reviewer-8b\""));
        assert!(!json.contains("\"max_tokens\""));
    }

    #[test]
    fn request_json_treats_unpinned_endpoint_hint_as_auto_route() {
        let request = ChatRequest::new("auto", vec![ChatMessage::user("review")])
            .with_routing_intent(crate::RoutingIntent {
                model_role: ModelRole::Reviewer,
                routing_preference: RoutingPreference::PreferFast,
                model_endpoint: Some(ModelEndpoint::FastReviewer),
                endpoint_pinned: false,
            });

        let json = request_json(&request);

        assert!(json.contains("\"model_role\":\"reviewer\""));
        assert!(json.contains("\"routing_preference\":\"prefer_fast\""));
        assert!(json.contains("\"prefer_fast\":true"));
        assert!(json.contains("\"endpoint_pinned\":false"));
        assert!(json.contains("\"endpoint_kind\":\"auto\""));
        assert!(!json.contains("\"model_endpoint\""));
    }

    #[test]
    fn parses_sse_frame_with_multiline_data_and_crlf() {
        let event = parse_sse_frame("event: meta\r\ndata: line one\r\ndata: line two\r\n\r\n")
            .expect("expected event");

        assert_eq!(
            event,
            BackendSseEvent {
                event: "meta".to_owned(),
                data: "line one\nline two".to_owned(),
            }
        );
    }

    #[test]
    fn parses_sse_frame_defaults_event_and_ignores_comments() {
        let event = parse_sse_frame(": keepalive\ndata: hello\n").expect("expected event");

        assert_eq!(
            event,
            BackendSseEvent {
                event: "message".to_owned(),
                data: "hello".to_owned(),
            }
        );
        assert_eq!(parse_sse_frame(": keepalive\n"), None);
    }

    #[test]
    fn applies_sse_frame_to_session() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        let delta =
            apply_sse_frame(&mut session, "event: delta\ndata: hel\n").expect("expected delta");
        let meta = apply_sse_frame(&mut session, "event: meta\ndata: {\"tokens\":1}\n")
            .expect("expected meta");
        let done = apply_sse_frame(&mut session, "event: done\ndata: \n").expect("expected done");

        assert_eq!(delta.content, "hel");
        assert_eq!(meta.kind, ChatChunkKind::Metadata);
        assert_eq!(done.state, StreamState::Completed);
        assert_eq!(session.history().last().unwrap().content, "hel");
    }

    #[test]
    fn applies_sse_pressure_frame_to_session() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        let busy = apply_sse_frame(&mut session, "event: busy\ndata: quality worker busy\n")
            .expect("expected busy");

        assert_eq!(busy.state, StreamState::Busy);
        assert_eq!(busy.content, "quality worker busy");
    }

    #[test]
    fn sse_frame_buffer_returns_complete_frames_across_chunks() {
        let mut buffer = SseFrameBuffer::new();

        assert!(buffer.push("event: delta\ndata: hel").is_empty());
        let frames = buffer.push("lo\n\n");

        assert_eq!(frames, vec!["event: delta\ndata: hello".to_owned()]);
        assert_eq!(buffer.pending(), "");
    }

    #[test]
    fn sse_frame_buffer_returns_multiple_complete_frames_and_keeps_tail() {
        let mut buffer = SseFrameBuffer::new();

        let frames = buffer.push("event: delta\ndata: one\n\nevent: delta\ndata: two\n\nevent:");

        assert_eq!(
            frames,
            vec![
                "event: delta\ndata: one".to_owned(),
                "event: delta\ndata: two".to_owned(),
            ]
        );
        assert_eq!(buffer.pending(), "event:");
        assert_eq!(buffer.finish().as_deref(), Some("event:"));
    }

    #[test]
    fn sse_frame_buffer_supports_crlf_boundaries() {
        let mut buffer = SseFrameBuffer::new();

        let frames = buffer.push("event: meta\r\ndata: ok\r\n\r\n");

        assert_eq!(frames, vec!["event: meta\r\ndata: ok".to_owned()]);
    }

    #[test]
    fn sse_frame_buffer_applies_frames_to_session() {
        let mut buffer = SseFrameBuffer::new();
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        let first = buffer.apply_to_session(&mut session, "event: delta\ndata: he");
        let second = buffer.apply_to_session(
            &mut session,
            "llo\n\nevent: done\ndata: \n\ntrailing partial",
        );

        assert!(first.is_empty());
        assert_eq!(second.len(), 2);
        assert_eq!(second[0].content, "hello");
        assert_eq!(second[1].state, StreamState::Completed);
        assert_eq!(session.history().last().unwrap().content, "hello");
        assert!(buffer.finish().is_some());
    }

    #[test]
    fn backend_events_update_session_chunks_and_finish_history() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        let start = session.begin_stream();
        let delta = apply_backend_event(&mut session, "delta", "hello");
        let meta = apply_backend_event(&mut session, "meta", "{\"tokens\":1}");
        let final_payload = apply_backend_event(&mut session, "final", "{\"answer\":\"hello\"}");
        let done = apply_backend_event(&mut session, "done", "");

        assert_eq!(start.sequence, 0);
        assert_eq!(delta.sequence, 1);
        assert_eq!(meta.kind, ChatChunkKind::Metadata);
        assert_eq!(final_payload.kind, ChatChunkKind::Final);
        assert_eq!(done.state, StreamState::Completed);
        assert_eq!(session.partial_answer(), "hello");
        assert_eq!(session.history().last().unwrap().content, "hello");
    }

    #[test]
    fn backend_error_after_delta_interrupts_and_keeps_partial_answer() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.begin_stream();
        apply_backend_event(&mut session, "delta", "partial");
        let error = apply_backend_event(&mut session, "error", "backend stream closed");

        assert_eq!(error.state, StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.last_error(), Some("backend stream closed"));
        assert!(session.history().is_empty());
    }

    #[test]
    fn backend_error_before_delta_fails_without_partial_answer() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.begin_stream();
        let error = apply_backend_event(&mut session, "error", "backend rejected request");

        assert_eq!(error.state, StreamState::Failed);
        assert_eq!(session.partial_answer(), "");
        assert_eq!(session.last_error(), Some("backend rejected request"));
    }

    #[test]
    fn incomplete_stream_closes_as_interrupted_only_when_partial_exists() {
        let mut with_partial = ChatSession::new("partial", ChatSessionConfig::default());
        with_partial.begin_stream();
        apply_backend_event(&mut with_partial, "delta", "partial");

        let interrupted = close_incomplete_stream(&mut with_partial, "missing done")
            .expect("expected interrupted chunk");

        assert_eq!(interrupted.state, StreamState::Interrupted);
        assert_eq!(with_partial.partial_answer(), "partial");

        let mut without_partial = ChatSession::new("empty", ChatSessionConfig::default());
        without_partial.begin_stream();
        let failed = close_incomplete_stream(&mut without_partial, "missing done")
            .expect("expected failed chunk");

        assert_eq!(failed.state, StreamState::Failed);
    }

    #[test]
    fn incomplete_stream_after_pressure_closes_as_interrupted_with_pressure_reason() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());
        apply_backend_event(&mut session, "backpressure", "pool queue full");

        let interrupted = close_incomplete_stream(&mut session, "missing done")
            .expect("expected interrupted pressure close");

        assert_eq!(interrupted.state, StreamState::Interrupted);
        assert_eq!(
            interrupted.content,
            "missing done after backpressure: pool queue full"
        );
        assert_eq!(
            session.last_error(),
            Some("missing done after backpressure: pool queue full")
        );
        assert!(session.history().is_empty());
    }

    #[test]
    fn pressure_backend_events_map_to_pressure_stream_states() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        let queued = apply_backend_event(&mut session, "queued", "waiting");
        let busy = apply_backend_event(&mut session, "busy", "quality worker busy");
        let backpressure = apply_backend_event(&mut session, "backpressure", "queue full");

        assert_eq!(queued.state, StreamState::Queued);
        assert_eq!(busy.state, StreamState::Busy);
        assert_eq!(backpressure.state, StreamState::Backpressure);
    }

    #[test]
    fn status_and_heartbeat_frames_do_not_clear_pressure_gate() {
        for (event, reason, expected_state) in [
            ("queued", "waiting for matching worker", StreamState::Queued),
            ("busy", "quality worker busy", StreamState::Busy),
            (
                "backpressure",
                "matching route saturated",
                StreamState::Backpressure,
            ),
        ] {
            let mut session = ChatSession::new(event, ChatSessionConfig::default());
            apply_backend_event(&mut session, event, reason);

            let status = apply_backend_event(&mut session, "status", "still draining queue");
            let heartbeat = apply_backend_event(&mut session, "heartbeat", "worker alive");
            let blocked = session
                .prompt_blocked_chunk()
                .expect("pressure gate should remain active after status frames");

            assert_eq!(status.kind, ChatChunkKind::Status, "{event}");
            assert_eq!(heartbeat.kind, ChatChunkKind::Status, "{event}");
            assert_eq!(status.state, StreamState::Streaming, "{event}");
            assert_eq!(heartbeat.state, StreamState::Streaming, "{event}");
            assert_eq!(session.state(), expected_state, "{event}");
            assert_eq!(blocked.state, expected_state, "{event}");
            assert_eq!(blocked.content, reason, "{event}");
            assert!(!session.can_submit_prompt(), "{event}");
            assert!(session.history().is_empty(), "{event}");
        }
    }

    #[test]
    fn status_and_heartbeat_frames_do_not_pollute_stream_context() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        let delta = apply_backend_event(&mut session, "delta", "partial");
        let status = apply_backend_event(&mut session, "status", "warming decoder");
        let heartbeat = apply_backend_event(&mut session, "heartbeat", "worker alive");

        assert_eq!(delta.kind, ChatChunkKind::Delta);
        assert_eq!(status.kind, ChatChunkKind::Status);
        assert_eq!(heartbeat.kind, ChatChunkKind::Status);
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0], ChatMessage::user("hello"));
        assert!(
            !session
                .history()
                .iter()
                .any(|message| message.content == "warming decoder")
        );
        assert!(
            !session
                .history()
                .iter()
                .any(|message| message.content == "worker alive")
        );

        let done = apply_backend_event(&mut session, "done", "");
        let history_contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();

        assert_eq!(done.state, StreamState::Completed);
        assert_eq!(history_contents, vec!["hello", "partial"]);
    }

    #[test]
    fn backend_final_answer_reconciles_history_before_done() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        apply_backend_event(&mut session, "delta", "draft");
        let final_payload =
            apply_backend_final_answer(&mut session, "{\"answer\":\"polished\"}", Some("polished"));
        apply_backend_event(&mut session, "done", "");

        assert_eq!(final_payload.kind, ChatChunkKind::Final);
        assert_eq!(session.partial_answer(), "polished");
        assert_eq!(session.history().last().unwrap().content, "polished");
    }

    #[test]
    fn backend_final_without_extracted_answer_preserves_streamed_text() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.begin_stream();
        apply_backend_event(&mut session, "delta", "streamed");
        apply_backend_final_answer(&mut session, "{\"answer\":\"ignored by adapter\"}", None);
        apply_backend_event(&mut session, "done", "");

        assert_eq!(session.partial_answer(), "streamed");
        assert_eq!(session.history().last().unwrap().content, "streamed");
    }

    #[test]
    fn late_backend_events_after_done_do_not_pollute_completed_context() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        apply_backend_event(&mut session, "delta", "clean");
        apply_backend_event(&mut session, "done", "");
        let chunks_after_done = session.chunks().len();

        let late_delta = apply_backend_event(&mut session, "delta", " polluted");
        let late_error = apply_backend_event(&mut session, "error", "late error");

        assert_eq!(late_delta.state, StreamState::Completed);
        assert_eq!(late_error.state, StreamState::Completed);
        assert_eq!(session.state(), StreamState::Completed);
        assert_eq!(session.partial_answer(), "clean");
        assert_eq!(session.history().last().unwrap().content, "clean");
        assert_eq!(session.last_error(), None);
        assert_eq!(session.chunks().len(), chunks_after_done);
    }

    #[test]
    fn late_backend_done_after_interrupted_stream_is_ignored() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.begin_stream();
        apply_backend_event(&mut session, "delta", "partial");
        apply_backend_event(&mut session, "error", "backend closed");
        let late_done = apply_backend_event(&mut session, "done", "");

        assert_eq!(late_done.state, StreamState::Interrupted);
        assert_eq!(session.state(), StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.last_error(), Some("backend closed"));
        assert!(session.history().is_empty());
    }

    #[test]
    fn late_backend_frames_after_interrupted_stream_do_not_replace_partial() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        apply_backend_event(&mut session, "delta", "partial");
        apply_backend_event(&mut session, "error", "backend closed");
        let chunks_after_interrupt = session.chunks().len();

        let late_delta = apply_backend_event(&mut session, "delta", " polluted");
        let late_final =
            apply_backend_final_answer(&mut session, "{\"answer\":\"polished\"}", Some("polished"));
        let late_error = apply_backend_event(&mut session, "error", "late error");

        assert_eq!(late_delta.state, StreamState::Interrupted);
        assert_eq!(late_final.state, StreamState::Interrupted);
        assert_eq!(late_error.state, StreamState::Interrupted);
        assert_eq!(session.state(), StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.last_error(), Some("backend closed"));
        assert_eq!(session.chunks().len(), chunks_after_interrupt);
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "hello");
    }

    #[test]
    fn read_timeout_interrupt_does_not_pollute_retry_context() {
        let mut session = ChatSession::new(
            "s1",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );

        session.submit_prompt("hello");
        session.begin_stream();
        apply_backend_event(&mut session, "delta", "partial");
        let timeout = apply_backend_event(&mut session, "error", "read timeout");
        let chunks_after_timeout = session.chunks().len();

        let timeout_display = timeout.display_snapshot();
        assert_eq!(timeout.state, StreamState::Interrupted);
        assert_eq!(timeout.content, "read timeout");
        assert_eq!(timeout_display.output_label, "interrupted");
        assert_eq!(timeout_display.appended, "[interrupted] read timeout");
        assert!(timeout_display.state_is_terminal);
        assert!(!timeout_display.state_is_pressure);
        assert!(!timeout_display.state_blocks_prompt_submit);

        let late_done = apply_backend_event(&mut session, "done", "");
        let late_delta = apply_backend_event(&mut session, "delta", " late");
        let late_final =
            apply_backend_final_answer(&mut session, "{\"answer\":\"late\"}", Some("late"));

        assert_eq!(late_done.state, StreamState::Interrupted);
        assert_eq!(late_delta.state, StreamState::Interrupted);
        assert_eq!(late_final.state, StreamState::Interrupted);
        assert_eq!(session.state(), StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.last_error(), Some("read timeout"));
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "hello");
        assert_eq!(session.chunks().len(), chunks_after_timeout);

        let retry = session
            .try_submit_and_begin_stream("retry")
            .expect("timeout-interrupted stream should allow retry");
        let request_contents = retry
            .request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();

        assert_eq!(request_contents, vec!["hello", "retry"]);
        assert!(!request_contents.contains(&"partial"));
        assert!(!request_contents.contains(&"read timeout"));
        assert!(!request_contents.contains(&"late"));
        assert_eq!(retry.request.max_tokens, Some(4096));
        assert_eq!(retry.start.state, StreamState::Streaming);
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.partial_answer(), "");
        assert_eq!(session.last_error(), None);
    }
}
