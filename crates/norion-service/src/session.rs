use crate::protocol::{ChatChunk, ChatMessage, ChatRequest, StreamState};

const DEFAULT_HISTORY_LIMIT: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSessionConfig {
    pub history_limit: usize,
    pub default_max_tokens: Option<usize>,
    pub tenant_id: String,
    pub workspace_id: String,
}

impl Default for ChatSessionConfig {
    fn default() -> Self {
        Self {
            history_limit: DEFAULT_HISTORY_LIMIT,
            default_max_tokens: None,
            tenant_id: "local".to_owned(),
            workspace_id: "default".to_owned(),
        }
    }
}

impl ChatSessionConfig {
    pub fn new(history_limit: usize) -> Self {
        Self {
            history_limit: history_limit.max(1),
            ..Self::default()
        }
    }

    pub fn with_default_max_tokens(mut self, max_tokens: Option<usize>) -> Self {
        self.default_max_tokens = max_tokens.map(|value| value.max(1));
        self
    }

    pub fn with_tenant_scope(
        mut self,
        tenant_id: impl AsRef<str>,
        workspace_id: impl AsRef<str>,
    ) -> Self {
        self.tenant_id = scope_value(tenant_id.as_ref(), "local");
        self.workspace_id = scope_value(workspace_id.as_ref(), "default");
        self
    }
}

#[derive(Debug, Clone)]
pub struct ChatSession {
    id: String,
    config: ChatSessionConfig,
    history: Vec<ChatMessage>,
    chunks: Vec<ChatChunk>,
    partial_answer: String,
    state: StreamState,
    next_sequence: u64,
    last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamOutcome {
    pub state: StreamState,
    pub partial_answer: String,
    pub last_error: Option<String>,
    pub pressure_reason: Option<String>,
    pub history_messages: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamOutcomeSnapshot {
    pub state: StreamState,
    pub state_label: String,
    pub history_messages: usize,
    pub answer_chars: usize,
    pub partial_chars: usize,
    pub last_error: Option<String>,
    pub pressure_reason: Option<String>,
    pub reason: Option<String>,
    pub is_terminal: bool,
    pub is_pressure: bool,
    pub state_blocks_prompt_submit: bool,
    pub has_partial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartedChatTurn {
    pub request: ChatRequest,
    pub start: ChatChunk,
}

impl StreamOutcome {
    pub fn is_complete(&self) -> bool {
        self.state == StreamState::Completed
    }

    pub fn has_partial_answer(&self) -> bool {
        !self.partial_answer.trim().is_empty()
    }

    pub fn snapshot(&self) -> StreamOutcomeSnapshot {
        StreamOutcomeSnapshot::from_outcome(self)
    }
}

impl StreamOutcomeSnapshot {
    pub fn from_outcome(outcome: &StreamOutcome) -> Self {
        let partial_chars = outcome.partial_answer.chars().count();
        let answer_chars = if outcome.state == StreamState::Completed {
            partial_chars
        } else {
            0
        };
        let reason = match outcome.state {
            StreamState::Interrupted | StreamState::Failed => outcome.last_error.clone(),
            StreamState::Queued | StreamState::Busy | StreamState::Backpressure => {
                outcome.pressure_reason.clone()
            }
            StreamState::Pending | StreamState::Streaming | StreamState::Completed => None,
        };

        Self {
            state: outcome.state,
            state_label: outcome.state.as_str().to_owned(),
            history_messages: outcome.history_messages,
            answer_chars,
            partial_chars,
            last_error: outcome.last_error.clone(),
            pressure_reason: outcome.pressure_reason.clone(),
            reason,
            is_terminal: outcome.state.is_terminal(),
            is_pressure: outcome.state.is_pressure(),
            state_blocks_prompt_submit: outcome.state.blocks_prompt_submit(),
            has_partial: partial_chars > 0,
        }
    }

    pub fn line(&self) -> String {
        match self.state {
            StreamState::Completed => format!(
                "completed history_messages={} answer_chars={}",
                self.history_messages, self.answer_chars
            ),
            StreamState::Interrupted => format!(
                "interrupted partial_chars={} reason={}",
                self.partial_chars,
                self.reason.as_deref().unwrap_or("unknown")
            ),
            StreamState::Failed => format!(
                "failed reason={}",
                self.reason.as_deref().unwrap_or("unknown")
            ),
            StreamState::Queued | StreamState::Busy | StreamState::Backpressure => {
                format!(
                    "{} reason={}",
                    self.state.as_str(),
                    self.reason.as_deref().unwrap_or("unknown")
                )
            }
            StreamState::Pending | StreamState::Streaming => self.state.as_str().to_owned(),
        }
    }
}

impl ChatSession {
    pub fn new(id: impl Into<String>, config: ChatSessionConfig) -> Self {
        Self {
            id: id.into(),
            config,
            history: Vec::new(),
            chunks: Vec::new(),
            partial_answer: String::new(),
            state: StreamState::Pending,
            next_sequence: 0,
            last_error: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn state(&self) -> StreamState {
        self.state
    }

    pub fn config(&self) -> &ChatSessionConfig {
        &self.config
    }

    pub fn history(&self) -> &[ChatMessage] {
        &self.history
    }

    pub fn chunks(&self) -> &[ChatChunk] {
        &self.chunks
    }

    pub fn partial_answer(&self) -> &str {
        &self.partial_answer
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn outcome(&self) -> StreamOutcome {
        StreamOutcome {
            state: self.state,
            partial_answer: self.partial_answer.clone(),
            last_error: self.last_error.clone(),
            pressure_reason: self
                .pressure_reason(self.state)
                .map(str::to_owned)
                .filter(|_| {
                    matches!(
                        self.state,
                        StreamState::Queued | StreamState::Busy | StreamState::Backpressure
                    )
                }),
            history_messages: self.history.len(),
        }
    }

    pub fn record_user(&mut self, content: impl Into<String>) {
        self.push_history(ChatMessage::user(content));
    }

    pub fn record_assistant(&mut self, content: impl Into<String>) {
        self.push_history(ChatMessage::assistant(content));
    }

    pub fn set_history_limit(&mut self, history_limit: usize) {
        self.config.history_limit = history_limit.max(1);
        self.truncate_history_to_limit();
    }

    pub fn set_default_max_tokens(&mut self, max_tokens: Option<usize>) {
        self.config.default_max_tokens = max_tokens.map(|value| value.max(1));
    }

    pub fn request_for_prompt(&self, prompt: impl Into<String>) -> ChatRequest {
        self.request_for_prompt_with_max_tokens(prompt, self.config.default_max_tokens)
    }

    pub fn request_for_prompt_with_max_tokens(
        &self,
        prompt: impl Into<String>,
        max_tokens: Option<usize>,
    ) -> ChatRequest {
        let mut messages = self.history.clone();
        messages.push(ChatMessage::user(prompt));
        ChatRequest::new(self.id.clone(), messages)
            .with_tenant_scope(&self.config.tenant_id, &self.config.workspace_id)
            .with_max_tokens(max_tokens)
    }

    pub fn submit_prompt(&mut self, prompt: impl Into<String>) -> ChatRequest {
        self.submit_prompt_with_max_tokens(prompt, self.config.default_max_tokens)
    }

    pub fn can_submit_prompt(&self) -> bool {
        self.prompt_blocked_chunk().is_none()
    }

    pub fn prompt_blocked_chunk(&self) -> Option<ChatChunk> {
        match self.state {
            StreamState::Queued => Some(ChatChunk::queued(
                self.next_sequence,
                self.pressure_reason(StreamState::Queued)
                    .unwrap_or("session request is queued"),
            )),
            StreamState::Busy => Some(ChatChunk::busy(
                self.next_sequence,
                self.pressure_reason(StreamState::Busy)
                    .unwrap_or("session is busy"),
            )),
            StreamState::Backpressure => Some(ChatChunk::backpressure(
                self.next_sequence,
                self.pressure_reason(StreamState::Backpressure)
                    .unwrap_or("session is under backpressure"),
            )),
            StreamState::Streaming => Some(ChatChunk::busy(
                self.next_sequence,
                "session stream is already active",
            )),
            StreamState::Pending
            | StreamState::Completed
            | StreamState::Interrupted
            | StreamState::Failed => None,
        }
    }

    pub fn try_submit_prompt(
        &mut self,
        prompt: impl Into<String>,
    ) -> Result<ChatRequest, ChatChunk> {
        self.try_submit_prompt_with_max_tokens(prompt, self.config.default_max_tokens)
    }

    pub fn try_submit_prompt_with_max_tokens(
        &mut self,
        prompt: impl Into<String>,
        max_tokens: Option<usize>,
    ) -> Result<ChatRequest, ChatChunk> {
        if let Some(blocked) = self.prompt_blocked_chunk() {
            return Err(blocked);
        }
        Ok(self.submit_prompt_with_max_tokens(prompt, max_tokens))
    }

    pub fn try_submit_and_begin_stream(
        &mut self,
        prompt: impl Into<String>,
    ) -> Result<StartedChatTurn, ChatChunk> {
        self.try_submit_and_begin_stream_with_max_tokens(prompt, self.config.default_max_tokens)
    }

    pub fn try_submit_and_begin_stream_with_max_tokens(
        &mut self,
        prompt: impl Into<String>,
        max_tokens: Option<usize>,
    ) -> Result<StartedChatTurn, ChatChunk> {
        let request = self.try_submit_prompt_with_max_tokens(prompt, max_tokens)?;
        let start = self.begin_stream();
        Ok(StartedChatTurn { request, start })
    }

    pub fn submit_prompt_with_max_tokens(
        &mut self,
        prompt: impl Into<String>,
        max_tokens: Option<usize>,
    ) -> ChatRequest {
        let prompt = prompt.into();
        let request = self.request_for_prompt_with_max_tokens(prompt.clone(), max_tokens);
        self.record_user(prompt);
        request
    }

    pub fn begin_stream(&mut self) -> ChatChunk {
        self.partial_answer.clear();
        self.last_error = None;
        self.state = StreamState::Streaming;
        let sequence = self.take_sequence();
        self.emit(ChatChunk::start(sequence))
    }

    pub fn queued(&mut self, reason: impl Into<String>) -> ChatChunk {
        if self.state.is_terminal() {
            return self.ignored_terminal_chunk();
        }
        self.state = StreamState::Queued;
        let sequence = self.take_sequence();
        self.emit(ChatChunk::queued(sequence, reason))
    }

    pub fn busy(&mut self, reason: impl Into<String>) -> ChatChunk {
        if self.state.is_terminal() {
            return self.ignored_terminal_chunk();
        }
        self.state = StreamState::Busy;
        let sequence = self.take_sequence();
        self.emit(ChatChunk::busy(sequence, reason))
    }

    pub fn backpressure(&mut self, reason: impl Into<String>) -> ChatChunk {
        if self.state.is_terminal() {
            return self.ignored_terminal_chunk();
        }
        self.state = StreamState::Backpressure;
        let sequence = self.take_sequence();
        self.emit(ChatChunk::backpressure(sequence, reason))
    }

    pub fn push_delta(&mut self, content: impl Into<String>) -> ChatChunk {
        if self.state.is_terminal() {
            return self.ignored_terminal_chunk();
        }
        let content = content.into();
        self.partial_answer.push_str(&content);
        self.state = StreamState::Streaming;
        let sequence = self.take_sequence();
        self.emit(ChatChunk::delta(sequence, content))
    }

    pub fn push_status(&mut self, content: impl Into<String>) -> ChatChunk {
        if self.state.is_terminal() {
            return self.ignored_terminal_chunk();
        }
        let sequence = self.take_sequence();
        self.emit(ChatChunk::status(sequence, content))
    }

    pub fn push_metadata(&mut self, content: impl Into<String>) -> ChatChunk {
        if self.state.is_terminal() {
            return self.ignored_terminal_chunk();
        }
        let sequence = self.take_sequence();
        self.emit(ChatChunk::metadata(sequence, content))
    }

    pub fn push_final_payload(&mut self, content: impl Into<String>) -> ChatChunk {
        if self.state.is_terminal() {
            return self.ignored_terminal_chunk();
        }
        let sequence = self.take_sequence();
        self.emit(ChatChunk::final_payload(sequence, content))
    }

    pub fn reconcile_final_answer(&mut self, answer: impl Into<String>) {
        if self.state.is_terminal() {
            return;
        }
        let answer = answer.into();
        if !answer.trim().is_empty() {
            self.partial_answer = answer;
            self.state = StreamState::Streaming;
        }
    }

    pub fn push_final_payload_with_answer(
        &mut self,
        content: impl Into<String>,
        answer: impl Into<String>,
    ) -> ChatChunk {
        self.reconcile_final_answer(answer);
        self.push_final_payload(content)
    }

    pub fn cancel_stream(&mut self) -> Option<ChatChunk> {
        if self.state == StreamState::Pending || self.state.is_terminal() {
            return None;
        }
        Some(self.interrupt("stream cancelled by user"))
    }

    pub fn finish(&mut self) -> ChatChunk {
        if self.state.is_terminal() {
            return self.ignored_terminal_chunk();
        }
        self.state = StreamState::Completed;
        if !self.partial_answer.trim().is_empty() {
            self.record_assistant(self.partial_answer.clone());
        }
        let sequence = self.take_sequence();
        self.emit(ChatChunk::done(sequence))
    }

    pub fn interrupt(&mut self, reason: impl Into<String>) -> ChatChunk {
        if self.state.is_terminal() {
            return self.ignored_terminal_chunk();
        }
        let reason = reason.into();
        self.state = StreamState::Interrupted;
        self.last_error = Some(reason.clone());
        let sequence = self.take_sequence();
        self.emit(ChatChunk::interrupted(sequence, reason))
    }

    pub fn fail(&mut self, reason: impl Into<String>) -> ChatChunk {
        if self.state.is_terminal() {
            return self.ignored_terminal_chunk();
        }
        let reason = reason.into();
        self.state = StreamState::Failed;
        self.last_error = Some(reason.clone());
        let sequence = self.take_sequence();
        self.emit(ChatChunk::failed(sequence, reason))
    }

    fn push_history(&mut self, message: ChatMessage) {
        self.history.push(message);
        self.truncate_history_to_limit();
    }

    fn truncate_history_to_limit(&mut self) {
        let overflow = self
            .history
            .len()
            .saturating_sub(self.config.history_limit.max(1));
        if overflow > 0 {
            self.history.drain(..overflow);
        }
    }

    fn take_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        sequence
    }

    fn emit(&mut self, chunk: ChatChunk) -> ChatChunk {
        self.chunks.push(chunk.clone());
        chunk
    }

    fn ignored_terminal_chunk(&self) -> ChatChunk {
        ChatChunk::new(
            self.next_sequence,
            self.state,
            crate::ChatChunkKind::Status,
            "",
        )
    }

    fn pressure_reason(&self, state: StreamState) -> Option<&str> {
        self.chunks
            .iter()
            .rev()
            .find(|chunk| chunk.state == state && !chunk.content.trim().is_empty())
            .map(|chunk| chunk.content.as_str())
    }
}

fn scope_value(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_owned()
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_are_emitted_in_order() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.begin_stream();
        session.push_delta("hel");
        session.push_delta("lo");
        session.push_metadata("{\"tokens\":2}");
        session.push_final_payload("{\"answer\":\"hello\"}");
        session.finish();

        let sequences = session
            .chunks()
            .iter()
            .map(|chunk| chunk.sequence)
            .collect::<Vec<_>>();
        assert_eq!(sequences, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(session.partial_answer(), "hello");
        assert_eq!(session.chunks()[3].kind.as_str(), "metadata");
        assert_eq!(session.chunks()[4].kind.as_str(), "final");
    }

    #[test]
    fn interrupted_stream_keeps_partial_answer() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.begin_stream();
        session.push_delta("partial ");
        session.push_delta("answer");
        let chunk = session.interrupt("backend stream closed");

        assert_eq!(chunk.state, StreamState::Interrupted);
        assert_eq!(session.state(), StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial answer");
        assert_eq!(session.last_error(), Some("backend stream closed"));
    }

    #[test]
    fn history_limit_is_configurable() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::new(3));

        session.record_user("one");
        session.record_assistant("two");
        session.record_user("three");
        session.record_assistant("four");

        let contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(contents, vec!["two", "three", "four"]);
    }

    #[test]
    fn runtime_history_limit_update_truncates_existing_context() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::new(8));
        session.record_user("one");
        session.record_assistant("two");
        session.record_user("three");
        session.record_assistant("four");

        session.set_history_limit(2);

        let contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(session.config().history_limit, 2);
        assert_eq!(contents, vec!["three", "four"]);
    }

    #[test]
    fn runtime_max_tokens_update_preserves_optional_backend_default() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.set_default_max_tokens(Some(0));
        let clamped = session.request_for_prompt("hello");
        session.set_default_max_tokens(None);
        let backend_default = session.request_for_prompt("hello");

        assert_eq!(clamped.max_tokens, Some(1));
        assert_eq!(backend_default.max_tokens, None);
        assert_ne!(backend_default.max_tokens, Some(128));
    }

    #[test]
    fn max_tokens_is_passed_without_128_default() {
        let session = ChatSession::new(
            "s1",
            ChatSessionConfig::new(8).with_default_max_tokens(Some(4096)),
        );

        let request = session.request_for_prompt("hello");
        let explicit = session.request_for_prompt_with_max_tokens("hello", Some(8192));
        let default_session = ChatSession::new("s2", ChatSessionConfig::default());

        assert_eq!(request.max_tokens, Some(4096));
        assert_eq!(explicit.max_tokens, Some(8192));
        assert_ne!(request.max_tokens, Some(128));
        assert_eq!(default_session.request_for_prompt("hello").max_tokens, None);
    }

    #[test]
    fn session_request_carries_configured_tenant_scope() {
        let session = ChatSession::new(
            "session-1",
            ChatSessionConfig::new(8).with_tenant_scope("tenant-a", "workspace-one"),
        );

        let request = session.request_for_prompt("review this");

        assert_eq!(request.tenant_id, "tenant-a");
        assert_eq!(request.workspace_id, "workspace-one");
        assert_eq!(request.session_id, "session-1");
    }

    #[test]
    fn request_can_express_model_routing_without_losing_token_budget() {
        let session = ChatSession::new(
            "s1",
            ChatSessionConfig::default().with_default_max_tokens(Some(8192)),
        );

        let request = session
            .request_for_prompt("review this")
            .prefer_fast()
            .with_model_role(crate::ModelRole::Reviewer)
            .with_model_endpoint(Some(crate::ModelEndpoint::FastReviewer));

        assert_eq!(request.max_tokens, Some(8192));
        assert_eq!(
            request.routing_preference,
            crate::RoutingPreference::PreferFast
        );
        assert_eq!(request.model_role, crate::ModelRole::Reviewer);
        assert_eq!(
            request
                .model_endpoint
                .as_ref()
                .map(|endpoint| endpoint.label()),
            Some("fast-reviewer")
        );
    }

    #[test]
    fn submitted_prompt_keeps_history_tokens_and_unpinned_route_hints() {
        let mut session = ChatSession::new(
            "s1",
            ChatSessionConfig::new(8).with_default_max_tokens(Some(8192)),
        );
        session.record_user("first question");
        session.record_assistant("first answer");

        let request = session
            .submit_prompt("review the next patch")
            .with_model_role(crate::ModelRole::Reviewer)
            .with_routing_preference(crate::RoutingPreference::PreferFast);

        let request_contents = request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        let history_contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        let intent = request.routing_intent();

        assert_eq!(
            request_contents,
            vec!["first question", "first answer", "review the next patch"]
        );
        assert_eq!(history_contents, request_contents);
        assert_eq!(request.max_tokens, Some(8192));
        assert_eq!(request.model_role, crate::ModelRole::Reviewer);
        assert_eq!(
            request.routing_preference,
            crate::RoutingPreference::PreferFast
        );
        assert_eq!(intent.endpoint_label(), "auto");
        assert!(!intent.endpoint_pinned);
        assert!(!request.endpoint_pinned());
    }

    #[test]
    fn queued_busy_and_backpressure_are_non_terminal_stream_states() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        let queued = session.queued("waiting for quality worker");
        let busy = session.busy("quality worker is busy");
        let backpressure = session.backpressure("too many queued streams");

        assert_eq!(queued.state, StreamState::Queued);
        assert_eq!(busy.state, StreamState::Busy);
        assert_eq!(backpressure.state, StreamState::Backpressure);
        assert!(!queued.state.is_terminal());
        assert!(!busy.state.is_terminal());
        assert!(!backpressure.state.is_terminal());
        assert_eq!(session.state(), StreamState::Backpressure);
    }

    #[test]
    fn submit_prompt_records_user_once_and_request_includes_history() {
        let mut session = ChatSession::new(
            "s1",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );
        session.record_user("earlier");
        session.record_assistant("context");

        let request = session.submit_prompt("next");

        let request_contents = request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        let history_contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();

        assert_eq!(request_contents, vec!["earlier", "context", "next"]);
        assert_eq!(history_contents, vec!["earlier", "context", "next"]);
        assert_eq!(request.max_tokens, Some(4096));
    }

    #[test]
    fn try_submit_prompt_blocks_active_stream_without_recording_user() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        let blocked = session
            .try_submit_prompt("second")
            .expect_err("active stream should block submit");

        assert_eq!(blocked.state, StreamState::Busy);
        assert_eq!(blocked.content, "session stream is already active");
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "hello");
    }

    #[test]
    fn try_submit_prompt_allows_next_turn_after_terminal_state() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        session.push_delta("hi");
        session.finish();
        let request = session
            .try_submit_prompt("next")
            .expect("completed stream should allow next turn");

        let contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(request.messages.last().unwrap().content, "next");
        assert_eq!(contents, vec!["hello", "hi", "next"]);
    }

    #[test]
    fn try_submit_and_begin_stream_recovers_after_interrupted_or_failed_turn() {
        let mut interrupted = ChatSession::new("interrupted", ChatSessionConfig::default());
        interrupted.submit_prompt("hello");
        interrupted.begin_stream();
        interrupted.push_delta("partial");
        interrupted.interrupt("backend closed");

        let resumed = interrupted
            .try_submit_and_begin_stream("next")
            .expect("interrupted stream should allow next turn");

        let resumed_contents = resumed
            .request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(resumed_contents, vec!["hello", "next"]);
        assert!(!resumed_contents.contains(&"partial"));
        assert_eq!(resumed.request.messages.last().unwrap().content, "next");
        assert_eq!(resumed.start.state, StreamState::Streaming);
        assert_eq!(interrupted.state(), StreamState::Streaming);
        assert_eq!(interrupted.partial_answer(), "");
        assert_eq!(interrupted.last_error(), None);
        assert_eq!(interrupted.history().len(), 2);
        assert_eq!(interrupted.history()[0].content, "hello");
        assert_eq!(interrupted.history()[1].content, "next");

        let mut failed = ChatSession::new("failed", ChatSessionConfig::default());
        failed.begin_stream();
        failed.fail("safe-device gate failed");

        let restarted = failed
            .try_submit_and_begin_stream("repair and retry")
            .expect("failed stream should allow next turn");

        assert_eq!(
            restarted.request.messages.last().unwrap().content,
            "repair and retry"
        );
        assert_eq!(failed.state(), StreamState::Streaming);
        assert_eq!(failed.partial_answer(), "");
        assert_eq!(failed.last_error(), None);
        assert_eq!(failed.history().last().unwrap().content, "repair and retry");
    }

    #[test]
    fn failed_stream_recovery_keeps_error_out_of_context_and_preserves_token_budget() {
        let mut session = ChatSession::new(
            "failed",
            ChatSessionConfig::default().with_default_max_tokens(Some(2048)),
        );
        session.submit_prompt("unsafe request");
        session.begin_stream();
        session.fail("safe-device gate failed");

        let restarted = session
            .try_submit_and_begin_stream("repair and retry")
            .expect("failed stream should allow next turn");
        let request_contents = restarted
            .request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        let history_contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();

        assert_eq!(request_contents, vec!["unsafe request", "repair and retry"]);
        assert!(!request_contents.contains(&"safe-device gate failed"));
        assert_eq!(history_contents, request_contents);
        assert_eq!(restarted.request.max_tokens, Some(2048));
        assert_eq!(restarted.start.state, StreamState::Streaming);
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.partial_answer(), "");
        assert_eq!(session.last_error(), None);
    }

    #[test]
    fn failed_stream_recovery_respects_history_limit_without_error_context() {
        let mut session = ChatSession::new(
            "failed",
            ChatSessionConfig::new(2).with_default_max_tokens(Some(2048)),
        );
        session.record_user("old user");
        session.record_assistant("old answer");
        session.submit_prompt("unsafe request");
        session.begin_stream();
        session.fail("safe-device gate failed");

        let restarted = session
            .try_submit_and_begin_stream("repair and retry")
            .expect("failed stream should allow next turn");
        let request_contents = restarted
            .request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        let history_contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            request_contents,
            vec!["old answer", "unsafe request", "repair and retry"]
        );
        assert!(!request_contents.contains(&"old user"));
        assert!(!request_contents.contains(&"safe-device gate failed"));
        assert_eq!(history_contents, vec!["unsafe request", "repair and retry"]);
        assert_eq!(session.config().history_limit, 2);
        assert_eq!(restarted.request.max_tokens, Some(2048));
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.partial_answer(), "");
        assert_eq!(session.last_error(), None);
    }

    #[test]
    fn try_submit_and_begin_stream_records_user_and_emits_start_atomically() {
        let mut session = ChatSession::new(
            "s1",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );

        let turn = session
            .try_submit_and_begin_stream("hello")
            .expect("expected started turn");

        assert_eq!(turn.request.messages.last().unwrap().content, "hello");
        assert_eq!(turn.request.max_tokens, Some(4096));
        assert_eq!(turn.start.sequence, 0);
        assert_eq!(turn.start.state, StreamState::Streaming);
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.history().last().unwrap().content, "hello");
        assert_eq!(session.partial_answer(), "");
        assert_eq!(session.chunks(), &[turn.start]);
    }

    #[test]
    fn try_submit_and_begin_stream_blocks_active_stream_without_recording_user() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session
            .try_submit_and_begin_stream("first")
            .expect("expected first turn");
        let blocked = session
            .try_submit_and_begin_stream("second")
            .expect_err("active stream should block second turn");

        assert_eq!(blocked.state, StreamState::Busy);
        assert_eq!(blocked.content, "session stream is already active");
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "first");
        assert_eq!(session.chunks().len(), 1);
    }

    #[test]
    fn prompt_blocked_chunk_reflects_pressure_state_without_mutating_chunks() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.backpressure("pool saturated");
        let blocked = session.prompt_blocked_chunk().unwrap();

        assert_eq!(blocked.state, StreamState::Backpressure);
        assert_eq!(blocked.content, "pool saturated");
        assert_eq!(session.chunks().len(), 1);
        assert!(!session.can_submit_prompt());
    }

    #[test]
    fn prompt_blocked_chunk_preserves_latest_pressure_reason() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.queued("waiting for reviewer");
        session.busy("worker fast-reviewer is busy");
        let busy = session.prompt_blocked_chunk().unwrap();
        session.backpressure("pool queue full");
        let backpressure = session.prompt_blocked_chunk().unwrap();

        assert_eq!(busy.state, StreamState::Busy);
        assert_eq!(busy.content, "worker fast-reviewer is busy");
        assert_eq!(backpressure.state, StreamState::Backpressure);
        assert_eq!(backpressure.content, "pool queue full");
        assert_eq!(session.chunks().len(), 3);
    }

    #[test]
    fn pressure_states_block_submit_without_recording_prompt_or_stream() {
        for (state, reason) in [
            (StreamState::Queued, "waiting for worker"),
            (StreamState::Busy, "engine_busy: #77 chat-stream"),
            (
                StreamState::Backpressure,
                "matching model workers are saturated",
            ),
        ] {
            let mut session = ChatSession::new(
                "s1",
                ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
            );
            match state {
                StreamState::Queued => {
                    session.queued(reason);
                }
                StreamState::Busy => {
                    session.busy(reason);
                }
                StreamState::Backpressure => {
                    session.backpressure(reason);
                }
                _ => unreachable!("test only covers pressure states"),
            }
            let chunks_after_pressure = session.chunks().len();

            let blocked = session
                .try_submit_and_begin_stream("should not record")
                .expect_err("pressure state should block prompt submit");

            assert_eq!(blocked.state, state);
            assert_eq!(blocked.content, reason);
            assert_eq!(session.state(), state);
            assert!(session.history().is_empty());
            assert_eq!(session.chunks().len(), chunks_after_pressure);
            assert_eq!(session.partial_answer(), "");
            assert_eq!(session.last_error(), None);
            assert!(!session.can_submit_prompt());
            assert_eq!(session.config().default_max_tokens, Some(4096));
        }
    }

    #[test]
    fn submitted_prompt_pairs_with_finished_assistant_answer() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        session.push_delta("hi");
        session.finish();

        let history = session
            .history()
            .iter()
            .map(|message| (message.role, message.content.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            history,
            vec![
                (crate::ChatRole::User, "hello"),
                (crate::ChatRole::Assistant, "hi"),
            ]
        );
    }

    #[test]
    fn final_answer_replaces_streamed_partial_before_done_records_history() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        session.push_delta("draft");
        let final_payload =
            session.push_final_payload_with_answer("{\"answer\":\"polished\"}", "polished");
        session.finish();

        assert_eq!(final_payload.kind.as_str(), "final");
        assert_eq!(session.partial_answer(), "polished");
        assert_eq!(session.history().last().unwrap().content, "polished");
    }

    #[test]
    fn empty_final_answer_does_not_replace_streamed_partial() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.begin_stream();
        session.push_delta("draft");
        session.push_final_payload_with_answer("{\"answer\":\"\"}", " ");
        session.finish();

        assert_eq!(session.partial_answer(), "draft");
        assert_eq!(session.history().last().unwrap().content, "draft");
    }

    #[test]
    fn outcome_summarizes_completed_stream() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        session.push_delta("hi");
        session.finish();

        let outcome = session.outcome();
        assert!(outcome.is_complete());
        assert!(outcome.has_partial_answer());
        assert_eq!(outcome.state, StreamState::Completed);
        assert_eq!(outcome.partial_answer, "hi");
        assert_eq!(outcome.history_messages, 2);
        assert_eq!(outcome.last_error, None);
    }

    #[test]
    fn outcome_summarizes_interrupted_partial_stream() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.begin_stream();
        session.push_delta("partial");
        session.interrupt("missing done");

        let outcome = session.outcome();
        assert!(!outcome.is_complete());
        assert!(outcome.has_partial_answer());
        assert_eq!(outcome.state, StreamState::Interrupted);
        assert_eq!(outcome.last_error.as_deref(), Some("missing done"));
        assert_eq!(outcome.history_messages, 0);
    }

    #[test]
    fn outcome_summarizes_pressure_reason_without_error() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.backpressure("pool queue full");

        let outcome = session.outcome();
        assert_eq!(outcome.state, StreamState::Backpressure);
        assert_eq!(outcome.pressure_reason.as_deref(), Some("pool queue full"));
        assert_eq!(outcome.last_error, None);
        assert_eq!(outcome.partial_answer, "");
    }

    #[test]
    fn outcome_snapshot_exposes_terminal_partial_and_pressure_display_fields() {
        let completed = StreamOutcome {
            state: StreamState::Completed,
            partial_answer: "final answer".to_owned(),
            last_error: None,
            pressure_reason: None,
            history_messages: 4,
        }
        .snapshot();
        let interrupted = StreamOutcome {
            state: StreamState::Interrupted,
            partial_answer: "partial".to_owned(),
            last_error: Some("backend stream closed".to_owned()),
            pressure_reason: None,
            history_messages: 3,
        }
        .snapshot();
        let backpressure = StreamOutcome {
            state: StreamState::Backpressure,
            partial_answer: String::new(),
            last_error: None,
            pressure_reason: Some("pool queue full".to_owned()),
            history_messages: 2,
        }
        .snapshot();

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
    fn timeout_interrupted_snapshot_keeps_retry_gate_open_without_partial_context() {
        let mut session = ChatSession::new(
            "s1",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );

        session.submit_prompt("hello");
        session.begin_stream();
        session.push_delta("partial");
        session.interrupt("read timeout");

        let snapshot = session.outcome().snapshot();
        assert_eq!(snapshot.state, StreamState::Interrupted);
        assert_eq!(snapshot.state_label, "interrupted");
        assert_eq!(snapshot.partial_chars, 7);
        assert_eq!(snapshot.answer_chars, 0);
        assert_eq!(snapshot.reason.as_deref(), Some("read timeout"));
        assert_eq!(snapshot.last_error.as_deref(), Some("read timeout"));
        assert!(snapshot.is_terminal);
        assert!(!snapshot.is_pressure);
        assert!(!snapshot.state_blocks_prompt_submit);
        assert!(snapshot.has_partial);
        assert_eq!(
            snapshot.line(),
            "interrupted partial_chars=7 reason=read timeout"
        );
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "hello");

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
        assert_eq!(retry.request.max_tokens, Some(4096));
        assert_eq!(retry.start.state, StreamState::Streaming);
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.partial_answer(), "");
        assert_eq!(session.last_error(), None);
    }

    #[test]
    fn cancel_stream_interrupts_active_stream_and_keeps_partial_without_history() {
        let mut session = ChatSession::new(
            "s1",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );

        session.submit_prompt("hello");
        session.begin_stream();
        session.push_delta("partial");
        let cancelled = session.cancel_stream().expect("expected cancel chunk");

        assert_eq!(cancelled.state, StreamState::Interrupted);
        assert_eq!(cancelled.content, "stream cancelled by user");
        assert_eq!(session.state(), StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.last_error(), Some("stream cancelled by user"));
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "hello");
        let snapshot = session.outcome().snapshot();
        assert_eq!(snapshot.state, StreamState::Interrupted);
        assert_eq!(snapshot.state_label, "interrupted");
        assert_eq!(snapshot.partial_chars, 7);
        assert_eq!(snapshot.answer_chars, 0);
        assert_eq!(snapshot.reason.as_deref(), Some("stream cancelled by user"));
        assert_eq!(
            snapshot.last_error.as_deref(),
            Some("stream cancelled by user")
        );
        assert!(snapshot.is_terminal);
        assert!(!snapshot.is_pressure);
        assert!(!snapshot.state_blocks_prompt_submit);
        assert!(snapshot.has_partial);
        assert_eq!(
            snapshot.line(),
            "interrupted partial_chars=7 reason=stream cancelled by user"
        );

        let resumed = session
            .try_submit_and_begin_stream("next")
            .expect("cancelled stream should allow the next turn");
        let request_contents = resumed
            .request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        let history_contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();

        assert_eq!(request_contents, vec!["hello", "next"]);
        assert!(!request_contents.contains(&"partial"));
        assert_eq!(history_contents, request_contents);
        assert_eq!(resumed.request.max_tokens, Some(4096));
        assert_eq!(resumed.start.state, StreamState::Streaming);
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.partial_answer(), "");
        assert_eq!(session.last_error(), None);
    }

    #[test]
    fn cancel_stream_recovery_respects_history_limit_without_partial_context() {
        let mut session = ChatSession::new(
            "s1",
            ChatSessionConfig::new(2).with_default_max_tokens(Some(4096)),
        );
        session.record_user("old user");
        session.record_assistant("old answer");
        session.submit_prompt("cancelled request");
        session.begin_stream();
        session.push_delta("partial answer");
        session.cancel_stream().expect("expected cancel chunk");

        let resumed = session
            .try_submit_and_begin_stream("next")
            .expect("cancelled stream should allow the next turn");
        let request_contents = resumed
            .request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        let history_contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            request_contents,
            vec!["old answer", "cancelled request", "next"]
        );
        assert!(!request_contents.contains(&"old user"));
        assert!(!request_contents.contains(&"partial answer"));
        assert_eq!(history_contents, vec!["cancelled request", "next"]);
        assert_eq!(session.config().history_limit, 2);
        assert_eq!(resumed.request.max_tokens, Some(4096));
        assert_eq!(resumed.start.state, StreamState::Streaming);
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.partial_answer(), "");
        assert_eq!(session.last_error(), None);
    }

    #[test]
    fn cancel_stream_interrupts_pressure_states_and_retry_drops_pressure_reason() {
        for (state, reason) in [
            (StreamState::Queued, "waiting for worker"),
            (StreamState::Busy, "backend engine is busy: #77 chat-stream"),
            (StreamState::Backpressure, "pool queue full"),
        ] {
            let mut session = ChatSession::new(
                state.as_str(),
                ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
            );
            session.record_user("hello");
            match state {
                StreamState::Queued => {
                    session.queued(reason);
                }
                StreamState::Busy => {
                    session.busy(reason);
                }
                StreamState::Backpressure => {
                    session.backpressure(reason);
                }
                _ => unreachable!("test only covers pressure states"),
            }
            let chunks_after_pressure = session.chunks().len();

            let cancelled = session.cancel_stream().expect("expected cancel chunk");
            let snapshot = session.outcome().snapshot();

            assert_eq!(cancelled.state, StreamState::Interrupted, "{state:?}");
            assert_eq!(cancelled.content, "stream cancelled by user", "{state:?}");
            assert_eq!(session.state(), StreamState::Interrupted, "{state:?}");
            assert_eq!(
                session.last_error(),
                Some("stream cancelled by user"),
                "{state:?}"
            );
            assert_eq!(session.partial_answer(), "", "{state:?}");
            assert_eq!(session.history().len(), 1, "{state:?}");
            assert_eq!(session.history()[0].content, "hello", "{state:?}");
            assert_eq!(
                session.chunks().len(),
                chunks_after_pressure + 1,
                "{state:?}"
            );
            assert_eq!(snapshot.state, StreamState::Interrupted, "{state:?}");
            assert_eq!(
                snapshot.reason.as_deref(),
                Some("stream cancelled by user"),
                "{state:?}"
            );
            assert_eq!(snapshot.pressure_reason, None, "{state:?}");
            assert!(!snapshot.is_pressure, "{state:?}");
            assert!(!snapshot.state_blocks_prompt_submit, "{state:?}");

            let retry = session
                .try_submit_and_begin_stream("retry")
                .expect("cancelled pressure state should allow retry");
            let request_contents = retry
                .request
                .messages
                .iter()
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>();

            assert_eq!(request_contents, vec!["hello", "retry"], "{state:?}");
            assert!(!request_contents.contains(&reason), "{state:?}");
            assert!(
                !request_contents.contains(&"stream cancelled by user"),
                "{state:?}"
            );
            assert_eq!(retry.request.max_tokens, Some(4096), "{state:?}");
            assert_eq!(retry.start.state, StreamState::Streaming, "{state:?}");
            assert_eq!(session.state(), StreamState::Streaming, "{state:?}");
            assert_eq!(session.last_error(), None, "{state:?}");
        }
    }

    #[test]
    fn cancel_stream_noops_when_pending_or_terminal() {
        let mut pending = ChatSession::new("pending", ChatSessionConfig::default());
        let mut completed = ChatSession::new("completed", ChatSessionConfig::default());
        completed.begin_stream();
        completed.push_delta("done");
        completed.finish();
        let chunks_after_done = completed.chunks().len();
        let history_after_done = completed.history().len();
        let partial_after_done = completed.partial_answer().to_owned();
        let mut interrupted = ChatSession::new("interrupted", ChatSessionConfig::default());
        interrupted.begin_stream();
        interrupted.push_delta("partial");
        interrupted.interrupt("missing done");
        let chunks_after_interrupt = interrupted.chunks().len();
        let partial_after_interrupt = interrupted.partial_answer().to_owned();
        let mut failed = ChatSession::new("failed", ChatSessionConfig::default());
        failed.begin_stream();
        failed.fail("safe-device gate failed");
        let chunks_after_fail = failed.chunks().len();
        let error_after_fail = failed.last_error().map(str::to_owned);

        assert_eq!(pending.cancel_stream(), None);
        assert_eq!(completed.cancel_stream(), None);
        assert_eq!(interrupted.cancel_stream(), None);
        assert_eq!(failed.cancel_stream(), None);
        assert_eq!(pending.state(), StreamState::Pending);
        assert_eq!(completed.state(), StreamState::Completed);
        assert_eq!(interrupted.state(), StreamState::Interrupted);
        assert_eq!(failed.state(), StreamState::Failed);
        assert_eq!(completed.chunks().len(), chunks_after_done);
        assert_eq!(completed.history().len(), history_after_done);
        assert_eq!(completed.partial_answer(), partial_after_done);
        assert_eq!(interrupted.chunks().len(), chunks_after_interrupt);
        assert_eq!(interrupted.partial_answer(), partial_after_interrupt);
        assert_eq!(interrupted.last_error(), Some("missing done"));
        assert_eq!(failed.chunks().len(), chunks_after_fail);
        assert_eq!(failed.last_error().map(str::to_owned), error_after_fail);
    }

    #[test]
    fn duplicate_done_does_not_record_assistant_twice() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        session.push_delta("hi");
        let first_done = session.finish();
        let second_done = session.finish();

        let assistant_messages = session
            .history()
            .iter()
            .filter(|message| message.role == crate::ChatRole::Assistant)
            .count();
        assert_eq!(first_done.state, StreamState::Completed);
        assert_eq!(second_done.state, StreamState::Completed);
        assert_eq!(assistant_messages, 1);
        assert_eq!(session.history().last().unwrap().content, "hi");
        assert_eq!(session.chunks().len(), 3);
    }

    #[test]
    fn late_events_after_completed_do_not_mutate_stream_state_or_history() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        session.push_delta("hi");
        session.finish();
        let chunks_after_done = session.chunks().len();

        let late_delta = session.push_delta(" late");
        session.push_final_payload_with_answer("{\"answer\":\"late\"}", "late");
        session.fail("late error");

        assert_eq!(late_delta.state, StreamState::Completed);
        assert_eq!(late_delta.content, "");
        assert_eq!(session.state(), StreamState::Completed);
        assert_eq!(session.partial_answer(), "hi");
        assert_eq!(session.history().last().unwrap().content, "hi");
        assert_eq!(session.last_error(), None);
        assert_eq!(session.chunks().len(), chunks_after_done);
    }

    #[test]
    fn late_done_after_interrupted_does_not_convert_to_completed() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.begin_stream();
        session.push_delta("partial");
        session.interrupt("missing done");
        let done = session.finish();

        assert_eq!(done.state, StreamState::Interrupted);
        assert_eq!(session.state(), StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.last_error(), Some("missing done"));
        assert!(session.history().is_empty());
    }

    #[test]
    fn late_payloads_after_interrupted_do_not_replace_partial_or_error() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("hello");
        session.begin_stream();
        session.push_delta("partial");
        session.interrupt("missing done");
        let chunks_after_interrupt = session.chunks().len();

        let late_delta = session.push_delta(" late");
        let late_final = session.push_final_payload_with_answer("{\"answer\":\"late\"}", "late");
        let late_error = session.fail("late error");

        assert_eq!(late_delta.state, StreamState::Interrupted);
        assert_eq!(late_final.state, StreamState::Interrupted);
        assert_eq!(late_error.state, StreamState::Interrupted);
        assert_eq!(late_delta.content, "");
        assert_eq!(late_final.content, "");
        assert_eq!(late_error.content, "");
        assert_eq!(session.state(), StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.last_error(), Some("missing done"));
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "hello");
        assert_eq!(session.chunks().len(), chunks_after_interrupt);
    }

    #[test]
    fn late_payloads_after_failed_do_not_replace_partial_error_or_retry_context() {
        let mut session = ChatSession::new(
            "s1",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );

        session.submit_prompt("unsafe request");
        session.begin_stream();
        session.push_delta("unsafe partial");
        session.fail("safe-device gate failed");
        let chunks_after_fail = session.chunks().len();

        let late_delta = session.push_delta(" late");
        let late_final = session.push_final_payload_with_answer("{\"answer\":\"late\"}", "late");
        let late_done = session.finish();
        let late_error = session.fail("late backend error");

        assert_eq!(late_delta.state, StreamState::Failed);
        assert_eq!(late_final.state, StreamState::Failed);
        assert_eq!(late_done.state, StreamState::Failed);
        assert_eq!(late_error.state, StreamState::Failed);
        assert_eq!(late_delta.content, "");
        assert_eq!(late_final.content, "");
        assert_eq!(late_done.content, "");
        assert_eq!(late_error.content, "");
        assert_eq!(session.state(), StreamState::Failed);
        assert_eq!(session.partial_answer(), "unsafe partial");
        assert_eq!(session.last_error(), Some("safe-device gate failed"));
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "unsafe request");
        assert_eq!(session.chunks().len(), chunks_after_fail);

        let restarted = session
            .try_submit_and_begin_stream("repair and retry")
            .expect("failed stream should allow retry after late terminal events");
        let request_contents = restarted
            .request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();

        assert_eq!(request_contents, vec!["unsafe request", "repair and retry"]);
        assert!(!request_contents.contains(&"unsafe partial"));
        assert!(!request_contents.contains(&"safe-device gate failed"));
        assert!(!request_contents.contains(&"late"));
        assert_eq!(restarted.request.max_tokens, Some(4096));
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.partial_answer(), "");
        assert_eq!(session.last_error(), None);
    }

    #[test]
    fn begin_stream_still_opens_next_turn_after_completed_stream() {
        let mut session = ChatSession::new("s1", ChatSessionConfig::default());

        session.submit_prompt("first");
        session.begin_stream();
        session.push_delta("one");
        session.finish();
        session.submit_prompt("second");
        let start = session.begin_stream();
        let delta = session.push_delta("two");

        assert_eq!(start.state, StreamState::Streaming);
        assert_eq!(delta.content, "two");
        assert_eq!(session.partial_answer(), "two");
        assert_eq!(session.state(), StreamState::Streaming);
    }
}
