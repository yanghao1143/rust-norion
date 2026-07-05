use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

impl ChatRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

impl fmt::Display for ChatRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(ChatRole::System, content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(ChatRole::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(ChatRole::Assistant, content)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelEndpoint {
    Quality12B,
    FastReviewer,
    SummaryTester,
    Worker(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelEndpointSelectionKind {
    Auto,
    BuiltIn,
    Custom,
}

impl ModelEndpointSelectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::BuiltIn => "built_in",
            Self::Custom => "custom",
        }
    }

    pub fn for_route(endpoint: Option<&ModelEndpoint>, endpoint_pinned: bool) -> Self {
        if !endpoint_pinned {
            return Self::Auto;
        }

        endpoint
            .map(ModelEndpoint::selection_kind)
            .unwrap_or(Self::Auto)
    }
}

impl ModelEndpoint {
    pub const BUILT_INS: [Self; 3] = [Self::Quality12B, Self::FastReviewer, Self::SummaryTester];

    pub fn label(&self) -> &str {
        match self {
            Self::Quality12B => "quality-12b",
            Self::FastReviewer => "fast-reviewer",
            Self::SummaryTester => "summary-tester",
            Self::Worker(worker) => worker.as_str(),
        }
    }

    pub fn from_label(value: &str) -> Option<Self> {
        let value = value.trim();
        match value.to_ascii_lowercase().as_str() {
            "quality-12b" | "quality" | "12b" => Some(Self::Quality12B),
            "fast-reviewer" | "reviewer-fast" => Some(Self::FastReviewer),
            "summary-tester" | "summarizer-tester" => Some(Self::SummaryTester),
            "" | "auto" | "default" | "none" => None,
            _ => Some(Self::Worker(value.to_owned())),
        }
    }

    pub fn is_auto_label(value: &str) -> bool {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "" | "auto" | "default" | "none"
        )
    }

    pub fn is_built_in(&self) -> bool {
        matches!(
            self,
            Self::Quality12B | Self::FastReviewer | Self::SummaryTester
        )
    }

    pub fn selection_kind(&self) -> ModelEndpointSelectionKind {
        if self.is_built_in() {
            ModelEndpointSelectionKind::BuiltIn
        } else {
            ModelEndpointSelectionKind::Custom
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRole {
    Assistant,
    Reviewer,
    Summarizer,
    Tester,
}

impl ModelRole {
    pub const ALL: [Self; 4] = [
        Self::Assistant,
        Self::Reviewer,
        Self::Summarizer,
        Self::Tester,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assistant => "assistant",
            Self::Reviewer => "reviewer",
            Self::Summarizer => "summarizer",
            Self::Tester => "tester",
        }
    }

    pub fn from_label(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "assistant" | "chat" => Some(Self::Assistant),
            "reviewer" | "review" => Some(Self::Reviewer),
            "summarizer" | "summary" | "summarize" => Some(Self::Summarizer),
            "tester" | "test" | "tests" => Some(Self::Tester),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingPreference {
    Balanced,
    PreferFast,
    PreferQuality,
}

impl RoutingPreference {
    pub const ALL: [Self; 3] = [Self::Balanced, Self::PreferFast, Self::PreferQuality];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::PreferFast => "prefer_fast",
            Self::PreferQuality => "prefer_quality",
        }
    }

    pub fn from_label(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "balanced" | "balance" | "default" => Some(Self::Balanced),
            "fast" | "prefer-fast" | "prefer_fast" => Some(Self::PreferFast),
            "quality" | "prefer-quality" | "prefer_quality" => Some(Self::PreferQuality),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRequest {
    pub session_id: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub messages: Vec<ChatMessage>,
    pub profile: String,
    pub output: String,
    pub max_tokens: Option<usize>,
    pub stream: bool,
    pub model_endpoint: Option<ModelEndpoint>,
    pub model_role: ModelRole,
    pub routing_preference: RoutingPreference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRequestWireSnapshot {
    pub model_role_label: String,
    pub routing_preference_label: String,
    pub prefer_fast: bool,
    pub prefer_quality: bool,
    pub sends_max_tokens: bool,
    pub max_tokens: Option<usize>,
    pub endpoint_pinned: bool,
    pub endpoint_kind: ModelEndpointSelectionKind,
    pub endpoint_kind_label: String,
    pub sends_model_endpoint: bool,
    pub model_endpoint_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRequestSubmissionSnapshot {
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingIntentWireSnapshot {
    pub model_role_label: String,
    pub routing_preference_label: String,
    pub prefer_fast: bool,
    pub prefer_quality: bool,
    pub endpoint_pinned: bool,
    pub endpoint_kind: ModelEndpointSelectionKind,
    pub endpoint_kind_label: String,
    pub sends_model_endpoint: bool,
    pub model_endpoint_label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRequestContextKind {
    SingleTurn,
    MultiTurn,
}

impl ChatRequestContextKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SingleTurn => "single_turn",
            Self::MultiTurn => "multi_turn",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingIntent {
    pub model_role: ModelRole,
    pub routing_preference: RoutingPreference,
    pub model_endpoint: Option<ModelEndpoint>,
    pub endpoint_pinned: bool,
}

impl RoutingIntent {
    pub fn auto_route(model_role: ModelRole, routing_preference: RoutingPreference) -> Self {
        Self {
            model_role,
            routing_preference,
            model_endpoint: None,
            endpoint_pinned: false,
        }
    }

    pub fn operator_pinned(
        model_role: ModelRole,
        routing_preference: RoutingPreference,
        model_endpoint: ModelEndpoint,
    ) -> Self {
        Self {
            model_role,
            routing_preference,
            model_endpoint: Some(model_endpoint),
            endpoint_pinned: true,
        }
    }

    pub fn endpoint_label(&self) -> &str {
        if !self.endpoint_pinned {
            return "auto";
        }
        self.model_endpoint
            .as_ref()
            .map(ModelEndpoint::label)
            .unwrap_or("auto")
    }

    pub fn model_role_label(&self) -> &'static str {
        self.model_role.as_str()
    }

    pub fn routing_preference_label(&self) -> &'static str {
        self.routing_preference.as_str()
    }

    pub fn endpoint_kind(&self) -> ModelEndpointSelectionKind {
        ModelEndpointSelectionKind::for_route(self.model_endpoint.as_ref(), self.endpoint_pinned)
    }

    pub fn endpoint_kind_label(&self) -> &'static str {
        self.endpoint_kind().as_str()
    }

    pub fn endpoint_auto(&self) -> bool {
        self.endpoint_kind() == ModelEndpointSelectionKind::Auto
    }

    pub fn endpoint_built_in(&self) -> bool {
        self.endpoint_kind() == ModelEndpointSelectionKind::BuiltIn
    }

    pub fn endpoint_custom(&self) -> bool {
        self.endpoint_kind() == ModelEndpointSelectionKind::Custom
    }

    pub fn wire_model_role_label(&self) -> &'static str {
        self.model_role_label()
    }

    pub fn wire_routing_preference_label(&self) -> &'static str {
        self.routing_preference_label()
    }

    pub fn wire_prefer_fast(&self) -> bool {
        self.routing_preference == RoutingPreference::PreferFast
    }

    pub fn wire_prefer_quality(&self) -> bool {
        self.routing_preference == RoutingPreference::PreferQuality
    }

    pub fn wire_endpoint_pinned(&self) -> bool {
        self.endpoint_pinned
    }

    pub fn wire_endpoint_kind_label(&self) -> &'static str {
        self.endpoint_kind_label()
    }

    pub fn wire_sends_model_endpoint(&self) -> bool {
        self.endpoint_pinned && self.model_endpoint.is_some()
    }

    pub fn wire_model_endpoint_label(&self) -> Option<&str> {
        self.model_endpoint
            .as_ref()
            .filter(|_| self.endpoint_pinned)
            .map(ModelEndpoint::label)
    }

    pub fn wire_snapshot(&self) -> RoutingIntentWireSnapshot {
        RoutingIntentWireSnapshot {
            model_role_label: self.wire_model_role_label().to_owned(),
            routing_preference_label: self.wire_routing_preference_label().to_owned(),
            prefer_fast: self.wire_prefer_fast(),
            prefer_quality: self.wire_prefer_quality(),
            endpoint_pinned: self.wire_endpoint_pinned(),
            endpoint_kind: self.endpoint_kind(),
            endpoint_kind_label: self.wire_endpoint_kind_label().to_owned(),
            sends_model_endpoint: self.wire_sends_model_endpoint(),
            model_endpoint_label: self.wire_model_endpoint_label().map(ToOwned::to_owned),
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "role={} preference={} endpoint={} pinned={}",
            self.model_role_label(),
            self.routing_preference_label(),
            self.endpoint_label(),
            self.endpoint_pinned
        )
    }
}

impl ChatRequest {
    pub fn new(session_id: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            session_id: session_id.into(),
            tenant_id: "local".to_owned(),
            workspace_id: "default".to_owned(),
            messages,
            profile: "coding".to_owned(),
            output: "raw".to_owned(),
            max_tokens: None,
            stream: true,
            model_endpoint: None,
            model_role: ModelRole::Assistant,
            routing_preference: RoutingPreference::Balanced,
        }
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

    pub fn with_max_tokens(mut self, max_tokens: Option<usize>) -> Self {
        self.max_tokens = max_tokens.map(|value| value.max(1));
        self
    }

    pub fn prefer_fast(mut self) -> Self {
        self.routing_preference = RoutingPreference::PreferFast;
        self
    }

    pub fn prefer_quality(mut self) -> Self {
        self.routing_preference = RoutingPreference::PreferQuality;
        self
    }

    pub fn with_routing_preference(mut self, preference: RoutingPreference) -> Self {
        self.routing_preference = preference;
        self
    }

    pub fn with_model_role(mut self, role: ModelRole) -> Self {
        self.model_role = role;
        self
    }

    pub fn with_model_endpoint(mut self, endpoint: Option<ModelEndpoint>) -> Self {
        self.model_endpoint = endpoint;
        self
    }

    pub fn with_routing_intent(mut self, intent: RoutingIntent) -> Self {
        self.model_role = intent.model_role;
        self.routing_preference = intent.routing_preference;
        self.model_endpoint = if intent.endpoint_pinned {
            intent.model_endpoint
        } else {
            None
        };
        self
    }

    pub fn routing_intent(&self) -> RoutingIntent {
        RoutingIntent {
            model_role: self.model_role,
            routing_preference: self.routing_preference,
            model_endpoint: self.model_endpoint.clone(),
            endpoint_pinned: self.model_endpoint.is_some(),
        }
    }

    pub fn endpoint_pinned(&self) -> bool {
        self.model_endpoint.is_some()
    }

    pub fn endpoint_label(&self) -> &str {
        self.model_endpoint
            .as_ref()
            .map(ModelEndpoint::label)
            .unwrap_or("auto")
    }

    pub fn endpoint_kind(&self) -> ModelEndpointSelectionKind {
        ModelEndpointSelectionKind::for_route(self.model_endpoint.as_ref(), self.endpoint_pinned())
    }

    pub fn endpoint_kind_label(&self) -> &'static str {
        self.endpoint_kind().as_str()
    }

    pub fn wire_endpoint_pinned(&self) -> bool {
        self.endpoint_pinned()
    }

    pub fn wire_endpoint_kind_label(&self) -> &'static str {
        self.endpoint_kind_label()
    }

    pub fn wire_sends_model_endpoint(&self) -> bool {
        self.model_endpoint.is_some()
    }

    pub fn wire_model_endpoint_label(&self) -> Option<&str> {
        self.model_endpoint.as_ref().map(ModelEndpoint::label)
    }

    pub fn wire_model_role_label(&self) -> &'static str {
        self.model_role_label()
    }

    pub fn wire_routing_preference_label(&self) -> &'static str {
        self.routing_preference_label()
    }

    pub fn wire_prefer_fast(&self) -> bool {
        self.routing_preference == RoutingPreference::PreferFast
    }

    pub fn wire_prefer_quality(&self) -> bool {
        self.routing_preference == RoutingPreference::PreferQuality
    }

    pub fn wire_sends_max_tokens(&self) -> bool {
        self.max_tokens.is_some()
    }

    pub fn wire_max_tokens(&self) -> Option<usize> {
        self.max_tokens
    }

    pub fn wire_snapshot(&self) -> ChatRequestWireSnapshot {
        let route_wire = self.routing_intent().wire_snapshot();
        ChatRequestWireSnapshot {
            model_role_label: route_wire.model_role_label,
            routing_preference_label: route_wire.routing_preference_label,
            prefer_fast: route_wire.prefer_fast,
            prefer_quality: route_wire.prefer_quality,
            sends_max_tokens: self.wire_sends_max_tokens(),
            max_tokens: self.wire_max_tokens(),
            endpoint_pinned: route_wire.endpoint_pinned,
            endpoint_kind: route_wire.endpoint_kind,
            endpoint_kind_label: route_wire.endpoint_kind_label,
            sends_model_endpoint: route_wire.sends_model_endpoint,
            model_endpoint_label: route_wire.model_endpoint_label,
        }
    }

    pub fn submission_snapshot(&self) -> ChatRequestSubmissionSnapshot {
        self.submission_snapshot_with_history_limit(None)
    }

    pub fn submission_snapshot_with_history_limit(
        &self,
        history_limit: Option<usize>,
    ) -> ChatRequestSubmissionSnapshot {
        let routing_intent = self.routing_intent();
        let wire = self.wire_snapshot();
        let context_messages = self.context_message_count();
        let endpoint_kind = self.endpoint_kind();
        let normalized_history_limit = history_limit.map(|limit| limit.max(1));
        let history_remaining =
            normalized_history_limit.map(|limit| limit.saturating_sub(context_messages));
        let history_messages_after_submit =
            normalized_history_limit.map(|limit| self.message_count().min(limit));
        let history_at_limit_after_submit = history_messages_after_submit
            .zip(normalized_history_limit)
            .map(|(messages_after_submit, limit)| messages_after_submit >= limit);
        let history_truncates_on_submit =
            normalized_history_limit.map(|limit| self.message_count() > limit);

        ChatRequestSubmissionSnapshot {
            model_role_label: self.model_role_label().to_owned(),
            routing_preference_label: self.routing_preference_label().to_owned(),
            endpoint_label: self.endpoint_label().to_owned(),
            endpoint_pinned: routing_intent.endpoint_pinned,
            endpoint_kind,
            endpoint_kind_label: self.endpoint_kind_label().to_owned(),
            endpoint_auto: endpoint_kind == ModelEndpointSelectionKind::Auto,
            endpoint_built_in: endpoint_kind == ModelEndpointSelectionKind::BuiltIn,
            endpoint_custom: endpoint_kind == ModelEndpointSelectionKind::Custom,
            wire_model_role_label: wire.model_role_label,
            wire_routing_preference_label: wire.routing_preference_label,
            wire_prefer_fast: wire.prefer_fast,
            wire_prefer_quality: wire.prefer_quality,
            wire_sends_max_tokens: wire.sends_max_tokens,
            wire_max_tokens: wire.max_tokens,
            wire_endpoint_pinned: wire.endpoint_pinned,
            wire_endpoint_kind_label: wire.endpoint_kind_label,
            wire_sends_model_endpoint: wire.sends_model_endpoint,
            wire_model_endpoint_label: wire.model_endpoint_label,
            routing_intent,
            messages: self.message_count(),
            context_messages,
            history_messages: context_messages,
            history_limit: normalized_history_limit,
            history_remaining,
            history_messages_after_submit,
            history_at_limit_after_submit,
            history_truncates_on_submit,
            context_kind: self.context_kind(),
            context_kind_label: self.context_kind_label().to_owned(),
            has_context: self.has_context(),
            is_single_turn: self.is_single_turn(),
            last_message_role_label: self.last_message_role_label().map(ToOwned::to_owned),
            last_message_chars: self.last_message_chars(),
            last_message_is_user: self.last_message_is_user(),
            last_user_chars: self.last_user_message_chars(),
            max_tokens: self.max_tokens,
            max_tokens_label: self
                .max_tokens
                .map(|value| value.to_string())
                .unwrap_or_else(|| "backend-default".to_owned()),
            stream: self.stream,
        }
    }

    pub fn model_role_label(&self) -> &'static str {
        self.model_role.as_str()
    }

    pub fn routing_preference_label(&self) -> &'static str {
        self.routing_preference.as_str()
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn context_message_count(&self) -> usize {
        self.messages.len().saturating_sub(1)
    }

    pub fn has_context(&self) -> bool {
        self.context_message_count() > 0
    }

    pub fn is_single_turn(&self) -> bool {
        self.context_message_count() == 0
    }

    pub fn context_kind(&self) -> ChatRequestContextKind {
        if self.has_context() {
            ChatRequestContextKind::MultiTurn
        } else {
            ChatRequestContextKind::SingleTurn
        }
    }

    pub fn context_kind_label(&self) -> &'static str {
        self.context_kind().as_str()
    }

    pub fn last_message(&self) -> Option<&ChatMessage> {
        self.messages.last()
    }

    pub fn last_message_role(&self) -> Option<ChatRole> {
        self.last_message().map(|message| message.role)
    }

    pub fn last_message_role_label(&self) -> Option<&'static str> {
        self.last_message_role().map(ChatRole::as_str)
    }

    pub fn last_message_chars(&self) -> usize {
        self.last_message()
            .map(|message| message.content.chars().count())
            .unwrap_or(0)
    }

    pub fn last_message_is_user(&self) -> bool {
        self.last_message_role() == Some(ChatRole::User)
    }

    pub fn last_user_message_chars(&self) -> usize {
        self.messages
            .iter()
            .rev()
            .find(|message| message.role == ChatRole::User)
            .map(|message| message.content.chars().count())
            .unwrap_or(0)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    Pending,
    Queued,
    Busy,
    Backpressure,
    Streaming,
    Completed,
    Interrupted,
    Failed,
}

impl StreamState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Queued => "queued",
            Self::Busy => "busy",
            Self::Backpressure => "backpressure",
            Self::Streaming => "streaming",
            Self::Completed => "completed",
            Self::Interrupted => "interrupted",
            Self::Failed => "failed",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Interrupted | Self::Failed)
    }

    pub fn is_pressure(self) -> bool {
        matches!(self, Self::Queued | Self::Busy | Self::Backpressure)
    }

    pub fn blocks_prompt_submit(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Busy | Self::Backpressure | Self::Streaming
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatChunkKind {
    Start,
    Delta,
    Status,
    Metadata,
    Final,
    Done,
    Error,
}

impl ChatChunkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Delta => "delta",
            Self::Status => "status",
            Self::Metadata => "metadata",
            Self::Final => "final",
            Self::Done => "done",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatChunk {
    pub sequence: u64,
    pub state: StreamState,
    pub kind: ChatChunkKind,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatChunkDisplaySnapshot {
    pub sequence: u64,
    pub kind: ChatChunkKind,
    pub kind_label: String,
    pub state: StreamState,
    pub state_label: String,
    pub output_label: String,
    pub content: String,
    pub content_chars: usize,
    pub appended: String,
    pub emits_output: bool,
    pub is_start: bool,
    pub is_delta: bool,
    pub is_status: bool,
    pub is_metadata: bool,
    pub is_final: bool,
    pub is_done: bool,
    pub is_error: bool,
    pub state_is_terminal: bool,
    pub state_is_pressure: bool,
    pub state_blocks_prompt_submit: bool,
}

impl ChatChunk {
    pub fn new(
        sequence: u64,
        state: StreamState,
        kind: ChatChunkKind,
        content: impl Into<String>,
    ) -> Self {
        Self {
            sequence,
            state,
            kind,
            content: content.into(),
        }
    }

    pub fn start(sequence: u64) -> Self {
        Self::new(sequence, StreamState::Streaming, ChatChunkKind::Start, "")
    }

    pub fn queued(sequence: u64, content: impl Into<String>) -> Self {
        Self::new(
            sequence,
            StreamState::Queued,
            ChatChunkKind::Status,
            content,
        )
    }

    pub fn busy(sequence: u64, content: impl Into<String>) -> Self {
        Self::new(sequence, StreamState::Busy, ChatChunkKind::Status, content)
    }

    pub fn backpressure(sequence: u64, content: impl Into<String>) -> Self {
        Self::new(
            sequence,
            StreamState::Backpressure,
            ChatChunkKind::Status,
            content,
        )
    }

    pub fn delta(sequence: u64, content: impl Into<String>) -> Self {
        Self::new(
            sequence,
            StreamState::Streaming,
            ChatChunkKind::Delta,
            content,
        )
    }

    pub fn status(sequence: u64, content: impl Into<String>) -> Self {
        Self::new(
            sequence,
            StreamState::Streaming,
            ChatChunkKind::Status,
            content,
        )
    }

    pub fn metadata(sequence: u64, content: impl Into<String>) -> Self {
        Self::new(
            sequence,
            StreamState::Streaming,
            ChatChunkKind::Metadata,
            content,
        )
    }

    pub fn final_payload(sequence: u64, content: impl Into<String>) -> Self {
        Self::new(
            sequence,
            StreamState::Streaming,
            ChatChunkKind::Final,
            content,
        )
    }

    pub fn done(sequence: u64) -> Self {
        Self::new(sequence, StreamState::Completed, ChatChunkKind::Done, "")
    }

    pub fn interrupted(sequence: u64, content: impl Into<String>) -> Self {
        Self::new(
            sequence,
            StreamState::Interrupted,
            ChatChunkKind::Error,
            content,
        )
    }

    pub fn failed(sequence: u64, content: impl Into<String>) -> Self {
        Self::new(sequence, StreamState::Failed, ChatChunkKind::Error, content)
    }

    pub fn output_label(&self) -> &'static str {
        match self.state {
            StreamState::Queued => "queued",
            StreamState::Busy => "busy",
            StreamState::Backpressure => "backpressure",
            StreamState::Interrupted => "interrupted",
            _ => self.kind.as_str(),
        }
    }

    pub fn display_append(&self) -> String {
        match self.kind {
            ChatChunkKind::Delta => self.content.clone(),
            ChatChunkKind::Done => String::new(),
            _ if self.content.is_empty() => String::new(),
            _ => format!("[{}] {}", self.output_label(), self.content),
        }
    }

    pub fn display_snapshot(&self) -> ChatChunkDisplaySnapshot {
        let appended = self.display_append();
        ChatChunkDisplaySnapshot {
            sequence: self.sequence,
            kind: self.kind,
            kind_label: self.kind.as_str().to_owned(),
            state: self.state,
            state_label: self.state.as_str().to_owned(),
            output_label: self.output_label().to_owned(),
            content: self.content.clone(),
            content_chars: self.content.chars().count(),
            emits_output: !appended.is_empty(),
            appended,
            is_start: self.kind == ChatChunkKind::Start,
            is_delta: self.kind == ChatChunkKind::Delta,
            is_status: self.kind == ChatChunkKind::Status,
            is_metadata: self.kind == ChatChunkKind::Metadata,
            is_final: self.kind == ChatChunkKind::Final,
            is_done: self.kind == ChatChunkKind::Done,
            is_error: self.kind == ChatChunkKind::Error,
            state_is_terminal: self.state.is_terminal(),
            state_is_pressure: self.state.is_pressure(),
            state_blocks_prompt_submit: self.state.blocks_prompt_submit(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_model_role_labels() {
        assert_eq!(
            ModelRole::ALL
                .iter()
                .map(|role| role.as_str())
                .collect::<Vec<_>>(),
            vec!["assistant", "reviewer", "summarizer", "tester"]
        );
        assert_eq!(ModelRole::from_label("chat"), Some(ModelRole::Assistant));
        assert_eq!(ModelRole::from_label("review"), Some(ModelRole::Reviewer));
        assert_eq!(
            ModelRole::from_label("summary"),
            Some(ModelRole::Summarizer)
        );
        assert_eq!(ModelRole::from_label("tests"), Some(ModelRole::Tester));
    }

    #[test]
    fn parses_routing_preference_labels() {
        assert_eq!(
            RoutingPreference::ALL
                .iter()
                .map(|preference| preference.as_str())
                .collect::<Vec<_>>(),
            vec!["balanced", "prefer_fast", "prefer_quality"]
        );
        assert_eq!(
            RoutingPreference::from_label("balanced"),
            Some(RoutingPreference::Balanced)
        );
        assert_eq!(
            RoutingPreference::from_label("fast"),
            Some(RoutingPreference::PreferFast)
        );
        assert_eq!(
            RoutingPreference::from_label("prefer_quality"),
            Some(RoutingPreference::PreferQuality)
        );
    }

    #[test]
    fn parses_model_endpoint_labels_and_auto_endpoint() {
        assert_eq!(
            ModelEndpoint::BUILT_INS
                .iter()
                .map(|endpoint| endpoint.label())
                .collect::<Vec<_>>(),
            vec!["quality-12b", "fast-reviewer", "summary-tester"]
        );
        assert_eq!(
            ModelEndpoint::from_label("quality"),
            Some(ModelEndpoint::Quality12B)
        );
        assert_eq!(
            ModelEndpoint::from_label("fast-reviewer"),
            Some(ModelEndpoint::FastReviewer)
        );
        assert_eq!(ModelEndpoint::from_label("auto"), None);
        assert!(ModelEndpoint::is_auto_label("default"));
        assert!(ModelEndpoint::is_auto_label("none"));
        assert!(!ModelEndpoint::is_auto_label("fast-reviewer"));
        assert!(ModelEndpoint::FastReviewer.is_built_in());
        assert_eq!(
            ModelEndpoint::FastReviewer.selection_kind(),
            ModelEndpointSelectionKind::BuiltIn
        );
        assert_eq!(
            ModelEndpoint::from_label("local-worker-a"),
            Some(ModelEndpoint::Worker("local-worker-a".to_owned()))
        );
        assert_eq!(
            ModelEndpoint::Worker("local-worker-a".to_owned()).selection_kind(),
            ModelEndpointSelectionKind::Custom
        );
        assert_eq!(
            ModelEndpointSelectionKind::for_route(Some(&ModelEndpoint::FastReviewer), false),
            ModelEndpointSelectionKind::Auto
        );
        assert_eq!(
            ModelEndpointSelectionKind::for_route(Some(&ModelEndpoint::FastReviewer), true),
            ModelEndpointSelectionKind::BuiltIn
        );
        assert_eq!(
            ModelEndpointSelectionKind::for_route(
                Some(&ModelEndpoint::Worker("local-worker-a".to_owned())),
                true
            ),
            ModelEndpointSelectionKind::Custom
        );
    }

    #[test]
    fn stream_state_classifies_wait_pressure_and_submit_boundaries() {
        assert!(!StreamState::Pending.is_terminal());
        assert!(!StreamState::Pending.is_pressure());
        assert!(!StreamState::Pending.blocks_prompt_submit());

        for state in [
            StreamState::Queued,
            StreamState::Busy,
            StreamState::Backpressure,
        ] {
            assert!(!state.is_terminal());
            assert!(state.is_pressure());
            assert!(state.blocks_prompt_submit());
        }

        assert!(!StreamState::Streaming.is_terminal());
        assert!(!StreamState::Streaming.is_pressure());
        assert!(StreamState::Streaming.blocks_prompt_submit());

        for state in [
            StreamState::Completed,
            StreamState::Interrupted,
            StreamState::Failed,
        ] {
            assert!(state.is_terminal());
            assert!(!state.is_pressure());
            assert!(!state.blocks_prompt_submit());
        }
    }

    #[test]
    fn chat_chunk_display_snapshot_carries_labels_text_and_pressure_state() {
        let delta = ChatChunk::delta(7, "hello");
        let queued = ChatChunk::queued(8, "waiting for reviewer");
        let interrupted = ChatChunk::interrupted(9, "missing done");
        let done = ChatChunk::done(10);

        let delta = delta.display_snapshot();
        let queued = queued.display_snapshot();
        let interrupted = interrupted.display_snapshot();
        let done = done.display_snapshot();

        assert_eq!(delta.sequence, 7);
        assert_eq!(delta.kind, ChatChunkKind::Delta);
        assert_eq!(delta.kind_label, "delta");
        assert_eq!(delta.state, StreamState::Streaming);
        assert_eq!(delta.state_label, "streaming");
        assert_eq!(delta.output_label, "delta");
        assert_eq!(delta.content, "hello");
        assert_eq!(delta.content_chars, 5);
        assert_eq!(delta.appended, "hello");
        assert!(delta.emits_output);
        assert!(delta.is_delta);
        assert!(!delta.state_is_terminal);
        assert!(!delta.state_is_pressure);
        assert!(delta.state_blocks_prompt_submit);

        assert_eq!(queued.output_label, "queued");
        assert_eq!(queued.appended, "[queued] waiting for reviewer");
        assert!(queued.is_status);
        assert!(!queued.state_is_terminal);
        assert!(queued.state_is_pressure);
        assert!(queued.state_blocks_prompt_submit);

        assert_eq!(interrupted.kind_label, "error");
        assert_eq!(interrupted.output_label, "interrupted");
        assert_eq!(interrupted.appended, "[interrupted] missing done");
        assert!(interrupted.is_error);
        assert!(interrupted.state_is_terminal);
        assert!(!interrupted.state_is_pressure);
        assert!(!interrupted.state_blocks_prompt_submit);

        assert_eq!(done.output_label, "done");
        assert_eq!(done.appended, "");
        assert!(!done.emits_output);
        assert!(done.is_done);
        assert!(done.state_is_terminal);
    }

    #[test]
    fn chat_request_exposes_route_intent_without_pinning_by_default() {
        let request = ChatRequest::new("s1", vec![ChatMessage::user("hello")])
            .with_routing_preference(RoutingPreference::PreferQuality);

        let intent = request.routing_intent();

        assert_eq!(intent.model_role, ModelRole::Assistant);
        assert_eq!(intent.routing_preference, RoutingPreference::PreferQuality);
        assert_eq!(intent.endpoint_label(), "auto");
        assert!(!intent.endpoint_pinned);
        assert!(!request.endpoint_pinned());
        assert_eq!(
            intent.summary(),
            "role=assistant preference=prefer_quality endpoint=auto pinned=false"
        );
    }

    #[test]
    fn chat_request_route_intent_marks_explicit_endpoint_as_pinned() {
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));

        let intent = request.routing_intent();

        assert_eq!(intent.endpoint_label(), "fast-reviewer");
        assert!(intent.endpoint_pinned);
        assert!(request.endpoint_pinned());
        assert_eq!(
            intent.summary(),
            "role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true"
        );
    }

    #[test]
    fn chat_request_exposes_message_boundary_helpers() {
        let request = ChatRequest::new(
            "s1",
            vec![
                ChatMessage::system("stay concise"),
                ChatMessage::user("first"),
                ChatMessage::assistant("answer"),
                ChatMessage::user("review this patch"),
            ],
        );

        assert_eq!(request.message_count(), 4);
        assert_eq!(request.context_message_count(), 3);
        assert!(request.has_context());
        assert!(!request.is_single_turn());
        assert_eq!(request.context_kind(), ChatRequestContextKind::MultiTurn);
        assert_eq!(request.context_kind_label(), "multi_turn");
        assert_eq!(request.last_message_role(), Some(ChatRole::User));
        assert_eq!(request.last_message_role_label(), Some("user"));
        assert_eq!(request.last_message_chars(), 17);
        assert!(request.last_message_is_user());
        assert_eq!(request.last_user_message_chars(), 17);

        let assistant_last = ChatRequest::new(
            "s1",
            vec![
                ChatMessage::user("first"),
                ChatMessage::assistant("final answer"),
            ],
        );
        assert_eq!(
            assistant_last.last_message_role(),
            Some(ChatRole::Assistant)
        );
        assert_eq!(assistant_last.last_message_role_label(), Some("assistant"));
        assert_eq!(assistant_last.last_message_chars(), 12);
        assert!(!assistant_last.last_message_is_user());
        assert_eq!(assistant_last.last_user_message_chars(), 5);

        let empty = ChatRequest::new("s1", Vec::new());
        assert_eq!(empty.message_count(), 0);
        assert_eq!(empty.context_message_count(), 0);
        assert!(!empty.has_context());
        assert!(empty.is_single_turn());
        assert_eq!(empty.context_kind(), ChatRequestContextKind::SingleTurn);
        assert_eq!(empty.context_kind_label(), "single_turn");
        assert_eq!(empty.last_message_role(), None);
        assert_eq!(empty.last_message_role_label(), None);
        assert_eq!(empty.last_message_chars(), 0);
        assert!(!empty.last_message_is_user());
        assert_eq!(empty.last_user_message_chars(), 0);
    }

    #[test]
    fn custom_endpoint_label_marks_operator_worker_pin() {
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_endpoint(ModelEndpoint::from_label("mlx-reviewer-8b"));

        let intent = request.routing_intent();

        assert_eq!(intent.endpoint_label(), "mlx-reviewer-8b");
        assert!(intent.endpoint_pinned);
        assert_eq!(
            intent.summary(),
            "role=assistant preference=balanced endpoint=mlx-reviewer-8b pinned=true"
        );
    }

    #[test]
    fn routing_intent_summary_treats_unpinned_endpoint_as_auto() {
        let intent = RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast);

        assert_eq!(intent.endpoint_label(), "auto");
        assert_eq!(intent.model_role_label(), "reviewer");
        assert_eq!(intent.routing_preference_label(), "prefer_fast");
        assert_eq!(intent.endpoint_kind(), ModelEndpointSelectionKind::Auto);
        assert_eq!(intent.endpoint_kind_label(), "auto");
        assert!(intent.endpoint_auto());
        assert!(!intent.endpoint_built_in());
        assert!(!intent.endpoint_custom());
        assert_eq!(intent.wire_model_role_label(), "reviewer");
        assert_eq!(intent.wire_routing_preference_label(), "prefer_fast");
        assert!(intent.wire_prefer_fast());
        assert!(!intent.wire_prefer_quality());
        assert!(!intent.wire_endpoint_pinned());
        assert_eq!(intent.wire_endpoint_kind_label(), "auto");
        assert!(!intent.wire_sends_model_endpoint());
        assert_eq!(intent.wire_model_endpoint_label(), None);
        assert_eq!(
            intent.summary(),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );

        let pinned = RoutingIntent::operator_pinned(
            ModelRole::Reviewer,
            RoutingPreference::PreferFast,
            ModelEndpoint::FastReviewer,
        );
        assert_eq!(pinned.endpoint_label(), "fast-reviewer");
        assert!(pinned.endpoint_pinned);
        assert_eq!(pinned.endpoint_kind(), ModelEndpointSelectionKind::BuiltIn);
        assert_eq!(pinned.endpoint_kind_label(), "built_in");
        assert!(!pinned.endpoint_auto());
        assert!(pinned.endpoint_built_in());
        assert!(!pinned.endpoint_custom());
        assert!(pinned.wire_endpoint_pinned());
        assert_eq!(pinned.wire_endpoint_kind_label(), "built_in");
        assert!(pinned.wire_sends_model_endpoint());
        assert_eq!(pinned.wire_model_endpoint_label(), Some("fast-reviewer"));

        let custom = RoutingIntent::operator_pinned(
            ModelRole::Reviewer,
            RoutingPreference::PreferFast,
            ModelEndpoint::Worker("mlx-reviewer-8b".to_owned()),
        );
        assert_eq!(custom.endpoint_label(), "mlx-reviewer-8b");
        assert_eq!(custom.endpoint_kind(), ModelEndpointSelectionKind::Custom);
        assert_eq!(custom.endpoint_kind_label(), "custom");
        assert!(!custom.endpoint_auto());
        assert!(!custom.endpoint_built_in());
        assert!(custom.endpoint_custom());
        assert!(custom.wire_endpoint_pinned());
        assert_eq!(custom.wire_endpoint_kind_label(), "custom");
        assert!(custom.wire_sends_model_endpoint());
        assert_eq!(custom.wire_model_endpoint_label(), Some("mlx-reviewer-8b"));
    }

    #[test]
    fn routing_intent_wire_snapshot_is_route_only_contract() {
        let auto = RoutingIntent {
            model_role: ModelRole::Reviewer,
            routing_preference: RoutingPreference::PreferFast,
            model_endpoint: Some(ModelEndpoint::FastReviewer),
            endpoint_pinned: false,
        };

        let wire = auto.wire_snapshot();

        assert_eq!(wire.model_role_label, "reviewer");
        assert_eq!(wire.routing_preference_label, "prefer_fast");
        assert!(wire.prefer_fast);
        assert!(!wire.prefer_quality);
        assert!(!wire.endpoint_pinned);
        assert_eq!(wire.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(wire.endpoint_kind_label, "auto");
        assert!(!wire.sends_model_endpoint);
        assert_eq!(wire.model_endpoint_label, None);

        let pinned = RoutingIntent::operator_pinned(
            ModelRole::Tester,
            RoutingPreference::PreferQuality,
            ModelEndpoint::Worker("mlx-test-4b".to_owned()),
        );
        let wire = pinned.wire_snapshot();

        assert_eq!(wire.model_role_label, "tester");
        assert_eq!(wire.routing_preference_label, "prefer_quality");
        assert!(!wire.prefer_fast);
        assert!(wire.prefer_quality);
        assert!(wire.endpoint_pinned);
        assert_eq!(wire.endpoint_kind, ModelEndpointSelectionKind::Custom);
        assert_eq!(wire.endpoint_kind_label, "custom");
        assert!(wire.sends_model_endpoint);
        assert_eq!(wire.model_endpoint_label.as_deref(), Some("mlx-test-4b"));
    }

    #[test]
    fn chat_request_applies_routing_intent_without_implicit_worker_pin() {
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_routing_intent(RoutingIntent {
                model_role: ModelRole::Reviewer,
                routing_preference: RoutingPreference::PreferFast,
                model_endpoint: Some(ModelEndpoint::FastReviewer),
                endpoint_pinned: false,
            });

        assert_eq!(request.model_role, ModelRole::Reviewer);
        assert_eq!(request.routing_preference, RoutingPreference::PreferFast);
        assert_eq!(request.model_endpoint, None);
        assert_eq!(request.model_role_label(), "reviewer");
        assert_eq!(request.routing_preference_label(), "prefer_fast");
        assert_eq!(request.endpoint_label(), "auto");
        assert_eq!(request.endpoint_kind(), ModelEndpointSelectionKind::Auto);
        assert_eq!(request.endpoint_kind_label(), "auto");
        assert!(!request.endpoint_pinned());
        assert!(!request.wire_endpoint_pinned());
        assert_eq!(request.wire_endpoint_kind_label(), "auto");
        assert!(!request.wire_sends_model_endpoint());
        assert_eq!(request.wire_model_endpoint_label(), None);
        assert_eq!(request.wire_model_role_label(), "reviewer");
        assert_eq!(request.wire_routing_preference_label(), "prefer_fast");
        assert!(request.wire_prefer_fast());
        assert!(!request.wire_prefer_quality());
        assert!(!request.wire_sends_max_tokens());
        assert_eq!(request.wire_max_tokens(), None);
        assert_eq!(
            request.routing_intent().summary(),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
    }

    #[test]
    fn chat_request_applies_explicit_pinned_routing_intent() {
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_routing_intent(RoutingIntent {
                model_role: ModelRole::Reviewer,
                routing_preference: RoutingPreference::PreferFast,
                model_endpoint: Some(ModelEndpoint::FastReviewer),
                endpoint_pinned: true,
            });

        assert_eq!(
            request.model_endpoint.as_ref().map(ModelEndpoint::label),
            Some("fast-reviewer")
        );
        assert_eq!(request.model_role_label(), "reviewer");
        assert_eq!(request.routing_preference_label(), "prefer_fast");
        assert_eq!(request.endpoint_label(), "fast-reviewer");
        assert_eq!(request.endpoint_kind(), ModelEndpointSelectionKind::BuiltIn);
        assert_eq!(request.endpoint_kind_label(), "built_in");
        assert!(request.endpoint_pinned());
        assert!(request.wire_endpoint_pinned());
        assert_eq!(request.wire_endpoint_kind_label(), "built_in");
        assert!(request.wire_sends_model_endpoint());
        assert_eq!(request.wire_model_endpoint_label(), Some("fast-reviewer"));
        assert_eq!(request.wire_model_role_label(), "reviewer");
        assert_eq!(request.wire_routing_preference_label(), "prefer_fast");
        assert!(request.wire_prefer_fast());
        assert!(!request.wire_prefer_quality());
        assert!(!request.wire_sends_max_tokens());
        assert_eq!(request.wire_max_tokens(), None);
        assert_eq!(
            request.routing_intent().summary(),
            "role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true"
        );
    }

    #[test]
    fn chat_request_wire_snapshot_matches_serializer_contract() {
        let auto = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_max_tokens(Some(8192))
            .with_model_role(ModelRole::Reviewer)
            .prefer_fast();

        let wire = auto.wire_snapshot();

        assert_eq!(wire.model_role_label, "reviewer");
        assert_eq!(wire.routing_preference_label, "prefer_fast");
        assert!(wire.prefer_fast);
        assert!(!wire.prefer_quality);
        assert!(wire.sends_max_tokens);
        assert_eq!(wire.max_tokens, Some(8192));
        assert!(!wire.endpoint_pinned);
        assert_eq!(wire.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(wire.endpoint_kind_label, "auto");
        assert!(!wire.sends_model_endpoint);
        assert_eq!(wire.model_endpoint_label, None);

        let pinned = ChatRequest::new("s1", vec![ChatMessage::user("test")])
            .with_model_role(ModelRole::Tester)
            .prefer_quality()
            .with_model_endpoint(Some(ModelEndpoint::Worker("mlx-test-4b".to_owned())));
        let wire = pinned.wire_snapshot();

        assert_eq!(wire.model_role_label, "tester");
        assert_eq!(wire.routing_preference_label, "prefer_quality");
        assert!(!wire.prefer_fast);
        assert!(wire.prefer_quality);
        assert!(!wire.sends_max_tokens);
        assert_eq!(wire.max_tokens, None);
        assert!(wire.endpoint_pinned);
        assert_eq!(wire.endpoint_kind, ModelEndpointSelectionKind::Custom);
        assert_eq!(wire.endpoint_kind_label, "custom");
        assert!(wire.sends_model_endpoint);
        assert_eq!(wire.model_endpoint_label.as_deref(), Some("mlx-test-4b"));
    }

    #[test]
    fn chat_request_submission_snapshot_carries_history_context_and_wire_contract() {
        let request = ChatRequest::new(
            "s1",
            vec![
                ChatMessage::user("one"),
                ChatMessage::assistant("two"),
                ChatMessage::user("three"),
            ],
        )
        .with_max_tokens(Some(8192))
        .with_model_role(ModelRole::Reviewer)
        .prefer_fast();

        let snapshot = request.submission_snapshot_with_history_limit(Some(2));

        assert_eq!(snapshot.routing_intent.model_role, ModelRole::Reviewer);
        assert_eq!(
            snapshot.routing_intent.routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(snapshot.routing_intent.endpoint_label(), "auto");
        assert!(!snapshot.routing_intent.endpoint_pinned);
        assert_eq!(snapshot.model_role_label, "reviewer");
        assert_eq!(snapshot.routing_preference_label, "prefer_fast");
        assert_eq!(snapshot.endpoint_label, "auto");
        assert!(!snapshot.endpoint_pinned);
        assert_eq!(snapshot.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(snapshot.endpoint_kind_label, "auto");
        assert!(snapshot.endpoint_auto);
        assert!(!snapshot.endpoint_built_in);
        assert!(!snapshot.endpoint_custom);
        assert_eq!(snapshot.wire_model_role_label, "reviewer");
        assert_eq!(snapshot.wire_routing_preference_label, "prefer_fast");
        assert!(snapshot.wire_prefer_fast);
        assert!(!snapshot.wire_prefer_quality);
        assert!(snapshot.wire_sends_max_tokens);
        assert_eq!(snapshot.wire_max_tokens, Some(8192));
        assert!(!snapshot.wire_endpoint_pinned);
        assert_eq!(snapshot.wire_endpoint_kind_label, "auto");
        assert!(!snapshot.wire_sends_model_endpoint);
        assert_eq!(snapshot.wire_model_endpoint_label, None);
        assert_eq!(snapshot.messages, 3);
        assert_eq!(snapshot.context_messages, 2);
        assert_eq!(snapshot.history_messages, 2);
        assert_eq!(snapshot.history_limit, Some(2));
        assert_eq!(snapshot.history_remaining, Some(0));
        assert_eq!(snapshot.history_messages_after_submit, Some(2));
        assert_eq!(snapshot.history_at_limit_after_submit, Some(true));
        assert_eq!(snapshot.history_truncates_on_submit, Some(true));
        assert_eq!(snapshot.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(snapshot.context_kind_label, "multi_turn");
        assert!(snapshot.has_context);
        assert!(!snapshot.is_single_turn);
        assert_eq!(snapshot.last_message_role_label.as_deref(), Some("user"));
        assert_eq!(snapshot.last_message_chars, 5);
        assert!(snapshot.last_message_is_user);
        assert_eq!(snapshot.last_user_chars, 5);
        assert_eq!(snapshot.max_tokens, Some(8192));
        assert_eq!(snapshot.max_tokens_label, "8192");
        assert!(snapshot.stream);
    }

    #[test]
    fn chat_request_submission_snapshot_keeps_backend_default_tokens_distinct_from_context() {
        let request = ChatRequest::new(
            "s1",
            vec![
                ChatMessage::user("first question"),
                ChatMessage::assistant("first answer"),
                ChatMessage::user("follow up"),
            ],
        )
        .with_model_role(ModelRole::Assistant)
        .prefer_quality();

        let snapshot = request.submission_snapshot_with_history_limit(Some(8));

        assert_eq!(snapshot.messages, 3);
        assert_eq!(snapshot.context_messages, 2);
        assert_eq!(snapshot.messages, snapshot.context_messages + 1);
        assert_eq!(snapshot.history_messages, 2);
        assert_eq!(snapshot.history_limit, Some(8));
        assert_eq!(snapshot.history_remaining, Some(6));
        assert_eq!(snapshot.history_messages_after_submit, Some(3));
        assert_eq!(snapshot.history_at_limit_after_submit, Some(false));
        assert_eq!(snapshot.history_truncates_on_submit, Some(false));
        assert_eq!(snapshot.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(snapshot.context_kind_label, "multi_turn");
        assert!(snapshot.has_context);
        assert!(!snapshot.is_single_turn);
        assert_eq!(snapshot.max_tokens, None);
        assert_eq!(snapshot.max_tokens_label, "backend-default");
        assert!(!snapshot.wire_sends_max_tokens);
        assert_eq!(snapshot.wire_max_tokens, None);
        assert_eq!(snapshot.model_role_label, "assistant");
        assert_eq!(snapshot.routing_preference_label, "prefer_quality");
        assert_eq!(snapshot.endpoint_label, "auto");
        assert!(!snapshot.endpoint_pinned);
        assert!(!snapshot.wire_sends_model_endpoint);
        assert_eq!(snapshot.wire_model_endpoint_label, None);
        assert_eq!(snapshot.last_message_role_label.as_deref(), Some("user"));
        assert_eq!(snapshot.last_user_chars, "follow up".len());
    }

    #[test]
    fn chat_request_submission_snapshot_clamps_zero_history_limit_without_token_side_effect() {
        let request = ChatRequest::new(
            "s1",
            vec![
                ChatMessage::user("first"),
                ChatMessage::assistant("second"),
                ChatMessage::user("third"),
            ],
        )
        .with_model_role(ModelRole::Reviewer)
        .prefer_fast();

        let snapshot = request.submission_snapshot_with_history_limit(Some(0));

        assert_eq!(snapshot.messages, 3);
        assert_eq!(snapshot.context_messages, 2);
        assert_eq!(snapshot.history_messages, 2);
        assert_eq!(snapshot.history_limit, Some(1));
        assert_eq!(snapshot.history_remaining, Some(0));
        assert_eq!(snapshot.history_messages_after_submit, Some(1));
        assert_eq!(snapshot.history_at_limit_after_submit, Some(true));
        assert_eq!(snapshot.history_truncates_on_submit, Some(true));
        assert_eq!(snapshot.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(snapshot.context_kind_label, "multi_turn");
        assert!(snapshot.has_context);
        assert!(!snapshot.is_single_turn);
        assert_eq!(snapshot.max_tokens, None);
        assert_eq!(snapshot.max_tokens_label, "backend-default");
        assert!(!snapshot.wire_sends_max_tokens);
        assert_eq!(snapshot.wire_max_tokens, None);
        assert_eq!(snapshot.model_role_label, "reviewer");
        assert_eq!(snapshot.routing_preference_label, "prefer_fast");
        assert_eq!(snapshot.endpoint_label, "auto");
        assert!(!snapshot.endpoint_pinned);
    }
}
