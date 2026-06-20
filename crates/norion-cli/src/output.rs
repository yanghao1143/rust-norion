use norion_service::{
    ChatChunk, ChatChunkKind, ChatRequest, ChatRequestContextKind, GateAdvice, GateDecision,
    ModelEndpointSelectionKind, RoutingIntent, StartedChatTurn, StreamOutcome, StreamState,
};
pub use norion_service::{ChatChunkDisplaySnapshot, StreamOutcomeSnapshot};

use crate::input::{
    InputAction, InputActionKind, InputActionSnapshot, SessionConfigUpdate,
    SessionConfigUpdateSnapshot,
};
use crate::status::CliStatusSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollIntent {
    KeepPosition,
    FollowLatest,
    ScrollToBottom,
}

impl ScrollIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KeepPosition => "keep_position",
            Self::FollowLatest => "follow_latest",
            Self::ScrollToBottom => "scroll_to_bottom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputUpdateSource {
    StreamChunk,
    LocalStatus,
    GateAdvice,
}

impl OutputUpdateSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StreamChunk => "stream_chunk",
            Self::LocalStatus => "local_status",
            Self::GateAdvice => "gate_advice",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputUpdate {
    pub appended: String,
    pub output_label: String,
    pub source: OutputUpdateSource,
    pub source_label: String,
    pub is_stream_chunk: bool,
    pub is_local_status: bool,
    pub is_gate_advice: bool,
    pub is_pressure_stream_chunk: bool,
    pub is_terminal_stream_chunk: bool,
    pub state: StreamState,
    pub state_label: String,
    pub state_is_terminal: bool,
    pub state_is_pressure: bool,
    pub state_blocks_prompt_submit: bool,
    pub gate_advice_detail: Option<GateAdvice>,
    pub gate_advice_action_label: Option<String>,
    pub gate_advice_state_label: Option<String>,
    pub gate_advice_reason: Option<String>,
    pub request_preview: Option<RequestPreviewSnapshot>,
    pub route_update: Option<RouteUpdateSnapshot>,
    pub session_config_update_detail: Option<SessionConfigUpdateSnapshot>,
    pub status_snapshot: Option<CliStatusSnapshot>,
    pub stream_outcome: Option<StreamOutcomeSnapshot>,
    pub stream_chunk: Option<ChatChunkDisplaySnapshot>,
    pub input_action_snapshot: Option<InputActionSnapshot>,
    pub scroll: ScrollIntent,
    pub scroll_label: String,
}

impl OutputUpdate {
    fn new(
        appended: String,
        output_label: impl Into<String>,
        source: OutputUpdateSource,
        state: StreamState,
        scroll: ScrollIntent,
    ) -> Self {
        let is_stream_chunk = source == OutputUpdateSource::StreamChunk;
        Self {
            appended,
            output_label: output_label.into(),
            source,
            source_label: source.as_str().to_owned(),
            is_stream_chunk,
            is_local_status: source == OutputUpdateSource::LocalStatus,
            is_gate_advice: source == OutputUpdateSource::GateAdvice,
            is_pressure_stream_chunk: is_stream_chunk && state.is_pressure(),
            is_terminal_stream_chunk: is_stream_chunk && state.is_terminal(),
            state,
            state_label: state.as_str().to_owned(),
            state_is_terminal: state.is_terminal(),
            state_is_pressure: state.is_pressure(),
            state_blocks_prompt_submit: state.blocks_prompt_submit(),
            gate_advice_detail: None,
            gate_advice_action_label: None,
            gate_advice_state_label: None,
            gate_advice_reason: None,
            request_preview: None,
            route_update: None,
            session_config_update_detail: None,
            status_snapshot: None,
            stream_outcome: None,
            stream_chunk: None,
            input_action_snapshot: None,
            scroll,
            scroll_label: scroll.as_str().to_owned(),
        }
    }

    fn with_gate_advice(mut self, advice: GateAdvice) -> Self {
        self.gate_advice_action_label = Some(advice.action_label().to_owned());
        self.gate_advice_state_label = Some(advice.state_label().to_owned());
        self.gate_advice_reason = Some(advice.reason.clone());
        self.gate_advice_detail = Some(advice);
        self
    }

    fn with_request_preview(mut self, preview: RequestPreviewSnapshot) -> Self {
        self.request_preview = Some(preview);
        self
    }

    fn with_route_update(mut self, route: RouteUpdateSnapshot) -> Self {
        self.route_update = Some(route);
        self
    }

    fn with_session_config_update(mut self, update: SessionConfigUpdateSnapshot) -> Self {
        self.session_config_update_detail = Some(update);
        self
    }

    fn with_status_snapshot(mut self, status: CliStatusSnapshot) -> Self {
        self.status_snapshot = Some(status);
        self
    }

    fn with_stream_outcome(mut self, outcome: StreamOutcomeSnapshot) -> Self {
        self.stream_outcome = Some(outcome);
        self
    }

    fn with_stream_chunk(mut self, chunk: ChatChunkDisplaySnapshot) -> Self {
        self.output_label = chunk.output_label.clone();
        self.is_pressure_stream_chunk = chunk.state_is_pressure;
        self.is_terminal_stream_chunk = chunk.state_is_terminal;
        self.state = chunk.state;
        self.state_label = chunk.state_label.clone();
        self.state_is_terminal = chunk.state_is_terminal;
        self.state_is_pressure = chunk.state_is_pressure;
        self.state_blocks_prompt_submit = chunk.state_blocks_prompt_submit;
        self.stream_chunk = Some(chunk);
        self
    }

    fn with_input_action_snapshot(mut self, snapshot: InputActionSnapshot) -> Self {
        self.input_action_snapshot = Some(snapshot);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteUpdateSnapshot {
    pub routing_intent: RoutingIntent,
    pub route: String,
    pub model_role_label: String,
    pub routing_preference_label: String,
    pub endpoint_label: String,
    pub endpoint_pinned: bool,
    pub endpoint_kind: ModelEndpointSelectionKind,
    pub endpoint_kind_label: String,
    pub endpoint_auto: bool,
    pub endpoint_built_in: bool,
    pub endpoint_custom: bool,
    pub wire_model_role_label: String,
    pub wire_routing_preference_label: String,
    pub wire_prefer_fast: bool,
    pub wire_prefer_quality: bool,
    pub wire_endpoint_pinned: bool,
    pub wire_endpoint_kind_label: String,
    pub wire_sends_model_endpoint: bool,
    pub wire_model_endpoint_label: Option<String>,
}

impl RouteUpdateSnapshot {
    pub fn from_intent(intent: &RoutingIntent) -> Self {
        let endpoint_kind = intent.endpoint_kind();
        let wire = intent.wire_snapshot();
        Self {
            route: intent.summary(),
            model_role_label: intent.model_role_label().to_owned(),
            routing_preference_label: intent.routing_preference_label().to_owned(),
            endpoint_label: intent.endpoint_label().to_owned(),
            endpoint_pinned: intent.endpoint_pinned,
            endpoint_kind,
            endpoint_kind_label: intent.endpoint_kind_label().to_owned(),
            endpoint_auto: intent.endpoint_auto(),
            endpoint_built_in: intent.endpoint_built_in(),
            endpoint_custom: intent.endpoint_custom(),
            wire_model_role_label: wire.model_role_label,
            wire_routing_preference_label: wire.routing_preference_label,
            wire_prefer_fast: wire.prefer_fast,
            wire_prefer_quality: wire.prefer_quality,
            wire_endpoint_pinned: wire.endpoint_pinned,
            wire_endpoint_kind_label: wire.endpoint_kind_label,
            wire_sends_model_endpoint: wire.sends_model_endpoint,
            wire_model_endpoint_label: wire.model_endpoint_label,
            routing_intent: intent.clone(),
        }
    }

    pub fn line(&self) -> &str {
        &self.route
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestPreviewSnapshot {
    pub routing_intent: RoutingIntent,
    pub model_role_label: String,
    pub routing_preference_label: String,
    pub endpoint_label: String,
    pub endpoint_pinned: bool,
    pub endpoint_kind: ModelEndpointSelectionKind,
    pub endpoint_kind_label: String,
    pub endpoint_auto: bool,
    pub endpoint_built_in: bool,
    pub endpoint_custom: bool,
    pub wire_model_role_label: String,
    pub wire_routing_preference_label: String,
    pub wire_prefer_fast: bool,
    pub wire_prefer_quality: bool,
    pub wire_sends_max_tokens: bool,
    pub wire_max_tokens: Option<usize>,
    pub wire_endpoint_pinned: bool,
    pub wire_endpoint_kind_label: String,
    pub wire_sends_model_endpoint: bool,
    pub wire_model_endpoint_label: Option<String>,
    pub messages: usize,
    pub context_messages: usize,
    pub history_messages: usize,
    pub history_limit: Option<usize>,
    pub history_remaining: Option<usize>,
    pub history_messages_after_submit: Option<usize>,
    pub history_at_limit_after_submit: Option<bool>,
    pub history_truncates_on_submit: Option<bool>,
    pub context_kind: ChatRequestContextKind,
    pub context_kind_label: String,
    pub has_context: bool,
    pub is_single_turn: bool,
    pub last_message_role_label: Option<String>,
    pub last_message_chars: usize,
    pub last_message_is_user: bool,
    pub last_user_chars: usize,
    pub max_tokens: Option<usize>,
    pub max_tokens_label: String,
    pub stream: bool,
    pub start_sequence: Option<u64>,
    pub start_state: Option<StreamState>,
    pub start_state_label: Option<String>,
    pub start_state_is_terminal: Option<bool>,
    pub start_state_is_pressure: Option<bool>,
    pub start_state_blocks_prompt_submit: Option<bool>,
    pub start_chunk: Option<ChatChunkDisplaySnapshot>,
}

impl RequestPreviewSnapshot {
    pub fn from_request(request: &ChatRequest) -> Self {
        Self::from_request_with_history_limit(request, None)
    }

    pub fn from_request_with_history_limit(
        request: &ChatRequest,
        history_limit: Option<usize>,
    ) -> Self {
        let submission = request.submission_snapshot_with_history_limit(history_limit);

        Self {
            model_role_label: submission.model_role_label,
            routing_preference_label: submission.routing_preference_label,
            endpoint_label: submission.endpoint_label,
            endpoint_pinned: submission.endpoint_pinned,
            endpoint_kind: submission.endpoint_kind,
            endpoint_kind_label: submission.endpoint_kind_label,
            endpoint_auto: submission.endpoint_auto,
            endpoint_built_in: submission.endpoint_built_in,
            endpoint_custom: submission.endpoint_custom,
            wire_model_role_label: submission.wire_model_role_label,
            wire_routing_preference_label: submission.wire_routing_preference_label,
            wire_prefer_fast: submission.wire_prefer_fast,
            wire_prefer_quality: submission.wire_prefer_quality,
            wire_sends_max_tokens: submission.wire_sends_max_tokens,
            wire_max_tokens: submission.wire_max_tokens,
            wire_endpoint_pinned: submission.wire_endpoint_pinned,
            wire_endpoint_kind_label: submission.wire_endpoint_kind_label,
            wire_sends_model_endpoint: submission.wire_sends_model_endpoint,
            wire_model_endpoint_label: submission.wire_model_endpoint_label,
            routing_intent: submission.routing_intent,
            messages: submission.messages,
            context_messages: submission.context_messages,
            history_messages: submission.history_messages,
            history_limit: submission.history_limit,
            history_remaining: submission.history_remaining,
            history_messages_after_submit: submission.history_messages_after_submit,
            history_at_limit_after_submit: submission.history_at_limit_after_submit,
            history_truncates_on_submit: submission.history_truncates_on_submit,
            context_kind: submission.context_kind,
            context_kind_label: submission.context_kind_label,
            has_context: submission.has_context,
            is_single_turn: submission.is_single_turn,
            last_message_role_label: submission.last_message_role_label,
            last_message_chars: submission.last_message_chars,
            last_message_is_user: submission.last_message_is_user,
            last_user_chars: submission.last_user_chars,
            max_tokens: submission.max_tokens,
            max_tokens_label: submission.max_tokens_label,
            stream: submission.stream,
            start_sequence: None,
            start_state: None,
            start_state_label: None,
            start_state_is_terminal: None,
            start_state_is_pressure: None,
            start_state_blocks_prompt_submit: None,
            start_chunk: None,
        }
    }

    pub fn from_started_turn(turn: &StartedChatTurn) -> Self {
        Self::from_started_turn_with_history_limit(turn, None)
    }

    pub fn from_started_turn_with_history_limit(
        turn: &StartedChatTurn,
        history_limit: Option<usize>,
    ) -> Self {
        let start_chunk = turn.start.display_snapshot();
        Self {
            start_sequence: Some(turn.start.sequence),
            start_state: Some(turn.start.state),
            start_state_label: Some(start_chunk.state_label.clone()),
            start_state_is_terminal: Some(start_chunk.state_is_terminal),
            start_state_is_pressure: Some(start_chunk.state_is_pressure),
            start_state_blocks_prompt_submit: Some(start_chunk.state_blocks_prompt_submit),
            start_chunk: Some(start_chunk),
            ..Self::from_request_with_history_limit(&turn.request, history_limit)
        }
    }

    pub fn line(&self) -> String {
        let mut line = format!(
            "{} messages={} last_user_chars={} max_tokens={} stream={}",
            self.routing_intent.summary(),
            self.messages,
            self.last_user_chars,
            self.max_tokens_label,
            self.stream
        );
        if let Some(start_sequence) = self.start_sequence {
            line.push_str(&format!(" start_sequence={start_sequence}"));
        }
        if let Some(start_state) = self.start_state {
            line.push_str(&format!(" start_state={}", start_state.as_str()));
        }
        line
    }
}

#[derive(Debug, Clone)]
pub struct OutputViewport {
    lines: Vec<String>,
    follow_latest: bool,
}

impl OutputViewport {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            follow_latest: true,
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn set_follow_latest(&mut self, follow_latest: bool) {
        self.follow_latest = follow_latest;
    }

    pub fn append_chunk(&mut self, chunk: &ChatChunk) -> OutputUpdate {
        let snapshot = chunk.display_snapshot();
        let appended = snapshot.appended.clone();
        if !appended.is_empty() {
            self.lines.push(appended.clone());
        }
        let scroll = if snapshot.state_is_terminal {
            ScrollIntent::ScrollToBottom
        } else if self.follow_latest {
            ScrollIntent::FollowLatest
        } else {
            ScrollIntent::KeepPosition
        };
        OutputUpdate::new(
            appended,
            snapshot.output_label.clone(),
            OutputUpdateSource::StreamChunk,
            snapshot.state,
            scroll,
        )
        .with_stream_chunk(snapshot)
    }

    pub fn append_gate_decision(&mut self, decision: &GateDecision) -> Option<OutputUpdate> {
        let chunk = decision.to_chunk(0)?;
        Some(self.append_chunk(&chunk))
    }

    pub fn append_input_action(&mut self, action: &InputAction) -> Option<OutputUpdate> {
        match action {
            InputAction::Blocked(chunk) | InputAction::StreamCancelled(chunk) => {
                Some(self.append_chunk(chunk))
            }
            InputAction::InputError(reason) => Some(self.append_input_error(reason)),
            InputAction::RoutingChanged(summary) => {
                Some(self.append_local_status("route", summary))
            }
            InputAction::Status(line) => Some(self.append_local_status("status", line)),
            InputAction::SessionConfigChanged { update, .. } => {
                Some(self.append_session_config_update(update))
            }
            InputAction::BufferChanged
            | InputAction::Send(_)
            | InputAction::StartStream(_)
            | InputAction::InsertNewline
            | InputAction::CancelStream
            | InputAction::Quit
            | InputAction::Noop => None,
        }
    }

    pub fn append_input_action_snapshot(
        &mut self,
        snapshot: &InputActionSnapshot,
    ) -> Option<OutputUpdate> {
        match snapshot.kind {
            InputActionKind::Blocked | InputActionKind::StreamCancelled => Some(
                self.append_chunk(&chunk_from_input_snapshot(snapshot)?)
                    .with_input_action_snapshot(snapshot.clone()),
            ),
            InputActionKind::InputError => Some(
                self.append_input_error(snapshot.local_status.as_ref()?)
                    .with_input_action_snapshot(snapshot.clone()),
            ),
            InputActionKind::RoutingChanged => Some(
                self.append_route_update(&snapshot.routing_intent)
                    .with_input_action_snapshot(snapshot.clone()),
            ),
            InputActionKind::Status => Some(
                self.append_local_status("status", snapshot.local_status.as_ref()?)
                    .with_input_action_snapshot(snapshot.clone()),
            ),
            InputActionKind::SessionConfigChanged => {
                let detail = snapshot.session_config_update_detail.clone()?;
                Some(
                    self.append_local_status("config", &detail.summary)
                        .with_session_config_update(detail)
                        .with_input_action_snapshot(snapshot.clone()),
                )
            }
            InputActionKind::BufferChanged
            | InputActionKind::Send
            | InputActionKind::StartStream
            | InputActionKind::InsertNewline
            | InputActionKind::CancelStream
            | InputActionKind::Quit
            | InputActionKind::Noop => None,
        }
    }

    pub fn append_gate_advice(&mut self, advice: &GateAdvice) -> OutputUpdate {
        let appended = format!("[advice] {}", advice.status_line());
        self.lines.push(appended.clone());
        let scroll = if self.follow_latest {
            ScrollIntent::FollowLatest
        } else {
            ScrollIntent::KeepPosition
        };
        OutputUpdate::new(
            appended,
            "advice",
            OutputUpdateSource::GateAdvice,
            advice.state,
            scroll,
        )
        .with_gate_advice(advice.clone())
    }

    pub fn append_request_preview(&mut self, request: &ChatRequest) -> OutputUpdate {
        let preview = RequestPreviewSnapshot::from_request(request);
        self.append_local_status("send", preview.line())
            .with_request_preview(preview)
    }

    pub fn append_request_preview_with_history_limit(
        &mut self,
        request: &ChatRequest,
        history_limit: Option<usize>,
    ) -> OutputUpdate {
        let preview =
            RequestPreviewSnapshot::from_request_with_history_limit(request, history_limit);
        self.append_local_status("send", preview.line())
            .with_request_preview(preview)
    }

    pub fn append_started_turn_preview(&mut self, turn: &StartedChatTurn) -> OutputUpdate {
        let preview = RequestPreviewSnapshot::from_started_turn(turn);
        self.append_local_status("send", preview.line())
            .with_request_preview(preview)
    }

    pub fn append_started_turn_preview_with_history_limit(
        &mut self,
        turn: &StartedChatTurn,
        history_limit: Option<usize>,
    ) -> OutputUpdate {
        let preview =
            RequestPreviewSnapshot::from_started_turn_with_history_limit(turn, history_limit);
        self.append_local_status("send", preview.line())
            .with_request_preview(preview)
    }

    pub fn append_route_update(&mut self, intent: &RoutingIntent) -> OutputUpdate {
        let route = RouteUpdateSnapshot::from_intent(intent);
        let line = route.line().to_owned();
        self.append_local_status("route", line)
            .with_route_update(route)
    }

    pub fn append_status_snapshot(&mut self, status: &CliStatusSnapshot) -> OutputUpdate {
        self.append_local_status("status", status.line())
            .with_status_snapshot(status.clone())
    }

    pub fn append_workers_snapshot(&mut self, status: &CliStatusSnapshot) -> Option<OutputUpdate> {
        Some(
            self.append_local_status("workers", status.workers_line()?)
                .with_status_snapshot(status.clone()),
        )
    }

    pub fn append_stream_outcome(&mut self, outcome: &StreamOutcome) -> OutputUpdate {
        let snapshot = outcome.snapshot();
        let line = snapshot.line();
        self.append_local_status("outcome", line)
            .with_stream_outcome(snapshot)
    }

    pub fn append_input_error(&mut self, reason: impl Into<String>) -> OutputUpdate {
        self.append_local_status("input", reason)
    }

    pub fn append_session_config_update(&mut self, update: &SessionConfigUpdate) -> OutputUpdate {
        let detail = update.snapshot();
        self.append_local_status("config", &detail.summary)
            .with_session_config_update(detail)
    }

    fn append_local_status(&mut self, label: &str, content: impl Into<String>) -> OutputUpdate {
        let appended = format!("[{label}] {}", content.into());
        self.lines.push(appended.clone());
        let scroll = if self.follow_latest {
            ScrollIntent::FollowLatest
        } else {
            ScrollIntent::KeepPosition
        };
        OutputUpdate::new(
            appended,
            label,
            OutputUpdateSource::LocalStatus,
            StreamState::Pending,
            scroll,
        )
    }
}

fn chunk_from_input_snapshot(snapshot: &InputActionSnapshot) -> Option<ChatChunk> {
    let state = snapshot.stream_state?;
    let reason = snapshot.reason.clone().unwrap_or_default();
    Some(match state {
        StreamState::Queued => ChatChunk::queued(0, reason),
        StreamState::Busy => ChatChunk::busy(0, reason),
        StreamState::Backpressure => ChatChunk::backpressure(0, reason),
        StreamState::Interrupted => ChatChunk::interrupted(0, reason),
        StreamState::Failed => ChatChunk::failed(0, reason),
        StreamState::Streaming => ChatChunk::status(0, reason),
        StreamState::Completed => ChatChunk::done(0),
        StreamState::Pending => {
            ChatChunk::new(0, StreamState::Pending, ChatChunkKind::Status, reason)
        }
    })
}

pub fn outcome_status(outcome: &StreamOutcome) -> String {
    outcome.snapshot().line()
}

pub fn gate_advice_status(decision: &GateDecision) -> String {
    decision.advice().status_line()
}

pub fn session_config_status(update: &SessionConfigUpdate) -> String {
    update.summary()
}

pub fn request_preview_status(request: &ChatRequest) -> String {
    RequestPreviewSnapshot::from_request(request).line()
}

pub fn request_preview_status_with_history_limit(
    request: &ChatRequest,
    history_limit: Option<usize>,
) -> String {
    RequestPreviewSnapshot::from_request_with_history_limit(request, history_limit).line()
}

pub fn route_update_status(intent: &RoutingIntent) -> String {
    RouteUpdateSnapshot::from_intent(intent).route
}

pub fn started_turn_preview_status(turn: &StartedChatTurn) -> String {
    RequestPreviewSnapshot::from_started_turn(turn).line()
}

pub fn started_turn_preview_status_with_history_limit(
    turn: &StartedChatTurn,
    history_limit: Option<usize>,
) -> String {
    RequestPreviewSnapshot::from_started_turn_with_history_limit(turn, history_limit).line()
}

impl Default for OutputViewport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use norion_service::{
        ChatChunk, ChatMessage, ChatRequest, ChatSession, ChatSessionConfig, FrontendGateSnapshot,
        GateAdviceAction, ModelEndpoint, ModelPoolGateSnapshot, ModelRole,
        ModelRouteWorkerPickerAction, ModelWorkerSnapshot, RoutingPreference,
    };

    use crate::input::CliInputConfig;

    #[test]
    fn appended_output_expresses_follow_latest_scroll() {
        let mut viewport = OutputViewport::new();
        let update = viewport.append_chunk(&ChatChunk::delta(0, "hello"));

        assert_eq!(viewport.lines(), &["hello".to_owned()]);
        assert_eq!(update.output_label, "delta");
        assert_eq!(update.source, OutputUpdateSource::StreamChunk);
        assert_eq!(update.source_label, "stream_chunk");
        assert!(update.is_stream_chunk);
        assert!(!update.is_local_status);
        assert!(!update.is_gate_advice);
        assert!(!update.is_pressure_stream_chunk);
        assert!(!update.is_terminal_stream_chunk);
        assert_eq!(update.scroll, ScrollIntent::FollowLatest);
        assert_eq!(update.scroll_label, "follow_latest");
        assert_eq!(update.state_label, "streaming");
        assert!(!update.state_is_terminal);
        assert!(!update.state_is_pressure);
        assert!(update.state_blocks_prompt_submit);
        assert_eq!(update.gate_advice_detail, None);
        assert_eq!(update.gate_advice_action_label, None);
        assert_eq!(update.gate_advice_state_label, None);
        assert_eq!(update.gate_advice_reason, None);
        assert_eq!(update.request_preview, None);
        assert_eq!(update.session_config_update_detail, None);
        assert_eq!(update.status_snapshot, None);
        let chunk = update
            .stream_chunk
            .as_ref()
            .expect("stream output should carry service chunk display snapshot");
        assert_eq!(chunk.output_label, "delta");
        assert_eq!(chunk.appended, "hello");
        assert!(chunk.is_delta);
        assert!(chunk.state_blocks_prompt_submit);
    }

    #[test]
    fn terminal_output_requests_scroll_to_bottom() {
        let mut viewport = OutputViewport::new();
        let update = viewport.append_chunk(&ChatChunk::interrupted(1, "timeout"));

        assert_eq!(update.state, StreamState::Interrupted);
        assert_eq!(update.output_label, "interrupted");
        assert_eq!(update.source, OutputUpdateSource::StreamChunk);
        assert!(update.is_stream_chunk);
        assert!(update.is_terminal_stream_chunk);
        assert!(!update.is_pressure_stream_chunk);
        assert_eq!(update.state_label, "interrupted");
        assert!(update.state_is_terminal);
        assert!(!update.state_is_pressure);
        assert!(!update.state_blocks_prompt_submit);
        assert_eq!(update.scroll, ScrollIntent::ScrollToBottom);
        assert_eq!(update.scroll_label, "scroll_to_bottom");
        let chunk = update
            .stream_chunk
            .as_ref()
            .expect("terminal stream output should carry service chunk display snapshot");
        assert_eq!(chunk.kind_label, "error");
        assert_eq!(chunk.output_label, "interrupted");
        assert_eq!(chunk.appended, "[interrupted] timeout");
        assert!(chunk.state_is_terminal);
    }

    #[test]
    fn manual_scroll_position_is_preserved_until_terminal_output() {
        let mut viewport = OutputViewport::new();
        viewport.set_follow_latest(false);

        let delta = viewport.append_chunk(&ChatChunk::delta(0, "partial"));
        let status = viewport.append_input_action(&InputAction::Status(
            "role=assistant preference=balanced endpoint=auto pinned=false state=streaming"
                .to_owned(),
        ));
        let advice = viewport.append_gate_advice(
            &GateDecision::blocked(StreamState::Queued, "waiting for worker").advice(),
        );
        let terminal = viewport.append_chunk(&ChatChunk::interrupted(2, "backend stream closed"));

        assert_eq!(delta.scroll, ScrollIntent::KeepPosition);
        assert_eq!(delta.output_label, "delta");
        assert_eq!(delta.scroll_label, "keep_position");
        assert_eq!(status.unwrap().scroll, ScrollIntent::KeepPosition);
        assert_eq!(advice.scroll, ScrollIntent::KeepPosition);
        assert_eq!(advice.output_label, "advice");
        assert_eq!(advice.scroll_label, "keep_position");
        assert_eq!(terminal.scroll, ScrollIntent::ScrollToBottom);
        assert_eq!(terminal.output_label, "interrupted");
        assert_eq!(terminal.scroll_label, "scroll_to_bottom");
    }

    #[test]
    fn pressure_states_are_labeled_for_terminal_output() {
        let mut viewport = OutputViewport::new();

        viewport.append_chunk(&ChatChunk::queued(0, "waiting for reviewer"));
        viewport.append_chunk(&ChatChunk::busy(1, "quality worker busy"));
        viewport.append_chunk(&ChatChunk::backpressure(2, "queue saturated"));

        assert_eq!(
            viewport.lines(),
            &[
                "[queued] waiting for reviewer".to_owned(),
                "[busy] quality worker busy".to_owned(),
                "[backpressure] queue saturated".to_owned(),
            ]
        );
    }

    #[test]
    fn blocked_gate_decision_renders_through_terminal_output() {
        let mut viewport = OutputViewport::new();
        let decision = GateDecision::blocked(StreamState::Busy, "backend engine is busy");

        let update = viewport.append_gate_decision(&decision).unwrap();

        assert_eq!(update.appended, "[busy] backend engine is busy");
        assert_eq!(update.output_label, "busy");
        assert_eq!(update.state, StreamState::Busy);
        assert_eq!(
            viewport.lines(),
            &["[busy] backend engine is busy".to_owned()]
        );
        assert!(
            viewport
                .append_gate_decision(&GateDecision::Allowed)
                .is_none()
        );
    }

    #[test]
    fn pressure_gate_decisions_render_with_specific_terminal_labels() {
        let mut viewport = OutputViewport::new();
        let queued = GateDecision::blocked(StreamState::Queued, "waiting for reviewer worker");
        let backpressure = GateDecision::blocked(StreamState::Backpressure, "pool queue full");

        let queued_update = viewport.append_gate_decision(&queued).unwrap();
        let backpressure_update = viewport.append_gate_decision(&backpressure).unwrap();

        assert_eq!(
            queued_update.appended,
            "[queued] waiting for reviewer worker"
        );
        assert_eq!(queued_update.output_label, "queued");
        assert_eq!(queued_update.state, StreamState::Queued);
        assert_eq!(
            backpressure_update.appended,
            "[backpressure] pool queue full"
        );
        assert_eq!(backpressure_update.output_label, "backpressure");
        assert_eq!(backpressure_update.state, StreamState::Backpressure);
        assert_eq!(
            viewport.lines(),
            &[
                "[queued] waiting for reviewer worker".to_owned(),
                "[backpressure] pool queue full".to_owned(),
            ]
        );
    }

    #[test]
    fn gate_advice_renders_stable_wait_and_retry_status() {
        let mut viewport = OutputViewport::new();
        let decision = GateDecision::blocked(StreamState::Backpressure, "queue full");
        let advice = decision.advice();

        let update = viewport.append_gate_advice(&advice);

        assert_eq!(
            gate_advice_status(&decision),
            "retry_later backpressure: queue full"
        );
        assert_eq!(
            update.appended,
            "[advice] retry_later backpressure: queue full"
        );
        assert_eq!(update.output_label, "advice");
        assert_eq!(update.source, OutputUpdateSource::GateAdvice);
        assert_eq!(update.source_label, "gate_advice");
        assert!(!update.is_stream_chunk);
        assert!(!update.is_local_status);
        assert!(update.is_gate_advice);
        assert!(!update.is_pressure_stream_chunk);
        assert!(!update.is_terminal_stream_chunk);
        assert_eq!(update.state, StreamState::Backpressure);
        let advice = update
            .gate_advice_detail
            .as_ref()
            .expect("gate advice output should carry structured advice");
        assert_eq!(advice.action, GateAdviceAction::RetryLater);
        assert_eq!(advice.state, StreamState::Backpressure);
        assert_eq!(advice.reason, "queue full");
        assert_eq!(
            update.gate_advice_action_label.as_deref(),
            Some("retry_later")
        );
        assert_eq!(
            update.gate_advice_state_label.as_deref(),
            Some("backpressure")
        );
        assert_eq!(update.gate_advice_reason.as_deref(), Some("queue full"));
        assert_eq!(update.request_preview, None);
        assert_eq!(update.session_config_update_detail, None);
        assert_eq!(update.status_snapshot, None);
        assert_eq!(
            viewport.lines(),
            &["[advice] retry_later backpressure: queue full".to_owned()]
        );
    }

    #[test]
    fn gate_advice_renders_wait_busy_and_repair_actions() {
        let mut viewport = OutputViewport::new();
        let queued = GateDecision::blocked(StreamState::Queued, "waiting for reviewer").advice();
        let busy = GateDecision::blocked(StreamState::Busy, "backend engine is busy").advice();
        let failed = GateDecision::blocked(StreamState::Failed, "safe-device gate failed").advice();

        let queued_update = viewport.append_gate_advice(&queued);
        let busy_update = viewport.append_gate_advice(&busy);
        let failed_update = viewport.append_gate_advice(&failed);

        assert_eq!(
            queued_update.appended,
            "[advice] wait_for_worker queued: waiting for reviewer"
        );
        assert_eq!(queued_update.state, StreamState::Queued);
        assert_eq!(
            queued_update
                .gate_advice_detail
                .as_ref()
                .map(|advice| advice.action),
            Some(GateAdviceAction::WaitForWorker)
        );
        assert_eq!(
            queued_update.gate_advice_action_label.as_deref(),
            Some("wait_for_worker")
        );
        assert_eq!(
            queued_update.gate_advice_state_label.as_deref(),
            Some("queued")
        );
        assert_eq!(
            queued_update.gate_advice_reason.as_deref(),
            Some("waiting for reviewer")
        );
        assert_eq!(
            busy_update.appended,
            "[advice] wait_for_current_stream busy: backend engine is busy"
        );
        assert_eq!(busy_update.state, StreamState::Busy);
        assert_eq!(
            busy_update
                .gate_advice_detail
                .as_ref()
                .map(|advice| advice.action),
            Some(GateAdviceAction::WaitForCurrentStream)
        );
        assert_eq!(
            busy_update.gate_advice_action_label.as_deref(),
            Some("wait_for_current_stream")
        );
        assert_eq!(
            busy_update.gate_advice_reason.as_deref(),
            Some("backend engine is busy")
        );
        assert_eq!(
            failed_update.appended,
            "[advice] repair_gate failed: safe-device gate failed"
        );
        assert_eq!(failed_update.state, StreamState::Failed);
        assert_eq!(
            failed_update
                .gate_advice_detail
                .as_ref()
                .map(|advice| advice.action),
            Some(GateAdviceAction::RepairGate)
        );
        assert_eq!(
            failed_update.gate_advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(
            failed_update.gate_advice_state_label.as_deref(),
            Some("failed")
        );
        assert_eq!(
            failed_update.gate_advice_reason.as_deref(),
            Some("safe-device gate failed")
        );
        assert_eq!(
            viewport.lines(),
            &[
                "[advice] wait_for_worker queued: waiting for reviewer".to_owned(),
                "[advice] wait_for_current_stream busy: backend engine is busy".to_owned(),
                "[advice] repair_gate failed: safe-device gate failed".to_owned(),
            ]
        );
    }

    #[test]
    fn input_errors_render_without_backend_stream_state() {
        let mut viewport = OutputViewport::new();

        let update = viewport.append_input_error("unknown slash command: /workerz");

        assert_eq!(update.appended, "[input] unknown slash command: /workerz");
        assert_eq!(update.output_label, "input");
        assert_eq!(update.source, OutputUpdateSource::LocalStatus);
        assert_eq!(update.source_label, "local_status");
        assert!(!update.is_stream_chunk);
        assert!(update.is_local_status);
        assert!(!update.is_gate_advice);
        assert!(!update.is_pressure_stream_chunk);
        assert!(!update.is_terminal_stream_chunk);
        assert_eq!(update.state, StreamState::Pending);
        assert_eq!(update.gate_advice_detail, None);
        assert_eq!(update.gate_advice_action_label, None);
        assert_eq!(update.gate_advice_state_label, None);
        assert_eq!(update.gate_advice_reason, None);
        assert_eq!(update.request_preview, None);
        assert_eq!(update.session_config_update_detail, None);
        assert_eq!(update.status_snapshot, None);
        assert_eq!(update.scroll, ScrollIntent::FollowLatest);
        assert_eq!(
            viewport.lines(),
            &["[input] unknown slash command: /workerz".to_owned()]
        );
    }

    #[test]
    fn session_config_updates_render_as_local_terminal_status() {
        let mut viewport = OutputViewport::new();
        let update = SessionConfigUpdate::DefaultMaxTokens(Some(8192));

        let output = viewport.append_session_config_update(&update);

        assert_eq!(session_config_status(&update), "max_tokens=8192");
        assert_eq!(output.appended, "[config] max_tokens=8192");
        assert_eq!(output.output_label, "config");
        assert_eq!(output.source, OutputUpdateSource::LocalStatus);
        assert!(output.is_local_status);
        assert_eq!(output.gate_advice_detail, None);
        assert_eq!(output.request_preview, None);
        assert_eq!(output.state, StreamState::Pending);
        assert_eq!(output.scroll, ScrollIntent::FollowLatest);
        let detail = output
            .session_config_update_detail
            .as_ref()
            .expect("config output should carry structured session config update");
        assert_eq!(detail.update, update);
        assert_eq!(detail.kind_label, "max_tokens");
        assert_eq!(detail.summary, "max_tokens=8192");
        assert!(detail.changes_max_tokens);
        assert!(!detail.changes_history_limit);
        assert_eq!(detail.max_tokens, Some(8192));
        assert_eq!(detail.max_tokens_label.as_deref(), Some("8192"));
        assert!(!detail.max_tokens_backend_default);
        assert_eq!(detail.history_limit, None);
        assert_eq!(viewport.lines(), &["[config] max_tokens=8192".to_owned()]);
    }

    #[test]
    fn session_config_output_exposes_history_limit_snapshot() {
        let mut viewport = OutputViewport::new();
        let update = SessionConfigUpdate::HistoryLimit(32);

        let output = viewport.append_session_config_update(&update);

        assert_eq!(output.appended, "[config] history_limit=32");
        assert_eq!(output.output_label, "config");
        assert_eq!(output.source_label, "local_status");
        assert_eq!(output.request_preview, None);
        assert_eq!(output.gate_advice_detail, None);
        let detail = output
            .session_config_update_detail
            .as_ref()
            .expect("history config output should carry structured update");
        assert_eq!(detail.update, update);
        assert_eq!(detail.kind_label, "history_limit");
        assert_eq!(detail.summary, "history_limit=32");
        assert!(!detail.changes_max_tokens);
        assert!(detail.changes_history_limit);
        assert_eq!(detail.max_tokens, None);
        assert_eq!(detail.max_tokens_label, None);
        assert!(!detail.max_tokens_backend_default);
        assert_eq!(detail.history_limit, Some(32));
    }

    #[test]
    fn visible_input_actions_render_through_viewport_helper() {
        let mut viewport = OutputViewport::new();

        let blocked = viewport
            .append_input_action(&InputAction::Blocked(ChatChunk::busy(0, "worker busy")))
            .expect("blocked should render");
        let cancelled = viewport
            .append_input_action(&InputAction::StreamCancelled(ChatChunk::interrupted(
                1,
                "stream cancelled by user",
            )))
            .expect("cancel should render");
        let route = viewport
            .append_input_action(&InputAction::RoutingChanged(
                "role=reviewer preference=prefer_fast endpoint=auto pinned=false".to_owned(),
            ))
            .expect("route should render");
        let status = viewport
            .append_input_action(&InputAction::Status(
                "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending"
                    .to_owned(),
            ))
            .expect("status should render");
        let config = viewport
            .append_input_action(&InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::HistoryLimit(16),
                summary: "history_limit=16".to_owned(),
            })
            .expect("config should render");

        assert_eq!(blocked.appended, "[busy] worker busy");
        assert_eq!(cancelled.appended, "[interrupted] stream cancelled by user");
        assert_eq!(cancelled.output_label, "interrupted");
        assert_eq!(cancelled.source, OutputUpdateSource::StreamChunk);
        assert_eq!(cancelled.state, StreamState::Interrupted);
        assert!(cancelled.state_is_terminal);
        assert!(!cancelled.state_is_pressure);
        assert!(!cancelled.state_blocks_prompt_submit);
        assert_eq!(cancelled.request_preview, None);
        assert_eq!(cancelled.route_update, None);
        assert_eq!(cancelled.session_config_update_detail, None);
        assert_eq!(cancelled.status_snapshot, None);
        assert_eq!(
            route.appended,
            "[route] role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
        assert_eq!(
            status.appended,
            "[status] role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending"
        );
        assert_eq!(config.appended, "[config] history_limit=16");
        let config_detail = config
            .session_config_update_detail
            .as_ref()
            .expect("input action config output should expose structured update");
        assert_eq!(config_detail.kind_label, "history_limit");
        assert_eq!(config_detail.history_limit, Some(16));
        assert!(
            viewport
                .append_input_action(&InputAction::CancelStream)
                .is_none()
        );
    }

    #[test]
    fn input_action_snapshot_output_preserves_structured_local_actions() {
        let route_config = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let route_action = InputAction::RoutingChanged(route_config.routing_summary());
        let route_snapshot = route_action.snapshot(&route_config);
        let config_action = InputAction::SessionConfigChanged {
            update: SessionConfigUpdate::HistoryLimit(24),
            summary: "history_limit=24".to_owned(),
        };
        let config_snapshot = config_action.snapshot(&route_config);
        let pressure_snapshot = InputAction::Blocked(ChatChunk::backpressure(7, "pool queue full"))
            .snapshot(&route_config);
        let mut viewport = OutputViewport::new();

        let route_output = viewport
            .append_input_action_snapshot(&route_snapshot)
            .expect("route snapshot should render");
        let config_output = viewport
            .append_input_action_snapshot(&config_snapshot)
            .expect("config snapshot should render");
        let pressure_output = viewport
            .append_input_action_snapshot(&pressure_snapshot)
            .expect("pressure snapshot should render");

        assert_eq!(
            route_output.appended,
            "[route] role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
        let route = route_output
            .route_update
            .as_ref()
            .expect("snapshot route output should carry structured route");
        assert_eq!(
            route_output.input_action_snapshot.as_ref(),
            Some(&route_snapshot)
        );
        assert_eq!(route.model_role_label, "reviewer");
        assert_eq!(route.routing_preference_label, "prefer_fast");
        assert_eq!(route.endpoint_label, "auto");
        assert!(!route.endpoint_pinned);
        assert!(!route.wire_sends_model_endpoint);
        assert_eq!(route.wire_model_endpoint_label, None);

        assert_eq!(config_output.appended, "[config] history_limit=24");
        let config = config_output
            .session_config_update_detail
            .as_ref()
            .expect("snapshot config output should carry structured config");
        assert_eq!(
            config_output.input_action_snapshot.as_ref(),
            Some(&config_snapshot)
        );
        assert_eq!(config.kind_label, "history_limit");
        assert_eq!(config.history_limit, Some(24));
        assert_eq!(config_output.route_update, None);

        assert_eq!(pressure_output.appended, "[backpressure] pool queue full");
        assert_eq!(pressure_output.output_label, "backpressure");
        assert_eq!(pressure_output.source, OutputUpdateSource::StreamChunk);
        assert!(pressure_output.is_pressure_stream_chunk);
        assert_eq!(pressure_output.state, StreamState::Backpressure);
        assert_eq!(
            pressure_output.input_action_snapshot.as_ref(),
            Some(&pressure_snapshot)
        );
        assert_eq!(pressure_output.route_update, None);
        assert_eq!(pressure_output.session_config_update_detail, None);
    }

    #[test]
    fn cancelled_input_action_snapshot_forces_terminal_scroll_without_request() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Assistant)
            .with_routing_preference(RoutingPreference::PreferQuality);
        let cancel_snapshot =
            InputAction::StreamCancelled(ChatChunk::interrupted(12, "stream cancelled by user"))
                .snapshot(&input);
        let mut viewport = OutputViewport::new();
        viewport.set_follow_latest(false);

        let output = viewport
            .append_input_action_snapshot(&cancel_snapshot)
            .expect("cancelled snapshot should render interrupted terminal output");

        assert_eq!(output.appended, "[interrupted] stream cancelled by user");
        assert_eq!(output.output_label, "interrupted");
        assert_eq!(output.source, OutputUpdateSource::StreamChunk);
        assert_eq!(output.state, StreamState::Interrupted);
        assert!(output.state_is_terminal);
        assert!(!output.state_is_pressure);
        assert!(!output.state_blocks_prompt_submit);
        assert_eq!(output.scroll, ScrollIntent::ScrollToBottom);
        assert_eq!(output.scroll_label, "scroll_to_bottom");
        assert_eq!(output.request_preview, None);
        assert_eq!(output.route_update, None);
        assert_eq!(output.session_config_update_detail, None);
        assert_eq!(
            output.input_action_snapshot.as_ref(),
            Some(&cancel_snapshot)
        );

        let action = output
            .input_action_snapshot
            .as_ref()
            .expect("UI host should receive the original input action snapshot");
        assert_eq!(action.kind, InputActionKind::StreamCancelled);
        assert_eq!(action.kind_label, "stream_cancelled");
        assert_eq!(action.request, None);
        assert_eq!(action.start_chunk, None);
        assert_eq!(action.stream_state, Some(StreamState::Interrupted));
        assert_eq!(action.stream_state_label.as_deref(), Some("interrupted"));
        assert_eq!(action.stream_state_is_terminal, Some(true));
        assert_eq!(action.stream_state_is_pressure, Some(false));
        assert_eq!(action.stream_state_blocks_prompt_submit, Some(false));
        let chunk = action
            .stream_chunk
            .as_ref()
            .expect("cancel action should carry interrupted display chunk");
        assert_eq!(chunk.sequence, 12);
        assert_eq!(chunk.state, StreamState::Interrupted);
        assert_eq!(chunk.output_label, "interrupted");
        assert_eq!(chunk.appended, "[interrupted] stream cancelled by user");
    }

    #[test]
    fn interrupted_terminal_chunks_are_not_labeled_as_hard_errors() {
        let mut viewport = OutputViewport::new();

        let interrupted = viewport.append_chunk(&ChatChunk::interrupted(0, "missing done"));
        let failed = viewport.append_chunk(&ChatChunk::failed(1, "safe-device gate failed"));

        assert_eq!(interrupted.appended, "[interrupted] missing done");
        assert_eq!(failed.appended, "[error] safe-device gate failed");
    }

    #[test]
    fn status_advice_renders_as_local_status_not_stream_pressure() {
        let mut viewport = OutputViewport::new();

        let repair = viewport
            .append_input_action(&InputAction::Status(
                "role=assistant preference=balanced endpoint=auto pinned=false state=streaming history=1 max_tokens=backend-default partial_chars=0 advice=repair_gate failed: safe-device gate failed"
                    .to_owned(),
            ))
            .expect("status should render");
        let busy = viewport
            .append_input_action(&InputAction::Status(
                "role=assistant preference=balanced endpoint=auto pinned=false state=streaming history=1 max_tokens=backend-default partial_chars=7 advice=wait_for_current_stream busy: session stream is already active"
                    .to_owned(),
            ))
            .expect("status should render");

        assert!(repair.appended.starts_with("[status] "));
        assert!(repair.appended.contains("advice=repair_gate"));
        assert!(!repair.appended.starts_with("[error] "));
        assert_eq!(repair.state, StreamState::Pending);
        assert_eq!(repair.source, OutputUpdateSource::LocalStatus);
        assert!(repair.is_local_status);
        assert!(!repair.is_pressure_stream_chunk);
        assert!(busy.appended.starts_with("[status] "));
        assert!(busy.appended.contains("advice=wait_for_current_stream"));
        assert!(!busy.appended.starts_with("[busy] "));
        assert_eq!(busy.state, StreamState::Pending);
        assert_eq!(busy.source, OutputUpdateSource::LocalStatus);
        assert!(busy.is_local_status);
        assert!(!busy.is_pressure_stream_chunk);
    }

    #[test]
    fn status_snapshot_output_keeps_cancelled_session_terminal_but_sendable() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        session.push_delta("partial");
        session.cancel_stream().expect("expected cancel chunk");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let mut viewport = OutputViewport::new();

        let output = viewport.append_status_snapshot(&status);

        assert_eq!(output.output_label, "status");
        assert_eq!(output.source, OutputUpdateSource::LocalStatus);
        assert!(!output.is_pressure_stream_chunk);
        assert_eq!(output.request_preview, None);
        assert_eq!(output.stream_outcome, None);
        assert!(output.appended.contains("state=interrupted"));
        assert!(
            output
                .appended
                .contains("last_error=stream cancelled by user")
        );
        assert!(
            output
                .appended
                .contains("advice=send_now pending: ready to send")
        );
        let structured = output
            .status_snapshot
            .as_ref()
            .expect("status output should carry structured cancelled snapshot");
        assert_eq!(structured, &status);
        assert_eq!(structured.state, StreamState::Interrupted);
        assert_eq!(structured.state_label, "interrupted");
        assert!(structured.state_is_terminal);
        assert!(!structured.state_is_pressure);
        assert!(!structured.state_blocks_prompt_submit);
        assert_eq!(structured.partial_chars, 7);
        assert_eq!(
            structured.last_error.as_deref(),
            Some("stream cancelled by user")
        );
        assert!(structured.send_allowed);
        assert_eq!(structured.send_block_state, None);
        assert_eq!(structured.send_block_chunk, None);
        assert_eq!(structured.route_send_allowed, Some(true));
        assert_eq!(structured.route_send_block_state, None);
        assert_eq!(structured.route_send_block_chunk, None);
        assert_eq!(
            structured.pool_status.as_deref(),
            Some("workers total=1 available=1 busy=0 saturated=0")
        );
        assert_eq!(
            structured.route_pool_status.as_deref(),
            Some("matching total=1 available=1 busy=0 saturated=0")
        );
        assert_eq!(viewport.lines(), &[output.appended]);
    }

    #[test]
    fn status_snapshot_output_exposes_pool_pressure_without_parsing_status_line() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#18 review".to_owned())),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let mut viewport = OutputViewport::new();

        let output = viewport.append_status_snapshot(&status);

        assert_eq!(output.output_label, "status");
        assert_eq!(output.source, OutputUpdateSource::LocalStatus);
        assert_eq!(output.state, StreamState::Pending);
        assert_eq!(output.request_preview, None);
        assert_eq!(output.session_config_update_detail, None);
        assert_eq!(output.gate_advice_detail, None);
        assert_eq!(
            output.appended,
            "[status] role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending history=0 max_tokens=backend-default partial_chars=0 advice=wait_for_worker queued: all model workers are busy; waiting for scheduler across 1 workers pool=workers total=1 available=0 busy=1 saturated=0 route_pool=matching total=1 available=0 busy=1 saturated=0"
        );
        let structured = output
            .status_snapshot
            .as_ref()
            .expect("status output should carry structured snapshot");
        assert_eq!(structured, &status);
        assert_eq!(structured.endpoint_label, "auto");
        assert!(!structured.endpoint_pinned);
        assert!(!structured.send_allowed);
        assert_eq!(structured.send_block_state, Some(StreamState::Queued));
        assert_eq!(structured.send_block_state_label.as_deref(), Some("queued"));
        assert!(structured.send_block_state_is_pressure);
        assert!(structured.send_block_state_blocks_prompt_submit);
        let send_block_chunk = structured
            .send_block_chunk
            .as_ref()
            .expect("status snapshot should expose display chunk for local send block");
        assert_eq!(send_block_chunk.output_label, "queued");
        assert_eq!(
            send_block_chunk.appended,
            "[queued] all model workers are busy; waiting for scheduler across 1 workers"
        );
        assert!(send_block_chunk.state_is_pressure);
        assert!(send_block_chunk.state_blocks_prompt_submit);
        assert_eq!(structured.route_send_allowed, Some(false));
        assert_eq!(structured.route_send_block_state, Some(StreamState::Queued));
        assert_eq!(
            structured.route_send_block_state_label.as_deref(),
            Some("queued")
        );
        let route_send_block_chunk = structured
            .route_send_block_chunk
            .as_ref()
            .expect("status snapshot should expose display chunk for route send block");
        assert_eq!(route_send_block_chunk.output_label, "queued");
        assert_eq!(
            route_send_block_chunk.appended,
            "[queued] all model workers are busy; waiting for scheduler across 1 workers"
        );
        assert!(route_send_block_chunk.state_is_pressure);
        assert!(route_send_block_chunk.state_blocks_prompt_submit);
        assert_eq!(structured.pool_has_busy_workers, Some(true));
        assert_eq!(structured.pool_capacity_state, Some(StreamState::Busy));
        assert_eq!(
            structured.pool_capacity_state_label.as_deref(),
            Some("busy")
        );
        assert_eq!(structured.pool_capacity_state_is_pressure, Some(true));
        assert_eq!(
            structured.pool_capacity_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(structured.route_pool_has_matching_busy_workers, Some(true));
        assert_eq!(
            structured.route_pool_capacity_state,
            Some(StreamState::Busy)
        );
        assert_eq!(
            structured.route_pool_capacity_state_label.as_deref(),
            Some("busy")
        );
        assert_eq!(structured.route_pool_capacity_state_is_pressure, Some(true));
        assert_eq!(
            structured.route_pool_capacity_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(viewport.lines(), &[output.appended]);
    }

    #[test]
    fn status_snapshot_output_carries_engine_busy_without_hiding_worker_state() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#77 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#18 review".to_owned())),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let mut viewport = OutputViewport::new();

        let output = viewport.append_status_snapshot(&status);

        assert_eq!(output.output_label, "status");
        assert_eq!(output.source, OutputUpdateSource::LocalStatus);
        assert!(output.appended.starts_with("[status] "));
        assert!(output.appended.contains(
            "advice=wait_for_current_stream busy: backend engine is busy: #77 chat-stream"
        ));
        assert!(!output.is_pressure_stream_chunk);
        assert_eq!(output.request_preview, None);
        assert_eq!(output.stream_outcome, None);
        let structured = output
            .status_snapshot
            .as_ref()
            .expect("status output should carry structured engine-busy snapshot");
        assert_eq!(structured, &status);
        assert!(!structured.send_allowed);
        assert_eq!(structured.send_block_state, Some(StreamState::Busy));
        assert_eq!(structured.send_block_state_label.as_deref(), Some("busy"));
        assert!(!structured.send_block_state_is_terminal);
        assert!(structured.send_block_state_is_pressure);
        assert!(structured.send_block_state_blocks_prompt_submit);
        assert_eq!(
            structured.send_block_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        let send_block_chunk = structured
            .send_block_chunk
            .as_ref()
            .expect("engine busy should expose a display chunk");
        assert_eq!(send_block_chunk.output_label, "busy");
        assert_eq!(
            send_block_chunk.appended,
            "[busy] backend engine is busy: #77 chat-stream"
        );
        assert!(send_block_chunk.state_is_pressure);
        assert!(send_block_chunk.state_blocks_prompt_submit);
        assert_eq!(structured.route_send_allowed, Some(false));
        assert_eq!(structured.route_send_block_state, Some(StreamState::Busy));
        assert_eq!(
            structured.route_send_block_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        assert_eq!(
            structured.pool_status.as_deref(),
            Some("workers total=2 available=1 busy=1 saturated=0")
        );
        assert_eq!(
            structured.route_pool_status.as_deref(),
            Some("matching total=2 available=1 busy=1 saturated=0")
        );
        let route_workers = structured
            .route_workers
            .as_ref()
            .expect("engine-busy output should still carry worker rows");
        assert_eq!(route_workers.len(), 2);
        assert!(route_workers.iter().all(|worker| worker.route_match));
        assert!(route_workers.iter().all(|worker| !worker.selectable));
        assert!(
            route_workers
                .iter()
                .all(|worker| worker.picker_action_label == "wait")
        );
        assert!(
            route_workers
                .iter()
                .all(|worker| worker.decision_action_label() == "wait_for_current_stream")
        );
        assert_eq!(route_workers[0].worker_status_label(), "available");
        assert_eq!(route_workers[1].worker_status_label(), "busy");
        assert_eq!(viewport.lines(), &[output.appended]);
    }

    #[test]
    fn status_snapshot_output_carries_repair_gate_without_hiding_worker_state() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                experience_hygiene_ok: false,
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#18 review".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(2, 2),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let mut viewport = OutputViewport::new();

        let output = viewport.append_status_snapshot(&status);

        assert_eq!(output.output_label, "status");
        assert_eq!(output.source, OutputUpdateSource::LocalStatus);
        assert!(output.appended.starts_with("[status] "));
        assert!(output.appended.contains("advice=repair_gate failed"));
        assert!(!output.is_pressure_stream_chunk);
        assert_eq!(output.request_preview, None);
        assert_eq!(output.stream_outcome, None);
        let structured = output
            .status_snapshot
            .as_ref()
            .expect("status output should carry structured repair-gate snapshot");
        assert_eq!(structured, &status);
        assert!(!structured.send_allowed);
        assert_eq!(structured.send_block_state, Some(StreamState::Failed));
        assert_eq!(structured.send_block_state_label.as_deref(), Some("failed"));
        assert!(structured.send_block_state_is_terminal);
        assert!(!structured.send_block_state_is_pressure);
        assert!(!structured.send_block_state_blocks_prompt_submit);
        assert_eq!(
            structured.send_block_reason.as_deref(),
            Some("experience hygiene gate failed")
        );
        let send_block_chunk = structured
            .send_block_chunk
            .as_ref()
            .expect("repair gate should expose a display chunk");
        assert_eq!(send_block_chunk.output_label, "error");
        assert_eq!(
            send_block_chunk.appended,
            "[error] experience hygiene gate failed"
        );
        assert_eq!(structured.route_send_allowed, Some(false));
        assert_eq!(structured.route_send_block_state, Some(StreamState::Failed));
        assert_eq!(
            structured.route_send_block_reason.as_deref(),
            Some("experience hygiene gate failed")
        );
        assert_eq!(
            structured.pool_status.as_deref(),
            Some("workers total=2 available=0 busy=1 saturated=1")
        );
        assert_eq!(
            structured.route_pool_status.as_deref(),
            Some("matching total=2 available=0 busy=1 saturated=1")
        );
        assert_eq!(
            structured.pool_capacity_state,
            Some(StreamState::Backpressure)
        );
        let route_workers = structured
            .route_workers
            .as_ref()
            .expect("repair-gate output should still carry worker rows");
        assert_eq!(route_workers.len(), 2);
        assert!(route_workers.iter().all(|worker| worker.route_match));
        assert!(route_workers.iter().all(|worker| !worker.selectable));
        assert!(
            route_workers
                .iter()
                .all(|worker| worker.picker_action_label == "repair_gate")
        );
        assert_eq!(route_workers[0].worker_status_label(), "busy");
        assert_eq!(route_workers[1].worker_status_label(), "backpressure");
        assert_eq!(viewport.lines(), &[output.appended]);
    }

    #[test]
    fn workers_snapshot_output_exposes_picker_state_without_parsing_worker_line() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let mut viewport = OutputViewport::new();

        let output = viewport
            .append_workers_snapshot(&status)
            .expect("model pool status should render workers output");

        assert_eq!(
            status.workers_line().as_deref(),
            Some(
                "role=reviewer preference=prefer_fast endpoint=auto pinned=false advice=send_now pending: ready to send pool=workers total=2 available=2 busy=0 saturated=0 route_pool=matching total=1 available=1 busy=0 saturated=0 workers=[endpoint=quality-12b status=available queue=0/1 active=none roles=assistant preferences=prefer_quality | endpoint=fast-reviewer status=available queue=0/1 active=none roles=reviewer preferences=prefer_fast]"
            )
        );
        assert_eq!(output.output_label, "workers");
        assert_eq!(output.source, OutputUpdateSource::LocalStatus);
        assert_eq!(output.request_preview, None);
        assert_eq!(output.session_config_update_detail, None);
        assert_eq!(output.gate_advice_detail, None);
        assert!(output.appended.starts_with("[workers] role=reviewer "));
        let structured = output
            .status_snapshot
            .as_ref()
            .expect("workers output should carry structured status snapshot");
        assert_eq!(structured, &status);
        assert_eq!(structured.endpoint_label, "auto");
        assert!(!structured.endpoint_pinned);
        let route_workers = structured
            .route_workers
            .as_ref()
            .expect("workers output should expose route worker picker state");
        assert_eq!(route_workers.len(), 2);
        assert_eq!(route_workers[0].endpoint_label(), "quality-12b");
        assert_eq!(route_workers[0].picker_action_label, "unavailable");
        let mismatch_decision = route_workers[0]
            .decision_display_snapshot()
            .expect("capability mismatch should expose a queued decision chunk");
        assert_eq!(mismatch_decision.output_label, "queued");
        assert_eq!(
            mismatch_decision.appended,
            "[queued] worker quality-12b does not match role=reviewer preference=prefer_fast"
        );
        assert!(mismatch_decision.state_blocks_prompt_submit);
        assert_eq!(route_workers[0].worker_status_display_snapshot(), None);
        assert_eq!(route_workers[1].endpoint_label(), "fast-reviewer");
        assert_eq!(route_workers[1].picker_action_label, "select");
        assert_eq!(route_workers[1].worker_status_display_snapshot(), None);
        assert_eq!(route_workers[1].decision_display_snapshot(), None);
    }

    #[test]
    fn workers_snapshot_output_carries_engine_busy_and_worker_health_rows() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#77 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#19 review".to_owned())),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let mut viewport = OutputViewport::new();

        let output = viewport
            .append_workers_snapshot(&status)
            .expect("engine-busy status should still render workers output");

        assert_eq!(output.output_label, "workers");
        assert_eq!(output.source, OutputUpdateSource::LocalStatus);
        assert!(output.appended.starts_with("[workers] role=reviewer "));
        assert!(output.appended.contains(
            "advice=wait_for_current_stream busy: backend engine is busy: #77 chat-stream"
        ));
        assert_eq!(output.request_preview, None);
        assert_eq!(output.session_config_update_detail, None);
        assert_eq!(output.stream_outcome, None);
        let structured = output
            .status_snapshot
            .as_ref()
            .expect("workers output should carry structured engine-busy status");
        assert_eq!(structured, &status);
        assert!(!structured.send_allowed);
        assert_eq!(structured.send_block_state, Some(StreamState::Busy));
        assert_eq!(
            structured.send_block_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        assert_eq!(structured.route_send_allowed, Some(false));
        assert_eq!(structured.route_send_block_state, Some(StreamState::Busy));
        assert_eq!(
            structured.route_send_block_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        assert_eq!(
            structured.pool_status.as_deref(),
            Some("workers total=2 available=1 busy=1 saturated=0")
        );
        assert_eq!(
            structured.route_pool_status.as_deref(),
            Some("matching total=2 available=1 busy=1 saturated=0")
        );
        let route_workers = structured
            .route_workers
            .as_ref()
            .expect("workers output should keep route worker rows under engine busy");
        assert_eq!(route_workers.len(), 2);
        assert!(route_workers.iter().all(|worker| worker.route_match));
        assert!(route_workers.iter().all(|worker| !worker.selectable));
        assert!(
            route_workers
                .iter()
                .all(|worker| worker.picker_action_label == "wait")
        );
        assert!(
            route_workers
                .iter()
                .all(|worker| worker.decision_action_label() == "wait_for_current_stream")
        );
        assert_eq!(route_workers[0].worker_status_label(), "available");
        assert_eq!(route_workers[1].worker_status_label(), "busy");
        assert!(route_workers.iter().all(|worker| {
            worker
                .decision_display_snapshot()
                .is_some_and(|chunk| chunk.output_label == "busy")
        }));
        assert_eq!(viewport.lines(), &[output.appended]);
    }

    #[test]
    fn workers_snapshot_output_carries_repair_gate_and_worker_health_rows() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                safe_device_ok: false,
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#19 review".to_owned())),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let mut viewport = OutputViewport::new();

        let output = viewport
            .append_workers_snapshot(&status)
            .expect("repair-gate status should still render workers output");

        assert_eq!(output.output_label, "workers");
        assert_eq!(output.source, OutputUpdateSource::LocalStatus);
        assert!(output.appended.starts_with("[workers] role=reviewer "));
        assert!(output.appended.contains("advice=repair_gate failed"));
        assert_eq!(output.request_preview, None);
        assert_eq!(output.session_config_update_detail, None);
        assert_eq!(output.stream_outcome, None);
        let structured = output
            .status_snapshot
            .as_ref()
            .expect("workers output should carry structured repair-gate status");
        assert_eq!(structured, &status);
        assert!(!structured.send_allowed);
        assert_eq!(structured.send_block_state, Some(StreamState::Failed));
        assert_eq!(
            structured.send_block_reason.as_deref(),
            Some("safe-device gate failed")
        );
        assert_eq!(structured.route_send_allowed, Some(false));
        assert_eq!(structured.route_send_block_state, Some(StreamState::Failed));
        assert_eq!(
            structured.route_send_block_reason.as_deref(),
            Some("safe-device gate failed")
        );
        assert_eq!(
            structured.pool_status.as_deref(),
            Some("workers total=2 available=1 busy=1 saturated=0")
        );
        assert_eq!(
            structured.route_pool_status.as_deref(),
            Some("matching total=2 available=1 busy=1 saturated=0")
        );
        let route_workers = structured
            .route_workers
            .as_ref()
            .expect("workers output should keep route worker rows under repair gate");
        assert_eq!(route_workers.len(), 2);
        assert!(route_workers.iter().all(|worker| worker.route_match));
        assert!(route_workers.iter().all(|worker| !worker.selectable));
        assert!(
            route_workers
                .iter()
                .all(|worker| worker.picker_action_label == "repair_gate")
        );
        assert_eq!(route_workers[0].worker_status_label(), "available");
        assert_eq!(route_workers[1].worker_status_label(), "busy");
        assert!(route_workers.iter().all(|worker| {
            worker
                .decision_display_snapshot()
                .is_some_and(|chunk| chunk.output_label == "error")
        }));
        assert_eq!(viewport.lines(), &[output.appended]);
    }

    #[test]
    fn workers_snapshot_projects_web_forge_fields_under_repair_gate_without_stream_side_effects() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                safe_device_ok: false,
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#41 review".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(1, 1),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let mut viewport = OutputViewport::new();

        let output = viewport
            .append_workers_snapshot(&status)
            .expect("repair-gate workers snapshot should render read-only host output");

        assert_eq!(output.output_label, "workers");
        assert_eq!(output.source, OutputUpdateSource::LocalStatus);
        assert_eq!(output.source_label, "local_status");
        assert!(output.is_local_status);
        assert!(!output.is_stream_chunk);
        assert!(!output.is_gate_advice);
        assert_eq!(output.state, StreamState::Pending);
        assert_eq!(output.request_preview, None);
        assert_eq!(output.route_update, None);
        assert_eq!(output.session_config_update_detail, None);
        assert_eq!(output.stream_outcome, None);
        assert_eq!(output.stream_chunk, None);
        assert_eq!(output.input_action_snapshot, None);
        assert!(output.appended.contains("advice=repair_gate failed"));

        let structured = output
            .status_snapshot
            .as_ref()
            .expect("workers output should carry the structured host snapshot");
        assert_eq!(structured, &status);
        assert_eq!(structured.history_messages, 0);
        assert_eq!(structured.partial_chars, 0);
        assert_eq!(structured.last_error, None);
        assert!(!structured.send_allowed);
        assert_eq!(structured.send_block_state, Some(StreamState::Failed));
        assert_eq!(
            structured.send_block_reason.as_deref(),
            Some("safe-device gate failed")
        );
        assert_eq!(structured.route_send_allowed, Some(false));
        assert_eq!(structured.route_send_block_state, Some(StreamState::Failed));
        assert_eq!(
            structured.route_send_block_reason.as_deref(),
            Some("safe-device gate failed")
        );
        assert_eq!(
            structured.gate_advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(
            structured.route_gate_advice_action_label.as_deref(),
            Some("repair_gate")
        );

        let workers = structured
            .workers
            .as_ref()
            .expect("host snapshot should expose worker health rows");
        assert_eq!(workers.len(), 3);
        assert_eq!(workers[0].endpoint_label(), "fast-reviewer");
        assert_eq!(workers[0].status_label(), "available");
        assert_eq!(workers[0].role_labels(), vec!["reviewer"]);
        assert_eq!(workers[0].preference_labels(), vec!["prefer_fast"]);
        assert_eq!(workers[0].status_display_snapshot(), None);
        assert_eq!(workers[1].endpoint_label(), "quality-12b");
        assert_eq!(workers[1].status_label(), "busy");
        let busy_health = workers[1]
            .status_display_snapshot()
            .expect("busy worker health should remain visible under repair gate");
        assert_eq!(busy_health.output_label, "busy");
        assert_eq!(
            busy_health.appended,
            "[busy] worker quality-12b is busy: #41 review"
        );
        assert_eq!(workers[2].endpoint_label(), "summary-tester");
        assert_eq!(workers[2].status_label(), "backpressure");
        let saturated_health = workers[2]
            .status_display_snapshot()
            .expect("saturated worker health should remain visible under repair gate");
        assert_eq!(saturated_health.output_label, "backpressure");
        assert_eq!(
            saturated_health.appended,
            "[backpressure] worker summary-tester queue is saturated: 1/1"
        );

        let route_workers = structured
            .route_workers
            .as_ref()
            .expect("host snapshot should expose route worker picker rows");
        assert_eq!(route_workers.len(), 3);
        assert!(route_workers.iter().all(|worker| worker.route_match));
        assert!(route_workers.iter().all(|worker| !worker.selectable));
        assert!(
            route_workers
                .iter()
                .all(|worker| worker.picker_action_label == "repair_gate")
        );
        assert!(
            route_workers
                .iter()
                .all(|worker| worker.decision_action_label() == "repair_gate")
        );
        assert!(
            route_workers
                .iter()
                .all(|worker| worker.decision_reason() == "safe-device gate failed")
        );
        assert_eq!(route_workers[0].worker_status_label(), "available");
        assert_eq!(route_workers[1].worker_status_label(), "busy");
        assert_eq!(route_workers[2].worker_status_label(), "backpressure");
        for worker in route_workers {
            assert_eq!(worker.selection_model_role_label, "reviewer");
            assert_eq!(worker.selection_routing_preference_label, "prefer_fast");
            assert!(worker.selection_wire_prefer_fast);
            assert!(worker.selection_wire_endpoint_pinned);
            assert!(worker.selection_wire_sends_model_endpoint);
            let decision = worker
                .decision_display_snapshot()
                .expect("repair gate picker row should expose a decision chunk");
            assert_eq!(decision.output_label, "error");
            assert_eq!(decision.appended, "[error] safe-device gate failed");
        }
        assert_eq!(viewport.lines(), &[output.appended]);
    }

    #[test]
    fn status_and_workers_host_snapshots_keep_local_envelope_under_gates() {
        let cases = [
            (
                "busy",
                StreamState::Busy,
                "backend engine is busy: #77 chat-stream",
                "backend engine is busy: #77 chat-stream",
                "busy",
                "wait",
                "available",
                None,
                ModelPoolGateSnapshot::new(
                    FrontendGateSnapshot {
                        engine_busy: true,
                        active_request: Some("#77 chat-stream".to_owned()),
                        ..FrontendGateSnapshot::default()
                    },
                    vec![
                        ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                            .with_roles([ModelRole::Reviewer])
                            .with_preferences([RoutingPreference::PreferFast]),
                    ],
                ),
            ),
            (
                "backpressure",
                StreamState::Backpressure,
                "model pool is saturated: 1 workers",
                "model pool is saturated: 1 workers",
                "backpressure",
                "wait",
                "backpressure",
                Some("backpressure"),
                ModelPoolGateSnapshot::new(
                    FrontendGateSnapshot::default(),
                    vec![
                        ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                            .with_roles([ModelRole::Reviewer])
                            .with_preferences([RoutingPreference::PreferFast])
                            .with_queue(1, 1),
                    ],
                ),
            ),
            (
                "repair_gate",
                StreamState::Failed,
                "safe-device gate failed",
                "safe-device gate failed",
                "error",
                "repair_gate",
                "available",
                None,
                ModelPoolGateSnapshot::new(
                    FrontendGateSnapshot {
                        safe_device_ok: false,
                        ..FrontendGateSnapshot::default()
                    },
                    vec![
                        ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                            .with_roles([ModelRole::Reviewer])
                            .with_preferences([RoutingPreference::PreferFast]),
                    ],
                ),
            ),
        ];

        for (
            case,
            expected_block_state,
            expected_send_reason,
            expected_route_reason,
            expected_chunk_label,
            expected_picker_action,
            expected_worker_status,
            expected_worker_status_chunk,
            gate,
        ) in cases
        {
            let input = CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast);
            let session = ChatSession::new("cli", ChatSessionConfig::default());
            let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
            let mut viewport = OutputViewport::new();

            let status_output = viewport.append_status_snapshot(&status);
            let workers_output = viewport
                .append_workers_snapshot(&status)
                .expect("model pool status should render workers output");

            for (label, output) in [("status", status_output), ("workers", workers_output)] {
                assert_eq!(output.output_label, label, "{case} {label}");
                assert_eq!(
                    output.source,
                    OutputUpdateSource::LocalStatus,
                    "{case} {label}"
                );
                assert_eq!(output.source_label, "local_status", "{case} {label}");
                assert!(output.is_local_status, "{case} {label}");
                assert!(!output.is_stream_chunk, "{case} {label}");
                assert!(!output.is_gate_advice, "{case} {label}");
                assert!(!output.is_pressure_stream_chunk, "{case} {label}");
                assert!(!output.is_terminal_stream_chunk, "{case} {label}");
                assert_eq!(output.state, StreamState::Pending, "{case} {label}");
                assert_eq!(output.state_label, "pending", "{case} {label}");
                assert!(!output.state_is_pressure, "{case} {label}");
                assert!(!output.state_is_terminal, "{case} {label}");
                assert!(!output.state_blocks_prompt_submit, "{case} {label}");
                assert_eq!(output.request_preview, None, "{case} {label}");
                assert_eq!(output.route_update, None, "{case} {label}");
                assert_eq!(output.session_config_update_detail, None, "{case} {label}");
                assert_eq!(output.gate_advice_detail, None, "{case} {label}");
                assert_eq!(output.stream_outcome, None, "{case} {label}");
                assert_eq!(output.stream_chunk, None, "{case} {label}");
                assert_eq!(output.input_action_snapshot, None, "{case} {label}");

                let structured = output
                    .status_snapshot
                    .as_ref()
                    .expect("host output should carry structured status snapshot");
                assert_eq!(structured, &status, "{case} {label}");
                assert_eq!(structured.history_messages, 0, "{case} {label}");
                assert_eq!(structured.partial_chars, 0, "{case} {label}");
                assert_eq!(structured.last_error, None, "{case} {label}");
                assert_eq!(
                    structured.max_tokens_label, "backend-default",
                    "{case} {label}"
                );
                assert!(!structured.send_allowed, "{case} {label}");
                assert_eq!(
                    structured.send_block_state,
                    Some(expected_block_state),
                    "{case} {label}"
                );
                assert_eq!(
                    structured.send_block_reason.as_deref(),
                    Some(expected_send_reason),
                    "{case} {label}"
                );
                assert_eq!(structured.route_send_allowed, Some(false), "{case} {label}");
                assert_eq!(
                    structured.route_send_block_state,
                    Some(expected_block_state),
                    "{case} {label}"
                );
                assert_eq!(
                    structured.route_send_block_reason.as_deref(),
                    Some(expected_route_reason),
                    "{case} {label}"
                );
                let send_block_chunk = structured
                    .send_block_chunk
                    .as_ref()
                    .expect("blocked status should expose a display chunk");
                assert_eq!(
                    send_block_chunk.output_label, expected_chunk_label,
                    "{case} {label}"
                );

                let workers = structured
                    .workers
                    .as_ref()
                    .expect("host snapshot should keep worker health rows");
                assert_eq!(workers.len(), 1, "{case} {label}");
                assert_eq!(
                    workers[0].endpoint_label(),
                    "fast-reviewer",
                    "{case} {label}"
                );
                assert_eq!(workers[0].role_labels(), vec!["reviewer"], "{case} {label}");
                assert_eq!(
                    workers[0].preference_labels(),
                    vec!["prefer_fast"],
                    "{case} {label}"
                );
                assert_eq!(
                    workers[0].status_label(),
                    expected_worker_status,
                    "{case} {label}"
                );
                assert_eq!(
                    workers[0]
                        .status_display_snapshot()
                        .as_ref()
                        .map(|chunk| chunk.output_label.as_str()),
                    expected_worker_status_chunk,
                    "{case} {label}"
                );

                let route_workers = structured
                    .route_workers
                    .as_ref()
                    .expect("host snapshot should keep route worker picker rows");
                assert_eq!(route_workers.len(), 1, "{case} {label}");
                assert_eq!(
                    route_workers[0].endpoint_label(),
                    "fast-reviewer",
                    "{case} {label}"
                );
                assert!(route_workers[0].route_match, "{case} {label}");
                assert!(!route_workers[0].selectable, "{case} {label}");
                assert_eq!(
                    route_workers[0].picker_action_label, expected_picker_action,
                    "{case} {label}"
                );
                assert_eq!(
                    route_workers[0].worker_status_label(),
                    expected_worker_status,
                    "{case} {label}"
                );
                assert_eq!(
                    route_workers[0].worker.role_labels(),
                    vec!["reviewer"],
                    "{case} {label}"
                );
                assert_eq!(
                    route_workers[0].worker.preference_labels(),
                    vec!["prefer_fast"],
                    "{case} {label}"
                );
                let route_decision_chunk = route_workers[0]
                    .decision_display_snapshot()
                    .expect("blocked route worker should expose a local display chunk");
                assert_eq!(
                    route_decision_chunk.output_label, expected_chunk_label,
                    "{case} {label}"
                );
            }
        }
    }

    #[test]
    fn route_backpressure_host_outputs_preserve_worker_rows_and_route_lane_snapshot() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(1, 1),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let mut viewport = OutputViewport::new();

        let status_output = viewport.append_status_snapshot(&status);
        let workers_output = viewport
            .append_workers_snapshot(&status)
            .expect("route backpressure should still render workers output");

        for (label, output) in [("status", status_output), ("workers", workers_output)] {
            assert_eq!(output.output_label, label, "{label}");
            assert_eq!(output.source, OutputUpdateSource::LocalStatus, "{label}");
            assert_eq!(output.request_preview, None, "{label}");
            assert_eq!(output.route_update, None, "{label}");
            assert_eq!(output.session_config_update_detail, None, "{label}");
            assert_eq!(output.gate_advice_detail, None, "{label}");
            assert_eq!(output.stream_outcome, None, "{label}");
            assert_eq!(output.stream_chunk, None, "{label}");

            let structured = output
                .status_snapshot
                .as_ref()
                .expect("host output should carry structured route backpressure snapshot");
            assert_eq!(structured, &status, "{label}");
            assert!(!structured.send_allowed, "{label}");
            assert_eq!(
                structured.send_block_state,
                Some(StreamState::Backpressure),
                "{label}"
            );
            assert_eq!(
                structured.send_block_reason.as_deref(),
                Some("matching model workers are saturated: 1 workers"),
                "{label}"
            );
            assert_eq!(structured.route_send_allowed, Some(false), "{label}");
            assert_eq!(
                structured.route_send_block_state,
                Some(StreamState::Backpressure),
                "{label}"
            );
            assert_eq!(
                structured.route_send_block_reason.as_deref(),
                Some("matching model workers are saturated: 1 workers"),
                "{label}"
            );
            assert_eq!(
                structured.pool_status.as_deref(),
                Some("workers total=2 available=1 busy=0 saturated=1"),
                "{label}"
            );
            assert_eq!(
                structured.route_pool_status.as_deref(),
                Some("matching total=1 available=0 busy=0 saturated=1"),
                "{label}"
            );
            assert_eq!(
                structured.route_pool_capacity_state,
                Some(StreamState::Backpressure),
                "{label}"
            );
            assert_eq!(
                structured.route_pool_capacity_state_label.as_deref(),
                Some("backpressure"),
                "{label}"
            );
            assert_eq!(
                structured.route_pool_has_matching_available_workers,
                Some(false),
                "{label}"
            );
            assert_eq!(structured.pool_has_available_workers, Some(true), "{label}");

            let send_block_chunk = structured
                .send_block_chunk
                .as_ref()
                .expect("route backpressure should expose a structured send block chunk");
            assert_eq!(send_block_chunk.output_label, "backpressure", "{label}");
            assert_eq!(
                send_block_chunk.appended,
                "[backpressure] matching model workers are saturated: 1 workers",
                "{label}"
            );

            let workers = structured
                .workers
                .as_ref()
                .expect("route backpressure should keep worker health rows");
            assert_eq!(workers.len(), 2, "{label}");
            assert_eq!(workers[0].endpoint_label(), "quality-12b", "{label}");
            assert_eq!(workers[0].status_label(), "available", "{label}");
            assert_eq!(workers[1].endpoint_label(), "fast-reviewer", "{label}");
            assert_eq!(workers[1].status_label(), "backpressure", "{label}");

            let route_workers = structured
                .route_workers
                .as_ref()
                .expect("route backpressure should keep route worker rows");
            assert_eq!(route_workers.len(), 2, "{label}");
            assert_eq!(route_workers[0].endpoint_label(), "quality-12b", "{label}");
            assert!(!route_workers[0].route_match, "{label}");
            assert!(!route_workers[0].selectable, "{label}");
            assert_eq!(
                route_workers[0].picker_action,
                ModelRouteWorkerPickerAction::Unavailable,
                "{label}"
            );
            assert_eq!(
                route_workers[0].picker_action_label, "unavailable",
                "{label}"
            );
            assert_eq!(
                route_workers[0].worker_status_label(),
                "available",
                "{label}"
            );
            assert_eq!(
                route_workers[1].endpoint_label(),
                "fast-reviewer",
                "{label}"
            );
            assert!(route_workers[1].route_match, "{label}");
            assert!(!route_workers[1].selectable, "{label}");
            assert_eq!(
                route_workers[1].picker_action,
                ModelRouteWorkerPickerAction::Wait,
                "{label}"
            );
            assert_eq!(route_workers[1].picker_action_label, "wait", "{label}");
            assert_eq!(
                route_workers[1].worker_status_label(),
                "backpressure",
                "{label}"
            );
            assert_eq!(
                route_workers[1].decision_action_label(),
                "retry_later",
                "{label}"
            );
            assert_eq!(
                route_workers[1].decision_state_label(),
                "backpressure",
                "{label}"
            );
        }
    }

    #[test]
    fn route_update_output_exposes_role_preference_and_optional_endpoint_boundary() {
        let auto_intent = RoutingIntent {
            model_role: ModelRole::Reviewer,
            routing_preference: RoutingPreference::PreferFast,
            model_endpoint: None,
            endpoint_pinned: false,
        };
        let pinned_intent = RoutingIntent {
            model_role: ModelRole::Tester,
            routing_preference: RoutingPreference::PreferQuality,
            model_endpoint: Some(ModelEndpoint::Worker("mlx-test-4b".to_owned())),
            endpoint_pinned: true,
        };
        let mut viewport = OutputViewport::new();

        let auto_output = viewport.append_route_update(&auto_intent);
        let pinned_output = viewport.append_route_update(&pinned_intent);

        assert_eq!(
            route_update_status(&auto_intent),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
        assert_eq!(
            auto_output.appended,
            "[route] role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
        assert_eq!(auto_output.output_label, "route");
        assert_eq!(auto_output.source, OutputUpdateSource::LocalStatus);
        assert_eq!(auto_output.request_preview, None);
        assert_eq!(auto_output.session_config_update_detail, None);
        assert_eq!(auto_output.status_snapshot, None);
        let auto_route = auto_output
            .route_update
            .as_ref()
            .expect("route output should carry structured route update");
        assert_eq!(auto_route.routing_intent, auto_intent);
        assert_eq!(auto_route.model_role_label, "reviewer");
        assert_eq!(auto_route.routing_preference_label, "prefer_fast");
        assert_eq!(auto_route.endpoint_label, "auto");
        assert!(!auto_route.endpoint_pinned);
        assert!(auto_route.endpoint_auto);
        assert_eq!(auto_route.endpoint_kind_label, "auto");
        assert_eq!(auto_route.wire_model_role_label, "reviewer");
        assert_eq!(auto_route.wire_routing_preference_label, "prefer_fast");
        assert!(auto_route.wire_prefer_fast);
        assert!(!auto_route.wire_prefer_quality);
        assert!(!auto_route.wire_endpoint_pinned);
        assert!(!auto_route.wire_sends_model_endpoint);
        assert_eq!(auto_route.wire_model_endpoint_label, None);

        assert_eq!(
            pinned_output.appended,
            "[route] role=tester preference=prefer_quality endpoint=mlx-test-4b pinned=true"
        );
        let pinned_route = pinned_output
            .route_update
            .as_ref()
            .expect("pinned route output should carry structured route update");
        assert_eq!(pinned_route.routing_intent, pinned_intent);
        assert_eq!(pinned_route.model_role_label, "tester");
        assert_eq!(pinned_route.routing_preference_label, "prefer_quality");
        assert_eq!(pinned_route.endpoint_label, "mlx-test-4b");
        assert!(pinned_route.endpoint_pinned);
        assert!(pinned_route.endpoint_custom);
        assert_eq!(pinned_route.endpoint_kind_label, "custom");
        assert!(!pinned_route.wire_prefer_fast);
        assert!(pinned_route.wire_prefer_quality);
        assert!(pinned_route.wire_endpoint_pinned);
        assert!(pinned_route.wire_sends_model_endpoint);
        assert_eq!(
            pinned_route.wire_model_endpoint_label.as_deref(),
            Some("mlx-test-4b")
        );
    }

    #[test]
    fn output_source_fields_distinguish_stream_pressure_from_local_status() {
        let mut viewport = OutputViewport::new();

        let queued = viewport.append_chunk(&ChatChunk::queued(0, "waiting for reviewer"));
        let busy_status = viewport
            .append_input_action(&InputAction::Status(
                "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending advice=wait_for_current_stream busy: session stream is already active"
                    .to_owned(),
            ))
            .expect("status should render");
        let backpressure_advice = viewport.append_gate_advice(
            &GateDecision::blocked(StreamState::Backpressure, "pool queue full").advice(),
        );

        assert_eq!(queued.output_label, "queued");
        assert_eq!(queued.source_label, "stream_chunk");
        assert!(queued.is_stream_chunk);
        assert!(queued.is_pressure_stream_chunk);
        assert!(!queued.is_terminal_stream_chunk);
        assert!(queued.state_is_pressure);
        assert!(queued.state_blocks_prompt_submit);
        let queued_chunk = queued
            .stream_chunk
            .as_ref()
            .expect("pressure stream output should carry service chunk display snapshot");
        assert_eq!(queued_chunk.output_label, "queued");
        assert_eq!(queued_chunk.appended, "[queued] waiting for reviewer");
        assert!(queued_chunk.state_is_pressure);
        assert!(queued_chunk.state_blocks_prompt_submit);

        assert_eq!(busy_status.output_label, "status");
        assert_eq!(busy_status.source_label, "local_status");
        assert!(busy_status.is_local_status);
        assert_eq!(busy_status.state, StreamState::Pending);
        assert!(!busy_status.state_is_pressure);
        assert!(!busy_status.is_pressure_stream_chunk);

        assert_eq!(backpressure_advice.output_label, "advice");
        assert_eq!(backpressure_advice.source_label, "gate_advice");
        assert!(backpressure_advice.is_gate_advice);
        assert_eq!(backpressure_advice.state, StreamState::Backpressure);
        assert!(backpressure_advice.state_is_pressure);
        assert!(!backpressure_advice.is_pressure_stream_chunk);
    }

    #[test]
    fn request_preview_summarizes_send_without_prompt_text_or_worker_pin() {
        let request = ChatRequest::new(
            "cli",
            vec![
                ChatMessage::assistant("prior answer"),
                ChatMessage::user("next prompt"),
            ],
        )
        .with_max_tokens(Some(8192))
        .with_model_role(ModelRole::Reviewer)
        .with_routing_preference(RoutingPreference::PreferFast);

        assert_eq!(
            request_preview_status(&request),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false messages=2 last_user_chars=11 max_tokens=8192 stream=true"
        );
        assert!(!request_preview_status(&request).contains("next prompt"));
    }

    #[test]
    fn request_preview_snapshot_exposes_send_boundary_without_prompt_text() {
        let request = ChatRequest::new(
            "cli",
            vec![
                ChatMessage::assistant("prior answer"),
                ChatMessage::user("sensitive patch details"),
            ],
        )
        .with_max_tokens(Some(8192))
        .with_model_role(ModelRole::Reviewer)
        .with_routing_preference(RoutingPreference::PreferFast);

        let preview = RequestPreviewSnapshot::from_request(&request);

        assert_eq!(preview.routing_intent.model_role, ModelRole::Reviewer);
        assert_eq!(
            preview.routing_intent.routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(preview.routing_intent.endpoint_label(), "auto");
        assert!(!preview.routing_intent.endpoint_pinned);
        assert_eq!(preview.model_role_label, "reviewer");
        assert_eq!(preview.routing_preference_label, "prefer_fast");
        assert_eq!(preview.endpoint_label, "auto");
        assert!(!preview.endpoint_pinned);
        assert_eq!(preview.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(preview.endpoint_kind_label, "auto");
        assert!(preview.endpoint_auto);
        assert!(!preview.endpoint_built_in);
        assert!(!preview.endpoint_custom);
        assert_eq!(preview.wire_model_role_label, "reviewer");
        assert_eq!(preview.wire_routing_preference_label, "prefer_fast");
        assert!(preview.wire_prefer_fast);
        assert!(!preview.wire_prefer_quality);
        assert!(preview.wire_sends_max_tokens);
        assert_eq!(preview.wire_max_tokens, Some(8192));
        assert_eq!(preview.wire_endpoint_pinned, preview.endpoint_pinned);
        assert_eq!(preview.wire_endpoint_kind_label, "auto");
        assert!(!preview.wire_sends_model_endpoint);
        assert_eq!(preview.wire_model_endpoint_label, None);
        assert_eq!(preview.messages, 2);
        assert_eq!(preview.context_messages, 1);
        assert_eq!(preview.history_messages, 1);
        assert_eq!(preview.history_limit, None);
        assert_eq!(preview.history_remaining, None);
        assert_eq!(preview.history_messages_after_submit, None);
        assert_eq!(preview.history_at_limit_after_submit, None);
        assert_eq!(preview.history_truncates_on_submit, None);
        assert_eq!(preview.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(preview.context_kind_label, "multi_turn");
        assert!(preview.has_context);
        assert!(!preview.is_single_turn);
        assert_eq!(preview.last_message_role_label.as_deref(), Some("user"));
        assert_eq!(preview.last_message_chars, 23);
        assert!(preview.last_message_is_user);
        assert_eq!(preview.last_user_chars, 23);
        assert_eq!(preview.max_tokens, Some(8192));
        assert_eq!(preview.max_tokens_label, "8192");
        assert!(preview.stream);
        assert_eq!(preview.start_sequence, None);
        assert_eq!(preview.start_state, None);
        assert_eq!(preview.start_state_label, None);
        assert_eq!(preview.start_state_is_terminal, None);
        assert_eq!(preview.start_state_is_pressure, None);
        assert_eq!(preview.start_state_blocks_prompt_submit, None);
        assert_eq!(preview.start_chunk, None);
        assert_eq!(
            preview.line(),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false messages=2 last_user_chars=23 max_tokens=8192 stream=true"
        );
        assert!(!preview.line().contains("sensitive patch details"));
    }

    #[test]
    fn request_preview_can_carry_history_policy_without_changing_terminal_line() {
        let request = ChatRequest::new(
            "cli",
            vec![
                ChatMessage::user("one"),
                ChatMessage::assistant("two"),
                ChatMessage::user("three"),
            ],
        )
        .with_model_role(ModelRole::Reviewer)
        .with_routing_preference(RoutingPreference::PreferQuality);
        let mut viewport = OutputViewport::new();

        let preview = RequestPreviewSnapshot::from_request_with_history_limit(&request, Some(2));
        let status = request_preview_status_with_history_limit(&request, Some(2));
        let update = viewport.append_request_preview_with_history_limit(&request, Some(2));

        assert_eq!(preview.messages, 3);
        assert_eq!(preview.context_messages, 2);
        assert_eq!(preview.history_messages, 2);
        assert_eq!(preview.history_limit, Some(2));
        assert_eq!(preview.history_remaining, Some(0));
        assert_eq!(preview.history_messages_after_submit, Some(2));
        assert_eq!(preview.history_at_limit_after_submit, Some(true));
        assert_eq!(preview.history_truncates_on_submit, Some(true));
        assert_eq!(preview.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(preview.model_role_label, "reviewer");
        assert_eq!(preview.routing_preference_label, "prefer_quality");
        assert!(!preview.endpoint_pinned);
        assert!(!preview.wire_sends_model_endpoint);
        assert_eq!(status, preview.line());
        assert_eq!(
            status,
            "role=reviewer preference=prefer_quality endpoint=auto pinned=false messages=3 last_user_chars=5 max_tokens=backend-default stream=true"
        );
        assert_eq!(update.request_preview.as_ref(), Some(&preview));
        assert_eq!(
            update.appended,
            "[send] role=reviewer preference=prefer_quality endpoint=auto pinned=false messages=3 last_user_chars=5 max_tokens=backend-default stream=true"
        );
        assert_eq!(viewport.lines(), &[update.appended]);
    }

    #[test]
    fn request_preview_marks_explicit_endpoint_pin_and_renders_as_local_send_line() {
        let request = ChatRequest::new("cli", vec![ChatMessage::user("review")])
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let mut viewport = OutputViewport::new();

        let update = viewport.append_request_preview(&request);
        let preview = RequestPreviewSnapshot::from_request(&request);

        assert_eq!(preview.model_role_label, "assistant");
        assert_eq!(preview.routing_preference_label, "balanced");
        assert_eq!(preview.endpoint_label, "fast-reviewer");
        assert!(preview.endpoint_pinned);
        assert!(!preview.has_context);
        assert!(preview.is_single_turn);
        assert_eq!(preview.endpoint_kind, ModelEndpointSelectionKind::BuiltIn);
        assert_eq!(preview.endpoint_kind_label, "built_in");
        assert!(!preview.endpoint_auto);
        assert!(preview.endpoint_built_in);
        assert!(!preview.endpoint_custom);
        assert_eq!(preview.wire_model_role_label, "assistant");
        assert_eq!(preview.wire_routing_preference_label, "balanced");
        assert!(!preview.wire_prefer_fast);
        assert!(!preview.wire_prefer_quality);
        assert!(!preview.wire_sends_max_tokens);
        assert_eq!(preview.wire_max_tokens, None);
        assert!(preview.wire_endpoint_pinned);
        assert_eq!(preview.wire_endpoint_kind_label, "built_in");
        assert!(preview.wire_sends_model_endpoint);
        assert_eq!(
            preview.wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );
        let update_preview = update
            .request_preview
            .as_ref()
            .expect("send output update should carry request preview");
        assert_eq!(update_preview, &preview);
        assert_eq!(update_preview.endpoint_label, "fast-reviewer");
        assert!(update_preview.endpoint_pinned);
        assert_eq!(
            update_preview.wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );
        assert_eq!(update.gate_advice_detail, None);
        assert_eq!(update.output_label, "send");
        assert_eq!(
            update.appended,
            "[send] role=assistant preference=balanced endpoint=fast-reviewer pinned=true messages=1 last_user_chars=6 max_tokens=backend-default stream=true"
        );
        assert_eq!(update.state, StreamState::Pending);
        assert_eq!(
            viewport.lines(),
            &["[send] role=assistant preference=balanced endpoint=fast-reviewer pinned=true messages=1 last_user_chars=6 max_tokens=backend-default stream=true".to_owned()]
        );
    }

    #[test]
    fn started_turn_preview_includes_start_chunk_state_for_terminal_hosts() {
        let request = ChatRequest::new("cli", vec![ChatMessage::user("hello")]);
        let turn = StartedChatTurn {
            request,
            start: ChatChunk::start(0),
        };
        let mut viewport = OutputViewport::new();

        let update = viewport.append_started_turn_preview(&turn);
        let policy_update = viewport.append_started_turn_preview_with_history_limit(&turn, Some(1));

        assert_eq!(
            update.appended,
            "[send] role=assistant preference=balanced endpoint=auto pinned=false messages=1 last_user_chars=5 max_tokens=backend-default stream=true start_sequence=0 start_state=streaming"
        );
        assert_eq!(update.output_label, "send");
        assert_eq!(update.source, OutputUpdateSource::LocalStatus);
        assert_eq!(update.source_label, "local_status");
        assert!(!update.is_stream_chunk);
        assert!(update.is_local_status);
        assert!(!update.is_gate_advice);
        assert_eq!(update.state, StreamState::Pending);
        assert_eq!(update.state_label, "pending");
        let preview = update
            .request_preview
            .as_ref()
            .expect("started output update should carry request preview");
        assert_eq!(preview.start_sequence, Some(0));
        assert_eq!(preview.start_state, Some(StreamState::Streaming));
        assert_eq!(preview.start_state_label.as_deref(), Some("streaming"));
        assert_eq!(preview.endpoint_label, "auto");
        assert!(!preview.endpoint_pinned);
        let start_chunk = preview
            .start_chunk
            .as_ref()
            .expect("started preview should carry service chunk display snapshot");
        assert_eq!(start_chunk.kind_label, "start");
        assert_eq!(start_chunk.output_label, "start");
        assert!(!start_chunk.emits_output);
        assert!(start_chunk.state_blocks_prompt_submit);
        assert_eq!(update.gate_advice_detail, None);
        assert_eq!(update.route_update, None);
        assert_eq!(update.session_config_update_detail, None);
        assert_eq!(update.status_snapshot, None);
        assert_eq!(update.stream_outcome, None);
        assert_eq!(update.stream_chunk, None);
        assert_eq!(update.input_action_snapshot, None);
        let policy_preview = policy_update
            .request_preview
            .as_ref()
            .expect("policy started output update should carry request preview");
        assert_eq!(policy_preview.history_limit, Some(1));
        assert_eq!(policy_preview.history_remaining, Some(1));
        assert_eq!(policy_preview.history_messages_after_submit, Some(1));
        assert_eq!(policy_preview.history_at_limit_after_submit, Some(true));
        assert_eq!(policy_preview.history_truncates_on_submit, Some(false));
        assert_eq!(policy_update.appended, update.appended);
    }

    #[test]
    fn started_turn_preview_snapshot_exposes_start_boundary() {
        let request = ChatRequest::new("cli", vec![ChatMessage::user("review patch")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let turn = StartedChatTurn {
            request,
            start: ChatChunk::start(4),
        };

        let preview = RequestPreviewSnapshot::from_started_turn(&turn);
        let policy_preview =
            RequestPreviewSnapshot::from_started_turn_with_history_limit(&turn, Some(1));

        assert_eq!(preview.routing_intent.endpoint_label(), "fast-reviewer");
        assert!(preview.routing_intent.endpoint_pinned);
        assert_eq!(preview.model_role_label, "reviewer");
        assert_eq!(preview.routing_preference_label, "prefer_fast");
        assert_eq!(preview.endpoint_label, "fast-reviewer");
        assert!(preview.endpoint_pinned);
        assert_eq!(preview.endpoint_kind, ModelEndpointSelectionKind::BuiltIn);
        assert_eq!(preview.endpoint_kind_label, "built_in");
        assert!(!preview.endpoint_auto);
        assert!(preview.endpoint_built_in);
        assert!(!preview.endpoint_custom);
        assert_eq!(preview.messages, 1);
        assert_eq!(preview.context_messages, 0);
        assert_eq!(preview.history_messages, 0);
        assert_eq!(preview.context_kind, ChatRequestContextKind::SingleTurn);
        assert_eq!(preview.context_kind_label, "single_turn");
        assert!(!preview.has_context);
        assert!(preview.is_single_turn);
        assert_eq!(preview.last_user_chars, 12);
        assert_eq!(preview.max_tokens, None);
        assert_eq!(preview.max_tokens_label, "backend-default");
        assert_eq!(preview.start_sequence, Some(4));
        assert_eq!(preview.start_state, Some(StreamState::Streaming));
        assert_eq!(preview.start_state_label.as_deref(), Some("streaming"));
        assert_eq!(preview.start_state_is_terminal, Some(false));
        assert_eq!(preview.start_state_is_pressure, Some(false));
        assert_eq!(preview.start_state_blocks_prompt_submit, Some(true));
        let start_chunk = preview
            .start_chunk
            .as_ref()
            .expect("started preview should carry service chunk display snapshot");
        assert_eq!(start_chunk.sequence, 4);
        assert_eq!(start_chunk.kind_label, "start");
        assert_eq!(start_chunk.state_label, "streaming");
        assert!(start_chunk.state_blocks_prompt_submit);
        assert_eq!(policy_preview.start_sequence, Some(4));
        assert_eq!(policy_preview.history_limit, Some(1));
        assert_eq!(policy_preview.history_remaining, Some(1));
        assert_eq!(policy_preview.history_messages_after_submit, Some(1));
        assert_eq!(policy_preview.history_at_limit_after_submit, Some(true));
        assert_eq!(policy_preview.history_truncates_on_submit, Some(false));
        assert_eq!(
            started_turn_preview_status_with_history_limit(&turn, Some(1)),
            policy_preview.line()
        );
        assert_eq!(
            preview.line(),
            "role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true messages=1 last_user_chars=12 max_tokens=backend-default stream=true start_sequence=4 start_state=streaming"
        );
        assert!(!preview.line().contains("review patch"));
    }

    #[test]
    fn started_turn_preview_marks_custom_worker_pin() {
        let request = ChatRequest::new(
            "cli",
            vec![
                ChatMessage::assistant("prior"),
                ChatMessage::user("review patch"),
            ],
        )
        .with_model_role(ModelRole::Reviewer)
        .with_routing_preference(RoutingPreference::PreferFast)
        .with_model_endpoint(Some(ModelEndpoint::Worker("mlx-reviewer-8b".to_owned())));
        let turn = StartedChatTurn {
            request,
            start: ChatChunk::start(4),
        };
        let mut viewport = OutputViewport::new();

        let update = viewport.append_started_turn_preview(&turn);
        let preview = RequestPreviewSnapshot::from_started_turn(&turn);

        assert_eq!(preview.endpoint_label, "mlx-reviewer-8b");
        assert!(preview.endpoint_pinned);
        assert_eq!(preview.endpoint_kind, ModelEndpointSelectionKind::Custom);
        assert_eq!(preview.endpoint_kind_label, "custom");
        assert!(!preview.endpoint_auto);
        assert!(!preview.endpoint_built_in);
        assert!(preview.endpoint_custom);
        assert_eq!(preview.wire_model_role_label, "reviewer");
        assert_eq!(preview.wire_routing_preference_label, "prefer_fast");
        assert!(preview.wire_prefer_fast);
        assert!(!preview.wire_prefer_quality);
        assert!(!preview.wire_sends_max_tokens);
        assert_eq!(preview.wire_max_tokens, None);
        assert!(preview.wire_endpoint_pinned);
        assert_eq!(preview.wire_endpoint_kind_label, "custom");
        assert!(preview.wire_sends_model_endpoint);
        assert_eq!(
            preview.wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-8b")
        );
        assert_eq!(preview.context_messages, 1);
        assert_eq!(preview.history_messages, 1);
        assert_eq!(preview.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(preview.context_kind_label, "multi_turn");
        assert!(preview.has_context);
        assert!(!preview.is_single_turn);
        let update_preview = update
            .request_preview
            .as_ref()
            .expect("started output update should carry request preview");
        assert_eq!(update_preview, &preview);
        assert_eq!(
            update_preview.wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-8b")
        );
        assert_eq!(update_preview.context_kind_label, "multi_turn");
        assert_eq!(update.gate_advice_detail, None);
        assert_eq!(
            update.appended,
            "[send] role=reviewer preference=prefer_fast endpoint=mlx-reviewer-8b pinned=true messages=2 last_user_chars=12 max_tokens=backend-default stream=true start_sequence=4 start_state=streaming"
        );
        assert!(!update.appended.contains("review patch"));
    }

    #[test]
    fn started_turn_preview_keeps_prompt_text_out_of_local_send_line() {
        let request = ChatRequest::new(
            "cli",
            vec![
                ChatMessage::assistant("prior answer"),
                ChatMessage::user("sensitive patch details"),
            ],
        )
        .with_max_tokens(Some(8192))
        .with_model_role(ModelRole::Reviewer)
        .with_routing_preference(RoutingPreference::PreferFast);
        let turn = StartedChatTurn {
            request,
            start: ChatChunk::start(3),
        };
        let mut viewport = OutputViewport::new();

        let update = viewport.append_started_turn_preview(&turn);

        assert_eq!(
            update.appended,
            "[send] role=reviewer preference=prefer_fast endpoint=auto pinned=false messages=2 last_user_chars=23 max_tokens=8192 stream=true start_sequence=3 start_state=streaming"
        );
        assert!(!update.appended.contains("sensitive patch details"));
        assert!(!viewport.lines()[0].contains("sensitive patch details"));
    }

    #[test]
    fn outcome_status_distinguishes_completed_interrupted_and_failed() {
        let completed = StreamOutcome {
            state: StreamState::Completed,
            partial_answer: "hello".to_owned(),
            last_error: None,
            pressure_reason: None,
            history_messages: 2,
        };
        let interrupted = StreamOutcome {
            state: StreamState::Interrupted,
            partial_answer: "partial".to_owned(),
            last_error: Some("missing done".to_owned()),
            pressure_reason: None,
            history_messages: 1,
        };
        let failed = StreamOutcome {
            state: StreamState::Failed,
            partial_answer: String::new(),
            last_error: Some("backend rejected".to_owned()),
            pressure_reason: None,
            history_messages: 1,
        };

        assert_eq!(
            outcome_status(&completed),
            "completed history_messages=2 answer_chars=5"
        );
        assert_eq!(
            outcome_status(&interrupted),
            "interrupted partial_chars=7 reason=missing done"
        );
        assert_eq!(outcome_status(&failed), "failed reason=backend rejected");
    }

    #[test]
    fn outcome_snapshot_exposes_terminal_counts_and_reasons_for_ui() {
        let completed = StreamOutcome {
            state: StreamState::Completed,
            partial_answer: "final answer".to_owned(),
            last_error: None,
            pressure_reason: None,
            history_messages: 4,
        };
        let interrupted = StreamOutcome {
            state: StreamState::Interrupted,
            partial_answer: "partial".to_owned(),
            last_error: Some("backend stream closed".to_owned()),
            pressure_reason: None,
            history_messages: 3,
        };
        let backpressure = StreamOutcome {
            state: StreamState::Backpressure,
            partial_answer: String::new(),
            last_error: None,
            pressure_reason: Some("pool queue full".to_owned()),
            history_messages: 2,
        };

        let completed = StreamOutcomeSnapshot::from_outcome(&completed);
        let interrupted = StreamOutcomeSnapshot::from_outcome(&interrupted);
        let backpressure = StreamOutcomeSnapshot::from_outcome(&backpressure);

        assert_eq!(completed.state, StreamState::Completed);
        assert_eq!(completed.state_label, "completed");
        assert_eq!(completed.history_messages, 4);
        assert_eq!(completed.answer_chars, 12);
        assert_eq!(completed.partial_chars, 12);
        assert_eq!(completed.reason, None);
        assert!(completed.is_terminal);
        assert!(!completed.is_pressure);
        assert!(!completed.state_blocks_prompt_submit);
        assert!(completed.has_partial);
        assert_eq!(
            completed.line(),
            "completed history_messages=4 answer_chars=12"
        );
        assert_eq!(interrupted.state, StreamState::Interrupted);
        assert_eq!(interrupted.state_label, "interrupted");
        assert_eq!(interrupted.answer_chars, 0);
        assert_eq!(interrupted.partial_chars, 7);
        assert_eq!(
            interrupted.last_error.as_deref(),
            Some("backend stream closed")
        );
        assert_eq!(interrupted.reason.as_deref(), Some("backend stream closed"));
        assert!(interrupted.is_terminal);
        assert!(!interrupted.is_pressure);
        assert!(!interrupted.state_blocks_prompt_submit);
        assert!(interrupted.has_partial);
        assert_eq!(
            interrupted.line(),
            "interrupted partial_chars=7 reason=backend stream closed"
        );
        assert_eq!(backpressure.state, StreamState::Backpressure);
        assert_eq!(backpressure.state_label, "backpressure");
        assert_eq!(
            backpressure.pressure_reason.as_deref(),
            Some("pool queue full")
        );
        assert_eq!(backpressure.reason.as_deref(), Some("pool queue full"));
        assert!(!backpressure.is_terminal);
        assert!(backpressure.is_pressure);
        assert!(backpressure.state_blocks_prompt_submit);
        assert!(!backpressure.has_partial);
        assert_eq!(backpressure.line(), "backpressure reason=pool queue full");
    }

    #[test]
    fn stream_outcome_output_carries_structured_terminal_and_pressure_state() {
        let interrupted = StreamOutcome {
            state: StreamState::Interrupted,
            partial_answer: "partial answer".to_owned(),
            last_error: Some("missing done".to_owned()),
            pressure_reason: None,
            history_messages: 3,
        };
        let backpressure = StreamOutcome {
            state: StreamState::Backpressure,
            partial_answer: String::new(),
            last_error: None,
            pressure_reason: Some("pool queue full".to_owned()),
            history_messages: 3,
        };
        let mut viewport = OutputViewport::new();

        let interrupted_output = viewport.append_stream_outcome(&interrupted);
        let backpressure_output = viewport.append_stream_outcome(&backpressure);

        assert_eq!(
            interrupted_output.appended,
            "[outcome] interrupted partial_chars=14 reason=missing done"
        );
        assert_eq!(interrupted_output.output_label, "outcome");
        assert_eq!(interrupted_output.source, OutputUpdateSource::LocalStatus);
        assert_eq!(interrupted_output.request_preview, None);
        assert_eq!(interrupted_output.route_update, None);
        assert_eq!(interrupted_output.session_config_update_detail, None);
        assert_eq!(interrupted_output.status_snapshot, None);
        let interrupted_snapshot = interrupted_output
            .stream_outcome
            .as_ref()
            .expect("outcome output should carry structured stream outcome");
        assert_eq!(interrupted_snapshot.state, StreamState::Interrupted);
        assert_eq!(interrupted_snapshot.state_label, "interrupted");
        assert!(interrupted_snapshot.is_terminal);
        assert!(!interrupted_snapshot.is_pressure);
        assert_eq!(interrupted_snapshot.partial_chars, 14);
        assert_eq!(interrupted_snapshot.reason.as_deref(), Some("missing done"));

        assert_eq!(
            backpressure_output.appended,
            "[outcome] backpressure reason=pool queue full"
        );
        let backpressure_snapshot = backpressure_output
            .stream_outcome
            .as_ref()
            .expect("pressure outcome output should carry structured stream outcome");
        assert_eq!(backpressure_snapshot.state, StreamState::Backpressure);
        assert_eq!(backpressure_snapshot.state_label, "backpressure");
        assert!(!backpressure_snapshot.is_terminal);
        assert!(backpressure_snapshot.is_pressure);
        assert!(backpressure_snapshot.state_blocks_prompt_submit);
        assert_eq!(
            backpressure_snapshot.pressure_reason.as_deref(),
            Some("pool queue full")
        );
    }

    #[test]
    fn outcome_status_preserves_pressure_reason() {
        let backpressure = StreamOutcome {
            state: StreamState::Backpressure,
            partial_answer: String::new(),
            last_error: None,
            pressure_reason: Some("pool queue full".to_owned()),
            history_messages: 0,
        };

        assert_eq!(
            outcome_status(&backpressure),
            "backpressure reason=pool queue full"
        );
    }

    #[test]
    fn outcome_status_preserves_pressure_close_reason_after_incomplete_stream() {
        let mut session =
            norion_service::ChatSession::new("cli", norion_service::ChatSessionConfig::default());
        norion_service::apply_backend_event(&mut session, "backpressure", "pool queue full");
        let interrupted = norion_service::close_incomplete_stream(&mut session, "missing done")
            .expect("pressure close should emit interrupted chunk");

        assert_eq!(interrupted.state, StreamState::Interrupted);
        assert_eq!(
            outcome_status(&session.outcome()),
            "interrupted partial_chars=0 reason=missing done after backpressure: pool queue full"
        );
    }
}
