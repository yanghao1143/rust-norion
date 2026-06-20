use norion_service::{
    ChatChunk, ChatChunkDisplaySnapshot, ChatChunkKind, ChatRequest, ChatRequestContextKind,
    ChatSession, FrontendGateSnapshot, GateAdvice, GateAdviceAction, GateDecision, GateSendControl,
    ModelEndpoint, ModelEndpointSelectionKind, ModelPoolGateSnapshot, ModelRole,
    ModelRouteWorkerSnapshot, ModelWorkerSnapshot, RoutingIntent, RoutingPreference,
    StartedChatTurn, StreamState,
};

use crate::status::{
    CliStatusSnapshot, cli_model_pool_status_line, cli_model_pool_workers_line, cli_status_line,
    cli_workers_unavailable_line,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliInputConfig {
    pub shift_enter_inserts_newline: bool,
    pub model_role: ModelRole,
    pub routing_preference: RoutingPreference,
    pub model_endpoint: Option<ModelEndpoint>,
}

impl Default for CliInputConfig {
    fn default() -> Self {
        Self {
            shift_enter_inserts_newline: true,
            model_role: ModelRole::Assistant,
            routing_preference: RoutingPreference::Balanced,
            model_endpoint: None,
        }
    }
}

impl CliInputConfig {
    pub fn with_model_role(mut self, role: ModelRole) -> Self {
        self.model_role = role;
        self
    }

    pub fn with_routing_preference(mut self, preference: RoutingPreference) -> Self {
        self.routing_preference = preference;
        self
    }

    pub fn prefer_fast(self) -> Self {
        self.with_routing_preference(RoutingPreference::PreferFast)
    }

    pub fn prefer_quality(self) -> Self {
        self.with_routing_preference(RoutingPreference::PreferQuality)
    }

    pub fn with_model_endpoint(mut self, endpoint: Option<ModelEndpoint>) -> Self {
        self.model_endpoint = endpoint;
        self
    }

    pub fn with_model_route_labels(
        self,
        role_label: &str,
        preference_label: Option<&str>,
        endpoint_label: Option<&str>,
    ) -> Result<Self, String> {
        let Some(role) = ModelRole::from_label(role_label) else {
            return Err(format!("unknown model role: {}", role_label.trim()));
        };
        let preference = preference_label
            .map(|label| {
                RoutingPreference::from_label(label)
                    .ok_or_else(|| format!("unknown routing preference: {}", label.trim()))
            })
            .transpose()?;

        let mut config = self.with_model_role(role);
        if let Some(preference) = preference {
            config = config.with_routing_preference(preference);
        }
        if let Some(endpoint_label) = endpoint_label {
            config = config.with_model_endpoint(ModelEndpoint::from_label(endpoint_label));
        }
        Ok(config)
    }

    pub fn routing_intent(&self) -> RoutingIntent {
        RoutingIntent {
            model_role: self.model_role,
            routing_preference: self.routing_preference,
            model_endpoint: self.model_endpoint.clone(),
            endpoint_pinned: self.model_endpoint.is_some(),
        }
    }

    pub fn routing_summary(&self) -> String {
        self.routing_intent().summary()
    }

    pub fn apply_routing_intent(&mut self, intent: RoutingIntent) {
        self.model_role = intent.model_role;
        self.routing_preference = intent.routing_preference;
        self.model_endpoint = if intent.endpoint_pinned {
            intent.model_endpoint
        } else {
            None
        };
    }

    pub fn with_routing_intent(mut self, intent: RoutingIntent) -> Self {
        self.apply_routing_intent(intent);
        self
    }

    pub fn apply_to_request(&self, request: ChatRequest) -> ChatRequest {
        request.with_routing_intent(self.routing_intent())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyInput {
    Char(char),
    Enter,
    ShiftEnter,
    Backspace,
    CtrlC,
    CtrlX,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    BufferChanged,
    Send(ChatRequest),
    StartStream(StartedChatTurn),
    Blocked(ChatChunk),
    StreamCancelled(ChatChunk),
    RoutingChanged(String),
    Status(String),
    SessionConfigChanged {
        update: SessionConfigUpdate,
        summary: String,
    },
    InputError(String),
    InsertNewline,
    CancelStream,
    Quit,
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputActionKind {
    BufferChanged,
    Send,
    StartStream,
    Blocked,
    StreamCancelled,
    RoutingChanged,
    Status,
    SessionConfigChanged,
    InputError,
    InsertNewline,
    CancelStream,
    Quit,
    Noop,
}

impl InputActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BufferChanged => "buffer_changed",
            Self::Send => "send",
            Self::StartStream => "start_stream",
            Self::Blocked => "blocked",
            Self::StreamCancelled => "stream_cancelled",
            Self::RoutingChanged => "routing_changed",
            Self::Status => "status",
            Self::SessionConfigChanged => "session_config_changed",
            Self::InputError => "input_error",
            Self::InsertNewline => "insert_newline",
            Self::CancelStream => "cancel_stream",
            Self::Quit => "quit",
            Self::Noop => "noop",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputRequestSnapshot {
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
pub struct InputActionSnapshot {
    pub kind: InputActionKind,
    pub kind_label: String,
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
    pub wire_endpoint_pinned: bool,
    pub wire_endpoint_kind_label: String,
    pub wire_sends_model_endpoint: bool,
    pub wire_model_endpoint_label: Option<String>,
    pub request: Option<InputRequestSnapshot>,
    pub session_config_update: Option<SessionConfigUpdate>,
    pub session_config_update_detail: Option<SessionConfigUpdateSnapshot>,
    pub stream_state: Option<StreamState>,
    pub stream_state_label: Option<String>,
    pub stream_state_is_terminal: Option<bool>,
    pub stream_state_is_pressure: Option<bool>,
    pub stream_state_blocks_prompt_submit: Option<bool>,
    pub stream_chunk: Option<ChatChunkDisplaySnapshot>,
    pub reason: Option<String>,
    pub local_status: Option<String>,
    pub start_sequence: Option<u64>,
    pub start_state: Option<StreamState>,
    pub start_state_label: Option<String>,
    pub start_state_is_terminal: Option<bool>,
    pub start_state_is_pressure: Option<bool>,
    pub start_state_blocks_prompt_submit: Option<bool>,
    pub start_chunk: Option<ChatChunkDisplaySnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputBufferKind {
    Empty,
    Prompt,
    StatusCommand,
    WorkerStatusCommand,
    RoutingCommand,
    SessionConfigCommand,
    InvalidCommand,
}

impl InputBufferKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Prompt => "prompt",
            Self::StatusCommand => "status_command",
            Self::WorkerStatusCommand => "worker_status_command",
            Self::RoutingCommand => "routing_command",
            Self::SessionConfigCommand => "session_config_command",
            Self::InvalidCommand => "invalid_command",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSubmitMode {
    Preview,
    Record,
    StartStream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputReadinessSnapshot {
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
    pub wire_endpoint_pinned: bool,
    pub wire_endpoint_kind_label: String,
    pub wire_sends_model_endpoint: bool,
    pub wire_model_endpoint_label: Option<String>,
    pub submit_mode: InputSubmitMode,
    pub submit_mode_label: String,
    pub buffer_chars: usize,
    pub trimmed_chars: usize,
    pub line_count: usize,
    pub buffer_kind: InputBufferKind,
    pub buffer_kind_label: String,
    pub command_preview: Option<InputCommandPreview>,
    pub request_preview: Option<InputRequestSnapshot>,
    pub prompt_submit_control: Option<GateSendControl>,
    pub enter_action: InputActionKind,
    pub enter_action_label: String,
    pub enter_enabled: bool,
    pub enter_submits_prompt: bool,
    pub enter_runs_local_command: bool,
    pub enter_is_blocked: bool,
    pub primary_action_label: String,
    pub primary_action_enabled: bool,
    pub primary_action_disabled_reason: Option<String>,
    pub preserves_buffer_on_enter: bool,
    pub clears_buffer_on_enter: bool,
    pub send_allowed: bool,
    pub records_user_on_enter: bool,
    pub starts_stream_on_enter: bool,
    pub advice_action: Option<GateAdviceAction>,
    pub advice_action_label: Option<String>,
    pub block_state: Option<StreamState>,
    pub block_state_label: Option<String>,
    pub block_state_is_terminal: bool,
    pub block_state_is_pressure: bool,
    pub block_state_blocks_prompt_submit: bool,
    pub block_chunk: Option<ChatChunkDisplaySnapshot>,
    pub block_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputCommandPreview {
    pub buffer_kind: InputBufferKind,
    pub buffer_kind_label: String,
    pub enter_action: InputActionKind,
    pub enter_action_label: String,
    pub routing_intent: Option<RoutingIntent>,
    pub routing_summary: Option<String>,
    pub model_role_label: Option<String>,
    pub routing_preference_label: Option<String>,
    pub endpoint_label: Option<String>,
    pub endpoint_pinned: Option<bool>,
    pub endpoint_kind: Option<ModelEndpointSelectionKind>,
    pub endpoint_kind_label: Option<String>,
    pub endpoint_auto: Option<bool>,
    pub endpoint_built_in: Option<bool>,
    pub endpoint_custom: Option<bool>,
    pub wire_model_role_label: Option<String>,
    pub wire_routing_preference_label: Option<String>,
    pub wire_prefer_fast: Option<bool>,
    pub wire_prefer_quality: Option<bool>,
    pub wire_endpoint_pinned: Option<bool>,
    pub wire_endpoint_kind_label: Option<String>,
    pub wire_sends_model_endpoint: Option<bool>,
    pub wire_model_endpoint_label: Option<String>,
    pub session_config_update: Option<SessionConfigUpdate>,
    pub session_config_update_detail: Option<SessionConfigUpdateSnapshot>,
    pub local_status: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputRoleOptionSnapshot {
    pub role: ModelRole,
    pub role_label: String,
    pub selected: bool,
    pub selection_intent: RoutingIntent,
    pub selection_summary: String,
    pub selection_model_role_label: String,
    pub selection_routing_preference_label: String,
    pub selection_endpoint_label: String,
    pub selection_endpoint_kind: ModelEndpointSelectionKind,
    pub selection_endpoint_kind_label: String,
    pub selection_endpoint_auto: bool,
    pub selection_endpoint_built_in: bool,
    pub selection_endpoint_custom: bool,
    pub selection_wire_model_role_label: String,
    pub selection_wire_routing_preference_label: String,
    pub selection_wire_prefer_fast: bool,
    pub selection_wire_prefer_quality: bool,
    pub selection_wire_endpoint_pinned: bool,
    pub selection_wire_endpoint_kind_label: String,
    pub selection_wire_sends_model_endpoint: bool,
    pub selection_wire_model_endpoint_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputPreferenceOptionSnapshot {
    pub preference: RoutingPreference,
    pub preference_label: String,
    pub selected: bool,
    pub selection_intent: RoutingIntent,
    pub selection_summary: String,
    pub selection_model_role_label: String,
    pub selection_routing_preference_label: String,
    pub selection_endpoint_label: String,
    pub selection_endpoint_kind: ModelEndpointSelectionKind,
    pub selection_endpoint_kind_label: String,
    pub selection_endpoint_auto: bool,
    pub selection_endpoint_built_in: bool,
    pub selection_endpoint_custom: bool,
    pub selection_wire_model_role_label: String,
    pub selection_wire_routing_preference_label: String,
    pub selection_wire_prefer_fast: bool,
    pub selection_wire_prefer_quality: bool,
    pub selection_wire_endpoint_pinned: bool,
    pub selection_wire_endpoint_kind_label: String,
    pub selection_wire_sends_model_endpoint: bool,
    pub selection_wire_model_endpoint_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEndpointOptionSnapshot {
    pub endpoint: Option<ModelEndpoint>,
    pub endpoint_label: String,
    pub endpoint_pinned: bool,
    pub endpoint_kind: ModelEndpointSelectionKind,
    pub endpoint_kind_label: String,
    pub endpoint_auto: bool,
    pub endpoint_built_in: bool,
    pub endpoint_custom: bool,
    pub selected: bool,
    pub selection_intent: RoutingIntent,
    pub selection_summary: String,
    pub selection_model_role_label: String,
    pub selection_routing_preference_label: String,
    pub selection_endpoint_label: String,
    pub selection_endpoint_kind: ModelEndpointSelectionKind,
    pub selection_endpoint_kind_label: String,
    pub selection_endpoint_auto: bool,
    pub selection_endpoint_built_in: bool,
    pub selection_endpoint_custom: bool,
    pub selection_wire_model_role_label: String,
    pub selection_wire_routing_preference_label: String,
    pub selection_wire_prefer_fast: bool,
    pub selection_wire_prefer_quality: bool,
    pub selection_wire_endpoint_pinned: bool,
    pub selection_wire_endpoint_kind_label: String,
    pub selection_wire_sends_model_endpoint: bool,
    pub selection_wire_model_endpoint_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputRouteOptionsSnapshot {
    pub roles: Vec<ModelRole>,
    pub role_labels: Vec<String>,
    pub role_options: Vec<InputRoleOptionSnapshot>,
    pub preferences: Vec<RoutingPreference>,
    pub preference_labels: Vec<String>,
    pub preference_options: Vec<InputPreferenceOptionSnapshot>,
    pub built_in_endpoints: Vec<ModelEndpoint>,
    pub built_in_endpoint_labels: Vec<String>,
    pub endpoint_options: Vec<InputEndpointOptionSnapshot>,
    pub auto_endpoint_label: String,
    pub auto_endpoint_selected: bool,
    pub auto_selection_intent: RoutingIntent,
    pub auto_selection_summary: String,
    pub auto_selection_model_role_label: String,
    pub auto_selection_routing_preference_label: String,
    pub auto_selection_endpoint_label: String,
    pub auto_selection_endpoint_kind: ModelEndpointSelectionKind,
    pub auto_selection_endpoint_kind_label: String,
    pub auto_selection_endpoint_auto: bool,
    pub auto_selection_endpoint_built_in: bool,
    pub auto_selection_endpoint_custom: bool,
    pub auto_selection_wire_model_role_label: String,
    pub auto_selection_wire_routing_preference_label: String,
    pub auto_selection_wire_prefer_fast: bool,
    pub auto_selection_wire_prefer_quality: bool,
    pub auto_selection_wire_endpoint_pinned: bool,
    pub auto_selection_wire_endpoint_kind_label: String,
    pub auto_selection_wire_sends_model_endpoint: bool,
    pub auto_selection_wire_model_endpoint_label: Option<String>,
    pub selected_role: ModelRole,
    pub selected_role_label: String,
    pub selected_preference: RoutingPreference,
    pub selected_preference_label: String,
    pub selected_endpoint_label: String,
    pub selected_endpoint_kind: ModelEndpointSelectionKind,
    pub selected_endpoint_kind_label: String,
    pub selected_endpoint_auto: bool,
    pub selected_endpoint_built_in: bool,
    pub selected_endpoint_custom: bool,
    pub endpoint_pinned: bool,
    pub selected_wire_model_role_label: String,
    pub selected_wire_routing_preference_label: String,
    pub selected_wire_prefer_fast: bool,
    pub selected_wire_prefer_quality: bool,
    pub selected_wire_endpoint_pinned: bool,
    pub selected_wire_endpoint_kind_label: String,
    pub selected_wire_sends_model_endpoint: bool,
    pub selected_wire_model_endpoint_label: Option<String>,
}

impl InputRoleOptionSnapshot {
    fn from_selection(current: &RoutingIntent, selection_intent: RoutingIntent) -> Self {
        let endpoint_kind = selection_intent.endpoint_kind();
        let selection_wire = selection_intent.wire_snapshot();

        Self {
            role: selection_intent.model_role,
            role_label: selection_intent.model_role_label().to_owned(),
            selected: current.model_role == selection_intent.model_role,
            selection_summary: selection_intent.summary(),
            selection_model_role_label: selection_intent.model_role_label().to_owned(),
            selection_routing_preference_label: selection_intent
                .routing_preference_label()
                .to_owned(),
            selection_endpoint_label: selection_intent.endpoint_label().to_owned(),
            selection_endpoint_kind: endpoint_kind,
            selection_endpoint_kind_label: selection_intent.endpoint_kind_label().to_owned(),
            selection_endpoint_auto: selection_intent.endpoint_auto(),
            selection_endpoint_built_in: selection_intent.endpoint_built_in(),
            selection_endpoint_custom: selection_intent.endpoint_custom(),
            selection_wire_model_role_label: selection_wire.model_role_label,
            selection_wire_routing_preference_label: selection_wire.routing_preference_label,
            selection_wire_prefer_fast: selection_wire.prefer_fast,
            selection_wire_prefer_quality: selection_wire.prefer_quality,
            selection_wire_endpoint_pinned: selection_wire.endpoint_pinned,
            selection_wire_endpoint_kind_label: selection_wire.endpoint_kind_label,
            selection_wire_sends_model_endpoint: selection_wire.sends_model_endpoint,
            selection_wire_model_endpoint_label: selection_wire.model_endpoint_label,
            selection_intent,
        }
    }
}

impl InputPreferenceOptionSnapshot {
    fn from_selection(current: &RoutingIntent, selection_intent: RoutingIntent) -> Self {
        let endpoint_kind = selection_intent.endpoint_kind();
        let selection_wire = selection_intent.wire_snapshot();

        Self {
            preference: selection_intent.routing_preference,
            preference_label: selection_intent.routing_preference_label().to_owned(),
            selected: current.routing_preference == selection_intent.routing_preference,
            selection_summary: selection_intent.summary(),
            selection_model_role_label: selection_intent.model_role_label().to_owned(),
            selection_routing_preference_label: selection_intent
                .routing_preference_label()
                .to_owned(),
            selection_endpoint_label: selection_intent.endpoint_label().to_owned(),
            selection_endpoint_kind: endpoint_kind,
            selection_endpoint_kind_label: selection_intent.endpoint_kind_label().to_owned(),
            selection_endpoint_auto: selection_intent.endpoint_auto(),
            selection_endpoint_built_in: selection_intent.endpoint_built_in(),
            selection_endpoint_custom: selection_intent.endpoint_custom(),
            selection_wire_model_role_label: selection_wire.model_role_label,
            selection_wire_routing_preference_label: selection_wire.routing_preference_label,
            selection_wire_prefer_fast: selection_wire.prefer_fast,
            selection_wire_prefer_quality: selection_wire.prefer_quality,
            selection_wire_endpoint_pinned: selection_wire.endpoint_pinned,
            selection_wire_endpoint_kind_label: selection_wire.endpoint_kind_label,
            selection_wire_sends_model_endpoint: selection_wire.sends_model_endpoint,
            selection_wire_model_endpoint_label: selection_wire.model_endpoint_label,
            selection_intent,
        }
    }
}

impl InputEndpointOptionSnapshot {
    fn from_selection(current: &RoutingIntent, selection_intent: RoutingIntent) -> Self {
        let endpoint_kind = selection_intent.endpoint_kind();
        let selection_wire = selection_intent.wire_snapshot();
        let selected = if selection_intent.endpoint_pinned {
            current.endpoint_pinned
                && current
                    .model_endpoint
                    .as_ref()
                    .zip(selection_intent.model_endpoint.as_ref())
                    .is_some_and(|(current_endpoint, selection_endpoint)| {
                        current_endpoint
                            .label()
                            .eq_ignore_ascii_case(selection_endpoint.label())
                    })
        } else {
            !current.endpoint_pinned
        };

        Self {
            endpoint: selection_intent.model_endpoint.clone(),
            endpoint_label: selection_intent.endpoint_label().to_owned(),
            endpoint_pinned: selection_intent.endpoint_pinned,
            endpoint_kind,
            endpoint_kind_label: selection_intent.endpoint_kind_label().to_owned(),
            endpoint_auto: selection_intent.endpoint_auto(),
            endpoint_built_in: selection_intent.endpoint_built_in(),
            endpoint_custom: selection_intent.endpoint_custom(),
            selected,
            selection_summary: selection_intent.summary(),
            selection_model_role_label: selection_intent.model_role_label().to_owned(),
            selection_routing_preference_label: selection_intent
                .routing_preference_label()
                .to_owned(),
            selection_endpoint_label: selection_intent.endpoint_label().to_owned(),
            selection_endpoint_kind: endpoint_kind,
            selection_endpoint_kind_label: selection_intent.endpoint_kind_label().to_owned(),
            selection_endpoint_auto: selection_intent.endpoint_auto(),
            selection_endpoint_built_in: selection_intent.endpoint_built_in(),
            selection_endpoint_custom: selection_intent.endpoint_custom(),
            selection_wire_model_role_label: selection_wire.model_role_label,
            selection_wire_routing_preference_label: selection_wire.routing_preference_label,
            selection_wire_prefer_fast: selection_wire.prefer_fast,
            selection_wire_prefer_quality: selection_wire.prefer_quality,
            selection_wire_endpoint_pinned: selection_wire.endpoint_pinned,
            selection_wire_endpoint_kind_label: selection_wire.endpoint_kind_label,
            selection_wire_sends_model_endpoint: selection_wire.sends_model_endpoint,
            selection_wire_model_endpoint_label: selection_wire.model_endpoint_label,
            selection_intent,
        }
    }
}

fn role_selection_intent(intent: &RoutingIntent, role: ModelRole) -> RoutingIntent {
    RoutingIntent {
        model_role: role,
        routing_preference: intent.routing_preference,
        model_endpoint: if intent.endpoint_pinned {
            intent.model_endpoint.clone()
        } else {
            None
        },
        endpoint_pinned: intent.endpoint_pinned,
    }
}

fn preference_selection_intent(
    intent: &RoutingIntent,
    preference: RoutingPreference,
) -> RoutingIntent {
    RoutingIntent {
        model_role: intent.model_role,
        routing_preference: preference,
        model_endpoint: if intent.endpoint_pinned {
            intent.model_endpoint.clone()
        } else {
            None
        },
        endpoint_pinned: intent.endpoint_pinned,
    }
}

fn role_options_for_intent(intent: &RoutingIntent) -> Vec<InputRoleOptionSnapshot> {
    ModelRole::ALL
        .into_iter()
        .map(|role| role_selection_intent(intent, role))
        .map(|selection_intent| InputRoleOptionSnapshot::from_selection(intent, selection_intent))
        .collect()
}

fn preference_options_for_intent(intent: &RoutingIntent) -> Vec<InputPreferenceOptionSnapshot> {
    RoutingPreference::ALL
        .into_iter()
        .map(|preference| preference_selection_intent(intent, preference))
        .map(|selection_intent| {
            InputPreferenceOptionSnapshot::from_selection(intent, selection_intent)
        })
        .collect()
}

fn endpoint_options_for_intent(intent: &RoutingIntent) -> Vec<InputEndpointOptionSnapshot> {
    std::iter::once(RoutingIntent::auto_route(
        intent.model_role,
        intent.routing_preference,
    ))
    .chain(ModelEndpoint::BUILT_INS.iter().cloned().map(|endpoint| {
        RoutingIntent::operator_pinned(intent.model_role, intent.routing_preference, endpoint)
    }))
    .map(|selection_intent| InputEndpointOptionSnapshot::from_selection(intent, selection_intent))
    .collect()
}

impl Default for InputRouteOptionsSnapshot {
    fn default() -> Self {
        let roles = ModelRole::ALL.to_vec();
        let role_labels = roles.iter().map(|role| role.as_str().to_owned()).collect();
        let preferences = RoutingPreference::ALL.to_vec();
        let preference_labels = preferences
            .iter()
            .map(|preference| preference.as_str().to_owned())
            .collect();
        let built_in_endpoints = ModelEndpoint::BUILT_INS.to_vec();
        let built_in_endpoint_labels = built_in_endpoints
            .iter()
            .map(|endpoint| endpoint.label().to_owned())
            .collect();
        let auto_selection_intent =
            RoutingIntent::auto_route(ModelRole::Assistant, RoutingPreference::Balanced);
        let auto_selection_endpoint_kind = auto_selection_intent.endpoint_kind();
        let auto_selection_wire = auto_selection_intent.wire_snapshot();
        let role_options = role_options_for_intent(&auto_selection_intent);
        let preference_options = preference_options_for_intent(&auto_selection_intent);
        let endpoint_options = endpoint_options_for_intent(&auto_selection_intent);

        Self {
            roles,
            role_labels,
            role_options,
            preferences,
            preference_labels,
            preference_options,
            built_in_endpoints,
            built_in_endpoint_labels,
            endpoint_options,
            auto_endpoint_label: "auto".to_owned(),
            auto_endpoint_selected: true,
            auto_selection_summary: auto_selection_intent.summary(),
            auto_selection_model_role_label: auto_selection_intent.model_role_label().to_owned(),
            auto_selection_routing_preference_label: auto_selection_intent
                .routing_preference_label()
                .to_owned(),
            auto_selection_endpoint_label: auto_selection_intent.endpoint_label().to_owned(),
            auto_selection_endpoint_kind,
            auto_selection_endpoint_kind_label: auto_selection_intent
                .endpoint_kind_label()
                .to_owned(),
            auto_selection_endpoint_auto: auto_selection_intent.endpoint_auto(),
            auto_selection_endpoint_built_in: auto_selection_intent.endpoint_built_in(),
            auto_selection_endpoint_custom: auto_selection_intent.endpoint_custom(),
            auto_selection_wire_model_role_label: auto_selection_wire.model_role_label,
            auto_selection_wire_routing_preference_label: auto_selection_wire
                .routing_preference_label,
            auto_selection_wire_prefer_fast: auto_selection_wire.prefer_fast,
            auto_selection_wire_prefer_quality: auto_selection_wire.prefer_quality,
            auto_selection_wire_endpoint_pinned: auto_selection_wire.endpoint_pinned,
            auto_selection_wire_endpoint_kind_label: auto_selection_wire.endpoint_kind_label,
            auto_selection_wire_sends_model_endpoint: auto_selection_wire.sends_model_endpoint,
            auto_selection_wire_model_endpoint_label: auto_selection_wire.model_endpoint_label,
            auto_selection_intent,
            selected_role: ModelRole::Assistant,
            selected_role_label: ModelRole::Assistant.as_str().to_owned(),
            selected_preference: RoutingPreference::Balanced,
            selected_preference_label: RoutingPreference::Balanced.as_str().to_owned(),
            selected_endpoint_label: "auto".to_owned(),
            selected_endpoint_kind: ModelEndpointSelectionKind::Auto,
            selected_endpoint_kind_label: ModelEndpointSelectionKind::Auto.as_str().to_owned(),
            selected_endpoint_auto: true,
            selected_endpoint_built_in: false,
            selected_endpoint_custom: false,
            endpoint_pinned: false,
            selected_wire_model_role_label: ModelRole::Assistant.as_str().to_owned(),
            selected_wire_routing_preference_label: RoutingPreference::Balanced.as_str().to_owned(),
            selected_wire_prefer_fast: false,
            selected_wire_prefer_quality: false,
            selected_wire_endpoint_pinned: false,
            selected_wire_endpoint_kind_label: ModelEndpointSelectionKind::Auto.as_str().to_owned(),
            selected_wire_sends_model_endpoint: false,
            selected_wire_model_endpoint_label: None,
        }
    }
}

impl InputRouteOptionsSnapshot {
    pub fn from_intent(intent: &RoutingIntent) -> Self {
        let selected_endpoint_kind = intent.endpoint_kind();
        let auto_selection_intent =
            RoutingIntent::auto_route(intent.model_role, intent.routing_preference);
        let auto_selection_endpoint_kind = auto_selection_intent.endpoint_kind();
        let auto_selection_wire = auto_selection_intent.wire_snapshot();
        let selected_wire = intent.wire_snapshot();
        let role_options = role_options_for_intent(intent);
        let preference_options = preference_options_for_intent(intent);
        let endpoint_options = endpoint_options_for_intent(intent);

        Self {
            role_options,
            preference_options,
            endpoint_options,
            auto_endpoint_selected: !intent.endpoint_pinned,
            auto_selection_summary: auto_selection_intent.summary(),
            auto_selection_model_role_label: auto_selection_intent.model_role_label().to_owned(),
            auto_selection_routing_preference_label: auto_selection_intent
                .routing_preference_label()
                .to_owned(),
            auto_selection_endpoint_label: auto_selection_intent.endpoint_label().to_owned(),
            auto_selection_endpoint_kind,
            auto_selection_endpoint_kind_label: auto_selection_intent
                .endpoint_kind_label()
                .to_owned(),
            auto_selection_endpoint_auto: auto_selection_intent.endpoint_auto(),
            auto_selection_endpoint_built_in: auto_selection_intent.endpoint_built_in(),
            auto_selection_endpoint_custom: auto_selection_intent.endpoint_custom(),
            auto_selection_wire_model_role_label: auto_selection_wire.model_role_label,
            auto_selection_wire_routing_preference_label: auto_selection_wire
                .routing_preference_label,
            auto_selection_wire_prefer_fast: auto_selection_wire.prefer_fast,
            auto_selection_wire_prefer_quality: auto_selection_wire.prefer_quality,
            auto_selection_wire_endpoint_pinned: auto_selection_wire.endpoint_pinned,
            auto_selection_wire_endpoint_kind_label: auto_selection_wire.endpoint_kind_label,
            auto_selection_wire_sends_model_endpoint: auto_selection_wire.sends_model_endpoint,
            auto_selection_wire_model_endpoint_label: auto_selection_wire.model_endpoint_label,
            auto_selection_intent,
            selected_role: intent.model_role,
            selected_role_label: intent.model_role_label().to_owned(),
            selected_preference: intent.routing_preference,
            selected_preference_label: intent.routing_preference_label().to_owned(),
            selected_endpoint_label: intent.endpoint_label().to_owned(),
            selected_endpoint_kind,
            selected_endpoint_kind_label: intent.endpoint_kind_label().to_owned(),
            selected_endpoint_auto: intent.endpoint_auto(),
            selected_endpoint_built_in: intent.endpoint_built_in(),
            selected_endpoint_custom: intent.endpoint_custom(),
            endpoint_pinned: intent.endpoint_pinned,
            selected_wire_model_role_label: selected_wire.model_role_label,
            selected_wire_routing_preference_label: selected_wire.routing_preference_label,
            selected_wire_prefer_fast: selected_wire.prefer_fast,
            selected_wire_prefer_quality: selected_wire.prefer_quality,
            selected_wire_endpoint_pinned: selected_wire.endpoint_pinned,
            selected_wire_endpoint_kind_label: selected_wire.endpoint_kind_label,
            selected_wire_sends_model_endpoint: selected_wire.sends_model_endpoint,
            selected_wire_model_endpoint_label: selected_wire.model_endpoint_label,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSessionPolicySnapshot {
    pub history_messages: usize,
    pub history_limit: usize,
    pub has_history: bool,
    pub is_empty_history: bool,
    pub max_tokens: Option<usize>,
    pub max_tokens_label: String,
    pub history_remaining: usize,
    pub history_at_limit: bool,
}

impl InputSessionPolicySnapshot {
    pub fn from_status(status: &CliStatusSnapshot) -> Self {
        let history_remaining = status.history_limit.saturating_sub(status.history_messages);

        Self {
            history_messages: status.history_messages,
            history_limit: status.history_limit,
            has_history: status.history_messages > 0,
            is_empty_history: status.history_messages == 0,
            max_tokens: status.max_tokens,
            max_tokens_label: status
                .max_tokens
                .map(|value| value.to_string())
                .unwrap_or_else(|| "backend-default".to_owned()),
            history_remaining,
            history_at_limit: history_remaining == 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputControlSnapshot {
    pub readiness: InputReadinessSnapshot,
    pub status: CliStatusSnapshot,
    pub route_options: InputRouteOptionsSnapshot,
    pub status_route_matches_readiness: bool,
    pub status_route_is_stale: bool,
    pub session_policy: InputSessionPolicySnapshot,
    pub send_enabled: bool,
    pub enter_submits_prompt: bool,
    pub enter_runs_local_command: bool,
    pub enter_is_blocked: bool,
    pub primary_action_label: String,
    pub primary_action_enabled: bool,
    pub primary_action_disabled_reason: Option<String>,
    pub preserves_buffer_on_enter: bool,
    pub clears_buffer_on_enter: bool,
    pub advice_action: Option<GateAdviceAction>,
    pub advice_action_label: Option<String>,
    pub advice_state_label: Option<String>,
    pub advice_reason: Option<String>,
    pub block_state: Option<StreamState>,
    pub block_state_label: Option<String>,
    pub block_state_is_terminal: bool,
    pub block_state_is_pressure: bool,
    pub block_state_blocks_prompt_submit: bool,
    pub block_chunk: Option<ChatChunkDisplaySnapshot>,
    pub block_reason: Option<String>,
    pub send_block_reason: Option<String>,
    pub route_gate_advice: Option<String>,
    pub route_gate_advice_detail: Option<GateAdvice>,
    pub route_gate_advice_action_label: Option<String>,
    pub route_gate_advice_state_label: Option<String>,
    pub route_gate_advice_reason: Option<String>,
    pub route_send_allowed: Option<bool>,
    pub route_send_block_state: Option<StreamState>,
    pub route_send_block_state_label: Option<String>,
    pub route_send_block_state_is_terminal: Option<bool>,
    pub route_send_block_state_is_pressure: Option<bool>,
    pub route_send_block_state_blocks_prompt_submit: Option<bool>,
    pub route_send_block_chunk: Option<ChatChunkDisplaySnapshot>,
    pub route_send_block_reason: Option<String>,
    pub pool_status: Option<String>,
    pub pool_queue_label: Option<String>,
    pub pool_capacity_state: Option<StreamState>,
    pub pool_capacity_state_label: Option<String>,
    pub pool_capacity_state_is_pressure: Option<bool>,
    pub pool_capacity_state_blocks_prompt_submit: Option<bool>,
    pub pool_has_workers: Option<bool>,
    pub pool_has_available_workers: Option<bool>,
    pub pool_has_busy_workers: Option<bool>,
    pub pool_has_saturated_workers: Option<bool>,
    pub pool_has_queued_requests: Option<bool>,
    pub pool_queue_is_saturated: Option<bool>,
    pub route_pool_status: Option<String>,
    pub route_pool_queue_label: Option<String>,
    pub route_pool_capacity_state: Option<StreamState>,
    pub route_pool_capacity_state_label: Option<String>,
    pub route_pool_capacity_state_is_pressure: Option<bool>,
    pub route_pool_capacity_state_blocks_prompt_submit: Option<bool>,
    pub route_pool_has_matching_workers: Option<bool>,
    pub route_pool_has_matching_available_workers: Option<bool>,
    pub route_pool_has_matching_busy_workers: Option<bool>,
    pub route_pool_has_matching_saturated_workers: Option<bool>,
    pub route_pool_has_matching_queued_requests: Option<bool>,
    pub route_pool_queue_is_saturated: Option<bool>,
    pub workers: Option<Vec<ModelWorkerSnapshot>>,
    pub route_workers: Option<Vec<ModelRouteWorkerSnapshot>>,
    pub request_preview: Option<InputRequestSnapshot>,
    pub prompt_submit_control: Option<GateSendControl>,
}

impl InputControlSnapshot {
    pub fn new(readiness: InputReadinessSnapshot, status: CliStatusSnapshot) -> Self {
        let send_enabled = readiness.send_allowed;
        let enter_submits_prompt = readiness.enter_submits_prompt;
        let enter_runs_local_command = readiness.enter_runs_local_command;
        let enter_is_blocked = readiness.enter_is_blocked;
        let primary_action_label = readiness.primary_action_label.clone();
        let primary_action_enabled = readiness.primary_action_enabled;
        let primary_action_disabled_reason = readiness.primary_action_disabled_reason.clone();
        let preserves_buffer_on_enter = readiness.preserves_buffer_on_enter;
        let clears_buffer_on_enter = readiness.clears_buffer_on_enter;
        let advice_action = readiness.advice_action.or_else(|| {
            status
                .gate_advice_detail
                .as_ref()
                .map(|advice| advice.action)
        });
        let block_state = readiness.block_state.or(status.send_block_state);
        let block_reason = readiness.block_reason.clone().or_else(|| {
            status
                .gate_advice_detail
                .as_ref()
                .map(|advice| advice.reason.clone())
        });
        let block_chunk = readiness
            .block_chunk
            .clone()
            .or_else(|| status.send_block_chunk.clone())
            .or_else(|| block_display_snapshot(block_state, block_reason.as_deref()));
        let request_preview = readiness.request_preview.clone();
        let prompt_submit_control = readiness.prompt_submit_control.clone();
        let session_policy = InputSessionPolicySnapshot::from_status(&status);
        let advice_action_label = advice_action.map(|action| action.as_str().to_owned());
        let block_state_label = block_state.map(|state| state.as_str().to_owned());
        let block_state_is_terminal = block_state.is_some_and(StreamState::is_terminal);
        let block_state_is_pressure = block_state.is_some_and(StreamState::is_pressure);
        let block_state_blocks_prompt_submit =
            block_state.is_some_and(StreamState::blocks_prompt_submit);
        let route_gate_advice = status.route_gate_advice.clone();
        let route_gate_advice_detail = status.route_gate_advice_detail.clone();
        let route_gate_advice_action_label = status.route_gate_advice_action_label.clone();
        let route_gate_advice_state_label = status.route_gate_advice_state_label.clone();
        let route_gate_advice_reason = status.route_gate_advice_reason.clone();
        let send_block_reason = status.send_block_reason.clone();
        let route_send_allowed = status.route_send_allowed;
        let route_send_block_state = status.route_send_block_state;
        let route_send_block_state_label = status.route_send_block_state_label.clone();
        let route_send_block_state_is_terminal = status.route_send_block_state_is_terminal;
        let route_send_block_state_is_pressure = status.route_send_block_state_is_pressure;
        let route_send_block_state_blocks_prompt_submit =
            status.route_send_block_state_blocks_prompt_submit;
        let route_send_block_chunk = status.route_send_block_chunk.clone();
        let route_send_block_reason = status.route_send_block_reason.clone();
        let pool_status = status.pool_status.clone();
        let pool_queue_label = status.pool_queue_label.clone();
        let pool_capacity_state = status.pool_capacity_state;
        let pool_capacity_state_label = status.pool_capacity_state_label.clone();
        let pool_capacity_state_is_pressure = status.pool_capacity_state_is_pressure;
        let pool_capacity_state_blocks_prompt_submit =
            status.pool_capacity_state_blocks_prompt_submit;
        let pool_has_workers = status.pool_has_workers;
        let pool_has_available_workers = status.pool_has_available_workers;
        let pool_has_busy_workers = status.pool_has_busy_workers;
        let pool_has_saturated_workers = status.pool_has_saturated_workers;
        let pool_has_queued_requests = status.pool_has_queued_requests;
        let pool_queue_is_saturated = status.pool_queue_is_saturated;
        let route_pool_status = status.route_pool_status.clone();
        let route_pool_queue_label = status.route_pool_queue_label.clone();
        let route_pool_capacity_state = status.route_pool_capacity_state;
        let route_pool_capacity_state_label = status.route_pool_capacity_state_label.clone();
        let route_pool_capacity_state_is_pressure = status.route_pool_capacity_state_is_pressure;
        let route_pool_capacity_state_blocks_prompt_submit =
            status.route_pool_capacity_state_blocks_prompt_submit;
        let route_pool_has_matching_workers = status.route_pool_has_matching_workers;
        let route_pool_has_matching_available_workers =
            status.route_pool_has_matching_available_workers;
        let route_pool_has_matching_busy_workers = status.route_pool_has_matching_busy_workers;
        let route_pool_has_matching_saturated_workers =
            status.route_pool_has_matching_saturated_workers;
        let route_pool_has_matching_queued_requests =
            status.route_pool_has_matching_queued_requests;
        let route_pool_queue_is_saturated = status.route_pool_queue_is_saturated;
        let workers = status.workers.clone();
        let route_workers = status.route_workers.clone();
        let advice_state_label = block_state_label
            .clone()
            .or_else(|| status.gate_advice_state_label.clone());
        let advice_reason = block_reason
            .clone()
            .or_else(|| status.gate_advice_reason.clone());
        let route_options = InputRouteOptionsSnapshot::from_intent(&readiness.routing_intent);
        let status_route_matches_readiness = status.routing_intent == readiness.routing_intent;
        let status_route_is_stale = !status_route_matches_readiness;

        Self {
            readiness,
            route_options,
            status,
            status_route_matches_readiness,
            status_route_is_stale,
            session_policy,
            send_enabled,
            enter_submits_prompt,
            enter_runs_local_command,
            enter_is_blocked,
            primary_action_label,
            primary_action_enabled,
            primary_action_disabled_reason,
            preserves_buffer_on_enter,
            clears_buffer_on_enter,
            advice_action,
            advice_action_label,
            advice_state_label,
            advice_reason,
            block_state,
            block_state_label,
            block_state_is_terminal,
            block_state_is_pressure,
            block_state_blocks_prompt_submit,
            block_chunk,
            block_reason,
            send_block_reason,
            route_gate_advice,
            route_gate_advice_detail,
            route_gate_advice_action_label,
            route_gate_advice_state_label,
            route_gate_advice_reason,
            route_send_allowed,
            route_send_block_state,
            route_send_block_state_label,
            route_send_block_state_is_terminal,
            route_send_block_state_is_pressure,
            route_send_block_state_blocks_prompt_submit,
            route_send_block_chunk,
            route_send_block_reason,
            pool_status,
            pool_queue_label,
            pool_capacity_state,
            pool_capacity_state_label,
            pool_capacity_state_is_pressure,
            pool_capacity_state_blocks_prompt_submit,
            pool_has_workers,
            pool_has_available_workers,
            pool_has_busy_workers,
            pool_has_saturated_workers,
            pool_has_queued_requests,
            pool_queue_is_saturated,
            route_pool_status,
            route_pool_queue_label,
            route_pool_capacity_state,
            route_pool_capacity_state_label,
            route_pool_capacity_state_is_pressure,
            route_pool_capacity_state_blocks_prompt_submit,
            route_pool_has_matching_workers,
            route_pool_has_matching_available_workers,
            route_pool_has_matching_busy_workers,
            route_pool_has_matching_saturated_workers,
            route_pool_has_matching_queued_requests,
            route_pool_queue_is_saturated,
            workers,
            route_workers,
            request_preview,
            prompt_submit_control,
        }
    }
}

impl InputSubmitMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::Record => "record",
            Self::StartStream => "start_stream",
        }
    }

    fn ready_prompt_action(self) -> InputActionKind {
        match self {
            Self::Preview | Self::Record => InputActionKind::Send,
            Self::StartStream => InputActionKind::StartStream,
        }
    }

    fn records_user_on_enter(self) -> bool {
        matches!(self, Self::Record | Self::StartStream)
    }

    fn starts_stream_on_enter(self) -> bool {
        matches!(self, Self::StartStream)
    }
}

fn enter_clears_buffer(action: InputActionKind) -> bool {
    matches!(
        action,
        InputActionKind::Send
            | InputActionKind::StartStream
            | InputActionKind::RoutingChanged
            | InputActionKind::Status
            | InputActionKind::SessionConfigChanged
    )
}

impl InputAction {
    pub fn kind(&self) -> InputActionKind {
        match self {
            Self::BufferChanged => InputActionKind::BufferChanged,
            Self::Send(_) => InputActionKind::Send,
            Self::StartStream(_) => InputActionKind::StartStream,
            Self::Blocked(_) => InputActionKind::Blocked,
            Self::StreamCancelled(_) => InputActionKind::StreamCancelled,
            Self::RoutingChanged(_) => InputActionKind::RoutingChanged,
            Self::Status(_) => InputActionKind::Status,
            Self::SessionConfigChanged { .. } => InputActionKind::SessionConfigChanged,
            Self::InputError(_) => InputActionKind::InputError,
            Self::InsertNewline => InputActionKind::InsertNewline,
            Self::CancelStream => InputActionKind::CancelStream,
            Self::Quit => InputActionKind::Quit,
            Self::Noop => InputActionKind::Noop,
        }
    }

    pub fn snapshot(&self, input: &CliInputConfig) -> InputActionSnapshot {
        let kind = self.kind();
        let request = match self {
            Self::Send(request) => Some(InputRequestSnapshot::from_request(request)),
            Self::StartStream(turn) => Some(InputRequestSnapshot::from_request(&turn.request)),
            _ => None,
        };
        let (stream_state, reason) = match self {
            Self::Blocked(chunk) | Self::StreamCancelled(chunk) => (
                Some(chunk.state),
                (!chunk.content.trim().is_empty()).then(|| chunk.content.clone()),
            ),
            _ => (None, None),
        };
        let stream_chunk = match self {
            Self::Blocked(chunk) | Self::StreamCancelled(chunk) => Some(chunk.display_snapshot()),
            _ => None,
        };
        let start_chunk = match self {
            Self::StartStream(turn) => Some(turn.start.display_snapshot()),
            _ => None,
        };
        let start_sequence = start_chunk.as_ref().map(|chunk| chunk.sequence);
        let start_state = start_chunk.as_ref().map(|chunk| chunk.state);
        let local_status = match self {
            Self::RoutingChanged(summary) | Self::Status(summary) => Some(summary.clone()),
            Self::SessionConfigChanged { summary, .. } => Some(summary.clone()),
            Self::InputError(reason) => Some(reason.clone()),
            _ => None,
        };
        let session_config_update = match self {
            Self::SessionConfigChanged { update, .. } => Some(update.clone()),
            _ => None,
        };
        let session_config_update_detail = session_config_update
            .as_ref()
            .map(SessionConfigUpdate::snapshot);
        let stream_state_label = stream_chunk.as_ref().map(|chunk| chunk.state_label.clone());
        let start_state_label = start_chunk.as_ref().map(|chunk| chunk.state_label.clone());
        let stream_state_is_terminal = stream_chunk.as_ref().map(|chunk| chunk.state_is_terminal);
        let stream_state_is_pressure = stream_chunk.as_ref().map(|chunk| chunk.state_is_pressure);
        let stream_state_blocks_prompt_submit = stream_chunk
            .as_ref()
            .map(|chunk| chunk.state_blocks_prompt_submit);
        let start_state_is_terminal = start_chunk.as_ref().map(|chunk| chunk.state_is_terminal);
        let start_state_is_pressure = start_chunk.as_ref().map(|chunk| chunk.state_is_pressure);
        let start_state_blocks_prompt_submit = start_chunk
            .as_ref()
            .map(|chunk| chunk.state_blocks_prompt_submit);
        let routing_intent = request
            .as_ref()
            .map(|request| request.routing_intent.clone())
            .unwrap_or_else(|| input.routing_intent());
        let endpoint_kind = routing_intent.endpoint_kind();
        let route_wire = routing_intent.wire_snapshot();

        InputActionSnapshot {
            kind,
            kind_label: kind.as_str().to_owned(),
            model_role_label: routing_intent.model_role_label().to_owned(),
            routing_preference_label: routing_intent.routing_preference_label().to_owned(),
            endpoint_label: routing_intent.endpoint_label().to_owned(),
            endpoint_pinned: routing_intent.endpoint_pinned,
            endpoint_kind,
            endpoint_kind_label: routing_intent.endpoint_kind_label().to_owned(),
            endpoint_auto: routing_intent.endpoint_auto(),
            endpoint_built_in: routing_intent.endpoint_built_in(),
            endpoint_custom: routing_intent.endpoint_custom(),
            wire_model_role_label: route_wire.model_role_label,
            wire_routing_preference_label: route_wire.routing_preference_label,
            wire_prefer_fast: route_wire.prefer_fast,
            wire_prefer_quality: route_wire.prefer_quality,
            wire_endpoint_pinned: route_wire.endpoint_pinned,
            wire_endpoint_kind_label: route_wire.endpoint_kind_label,
            wire_sends_model_endpoint: route_wire.sends_model_endpoint,
            wire_model_endpoint_label: route_wire.model_endpoint_label,
            routing_intent,
            request,
            session_config_update,
            session_config_update_detail,
            stream_state,
            stream_state_label,
            stream_state_is_terminal,
            stream_state_is_pressure,
            stream_state_blocks_prompt_submit,
            stream_chunk,
            reason,
            local_status,
            start_sequence,
            start_state,
            start_state_label,
            start_state_is_terminal,
            start_state_is_pressure,
            start_state_blocks_prompt_submit,
            start_chunk,
        }
    }
}

impl InputRequestSnapshot {
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionConfigUpdate {
    DefaultMaxTokens(Option<usize>),
    HistoryLimit(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfigUpdateSnapshot {
    pub update: SessionConfigUpdate,
    pub kind_label: String,
    pub summary: String,
    pub changes_max_tokens: bool,
    pub changes_history_limit: bool,
    pub max_tokens: Option<usize>,
    pub max_tokens_label: Option<String>,
    pub max_tokens_backend_default: bool,
    pub history_limit: Option<usize>,
}

impl SessionConfigUpdate {
    pub fn default_max_tokens_from_label(value: &str) -> Result<Self, String> {
        let value = value.trim();
        match value.to_ascii_lowercase().as_str() {
            "" => Err("missing max token budget".to_owned()),
            "auto" | "default" | "backend" | "none" | "off" => Ok(Self::DefaultMaxTokens(None)),
            _ => parse_positive_usize("max token budget", value)
                .map(|value| Self::DefaultMaxTokens(Some(value))),
        }
    }

    pub fn history_limit_from_label(value: &str) -> Result<Self, String> {
        let value = value.trim();
        if value.is_empty() {
            return Err("missing history limit".to_owned());
        }
        parse_positive_usize("history limit", value).map(Self::HistoryLimit)
    }

    pub fn apply_to_session(&self, session: &mut ChatSession) {
        match self {
            Self::DefaultMaxTokens(max_tokens) => session.set_default_max_tokens(*max_tokens),
            Self::HistoryLimit(history_limit) => session.set_history_limit(*history_limit),
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::DefaultMaxTokens(Some(max_tokens)) => format!("max_tokens={max_tokens}"),
            Self::DefaultMaxTokens(None) => "max_tokens=backend-default".to_owned(),
            Self::HistoryLimit(history_limit) => format!("history_limit={history_limit}"),
        }
    }

    pub fn snapshot(&self) -> SessionConfigUpdateSnapshot {
        match self {
            Self::DefaultMaxTokens(max_tokens) => SessionConfigUpdateSnapshot {
                update: self.clone(),
                kind_label: "max_tokens".to_owned(),
                summary: self.summary(),
                changes_max_tokens: true,
                changes_history_limit: false,
                max_tokens: *max_tokens,
                max_tokens_label: Some(
                    max_tokens
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "backend-default".to_owned()),
                ),
                max_tokens_backend_default: max_tokens.is_none(),
                history_limit: None,
            },
            Self::HistoryLimit(history_limit) => SessionConfigUpdateSnapshot {
                update: self.clone(),
                kind_label: "history_limit".to_owned(),
                summary: self.summary(),
                changes_max_tokens: false,
                changes_history_limit: true,
                max_tokens: None,
                max_tokens_label: None,
                max_tokens_backend_default: false,
                history_limit: Some(*history_limit),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CliInput {
    buffer: String,
    config: CliInputConfig,
}

impl CliInput {
    pub fn new(config: CliInputConfig) -> Self {
        Self {
            buffer: String::new(),
            config,
        }
    }

    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    pub fn config(&self) -> &CliInputConfig {
        &self.config
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn routing_summary(&self) -> String {
        self.config.routing_summary()
    }

    pub fn action_snapshot(&self, action: &InputAction) -> InputActionSnapshot {
        action.snapshot(&self.config)
    }

    pub fn select_model_role(&mut self, role: ModelRole) -> InputAction {
        self.config.model_role = role;
        InputAction::RoutingChanged(self.routing_summary())
    }

    pub fn select_model_role_label(&mut self, label: &str) -> InputAction {
        match ModelRole::from_label(label) {
            Some(role) => self.select_model_role(role),
            None => InputAction::InputError(format!("unknown model role: {}", label.trim())),
        }
    }

    pub fn select_routing_preference(&mut self, preference: RoutingPreference) -> InputAction {
        self.config.routing_preference = preference;
        InputAction::RoutingChanged(self.routing_summary())
    }

    pub fn select_routing_preference_label(&mut self, label: &str) -> InputAction {
        match RoutingPreference::from_label(label) {
            Some(preference) => self.select_routing_preference(preference),
            None => {
                InputAction::InputError(format!("unknown routing preference: {}", label.trim()))
            }
        }
    }

    pub fn select_model_endpoint(&mut self, endpoint: Option<ModelEndpoint>) -> InputAction {
        self.config.model_endpoint = endpoint;
        InputAction::RoutingChanged(self.routing_summary())
    }

    pub fn select_model_endpoint_label(&mut self, label: &str) -> InputAction {
        self.select_model_endpoint(ModelEndpoint::from_label(label))
    }

    pub fn select_routing_intent(&mut self, intent: RoutingIntent) -> InputAction {
        self.config.apply_routing_intent(intent);
        InputAction::RoutingChanged(self.routing_summary())
    }

    pub fn select_model_route_labels(
        &mut self,
        role_label: &str,
        preference_label: Option<&str>,
        endpoint_label: Option<&str>,
    ) -> InputAction {
        match self.config.clone().with_model_route_labels(
            role_label,
            preference_label,
            endpoint_label,
        ) {
            Ok(config) => {
                self.config = config;
                InputAction::RoutingChanged(self.routing_summary())
            }
            Err(error) => InputAction::InputError(error),
        }
    }

    pub fn apply_session_config_update(
        &mut self,
        session: &mut ChatSession,
        update: SessionConfigUpdate,
    ) -> InputAction {
        update.apply_to_session(session);
        session_config_input_action(update)
    }

    pub fn set_default_max_tokens(
        &mut self,
        session: &mut ChatSession,
        max_tokens: Option<usize>,
    ) -> InputAction {
        self.apply_session_config_update(session, SessionConfigUpdate::DefaultMaxTokens(max_tokens))
    }

    pub fn set_default_max_tokens_label(
        &mut self,
        session: &mut ChatSession,
        label: &str,
    ) -> InputAction {
        match parse_max_tokens_update(label) {
            Ok(update) => self.apply_session_config_update(session, update),
            Err(error) => InputAction::InputError(error),
        }
    }

    pub fn set_history_limit(
        &mut self,
        session: &mut ChatSession,
        history_limit: usize,
    ) -> InputAction {
        self.apply_session_config_update(session, SessionConfigUpdate::HistoryLimit(history_limit))
    }

    pub fn set_history_limit_label(
        &mut self,
        session: &mut ChatSession,
        label: &str,
    ) -> InputAction {
        match parse_history_limit_update(label) {
            Ok(update) => self.apply_session_config_update(session, update),
            Err(error) => InputAction::InputError(error),
        }
    }

    pub fn control_snapshot(
        &self,
        session: &ChatSession,
        submit_mode: InputSubmitMode,
    ) -> InputControlSnapshot {
        InputControlSnapshot::new(
            self.readiness_with_gate_decision(session, None, submit_mode),
            CliStatusSnapshot::new(&self.config, session, None),
        )
    }

    pub fn control_snapshot_with_gate(
        &self,
        session: &ChatSession,
        gate: &FrontendGateSnapshot,
        submit_mode: InputSubmitMode,
    ) -> InputControlSnapshot {
        let decision = gate.decision();
        InputControlSnapshot::new(
            self.readiness_with_gate_decision(session, Some(decision.clone()), submit_mode),
            CliStatusSnapshot::new(&self.config, session, Some(&decision)),
        )
    }

    pub fn control_snapshot_with_model_pool_gate(
        &self,
        session: &ChatSession,
        gate: &ModelPoolGateSnapshot,
        submit_mode: InputSubmitMode,
    ) -> InputControlSnapshot {
        InputControlSnapshot::new(
            self.readiness_with_gate_decision(
                session,
                Some(gate.decision_for_intent(&self.config.routing_intent())),
                submit_mode,
            ),
            CliStatusSnapshot::from_model_pool_gate(&self.config, session, gate),
        )
    }

    pub fn readiness(&self, session: &ChatSession) -> InputReadinessSnapshot {
        self.readiness_with_gate_decision(session, None, InputSubmitMode::Preview)
    }

    pub fn readiness_recording(&self, session: &ChatSession) -> InputReadinessSnapshot {
        self.readiness_with_gate_decision(session, None, InputSubmitMode::Record)
    }

    pub fn readiness_starting(&self, session: &ChatSession) -> InputReadinessSnapshot {
        self.readiness_with_gate_decision(session, None, InputSubmitMode::StartStream)
    }

    pub fn readiness_with_gate(
        &self,
        session: &ChatSession,
        gate: &FrontendGateSnapshot,
    ) -> InputReadinessSnapshot {
        self.readiness_with_gate_decision(session, Some(gate.decision()), InputSubmitMode::Preview)
    }

    pub fn readiness_with_gate_and_record(
        &self,
        session: &ChatSession,
        gate: &FrontendGateSnapshot,
    ) -> InputReadinessSnapshot {
        self.readiness_with_gate_decision(session, Some(gate.decision()), InputSubmitMode::Record)
    }

    pub fn readiness_with_gate_and_start(
        &self,
        session: &ChatSession,
        gate: &FrontendGateSnapshot,
    ) -> InputReadinessSnapshot {
        self.readiness_with_gate_decision(
            session,
            Some(gate.decision()),
            InputSubmitMode::StartStream,
        )
    }

    pub fn readiness_with_model_pool_gate(
        &self,
        session: &ChatSession,
        gate: &ModelPoolGateSnapshot,
    ) -> InputReadinessSnapshot {
        self.readiness_with_gate_decision(
            session,
            Some(gate.decision_for_intent(&self.config.routing_intent())),
            InputSubmitMode::Preview,
        )
    }

    pub fn readiness_with_model_pool_gate_and_record(
        &self,
        session: &ChatSession,
        gate: &ModelPoolGateSnapshot,
    ) -> InputReadinessSnapshot {
        self.readiness_with_gate_decision(
            session,
            Some(gate.decision_for_intent(&self.config.routing_intent())),
            InputSubmitMode::Record,
        )
    }

    pub fn readiness_with_model_pool_gate_and_start(
        &self,
        session: &ChatSession,
        gate: &ModelPoolGateSnapshot,
    ) -> InputReadinessSnapshot {
        self.readiness_with_gate_decision(
            session,
            Some(gate.decision_for_intent(&self.config.routing_intent())),
            InputSubmitMode::StartStream,
        )
    }

    pub fn handle_key(&mut self, key: KeyInput, session: &ChatSession) -> InputAction {
        self.handle_key_with_gate_decision(key, session, None)
    }

    pub fn handle_key_recording(
        &mut self,
        key: KeyInput,
        session: &mut ChatSession,
    ) -> InputAction {
        self.handle_key_with_gate_decision_and_record(key, session, None)
    }

    pub fn handle_key_canceling(
        &mut self,
        key: KeyInput,
        session: &mut ChatSession,
    ) -> InputAction {
        match key {
            KeyInput::CtrlX => self.cancel_stream(session),
            _ => self.handle_key_recording(key, session),
        }
    }

    pub fn handle_key_with_gate(
        &mut self,
        key: KeyInput,
        session: &ChatSession,
        gate: &FrontendGateSnapshot,
    ) -> InputAction {
        self.handle_key_with_gate_decision(key, session, Some(gate.decision()))
    }

    pub fn handle_key_with_gate_and_record(
        &mut self,
        key: KeyInput,
        session: &mut ChatSession,
        gate: &FrontendGateSnapshot,
    ) -> InputAction {
        self.handle_key_with_gate_decision_and_record(key, session, Some(gate.decision()))
    }

    pub fn handle_key_with_gate_and_cancel(
        &mut self,
        key: KeyInput,
        session: &mut ChatSession,
        gate: &FrontendGateSnapshot,
    ) -> InputAction {
        match key {
            KeyInput::CtrlX => self.cancel_stream(session),
            _ => self.handle_key_with_gate_and_record(key, session, gate),
        }
    }

    pub fn handle_key_with_model_pool_gate(
        &mut self,
        key: KeyInput,
        session: &ChatSession,
        gate: &ModelPoolGateSnapshot,
    ) -> InputAction {
        if let Some(action) = self.model_pool_local_status(key.clone(), session, gate) {
            return action;
        }
        let decision = gate.decision_for_intent(&self.config.routing_intent());
        self.handle_key_with_gate_decision(key, session, Some(decision))
    }

    pub fn handle_key_with_model_pool_gate_and_record(
        &mut self,
        key: KeyInput,
        session: &mut ChatSession,
        gate: &ModelPoolGateSnapshot,
    ) -> InputAction {
        if let Some(action) = self.model_pool_local_status(key.clone(), session, gate) {
            return action;
        }
        let decision = gate.decision_for_intent(&self.config.routing_intent());
        self.handle_key_with_gate_decision_and_record(key, session, Some(decision))
    }

    pub fn handle_key_with_model_pool_gate_and_cancel(
        &mut self,
        key: KeyInput,
        session: &mut ChatSession,
        gate: &ModelPoolGateSnapshot,
    ) -> InputAction {
        match key {
            KeyInput::CtrlX => self.cancel_stream(session),
            _ => self.handle_key_with_model_pool_gate_and_record(key, session, gate),
        }
    }

    pub fn handle_key_starting(&mut self, key: KeyInput, session: &mut ChatSession) -> InputAction {
        self.handle_key_with_gate_decision_and_start(key, session, None)
    }

    pub fn handle_key_with_gate_and_start(
        &mut self,
        key: KeyInput,
        session: &mut ChatSession,
        gate: &FrontendGateSnapshot,
    ) -> InputAction {
        self.handle_key_with_gate_decision_and_start(key, session, Some(gate.decision()))
    }

    pub fn handle_key_with_model_pool_gate_and_start(
        &mut self,
        key: KeyInput,
        session: &mut ChatSession,
        gate: &ModelPoolGateSnapshot,
    ) -> InputAction {
        if let Some(action) = self.model_pool_local_status(key.clone(), session, gate) {
            return action;
        }
        let decision = gate.decision_for_intent(&self.config.routing_intent());
        self.handle_key_with_gate_decision_and_start(key, session, Some(decision))
    }

    pub fn cancel_stream(&mut self, session: &mut ChatSession) -> InputAction {
        session
            .cancel_stream()
            .map(InputAction::StreamCancelled)
            .unwrap_or(InputAction::CancelStream)
    }

    fn model_pool_local_status(
        &mut self,
        key: KeyInput,
        session: &ChatSession,
        gate: &ModelPoolGateSnapshot,
    ) -> Option<InputAction> {
        if key != KeyInput::Enter {
            return None;
        }

        let prompt = self.buffer.trim();
        if is_status_command(prompt) {
            self.buffer.clear();
            return Some(InputAction::Status(cli_model_pool_status_line(
                &self.config,
                session,
                gate,
            )));
        }
        if is_worker_status_command(prompt) {
            self.buffer.clear();
            return Some(InputAction::Status(cli_model_pool_workers_line(
                &self.config,
                session,
                gate,
            )));
        }

        None
    }

    fn handle_key_with_gate_decision(
        &mut self,
        key: KeyInput,
        session: &ChatSession,
        gate: Option<GateDecision>,
    ) -> InputAction {
        match key {
            KeyInput::Char(ch) => {
                self.buffer.push(ch);
                InputAction::BufferChanged
            }
            KeyInput::Backspace => {
                self.buffer.pop();
                InputAction::BufferChanged
            }
            KeyInput::ShiftEnter if self.config.shift_enter_inserts_newline => {
                self.buffer.push('\n');
                InputAction::InsertNewline
            }
            KeyInput::Enter => self.submit(session, gate),
            KeyInput::CtrlX => InputAction::CancelStream,
            KeyInput::CtrlC => InputAction::Quit,
            KeyInput::ShiftEnter => InputAction::Noop,
        }
    }

    fn handle_key_with_gate_decision_and_start(
        &mut self,
        key: KeyInput,
        session: &mut ChatSession,
        gate: Option<GateDecision>,
    ) -> InputAction {
        match key {
            KeyInput::Char(ch) => {
                self.buffer.push(ch);
                InputAction::BufferChanged
            }
            KeyInput::Backspace => {
                self.buffer.pop();
                InputAction::BufferChanged
            }
            KeyInput::ShiftEnter if self.config.shift_enter_inserts_newline => {
                self.buffer.push('\n');
                InputAction::InsertNewline
            }
            KeyInput::Enter => self.submit_starting(session, gate),
            KeyInput::CtrlX => InputAction::CancelStream,
            KeyInput::CtrlC => InputAction::Quit,
            KeyInput::ShiftEnter => InputAction::Noop,
        }
    }

    fn handle_key_with_gate_decision_and_record(
        &mut self,
        key: KeyInput,
        session: &mut ChatSession,
        gate: Option<GateDecision>,
    ) -> InputAction {
        match key {
            KeyInput::Char(ch) => {
                self.buffer.push(ch);
                InputAction::BufferChanged
            }
            KeyInput::Backspace => {
                self.buffer.pop();
                InputAction::BufferChanged
            }
            KeyInput::ShiftEnter if self.config.shift_enter_inserts_newline => {
                self.buffer.push('\n');
                InputAction::InsertNewline
            }
            KeyInput::Enter => self.submit_recording(session, gate),
            KeyInput::CtrlX => InputAction::CancelStream,
            KeyInput::CtrlC => InputAction::Quit,
            KeyInput::ShiftEnter => InputAction::Noop,
        }
    }

    fn submit(&mut self, session: &ChatSession, gate: Option<GateDecision>) -> InputAction {
        let prompt = self.buffer.trim().to_owned();
        if prompt.is_empty() {
            return InputAction::Noop;
        }
        if is_status_command(&prompt) {
            self.buffer.clear();
            return InputAction::Status(cli_status_line(&self.config, session, gate.as_ref()));
        }
        if is_worker_status_command(&prompt) {
            self.buffer.clear();
            return InputAction::Status(cli_workers_unavailable_line(&self.config));
        }
        match self.apply_slash_command(&prompt) {
            SlashCommandResult::Accepted(action) => {
                self.buffer.clear();
                return action;
            }
            SlashCommandResult::Rejected(reason) => return InputAction::InputError(reason),
            SlashCommandResult::NotCommand => {}
        }
        if let Some(blocked) = submit_blocked_chunk(session, gate) {
            return InputAction::Blocked(blocked);
        }
        self.buffer.clear();
        InputAction::Send(
            self.config
                .apply_to_request(session.request_for_prompt(prompt)),
        )
    }

    fn submit_recording(
        &mut self,
        session: &mut ChatSession,
        gate: Option<GateDecision>,
    ) -> InputAction {
        let prompt = self.buffer.trim().to_owned();
        if prompt.is_empty() {
            return InputAction::Noop;
        }
        if is_status_command(&prompt) {
            self.buffer.clear();
            return InputAction::Status(cli_status_line(&self.config, session, gate.as_ref()));
        }
        if is_worker_status_command(&prompt) {
            self.buffer.clear();
            return InputAction::Status(cli_workers_unavailable_line(&self.config));
        }
        match self.apply_slash_command(&prompt) {
            SlashCommandResult::Accepted(action) => {
                if let InputAction::SessionConfigChanged { update, .. } = &action {
                    update.apply_to_session(session);
                }
                self.buffer.clear();
                return action;
            }
            SlashCommandResult::Rejected(reason) => return InputAction::InputError(reason),
            SlashCommandResult::NotCommand => {}
        }
        if let Some(blocked) = submit_blocked_chunk(session, gate) {
            return InputAction::Blocked(blocked);
        }
        match session.try_submit_prompt(prompt) {
            Ok(request) => {
                self.buffer.clear();
                InputAction::Send(self.config.apply_to_request(request))
            }
            Err(blocked) => InputAction::Blocked(blocked),
        }
    }

    fn submit_starting(
        &mut self,
        session: &mut ChatSession,
        gate: Option<GateDecision>,
    ) -> InputAction {
        let prompt = self.buffer.trim().to_owned();
        if prompt.is_empty() {
            return InputAction::Noop;
        }
        if is_status_command(&prompt) {
            self.buffer.clear();
            return InputAction::Status(cli_status_line(&self.config, session, gate.as_ref()));
        }
        if is_worker_status_command(&prompt) {
            self.buffer.clear();
            return InputAction::Status(cli_workers_unavailable_line(&self.config));
        }
        match self.apply_slash_command(&prompt) {
            SlashCommandResult::Accepted(action) => {
                if let InputAction::SessionConfigChanged { update, .. } = &action {
                    update.apply_to_session(session);
                }
                self.buffer.clear();
                return action;
            }
            SlashCommandResult::Rejected(reason) => return InputAction::InputError(reason),
            SlashCommandResult::NotCommand => {}
        }
        if let Some(blocked) = submit_blocked_chunk(session, gate) {
            return InputAction::Blocked(blocked);
        }
        match session.try_submit_and_begin_stream(prompt) {
            Ok(turn) => {
                self.buffer.clear();
                InputAction::StartStream(StartedChatTurn {
                    request: self.config.apply_to_request(turn.request),
                    start: turn.start,
                })
            }
            Err(blocked) => InputAction::Blocked(blocked),
        }
    }

    fn apply_slash_command(&mut self, prompt: &str) -> SlashCommandResult {
        let Some(preview) = preview_local_command(&self.config, prompt) else {
            return SlashCommandResult::NotCommand;
        };

        match preview.enter_action {
            InputActionKind::RoutingChanged => {
                let intent = preview
                    .routing_intent
                    .expect("routing preview should include intent");
                self.config.apply_routing_intent(intent);
                SlashCommandResult::Accepted(InputAction::RoutingChanged(
                    preview
                        .local_status
                        .unwrap_or_else(|| self.routing_summary()),
                ))
            }
            InputActionKind::SessionConfigChanged => {
                let update = preview
                    .session_config_update
                    .expect("session config preview should include update");
                session_config_action(update)
            }
            InputActionKind::InputError => SlashCommandResult::Rejected(
                preview
                    .error
                    .unwrap_or_else(|| "invalid slash command".to_owned()),
            ),
            InputActionKind::Status => SlashCommandResult::NotCommand,
            _ => SlashCommandResult::Rejected("invalid slash command".to_owned()),
        }
    }

    fn readiness_with_gate_decision(
        &self,
        session: &ChatSession,
        gate: Option<GateDecision>,
        submit_mode: InputSubmitMode,
    ) -> InputReadinessSnapshot {
        let prompt = self.buffer.trim();
        let routing_intent = self.config.routing_intent();
        let endpoint_kind = routing_intent.endpoint_kind();
        let route_wire = routing_intent.wire_snapshot();
        let command_preview = preview_local_command(&self.config, prompt);
        let buffer_kind = command_preview
            .as_ref()
            .map(|preview| preview.buffer_kind)
            .unwrap_or_else(|| classify_buffer(prompt));
        let request_preview = (buffer_kind == InputBufferKind::Prompt).then(|| {
            let request = self
                .config
                .apply_to_request(session.request_for_prompt(prompt));
            InputRequestSnapshot::from_request_with_history_limit(
                &request,
                Some(session.config().history_limit),
            )
        });
        let blocked = if buffer_kind == InputBufferKind::Prompt {
            submit_blocked_chunk(session, gate)
        } else {
            None
        };
        let enter_action = if let Some(command_preview) = command_preview.as_ref() {
            command_preview.enter_action
        } else {
            match buffer_kind {
                InputBufferKind::Empty => InputActionKind::Noop,
                InputBufferKind::Prompt => {
                    if blocked.is_some() {
                        InputActionKind::Blocked
                    } else {
                        submit_mode.ready_prompt_action()
                    }
                }
                InputBufferKind::StatusCommand
                | InputBufferKind::WorkerStatusCommand
                | InputBufferKind::RoutingCommand
                | InputBufferKind::SessionConfigCommand
                | InputBufferKind::InvalidCommand => unreachable!("local commands have previews"),
            }
        };
        let enter_action = if buffer_kind == InputBufferKind::Prompt {
            if blocked.is_some() {
                InputActionKind::Blocked
            } else {
                enter_action
            }
        } else {
            enter_action
        };
        let block_chunk = blocked.as_ref().map(ChatChunk::display_snapshot);
        let is_blocked = block_chunk.is_some();
        let (block_state, block_reason, blocked_advice_action) = blocked
            .as_ref()
            .map(|chunk| {
                let advice_action = GateDecision::blocked(chunk.state, chunk.content.clone())
                    .advice()
                    .action;
                (
                    Some(chunk.state),
                    (!chunk.content.trim().is_empty()).then(|| chunk.content.clone()),
                    Some(advice_action),
                )
            })
            .unwrap_or((None, None, None));
        let advice_action = blocked_advice_action.or_else(|| {
            (buffer_kind == InputBufferKind::Prompt
                && matches!(
                    enter_action,
                    InputActionKind::Send | InputActionKind::StartStream
                ))
            .then_some(GateAdviceAction::SendNow)
        });
        let advice_action_label = advice_action.map(|action| action.as_str().to_owned());
        let block_state_label = block_state.map(|state| state.as_str().to_owned());
        let block_state_is_terminal = block_state.is_some_and(StreamState::is_terminal);
        let block_state_is_pressure = block_state.is_some_and(StreamState::is_pressure);
        let block_state_blocks_prompt_submit =
            block_state.is_some_and(StreamState::blocks_prompt_submit);
        let enter_enabled = !prompt.is_empty();
        let prompt_submit_control = match buffer_kind {
            InputBufferKind::Empty => Some(GateDecision::Allowed.send_control(false)),
            InputBufferKind::Prompt => Some(
                block_state
                    .map(|state| {
                        GateDecision::blocked(
                            state,
                            block_reason
                                .clone()
                                .unwrap_or_else(|| "prompt submit is blocked".to_owned()),
                        )
                    })
                    .unwrap_or(GateDecision::Allowed)
                    .send_control(true),
            ),
            InputBufferKind::StatusCommand
            | InputBufferKind::WorkerStatusCommand
            | InputBufferKind::RoutingCommand
            | InputBufferKind::SessionConfigCommand
            | InputBufferKind::InvalidCommand => None,
        };
        let enter_submits_prompt = buffer_kind == InputBufferKind::Prompt
            && !is_blocked
            && matches!(
                enter_action,
                InputActionKind::Send | InputActionKind::StartStream
            );
        let enter_runs_local_command = matches!(
            enter_action,
            InputActionKind::RoutingChanged
                | InputActionKind::Status
                | InputActionKind::SessionConfigChanged
        );
        let enter_is_blocked = enter_action == InputActionKind::Blocked;
        let primary_action_label =
            primary_action_label(enter_action, buffer_kind, advice_action).to_owned();
        let primary_action_enabled = enter_enabled
            && !matches!(
                enter_action,
                InputActionKind::Blocked | InputActionKind::InputError | InputActionKind::Noop
            );
        let primary_action_disabled_reason = primary_action_disabled_reason(
            enter_enabled,
            enter_action,
            block_reason.as_ref(),
            command_preview.as_ref(),
        );
        let clears_buffer_on_enter = enter_enabled && enter_clears_buffer(enter_action);
        let preserves_buffer_on_enter = enter_enabled && !clears_buffer_on_enter;

        InputReadinessSnapshot {
            model_role_label: routing_intent.model_role_label().to_owned(),
            routing_preference_label: routing_intent.routing_preference_label().to_owned(),
            endpoint_label: routing_intent.endpoint_label().to_owned(),
            endpoint_pinned: routing_intent.endpoint_pinned,
            endpoint_kind,
            endpoint_kind_label: routing_intent.endpoint_kind_label().to_owned(),
            endpoint_auto: routing_intent.endpoint_auto(),
            endpoint_built_in: routing_intent.endpoint_built_in(),
            endpoint_custom: routing_intent.endpoint_custom(),
            wire_model_role_label: route_wire.model_role_label,
            wire_routing_preference_label: route_wire.routing_preference_label,
            wire_prefer_fast: route_wire.prefer_fast,
            wire_prefer_quality: route_wire.prefer_quality,
            wire_endpoint_pinned: route_wire.endpoint_pinned,
            wire_endpoint_kind_label: route_wire.endpoint_kind_label,
            wire_sends_model_endpoint: route_wire.sends_model_endpoint,
            wire_model_endpoint_label: route_wire.model_endpoint_label,
            routing_intent,
            submit_mode,
            submit_mode_label: submit_mode.as_str().to_owned(),
            buffer_chars: self.buffer.chars().count(),
            trimmed_chars: prompt.chars().count(),
            line_count: self.buffer.lines().count().max(1),
            buffer_kind,
            buffer_kind_label: buffer_kind.as_str().to_owned(),
            command_preview,
            request_preview,
            prompt_submit_control,
            enter_action,
            enter_action_label: enter_action.as_str().to_owned(),
            enter_enabled,
            enter_submits_prompt,
            enter_runs_local_command,
            enter_is_blocked,
            primary_action_label,
            primary_action_enabled,
            primary_action_disabled_reason,
            preserves_buffer_on_enter,
            clears_buffer_on_enter,
            send_allowed: matches!(
                enter_action,
                InputActionKind::Send | InputActionKind::StartStream
            ),
            records_user_on_enter: buffer_kind == InputBufferKind::Prompt
                && !is_blocked
                && submit_mode.records_user_on_enter(),
            starts_stream_on_enter: buffer_kind == InputBufferKind::Prompt
                && !is_blocked
                && submit_mode.starts_stream_on_enter(),
            advice_action,
            advice_action_label,
            block_state,
            block_state_label,
            block_state_is_terminal,
            block_state_is_pressure,
            block_state_blocks_prompt_submit,
            block_chunk,
            block_reason,
        }
    }
}

impl Default for CliInput {
    fn default() -> Self {
        Self::new(CliInputConfig::default())
    }
}

enum SlashCommandResult {
    Accepted(InputAction),
    Rejected(String),
    NotCommand,
}

fn submit_blocked_chunk(session: &ChatSession, gate: Option<GateDecision>) -> Option<ChatChunk> {
    match gate {
        Some(
            decision @ GateDecision::Blocked {
                state: norion_service::StreamState::Failed,
                ..
            },
        ) => decision.to_chunk(0),
        Some(decision) => session
            .prompt_blocked_chunk()
            .or_else(|| decision.to_chunk(0)),
        None => session.prompt_blocked_chunk(),
    }
}

fn block_display_snapshot(
    state: Option<StreamState>,
    reason: Option<&str>,
) -> Option<ChatChunkDisplaySnapshot> {
    let state = state?;
    let reason = reason.unwrap_or_default();
    let chunk = match state {
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
    };
    Some(chunk.display_snapshot())
}

fn primary_action_label(
    enter_action: InputActionKind,
    buffer_kind: InputBufferKind,
    advice_action: Option<GateAdviceAction>,
) -> &'static str {
    match enter_action {
        InputActionKind::Send | InputActionKind::StartStream => "send",
        InputActionKind::RoutingChanged => "apply_route",
        InputActionKind::Status if buffer_kind == InputBufferKind::WorkerStatusCommand => {
            "show_workers"
        }
        InputActionKind::Status => "show_status",
        InputActionKind::SessionConfigChanged => "apply_config",
        InputActionKind::Blocked => advice_action
            .map(GateAdviceAction::as_str)
            .unwrap_or("wait"),
        InputActionKind::InputError => "fix_command",
        InputActionKind::InsertNewline => "insert_newline",
        InputActionKind::CancelStream => "cancel_stream",
        InputActionKind::Quit => "quit",
        InputActionKind::BufferChanged => "edit",
        InputActionKind::StreamCancelled => "stream_cancelled",
        InputActionKind::Noop => "type_prompt",
    }
}

fn primary_action_disabled_reason(
    enter_enabled: bool,
    enter_action: InputActionKind,
    block_reason: Option<&String>,
    command_preview: Option<&InputCommandPreview>,
) -> Option<String> {
    if !enter_enabled {
        return Some("empty input".to_owned());
    }

    match enter_action {
        InputActionKind::Blocked => Some(
            block_reason
                .cloned()
                .unwrap_or_else(|| "prompt submit is blocked".to_owned()),
        ),
        InputActionKind::InputError => Some(
            command_preview
                .and_then(|preview| preview.error.clone())
                .unwrap_or_else(|| "invalid command".to_owned()),
        ),
        _ => None,
    }
}

fn session_config_input_action(update: SessionConfigUpdate) -> InputAction {
    let summary = update.summary();
    InputAction::SessionConfigChanged { update, summary }
}

fn session_config_action(update: SessionConfigUpdate) -> SlashCommandResult {
    SlashCommandResult::Accepted(session_config_input_action(update))
}

fn parse_max_tokens_update(value: &str) -> Result<SessionConfigUpdate, String> {
    SessionConfigUpdate::default_max_tokens_from_label(value)
}

fn parse_history_limit_update(value: &str) -> Result<SessionConfigUpdate, String> {
    SessionConfigUpdate::history_limit_from_label(value)
}

fn parse_positive_usize(name: &str, value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map(|value| value.max(1))
        .map_err(|_| format!("{name} must be a positive integer"))
}

fn preview_local_command(config: &CliInputConfig, prompt: &str) -> Option<InputCommandPreview> {
    if prompt.is_empty() || !prompt.starts_with('/') {
        return None;
    }
    if is_status_command(prompt) {
        return Some(command_preview(
            InputBufferKind::StatusCommand,
            InputActionKind::Status,
            None,
            None,
            None,
        ));
    }
    if is_worker_status_command(prompt) {
        return Some(command_preview(
            InputBufferKind::WorkerStatusCommand,
            InputActionKind::Status,
            None,
            None,
            None,
        ));
    }

    let command = prompt
        .strip_prefix('/')
        .expect("slash command preview only runs for slash-prefixed input");
    let (name, rest) = command
        .split_once(char::is_whitespace)
        .map(|(name, rest)| (name, rest.trim()))
        .unwrap_or((command, ""));
    Some(match name {
        "role" => match ModelRole::from_label(rest) {
            Some(role) => routing_command_preview(config.clone().with_model_role(role)),
            None => input_error_preview(format!("unknown model role: {rest}")),
        },
        "prefer" | "preference" => match RoutingPreference::from_label(rest) {
            Some(preference) => {
                routing_command_preview(config.clone().with_routing_preference(preference))
            }
            None => input_error_preview(format!("unknown routing preference: {rest}")),
        },
        "endpoint" | "worker" => {
            if rest.is_empty() {
                input_error_preview("missing model endpoint".to_owned())
            } else {
                routing_command_preview(
                    config
                        .clone()
                        .with_model_endpoint(ModelEndpoint::from_label(rest)),
                )
            }
        }
        "max-tokens" | "tokens" => match parse_max_tokens_update(rest) {
            Ok(update) => session_config_preview(update),
            Err(error) => input_error_preview(error),
        },
        "history-limit" | "history" => match parse_history_limit_update(rest) {
            Ok(update) => session_config_preview(update),
            Err(error) => input_error_preview(error),
        },
        "model" => preview_model_command(config, rest),
        _ => input_error_preview(format!("unknown slash command: /{name}")),
    })
}

fn preview_model_command(config: &CliInputConfig, rest: &str) -> InputCommandPreview {
    let mut parts = rest.split_whitespace();
    let Some(raw_role) = parts.next() else {
        return input_error_preview("missing model role".to_owned());
    };
    let raw_preference = parts.next();
    let raw_endpoint = parts.next();
    if let Some(extra) = parts.next() {
        return input_error_preview(format!("unexpected model command argument: {extra}"));
    }

    match config
        .clone()
        .with_model_route_labels(raw_role, raw_preference, raw_endpoint)
    {
        Ok(preview) => routing_command_preview(preview),
        Err(error) => input_error_preview(error),
    }
}

fn routing_command_preview(config: CliInputConfig) -> InputCommandPreview {
    let intent = config.routing_intent();
    command_preview(
        InputBufferKind::RoutingCommand,
        InputActionKind::RoutingChanged,
        Some(intent.clone()),
        None,
        Some(intent.summary()),
    )
}

fn session_config_preview(update: SessionConfigUpdate) -> InputCommandPreview {
    command_preview(
        InputBufferKind::SessionConfigCommand,
        InputActionKind::SessionConfigChanged,
        None,
        Some(update.clone()),
        Some(update.summary()),
    )
}

fn input_error_preview(error: String) -> InputCommandPreview {
    InputCommandPreview {
        buffer_kind: InputBufferKind::InvalidCommand,
        buffer_kind_label: InputBufferKind::InvalidCommand.as_str().to_owned(),
        enter_action: InputActionKind::InputError,
        enter_action_label: InputActionKind::InputError.as_str().to_owned(),
        routing_intent: None,
        routing_summary: None,
        model_role_label: None,
        routing_preference_label: None,
        endpoint_label: None,
        endpoint_pinned: None,
        endpoint_kind: None,
        endpoint_kind_label: None,
        endpoint_auto: None,
        endpoint_built_in: None,
        endpoint_custom: None,
        wire_model_role_label: None,
        wire_routing_preference_label: None,
        wire_prefer_fast: None,
        wire_prefer_quality: None,
        wire_endpoint_pinned: None,
        wire_endpoint_kind_label: None,
        wire_sends_model_endpoint: None,
        wire_model_endpoint_label: None,
        session_config_update: None,
        session_config_update_detail: None,
        local_status: None,
        error: Some(error),
    }
}

fn command_preview(
    buffer_kind: InputBufferKind,
    enter_action: InputActionKind,
    routing_intent: Option<RoutingIntent>,
    session_config_update: Option<SessionConfigUpdate>,
    local_status: Option<String>,
) -> InputCommandPreview {
    let endpoint_kind = routing_intent.as_ref().map(RoutingIntent::endpoint_kind);
    let route_wire = routing_intent.as_ref().map(RoutingIntent::wire_snapshot);

    InputCommandPreview {
        buffer_kind,
        buffer_kind_label: buffer_kind.as_str().to_owned(),
        enter_action,
        enter_action_label: enter_action.as_str().to_owned(),
        routing_summary: routing_intent.as_ref().map(RoutingIntent::summary),
        model_role_label: routing_intent
            .as_ref()
            .map(|intent| intent.model_role_label().to_owned()),
        routing_preference_label: routing_intent
            .as_ref()
            .map(|intent| intent.routing_preference_label().to_owned()),
        endpoint_label: routing_intent
            .as_ref()
            .map(|intent| intent.endpoint_label().to_owned()),
        endpoint_pinned: routing_intent.as_ref().map(|intent| intent.endpoint_pinned),
        endpoint_kind,
        endpoint_kind_label: routing_intent
            .as_ref()
            .map(|intent| intent.endpoint_kind_label().to_owned()),
        endpoint_auto: routing_intent.as_ref().map(RoutingIntent::endpoint_auto),
        endpoint_built_in: routing_intent
            .as_ref()
            .map(RoutingIntent::endpoint_built_in),
        endpoint_custom: routing_intent.as_ref().map(RoutingIntent::endpoint_custom),
        wire_model_role_label: route_wire
            .as_ref()
            .map(|wire| wire.model_role_label.clone()),
        wire_routing_preference_label: route_wire
            .as_ref()
            .map(|wire| wire.routing_preference_label.clone()),
        wire_prefer_fast: route_wire.as_ref().map(|wire| wire.prefer_fast),
        wire_prefer_quality: route_wire.as_ref().map(|wire| wire.prefer_quality),
        wire_endpoint_pinned: route_wire.as_ref().map(|wire| wire.endpoint_pinned),
        wire_endpoint_kind_label: route_wire
            .as_ref()
            .map(|wire| wire.endpoint_kind_label.clone()),
        wire_sends_model_endpoint: route_wire.as_ref().map(|wire| wire.sends_model_endpoint),
        wire_model_endpoint_label: route_wire
            .as_ref()
            .and_then(|wire| wire.model_endpoint_label.clone()),
        routing_intent,
        session_config_update_detail: session_config_update
            .as_ref()
            .map(SessionConfigUpdate::snapshot),
        session_config_update,
        local_status,
        error: None,
    }
}

fn classify_buffer(prompt: &str) -> InputBufferKind {
    if prompt.is_empty() {
        return InputBufferKind::Empty;
    }
    if is_status_command(prompt) {
        return InputBufferKind::StatusCommand;
    }
    if is_worker_status_command(prompt) {
        return InputBufferKind::WorkerStatusCommand;
    }
    let Some(command) = prompt.strip_prefix('/') else {
        return InputBufferKind::Prompt;
    };
    let (name, rest) = command
        .split_once(char::is_whitespace)
        .map(|(name, rest)| (name, rest.trim()))
        .unwrap_or((command, ""));
    match name {
        "role" => ModelRole::from_label(rest)
            .map(|_| InputBufferKind::RoutingCommand)
            .unwrap_or(InputBufferKind::InvalidCommand),
        "prefer" | "preference" => RoutingPreference::from_label(rest)
            .map(|_| InputBufferKind::RoutingCommand)
            .unwrap_or(InputBufferKind::InvalidCommand),
        "endpoint" | "worker" => {
            if rest.is_empty() {
                InputBufferKind::InvalidCommand
            } else {
                InputBufferKind::RoutingCommand
            }
        }
        "model" => classify_model_command(rest),
        "max-tokens" | "tokens" => parse_max_tokens_update(rest)
            .map(|_| InputBufferKind::SessionConfigCommand)
            .unwrap_or(InputBufferKind::InvalidCommand),
        "history-limit" | "history" => parse_history_limit_update(rest)
            .map(|_| InputBufferKind::SessionConfigCommand)
            .unwrap_or(InputBufferKind::InvalidCommand),
        _ => InputBufferKind::InvalidCommand,
    }
}

fn classify_model_command(rest: &str) -> InputBufferKind {
    let mut parts = rest.split_whitespace();
    let Some(raw_role) = parts.next() else {
        return InputBufferKind::InvalidCommand;
    };
    if ModelRole::from_label(raw_role).is_none() {
        return InputBufferKind::InvalidCommand;
    }
    if let Some(raw_preference) = parts.next() {
        if RoutingPreference::from_label(raw_preference).is_none() {
            return InputBufferKind::InvalidCommand;
        }
    }
    let _endpoint_update = parts.next().map(ModelEndpoint::from_label);
    if parts.next().is_some() {
        return InputBufferKind::InvalidCommand;
    }
    InputBufferKind::RoutingCommand
}

fn is_status_command(prompt: &str) -> bool {
    let Some(command) = prompt.strip_prefix('/') else {
        return false;
    };
    let (name, rest) = command
        .split_once(char::is_whitespace)
        .map(|(name, rest)| (name, rest.trim()))
        .unwrap_or((command, ""));
    matches!(name, "status" | "state") && rest.is_empty()
}

fn is_worker_status_command(prompt: &str) -> bool {
    let Some(command) = prompt.strip_prefix('/') else {
        return false;
    };
    let (name, rest) = command
        .split_once(char::is_whitespace)
        .map(|(name, rest)| (name, rest.trim()))
        .unwrap_or((command, ""));
    matches!(name, "workers" | "worker-status" | "endpoints") && rest.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use norion_service::{
        ChatSession, ChatSessionConfig, ModelRouteWorkerPickerAction, ModelWorkerSnapshot,
        StreamState,
    };

    #[test]
    fn enter_sends_prompt_and_clears_buffer() {
        let session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );
        let mut input = CliInput::default();
        input.handle_key(KeyInput::Char('h'), &session);
        input.handle_key(KeyInput::Char('i'), &session);

        let action = input.handle_key(KeyInput::Enter, &session);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
        assert_eq!(input.buffer(), "");
        assert_eq!(request.messages.last().unwrap().content, "hi");
        assert_eq!(request.max_tokens, Some(4096));
    }

    #[test]
    fn enter_preview_allows_send_after_interrupted_or_failed_session() {
        let mut interrupted = ChatSession::new("cli", ChatSessionConfig::default());
        interrupted
            .try_submit_and_begin_stream("hello")
            .expect("expected first stream");
        interrupted.push_delta("partial");
        interrupted.interrupt("backend stream closed");
        let mut input = CliInput::default();
        for ch in "next".chars() {
            input.handle_key(KeyInput::Char(ch), &interrupted);
        }

        let action = input.handle_key(KeyInput::Enter, &interrupted);

        let InputAction::Send(request) = action else {
            panic!("expected send action after interrupted session");
        };
        let request_contents = request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(request_contents, vec!["hello", "next"]);
        assert_eq!(interrupted.state(), StreamState::Interrupted);
        assert_eq!(interrupted.partial_answer(), "partial");
        assert_eq!(interrupted.last_error(), Some("backend stream closed"));
        assert!(input.buffer().is_empty());

        let mut failed = ChatSession::new("cli", ChatSessionConfig::default());
        failed.begin_stream();
        failed.fail("safe-device gate failed");
        let mut retry_input = CliInput::default();
        for ch in "repair and retry".chars() {
            retry_input.handle_key(KeyInput::Char(ch), &failed);
        }

        let action = retry_input.handle_key(KeyInput::Enter, &failed);

        let InputAction::Send(request) = action else {
            panic!("expected send action after failed session");
        };
        assert_eq!(request.messages.last().unwrap().content, "repair and retry");
        assert_eq!(failed.state(), StreamState::Failed);
        assert_eq!(failed.last_error(), Some("safe-device gate failed"));
        assert!(retry_input.buffer().is_empty());
    }

    #[test]
    fn shift_enter_is_reserved_for_newline() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        input.handle_key(KeyInput::Char('h'), &session);

        let action = input.handle_key(KeyInput::ShiftEnter, &session);

        assert_eq!(action, InputAction::InsertNewline);
        assert_eq!(input.buffer(), "h\n");
    }

    #[test]
    fn configured_routing_is_applied_when_enter_sends_prompt() {
        let session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(2048)),
        );
        let config = CliInputConfig::default()
            .prefer_fast()
            .with_model_role(ModelRole::Reviewer)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let mut input = CliInput::new(config);
        input.handle_key(KeyInput::Char('r'), &session);
        input.handle_key(KeyInput::Char('e'), &session);
        input.handle_key(KeyInput::Char('v'), &session);

        let action = input.handle_key(KeyInput::Enter, &session);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
        assert_eq!(request.max_tokens, Some(2048));
        assert_eq!(request.routing_preference, RoutingPreference::PreferFast);
        assert_eq!(request.model_role, ModelRole::Reviewer);
        assert_eq!(request.model_endpoint, Some(ModelEndpoint::FastReviewer));
    }

    #[test]
    fn input_config_exposes_routing_intent_for_status_bars() {
        let config = CliInputConfig::default()
            .prefer_fast()
            .with_model_role(ModelRole::Reviewer);

        let intent = config.routing_intent();

        assert_eq!(intent.model_role, ModelRole::Reviewer);
        assert_eq!(intent.routing_preference, RoutingPreference::PreferFast);
        assert_eq!(intent.endpoint_label(), "auto");
        assert!(!intent.endpoint_pinned);
        assert_eq!(
            config.routing_summary(),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
    }

    #[test]
    fn input_config_parses_model_route_labels_without_mutating_original_config() {
        let config = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));

        let preview = config
            .clone()
            .with_model_route_labels("tester", Some("quality"), Some("auto"))
            .expect("expected parsed route labels");
        let invalid =
            config
                .clone()
                .with_model_route_labels("summarizer", Some("cheap"), Some("auto"));

        assert_eq!(preview.model_role, ModelRole::Tester);
        assert_eq!(preview.routing_preference, RoutingPreference::PreferQuality);
        assert_eq!(preview.model_endpoint, None);
        assert_eq!(
            preview.routing_summary(),
            "role=tester preference=prefer_quality endpoint=auto pinned=false"
        );
        assert_eq!(invalid, Err("unknown routing preference: cheap".to_owned()));
        assert_eq!(config.model_role, ModelRole::Reviewer);
        assert_eq!(config.routing_preference, RoutingPreference::PreferFast);
        assert_eq!(config.model_endpoint, Some(ModelEndpoint::FastReviewer));
    }

    #[test]
    fn route_selector_methods_update_config_without_sending_or_clearing_buffer() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );
        session.record_user("first");
        session.record_assistant("answer");
        let mut input = CliInput::default();
        for ch in "review this patch".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let role = input.select_model_role(ModelRole::Reviewer);
        let preference = input.select_routing_preference(RoutingPreference::PreferFast);
        let endpoint = input.select_model_endpoint_label("fast-reviewer");

        assert_eq!(
            input.action_snapshot(&role).local_status.as_deref(),
            Some("role=reviewer preference=balanced endpoint=auto pinned=false")
        );
        assert_eq!(
            input.action_snapshot(&preference).local_status.as_deref(),
            Some("role=reviewer preference=prefer_fast endpoint=auto pinned=false")
        );
        assert_eq!(
            input.action_snapshot(&endpoint).local_status.as_deref(),
            Some("role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true")
        );
        assert_eq!(input.buffer(), "review this patch");
        assert_eq!(session.history().len(), 2);

        let action = input.handle_key_recording(KeyInput::Enter, &mut session);
        let InputAction::Send(request) = action else {
            panic!("expected send action after selector updates");
        };

        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.max_tokens, Some(4096));
        assert_eq!(request.model_role, ModelRole::Reviewer);
        assert_eq!(request.routing_preference, RoutingPreference::PreferFast);
        assert_eq!(
            request.model_endpoint.as_ref().map(ModelEndpoint::label),
            Some("fast-reviewer")
        );
        assert!(request.endpoint_pinned());
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn route_label_selectors_update_route_without_sending_or_pinning_auto_route() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(8192)),
        );
        session.record_user("first");
        let mut input = CliInput::default();
        for ch in "review this patch".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let role = input.select_model_role_label("review");
        let preference = input.select_routing_preference_label("quality");

        assert_eq!(
            input.action_snapshot(&role).local_status.as_deref(),
            Some("role=reviewer preference=balanced endpoint=auto pinned=false")
        );
        assert_eq!(
            input.action_snapshot(&preference).local_status.as_deref(),
            Some("role=reviewer preference=prefer_quality endpoint=auto pinned=false")
        );
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferQuality
        );
        assert_eq!(input.config().model_endpoint, None);
        assert_eq!(input.buffer(), "review this patch");
        assert_eq!(session.history().len(), 1);

        let action = input.handle_key_recording(KeyInput::Enter, &mut session);
        let InputAction::Send(request) = action else {
            panic!("expected send action after route label selectors");
        };
        assert_eq!(request.model_role, ModelRole::Reviewer);
        assert_eq!(request.routing_preference, RoutingPreference::PreferQuality);
        assert_eq!(request.model_endpoint, None);
        assert!(!request.endpoint_pinned());
        assert_eq!(request.max_tokens, Some(8192));
    }

    #[test]
    fn invalid_route_label_selector_is_local_and_does_not_mutate_prompt_or_session() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session.record_user("first");
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "still editing".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let role = input.select_model_role_label("architect");
        let preference = input.select_routing_preference_label("cheap");

        assert_eq!(
            role,
            InputAction::InputError("unknown model role: architect".to_owned())
        );
        assert_eq!(
            preference,
            InputAction::InputError("unknown routing preference: cheap".to_owned())
        );
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(input.buffer(), "still editing");
        assert_eq!(session.history().len(), 1);
    }

    #[test]
    fn combined_route_label_selector_preserves_or_clears_endpoint_pin_explicitly() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::new(
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer)),
        );
        for ch in "next turn".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let preserve_pin = input.select_model_route_labels("tester", Some("fast"), None);
        let clear_pin = input.select_model_route_labels("reviewer", Some("quality"), Some("auto"));

        assert_eq!(
            input.action_snapshot(&preserve_pin).local_status.as_deref(),
            Some("role=tester preference=prefer_fast endpoint=fast-reviewer pinned=true")
        );
        assert_eq!(
            input.action_snapshot(&clear_pin).local_status.as_deref(),
            Some("role=reviewer preference=prefer_quality endpoint=auto pinned=false")
        );
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferQuality
        );
        assert_eq!(input.config().model_endpoint, None);
        assert_eq!(input.buffer(), "next turn");

        let action = input.handle_key_recording(KeyInput::Enter, &mut session);
        let InputAction::Send(request) = action else {
            panic!("expected send after combined route label selector");
        };
        assert_eq!(request.model_role, ModelRole::Reviewer);
        assert_eq!(request.routing_preference, RoutingPreference::PreferQuality);
        assert_eq!(request.routing_intent().endpoint_label(), "auto");
        assert!(!request.endpoint_pinned());
    }

    #[test]
    fn combined_route_label_selector_is_atomic_on_invalid_labels() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast)
                .with_model_endpoint(Some(ModelEndpoint::FastReviewer)),
        );
        for ch in "still editing".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let invalid_role = input.select_model_route_labels("architect", Some("quality"), None);
        let invalid_preference =
            input.select_model_route_labels("summarizer", Some("cheap"), Some("auto"));

        assert_eq!(
            invalid_role,
            InputAction::InputError("unknown model role: architect".to_owned())
        );
        assert_eq!(
            invalid_preference,
            InputAction::InputError("unknown routing preference: cheap".to_owned())
        );
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(
            input.config().model_endpoint,
            Some(ModelEndpoint::FastReviewer)
        );
        assert_eq!(input.buffer(), "still editing");
    }

    #[test]
    fn session_config_selector_updates_tokens_without_sending_or_clearing_buffer() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::new(8));
        session.record_user("first");
        session.record_assistant("answer");
        let mut input = CliInput::default();
        for ch in "continue with detail".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let action = input.set_default_max_tokens(&mut session, Some(8192));
        let snapshot = input.action_snapshot(&action);

        assert_eq!(
            action,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::DefaultMaxTokens(Some(8192)),
                summary: "max_tokens=8192".to_owned()
            }
        );
        assert_eq!(
            snapshot.session_config_update,
            Some(SessionConfigUpdate::DefaultMaxTokens(Some(8192)))
        );
        assert_eq!(snapshot.local_status.as_deref(), Some("max_tokens=8192"));
        assert_eq!(input.buffer(), "continue with detail");
        assert_eq!(session.history().len(), 2);

        let readiness = input.readiness_recording(&session);
        assert_eq!(
            readiness
                .request_preview
                .as_ref()
                .map(|request| request.max_tokens),
            Some(Some(8192))
        );

        let send = input.handle_key_recording(KeyInput::Enter, &mut session);
        let InputAction::Send(request) = send else {
            panic!("expected send after direct max_tokens update");
        };
        assert_eq!(request.max_tokens, Some(8192));
        assert_eq!(request.messages.len(), 3);
        assert_eq!(session.history().len(), 3);
        assert!(input.buffer().is_empty());

        let clear = input.set_default_max_tokens(&mut session, None);
        assert_eq!(
            clear,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::DefaultMaxTokens(None),
                summary: "max_tokens=backend-default".to_owned()
            }
        );
        assert_eq!(
            session.request_for_prompt("backend decides").max_tokens,
            None
        );
    }

    #[test]
    fn session_config_label_selectors_share_cli_parsing_without_sending_prompt() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );
        session.record_user("first");
        let mut input = CliInput::default();
        for ch in "still editing".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let off = input.set_default_max_tokens_label(&mut session, "off");
        let limit = input.set_history_limit_label(&mut session, "1");
        let invalid = input.set_default_max_tokens_label(&mut session, "many");

        assert_eq!(
            off,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::DefaultMaxTokens(None),
                summary: "max_tokens=backend-default".to_owned()
            }
        );
        assert_eq!(
            limit,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::HistoryLimit(1),
                summary: "history_limit=1".to_owned()
            }
        );
        assert_eq!(
            invalid,
            InputAction::InputError("max token budget must be a positive integer".to_owned())
        );
        assert_eq!(input.buffer(), "still editing");
        assert_eq!(session.config().default_max_tokens, None);
        assert_eq!(session.config().history_limit, 1);
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.request_for_prompt("next").max_tokens, None);
    }

    #[test]
    fn session_config_update_parses_ui_labels_without_mutating_session() {
        let session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );

        assert_eq!(
            SessionConfigUpdate::default_max_tokens_from_label("off"),
            Ok(SessionConfigUpdate::DefaultMaxTokens(None))
        );
        assert_eq!(
            SessionConfigUpdate::default_max_tokens_from_label("0"),
            Ok(SessionConfigUpdate::DefaultMaxTokens(Some(1)))
        );
        assert_eq!(
            SessionConfigUpdate::history_limit_from_label("32"),
            Ok(SessionConfigUpdate::HistoryLimit(32))
        );
        assert_eq!(
            SessionConfigUpdate::history_limit_from_label("many"),
            Err("history limit must be a positive integer".to_owned())
        );
        assert_eq!(session.config().default_max_tokens, Some(4096));
        assert_eq!(session.config().history_limit, 64);
        assert!(session.history().is_empty());
    }

    #[test]
    fn session_config_selector_truncates_history_without_sending_or_clearing_buffer() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::new(8));
        session.record_user("one");
        session.record_assistant("two");
        session.record_user("three");
        session.record_assistant("four");
        let mut input = CliInput::default();
        for ch in "next turn".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let action = input.set_history_limit(&mut session, 2);

        assert_eq!(
            action,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::HistoryLimit(2),
                summary: "history_limit=2".to_owned()
            }
        );
        assert_eq!(input.buffer(), "next turn");
        let contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(contents, vec!["three", "four"]);

        let readiness = input.readiness_recording(&session);
        let preview = readiness
            .request_preview
            .as_ref()
            .expect("prompt should preview after history update");
        assert_eq!(preview.context_messages, 2);
        assert_eq!(preview.history_messages, 2);
        assert_eq!(preview.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(preview.context_kind_label, "multi_turn");
        assert_eq!(preview.messages, 3);

        let send = input.handle_key_recording(KeyInput::Enter, &mut session);
        let InputAction::Send(request) = send else {
            panic!("expected send after direct history update");
        };
        let request_contents = request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(request_contents, vec!["three", "four", "next turn"]);
        assert_eq!(session.config().history_limit, 2);
        assert_eq!(session.history().len(), 2);
    }

    #[test]
    fn route_selector_intent_preserves_auto_boundary_despite_endpoint_hint() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::new(
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer)),
        );
        for ch in "deep answer".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.select_routing_intent(RoutingIntent {
            model_role: ModelRole::Assistant,
            routing_preference: RoutingPreference::PreferQuality,
            model_endpoint: Some(ModelEndpoint::FastReviewer),
            endpoint_pinned: false,
        });

        assert_eq!(
            input.action_snapshot(&action).local_status.as_deref(),
            Some("role=assistant preference=prefer_quality endpoint=auto pinned=false")
        );
        assert_eq!(input.config().model_endpoint, None);
        assert_eq!(input.buffer(), "deep answer");

        let request = input
            .readiness(&session)
            .request_preview
            .expect("prompt readiness should expose request metadata");
        assert_eq!(request.routing_intent.endpoint_label(), "auto");
        assert!(!request.routing_intent.endpoint_pinned);
    }

    #[test]
    fn default_enter_does_not_pin_worker_endpoint() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "hello".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
        assert_eq!(request.model_role, ModelRole::Assistant);
        assert_eq!(request.routing_preference, RoutingPreference::Balanced);
        assert_eq!(request.model_endpoint, None);
    }

    #[test]
    fn slash_commands_update_routing_without_sending_prompt() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/role reviewer".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::RoutingChanged(
                "role=reviewer preference=balanced endpoint=auto pinned=false".to_owned()
            )
        );
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert!(input.buffer().is_empty());
        assert_eq!(
            input.routing_summary(),
            "role=reviewer preference=balanced endpoint=auto pinned=false"
        );
    }

    #[test]
    fn routing_commands_stay_local_in_recording_and_starting_paths() {
        let mut recording_session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut recording_input = CliInput::default();
        for ch in "/model reviewer fast".chars() {
            recording_input.handle_key_recording(KeyInput::Char(ch), &mut recording_session);
        }

        let recording_action =
            recording_input.handle_key_recording(KeyInput::Enter, &mut recording_session);

        assert_eq!(
            recording_action,
            InputAction::RoutingChanged(
                "role=reviewer preference=prefer_fast endpoint=auto pinned=false".to_owned()
            )
        );
        assert!(recording_session.history().is_empty());
        assert!(recording_session.chunks().is_empty());
        assert_eq!(recording_session.state(), StreamState::Pending);
        assert!(recording_input.buffer().is_empty());

        let mut starting_session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut starting_input = CliInput::default();
        for ch in "/model tester quality summary-tester".chars() {
            starting_input.handle_key_starting(KeyInput::Char(ch), &mut starting_session);
        }

        let starting_action =
            starting_input.handle_key_starting(KeyInput::Enter, &mut starting_session);

        assert_eq!(
            starting_action,
            InputAction::RoutingChanged(
                "role=tester preference=prefer_quality endpoint=summary-tester pinned=true"
                    .to_owned()
            )
        );
        assert!(starting_session.history().is_empty());
        assert!(starting_session.chunks().is_empty());
        assert_eq!(starting_session.state(), StreamState::Pending);
        assert!(starting_input.buffer().is_empty());
    }

    #[test]
    fn routing_commands_under_start_handler_stay_local_when_engine_busy() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#77 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        let mut input = CliInput::default();
        for ch in "/model reviewer fast".chars() {
            input.handle_key_with_model_pool_gate_and_start(
                KeyInput::Char(ch),
                &mut session,
                &gate,
            );
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);

        assert_eq!(
            action,
            InputAction::RoutingChanged(
                "role=reviewer preference=prefer_fast endpoint=auto pinned=false".to_owned()
            )
        );
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(input.config().model_endpoint, None);
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
        assert!(input.buffer().is_empty());

        for ch in "/model tester speedy".chars() {
            input.handle_key_with_model_pool_gate_and_start(
                KeyInput::Char(ch),
                &mut session,
                &gate,
            );
        }
        let invalid =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);

        assert_eq!(
            invalid,
            InputAction::InputError("unknown routing preference: speedy".to_owned())
        );
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(input.buffer(), "/model tester speedy");
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn local_route_and_config_commands_under_start_handler_stay_local_when_backend_offline() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                backend_online: false,
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        let mut input = CliInput::default();
        for ch in "/model reviewer fast".chars() {
            input.handle_key_with_model_pool_gate_and_start(
                KeyInput::Char(ch),
                &mut session,
                &gate,
            );
        }

        let route_action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);
        let route_snapshot = input.action_snapshot(&route_action);

        assert_eq!(
            route_action,
            InputAction::RoutingChanged(
                "role=reviewer preference=prefer_fast endpoint=auto pinned=false".to_owned()
            )
        );
        assert_eq!(route_snapshot.kind, InputActionKind::RoutingChanged);
        assert_eq!(route_snapshot.kind_label, "routing_changed");
        assert_eq!(
            route_snapshot.local_status.as_deref(),
            Some("role=reviewer preference=prefer_fast endpoint=auto pinned=false")
        );
        assert_eq!(route_snapshot.request, None);
        assert_eq!(route_snapshot.start_chunk, None);
        assert_eq!(route_snapshot.start_state, None);
        assert_eq!(route_snapshot.stream_chunk, None);
        assert_eq!(route_snapshot.stream_state, None);
        assert_eq!(route_snapshot.reason, None);
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
        assert!(input.buffer().is_empty());

        for ch in "/max-tokens auto".chars() {
            input.handle_key_with_model_pool_gate_and_start(
                KeyInput::Char(ch),
                &mut session,
                &gate,
            );
        }
        let config_action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);
        let config_snapshot = input.action_snapshot(&config_action);

        assert_eq!(
            config_action,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::DefaultMaxTokens(None),
                summary: "max_tokens=backend-default".to_owned(),
            }
        );
        assert_eq!(config_snapshot.kind, InputActionKind::SessionConfigChanged);
        assert_eq!(config_snapshot.kind_label, "session_config_changed");
        assert_eq!(
            config_snapshot.session_config_update,
            Some(SessionConfigUpdate::DefaultMaxTokens(None))
        );
        assert_eq!(
            config_snapshot.local_status.as_deref(),
            Some("max_tokens=backend-default")
        );
        assert_eq!(config_snapshot.request, None);
        assert_eq!(config_snapshot.start_chunk, None);
        assert_eq!(config_snapshot.start_state, None);
        assert_eq!(config_snapshot.stream_chunk, None);
        assert_eq!(config_snapshot.stream_state, None);
        assert_eq!(config_snapshot.reason, None);
        assert_eq!(session.config().default_max_tokens, None);
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn model_command_sets_role_preference_and_optional_endpoint() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/model reviewer fast fast-reviewer".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::RoutingChanged(
                "role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true"
                    .to_owned()
            )
        );
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(
            input.config().model_endpoint,
            Some(ModelEndpoint::FastReviewer)
        );
    }

    #[test]
    fn model_command_without_endpoint_keeps_auto_route_unpinned_by_default() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/model reviewer fast".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::RoutingChanged(
                "role=reviewer preference=prefer_fast endpoint=auto pinned=false".to_owned()
            )
        );
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(input.config().model_endpoint, None);
    }

    #[test]
    fn model_command_route_serializes_hints_without_endpoint_pin() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/model reviewer fast".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        assert!(matches!(
            input.handle_key(KeyInput::Enter, &session),
            InputAction::RoutingChanged(_)
        ));
        for ch in "quick review".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
        let json = norion_service::request_json(&request);
        assert!(json.contains("\"model_role\":\"reviewer\""));
        assert!(json.contains("\"routing_preference\":\"prefer_fast\""));
        assert!(!json.contains("\"model_endpoint\""));
        assert!(!request.endpoint_pinned());
    }

    #[test]
    fn endpoint_auto_clears_operator_pinned_worker() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let config =
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let mut input = CliInput::new(config);
        for ch in "/endpoint auto".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::RoutingChanged(
                "role=assistant preference=balanced endpoint=auto pinned=false".to_owned()
            )
        );
        assert_eq!(input.config().model_endpoint, None);
    }

    #[test]
    fn worker_auto_alias_clears_operator_pinned_worker() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let config =
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let mut input = CliInput::new(config);
        for ch in "/worker auto".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::RoutingChanged(
                "role=assistant preference=balanced endpoint=auto pinned=false".to_owned()
            )
        );
        assert_eq!(input.config().model_endpoint, None);
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn endpoint_command_serializes_explicit_worker_pin() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/endpoint fast-reviewer".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        assert!(matches!(
            input.handle_key(KeyInput::Enter, &session),
            InputAction::RoutingChanged(_)
        ));
        for ch in "review".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
        let json = norion_service::request_json(&request);
        assert!(request.endpoint_pinned());
        assert!(json.contains("\"model_endpoint\":\"fast-reviewer\""));
    }

    #[test]
    fn endpoint_command_accepts_custom_worker_pin() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/endpoint mlx-reviewer-8b".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        assert_eq!(
            input.handle_key(KeyInput::Enter, &session),
            InputAction::RoutingChanged(
                "role=assistant preference=balanced endpoint=mlx-reviewer-8b pinned=true"
                    .to_owned()
            )
        );
        for ch in "review".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
        let json = norion_service::request_json(&request);
        assert!(request.endpoint_pinned());
        assert!(json.contains("\"model_endpoint\":\"mlx-reviewer-8b\""));
    }

    #[test]
    fn worker_alias_serializes_explicit_worker_pin() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/worker fast-reviewer".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        assert_eq!(
            input.handle_key(KeyInput::Enter, &session),
            InputAction::RoutingChanged(
                "role=assistant preference=balanced endpoint=fast-reviewer pinned=true".to_owned()
            )
        );
        for ch in "review".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
        let json = norion_service::request_json(&request);
        assert!(request.endpoint_pinned());
        assert_eq!(
            request.model_endpoint.as_ref().map(ModelEndpoint::label),
            Some("fast-reviewer")
        );
        assert!(json.contains("\"model_endpoint\":\"fast-reviewer\""));
    }

    #[test]
    fn endpoint_command_requires_explicit_worker_or_auto() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let config =
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let mut input = CliInput::new(config);
        for ch in "/endpoint".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::InputError("missing model endpoint".to_owned())
        );
        assert_eq!(
            input.config().model_endpoint,
            Some(ModelEndpoint::FastReviewer)
        );
        assert_eq!(input.buffer(), "/endpoint");
    }

    #[test]
    fn worker_command_requires_explicit_worker_or_auto() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let config =
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let mut input = CliInput::new(config);
        for ch in "/worker".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::InputError("missing model endpoint".to_owned())
        );
        assert_eq!(
            input.config().model_endpoint,
            Some(ModelEndpoint::FastReviewer)
        );
        assert_eq!(input.buffer(), "/worker");
    }

    #[test]
    fn model_command_endpoint_auto_clears_operator_pinned_worker() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let config =
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let mut input = CliInput::new(config);
        for ch in "/model reviewer fast auto".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::RoutingChanged(
                "role=reviewer preference=prefer_fast endpoint=auto pinned=false".to_owned()
            )
        );
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(input.config().model_endpoint, None);
    }

    #[test]
    fn model_command_without_endpoint_keeps_existing_operator_pin() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let config =
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let mut input = CliInput::new(config);
        for ch in "/model tester quality".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::RoutingChanged(
                "role=tester preference=prefer_quality endpoint=fast-reviewer pinned=true"
                    .to_owned()
            )
        );
        assert_eq!(input.config().model_role, ModelRole::Tester);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferQuality
        );
        assert_eq!(
            input.config().model_endpoint,
            Some(ModelEndpoint::FastReviewer)
        );
    }

    #[test]
    fn max_tokens_command_reports_session_config_update_without_sending_prompt() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/max-tokens 8192".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::DefaultMaxTokens(Some(8192)),
                summary: "max_tokens=8192".to_owned(),
            }
        );
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn recording_max_tokens_command_applies_to_session() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/max-tokens auto".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }
        session.set_default_max_tokens(Some(4096));

        let action = input.handle_key_recording(KeyInput::Enter, &mut session);

        assert_eq!(
            action,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::DefaultMaxTokens(None),
                summary: "max_tokens=backend-default".to_owned(),
            }
        );
        assert_eq!(session.request_for_prompt("hello").max_tokens, None);
        assert!(session.history().is_empty());
    }

    #[test]
    fn recording_history_limit_command_applies_and_truncates_session() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::new(8));
        session.record_user("one");
        session.record_assistant("two");
        session.record_user("three");
        let mut input = CliInput::default();
        for ch in "/history-limit 2".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let action = input.handle_key_recording(KeyInput::Enter, &mut session);

        assert_eq!(
            action,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::HistoryLimit(2),
                summary: "history_limit=2".to_owned(),
            }
        );
        let contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(contents, vec!["two", "three"]);
    }

    #[test]
    fn session_config_commands_under_start_handler_stay_local_when_engine_busy() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::new(8).with_default_max_tokens(Some(4096)),
        );
        session.record_user("one");
        session.record_assistant("two");
        session.record_user("three");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#77 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        let mut input = CliInput::default();
        for ch in "/history-limit 2".chars() {
            input.handle_key_with_model_pool_gate_and_start(
                KeyInput::Char(ch),
                &mut session,
                &gate,
            );
        }

        let history_action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);
        let history_snapshot = input.action_snapshot(&history_action);

        assert_eq!(
            history_action,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::HistoryLimit(2),
                summary: "history_limit=2".to_owned(),
            }
        );
        assert_eq!(history_snapshot.kind, InputActionKind::SessionConfigChanged);
        assert_eq!(history_snapshot.kind_label, "session_config_changed");
        assert_eq!(
            history_snapshot.session_config_update,
            Some(SessionConfigUpdate::HistoryLimit(2))
        );
        let history_update = history_snapshot
            .session_config_update_detail
            .as_ref()
            .expect("history-limit action should expose structured update");
        assert_eq!(history_update.kind_label, "history_limit");
        assert_eq!(history_update.summary, "history_limit=2");
        assert!(!history_update.changes_max_tokens);
        assert!(history_update.changes_history_limit);
        assert_eq!(history_update.max_tokens, None);
        assert_eq!(history_update.max_tokens_label, None);
        assert!(!history_update.max_tokens_backend_default);
        assert_eq!(history_update.history_limit, Some(2));
        assert_eq!(
            history_snapshot.local_status.as_deref(),
            Some("history_limit=2")
        );
        assert_eq!(history_snapshot.request, None);
        assert_eq!(history_snapshot.start_chunk, None);
        assert_eq!(history_snapshot.start_state, None);
        assert_eq!(history_snapshot.stream_chunk, None);
        assert_eq!(history_snapshot.stream_state, None);
        assert_eq!(history_snapshot.reason, None);
        let contents = session
            .history()
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(contents, vec!["two", "three"]);
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.chunks().is_empty());
        assert!(input.buffer().is_empty());

        for ch in "/max-tokens auto".chars() {
            input.handle_key_with_model_pool_gate_and_start(
                KeyInput::Char(ch),
                &mut session,
                &gate,
            );
        }
        let token_action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);
        let token_snapshot = input.action_snapshot(&token_action);

        assert_eq!(
            token_action,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::DefaultMaxTokens(None),
                summary: "max_tokens=backend-default".to_owned(),
            }
        );
        assert_eq!(token_snapshot.kind, InputActionKind::SessionConfigChanged);
        assert_eq!(token_snapshot.kind_label, "session_config_changed");
        assert_eq!(
            token_snapshot.session_config_update,
            Some(SessionConfigUpdate::DefaultMaxTokens(None))
        );
        let token_update = token_snapshot
            .session_config_update_detail
            .as_ref()
            .expect("max-tokens action should expose structured update");
        assert_eq!(token_update.kind_label, "max_tokens");
        assert_eq!(token_update.summary, "max_tokens=backend-default");
        assert!(token_update.changes_max_tokens);
        assert!(!token_update.changes_history_limit);
        assert_eq!(token_update.max_tokens, None);
        assert_eq!(
            token_update.max_tokens_label.as_deref(),
            Some("backend-default")
        );
        assert!(token_update.max_tokens_backend_default);
        assert_eq!(token_update.history_limit, None);
        assert_eq!(
            token_snapshot.local_status.as_deref(),
            Some("max_tokens=backend-default")
        );
        assert_eq!(token_snapshot.request, None);
        assert_eq!(token_snapshot.start_chunk, None);
        assert_eq!(token_snapshot.start_state, None);
        assert_eq!(token_snapshot.stream_chunk, None);
        assert_eq!(token_snapshot.stream_state, None);
        assert_eq!(token_snapshot.reason, None);
        assert_eq!(session.config().default_max_tokens, None);
        assert_eq!(session.request_for_prompt("next").max_tokens, None);
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.chunks().is_empty());
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn invalid_session_config_command_under_start_handler_is_not_busy_or_prompt() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::new(8).with_default_max_tokens(Some(4096)),
        );
        session.record_user("one");
        session.record_assistant("two");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#77 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        let mut input = CliInput::default();
        for ch in "/history-limit many".chars() {
            input.handle_key_with_model_pool_gate_and_start(
                KeyInput::Char(ch),
                &mut session,
                &gate,
            );
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);

        assert_eq!(
            action,
            InputAction::InputError("history limit must be a positive integer".to_owned())
        );
        assert_eq!(input.buffer(), "/history-limit many");
        assert_eq!(session.config().history_limit, 8);
        assert_eq!(session.config().default_max_tokens, Some(4096));
        assert_eq!(session.state(), StreamState::Pending);
        assert_eq!(session.history().len(), 2);
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn invalid_local_commands_under_start_handler_stay_input_errors_when_backend_offline() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::new(8).with_default_max_tokens(Some(4096)),
        );
        session.record_user("one");
        session.record_assistant("two");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                backend_online: false,
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        let mut route_input = CliInput::default();
        for ch in "/model reviewer speedy".chars() {
            route_input.handle_key_with_model_pool_gate_and_start(
                KeyInput::Char(ch),
                &mut session,
                &gate,
            );
        }

        let route_error = route_input.handle_key_with_model_pool_gate_and_start(
            KeyInput::Enter,
            &mut session,
            &gate,
        );
        let route_snapshot = route_input.action_snapshot(&route_error);

        assert_eq!(
            route_error,
            InputAction::InputError("unknown routing preference: speedy".to_owned())
        );
        assert_eq!(route_snapshot.kind, InputActionKind::InputError);
        assert_eq!(route_snapshot.kind_label, "input_error");
        assert_eq!(
            route_snapshot.local_status.as_deref(),
            Some("unknown routing preference: speedy")
        );
        assert_eq!(route_snapshot.request, None);
        assert_eq!(route_snapshot.start_chunk, None);
        assert_eq!(route_snapshot.start_state, None);
        assert_eq!(route_snapshot.stream_chunk, None);
        assert_eq!(route_snapshot.stream_state, None);
        assert_eq!(route_snapshot.reason, None);
        assert_eq!(route_input.buffer(), "/model reviewer speedy");
        assert_eq!(route_input.config().model_role, ModelRole::Assistant);
        assert_eq!(
            route_input.config().routing_preference,
            RoutingPreference::Balanced
        );

        let mut token_input = CliInput::default();
        for ch in "/max-tokens many".chars() {
            token_input.handle_key_with_model_pool_gate_and_start(
                KeyInput::Char(ch),
                &mut session,
                &gate,
            );
        }

        let token_error = token_input.handle_key_with_model_pool_gate_and_start(
            KeyInput::Enter,
            &mut session,
            &gate,
        );
        let token_snapshot = token_input.action_snapshot(&token_error);

        assert_eq!(
            token_error,
            InputAction::InputError("max token budget must be a positive integer".to_owned())
        );
        assert_eq!(token_snapshot.kind, InputActionKind::InputError);
        assert_eq!(token_snapshot.kind_label, "input_error");
        assert_eq!(
            token_snapshot.local_status.as_deref(),
            Some("max token budget must be a positive integer")
        );
        assert_eq!(token_snapshot.request, None);
        assert_eq!(token_snapshot.start_chunk, None);
        assert_eq!(token_snapshot.start_state, None);
        assert_eq!(token_snapshot.stream_chunk, None);
        assert_eq!(token_snapshot.stream_state, None);
        assert_eq!(token_snapshot.reason, None);
        assert_eq!(token_input.buffer(), "/max-tokens many");
        assert_eq!(session.config().history_limit, 8);
        assert_eq!(session.config().default_max_tokens, Some(4096));
        assert_eq!(session.state(), StreamState::Pending);
        assert_eq!(session.history().len(), 2);
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn invalid_max_tokens_command_is_not_sent_as_prompt() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/max-tokens many".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::InputError("max token budget must be a positive integer".to_owned())
        );
        assert_eq!(input.buffer(), "/max-tokens many");
    }

    #[test]
    fn status_command_reports_session_state_without_sending_prompt() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        session.push_delta("partial");
        let mut input = CliInput::default();
        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=assistant preference=balanced endpoint=auto pinned=false state=streaming history=1 max_tokens=4096 partial_chars=7 advice=wait_for_current_stream busy: session stream is already active"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn status_command_after_cancel_reports_interrupted_partial_without_request() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        session.push_delta("partial answer");
        session.cancel_stream().expect("expected cancel chunk");
        let mut input = CliInput::default();
        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=assistant preference=balanced endpoint=auto pinned=false state=interrupted history=1 max_tokens=4096 partial_chars=14 last_error=stream cancelled by user"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());
        assert_eq!(session.state(), StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial answer");
        assert_eq!(session.last_error(), Some("stream cancelled by user"));
        assert_eq!(session.history().len(), 1);
    }

    #[test]
    fn status_command_includes_gate_advice_when_gate_is_supplied() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = FrontendGateSnapshot {
            engine_busy: true,
            active_request: Some("#21 chat-stream".to_owned()),
            ..FrontendGateSnapshot::default()
        };
        let mut input = CliInput::default();
        for ch in "/state".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_gate(KeyInput::Enter, &session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=assistant preference=balanced endpoint=auto pinned=false state=pending history=0 max_tokens=backend-default partial_chars=0 advice=wait_for_current_stream busy: backend engine is busy: #21 chat-stream"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn status_command_prefers_active_session_over_allowed_frontend_gate() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        let gate = FrontendGateSnapshot::default();
        let mut input = CliInput::default();
        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_gate(KeyInput::Enter, &session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=assistant preference=balanced endpoint=auto pinned=false state=streaming history=1 max_tokens=backend-default partial_chars=0 advice=wait_for_current_stream busy: session stream is already active"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.history().len(), 1);
    }

    #[test]
    fn status_command_keeps_governance_gate_over_active_session_pressure() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        let gate = FrontendGateSnapshot {
            safe_device_ok: false,
            ..FrontendGateSnapshot::default()
        };
        let mut input = CliInput::default();
        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_gate(KeyInput::Enter, &session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=assistant preference=balanced endpoint=auto pinned=false state=streaming history=1 max_tokens=backend-default partial_chars=0 advice=repair_gate failed: safe-device gate failed"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());

        let offline_gate = FrontendGateSnapshot {
            backend_online: false,
            ..FrontendGateSnapshot::default()
        };
        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let offline_action = input.handle_key_with_gate(KeyInput::Enter, &session, &offline_gate);
        let offline_snapshot = input.action_snapshot(&offline_action);

        let InputAction::Status(offline_line) = offline_action else {
            panic!("expected offline status action");
        };
        assert_eq!(
            offline_line,
            "role=assistant preference=balanced endpoint=auto pinned=false state=streaming history=1 max_tokens=backend-default partial_chars=0 advice=repair_gate failed: backend is offline"
        );
        assert_eq!(offline_snapshot.kind, InputActionKind::Status);
        assert_eq!(offline_snapshot.kind_label, "status");
        assert_eq!(
            offline_snapshot.local_status.as_deref(),
            Some(offline_line.as_str())
        );
        assert_eq!(offline_snapshot.request, None);
        assert_eq!(offline_snapshot.start_chunk, None);
        assert_eq!(offline_snapshot.start_state, None);
        assert_eq!(offline_snapshot.stream_chunk, None);
        assert_eq!(offline_snapshot.stream_state, None);
        assert_eq!(offline_snapshot.reason, None);
        assert!(input.buffer().is_empty());
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.history().len(), 1);
    }

    #[test]
    fn status_command_under_model_pool_gate_includes_pool_capacity() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("quality".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_model_pool_gate(KeyInput::Enter, &session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending history=0 max_tokens=backend-default partial_chars=0 advice=send_now pending: ready to send pool=workers total=2 available=1 busy=1 saturated=0"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn status_command_under_repair_gate_stays_read_only_and_does_not_start_stream() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                experience_hygiene_ok: false,
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending history=0 max_tokens=backend-default partial_chars=0 advice=repair_gate failed: experience hygiene gate failed pool=workers total=1 available=1 busy=0 saturated=0 route_pool=matching total=1 available=1 busy=0 saturated=0"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());

        let hygiene_gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                experience_hygiene_ok: false,
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        for ch in "/workers".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_model_pool_gate_and_start(
            KeyInput::Enter,
            &mut session,
            &hygiene_gate,
        );
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected hygiene worker status action");
        };
        assert_eq!(
            line,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false advice=repair_gate failed: experience hygiene gate failed pool=workers total=1 available=1 busy=0 saturated=0 route_pool=matching total=1 available=1 busy=0 saturated=0 workers=[endpoint=fast-reviewer status=available queue=0/1 active=none roles=reviewer preferences=prefer_fast]"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert_eq!(snapshot.model_role_label, "reviewer");
        assert_eq!(snapshot.routing_preference_label, "prefer_fast");
        assert_eq!(snapshot.endpoint_label, "auto");
        assert!(!snapshot.endpoint_pinned);
        assert!(!snapshot.wire_sends_model_endpoint);
        assert!(input.buffer().is_empty());
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn status_command_under_engine_busy_stays_read_only_and_does_not_start_stream() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
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
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert!(line.contains(
            "advice=wait_for_current_stream busy: backend engine is busy: #77 chat-stream"
        ));
        assert!(line.contains("pool=workers total=1 available=1 busy=0 saturated=0"));
        assert!(line.contains("route_pool=matching total=1 available=1 busy=0 saturated=0"));
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn status_and_workers_commands_under_backend_offline_stay_read_only() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                backend_online: false,
                engine_busy: true,
                safe_device_ok: false,
                experience_hygiene_ok: false,
                queued_requests: 8,
                queue_limit: 8,
                active_request: Some("#42 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let status_action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);
        let status_snapshot = input.action_snapshot(&status_action);

        let InputAction::Status(status_line) = status_action else {
            panic!("expected offline status action");
        };
        assert_eq!(
            status_line,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending history=0 max_tokens=backend-default partial_chars=0 advice=repair_gate failed: backend is offline pool=workers total=1 available=1 busy=0 saturated=0 route_pool=matching total=1 available=1 busy=0 saturated=0"
        );
        assert_eq!(status_snapshot.kind, InputActionKind::Status);
        assert_eq!(status_snapshot.kind_label, "status");
        assert_eq!(
            status_snapshot.local_status.as_deref(),
            Some(status_line.as_str())
        );
        assert_eq!(status_snapshot.request, None);
        assert_eq!(status_snapshot.start_chunk, None);
        assert_eq!(status_snapshot.start_state, None);
        assert_eq!(status_snapshot.stream_chunk, None);
        assert_eq!(status_snapshot.stream_state, None);
        assert_eq!(status_snapshot.reason, None);
        assert!(input.buffer().is_empty());
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());

        for ch in "/workers".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let workers_action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);
        let workers_snapshot = input.action_snapshot(&workers_action);

        let InputAction::Status(workers_line) = workers_action else {
            panic!("expected offline workers action");
        };
        assert_eq!(
            workers_line,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false advice=repair_gate failed: backend is offline pool=workers total=1 available=1 busy=0 saturated=0 route_pool=matching total=1 available=1 busy=0 saturated=0 workers=[endpoint=fast-reviewer status=available queue=0/1 active=none roles=reviewer preferences=prefer_fast]"
        );
        assert_eq!(workers_snapshot.kind, InputActionKind::Status);
        assert_eq!(workers_snapshot.kind_label, "status");
        assert_eq!(
            workers_snapshot.local_status.as_deref(),
            Some(workers_line.as_str())
        );
        assert_eq!(workers_snapshot.request, None);
        assert_eq!(workers_snapshot.start_chunk, None);
        assert_eq!(workers_snapshot.start_state, None);
        assert_eq!(workers_snapshot.stream_chunk, None);
        assert_eq!(workers_snapshot.stream_state, None);
        assert_eq!(workers_snapshot.reason, None);
        assert!(input.buffer().is_empty());
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn workers_command_without_pool_gate_is_local_unavailable_status() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/workers".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=assistant preference=balanced endpoint=auto pinned=false workers=unavailable reason=model-pool-gate-not-attached"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn workers_command_under_model_pool_gate_lists_workers_without_pinning_auto_route() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("quality".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "/workers".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_model_pool_gate(KeyInput::Enter, &session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false advice=send_now pending: ready to send pool=workers total=2 available=1 busy=1 saturated=0 workers=[endpoint=quality-12b status=busy queue=0/1 active=quality | endpoint=fast-reviewer status=available queue=0/1 active=none]"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn workers_command_under_repair_gate_stays_read_only_and_does_not_start_stream() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                safe_device_ok: false,
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "/workers".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false advice=repair_gate failed: safe-device gate failed pool=workers total=1 available=1 busy=0 saturated=0 route_pool=matching total=1 available=1 busy=0 saturated=0 workers=[endpoint=fast-reviewer status=available queue=0/1 active=none roles=reviewer preferences=prefer_fast]"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn workers_command_under_engine_busy_stays_read_only_and_does_not_start_stream() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
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
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "/workers".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert!(line.contains(
            "advice=wait_for_current_stream busy: backend engine is busy: #77 chat-stream"
        ));
        assert!(line.contains("workers=[endpoint=fast-reviewer status=available"));
        assert!(line.contains("roles=reviewer preferences=prefer_fast"));
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn workers_command_under_model_pool_gate_prefers_active_session_pressure() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        let history_len_before = session.history().len();
        let chunks_len_before = session.chunks().len();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "/workers".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_model_pool_gate(KeyInput::Enter, &session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false advice=wait_for_current_stream busy: session stream is already active pool=workers total=1 available=1 busy=0 saturated=0 workers=[endpoint=fast-reviewer status=available queue=0/1 active=none]"
        );
        assert_eq!(snapshot.kind, InputActionKind::Status);
        assert_eq!(snapshot.kind_label, "status");
        assert_eq!(snapshot.local_status.as_deref(), Some(line.as_str()));
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert!(input.buffer().is_empty());
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.history().len(), history_len_before);
        assert_eq!(session.chunks().len(), chunks_len_before);
    }

    #[test]
    fn workers_command_under_model_pool_gate_shows_pinned_worker_pressure() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_busy(true, Some("#9 review".to_owned())),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer)),
        );
        for ch in "/endpoints".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_model_pool_gate(KeyInput::Enter, &session, &gate);

        let InputAction::Status(line) = action else {
            panic!("expected status action");
        };
        assert_eq!(
            line,
            "role=assistant preference=balanced endpoint=fast-reviewer pinned=true advice=wait_for_current_stream busy: worker fast-reviewer is busy: #9 review pool=workers total=2 available=1 busy=1 saturated=0 workers=[endpoint=quality-12b status=available queue=0/1 active=none | endpoint=fast-reviewer status=busy queue=0/1 active=#9 review]"
        );
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn invalid_slash_command_is_not_sent_as_prompt() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/role speedy".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::InputError("unknown model role: speedy".to_owned())
        );
        assert_eq!(input.buffer(), "/role speedy");
    }

    #[test]
    fn unknown_slash_command_is_not_sent_as_prompt() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/unknown hello".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::InputError("unknown slash command: /unknown".to_owned())
        );
        assert_eq!(input.buffer(), "/unknown hello");
    }

    #[test]
    fn invalid_model_command_does_not_partially_update_routing() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/model reviewer speedy".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        assert_eq!(
            action,
            InputAction::InputError("unknown routing preference: speedy".to_owned())
        );
        assert_eq!(input.config().model_role, ModelRole::Assistant);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::Balanced
        );
        assert_eq!(input.config().model_endpoint, None);
        assert_eq!(input.buffer(), "/model reviewer speedy");
    }

    #[test]
    fn gated_enter_blocks_send_and_keeps_prompt_buffer_when_backend_busy() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = FrontendGateSnapshot {
            engine_busy: true,
            active_request: Some("#42 chat-stream 1200ms".to_owned()),
            ..FrontendGateSnapshot::default()
        };
        let mut input = CliInput::default();
        for ch in "please answer".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_gate(KeyInput::Enter, &session, &gate);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, norion_service::StreamState::Busy);
        assert_eq!(
            chunk.content,
            "backend engine is busy: #42 chat-stream 1200ms"
        );
        assert_eq!(input.buffer(), "please answer");
    }

    #[test]
    fn gated_enter_allows_send_and_preserves_routing_when_gate_allows() {
        let session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );
        let gate = FrontendGateSnapshot::default();
        let config = CliInputConfig::default()
            .prefer_quality()
            .with_model_role(ModelRole::Assistant)
            .with_model_endpoint(Some(ModelEndpoint::Quality12B));
        let mut input = CliInput::new(config);
        for ch in "deep answer".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_gate(KeyInput::Enter, &session, &gate);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
        assert_eq!(request.max_tokens, Some(4096));
        assert_eq!(request.routing_preference, RoutingPreference::PreferQuality);
        assert_eq!(request.model_endpoint, Some(ModelEndpoint::Quality12B));
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn model_pool_gate_blocks_pinned_busy_worker_and_keeps_prompt_buffer() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_busy(true, Some("#9 review".to_owned())),
            ],
        );
        let config =
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let mut input = CliInput::new(config);
        for ch in "review this".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_model_pool_gate(KeyInput::Enter, &session, &gate);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Busy);
        assert_eq!(chunk.content, "worker fast-reviewer is busy: #9 review");
        assert_eq!(input.buffer(), "review this");
    }

    #[test]
    fn model_pool_gate_allows_auto_route_when_another_worker_is_available() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("deep answer".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "quick pass".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_model_pool_gate(KeyInput::Enter, &session, &gate);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
        assert_eq!(request.model_role, ModelRole::Reviewer);
        assert_eq!(request.routing_preference, RoutingPreference::PreferFast);
        assert_eq!(request.model_endpoint, None);
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn model_pool_gate_blocks_auto_route_when_no_worker_matches_capabilities() {
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
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Tester)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "run tests".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key_with_model_pool_gate(KeyInput::Enter, &session, &gate);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Queued);
        assert_eq!(
            chunk.content,
            "no model worker matches role=tester preference=prefer_fast"
        );
        assert_eq!(input.buffer(), "run tests");
    }

    #[test]
    fn recording_model_pool_gate_does_not_record_user_when_pinned_worker_is_saturated() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester).with_queue(2, 2)],
        );
        let config =
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::SummaryTester));
        let mut input = CliInput::new(config);
        for ch in "summarize".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_record(KeyInput::Enter, &mut session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Backpressure);
        assert_eq!(snapshot.kind, InputActionKind::Blocked);
        assert_eq!(snapshot.kind_label, "blocked");
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_state, Some(StreamState::Backpressure));
        assert_eq!(snapshot.stream_state_label.as_deref(), Some("backpressure"));
        assert_eq!(snapshot.stream_state_is_pressure, Some(true));
        assert_eq!(snapshot.stream_state_blocks_prompt_submit, Some(true));
        assert_eq!(
            snapshot.reason.as_deref(),
            Some("worker summary-tester queue is saturated: 2/2")
        );
        assert!(session.history().is_empty());
        assert_eq!(input.buffer(), "summarize");
    }

    #[test]
    fn recording_model_pool_gate_blocks_capability_no_match_without_history() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
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
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Tester)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "run tests".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_record(KeyInput::Enter, &mut session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Queued);
        assert_eq!(
            chunk.content,
            "no model worker matches role=tester preference=prefer_fast"
        );
        assert_eq!(snapshot.kind, InputActionKind::Blocked);
        assert_eq!(snapshot.kind_label, "blocked");
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_state, Some(StreamState::Queued));
        assert_eq!(snapshot.stream_state_label.as_deref(), Some("queued"));
        assert_eq!(snapshot.stream_state_is_pressure, Some(true));
        assert_eq!(snapshot.stream_state_blocks_prompt_submit, Some(true));
        assert_eq!(
            snapshot.reason.as_deref(),
            Some("no model worker matches role=tester preference=prefer_fast")
        );
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
        assert_eq!(session.state(), StreamState::Pending);
        assert_eq!(input.buffer(), "run tests");
    }

    #[test]
    fn recording_model_pool_gate_preserves_context_limits_and_unpinned_route() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::new(8).with_default_max_tokens(Some(8192)),
        );
        session.record_user("first question");
        session.record_assistant("first answer");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("quality".to_owned()))
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "review second patch".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_record(KeyInput::Enter, &mut session, &gate);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
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

        assert_eq!(
            request_contents,
            vec!["first question", "first answer", "review second patch"]
        );
        assert_eq!(history_contents, request_contents);
        assert_eq!(request.max_tokens, Some(8192));
        assert_eq!(request.model_role, ModelRole::Reviewer);
        assert_eq!(request.routing_preference, RoutingPreference::PreferFast);
        assert_eq!(request.model_endpoint, None);
        assert!(!request.endpoint_pinned());
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.chunks().is_empty());
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn recording_enter_adds_user_to_session_history_after_gate_allows() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );
        let gate = FrontendGateSnapshot::default();
        let mut input = CliInput::default();
        for ch in "hello".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let action = input.handle_key_with_gate_and_record(KeyInput::Enter, &mut session, &gate);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
        assert_eq!(request.messages.last().unwrap().content, "hello");
        assert_eq!(request.model_endpoint, None);
        assert_eq!(session.history().last().unwrap().content, "hello");
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn recording_enter_does_not_record_user_when_gate_blocks() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = FrontendGateSnapshot {
            queued_requests: 2,
            queue_limit: 2,
            ..FrontendGateSnapshot::default()
        };
        let mut input = CliInput::default();
        for ch in "hello".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let action = input.handle_key_with_gate_and_record(KeyInput::Enter, &mut session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, norion_service::StreamState::Backpressure);
        assert_eq!(snapshot.kind, InputActionKind::Blocked);
        assert_eq!(snapshot.kind_label, "blocked");
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_state, Some(StreamState::Backpressure));
        assert_eq!(snapshot.stream_state_label.as_deref(), Some("backpressure"));
        assert_eq!(snapshot.stream_state_is_pressure, Some(true));
        assert_eq!(snapshot.stream_state_blocks_prompt_submit, Some(true));
        assert_eq!(
            snapshot.reason.as_deref(),
            Some("model queue is saturated: 2/2")
        );
        assert!(session.history().is_empty());
        assert_eq!(input.buffer(), "hello");
    }

    #[test]
    fn enter_blocks_without_send_when_session_stream_is_active() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session.submit_prompt("first");
        session.begin_stream();
        let mut input = CliInput::default();
        for ch in "second".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Busy);
        assert_eq!(chunk.content, "session stream is already active");
        assert_eq!(input.buffer(), "second");
    }

    #[test]
    fn enter_blocks_with_latest_session_pressure_reason() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session.backpressure("pool queue full");
        let mut input = CliInput::default();
        for ch in "second".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let action = input.handle_key_recording(KeyInput::Enter, &mut session);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Backpressure);
        assert_eq!(chunk.content, "pool queue full");
        assert!(session.history().is_empty());
        assert_eq!(input.buffer(), "second");
    }

    #[test]
    fn recording_enter_blocks_active_stream_without_recording_user() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session.submit_prompt("first");
        session.begin_stream();
        let mut input = CliInput::default();
        for ch in "second".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let action = input.handle_key_recording(KeyInput::Enter, &mut session);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Busy);
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "first");
        assert_eq!(input.buffer(), "second");
    }

    #[test]
    fn recording_enter_recovers_after_interrupted_or_failed_session() {
        let mut interrupted = ChatSession::new("cli", ChatSessionConfig::default());
        interrupted
            .try_submit_and_begin_stream("hello")
            .expect("expected first stream");
        interrupted.push_delta("partial");
        interrupted.interrupt("backend stream closed");
        let mut input = CliInput::default();
        for ch in "next".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut interrupted);
        }

        let action = input.handle_key_recording(KeyInput::Enter, &mut interrupted);

        let InputAction::Send(request) = action else {
            panic!("expected send action after interrupted session");
        };
        let request_contents = request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(request_contents, vec!["hello", "next"]);
        assert_eq!(interrupted.state(), StreamState::Interrupted);
        assert_eq!(interrupted.partial_answer(), "partial");
        assert_eq!(interrupted.last_error(), Some("backend stream closed"));
        assert_eq!(interrupted.history().len(), 2);
        assert_eq!(interrupted.history()[1].content, "next");
        assert!(input.buffer().is_empty());

        let mut failed = ChatSession::new("cli", ChatSessionConfig::default());
        failed.begin_stream();
        failed.fail("safe-device gate failed");
        let mut retry_input = CliInput::default();
        for ch in "repair and retry".chars() {
            retry_input.handle_key_recording(KeyInput::Char(ch), &mut failed);
        }

        let action = retry_input.handle_key_recording(KeyInput::Enter, &mut failed);

        let InputAction::Send(request) = action else {
            panic!("expected send action after failed session");
        };
        assert_eq!(request.messages.last().unwrap().content, "repair and retry");
        assert_eq!(failed.state(), StreamState::Failed);
        assert_eq!(failed.last_error(), Some("safe-device gate failed"));
        assert_eq!(failed.history().last().unwrap().content, "repair and retry");
        assert!(retry_input.buffer().is_empty());
    }

    #[test]
    fn canceling_ctrl_x_interrupts_active_stream_and_keeps_partial() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session.submit_prompt("hello");
        session.begin_stream();
        session.push_delta("partial");
        let mut input = CliInput::default();

        let action = input.handle_key_canceling(KeyInput::CtrlX, &mut session);

        let InputAction::StreamCancelled(chunk) = action else {
            panic!("expected stream-cancelled action");
        };
        assert_eq!(chunk.state, StreamState::Interrupted);
        assert_eq!(chunk.content, "stream cancelled by user");
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "hello");
    }

    #[test]
    fn canceling_ctrl_x_noops_when_no_stream_is_active() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();

        let action = input.handle_key_canceling(KeyInput::CtrlX, &mut session);
        let snapshot = input.action_snapshot(&action);

        assert_eq!(action, InputAction::CancelStream);
        assert_eq!(snapshot.kind, InputActionKind::CancelStream);
        assert_eq!(snapshot.kind_label, "cancel_stream");
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert_eq!(snapshot.local_status, None);
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn model_pool_cancel_handler_noops_without_active_stream_even_when_gate_blocks() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                backend_online: false,
                engine_busy: true,
                active_request: Some("#77 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        let mut input = CliInput::new(CliInputConfig::default().prefer_fast());

        let action =
            input.handle_key_with_model_pool_gate_and_cancel(KeyInput::CtrlX, &mut session, &gate);
        let snapshot = input.action_snapshot(&action);

        assert_eq!(action, InputAction::CancelStream);
        assert_eq!(snapshot.kind, InputActionKind::CancelStream);
        assert_eq!(snapshot.kind_label, "cancel_stream");
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.reason, None);
        assert_eq!(snapshot.local_status, None);
        assert_eq!(snapshot.routing_preference_label, "prefer_fast");
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn model_pool_cancel_handler_prefers_cancel_over_gate_state() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected started stream");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                queued_requests: 8,
                queue_limit: 8,
                ..FrontendGateSnapshot::default()
            },
            Vec::new(),
        );
        let mut input = CliInput::default();

        let action =
            input.handle_key_with_model_pool_gate_and_cancel(KeyInput::CtrlX, &mut session, &gate);

        let InputAction::StreamCancelled(chunk) = action else {
            panic!("expected stream-cancelled action");
        };
        assert_eq!(chunk.state, StreamState::Interrupted);
        assert_eq!(chunk.content, "stream cancelled by user");
        assert_eq!(session.state(), StreamState::Interrupted);
    }

    #[test]
    fn model_pool_cancel_handler_prefers_cancel_over_engine_busy() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected started stream");
        session.push_delta("partial");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#77 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        let mut input = CliInput::default();

        let action =
            input.handle_key_with_model_pool_gate_and_cancel(KeyInput::CtrlX, &mut session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::StreamCancelled(chunk) = action else {
            panic!("expected stream-cancelled action");
        };
        assert_eq!(chunk.state, StreamState::Interrupted);
        assert_eq!(chunk.content, "stream cancelled by user");
        assert_eq!(session.state(), StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.history().len(), 1);
        assert_eq!(snapshot.kind, InputActionKind::StreamCancelled);
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.stream_state, Some(StreamState::Interrupted));
        assert_eq!(snapshot.reason.as_deref(), Some("stream cancelled by user"));
    }

    #[test]
    fn model_pool_cancel_handler_prefers_cancel_over_repair_gate() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected started stream");
        session.push_delta("partial");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                backend_online: false,
                engine_busy: true,
                safe_device_ok: false,
                experience_hygiene_ok: false,
                queued_requests: 8,
                queue_limit: 8,
                active_request: Some("#77 chat-stream".to_owned()),
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        let mut input = CliInput::default();

        let action =
            input.handle_key_with_model_pool_gate_and_cancel(KeyInput::CtrlX, &mut session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::StreamCancelled(chunk) = action else {
            panic!("expected stream-cancelled action");
        };
        assert_eq!(chunk.state, StreamState::Interrupted);
        assert_eq!(chunk.content, "stream cancelled by user");
        assert_eq!(snapshot.kind, InputActionKind::StreamCancelled);
        assert_eq!(snapshot.kind_label, "stream_cancelled");
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_state, Some(StreamState::Interrupted));
        assert_eq!(snapshot.stream_state_label.as_deref(), Some("interrupted"));
        assert_eq!(snapshot.stream_state_is_terminal, Some(true));
        assert_eq!(snapshot.stream_state_is_pressure, Some(false));
        assert_eq!(snapshot.stream_state_blocks_prompt_submit, Some(false));
        assert_eq!(snapshot.reason.as_deref(), Some("stream cancelled by user"));
        let stream_chunk = snapshot
            .stream_chunk
            .as_ref()
            .expect("cancel over repair gate should expose interrupted display chunk");
        assert_eq!(stream_chunk.output_label, "interrupted");
        assert_eq!(
            stream_chunk.appended,
            "[interrupted] stream cancelled by user"
        );
        assert_eq!(session.state(), StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.history().len(), 1);
    }

    #[test]
    fn cancel_action_snapshot_exposes_interrupted_chunk_without_request_metadata() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected started stream");
        session.push_delta("partial");
        let mut input = CliInput::new(CliInputConfig::default().prefer_quality());

        let action = input.handle_key_canceling(KeyInput::CtrlX, &mut session);
        let snapshot = input.action_snapshot(&action);

        assert_eq!(snapshot.kind, InputActionKind::StreamCancelled);
        assert_eq!(snapshot.kind_label, "stream_cancelled");
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.stream_state, Some(StreamState::Interrupted));
        assert_eq!(snapshot.stream_state_label.as_deref(), Some("interrupted"));
        assert_eq!(snapshot.stream_state_is_terminal, Some(true));
        assert_eq!(snapshot.stream_state_is_pressure, Some(false));
        assert_eq!(snapshot.stream_state_blocks_prompt_submit, Some(false));
        assert_eq!(snapshot.reason.as_deref(), Some("stream cancelled by user"));
        let stream_chunk = snapshot
            .stream_chunk
            .as_ref()
            .expect("cancelled action should expose a stream chunk");
        assert_eq!(stream_chunk.kind_label, "error");
        assert_eq!(stream_chunk.state_label, "interrupted");
        assert_eq!(stream_chunk.output_label, "interrupted");
        assert_eq!(
            stream_chunk.appended,
            "[interrupted] stream cancelled by user"
        );
        assert!(stream_chunk.state_is_terminal);
        assert!(!stream_chunk.state_is_pressure);
        assert!(!stream_chunk.state_blocks_prompt_submit);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_sequence, None);
        assert_eq!(snapshot.model_role_label, "assistant");
        assert_eq!(snapshot.routing_preference_label, "prefer_quality");
        assert_eq!(snapshot.endpoint_label, "auto");
        assert!(!snapshot.wire_sends_model_endpoint);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(session.history().len(), 1);
    }

    #[test]
    fn control_snapshot_reenables_start_after_cancel_without_promoting_partial_context() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(4096)),
        );
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected started stream");
        session.push_delta("partial answer");
        session.cancel_stream().expect("expected cancel chunk");
        let mut input = CliInput::default();
        for ch in "next question".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let control = input.control_snapshot(&session, InputSubmitMode::StartStream);

        assert!(control.send_enabled);
        assert!(control.enter_submits_prompt);
        assert!(!control.enter_runs_local_command);
        assert!(!control.enter_is_blocked);
        assert_eq!(control.primary_action_label, "send");
        assert!(control.primary_action_enabled);
        assert_eq!(control.primary_action_disabled_reason, None);
        assert_eq!(control.advice_action, Some(GateAdviceAction::SendNow));
        assert_eq!(control.advice_action_label.as_deref(), Some("send_now"));
        assert_eq!(control.block_state, None);
        assert_eq!(control.block_reason, None);
        assert!(!control.preserves_buffer_on_enter);
        assert!(control.clears_buffer_on_enter);
        assert_eq!(control.readiness.enter_action, InputActionKind::StartStream);
        assert!(control.readiness.records_user_on_enter);
        assert!(control.readiness.starts_stream_on_enter);

        let request = control
            .request_preview
            .as_ref()
            .expect("cancelled session should preview the next request");
        assert_eq!(request.messages, 2);
        assert_eq!(request.context_messages, 1);
        assert_eq!(request.messages, request.context_messages + 1);
        assert_eq!(request.history_messages, 1);
        assert_eq!(request.history_limit, Some(64));
        assert_eq!(request.history_remaining, Some(63));
        assert_eq!(request.history_messages_after_submit, Some(2));
        assert_eq!(request.history_at_limit_after_submit, Some(false));
        assert_eq!(request.history_truncates_on_submit, Some(false));
        assert_eq!(request.context_kind, ChatRequestContextKind::MultiTurn);
        assert!(request.has_context);
        assert!(!request.is_single_turn);
        assert_eq!(request.last_user_chars, 13);
        assert_eq!(request.max_tokens, Some(4096));
        assert_eq!(request.max_tokens_label, "4096");
        assert!(request.wire_sends_max_tokens);
        assert_eq!(request.wire_max_tokens, Some(4096));

        let prompt_control = control
            .prompt_submit_control
            .as_ref()
            .expect("prompt control should expose send-now after cancel");
        assert!(prompt_control.prompt_present);
        assert!(prompt_control.send_allowed);
        assert_eq!(prompt_control.action, GateAdviceAction::SendNow);
        assert_eq!(prompt_control.state, StreamState::Pending);
        assert_eq!(prompt_control.primary_action_disabled_reason, None);
        assert!(!prompt_control.preserves_prompt);
        assert!(prompt_control.clears_prompt);
        assert_eq!(session.state(), StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial answer");
        assert_eq!(session.last_error(), Some("stream cancelled by user"));
        assert_eq!(session.history().len(), 1);
        assert_eq!(input.buffer(), "next question");
    }

    #[test]
    fn model_pool_start_recovers_after_cancel_without_recording_partial_assistant() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected started stream");
        session.push_delta("partial answer");
        let blocked_gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                queued_requests: 8,
                queue_limit: 8,
                ..FrontendGateSnapshot::default()
            },
            Vec::new(),
        );
        let ready_gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let mut input = CliInput::default();

        let cancel = input.handle_key_with_model_pool_gate_and_cancel(
            KeyInput::CtrlX,
            &mut session,
            &blocked_gate,
        );

        let InputAction::StreamCancelled(cancelled) = cancel else {
            panic!("expected stream-cancelled action");
        };
        assert_eq!(cancelled.state, StreamState::Interrupted);
        assert_eq!(session.partial_answer(), "partial answer");
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "hello");

        for ch in "next question".chars() {
            input.handle_key_with_model_pool_gate_and_start(
                KeyInput::Char(ch),
                &mut session,
                &ready_gate,
            );
        }
        let restart = input.handle_key_with_model_pool_gate_and_start(
            KeyInput::Enter,
            &mut session,
            &ready_gate,
        );

        let InputAction::StartStream(turn) = restart else {
            panic!("expected start-stream action after cancel");
        };
        let restart_snapshot = input.action_snapshot(&InputAction::StartStream(turn.clone()));
        assert_eq!(restart_snapshot.kind, InputActionKind::StartStream);
        assert_eq!(restart_snapshot.stream_chunk, None);
        assert_eq!(restart_snapshot.stream_state, None);
        assert_eq!(restart_snapshot.start_state, Some(StreamState::Streaming));
        assert_eq!(
            restart_snapshot.start_state_label.as_deref(),
            Some("streaming")
        );
        let request_contents = turn
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
        assert_eq!(request_contents, vec!["hello", "next question"]);
        assert_eq!(history_contents, request_contents);
        let request_snapshot = restart_snapshot
            .request
            .as_ref()
            .expect("restart should expose structured request metadata");
        assert_eq!(request_snapshot.messages, 2);
        assert_eq!(request_snapshot.context_messages, 1);
        assert_eq!(request_snapshot.history_messages, 1);
        assert_eq!(request_snapshot.history_limit, None);
        assert_eq!(request_snapshot.history_messages_after_submit, None);
        assert_eq!(
            request_snapshot.context_kind,
            ChatRequestContextKind::MultiTurn
        );
        assert!(request_snapshot.has_context);
        assert!(!request_snapshot.is_single_turn);
        assert_eq!(request_snapshot.max_tokens_label, "backend-default");
        assert!(!request_snapshot.wire_sends_max_tokens);
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.partial_answer(), "");
        assert_eq!(session.last_error(), None);
        assert_eq!(request_snapshot.last_user_chars, 13);
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn starting_enter_records_user_emits_start_and_preserves_routing() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(8192)),
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_quality()
                .with_model_role(ModelRole::Assistant),
        );
        for ch in "deep answer".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let action = input.handle_key_starting(KeyInput::Enter, &mut session);

        let InputAction::StartStream(turn) = action else {
            panic!("expected start-stream action");
        };
        assert_eq!(turn.request.messages.last().unwrap().content, "deep answer");
        assert_eq!(turn.request.max_tokens, Some(8192));
        assert_eq!(
            turn.request.routing_preference,
            RoutingPreference::PreferQuality
        );
        assert_eq!(turn.request.model_endpoint, None);
        assert_eq!(turn.start.state, StreamState::Streaming);
        assert_eq!(turn.start.sequence, 0);
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.history().last().unwrap().content, "deep answer");
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn starting_enter_recovers_after_interrupted_or_failed_session() {
        let mut interrupted = ChatSession::new("cli", ChatSessionConfig::default());
        interrupted
            .try_submit_and_begin_stream("hello")
            .expect("expected first stream");
        interrupted.push_delta("partial");
        interrupted.interrupt("backend stream closed");
        let mut input = CliInput::default();
        for ch in "next".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut interrupted);
        }

        let action = input.handle_key_starting(KeyInput::Enter, &mut interrupted);

        let InputAction::StartStream(turn) = action else {
            panic!("expected start-stream action after interrupted session");
        };
        let request_contents = turn
            .request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(request_contents, vec!["hello", "next"]);
        assert_eq!(turn.start.state, StreamState::Streaming);
        assert_eq!(interrupted.state(), StreamState::Streaming);
        assert_eq!(interrupted.partial_answer(), "");
        assert_eq!(interrupted.last_error(), None);
        assert!(input.buffer().is_empty());

        let mut failed = ChatSession::new("cli", ChatSessionConfig::default());
        failed.begin_stream();
        failed.fail("safe-device gate failed");
        let mut retry_input = CliInput::default();
        for ch in "repair and retry".chars() {
            retry_input.handle_key_starting(KeyInput::Char(ch), &mut failed);
        }

        let action = retry_input.handle_key_starting(KeyInput::Enter, &mut failed);

        let InputAction::StartStream(turn) = action else {
            panic!("expected start-stream action after failed session");
        };
        assert_eq!(
            turn.request.messages.last().unwrap().content,
            "repair and retry"
        );
        assert_eq!(failed.state(), StreamState::Streaming);
        assert_eq!(failed.last_error(), None);
        assert!(retry_input.buffer().is_empty());
    }

    #[test]
    fn starting_enter_blocks_active_stream_without_recording_second_user() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("first")
            .expect("expected first turn");
        let mut input = CliInput::default();
        for ch in "second".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let action = input.handle_key_starting(KeyInput::Enter, &mut session);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Busy);
        assert_eq!(chunk.content, "session stream is already active");
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "first");
        assert_eq!(input.buffer(), "second");
    }

    #[test]
    fn starting_enter_respects_frontend_gate_before_recording_user() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = FrontendGateSnapshot {
            engine_busy: true,
            active_request: Some("#11 active".to_owned()),
            ..FrontendGateSnapshot::default()
        };
        let mut input = CliInput::default();
        for ch in "hello".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let action = input.handle_key_with_gate_and_start(KeyInput::Enter, &mut session, &gate);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Busy);
        assert_eq!(chunk.content, "backend engine is busy: #11 active");
        assert!(session.history().is_empty());
        assert_eq!(session.state(), StreamState::Pending);
        assert_eq!(input.buffer(), "hello");
    }

    #[test]
    fn starting_model_pool_gate_allows_auto_route_and_emits_start() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("quality".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "quick review".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);

        let InputAction::StartStream(turn) = action else {
            panic!("expected start-stream action");
        };
        assert_eq!(turn.request.model_role, ModelRole::Reviewer);
        assert_eq!(
            turn.request.routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(turn.request.model_endpoint, None);
        assert_eq!(turn.start.state, StreamState::Streaming);
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn starting_model_pool_gate_preserves_context_limits_and_unpinned_route() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::new(8).with_default_max_tokens(Some(8192)),
        );
        session.record_user("first question");
        session.record_assistant("first answer");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("quality".to_owned()))
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "review second patch".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);

        let InputAction::StartStream(turn) = action else {
            panic!("expected start-stream action");
        };
        let request_contents = turn
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
            vec!["first question", "first answer", "review second patch"]
        );
        assert_eq!(history_contents, request_contents);
        assert_eq!(turn.request.max_tokens, Some(8192));
        assert_eq!(turn.request.model_role, ModelRole::Reviewer);
        assert_eq!(
            turn.request.routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(turn.request.model_endpoint, None);
        assert!(!turn.request.endpoint_pinned());
        assert_eq!(turn.start.state, StreamState::Streaming);
        assert_eq!(session.state(), StreamState::Streaming);
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn starting_model_pool_gate_prefers_active_session_over_route_queue() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("first")
            .expect("expected active stream");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#18 review".to_owned())),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .prefer_fast()
                .with_model_role(ModelRole::Reviewer),
        );
        for ch in "second".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Busy);
        assert_eq!(chunk.content, "session stream is already active");
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "first");
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(input.buffer(), "second");
    }

    #[test]
    fn starting_model_pool_gate_keeps_repair_gate_over_active_session() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("first")
            .expect("expected active stream");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                safe_device_ok: false,
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        let mut input = CliInput::default();
        for ch in "second".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);
        let snapshot = input.action_snapshot(&action);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Failed);
        assert_eq!(chunk.content, "safe-device gate failed");
        assert_eq!(snapshot.kind, InputActionKind::Blocked);
        assert_eq!(snapshot.kind_label, "blocked");
        assert_eq!(snapshot.request, None);
        assert_eq!(snapshot.start_chunk, None);
        assert_eq!(snapshot.start_state, None);
        assert_eq!(snapshot.stream_state, Some(StreamState::Failed));
        assert_eq!(snapshot.stream_state_label.as_deref(), Some("failed"));
        assert_eq!(snapshot.stream_state_is_terminal, Some(true));
        assert_eq!(snapshot.stream_state_is_pressure, Some(false));
        assert_eq!(snapshot.stream_state_blocks_prompt_submit, Some(false));
        let stream_chunk = snapshot
            .stream_chunk
            .as_ref()
            .expect("repair-gate blocked action should expose display snapshot");
        assert_eq!(stream_chunk.output_label, "error");
        assert_eq!(stream_chunk.appended, "[error] safe-device gate failed");
        assert_eq!(snapshot.reason.as_deref(), Some("safe-device gate failed"));
        assert_eq!(snapshot.local_status, None);
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0].content, "first");
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.partial_answer(), "");
        assert_eq!(input.buffer(), "second");
    }

    #[test]
    fn starting_model_pool_gate_blocks_pinned_busy_worker_without_start() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_busy(true, Some("#12 review".to_owned())),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer)),
        );
        for ch in "review".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Busy);
        assert_eq!(chunk.content, "worker fast-reviewer is busy: #12 review");
        assert!(session.history().is_empty());
        assert_eq!(session.chunks().len(), 0);
        assert_eq!(input.buffer(), "review");
    }

    #[test]
    fn starting_model_pool_gate_blocks_pinned_capability_mismatch_without_history() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Assistant)
                .with_routing_preference(RoutingPreference::PreferQuality)
                .with_model_endpoint(Some(ModelEndpoint::FastReviewer)),
        );
        for ch in "deep answer".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let action =
            input.handle_key_with_model_pool_gate_and_start(KeyInput::Enter, &mut session, &gate);

        let InputAction::Blocked(chunk) = action else {
            panic!("expected blocked action");
        };
        assert_eq!(chunk.state, StreamState::Queued);
        assert_eq!(
            chunk.content,
            "worker fast-reviewer does not match role=assistant preference=prefer_quality"
        );
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
        assert_eq!(session.state(), StreamState::Pending);
        assert_eq!(input.buffer(), "deep answer");
    }

    #[test]
    fn input_action_snapshot_exposes_send_boundary_without_prompt_text() {
        let session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(8192)),
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "sensitive patch details".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);
        let snapshot = input.action_snapshot(&action);

        assert_eq!(snapshot.kind, InputActionKind::Send);
        assert_eq!(snapshot.kind_label, "send");
        assert_eq!(snapshot.routing_intent.model_role, ModelRole::Reviewer);
        assert_eq!(
            snapshot.routing_intent.routing_preference,
            RoutingPreference::PreferFast
        );
        assert!(!snapshot.routing_intent.endpoint_pinned);
        let request = snapshot
            .request
            .expect("send should expose request metadata");
        assert_eq!(request.messages, 1);
        assert_eq!(request.context_messages, 0);
        assert_eq!(request.history_messages, 0);
        assert_eq!(request.context_kind, ChatRequestContextKind::SingleTurn);
        assert_eq!(request.context_kind_label, "single_turn");
        assert_eq!(request.last_user_chars, 23);
        assert_eq!(request.max_tokens, Some(8192));
        assert_eq!(request.model_role_label, "reviewer");
        assert_eq!(request.routing_preference_label, "prefer_fast");
        assert_eq!(request.endpoint_label, "auto");
        assert!(!request.endpoint_pinned);
        assert_eq!(request.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(request.endpoint_kind_label, "auto");
        assert!(request.endpoint_auto);
        assert!(!request.endpoint_built_in);
        assert!(!request.endpoint_custom);
        assert_eq!(request.routing_intent.model_role, ModelRole::Reviewer);
        assert_eq!(
            request.routing_intent.routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(snapshot.local_status, None);
        assert_eq!(snapshot.reason, None);
    }

    #[test]
    fn input_action_snapshot_request_route_wins_over_stale_render_config() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let stale_config = CliInputConfig::default();
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferQuality)
                .with_model_endpoint(Some(ModelEndpoint::Worker(
                    "mlx-reviewer-quality".to_owned(),
                ))),
        );
        for ch in "review route snapshot".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);
        let snapshot = action.snapshot(&stale_config);

        assert_eq!(snapshot.kind, InputActionKind::Send);
        assert_eq!(snapshot.model_role_label, "reviewer");
        assert_eq!(snapshot.routing_preference_label, "prefer_quality");
        assert_eq!(snapshot.endpoint_label, "mlx-reviewer-quality");
        assert!(snapshot.endpoint_pinned);
        assert_eq!(snapshot.endpoint_kind, ModelEndpointSelectionKind::Custom);
        assert_eq!(snapshot.endpoint_kind_label, "custom");
        assert!(!snapshot.endpoint_auto);
        assert!(!snapshot.endpoint_built_in);
        assert!(snapshot.endpoint_custom);
        assert_eq!(snapshot.wire_model_role_label, "reviewer");
        assert_eq!(snapshot.wire_routing_preference_label, "prefer_quality");
        assert!(!snapshot.wire_prefer_fast);
        assert!(snapshot.wire_prefer_quality);
        assert!(snapshot.wire_endpoint_pinned);
        assert_eq!(snapshot.wire_endpoint_kind_label, "custom");
        assert!(snapshot.wire_sends_model_endpoint);
        assert_eq!(
            snapshot.wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-quality")
        );

        let request = snapshot
            .request
            .as_ref()
            .expect("send action should carry the request route");
        assert_eq!(snapshot.routing_intent, request.routing_intent);
        assert_eq!(request.model_role_label, "reviewer");
        assert_eq!(request.routing_preference_label, "prefer_quality");
        assert_eq!(request.endpoint_label, "mlx-reviewer-quality");
        assert!(request.endpoint_pinned);
        assert!(request.wire_sends_model_endpoint);
        assert_eq!(
            request.wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-quality")
        );

        let mut active = ChatSession::new("cli", ChatSessionConfig::default());
        active
            .try_submit_and_begin_stream("first")
            .expect("expected active stream");
        let mut blocked_input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Tester)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "run tests".chars() {
            blocked_input.handle_key(KeyInput::Char(ch), &active);
        }
        let blocked = blocked_input.handle_key(KeyInput::Enter, &active);
        let blocked_snapshot = blocked_input.action_snapshot(&blocked);

        assert_eq!(blocked_snapshot.kind, InputActionKind::Blocked);
        assert_eq!(blocked_snapshot.request, None);
        assert_eq!(blocked_snapshot.stream_state, Some(StreamState::Busy));
        assert_eq!(blocked_snapshot.model_role_label, "tester");
        assert_eq!(blocked_snapshot.routing_preference_label, "prefer_fast");
        assert_eq!(blocked_snapshot.endpoint_label, "auto");
        assert!(!blocked_snapshot.endpoint_pinned);
        assert!(!blocked_snapshot.wire_sends_model_endpoint);
    }

    #[test]
    fn input_action_snapshot_exposes_start_and_block_states_for_ui() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "hello".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let started = input.handle_key_starting(KeyInput::Enter, &mut session);
        let started_snapshot = input.action_snapshot(&started);

        assert_eq!(started_snapshot.kind, InputActionKind::StartStream);
        assert_eq!(started_snapshot.kind_label, "start_stream");
        assert_eq!(started_snapshot.start_sequence, Some(0));
        assert_eq!(started_snapshot.start_state, Some(StreamState::Streaming));
        assert_eq!(
            started_snapshot.start_state_label.as_deref(),
            Some("streaming")
        );
        assert_eq!(started_snapshot.start_state_is_terminal, Some(false));
        assert_eq!(started_snapshot.start_state_is_pressure, Some(false));
        assert_eq!(
            started_snapshot.start_state_blocks_prompt_submit,
            Some(true)
        );
        let start_chunk = started_snapshot
            .start_chunk
            .as_ref()
            .expect("start stream action should carry service chunk display snapshot");
        assert_eq!(start_chunk.sequence, 0);
        assert_eq!(start_chunk.kind_label, "start");
        assert_eq!(start_chunk.state_label, "streaming");
        assert_eq!(start_chunk.output_label, "start");
        assert!(!start_chunk.emits_output);
        assert!(start_chunk.state_blocks_prompt_submit);
        assert_eq!(started_snapshot.stream_chunk, None);
        assert_eq!(started_snapshot.stream_state_is_terminal, None);
        assert_eq!(started_snapshot.stream_state_is_pressure, None);
        assert_eq!(started_snapshot.stream_state_blocks_prompt_submit, None);
        assert_eq!(
            started_snapshot
                .request
                .as_ref()
                .map(|request| request.last_user_chars),
            Some(5)
        );

        for ch in "second".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }
        let blocked = input.handle_key_starting(KeyInput::Enter, &mut session);
        let blocked_snapshot = input.action_snapshot(&blocked);

        assert_eq!(blocked_snapshot.kind, InputActionKind::Blocked);
        assert_eq!(blocked_snapshot.kind_label, "blocked");
        assert_eq!(blocked_snapshot.stream_state, Some(StreamState::Busy));
        assert_eq!(blocked_snapshot.stream_state_label.as_deref(), Some("busy"));
        assert_eq!(blocked_snapshot.stream_state_is_terminal, Some(false));
        assert_eq!(blocked_snapshot.stream_state_is_pressure, Some(true));
        assert_eq!(
            blocked_snapshot.stream_state_blocks_prompt_submit,
            Some(true)
        );
        let stream_chunk = blocked_snapshot
            .stream_chunk
            .as_ref()
            .expect("blocked action should carry service chunk display snapshot");
        assert_eq!(stream_chunk.kind_label, "status");
        assert_eq!(stream_chunk.state_label, "busy");
        assert_eq!(stream_chunk.output_label, "busy");
        assert_eq!(
            stream_chunk.appended,
            "[busy] session stream is already active"
        );
        assert!(stream_chunk.state_is_pressure);
        assert!(stream_chunk.state_blocks_prompt_submit);
        assert_eq!(blocked_snapshot.start_chunk, None);
        assert_eq!(blocked_snapshot.start_state_is_terminal, None);
        assert_eq!(blocked_snapshot.start_state_is_pressure, None);
        assert_eq!(blocked_snapshot.start_state_blocks_prompt_submit, None);
        assert_eq!(
            blocked_snapshot.reason.as_deref(),
            Some("session stream is already active")
        );
        assert_eq!(blocked_snapshot.request, None);
        assert_eq!(input.buffer(), "second");
    }

    #[test]
    fn input_action_snapshot_exposes_local_route_config_and_error_actions() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/model reviewer fast".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let route = input.handle_key_recording(KeyInput::Enter, &mut session);
        let route_snapshot = input.action_snapshot(&route);

        assert_eq!(route_snapshot.kind, InputActionKind::RoutingChanged);
        assert_eq!(route_snapshot.kind_label, "routing_changed");
        assert_eq!(
            route_snapshot.routing_intent.model_role,
            ModelRole::Reviewer
        );
        assert_eq!(
            route_snapshot.routing_intent.routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(route_snapshot.model_role_label, "reviewer");
        assert_eq!(route_snapshot.routing_preference_label, "prefer_fast");
        assert_eq!(route_snapshot.endpoint_label, "auto");
        assert!(!route_snapshot.endpoint_pinned);
        assert_eq!(
            route_snapshot.endpoint_kind,
            ModelEndpointSelectionKind::Auto
        );
        assert_eq!(route_snapshot.endpoint_kind_label, "auto");
        assert!(route_snapshot.endpoint_auto);
        assert!(!route_snapshot.endpoint_built_in);
        assert!(!route_snapshot.endpoint_custom);
        assert_eq!(route_snapshot.wire_model_role_label, "reviewer");
        assert_eq!(route_snapshot.wire_routing_preference_label, "prefer_fast");
        assert!(route_snapshot.wire_prefer_fast);
        assert!(!route_snapshot.wire_prefer_quality);
        assert!(!route_snapshot.wire_endpoint_pinned);
        assert_eq!(route_snapshot.wire_endpoint_kind_label, "auto");
        assert!(!route_snapshot.wire_sends_model_endpoint);
        assert_eq!(route_snapshot.wire_model_endpoint_label, None);
        assert_eq!(
            route_snapshot.local_status.as_deref(),
            Some("role=reviewer preference=prefer_fast endpoint=auto pinned=false")
        );

        let pin = input.select_model_endpoint_label("fast-reviewer");
        let pin_snapshot = input.action_snapshot(&pin);

        assert_eq!(pin_snapshot.kind, InputActionKind::RoutingChanged);
        assert_eq!(pin_snapshot.endpoint_label, "fast-reviewer");
        assert!(pin_snapshot.endpoint_pinned);
        assert_eq!(
            pin_snapshot.endpoint_kind,
            ModelEndpointSelectionKind::BuiltIn
        );
        assert_eq!(pin_snapshot.endpoint_kind_label, "built_in");
        assert!(!pin_snapshot.endpoint_auto);
        assert!(pin_snapshot.endpoint_built_in);
        assert!(!pin_snapshot.endpoint_custom);
        assert_eq!(pin_snapshot.wire_model_role_label, "reviewer");
        assert_eq!(pin_snapshot.wire_routing_preference_label, "prefer_fast");
        assert!(pin_snapshot.wire_prefer_fast);
        assert!(!pin_snapshot.wire_prefer_quality);
        assert!(pin_snapshot.wire_endpoint_pinned);
        assert_eq!(pin_snapshot.wire_endpoint_kind_label, "built_in");
        assert!(pin_snapshot.wire_sends_model_endpoint);
        assert_eq!(
            pin_snapshot.wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );

        for ch in "/max-tokens auto".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }
        let config = input.handle_key_recording(KeyInput::Enter, &mut session);
        let config_snapshot = input.action_snapshot(&config);

        assert_eq!(config_snapshot.kind, InputActionKind::SessionConfigChanged);
        assert_eq!(config_snapshot.kind_label, "session_config_changed");
        assert_eq!(
            config_snapshot.session_config_update,
            Some(SessionConfigUpdate::DefaultMaxTokens(None))
        );
        let config_update = config_snapshot
            .session_config_update_detail
            .as_ref()
            .expect("session config action should expose structured update");
        assert_eq!(config_update.kind_label, "max_tokens");
        assert_eq!(config_update.summary, "max_tokens=backend-default");
        assert!(config_update.changes_max_tokens);
        assert!(!config_update.changes_history_limit);
        assert_eq!(config_update.max_tokens, None);
        assert_eq!(
            config_update.max_tokens_label.as_deref(),
            Some("backend-default")
        );
        assert!(config_update.max_tokens_backend_default);
        assert_eq!(config_update.history_limit, None);
        assert_eq!(
            config_snapshot.local_status.as_deref(),
            Some("max_tokens=backend-default")
        );

        for ch in "/worker".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }
        let error = input.handle_key_recording(KeyInput::Enter, &mut session);
        let error_snapshot = input.action_snapshot(&error);

        assert_eq!(error_snapshot.kind, InputActionKind::InputError);
        assert_eq!(error_snapshot.kind_label, "input_error");
        assert_eq!(
            error_snapshot.local_status.as_deref(),
            Some("missing model endpoint")
        );
        assert_eq!(error_snapshot.request, None);
    }

    #[test]
    fn readiness_snapshot_classifies_empty_prompt_and_local_commands_without_mutation() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();

        let empty = input.readiness(&session);

        assert_eq!(empty.buffer_kind, InputBufferKind::Empty);
        assert_eq!(empty.buffer_kind_label, "empty");
        assert_eq!(empty.enter_action, InputActionKind::Noop);
        assert_eq!(empty.enter_action_label, "noop");
        assert!(!empty.enter_enabled);
        assert!(!empty.enter_submits_prompt);
        assert!(!empty.enter_runs_local_command);
        assert!(!empty.enter_is_blocked);
        assert_eq!(empty.primary_action_label, "type_prompt");
        assert!(!empty.primary_action_enabled);
        assert_eq!(
            empty.primary_action_disabled_reason.as_deref(),
            Some("empty input")
        );
        assert!(!empty.send_allowed);
        let empty_control = empty
            .prompt_submit_control
            .as_ref()
            .expect("empty input should expose disabled prompt submit control");
        assert!(!empty_control.prompt_present);
        assert!(!empty_control.send_allowed);
        assert_eq!(empty_control.primary_action_label, "type_prompt");
        assert_eq!(
            empty_control.primary_action_disabled_reason.as_deref(),
            Some("empty input")
        );

        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let status = input.readiness(&session);

        assert_eq!(status.buffer_kind, InputBufferKind::StatusCommand);
        assert_eq!(status.buffer_kind_label, "status_command");
        assert_eq!(status.enter_action, InputActionKind::Status);
        assert_eq!(status.enter_action_label, "status");
        assert_eq!(
            status
                .command_preview
                .as_ref()
                .map(|preview| preview.enter_action),
            Some(InputActionKind::Status)
        );
        assert_eq!(
            status
                .command_preview
                .as_ref()
                .map(|preview| preview.enter_action_label.as_str()),
            Some("status")
        );
        assert!(status.enter_enabled);
        assert!(!status.enter_submits_prompt);
        assert!(status.enter_runs_local_command);
        assert!(!status.enter_is_blocked);
        assert_eq!(status.primary_action_label, "show_status");
        assert!(status.primary_action_enabled);
        assert_eq!(status.primary_action_disabled_reason, None);
        assert!(!status.send_allowed);
        assert_eq!(status.prompt_submit_control, None);
        assert_eq!(input.buffer(), "/status");

        input.clear();
        for ch in "/max-tokens auto".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let config = input.readiness(&session);

        assert_eq!(config.buffer_kind, InputBufferKind::SessionConfigCommand);
        assert_eq!(config.buffer_kind_label, "session_config_command");
        assert_eq!(config.enter_action, InputActionKind::SessionConfigChanged);
        assert_eq!(config.enter_action_label, "session_config_changed");
        assert!(!config.enter_submits_prompt);
        assert!(config.enter_runs_local_command);
        assert!(!config.enter_is_blocked);
        assert_eq!(config.primary_action_label, "apply_config");
        assert!(config.primary_action_enabled);
        assert_eq!(config.primary_action_disabled_reason, None);
        assert_eq!(
            config
                .command_preview
                .as_ref()
                .and_then(|preview| preview.session_config_update.clone()),
            Some(SessionConfigUpdate::DefaultMaxTokens(None))
        );
        assert_eq!(input.buffer(), "/max-tokens auto");

        input.clear();
        for ch in "/worker".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let invalid = input.readiness(&session);

        assert_eq!(invalid.buffer_kind, InputBufferKind::InvalidCommand);
        assert_eq!(invalid.buffer_kind_label, "invalid_command");
        assert_eq!(invalid.enter_action, InputActionKind::InputError);
        assert_eq!(invalid.enter_action_label, "input_error");
        assert!(!invalid.enter_submits_prompt);
        assert!(!invalid.enter_runs_local_command);
        assert!(!invalid.enter_is_blocked);
        assert_eq!(invalid.primary_action_label, "fix_command");
        assert!(!invalid.primary_action_enabled);
        assert_eq!(
            invalid.primary_action_disabled_reason.as_deref(),
            Some("missing model endpoint")
        );
        assert_eq!(
            invalid
                .command_preview
                .as_ref()
                .and_then(|preview| preview.error.as_deref()),
            Some("missing model endpoint")
        );
        assert_eq!(invalid.block_state, None);
    }

    #[test]
    fn readiness_command_preview_exposes_route_updates_without_mutation() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/model reviewer fast".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let model = input.readiness(&session);

        let model_preview = model
            .command_preview
            .as_ref()
            .expect("model command should have preview");
        assert_eq!(model.buffer_kind, InputBufferKind::RoutingCommand);
        assert_eq!(model.buffer_kind_label, "routing_command");
        assert_eq!(model.enter_action, InputActionKind::RoutingChanged);
        assert_eq!(model.enter_action_label, "routing_changed");
        assert_eq!(model_preview.buffer_kind_label, "routing_command");
        assert_eq!(model_preview.enter_action_label, "routing_changed");
        let intent = model_preview
            .routing_intent
            .as_ref()
            .expect("routing command should preview intent");
        assert_eq!(intent.model_role, ModelRole::Reviewer);
        assert_eq!(intent.routing_preference, RoutingPreference::PreferFast);
        assert_eq!(intent.endpoint_label(), "auto");
        assert!(!intent.endpoint_pinned);
        assert_eq!(
            model_preview.routing_summary.as_deref(),
            Some("role=reviewer preference=prefer_fast endpoint=auto pinned=false")
        );
        assert_eq!(model_preview.model_role_label.as_deref(), Some("reviewer"));
        assert_eq!(
            model_preview.routing_preference_label.as_deref(),
            Some("prefer_fast")
        );
        assert_eq!(model_preview.endpoint_label.as_deref(), Some("auto"));
        assert_eq!(model_preview.endpoint_pinned, Some(false));
        assert_eq!(
            model_preview.endpoint_kind,
            Some(ModelEndpointSelectionKind::Auto)
        );
        assert_eq!(model_preview.endpoint_kind_label.as_deref(), Some("auto"));
        assert_eq!(model_preview.endpoint_auto, Some(true));
        assert_eq!(model_preview.endpoint_built_in, Some(false));
        assert_eq!(model_preview.endpoint_custom, Some(false));
        assert_eq!(
            model_preview.wire_model_role_label.as_deref(),
            Some("reviewer")
        );
        assert_eq!(
            model_preview.wire_routing_preference_label.as_deref(),
            Some("prefer_fast")
        );
        assert_eq!(model_preview.wire_prefer_fast, Some(true));
        assert_eq!(model_preview.wire_prefer_quality, Some(false));
        assert_eq!(model_preview.wire_endpoint_pinned, Some(false));
        assert_eq!(
            model_preview.wire_endpoint_kind_label.as_deref(),
            Some("auto")
        );
        assert_eq!(model_preview.wire_sends_model_endpoint, Some(false));
        assert_eq!(model_preview.wire_model_endpoint_label, None);
        assert_eq!(
            model_preview.local_status.as_deref(),
            Some("role=reviewer preference=prefer_fast endpoint=auto pinned=false")
        );
        assert_eq!(input.config().model_role, ModelRole::Assistant);
        assert_eq!(input.buffer(), "/model reviewer fast");

        input.clear();
        for ch in "/endpoint fast-reviewer".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let endpoint = input.readiness(&session);
        let endpoint_intent = endpoint
            .command_preview
            .as_ref()
            .and_then(|preview| preview.routing_intent.as_ref())
            .expect("endpoint command should preview route intent");

        assert_eq!(endpoint_intent.endpoint_label(), "fast-reviewer");
        assert!(endpoint_intent.endpoint_pinned);
        let endpoint_preview = endpoint
            .command_preview
            .as_ref()
            .expect("endpoint command should preview route");
        assert_eq!(endpoint_preview.wire_endpoint_pinned, Some(true));
        assert_eq!(
            endpoint_preview.wire_endpoint_kind_label.as_deref(),
            Some("built_in")
        );
        assert_eq!(endpoint_preview.wire_sends_model_endpoint, Some(true));
        assert_eq!(
            endpoint_preview.wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );
        assert_eq!(
            endpoint
                .command_preview
                .as_ref()
                .and_then(|preview| preview.local_status.as_deref()),
            Some("role=assistant preference=balanced endpoint=fast-reviewer pinned=true")
        );
        assert_eq!(input.config().model_endpoint, None);

        let mut pinned_input = CliInput::new(
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer)),
        );
        for ch in "/model reviewer fast auto".chars() {
            pinned_input.handle_key(KeyInput::Char(ch), &session);
        }
        let auto_clear = pinned_input.readiness(&session);
        let auto_clear_preview = auto_clear
            .command_preview
            .as_ref()
            .expect("auto endpoint should preview clearing the pin");

        assert_eq!(auto_clear_preview.endpoint_label.as_deref(), Some("auto"));
        assert_eq!(auto_clear_preview.endpoint_pinned, Some(false));
        assert_eq!(auto_clear_preview.wire_endpoint_pinned, Some(false));
        assert_eq!(
            auto_clear_preview.wire_endpoint_kind_label.as_deref(),
            Some("auto")
        );
        assert_eq!(auto_clear_preview.wire_sends_model_endpoint, Some(false));
        assert_eq!(auto_clear_preview.wire_model_endpoint_label, None);
        assert_eq!(
            auto_clear_preview.local_status.as_deref(),
            Some("role=reviewer preference=prefer_fast endpoint=auto pinned=false")
        );
        assert_eq!(
            pinned_input.config().model_endpoint,
            Some(ModelEndpoint::FastReviewer)
        );
    }

    #[test]
    fn readiness_command_preview_reports_config_updates_and_errors() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "/history-limit 16".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let history = input.readiness(&session);
        let preview = history
            .command_preview
            .as_ref()
            .expect("history command should have preview");

        assert_eq!(preview.buffer_kind, InputBufferKind::SessionConfigCommand);
        assert_eq!(preview.buffer_kind_label, "session_config_command");
        assert_eq!(preview.enter_action_label, "session_config_changed");
        assert_eq!(
            preview.session_config_update,
            Some(SessionConfigUpdate::HistoryLimit(16))
        );
        let update = preview
            .session_config_update_detail
            .as_ref()
            .expect("session config preview should expose structured update");
        assert_eq!(update.kind_label, "history_limit");
        assert_eq!(update.summary, "history_limit=16");
        assert!(!update.changes_max_tokens);
        assert!(update.changes_history_limit);
        assert_eq!(update.max_tokens, None);
        assert_eq!(update.max_tokens_label, None);
        assert!(!update.max_tokens_backend_default);
        assert_eq!(update.history_limit, Some(16));
        assert_eq!(preview.local_status.as_deref(), Some("history_limit=16"));

        input.clear();
        for ch in "/model reviewer fast fast-reviewer extra".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let error = input.readiness(&session);
        let error_preview = error
            .command_preview
            .as_ref()
            .expect("invalid command should have preview");

        assert_eq!(error.buffer_kind, InputBufferKind::InvalidCommand);
        assert_eq!(error.buffer_kind_label, "invalid_command");
        assert_eq!(error.enter_action, InputActionKind::InputError);
        assert_eq!(error.enter_action_label, "input_error");
        assert_eq!(error_preview.buffer_kind_label, "invalid_command");
        assert_eq!(error_preview.enter_action_label, "input_error");
        assert_eq!(
            error_preview.error.as_deref(),
            Some("unexpected model command argument: extra")
        );
        assert_eq!(error_preview.local_status, None);
        assert_eq!(error_preview.routing_intent, None);
        assert_eq!(error_preview.wire_model_role_label, None);
        assert_eq!(error_preview.wire_routing_preference_label, None);
        assert_eq!(error_preview.wire_prefer_fast, None);
        assert_eq!(error_preview.wire_prefer_quality, None);
        assert_eq!(error_preview.wire_endpoint_pinned, None);
        assert_eq!(error_preview.wire_endpoint_kind_label, None);
        assert_eq!(error_preview.wire_sends_model_endpoint, None);
        assert_eq!(error_preview.wire_model_endpoint_label, None);
    }

    #[test]
    fn slash_command_execution_reuses_readiness_preview_contract() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::new(64));
        let mut input = CliInput::new(
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer)),
        );
        for ch in "/model reviewer fast auto".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }

        let preview = input
            .readiness_recording(&session)
            .command_preview
            .expect("model command should preview route");
        let action = input.handle_key_recording(KeyInput::Enter, &mut session);

        assert_eq!(
            action,
            InputAction::RoutingChanged(preview.local_status.unwrap())
        );
        assert_eq!(input.config().model_role, ModelRole::Reviewer);
        assert_eq!(
            input.config().routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(input.config().model_endpoint, None);
        assert!(session.history().is_empty());

        for ch in "/history-limit 8".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }
        let preview = input
            .readiness_recording(&session)
            .command_preview
            .expect("history command should preview config");
        let action = input.handle_key_recording(KeyInput::Enter, &mut session);

        assert_eq!(
            preview.session_config_update,
            Some(SessionConfigUpdate::HistoryLimit(8))
        );
        assert_eq!(
            action,
            InputAction::SessionConfigChanged {
                update: SessionConfigUpdate::HistoryLimit(8),
                summary: "history_limit=8".to_owned()
            }
        );
        assert_eq!(session.config().history_limit, 8);

        for ch in "/model reviewer fast fast-reviewer extra".chars() {
            input.handle_key_recording(KeyInput::Char(ch), &mut session);
        }
        let preview = input
            .readiness_recording(&session)
            .command_preview
            .expect("invalid command should preview error");
        let action = input.handle_key_recording(KeyInput::Enter, &mut session);

        assert_eq!(
            preview.error.as_deref(),
            Some("unexpected model command argument: extra")
        );
        assert_eq!(
            action,
            InputAction::InputError("unexpected model command argument: extra".to_owned())
        );
    }

    #[test]
    fn readiness_snapshot_reports_prompt_send_and_session_pressure_without_prompt_text() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "review sensitive patch".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let ready = input.readiness(&session);

        assert_eq!(ready.buffer_kind, InputBufferKind::Prompt);
        assert_eq!(ready.submit_mode, InputSubmitMode::Preview);
        assert_eq!(ready.enter_action, InputActionKind::Send);
        assert!(ready.enter_enabled);
        assert!(ready.send_allowed);
        assert!(!ready.preserves_buffer_on_enter);
        assert!(ready.clears_buffer_on_enter);
        assert!(!ready.records_user_on_enter);
        assert!(!ready.starts_stream_on_enter);
        assert_eq!(ready.advice_action, Some(GateAdviceAction::SendNow));
        assert_eq!(ready.advice_action_label.as_deref(), Some("send_now"));
        let prompt_control = ready
            .prompt_submit_control
            .as_ref()
            .expect("prompt readiness should expose prompt submit control");
        assert!(prompt_control.prompt_present);
        assert!(prompt_control.send_allowed);
        assert_eq!(prompt_control.action, GateAdviceAction::SendNow);
        assert_eq!(prompt_control.action_label, "send_now");
        assert_eq!(prompt_control.primary_action_label, "send");
        assert!(prompt_control.primary_action_enabled);
        assert_eq!(prompt_control.primary_action_disabled_reason, None);
        assert_eq!(prompt_control.block_chunk, None);
        assert!(!prompt_control.preserves_prompt);
        assert!(prompt_control.clears_prompt);
        assert_eq!(ready.block_state_label, None);
        assert!(!ready.block_state_is_terminal);
        assert!(!ready.block_state_is_pressure);
        assert!(!ready.block_state_blocks_prompt_submit);
        assert_eq!(ready.block_chunk, None);
        assert_eq!(ready.trimmed_chars, 22);
        assert_eq!(ready.routing_intent.model_role, ModelRole::Reviewer);
        assert_eq!(
            ready.routing_intent.routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(ready.model_role_label, "reviewer");
        assert_eq!(ready.routing_preference_label, "prefer_fast");
        assert_eq!(ready.endpoint_label, "auto");
        assert!(!ready.endpoint_pinned);
        assert_eq!(ready.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(ready.endpoint_kind_label, "auto");
        assert!(ready.endpoint_auto);
        assert!(!ready.endpoint_built_in);
        assert!(!ready.endpoint_custom);
        assert_eq!(ready.wire_model_role_label, "reviewer");
        assert_eq!(ready.wire_routing_preference_label, "prefer_fast");
        assert!(ready.wire_prefer_fast);
        assert!(!ready.wire_prefer_quality);
        assert!(!ready.wire_endpoint_pinned);
        assert_eq!(ready.wire_endpoint_kind_label, "auto");
        assert!(!ready.wire_sends_model_endpoint);
        assert_eq!(ready.wire_model_endpoint_label, None);
        assert_eq!(ready.block_reason, None);

        let mut busy_session = ChatSession::new("cli", ChatSessionConfig::default());
        busy_session
            .try_submit_and_begin_stream("first")
            .expect("expected active stream");
        let blocked = input.readiness(&busy_session);

        assert_eq!(blocked.buffer_kind, InputBufferKind::Prompt);
        assert_eq!(blocked.enter_action, InputActionKind::Blocked);
        assert!(!blocked.send_allowed);
        assert!(blocked.preserves_buffer_on_enter);
        assert!(!blocked.clears_buffer_on_enter);
        assert!(!blocked.records_user_on_enter);
        assert!(!blocked.starts_stream_on_enter);
        assert_eq!(
            blocked.advice_action,
            Some(GateAdviceAction::WaitForCurrentStream)
        );
        assert_eq!(
            blocked.advice_action_label.as_deref(),
            Some("wait_for_current_stream")
        );
        assert_eq!(blocked.block_state, Some(StreamState::Busy));
        assert_eq!(blocked.block_state_label.as_deref(), Some("busy"));
        assert!(!blocked.block_state_is_terminal);
        assert!(blocked.block_state_is_pressure);
        assert!(blocked.block_state_blocks_prompt_submit);
        let block_chunk = blocked
            .block_chunk
            .as_ref()
            .expect("blocked readiness should carry service chunk display snapshot");
        assert_eq!(block_chunk.output_label, "busy");
        assert_eq!(block_chunk.state_label, "busy");
        assert_eq!(
            block_chunk.appended,
            "[busy] session stream is already active"
        );
        assert!(block_chunk.state_is_pressure);
        assert!(block_chunk.state_blocks_prompt_submit);
        assert_eq!(
            blocked.block_reason.as_deref(),
            Some("session stream is already active")
        );
        let blocked_control = blocked
            .prompt_submit_control
            .as_ref()
            .expect("blocked prompt should expose wait control");
        assert!(blocked_control.prompt_present);
        assert!(!blocked_control.send_allowed);
        assert_eq!(
            blocked_control.action,
            GateAdviceAction::WaitForCurrentStream
        );
        assert_eq!(blocked_control.action_label, "wait_for_current_stream");
        assert_eq!(blocked_control.state, StreamState::Busy);
        assert_eq!(blocked_control.state_label, "busy");
        assert!(blocked_control.state_is_pressure);
        assert!(blocked_control.state_blocks_prompt_submit);
        assert_eq!(
            blocked_control.primary_action_disabled_reason.as_deref(),
            Some("session stream is already active")
        );
        let control_block_chunk = blocked_control
            .block_chunk
            .as_ref()
            .expect("prompt submit control should expose display chunk");
        assert_eq!(control_block_chunk.output_label, "busy");
        assert_eq!(
            control_block_chunk.appended,
            "[busy] session stream is already active"
        );
        assert!(control_block_chunk.state_blocks_prompt_submit);
        assert!(blocked_control.preserves_prompt);
        assert!(!blocked_control.clears_prompt);
        assert_eq!(blocked.model_role_label, "reviewer");
        assert_eq!(blocked.routing_preference_label, "prefer_fast");
        assert_eq!(blocked.endpoint_label, "auto");
        assert!(!blocked.endpoint_pinned);
        assert_eq!(blocked.endpoint_kind_label, "auto");
        assert!(!blocked.wire_endpoint_pinned);
        assert!(!blocked.wire_sends_model_endpoint);
        assert_eq!(blocked.wire_model_endpoint_label, None);
        assert_eq!(input.buffer(), "review sensitive patch");
    }

    #[test]
    fn readiness_request_preview_preserves_context_tokens_and_auto_route() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(8192)),
        );
        session.record_user("first");
        session.record_assistant("answer");
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "review this patch".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let readiness = input.readiness_recording(&session);

        let request = readiness
            .request_preview
            .as_ref()
            .expect("prompt readiness should preview the outbound request");
        assert_eq!(request.messages, 3);
        assert_eq!(request.context_messages, 2);
        assert_eq!(request.messages, request.context_messages + 1);
        assert_eq!(request.history_messages, 2);
        assert_eq!(request.history_limit, Some(64));
        assert_eq!(request.history_remaining, Some(62));
        assert_eq!(request.history_messages_after_submit, Some(3));
        assert_eq!(request.history_at_limit_after_submit, Some(false));
        assert_eq!(request.history_truncates_on_submit, Some(false));
        assert_eq!(request.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(request.context_kind_label, "multi_turn");
        assert!(request.has_context);
        assert!(!request.is_single_turn);
        assert_eq!(request.last_message_role_label.as_deref(), Some("user"));
        assert_eq!(request.last_message_chars, 17);
        assert!(request.last_message_is_user);
        assert_eq!(request.last_user_chars, 17);
        assert_eq!(request.max_tokens, Some(8192));
        assert_eq!(request.max_tokens_label, "8192");
        assert_ne!(request.max_tokens, request.history_limit);
        assert!(request.stream);
        assert_eq!(request.model_role_label, "reviewer");
        assert_eq!(request.routing_preference_label, "prefer_fast");
        assert_eq!(request.endpoint_label, "auto");
        assert!(!request.endpoint_pinned);
        assert_eq!(request.wire_model_role_label, "reviewer");
        assert_eq!(request.wire_routing_preference_label, "prefer_fast");
        assert!(request.wire_prefer_fast);
        assert!(!request.wire_prefer_quality);
        assert!(request.wire_sends_max_tokens);
        assert_eq!(request.wire_max_tokens, Some(8192));
        assert_eq!(request.wire_endpoint_pinned, request.endpoint_pinned);
        assert_eq!(request.wire_endpoint_kind_label, "auto");
        assert!(!request.wire_sends_model_endpoint);
        assert_eq!(request.wire_model_endpoint_label, None);
        assert_eq!(request.routing_intent.model_role, ModelRole::Reviewer);
        assert_eq!(
            request.routing_intent.routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(request.routing_intent.endpoint_label(), "auto");
        assert!(!request.routing_intent.endpoint_pinned);
        assert_eq!(readiness.model_role_label, "reviewer");
        assert_eq!(readiness.routing_preference_label, "prefer_fast");
        assert_eq!(readiness.endpoint_label, "auto");
        assert!(!readiness.endpoint_pinned);
        assert_eq!(readiness.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(readiness.endpoint_kind_label, "auto");
        assert_eq!(readiness.wire_model_role_label, "reviewer");
        assert_eq!(readiness.wire_routing_preference_label, "prefer_fast");
        assert!(readiness.wire_prefer_fast);
        assert!(!readiness.wire_prefer_quality);
        assert!(!readiness.wire_endpoint_pinned);
        assert_eq!(readiness.wire_endpoint_kind_label, "auto");
        assert!(!readiness.wire_sends_model_endpoint);
        assert_eq!(readiness.wire_model_endpoint_label, None);
        assert_eq!(readiness.advice_action, Some(GateAdviceAction::SendNow));

        input.clear();
        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        assert_eq!(input.readiness_recording(&session).request_preview, None);
    }

    #[test]
    fn request_preview_exposes_history_limit_pressure_for_next_submit() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::new(2));
        session.record_user("one");
        session.record_assistant("two");
        let mut input = CliInput::default();
        for ch in "three".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let request = input
            .readiness_recording(&session)
            .request_preview
            .expect("prompt readiness should preview history policy");

        assert_eq!(request.messages, 3);
        assert_eq!(request.context_messages, 2);
        assert_eq!(request.messages, request.context_messages + 1);
        assert_eq!(request.history_messages, 2);
        assert_eq!(request.history_limit, Some(2));
        assert_eq!(request.history_remaining, Some(0));
        assert_eq!(request.history_messages_after_submit, Some(2));
        assert_eq!(request.history_at_limit_after_submit, Some(true));
        assert_eq!(request.history_truncates_on_submit, Some(true));
        assert_eq!(request.context_kind, ChatRequestContextKind::MultiTurn);
        assert!(request.has_context);
        assert!(!request.is_single_turn);
        assert_eq!(request.max_tokens, None);
        assert_eq!(request.max_tokens_label, "backend-default");
        assert!(!request.wire_sends_max_tokens);
        assert_eq!(request.wire_max_tokens, None);

        let send_action = InputAction::Send(ChatRequest::new(
            "cli",
            vec![norion_service::ChatMessage::user("standalone")],
        ));
        let send_snapshot = input.action_snapshot(&send_action);
        let action_request = send_snapshot
            .request
            .expect("standalone action snapshot should still carry request metadata");
        assert_eq!(action_request.history_limit, None);
        assert_eq!(action_request.history_remaining, None);
        assert_eq!(action_request.history_messages_after_submit, None);
        assert_eq!(action_request.history_at_limit_after_submit, None);
        assert_eq!(action_request.history_truncates_on_submit, None);
    }

    #[test]
    fn route_options_classify_endpoint_selection_without_guessing_labels() {
        let auto = InputRouteOptionsSnapshot::default();
        assert_eq!(auto.selected_endpoint_label, "auto");
        assert_eq!(
            auto.selected_endpoint_kind,
            ModelEndpointSelectionKind::Auto
        );
        assert_eq!(auto.selected_endpoint_kind_label, "auto");
        assert!(auto.selected_endpoint_auto);
        assert!(!auto.selected_endpoint_built_in);
        assert!(!auto.selected_endpoint_custom);
        assert!(!auto.endpoint_pinned);
        assert_eq!(auto.selected_wire_model_role_label, "assistant");
        assert_eq!(auto.selected_wire_routing_preference_label, "balanced");
        assert!(!auto.selected_wire_prefer_fast);
        assert!(!auto.selected_wire_prefer_quality);
        assert!(!auto.selected_wire_endpoint_pinned);
        assert_eq!(auto.selected_wire_endpoint_kind_label, "auto");
        assert!(!auto.selected_wire_sends_model_endpoint);
        assert_eq!(auto.selected_wire_model_endpoint_label, None);
        assert_eq!(auto.role_options.len(), 4);
        assert_eq!(
            auto.role_options
                .iter()
                .filter(|option| option.selected)
                .map(|option| option.role_label.as_str())
                .collect::<Vec<_>>(),
            vec!["assistant"]
        );
        let reviewer_option = auto
            .role_options
            .iter()
            .find(|option| option.role == ModelRole::Reviewer)
            .expect("reviewer role option should be present");
        assert_eq!(reviewer_option.role_label, "reviewer");
        assert_eq!(
            reviewer_option.selection_summary,
            "role=reviewer preference=balanced endpoint=auto pinned=false"
        );
        assert_eq!(reviewer_option.selection_wire_model_role_label, "reviewer");
        assert_eq!(
            reviewer_option.selection_wire_routing_preference_label,
            "balanced"
        );
        assert!(!reviewer_option.selection_wire_endpoint_pinned);
        assert!(!reviewer_option.selection_wire_sends_model_endpoint);
        assert_eq!(reviewer_option.selection_wire_model_endpoint_label, None);
        assert_eq!(auto.preference_options.len(), 3);
        assert_eq!(
            auto.preference_options
                .iter()
                .filter(|option| option.selected)
                .map(|option| option.preference_label.as_str())
                .collect::<Vec<_>>(),
            vec!["balanced"]
        );
        let fast_option = auto
            .preference_options
            .iter()
            .find(|option| option.preference == RoutingPreference::PreferFast)
            .expect("prefer-fast option should be present");
        assert_eq!(fast_option.preference_label, "prefer_fast");
        assert_eq!(
            fast_option.selection_summary,
            "role=assistant preference=prefer_fast endpoint=auto pinned=false"
        );
        assert_eq!(fast_option.selection_wire_model_role_label, "assistant");
        assert_eq!(
            fast_option.selection_wire_routing_preference_label,
            "prefer_fast"
        );
        assert!(fast_option.selection_wire_prefer_fast);
        assert!(!fast_option.selection_wire_prefer_quality);
        assert!(!fast_option.selection_wire_endpoint_pinned);
        assert!(!fast_option.selection_wire_sends_model_endpoint);
        assert_eq!(fast_option.selection_wire_model_endpoint_label, None);
        assert_eq!(auto.endpoint_options.len(), 4);
        assert_eq!(auto.endpoint_options[0].endpoint_label, "auto");
        assert!(auto.endpoint_options[0].selected);
        assert!(auto.endpoint_options[0].endpoint_auto);
        assert!(!auto.endpoint_options[0].endpoint_pinned);
        assert_eq!(
            auto.endpoint_options[0].selection_summary,
            "role=assistant preference=balanced endpoint=auto pinned=false"
        );
        assert!(!auto.endpoint_options[0].selection_wire_endpoint_pinned);
        assert!(!auto.endpoint_options[0].selection_wire_sends_model_endpoint);
        assert_eq!(auto.endpoint_options[1].endpoint_label, "quality-12b");
        assert!(!auto.endpoint_options[1].selected);
        assert!(auto.endpoint_options[1].endpoint_built_in);
        assert!(auto.endpoint_options[1].endpoint_pinned);
        assert_eq!(
            auto.endpoint_options[1].selection_summary,
            "role=assistant preference=balanced endpoint=quality-12b pinned=true"
        );
        assert!(auto.endpoint_options[1].selection_wire_endpoint_pinned);
        assert_eq!(
            auto.endpoint_options[1]
                .selection_wire_model_endpoint_label
                .as_deref(),
            Some("quality-12b")
        );
        assert!(auto.auto_endpoint_selected);
        assert_eq!(
            auto.auto_selection_summary,
            "role=assistant preference=balanced endpoint=auto pinned=false"
        );
        assert_eq!(auto.auto_selection_model_role_label, "assistant");
        assert_eq!(auto.auto_selection_routing_preference_label, "balanced");
        assert_eq!(auto.auto_selection_endpoint_label, "auto");
        assert_eq!(
            auto.auto_selection_endpoint_kind,
            ModelEndpointSelectionKind::Auto
        );
        assert_eq!(auto.auto_selection_endpoint_kind_label, "auto");
        assert!(auto.auto_selection_endpoint_auto);
        assert!(!auto.auto_selection_endpoint_built_in);
        assert!(!auto.auto_selection_endpoint_custom);
        assert_eq!(auto.auto_selection_wire_model_role_label, "assistant");
        assert_eq!(
            auto.auto_selection_wire_routing_preference_label,
            "balanced"
        );
        assert!(!auto.auto_selection_wire_prefer_fast);
        assert!(!auto.auto_selection_wire_prefer_quality);
        assert!(!auto.auto_selection_wire_endpoint_pinned);
        assert_eq!(auto.auto_selection_wire_endpoint_kind_label, "auto");
        assert!(!auto.auto_selection_wire_sends_model_endpoint);
        assert_eq!(auto.auto_selection_wire_model_endpoint_label, None);

        let built_in = InputRouteOptionsSnapshot::from_intent(&RoutingIntent::operator_pinned(
            ModelRole::Reviewer,
            RoutingPreference::PreferFast,
            ModelEndpoint::FastReviewer,
        ));
        assert_eq!(built_in.selected_endpoint_label, "fast-reviewer");
        assert_eq!(
            built_in.selected_endpoint_kind,
            ModelEndpointSelectionKind::BuiltIn
        );
        assert_eq!(built_in.selected_endpoint_kind_label, "built_in");
        assert!(!built_in.selected_endpoint_auto);
        assert!(built_in.selected_endpoint_built_in);
        assert!(!built_in.selected_endpoint_custom);
        assert!(built_in.endpoint_pinned);
        assert_eq!(built_in.selected_wire_model_role_label, "reviewer");
        assert_eq!(
            built_in.selected_wire_routing_preference_label,
            "prefer_fast"
        );
        assert!(built_in.selected_wire_prefer_fast);
        assert!(!built_in.selected_wire_prefer_quality);
        assert!(built_in.selected_wire_endpoint_pinned);
        assert_eq!(built_in.selected_wire_endpoint_kind_label, "built_in");
        assert!(built_in.selected_wire_sends_model_endpoint);
        assert_eq!(
            built_in.selected_wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );
        assert_eq!(
            built_in
                .role_options
                .iter()
                .filter(|option| option.selected)
                .map(|option| option.role_label.as_str())
                .collect::<Vec<_>>(),
            vec!["reviewer"]
        );
        let tester_option = built_in
            .role_options
            .iter()
            .find(|option| option.role == ModelRole::Tester)
            .expect("tester role option should be present");
        assert_eq!(
            tester_option.selection_summary,
            "role=tester preference=prefer_fast endpoint=fast-reviewer pinned=true"
        );
        assert_eq!(tester_option.selection_wire_model_role_label, "tester");
        assert!(tester_option.selection_wire_prefer_fast);
        assert!(tester_option.selection_wire_endpoint_pinned);
        assert_eq!(tester_option.selection_wire_endpoint_kind_label, "built_in");
        assert!(tester_option.selection_wire_sends_model_endpoint);
        assert_eq!(
            tester_option.selection_wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );
        assert_eq!(
            built_in
                .preference_options
                .iter()
                .filter(|option| option.selected)
                .map(|option| option.preference_label.as_str())
                .collect::<Vec<_>>(),
            vec!["prefer_fast"]
        );
        let quality_option = built_in
            .preference_options
            .iter()
            .find(|option| option.preference == RoutingPreference::PreferQuality)
            .expect("prefer-quality option should be present");
        assert_eq!(
            quality_option.selection_summary,
            "role=reviewer preference=prefer_quality endpoint=fast-reviewer pinned=true"
        );
        assert_eq!(quality_option.selection_wire_model_role_label, "reviewer");
        assert_eq!(
            quality_option.selection_wire_routing_preference_label,
            "prefer_quality"
        );
        assert!(!quality_option.selection_wire_prefer_fast);
        assert!(quality_option.selection_wire_prefer_quality);
        assert!(quality_option.selection_wire_endpoint_pinned);
        assert_eq!(
            quality_option.selection_wire_endpoint_kind_label,
            "built_in"
        );
        assert!(quality_option.selection_wire_sends_model_endpoint);
        assert_eq!(
            quality_option
                .selection_wire_model_endpoint_label
                .as_deref(),
            Some("fast-reviewer")
        );
        assert!(!built_in.auto_endpoint_selected);
        assert_eq!(
            built_in.auto_selection_summary,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
        assert_eq!(built_in.auto_selection_model_role_label, "reviewer");
        assert_eq!(
            built_in.auto_selection_routing_preference_label,
            "prefer_fast"
        );
        assert_eq!(built_in.auto_selection_endpoint_label, "auto");
        assert_eq!(
            built_in.auto_selection_endpoint_kind,
            ModelEndpointSelectionKind::Auto
        );
        assert_eq!(built_in.auto_selection_endpoint_kind_label, "auto");
        assert!(built_in.auto_selection_endpoint_auto);
        assert!(!built_in.auto_selection_endpoint_built_in);
        assert!(!built_in.auto_selection_endpoint_custom);
        assert_eq!(built_in.auto_selection_wire_model_role_label, "reviewer");
        assert_eq!(
            built_in.auto_selection_wire_routing_preference_label,
            "prefer_fast"
        );
        assert!(built_in.auto_selection_wire_prefer_fast);
        assert!(!built_in.auto_selection_wire_prefer_quality);
        assert!(!built_in.auto_selection_wire_endpoint_pinned);
        assert_eq!(built_in.auto_selection_wire_endpoint_kind_label, "auto");
        assert!(!built_in.auto_selection_wire_sends_model_endpoint);
        assert_eq!(built_in.auto_selection_wire_model_endpoint_label, None);
        assert_eq!(
            built_in
                .endpoint_options
                .iter()
                .filter(|option| option.selected)
                .map(|option| option.endpoint_label.as_str())
                .collect::<Vec<_>>(),
            vec!["fast-reviewer"]
        );
        let built_in_auto = &built_in.endpoint_options[0];
        assert_eq!(built_in_auto.endpoint_label, "auto");
        assert!(!built_in_auto.selected);
        assert_eq!(
            built_in_auto.selection_summary,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
        assert!(built_in_auto.selection_wire_prefer_fast);
        assert!(!built_in_auto.selection_wire_endpoint_pinned);
        assert!(!built_in_auto.selection_wire_sends_model_endpoint);
        let fast_option = built_in
            .endpoint_options
            .iter()
            .find(|option| option.endpoint_label == "fast-reviewer")
            .expect("built-in endpoint option should be present");
        assert!(fast_option.selected);
        assert_eq!(
            fast_option.selection_summary,
            "role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true"
        );
        assert!(fast_option.selection_endpoint_built_in);
        assert!(fast_option.selection_wire_prefer_fast);
        assert!(fast_option.selection_wire_endpoint_pinned);
        assert_eq!(fast_option.selection_wire_endpoint_kind_label, "built_in");
        assert!(fast_option.selection_wire_sends_model_endpoint);
        assert_eq!(
            fast_option.selection_wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );

        let custom = InputRouteOptionsSnapshot::from_intent(&RoutingIntent::operator_pinned(
            ModelRole::Reviewer,
            RoutingPreference::PreferFast,
            ModelEndpoint::Worker("mlx-reviewer-8b".to_owned()),
        ));
        assert_eq!(custom.selected_endpoint_label, "mlx-reviewer-8b");
        assert_eq!(
            custom.selected_endpoint_kind,
            ModelEndpointSelectionKind::Custom
        );
        assert_eq!(custom.selected_endpoint_kind_label, "custom");
        assert!(!custom.selected_endpoint_auto);
        assert!(!custom.selected_endpoint_built_in);
        assert!(custom.selected_endpoint_custom);
        assert!(custom.endpoint_pinned);
        assert!(custom.selected_wire_endpoint_pinned);
        assert_eq!(custom.selected_wire_endpoint_kind_label, "custom");
        assert!(custom.selected_wire_sends_model_endpoint);
        assert_eq!(
            custom.selected_wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-8b")
        );
        assert_eq!(
            custom
                .endpoint_options
                .iter()
                .filter(|option| option.selected)
                .count(),
            0
        );

        let unpinned_hint = InputRouteOptionsSnapshot::from_intent(&RoutingIntent {
            model_role: ModelRole::Reviewer,
            routing_preference: RoutingPreference::PreferFast,
            model_endpoint: Some(ModelEndpoint::FastReviewer),
            endpoint_pinned: false,
        });
        assert_eq!(unpinned_hint.selected_endpoint_label, "auto");
        assert_eq!(
            unpinned_hint.selected_endpoint_kind,
            ModelEndpointSelectionKind::Auto
        );
        assert_eq!(unpinned_hint.selected_endpoint_kind_label, "auto");
        assert!(unpinned_hint.selected_endpoint_auto);
        assert!(!unpinned_hint.selected_endpoint_built_in);
        assert!(!unpinned_hint.selected_endpoint_custom);
        assert!(!unpinned_hint.endpoint_pinned);
        assert_eq!(unpinned_hint.selected_wire_model_role_label, "reviewer");
        assert_eq!(
            unpinned_hint.selected_wire_routing_preference_label,
            "prefer_fast"
        );
        assert!(unpinned_hint.selected_wire_prefer_fast);
        assert!(!unpinned_hint.selected_wire_prefer_quality);
        assert!(!unpinned_hint.selected_wire_endpoint_pinned);
        assert_eq!(unpinned_hint.selected_wire_endpoint_kind_label, "auto");
        assert!(!unpinned_hint.selected_wire_sends_model_endpoint);
        assert_eq!(unpinned_hint.selected_wire_model_endpoint_label, None);
        assert!(unpinned_hint.auto_endpoint_selected);
        assert_eq!(
            unpinned_hint.auto_selection_summary,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
        assert!(!unpinned_hint.auto_selection_wire_endpoint_pinned);
        assert!(!unpinned_hint.auto_selection_wire_sends_model_endpoint);
        assert_eq!(
            unpinned_hint
                .endpoint_options
                .iter()
                .filter(|option| option.selected)
                .map(|option| option.endpoint_label.as_str())
                .collect::<Vec<_>>(),
            vec!["auto"]
        );
    }

    #[test]
    fn control_route_options_follow_readiness_send_boundary_not_stale_status() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferQuality)
                .with_model_endpoint(Some(ModelEndpoint::Worker(
                    "mlx-reviewer-quality".to_owned(),
                ))),
        );
        for ch in "review the final patch".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let readiness = input.readiness_recording(&session);
        let stale_status = CliStatusSnapshot::new(&CliInputConfig::default(), &session, None);
        let control = InputControlSnapshot::new(readiness, stale_status);

        assert!(!control.status_route_matches_readiness);
        assert!(control.status_route_is_stale);
        assert_eq!(control.status.model_role_label, "assistant");
        assert_eq!(control.status.routing_preference_label, "balanced");
        assert_eq!(control.status.endpoint_label, "auto");
        assert!(!control.status.endpoint_pinned);
        assert_eq!(control.status.wire_endpoint_kind_label, "auto");
        assert!(!control.status.wire_sends_model_endpoint);

        assert_eq!(control.readiness.model_role_label, "reviewer");
        assert_eq!(control.readiness.routing_preference_label, "prefer_quality");
        assert_eq!(control.readiness.endpoint_label, "mlx-reviewer-quality");
        assert!(control.readiness.endpoint_pinned);
        assert_eq!(control.readiness.endpoint_kind_label, "custom");
        assert!(control.readiness.wire_prefer_quality);
        assert!(control.readiness.wire_endpoint_pinned);
        assert_eq!(control.readiness.wire_endpoint_kind_label, "custom");
        assert!(control.readiness.wire_sends_model_endpoint);
        assert_eq!(
            control.readiness.wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-quality")
        );

        assert_eq!(control.route_options.selected_role, ModelRole::Reviewer);
        assert_eq!(control.route_options.selected_role_label, "reviewer");
        assert_eq!(
            control.route_options.selected_preference,
            RoutingPreference::PreferQuality
        );
        assert_eq!(
            control.route_options.selected_preference_label,
            "prefer_quality"
        );
        assert_eq!(
            control.route_options.selected_endpoint_label,
            "mlx-reviewer-quality"
        );
        assert_eq!(
            control.route_options.selected_endpoint_kind,
            ModelEndpointSelectionKind::Custom
        );
        assert_eq!(control.route_options.selected_endpoint_kind_label, "custom");
        assert!(!control.route_options.selected_endpoint_auto);
        assert!(!control.route_options.selected_endpoint_built_in);
        assert!(control.route_options.selected_endpoint_custom);
        assert!(control.route_options.endpoint_pinned);
        assert_eq!(
            control.route_options.selected_wire_model_role_label,
            "reviewer"
        );
        assert_eq!(
            control.route_options.selected_wire_routing_preference_label,
            "prefer_quality"
        );
        assert!(!control.route_options.selected_wire_prefer_fast);
        assert!(control.route_options.selected_wire_prefer_quality);
        assert!(control.route_options.selected_wire_endpoint_pinned);
        assert_eq!(
            control.route_options.selected_wire_endpoint_kind_label,
            "custom"
        );
        assert!(control.route_options.selected_wire_sends_model_endpoint);
        assert_eq!(
            control
                .route_options
                .selected_wire_model_endpoint_label
                .as_deref(),
            Some("mlx-reviewer-quality")
        );

        let request = control
            .request_preview
            .as_ref()
            .expect("prompt readiness should carry the outbound request preview");
        assert_eq!(request.model_role_label, "reviewer");
        assert_eq!(request.routing_preference_label, "prefer_quality");
        assert_eq!(request.endpoint_label, "mlx-reviewer-quality");
        assert!(request.endpoint_pinned);
        assert!(request.wire_prefer_quality);
        assert!(request.wire_sends_model_endpoint);
        assert_eq!(
            request.wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-quality")
        );
    }

    #[test]
    fn control_snapshot_combines_enter_send_state_and_worker_picker() {
        let session = ChatSession::new(
            "cli",
            ChatSessionConfig::new(16).with_default_max_tokens(Some(4096)),
        );
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#42 review".to_owned())),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast)
                .with_model_endpoint(Some(ModelEndpoint::FastReviewer)),
        );
        for ch in "quick review".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let control = input.control_snapshot_with_model_pool_gate(
            &session,
            &gate,
            InputSubmitMode::StartStream,
        );

        assert!(control.status_route_matches_readiness);
        assert!(!control.status_route_is_stale);
        assert!(!control.send_enabled);
        assert!(!control.enter_submits_prompt);
        assert!(!control.enter_runs_local_command);
        assert!(control.enter_is_blocked);
        assert_eq!(control.primary_action_label, "wait_for_current_stream");
        assert!(!control.primary_action_enabled);
        assert_eq!(
            control.primary_action_disabled_reason.as_deref(),
            Some("worker fast-reviewer is busy: #42 review")
        );
        assert!(control.preserves_buffer_on_enter);
        assert!(!control.clears_buffer_on_enter);
        assert_eq!(
            control.advice_action,
            Some(GateAdviceAction::WaitForCurrentStream)
        );
        assert_eq!(
            control.advice_action_label.as_deref(),
            Some("wait_for_current_stream")
        );
        assert_eq!(control.block_state, Some(StreamState::Busy));
        assert_eq!(control.block_state_label.as_deref(), Some("busy"));
        assert!(!control.block_state_is_terminal);
        assert!(control.block_state_is_pressure);
        assert!(control.block_state_blocks_prompt_submit);
        let block_chunk = control
            .block_chunk
            .as_ref()
            .expect("control wait state should carry service chunk display snapshot");
        assert_eq!(block_chunk.output_label, "busy");
        assert_eq!(
            block_chunk.appended,
            "[busy] worker fast-reviewer is busy: #42 review"
        );
        assert!(block_chunk.state_blocks_prompt_submit);
        let prompt_control = control
            .prompt_submit_control
            .as_ref()
            .expect("control snapshot should expose prompt submit control");
        assert!(!prompt_control.send_allowed);
        assert_eq!(
            prompt_control.action,
            GateAdviceAction::WaitForCurrentStream
        );
        assert_eq!(
            prompt_control.primary_action_label,
            "wait_for_current_stream"
        );
        assert_eq!(
            prompt_control.primary_action_disabled_reason.as_deref(),
            Some("worker fast-reviewer is busy: #42 review")
        );
        let prompt_control_block = prompt_control
            .block_chunk
            .as_ref()
            .expect("prompt control should expose service chunk display snapshot");
        assert_eq!(prompt_control_block.output_label, "busy");
        assert_eq!(
            prompt_control_block.appended,
            "[busy] worker fast-reviewer is busy: #42 review"
        );
        assert!(prompt_control.preserves_prompt);
        assert!(!prompt_control.clears_prompt);
        assert_eq!(
            control.block_reason.as_deref(),
            Some("worker fast-reviewer is busy: #42 review")
        );
        assert_eq!(
            control.send_block_reason.as_deref(),
            Some("worker fast-reviewer is busy: #42 review")
        );
        assert_eq!(
            control.route_gate_advice.as_deref(),
            Some("wait_for_current_stream busy: worker fast-reviewer is busy: #42 review")
        );
        let route_advice = control
            .route_gate_advice_detail
            .as_ref()
            .expect("model-pool control should carry route advice");
        assert_eq!(route_advice.action, GateAdviceAction::WaitForCurrentStream);
        assert_eq!(route_advice.state, StreamState::Busy);
        assert_eq!(
            control.route_gate_advice_action_label.as_deref(),
            Some("wait_for_current_stream")
        );
        assert_eq!(
            control.route_gate_advice_state_label.as_deref(),
            Some("busy")
        );
        assert_eq!(
            control.route_gate_advice_reason.as_deref(),
            Some("worker fast-reviewer is busy: #42 review")
        );
        assert_eq!(control.advice_state_label.as_deref(), Some("busy"));
        assert_eq!(
            control.advice_reason.as_deref(),
            Some("worker fast-reviewer is busy: #42 review")
        );
        assert_eq!(control.route_send_allowed, Some(false));
        assert_eq!(control.route_send_block_state, Some(StreamState::Busy));
        assert_eq!(
            control.route_send_block_state_label.as_deref(),
            Some("busy")
        );
        assert_eq!(control.route_send_block_state_is_terminal, Some(false));
        assert_eq!(control.route_send_block_state_is_pressure, Some(true));
        assert_eq!(
            control.route_send_block_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(
            control.route_send_block_reason.as_deref(),
            Some("worker fast-reviewer is busy: #42 review")
        );
        let route_block_chunk = control
            .route_send_block_chunk
            .as_ref()
            .expect("route send block should expose service chunk display snapshot");
        assert_eq!(route_block_chunk.output_label, "busy");
        assert_eq!(
            route_block_chunk.appended,
            "[busy] worker fast-reviewer is busy: #42 review"
        );
        assert!(route_block_chunk.state_blocks_prompt_submit);
        assert_eq!(
            control.pool_status.as_deref(),
            Some("workers total=2 available=1 busy=1 saturated=0")
        );
        assert_eq!(control.pool_queue_label.as_deref(), Some("0/2"));
        assert_eq!(control.pool_has_workers, Some(true));
        assert_eq!(control.pool_has_available_workers, Some(true));
        assert_eq!(control.pool_has_busy_workers, Some(true));
        assert_eq!(control.pool_has_saturated_workers, Some(false));
        assert_eq!(control.pool_has_queued_requests, Some(false));
        assert_eq!(control.pool_queue_is_saturated, Some(false));
        assert_eq!(control.pool_capacity_state, Some(StreamState::Pending));
        assert_eq!(
            control.pool_capacity_state_label.as_deref(),
            Some("pending")
        );
        assert_eq!(control.pool_capacity_state_is_pressure, Some(false));
        assert_eq!(
            control.pool_capacity_state_blocks_prompt_submit,
            Some(false)
        );
        assert_eq!(
            control.route_pool_status.as_deref(),
            Some("matching total=1 available=0 busy=1 saturated=0")
        );
        assert_eq!(control.route_pool_queue_label.as_deref(), Some("0/1"));
        assert_eq!(control.route_pool_capacity_state, Some(StreamState::Busy));
        assert_eq!(
            control.route_pool_capacity_state_label.as_deref(),
            Some("busy")
        );
        assert_eq!(control.route_pool_capacity_state_is_pressure, Some(true));
        assert_eq!(
            control.route_pool_capacity_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(control.route_pool_has_matching_workers, Some(true));
        assert_eq!(
            control.route_pool_has_matching_available_workers,
            Some(false)
        );
        assert_eq!(control.route_pool_has_matching_busy_workers, Some(true));
        assert_eq!(
            control.route_pool_has_matching_saturated_workers,
            Some(false)
        );
        assert_eq!(control.route_pool_has_matching_queued_requests, Some(false));
        assert_eq!(control.route_pool_queue_is_saturated, Some(false));
        let request = control
            .request_preview
            .as_ref()
            .expect("control snapshot should include prompt request metadata");
        assert_eq!(request.messages, 1);
        assert_eq!(request.context_messages, 0);
        assert_eq!(request.history_messages, 0);
        assert_eq!(request.context_kind, ChatRequestContextKind::SingleTurn);
        assert_eq!(request.context_kind_label, "single_turn");
        assert!(!request.has_context);
        assert!(request.is_single_turn);
        assert_eq!(request.max_tokens, Some(4096));
        assert_eq!(request.max_tokens_label, "4096");
        assert_eq!(request.model_role_label, "reviewer");
        assert_eq!(request.routing_preference_label, "prefer_fast");
        assert_eq!(request.endpoint_label, "fast-reviewer");
        assert!(request.endpoint_pinned);
        assert_eq!(request.routing_intent.endpoint_label(), "fast-reviewer");
        assert!(request.routing_intent.endpoint_pinned);
        assert_eq!(control.session_policy.history_messages, 0);
        assert_eq!(control.session_policy.history_limit, 16);
        assert!(!control.session_policy.has_history);
        assert!(control.session_policy.is_empty_history);
        assert_eq!(control.session_policy.max_tokens, Some(4096));
        assert_eq!(control.session_policy.max_tokens_label, "4096");
        assert_eq!(control.session_policy.history_remaining, 16);
        assert!(!control.session_policy.history_at_limit);
        assert_eq!(control.readiness.enter_action, InputActionKind::Blocked);
        assert!(control.readiness.preserves_buffer_on_enter);
        assert!(!control.readiness.clears_buffer_on_enter);
        assert!(!control.readiness.records_user_on_enter);
        assert!(!control.readiness.starts_stream_on_enter);
        assert_eq!(
            control
                .route_options
                .roles
                .iter()
                .map(|role| role.as_str())
                .collect::<Vec<_>>(),
            vec!["assistant", "reviewer", "summarizer", "tester"]
        );
        assert_eq!(
            control.route_options.role_labels,
            vec!["assistant", "reviewer", "summarizer", "tester"]
        );
        assert_eq!(
            control
                .route_options
                .preferences
                .iter()
                .map(|preference| preference.as_str())
                .collect::<Vec<_>>(),
            vec!["balanced", "prefer_fast", "prefer_quality"]
        );
        assert_eq!(
            control.route_options.preference_labels,
            vec!["balanced", "prefer_fast", "prefer_quality"]
        );
        assert_eq!(
            control
                .route_options
                .built_in_endpoints
                .iter()
                .map(|endpoint| endpoint.label())
                .collect::<Vec<_>>(),
            vec!["quality-12b", "fast-reviewer", "summary-tester"]
        );
        assert_eq!(
            control.route_options.built_in_endpoint_labels,
            vec!["quality-12b", "fast-reviewer", "summary-tester"]
        );
        assert_eq!(control.route_options.auto_endpoint_label, "auto");
        assert_eq!(control.route_options.selected_role, ModelRole::Reviewer);
        assert_eq!(control.route_options.selected_role_label, "reviewer");
        assert_eq!(
            control.route_options.selected_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(
            control.route_options.selected_preference_label,
            "prefer_fast"
        );
        assert_eq!(
            control.route_options.selected_endpoint_label,
            "fast-reviewer"
        );
        assert_eq!(
            control.route_options.selected_endpoint_kind,
            ModelEndpointSelectionKind::BuiltIn
        );
        assert_eq!(
            control.route_options.selected_endpoint_kind_label,
            "built_in"
        );
        assert!(!control.route_options.selected_endpoint_auto);
        assert!(control.route_options.selected_endpoint_built_in);
        assert!(!control.route_options.selected_endpoint_custom);
        assert!(control.route_options.endpoint_pinned);
        assert_eq!(
            control.route_options.selected_wire_model_role_label,
            "reviewer"
        );
        assert_eq!(
            control.route_options.selected_wire_routing_preference_label,
            "prefer_fast"
        );
        assert!(control.route_options.selected_wire_prefer_fast);
        assert!(!control.route_options.selected_wire_prefer_quality);
        assert!(control.route_options.selected_wire_endpoint_pinned);
        assert_eq!(
            control.route_options.selected_wire_endpoint_kind_label,
            "built_in"
        );
        assert!(control.route_options.selected_wire_sends_model_endpoint);
        assert_eq!(
            control
                .route_options
                .selected_wire_model_endpoint_label
                .as_deref(),
            Some("fast-reviewer")
        );
        assert_eq!(request.endpoint_kind, ModelEndpointSelectionKind::BuiltIn);
        assert_eq!(request.endpoint_kind_label, "built_in");
        assert!(!request.endpoint_auto);
        assert!(request.endpoint_built_in);
        assert!(!request.endpoint_custom);
        assert_eq!(request.wire_model_role_label, "reviewer");
        assert_eq!(request.wire_routing_preference_label, "prefer_fast");
        assert!(request.wire_prefer_fast);
        assert!(!request.wire_prefer_quality);
        assert!(request.wire_sends_max_tokens);
        assert_eq!(request.wire_max_tokens, Some(4096));
        assert!(request.wire_endpoint_pinned);
        assert_eq!(request.wire_endpoint_kind_label, "built_in");
        assert!(request.wire_sends_model_endpoint);
        assert_eq!(
            request.wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );

        let workers = control
            .workers
            .as_ref()
            .expect("model-pool control should carry worker rows");
        assert_eq!(workers.len(), 2);
        assert_eq!(workers[0].endpoint.label(), "quality-12b");
        assert_eq!(workers[1].endpoint.label(), "fast-reviewer");
        assert_eq!(control.workers, control.status.workers);
        assert_eq!(control.route_workers, control.status.route_workers);
        let route_workers = control
            .route_workers
            .as_ref()
            .expect("model-pool control should carry worker picker rows");
        assert_eq!(route_workers.len(), 2);
        assert!(!route_workers[0].endpoint_selected);
        assert!(!route_workers[0].route_match);
        assert_eq!(
            route_workers[0].picker_action,
            ModelRouteWorkerPickerAction::Select
        );
        assert_eq!(route_workers[0].picker_action_label, "select");
        assert!(route_workers[1].endpoint_selected);
        assert!(route_workers[1].route_match);
        assert!(!route_workers[1].selectable);
        assert_eq!(
            route_workers[1].picker_action,
            ModelRouteWorkerPickerAction::Current
        );
        assert_eq!(route_workers[1].picker_action_label, "current");
        assert_eq!(
            route_workers[1].selection_summary,
            "role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true"
        );
        assert_eq!(route_workers[1].selection_endpoint_label, "fast-reviewer");
        assert_eq!(
            route_workers[1].selection_endpoint_kind,
            ModelEndpointSelectionKind::BuiltIn
        );
        assert_eq!(route_workers[1].selection_endpoint_kind_label, "built_in");
        assert!(route_workers[1].selection_wire_endpoint_pinned);
        assert_eq!(
            route_workers[1].selection_wire_endpoint_kind_label,
            "built_in"
        );
        assert!(route_workers[1].selection_wire_sends_model_endpoint);
        assert_eq!(
            route_workers[1]
                .selection_wire_model_endpoint_label
                .as_deref(),
            Some("fast-reviewer")
        );
    }

    #[test]
    fn control_snapshot_maps_pinned_saturated_worker_to_retry_later_backpressure() {
        let session = ChatSession::new(
            "cli",
            ChatSessionConfig::new(8).with_default_max_tokens(Some(2048)),
        );
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester).with_queue(2, 2)],
        );
        let mut input = CliInput::new(
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::SummaryTester)),
        );
        for ch in "summarize backlog".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let control = input.control_snapshot_with_model_pool_gate(
            &session,
            &gate,
            InputSubmitMode::StartStream,
        );

        assert!(!control.send_enabled);
        assert!(!control.enter_submits_prompt);
        assert!(control.enter_is_blocked);
        assert_eq!(control.primary_action_label, "retry_later");
        assert!(!control.primary_action_enabled);
        assert_eq!(
            control.primary_action_disabled_reason.as_deref(),
            Some("worker summary-tester queue is saturated: 2/2")
        );
        assert_eq!(control.advice_action, Some(GateAdviceAction::RetryLater));
        assert_eq!(control.advice_action_label.as_deref(), Some("retry_later"));
        assert_eq!(control.block_state, Some(StreamState::Backpressure));
        assert_eq!(control.block_state_label.as_deref(), Some("backpressure"));
        assert!(!control.block_state_is_terminal);
        assert!(control.block_state_is_pressure);
        assert!(control.block_state_blocks_prompt_submit);
        assert!(control.preserves_buffer_on_enter);
        assert!(!control.clears_buffer_on_enter);
        assert_eq!(
            control.send_block_reason.as_deref(),
            Some("worker summary-tester queue is saturated: 2/2")
        );
        let block_chunk = control
            .block_chunk
            .as_ref()
            .expect("pinned saturated worker should expose a backpressure chunk");
        assert_eq!(block_chunk.output_label, "backpressure");
        assert_eq!(
            block_chunk.appended,
            "[backpressure] worker summary-tester queue is saturated: 2/2"
        );
        assert!(block_chunk.state_is_pressure);
        assert!(block_chunk.state_blocks_prompt_submit);

        let prompt_control = control
            .prompt_submit_control
            .as_ref()
            .expect("backpressure should expose prompt submit control");
        assert!(prompt_control.prompt_present);
        assert!(!prompt_control.send_allowed);
        assert_eq!(prompt_control.action, GateAdviceAction::RetryLater);
        assert_eq!(prompt_control.action_label, "retry_later");
        assert_eq!(prompt_control.primary_action_label, "retry_later");
        assert_eq!(
            prompt_control.primary_action_disabled_reason.as_deref(),
            Some("worker summary-tester queue is saturated: 2/2")
        );
        assert_eq!(prompt_control.state, StreamState::Backpressure);
        assert_eq!(prompt_control.state_label, "backpressure");
        assert!(prompt_control.state_is_pressure);
        assert!(prompt_control.state_blocks_prompt_submit);
        assert!(prompt_control.preserves_prompt);
        assert!(!prompt_control.clears_prompt);

        assert_eq!(control.route_send_allowed, Some(false));
        assert_eq!(
            control.route_send_block_state,
            Some(StreamState::Backpressure)
        );
        assert_eq!(
            control.route_send_block_state_label.as_deref(),
            Some("backpressure")
        );
        assert_eq!(control.route_send_block_state_is_terminal, Some(false));
        assert_eq!(control.route_send_block_state_is_pressure, Some(true));
        assert_eq!(
            control.route_send_block_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(
            control.route_send_block_reason.as_deref(),
            Some("worker summary-tester queue is saturated: 2/2")
        );
        let route_block = control
            .route_send_block_chunk
            .as_ref()
            .expect("route send block should expose the backpressure chunk");
        assert_eq!(route_block.appended, block_chunk.appended);

        let request = control
            .request_preview
            .as_ref()
            .expect("blocked prompt should keep next request preview visible");
        assert_eq!(request.messages, 1);
        assert_eq!(request.context_messages, 0);
        assert_eq!(request.history_messages, 0);
        assert_eq!(request.max_tokens, Some(2048));
        assert_eq!(request.endpoint_label, "summary-tester");
        assert!(request.endpoint_pinned);
        assert!(request.wire_sends_model_endpoint);
        assert_eq!(
            request.wire_model_endpoint_label.as_deref(),
            Some("summary-tester")
        );
        assert_eq!(control.readiness.enter_action, InputActionKind::Blocked);
        assert!(!control.readiness.records_user_on_enter);
        assert!(!control.readiness.starts_stream_on_enter);
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
        assert_eq!(input.buffer(), "summarize backlog");
    }

    #[test]
    fn control_snapshot_maps_auto_route_saturated_match_to_backpressure() {
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
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "review this".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let control = input.control_snapshot_with_model_pool_gate(
            &session,
            &gate,
            InputSubmitMode::StartStream,
        );

        assert!(!control.send_enabled);
        assert!(!control.enter_submits_prompt);
        assert!(!control.enter_runs_local_command);
        assert!(control.enter_is_blocked);
        assert_eq!(control.primary_action_label, "retry_later");
        assert!(!control.primary_action_enabled);
        assert_eq!(
            control.primary_action_disabled_reason.as_deref(),
            Some("matching model workers are saturated: 1 workers")
        );
        assert_eq!(control.advice_action, Some(GateAdviceAction::RetryLater));
        assert_eq!(control.advice_action_label.as_deref(), Some("retry_later"));
        assert_eq!(control.block_state, Some(StreamState::Backpressure));
        assert_eq!(control.block_state_label.as_deref(), Some("backpressure"));
        assert!(!control.block_state_is_terminal);
        assert!(control.block_state_is_pressure);
        assert!(control.block_state_blocks_prompt_submit);
        assert_eq!(
            control.send_block_reason.as_deref(),
            Some("matching model workers are saturated: 1 workers")
        );
        let block_chunk = control
            .block_chunk
            .as_ref()
            .expect("auto-route saturated match should expose a backpressure chunk");
        assert_eq!(block_chunk.output_label, "backpressure");
        assert_eq!(
            block_chunk.appended,
            "[backpressure] matching model workers are saturated: 1 workers"
        );
        assert!(block_chunk.state_is_pressure);
        assert!(block_chunk.state_blocks_prompt_submit);

        let prompt_control = control
            .prompt_submit_control
            .as_ref()
            .expect("auto-route backpressure should expose prompt submit control");
        assert!(prompt_control.prompt_present);
        assert!(!prompt_control.send_allowed);
        assert_eq!(prompt_control.action, GateAdviceAction::RetryLater);
        assert_eq!(prompt_control.action_label, "retry_later");
        assert_eq!(prompt_control.primary_action_label, "retry_later");
        assert_eq!(
            prompt_control.primary_action_disabled_reason.as_deref(),
            Some("matching model workers are saturated: 1 workers")
        );
        assert_eq!(prompt_control.state, StreamState::Backpressure);
        assert_eq!(prompt_control.state_label, "backpressure");
        assert!(prompt_control.state_is_pressure);
        assert!(prompt_control.state_blocks_prompt_submit);
        assert!(prompt_control.preserves_prompt);
        assert!(!prompt_control.clears_prompt);

        assert_eq!(control.route_send_allowed, Some(false));
        assert_eq!(
            control.route_send_block_state,
            Some(StreamState::Backpressure)
        );
        assert_eq!(
            control.route_send_block_state_label.as_deref(),
            Some("backpressure")
        );
        assert_eq!(control.route_send_block_state_is_terminal, Some(false));
        assert_eq!(control.route_send_block_state_is_pressure, Some(true));
        assert_eq!(
            control.route_send_block_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(
            control.route_send_block_reason.as_deref(),
            Some("matching model workers are saturated: 1 workers")
        );
        let route_block = control
            .route_send_block_chunk
            .as_ref()
            .expect("route send block should expose the backpressure chunk");
        assert_eq!(route_block.appended, block_chunk.appended);

        assert_eq!(
            control.pool_status.as_deref(),
            Some("workers total=2 available=1 busy=0 saturated=1")
        );
        assert_eq!(control.pool_queue_label.as_deref(), Some("1/2"));
        assert_eq!(control.pool_has_available_workers, Some(true));
        assert_eq!(control.pool_has_saturated_workers, Some(true));
        assert_eq!(control.pool_queue_is_saturated, Some(false));
        assert_eq!(control.pool_capacity_state, Some(StreamState::Queued));
        assert_eq!(control.pool_capacity_state_label.as_deref(), Some("queued"));
        assert_eq!(control.pool_capacity_state_is_pressure, Some(true));
        assert_eq!(control.pool_capacity_state_blocks_prompt_submit, Some(true));
        assert_eq!(
            control.route_pool_status.as_deref(),
            Some("matching total=1 available=0 busy=0 saturated=1")
        );
        assert_eq!(control.route_pool_queue_label.as_deref(), Some("1/1"));
        assert_eq!(
            control.route_pool_capacity_state,
            Some(StreamState::Backpressure)
        );
        assert_eq!(
            control.route_pool_capacity_state_label.as_deref(),
            Some("backpressure")
        );
        assert_eq!(control.route_pool_capacity_state_is_pressure, Some(true));
        assert_eq!(
            control.route_pool_capacity_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(control.route_pool_has_matching_workers, Some(true));
        assert_eq!(
            control.route_pool_has_matching_available_workers,
            Some(false)
        );
        assert_eq!(control.route_pool_has_matching_busy_workers, Some(false));
        assert_eq!(
            control.route_pool_has_matching_saturated_workers,
            Some(true)
        );
        assert_eq!(control.route_pool_has_matching_queued_requests, Some(true));
        assert_eq!(control.route_pool_queue_is_saturated, Some(true));

        let request = control
            .request_preview
            .as_ref()
            .expect("blocked auto-route prompt should keep request preview visible");
        assert_eq!(request.messages, 1);
        assert_eq!(request.context_messages, 0);
        assert_eq!(request.history_messages, 0);
        assert_eq!(request.max_tokens, None);
        assert_eq!(request.max_tokens_label, "backend-default");
        assert_eq!(request.model_role_label, "reviewer");
        assert_eq!(request.routing_preference_label, "prefer_fast");
        assert_eq!(request.endpoint_label, "auto");
        assert!(!request.endpoint_pinned);
        assert!(!request.wire_sends_model_endpoint);
        assert_eq!(request.wire_model_endpoint_label, None);
        assert_eq!(control.readiness.enter_action, InputActionKind::Blocked);
        assert!(!control.readiness.records_user_on_enter);
        assert!(!control.readiness.starts_stream_on_enter);
        assert!(control.readiness.preserves_buffer_on_enter);
        assert!(!control.readiness.clears_buffer_on_enter);

        let route_workers = control
            .route_workers
            .as_ref()
            .expect("auto-route backpressure should carry worker picker rows");
        assert_eq!(route_workers.len(), 2);
        assert_eq!(route_workers[0].endpoint_label(), "quality-12b");
        assert!(!route_workers[0].route_match);
        assert!(!route_workers[0].selectable);
        assert_eq!(
            route_workers[0].picker_action,
            ModelRouteWorkerPickerAction::Unavailable
        );
        assert_eq!(route_workers[0].picker_action_label, "unavailable");
        assert_eq!(route_workers[0].worker_status_label(), "available");
        assert_eq!(route_workers[1].endpoint_label(), "fast-reviewer");
        assert!(route_workers[1].route_match);
        assert!(!route_workers[1].selectable);
        assert_eq!(
            route_workers[1].picker_action,
            ModelRouteWorkerPickerAction::Wait
        );
        assert_eq!(route_workers[1].picker_action_label, "wait");
        assert_eq!(route_workers[1].worker_status_label(), "backpressure");
        assert_eq!(route_workers[1].decision_action_label(), "retry_later");
        assert_eq!(route_workers[1].decision_state_label(), "backpressure");

        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
        assert_eq!(input.buffer(), "review this");
    }

    #[test]
    fn control_snapshot_keeps_frontend_repair_gate_over_available_route_worker() {
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
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "quick review".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let control = input.control_snapshot_with_model_pool_gate(
            &session,
            &gate,
            InputSubmitMode::StartStream,
        );

        assert!(!control.send_enabled);
        assert_eq!(control.advice_action, Some(GateAdviceAction::RepairGate));
        assert_eq!(control.advice_action_label.as_deref(), Some("repair_gate"));
        assert_eq!(control.block_state, Some(StreamState::Failed));
        assert_eq!(control.block_state_label.as_deref(), Some("failed"));
        assert_eq!(
            control.block_reason.as_deref(),
            Some("safe-device gate failed")
        );
        let block_chunk = control
            .block_chunk
            .as_ref()
            .expect("safe-device gate should expose a failed display chunk");
        assert_eq!(block_chunk.output_label, "error");
        assert_eq!(block_chunk.state_label, "failed");
        assert_eq!(block_chunk.appended, "[error] safe-device gate failed");
        assert!(!block_chunk.state_is_pressure);
        assert!(!block_chunk.state_blocks_prompt_submit);
        let prompt_control = control
            .prompt_submit_control
            .as_ref()
            .expect("safe-device repair gate should expose prompt submit control");
        assert!(prompt_control.prompt_present);
        assert!(!prompt_control.send_allowed);
        assert_eq!(prompt_control.action, GateAdviceAction::RepairGate);
        assert_eq!(prompt_control.action_label, "repair_gate");
        assert_eq!(prompt_control.primary_action_label, "repair_gate");
        assert_eq!(
            prompt_control.primary_action_disabled_reason.as_deref(),
            Some("safe-device gate failed")
        );
        assert_eq!(prompt_control.state, StreamState::Failed);
        assert_eq!(prompt_control.state_label, "failed");
        assert!(prompt_control.state_is_terminal);
        assert!(!prompt_control.state_is_pressure);
        assert!(!prompt_control.state_blocks_prompt_submit);
        assert!(prompt_control.preserves_prompt);
        assert!(!prompt_control.clears_prompt);
        let prompt_block = prompt_control
            .block_chunk
            .as_ref()
            .expect("prompt control should expose the failed display chunk");
        assert_eq!(prompt_block.appended, block_chunk.appended);
        assert_eq!(control.route_send_allowed, Some(false));
        assert_eq!(control.route_send_block_state, Some(StreamState::Failed));
        assert_eq!(
            control.route_send_block_state_label.as_deref(),
            Some("failed")
        );
        assert_eq!(control.route_send_block_state_is_terminal, Some(true));
        assert_eq!(control.route_send_block_state_is_pressure, Some(false));
        assert_eq!(
            control.route_send_block_state_blocks_prompt_submit,
            Some(false)
        );
        assert_eq!(
            control.route_send_block_reason.as_deref(),
            Some("safe-device gate failed")
        );
        let route_block = control
            .route_send_block_chunk
            .as_ref()
            .expect("route send block should expose the failed display chunk");
        assert_eq!(route_block.appended, block_chunk.appended);
        assert_eq!(
            control.route_gate_advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(
            control.route_gate_advice_state_label.as_deref(),
            Some("failed")
        );
        assert_eq!(
            control.route_gate_advice_reason.as_deref(),
            Some("safe-device gate failed")
        );
        assert_eq!(
            control.pool_status.as_deref(),
            Some("workers total=1 available=1 busy=0 saturated=0")
        );
        assert_eq!(
            control.route_pool_status.as_deref(),
            Some("matching total=1 available=1 busy=0 saturated=0")
        );
        let request = control
            .request_preview
            .as_ref()
            .expect("repair gate should preserve next request preview");
        assert_eq!(request.messages, 1);
        assert_eq!(request.context_messages, 0);
        assert_eq!(request.history_messages, 0);
        assert_eq!(request.history_limit, Some(64));
        assert_eq!(request.context_kind, ChatRequestContextKind::SingleTurn);
        assert_eq!(request.context_kind_label, "single_turn");
        assert_eq!(request.max_tokens, None);
        assert_eq!(request.max_tokens_label, "backend-default");
        assert!(!request.wire_sends_max_tokens);
        assert_eq!(request.wire_max_tokens, None);
        assert_eq!(request.model_role_label, "reviewer");
        assert_eq!(request.routing_preference_label, "prefer_fast");
        assert_eq!(request.endpoint_label, "auto");
        assert!(!request.endpoint_pinned);
        assert_eq!(control.readiness.enter_action, InputActionKind::Blocked);
        assert!(!control.readiness.records_user_on_enter);
        assert!(!control.readiness.starts_stream_on_enter);
        assert!(control.readiness.preserves_buffer_on_enter);
        assert!(!control.readiness.clears_buffer_on_enter);
        let route_workers = control
            .route_workers
            .as_ref()
            .expect("model-pool control should carry worker picker rows");
        assert_eq!(route_workers.len(), 1);
        assert_eq!(route_workers[0].endpoint_label(), "fast-reviewer");
        assert!(route_workers[0].route_match);
        assert!(!route_workers[0].selectable);
        assert_eq!(
            route_workers[0].picker_action,
            ModelRouteWorkerPickerAction::RepairGate
        );
        assert_eq!(route_workers[0].picker_action_label, "repair_gate");
        assert_eq!(route_workers[0].worker_status_label(), "available");
        assert_eq!(
            route_workers[0].decision,
            GateDecision::blocked(StreamState::Failed, "safe-device gate failed")
        );
        assert_eq!(route_workers[0].decision_action_label(), "repair_gate");
        assert_eq!(route_workers[0].decision_state_label(), "failed");
        assert_eq!(
            route_workers[0].decision_reason(),
            "safe-device gate failed"
        );
        assert_eq!(
            route_workers[0].selection_summary,
            "role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true"
        );
        assert!(route_workers[0].selection_wire_endpoint_pinned);
        assert!(route_workers[0].selection_wire_sends_model_endpoint);
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
        assert_eq!(input.buffer(), "quick review");
    }

    #[test]
    fn control_snapshot_keeps_backend_offline_gate_over_available_route_worker() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                backend_online: false,
                engine_busy: true,
                safe_device_ok: false,
                experience_hygiene_ok: false,
                queued_requests: 8,
                queue_limit: 8,
                active_request: Some("#42 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "offline review".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let control = input.control_snapshot_with_model_pool_gate(
            &session,
            &gate,
            InputSubmitMode::StartStream,
        );

        assert!(!control.send_enabled);
        assert_eq!(control.advice_action, Some(GateAdviceAction::RepairGate));
        assert_eq!(control.advice_action_label.as_deref(), Some("repair_gate"));
        assert_eq!(control.block_state, Some(StreamState::Failed));
        assert_eq!(control.block_state_label.as_deref(), Some("failed"));
        assert_eq!(control.block_reason.as_deref(), Some("backend is offline"));
        let block_chunk = control
            .block_chunk
            .as_ref()
            .expect("offline gate should expose a failed display chunk");
        assert_eq!(block_chunk.output_label, "error");
        assert_eq!(block_chunk.appended, "[error] backend is offline");

        let prompt_control = control
            .prompt_submit_control
            .as_ref()
            .expect("offline gate should expose prompt submit control");
        assert!(prompt_control.prompt_present);
        assert!(!prompt_control.send_allowed);
        assert_eq!(prompt_control.action, GateAdviceAction::RepairGate);
        assert_eq!(prompt_control.action_label, "repair_gate");
        assert_eq!(prompt_control.state, StreamState::Failed);
        assert_eq!(prompt_control.state_label, "failed");
        assert!(prompt_control.state_is_terminal);
        assert!(!prompt_control.state_is_pressure);
        assert!(!prompt_control.state_blocks_prompt_submit);
        assert_eq!(prompt_control.reason, "backend is offline");
        assert_eq!(prompt_control.primary_action_label, "repair_gate");
        assert!(!prompt_control.primary_action_enabled);
        assert_eq!(
            prompt_control.primary_action_disabled_reason.as_deref(),
            Some("backend is offline")
        );
        assert!(prompt_control.preserves_prompt);
        assert!(!prompt_control.clears_prompt);
        let prompt_block = prompt_control
            .block_chunk
            .as_ref()
            .expect("offline prompt control should expose the failed display chunk");
        assert_eq!(prompt_block.appended, "[error] backend is offline");

        assert_eq!(control.route_send_allowed, Some(false));
        assert_eq!(control.route_send_block_state, Some(StreamState::Failed));
        assert_eq!(
            control.route_send_block_state_label.as_deref(),
            Some("failed")
        );
        assert_eq!(control.route_send_block_state_is_terminal, Some(true));
        assert_eq!(control.route_send_block_state_is_pressure, Some(false));
        assert_eq!(
            control.route_send_block_state_blocks_prompt_submit,
            Some(false)
        );
        assert_eq!(
            control.route_send_block_reason.as_deref(),
            Some("backend is offline")
        );
        assert_eq!(
            control.route_gate_advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(
            control.route_gate_advice_state_label.as_deref(),
            Some("failed")
        );
        assert_eq!(
            control.route_gate_advice_reason.as_deref(),
            Some("backend is offline")
        );
        let request = control
            .request_preview
            .as_ref()
            .expect("offline gate should preserve next request preview");
        assert_eq!(request.messages, 1);
        assert_eq!(request.context_messages, 0);
        assert_eq!(request.max_tokens_label, "backend-default");
        assert!(!request.wire_sends_max_tokens);
        assert_eq!(request.model_role_label, "reviewer");
        assert_eq!(request.routing_preference_label, "prefer_fast");
        assert_eq!(request.endpoint_label, "auto");
        assert!(!request.endpoint_pinned);

        let route_workers = control
            .route_workers
            .as_ref()
            .expect("model-pool control should carry worker picker rows");
        assert_eq!(route_workers.len(), 1);
        assert_eq!(route_workers[0].endpoint_label(), "fast-reviewer");
        assert!(route_workers[0].route_match);
        assert!(!route_workers[0].selectable);
        assert_eq!(
            route_workers[0].picker_action,
            ModelRouteWorkerPickerAction::RepairGate
        );
        assert_eq!(route_workers[0].picker_action_label, "repair_gate");
        assert_eq!(route_workers[0].worker_status_label(), "available");
        assert_eq!(
            route_workers[0].decision,
            GateDecision::blocked(StreamState::Failed, "backend is offline")
        );
        assert_eq!(route_workers[0].decision_reason(), "backend is offline");
        assert_eq!(control.readiness.enter_action, InputActionKind::Blocked);
        assert!(!control.readiness.records_user_on_enter);
        assert!(!control.readiness.starts_stream_on_enter);
        assert!(control.readiness.preserves_buffer_on_enter);
        assert!(!control.readiness.clears_buffer_on_enter);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
        assert_eq!(input.buffer(), "offline review");

        let mut workers_input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "/workers".chars() {
            workers_input.handle_key(KeyInput::Char(ch), &session);
        }
        let workers_control = workers_input.control_snapshot_with_model_pool_gate(
            &session,
            &gate,
            InputSubmitMode::StartStream,
        );

        assert!(!workers_control.send_enabled);
        assert!(!workers_control.enter_submits_prompt);
        assert!(workers_control.enter_runs_local_command);
        assert!(!workers_control.enter_is_blocked);
        assert_eq!(
            workers_control.readiness.buffer_kind,
            InputBufferKind::WorkerStatusCommand
        );
        assert_eq!(
            workers_control.readiness.enter_action,
            InputActionKind::Status
        );
        assert_eq!(workers_control.primary_action_label, "show_workers");
        assert!(workers_control.primary_action_enabled);
        assert_eq!(workers_control.primary_action_disabled_reason, None);
        assert!(!workers_control.preserves_buffer_on_enter);
        assert!(workers_control.clears_buffer_on_enter);
        assert_eq!(workers_control.request_preview, None);
        assert_eq!(workers_control.prompt_submit_control, None);
        assert_eq!(
            workers_control.advice_action,
            Some(GateAdviceAction::RepairGate)
        );
        assert_eq!(
            workers_control.advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(workers_control.block_state, Some(StreamState::Failed));
        assert_eq!(
            workers_control.block_reason.as_deref(),
            Some("backend is offline")
        );
        assert_eq!(workers_control.route_send_allowed, Some(false));
        assert_eq!(
            workers_control.route_send_block_reason.as_deref(),
            Some("backend is offline")
        );
        let workers_rows = workers_control
            .workers
            .as_ref()
            .expect("workers command control should carry worker rows");
        assert_eq!(workers_rows.len(), 1);
        assert_eq!(workers_rows[0].endpoint_label(), "fast-reviewer");
        assert_eq!(workers_rows[0].status_label(), "available");
        let worker_route_rows = workers_control
            .route_workers
            .as_ref()
            .expect("workers command control should carry route picker rows");
        assert_eq!(worker_route_rows.len(), 1);
        assert!(worker_route_rows[0].route_match);
        assert!(!worker_route_rows[0].selectable);
        assert_eq!(
            worker_route_rows[0].picker_action,
            ModelRouteWorkerPickerAction::RepairGate
        );
        assert_eq!(worker_route_rows[0].picker_action_label, "repair_gate");
        assert_eq!(
            worker_route_rows[0].decision,
            GateDecision::blocked(StreamState::Failed, "backend is offline")
        );
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());
        assert_eq!(workers_input.buffer(), "/workers");

        let mut active_session = ChatSession::new("cli", ChatSessionConfig::default());
        active_session
            .try_submit_and_begin_stream("in flight")
            .expect("expected active stream");
        active_session.push_delta("partial");
        let mut active_input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "offline follow-up".chars() {
            active_input.handle_key(KeyInput::Char(ch), &active_session);
        }

        let active_control = active_input.control_snapshot_with_model_pool_gate(
            &active_session,
            &gate,
            InputSubmitMode::StartStream,
        );

        assert!(!active_control.send_enabled);
        assert_eq!(
            active_control.advice_action,
            Some(GateAdviceAction::RepairGate)
        );
        assert_eq!(
            active_control.advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(active_control.block_state, Some(StreamState::Failed));
        assert_eq!(
            active_control.block_reason.as_deref(),
            Some("backend is offline")
        );
        assert_eq!(
            active_control.send_block_reason.as_deref(),
            Some("backend is offline")
        );
        let active_prompt_control = active_control
            .prompt_submit_control
            .as_ref()
            .expect("offline active stream should expose prompt submit control");
        assert_eq!(active_prompt_control.action, GateAdviceAction::RepairGate);
        assert_eq!(active_prompt_control.state, StreamState::Failed);
        assert!(active_prompt_control.state_is_terminal);
        assert!(!active_prompt_control.state_is_pressure);
        assert!(active_prompt_control.preserves_prompt);
        assert!(!active_prompt_control.clears_prompt);
        let active_request = active_control
            .request_preview
            .as_ref()
            .expect("offline active stream should preserve next request preview");
        assert_eq!(active_request.messages, 2);
        assert_eq!(active_request.context_messages, 1);
        assert_eq!(active_request.history_messages, 1);
        assert_eq!(active_request.last_user_chars, 17);
        assert_eq!(
            active_control.readiness.enter_action,
            InputActionKind::Blocked
        );
        assert!(!active_control.readiness.records_user_on_enter);
        assert!(!active_control.readiness.starts_stream_on_enter);
        assert!(active_control.readiness.preserves_buffer_on_enter);
        assert!(!active_control.readiness.clears_buffer_on_enter);
        assert_eq!(active_session.state(), StreamState::Streaming);
        assert_eq!(active_session.history().len(), 1);
        assert_eq!(active_session.partial_answer(), "partial");
        assert_eq!(active_input.buffer(), "offline follow-up");
    }

    #[test]
    fn control_snapshot_preserves_request_preview_when_experience_gate_blocks_send() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::new(4));
        session.record_user("previous user");
        session.record_assistant("previous assistant");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                experience_hygiene_ok: false,
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
            ],
        );
        let mut input = CliInput::new(
            CliInputConfig::default().with_routing_preference(RoutingPreference::PreferQuality),
        );
        for ch in "continue with context".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let control = input.control_snapshot_with_model_pool_gate(
            &session,
            &gate,
            InputSubmitMode::StartStream,
        );

        assert!(!control.send_enabled);
        assert!(!control.enter_submits_prompt);
        assert!(!control.readiness.records_user_on_enter);
        assert!(!control.readiness.starts_stream_on_enter);
        assert!(control.preserves_buffer_on_enter);
        assert!(!control.clears_buffer_on_enter);
        assert_eq!(control.advice_action, Some(GateAdviceAction::RepairGate));
        assert_eq!(control.advice_action_label.as_deref(), Some("repair_gate"));
        assert_eq!(control.block_state, Some(StreamState::Failed));
        assert_eq!(control.block_state_label.as_deref(), Some("failed"));
        let block_chunk = control
            .block_chunk
            .as_ref()
            .expect("experience gate should expose a display chunk");
        assert_eq!(block_chunk.output_label, "error");
        assert_eq!(block_chunk.state_label, "failed");
        assert_eq!(
            block_chunk.appended,
            "[error] experience hygiene gate failed"
        );
        assert!(!block_chunk.state_is_pressure);
        assert!(!block_chunk.state_blocks_prompt_submit);
        assert_eq!(
            control.block_reason.as_deref(),
            Some("experience hygiene gate failed")
        );
        let prompt_control = control
            .prompt_submit_control
            .as_ref()
            .expect("blocked prompt should expose send control");
        assert!(prompt_control.prompt_present);
        assert_eq!(prompt_control.action, GateAdviceAction::RepairGate);
        assert_eq!(prompt_control.action_label, "repair_gate");
        assert!(!prompt_control.send_allowed);
        assert_eq!(prompt_control.primary_action_label, "repair_gate");
        assert_eq!(
            prompt_control.primary_action_disabled_reason.as_deref(),
            Some("experience hygiene gate failed")
        );
        assert_eq!(prompt_control.state, StreamState::Failed);
        assert_eq!(prompt_control.state_label, "failed");
        assert!(prompt_control.state_is_terminal);
        assert!(!prompt_control.state_is_pressure);
        assert!(!prompt_control.state_blocks_prompt_submit);
        assert!(prompt_control.preserves_prompt);
        assert!(!prompt_control.clears_prompt);
        let prompt_block_chunk = prompt_control
            .block_chunk
            .as_ref()
            .expect("prompt control should expose the same display chunk");
        assert_eq!(prompt_block_chunk.appended, block_chunk.appended);
        assert_eq!(control.route_send_allowed, Some(false));
        assert_eq!(control.route_send_block_state, Some(StreamState::Failed));
        assert_eq!(
            control.route_send_block_state_label.as_deref(),
            Some("failed")
        );
        let route_block_chunk = control
            .route_send_block_chunk
            .as_ref()
            .expect("route send block should expose a display chunk");
        assert_eq!(route_block_chunk.output_label, "error");
        assert_eq!(
            route_block_chunk.appended,
            "[error] experience hygiene gate failed"
        );
        assert_eq!(
            control.route_gate_advice_reason.as_deref(),
            Some("experience hygiene gate failed")
        );
        assert_eq!(
            control.pool_status.as_deref(),
            Some("workers total=1 available=1 busy=0 saturated=0")
        );
        assert_eq!(
            control.route_pool_status.as_deref(),
            Some("matching total=1 available=1 busy=0 saturated=0")
        );

        let request = control
            .request_preview
            .as_ref()
            .expect("blocked prompt should keep next request preview visible");
        assert_eq!(request.messages, 3);
        assert_eq!(request.context_messages, 2);
        assert_eq!(request.messages, request.context_messages + 1);
        assert_eq!(request.history_messages, 2);
        assert_eq!(request.history_limit, Some(4));
        assert_eq!(request.history_remaining, Some(2));
        assert_eq!(request.history_messages_after_submit, Some(3));
        assert_eq!(request.history_at_limit_after_submit, Some(false));
        assert_eq!(request.history_truncates_on_submit, Some(false));
        assert_eq!(request.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(request.context_kind_label, "multi_turn");
        assert!(request.has_context);
        assert!(!request.is_single_turn);
        assert_eq!(request.max_tokens, None);
        assert_eq!(request.max_tokens_label, "backend-default");
        assert!(!request.wire_sends_max_tokens);
        assert_eq!(request.wire_max_tokens, None);
        assert_eq!(request.model_role_label, "assistant");
        assert_eq!(request.routing_preference_label, "prefer_quality");
        assert_eq!(request.endpoint_label, "auto");
        assert!(!request.endpoint_pinned);
        assert!(request.wire_prefer_quality);
        assert!(!request.wire_endpoint_pinned);
        assert!(!request.wire_sends_model_endpoint);
        assert_eq!(request.wire_model_endpoint_label, None);

        let route_workers = control
            .route_workers
            .as_ref()
            .expect("model-pool control should carry worker picker rows");
        assert_eq!(route_workers.len(), 1);
        assert_eq!(
            route_workers[0].decision,
            GateDecision::blocked(StreamState::Failed, "experience hygiene gate failed")
        );
        assert_eq!(route_workers[0].picker_action_label, "repair_gate");
        assert_eq!(route_workers[0].worker_status_label(), "available");
        assert_eq!(session.state(), StreamState::Pending);
        assert_eq!(session.history().len(), 2);
        assert!(session.chunks().is_empty());
        assert_eq!(input.buffer(), "continue with context");
    }

    #[test]
    fn control_snapshot_preserves_request_preview_when_engine_busy_blocks_send() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::new(4).with_default_max_tokens(Some(2048)),
        );
        session.record_user("previous user");
        session.record_assistant("previous assistant");
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#77 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let mut input = CliInput::new(
            CliInputConfig::default().with_routing_preference(RoutingPreference::PreferQuality),
        );
        for ch in "next question".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let control = input.control_snapshot_with_model_pool_gate(
            &session,
            &gate,
            InputSubmitMode::StartStream,
        );

        assert!(!control.send_enabled);
        assert!(!control.enter_submits_prompt);
        assert!(control.enter_is_blocked);
        assert_eq!(control.primary_action_label, "wait_for_current_stream");
        assert!(!control.primary_action_enabled);
        assert_eq!(
            control.primary_action_disabled_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        assert_eq!(
            control.advice_action,
            Some(GateAdviceAction::WaitForCurrentStream)
        );
        assert_eq!(control.block_state, Some(StreamState::Busy));
        assert_eq!(control.block_state_label.as_deref(), Some("busy"));
        assert!(!control.block_state_is_terminal);
        assert!(control.block_state_is_pressure);
        assert!(control.block_state_blocks_prompt_submit);
        assert!(control.preserves_buffer_on_enter);
        assert!(!control.clears_buffer_on_enter);
        assert_eq!(
            control.send_block_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        let block_chunk = control
            .block_chunk
            .as_ref()
            .expect("engine busy should expose a busy display chunk");
        assert_eq!(block_chunk.output_label, "busy");
        assert_eq!(
            block_chunk.appended,
            "[busy] backend engine is busy: #77 chat-stream"
        );
        assert!(block_chunk.state_blocks_prompt_submit);
        let prompt_control = control
            .prompt_submit_control
            .as_ref()
            .expect("engine busy should expose prompt submit control");
        assert!(prompt_control.prompt_present);
        assert!(!prompt_control.send_allowed);
        assert_eq!(
            prompt_control.action,
            GateAdviceAction::WaitForCurrentStream
        );
        assert_eq!(prompt_control.action_label, "wait_for_current_stream");
        assert_eq!(
            prompt_control.primary_action_label,
            "wait_for_current_stream"
        );
        assert_eq!(prompt_control.state, StreamState::Busy);
        assert_eq!(prompt_control.state_label, "busy");
        assert!(!prompt_control.state_is_terminal);
        assert!(prompt_control.state_is_pressure);
        assert!(prompt_control.state_blocks_prompt_submit);
        assert_eq!(
            prompt_control.primary_action_disabled_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        assert!(prompt_control.preserves_prompt);
        assert!(!prompt_control.clears_prompt);
        let prompt_block = prompt_control
            .block_chunk
            .as_ref()
            .expect("prompt control should expose the busy display chunk");
        assert_eq!(prompt_block.appended, block_chunk.appended);
        assert_eq!(control.route_send_allowed, Some(false));
        assert_eq!(control.route_send_block_state, Some(StreamState::Busy));
        assert_eq!(
            control.route_send_block_state_label.as_deref(),
            Some("busy")
        );
        assert_eq!(control.route_send_block_state_is_terminal, Some(false));
        assert_eq!(control.route_send_block_state_is_pressure, Some(true));
        assert_eq!(
            control.route_send_block_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(
            control.route_send_block_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        let route_block = control
            .route_send_block_chunk
            .as_ref()
            .expect("route send block should expose the frontend gate chunk");
        assert_eq!(route_block.output_label, "busy");
        assert_eq!(
            route_block.appended,
            "[busy] backend engine is busy: #77 chat-stream"
        );
        let request = control
            .request_preview
            .as_ref()
            .expect("blocked prompt should still preview the next request");
        assert_eq!(request.messages, 3);
        assert_eq!(request.context_messages, 2);
        assert_eq!(request.messages, request.context_messages + 1);
        assert_eq!(request.history_messages, 2);
        assert_eq!(request.history_limit, Some(4));
        assert_eq!(request.history_remaining, Some(2));
        assert_eq!(request.history_messages_after_submit, Some(3));
        assert_eq!(request.history_at_limit_after_submit, Some(false));
        assert_eq!(request.history_truncates_on_submit, Some(false));
        assert_eq!(request.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(request.context_kind_label, "multi_turn");
        assert!(request.has_context);
        assert!(!request.is_single_turn);
        assert_eq!(request.max_tokens, Some(2048));
        assert_eq!(request.max_tokens_label, "2048");
        assert!(request.wire_sends_max_tokens);
        assert_eq!(request.wire_max_tokens, Some(2048));
        assert_eq!(request.routing_preference_label, "prefer_quality");
        assert_eq!(request.endpoint_label, "auto");
        assert!(!request.endpoint_pinned);
        assert_eq!(control.session_policy.history_messages, 2);
        assert_eq!(control.session_policy.history_limit, 4);
        assert_eq!(control.session_policy.max_tokens, Some(2048));
        assert_eq!(control.session_policy.max_tokens_label, "2048");
        assert_eq!(control.readiness.enter_action, InputActionKind::Blocked);
        assert!(!control.readiness.records_user_on_enter);
        assert!(!control.readiness.starts_stream_on_enter);
        assert!(control.readiness.preserves_buffer_on_enter);
        assert!(!control.readiness.clears_buffer_on_enter);
        assert_eq!(session.state(), StreamState::Pending);
        assert_eq!(session.history().len(), 2);
        assert!(session.chunks().is_empty());
        assert_eq!(input.buffer(), "next question");
    }

    #[test]
    fn control_snapshot_keeps_request_preview_when_active_session_blocks_send() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::new(4).with_default_max_tokens(Some(4096)),
        );
        session.record_user("previous user");
        session.record_assistant("previous assistant");
        session
            .try_submit_and_begin_stream("in flight")
            .expect("expected active stream");
        session.push_delta("partial");
        let mut input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "next question".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let control = input.control_snapshot(&session, InputSubmitMode::StartStream);

        assert!(!control.send_enabled);
        assert!(!control.enter_submits_prompt);
        assert!(control.enter_is_blocked);
        assert_eq!(control.primary_action_label, "wait_for_current_stream");
        assert!(!control.primary_action_enabled);
        assert_eq!(
            control.primary_action_disabled_reason.as_deref(),
            Some("session stream is already active")
        );
        assert_eq!(
            control.advice_action,
            Some(GateAdviceAction::WaitForCurrentStream)
        );
        assert_eq!(
            control.advice_action_label.as_deref(),
            Some("wait_for_current_stream")
        );
        assert_eq!(control.block_state, Some(StreamState::Busy));
        assert_eq!(control.block_state_label.as_deref(), Some("busy"));
        assert!(!control.block_state_is_terminal);
        assert!(control.block_state_is_pressure);
        assert!(control.block_state_blocks_prompt_submit);
        assert!(control.preserves_buffer_on_enter);
        assert!(!control.clears_buffer_on_enter);
        assert_eq!(
            control.block_reason.as_deref(),
            Some("session stream is already active")
        );
        assert_eq!(
            control.send_block_reason.as_deref(),
            Some("session stream is already active")
        );
        let block_chunk = control
            .block_chunk
            .as_ref()
            .expect("active session pressure should expose a display chunk");
        assert_eq!(block_chunk.output_label, "busy");
        assert_eq!(block_chunk.state_label, "busy");
        assert_eq!(
            block_chunk.appended,
            "[busy] session stream is already active"
        );
        assert!(block_chunk.state_is_pressure);
        assert!(block_chunk.state_blocks_prompt_submit);
        let prompt_control = control
            .prompt_submit_control
            .as_ref()
            .expect("active session pressure should expose prompt submit control");
        assert!(prompt_control.prompt_present);
        assert!(!prompt_control.send_allowed);
        assert_eq!(
            prompt_control.action,
            GateAdviceAction::WaitForCurrentStream
        );
        assert_eq!(prompt_control.action_label, "wait_for_current_stream");
        assert_eq!(
            prompt_control.primary_action_label,
            "wait_for_current_stream"
        );
        assert_eq!(prompt_control.state, StreamState::Busy);
        assert_eq!(prompt_control.state_label, "busy");
        assert!(!prompt_control.state_is_terminal);
        assert!(prompt_control.state_is_pressure);
        assert!(prompt_control.state_blocks_prompt_submit);
        assert_eq!(
            prompt_control.primary_action_disabled_reason.as_deref(),
            Some("session stream is already active")
        );
        assert!(prompt_control.preserves_prompt);
        assert!(!prompt_control.clears_prompt);
        let prompt_block = prompt_control
            .block_chunk
            .as_ref()
            .expect("prompt submit control should expose the same block chunk");
        assert_eq!(prompt_block.appended, block_chunk.appended);

        let request = control
            .request_preview
            .as_ref()
            .expect("blocked active session should keep next request preview visible");
        assert_eq!(request.messages, 4);
        assert_eq!(request.context_messages, 3);
        assert_eq!(request.messages, request.context_messages + 1);
        assert_eq!(request.history_messages, 3);
        assert_eq!(request.history_limit, Some(4));
        assert_eq!(request.history_remaining, Some(1));
        assert_eq!(request.history_messages_after_submit, Some(4));
        assert_eq!(request.history_at_limit_after_submit, Some(true));
        assert_eq!(request.history_truncates_on_submit, Some(false));
        assert_eq!(request.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(request.context_kind_label, "multi_turn");
        assert!(request.has_context);
        assert!(!request.is_single_turn);
        assert_eq!(request.max_tokens, Some(4096));
        assert_eq!(request.max_tokens_label, "4096");
        assert!(request.wire_sends_max_tokens);
        assert_eq!(request.wire_max_tokens, Some(4096));
        assert_eq!(request.model_role_label, "reviewer");
        assert_eq!(request.routing_preference_label, "prefer_fast");
        assert_eq!(request.endpoint_label, "auto");
        assert!(!request.endpoint_pinned);
        assert!(request.stream);
        assert_eq!(control.session_policy.history_messages, 3);
        assert_eq!(control.session_policy.history_limit, 4);
        assert_eq!(control.session_policy.max_tokens, Some(4096));
        assert_eq!(control.session_policy.max_tokens_label, "4096");
        assert_eq!(control.session_policy.history_remaining, 1);
        assert!(!control.session_policy.history_at_limit);
        assert_eq!(control.readiness.enter_action, InputActionKind::Blocked);
        assert!(!control.readiness.records_user_on_enter);
        assert!(!control.readiness.starts_stream_on_enter);
        assert!(control.readiness.preserves_buffer_on_enter);
        assert!(!control.readiness.clears_buffer_on_enter);
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(session.history().len(), 3);
        assert_eq!(session.partial_answer(), "partial");
        assert_eq!(input.buffer(), "next question");
    }

    #[test]
    fn control_snapshot_session_policy_exposes_display_ready_context_limits() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::new(2));
        session.record_user("one");
        session.record_assistant("two");
        let input = CliInput::default();

        let control = input.control_snapshot(&session, InputSubmitMode::Preview);

        assert_eq!(control.session_policy.history_messages, 2);
        assert_eq!(control.session_policy.history_limit, 2);
        assert!(control.session_policy.has_history);
        assert!(!control.session_policy.is_empty_history);
        assert_eq!(control.session_policy.max_tokens, None);
        assert_eq!(control.session_policy.max_tokens_label, "backend-default");
        assert_eq!(control.session_policy.history_remaining, 0);
        assert!(control.session_policy.history_at_limit);
        assert_eq!(
            control.status.line(),
            "role=assistant preference=balanced endpoint=auto pinned=false state=pending history=2 max_tokens=backend-default partial_chars=0"
        );
        assert_eq!(control.route_gate_advice, None);
        assert_eq!(control.route_gate_advice_detail, None);
        assert_eq!(control.route_gate_advice_action_label, None);
        assert_eq!(control.route_gate_advice_state_label, None);
        assert_eq!(control.route_gate_advice_reason, None);
        assert_eq!(control.send_block_reason, None);
        assert_eq!(control.route_send_allowed, None);
        assert_eq!(control.route_send_block_state, None);
        assert_eq!(control.route_send_block_state_label, None);
        assert_eq!(control.route_send_block_reason, None);
        assert_eq!(control.pool_status, None);
        assert_eq!(control.pool_queue_label, None);
        assert_eq!(control.pool_has_workers, None);
        assert_eq!(control.pool_has_available_workers, None);
        assert_eq!(control.pool_has_busy_workers, None);
        assert_eq!(control.pool_has_saturated_workers, None);
        assert_eq!(control.pool_has_queued_requests, None);
        assert_eq!(control.pool_queue_is_saturated, None);
        assert_eq!(control.pool_capacity_state, None);
        assert_eq!(control.pool_capacity_state_label, None);
        assert_eq!(control.pool_capacity_state_is_pressure, None);
        assert_eq!(control.pool_capacity_state_blocks_prompt_submit, None);
        assert_eq!(control.route_pool_status, None);
        assert_eq!(control.route_pool_queue_label, None);
        assert_eq!(control.route_pool_capacity_state, None);
        assert_eq!(control.route_pool_capacity_state_label, None);
        assert_eq!(control.route_pool_capacity_state_is_pressure, None);
        assert_eq!(control.route_pool_capacity_state_blocks_prompt_submit, None);
        assert_eq!(control.route_pool_has_matching_workers, None);
        assert_eq!(control.route_pool_has_matching_available_workers, None);
        assert_eq!(control.route_pool_has_matching_busy_workers, None);
        assert_eq!(control.route_pool_has_matching_saturated_workers, None);
        assert_eq!(control.route_pool_has_matching_queued_requests, None);
        assert_eq!(control.route_pool_queue_is_saturated, None);
        assert_eq!(control.workers, None);
        assert_eq!(control.route_workers, None);
    }

    #[test]
    fn control_snapshot_request_preview_keeps_context_messages_distinct_from_backend_default_tokens()
     {
        let mut session = ChatSession::new("cli", ChatSessionConfig::new(2));
        session.record_user("one");
        session.record_assistant("two");
        let mut input = CliInput::default();
        for ch in "three".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let control = input.control_snapshot(&session, InputSubmitMode::StartStream);

        assert!(control.send_enabled);
        assert!(control.enter_submits_prompt);
        assert_eq!(control.primary_action_label, "send");
        assert_eq!(control.session_policy.history_messages, 2);
        assert_eq!(control.session_policy.history_limit, 2);
        assert_eq!(control.session_policy.max_tokens, None);
        assert_eq!(control.session_policy.max_tokens_label, "backend-default");
        assert_eq!(control.session_policy.history_remaining, 0);
        assert!(control.session_policy.history_at_limit);

        let request = control
            .request_preview
            .as_ref()
            .expect("prompt control should preview the next outbound request");
        assert_eq!(request.messages, 3);
        assert_eq!(request.context_messages, 2);
        assert_eq!(request.history_messages, 2);
        assert_eq!(request.history_limit, Some(2));
        assert_eq!(request.history_remaining, Some(0));
        assert_eq!(request.history_messages_after_submit, Some(2));
        assert_eq!(request.history_at_limit_after_submit, Some(true));
        assert_eq!(request.history_truncates_on_submit, Some(true));
        assert_eq!(request.context_kind, ChatRequestContextKind::MultiTurn);
        assert!(request.has_context);
        assert!(!request.is_single_turn);
        assert_eq!(request.max_tokens, None);
        assert_eq!(request.max_tokens_label, "backend-default");
        assert!(!request.wire_sends_max_tokens);
        assert_eq!(request.wire_max_tokens, None);
        assert!(request.stream);

        let prompt_control = control
            .prompt_submit_control
            .as_ref()
            .expect("prompt control should expose send enablement");
        assert!(prompt_control.prompt_present);
        assert!(prompt_control.send_allowed);
        assert_eq!(prompt_control.action, GateAdviceAction::SendNow);
        assert_eq!(prompt_control.state, StreamState::Pending);
        assert_eq!(prompt_control.primary_action_disabled_reason, None);
        assert!(!prompt_control.preserves_prompt);
        assert!(prompt_control.clears_prompt);
        assert_eq!(session.history().len(), 2);
        assert!(session.chunks().is_empty());
    }

    #[test]
    fn start_stream_action_snapshot_keeps_context_policy_distinct_from_backend_default_tokens() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::new(2));
        session.record_user("one");
        session.record_assistant("two");
        let mut input = CliInput::default();
        for ch in "three".chars() {
            input.handle_key_starting(KeyInput::Char(ch), &mut session);
        }

        let action = input.handle_key_starting(KeyInput::Enter, &mut session);
        let snapshot = input.action_snapshot(&action);

        assert_eq!(snapshot.kind, InputActionKind::StartStream);
        assert_eq!(snapshot.kind_label, "start_stream");
        let request = snapshot
            .request
            .as_ref()
            .expect("start stream should expose outbound request metadata");
        assert_eq!(request.messages, 3);
        assert_eq!(request.context_messages, 2);
        assert_eq!(request.messages, request.context_messages + 1);
        assert_eq!(request.history_messages, 2);
        assert_eq!(request.history_limit, None);
        assert_eq!(request.history_remaining, None);
        assert_eq!(request.history_messages_after_submit, None);
        assert_eq!(request.history_at_limit_after_submit, None);
        assert_eq!(request.history_truncates_on_submit, None);
        assert_eq!(request.context_kind, ChatRequestContextKind::MultiTurn);
        assert_eq!(request.context_kind_label, "multi_turn");
        assert!(request.has_context);
        assert!(!request.is_single_turn);
        assert_eq!(request.last_message_role_label.as_deref(), Some("user"));
        assert_eq!(request.last_message_chars, 5);
        assert!(request.last_message_is_user);
        assert_eq!(request.last_user_chars, 5);
        assert_eq!(request.max_tokens, None);
        assert_eq!(request.max_tokens_label, "backend-default");
        assert!(!request.wire_sends_max_tokens);
        assert_eq!(request.wire_max_tokens, None);
        assert!(request.stream);
        assert_eq!(snapshot.local_status, None);
        assert_eq!(snapshot.reason, None);
        assert_eq!(snapshot.stream_state, None);
        assert_eq!(snapshot.stream_state_label, None);
        assert_eq!(snapshot.stream_chunk, None);
        assert_eq!(snapshot.start_sequence, Some(0));
        assert_eq!(snapshot.start_state, Some(StreamState::Streaming));
        assert_eq!(snapshot.start_state_label.as_deref(), Some("streaming"));
        assert_eq!(snapshot.start_state_is_terminal, Some(false));
        assert_eq!(snapshot.start_state_is_pressure, Some(false));
        assert_eq!(snapshot.start_state_blocks_prompt_submit, Some(true));
        let start_chunk = snapshot
            .start_chunk
            .as_ref()
            .expect("start stream should expose the streaming start chunk");
        assert_eq!(start_chunk.sequence, 0);
        assert_eq!(start_chunk.state, StreamState::Streaming);
        assert_eq!(start_chunk.output_label, "start");
        assert_eq!(start_chunk.state_label, "streaming");
        assert_eq!(session.state(), StreamState::Streaming);
        assert_eq!(
            session
                .history()
                .iter()
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>(),
            vec!["two", "three"]
        );
        assert_eq!(input.buffer(), "");
    }

    #[test]
    fn readiness_snapshot_matches_recording_and_starting_enter_modes() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "hello".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let preview = input.readiness(&session);
        let recording = input.readiness_recording(&session);
        let starting = input.readiness_starting(&session);

        assert_eq!(preview.submit_mode, InputSubmitMode::Preview);
        assert_eq!(preview.submit_mode_label, "preview");
        assert_eq!(preview.enter_action, InputActionKind::Send);
        assert!(preview.enter_submits_prompt);
        assert!(!preview.enter_runs_local_command);
        assert!(!preview.enter_is_blocked);
        assert_eq!(preview.primary_action_label, "send");
        assert!(preview.primary_action_enabled);
        assert_eq!(preview.primary_action_disabled_reason, None);
        assert!(!preview.preserves_buffer_on_enter);
        assert!(preview.clears_buffer_on_enter);
        assert!(!preview.records_user_on_enter);
        assert!(!preview.starts_stream_on_enter);
        assert_eq!(recording.submit_mode, InputSubmitMode::Record);
        assert_eq!(recording.submit_mode_label, "record");
        assert_eq!(recording.enter_action, InputActionKind::Send);
        assert!(recording.enter_submits_prompt);
        assert_eq!(recording.primary_action_label, "send");
        assert!(recording.primary_action_enabled);
        assert!(!recording.preserves_buffer_on_enter);
        assert!(recording.clears_buffer_on_enter);
        assert!(recording.records_user_on_enter);
        assert!(!recording.starts_stream_on_enter);
        assert_eq!(starting.submit_mode, InputSubmitMode::StartStream);
        assert_eq!(starting.submit_mode_label, "start_stream");
        assert_eq!(starting.enter_action, InputActionKind::StartStream);
        assert!(starting.enter_submits_prompt);
        assert_eq!(starting.primary_action_label, "send");
        assert!(starting.primary_action_enabled);
        assert!(!starting.preserves_buffer_on_enter);
        assert!(starting.clears_buffer_on_enter);
        assert!(starting.records_user_on_enter);
        assert!(starting.starts_stream_on_enter);

        let mut busy_session = ChatSession::new("cli", ChatSessionConfig::default());
        busy_session
            .try_submit_and_begin_stream("first")
            .expect("expected active stream");
        let blocked_start = input.readiness_starting(&busy_session);

        assert_eq!(blocked_start.enter_action, InputActionKind::Blocked);
        assert!(!blocked_start.enter_submits_prompt);
        assert!(!blocked_start.enter_runs_local_command);
        assert!(blocked_start.enter_is_blocked);
        assert_eq!(
            blocked_start.primary_action_label,
            "wait_for_current_stream"
        );
        assert!(!blocked_start.primary_action_enabled);
        assert_eq!(
            blocked_start.primary_action_disabled_reason.as_deref(),
            Some("session stream is already active")
        );
        assert!(blocked_start.preserves_buffer_on_enter);
        assert!(!blocked_start.clears_buffer_on_enter);
        assert!(!blocked_start.records_user_on_enter);
        assert!(!blocked_start.starts_stream_on_enter);
        assert_eq!(blocked_start.block_state, Some(StreamState::Busy));
    }

    #[test]
    fn readiness_snapshot_uses_frontend_gate_but_keeps_local_commands_available() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = FrontendGateSnapshot {
            engine_busy: true,
            active_request: Some("#21 chat-stream".to_owned()),
            ..FrontendGateSnapshot::default()
        };
        let mut input = CliInput::default();
        for ch in "hello".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let blocked = input.readiness_with_gate(&session, &gate);

        assert_eq!(blocked.enter_action, InputActionKind::Blocked);
        assert!(blocked.enter_is_blocked);
        assert_eq!(blocked.primary_action_label, "wait_for_current_stream");
        assert!(!blocked.primary_action_enabled);
        assert_eq!(
            blocked.primary_action_disabled_reason.as_deref(),
            Some("backend engine is busy: #21 chat-stream")
        );
        assert!(blocked.preserves_buffer_on_enter);
        assert!(!blocked.clears_buffer_on_enter);
        assert_eq!(blocked.block_state, Some(StreamState::Busy));
        assert_eq!(
            blocked.block_reason.as_deref(),
            Some("backend engine is busy: #21 chat-stream")
        );

        input.clear();
        for ch in "/status".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let status = input.readiness_with_gate(&session, &gate);

        assert_eq!(status.buffer_kind, InputBufferKind::StatusCommand);
        assert_eq!(status.enter_action, InputActionKind::Status);
        assert!(status.enter_runs_local_command);
        assert_eq!(status.primary_action_label, "show_status");
        assert!(status.primary_action_enabled);
        assert!(!status.preserves_buffer_on_enter);
        assert!(status.clears_buffer_on_enter);
        assert_eq!(status.advice_action, None);
        assert_eq!(status.block_state, None);
        assert!(!status.send_allowed);

        let offline_gate = FrontendGateSnapshot {
            backend_online: false,
            ..FrontendGateSnapshot::default()
        };
        input.clear();
        let empty = input.readiness_with_gate(&session, &offline_gate);

        assert_eq!(empty.buffer_kind, InputBufferKind::Empty);
        assert_eq!(empty.buffer_kind_label, "empty");
        assert_eq!(empty.enter_action, InputActionKind::Noop);
        assert_eq!(empty.enter_action_label, "noop");
        assert!(!empty.enter_enabled);
        assert!(!empty.enter_submits_prompt);
        assert!(!empty.enter_runs_local_command);
        assert!(!empty.enter_is_blocked);
        assert_eq!(empty.primary_action_label, "type_prompt");
        assert!(!empty.primary_action_enabled);
        assert_eq!(
            empty.primary_action_disabled_reason.as_deref(),
            Some("empty input")
        );
        assert_eq!(empty.advice_action, None);
        assert_eq!(empty.advice_action_label, None);
        assert_eq!(empty.block_state, None);
        assert_eq!(empty.block_reason, None);
        assert_eq!(empty.request_preview, None);
        let empty_control = empty
            .prompt_submit_control
            .as_ref()
            .expect("empty input should still expose prompt submit affordance state");
        assert!(!empty_control.prompt_present);
        assert!(!empty_control.send_allowed);
        assert_eq!(empty_control.primary_action_label, "type_prompt");
        assert_eq!(
            empty_control.primary_action_disabled_reason.as_deref(),
            Some("empty input")
        );
        assert_eq!(empty_control.block_chunk, None);
        assert!(!empty_control.preserves_prompt);
        assert!(!empty_control.clears_prompt);
        assert!(!empty.records_user_on_enter);
        assert!(!empty.starts_stream_on_enter);

        let noop =
            input.handle_key_with_gate_and_start(KeyInput::Enter, &mut session, &offline_gate);
        let noop_snapshot = input.action_snapshot(&noop);
        assert_eq!(noop, InputAction::Noop);
        assert_eq!(noop_snapshot.kind, InputActionKind::Noop);
        assert_eq!(noop_snapshot.kind_label, "noop");
        assert_eq!(noop_snapshot.request, None);
        assert_eq!(noop_snapshot.local_status, None);
        assert_eq!(noop_snapshot.start_chunk, None);
        assert_eq!(noop_snapshot.start_state, None);
        assert_eq!(noop_snapshot.stream_chunk, None);
        assert_eq!(noop_snapshot.stream_state, None);
        assert_eq!(noop_snapshot.reason, None);
        assert_eq!(input.buffer(), "");
        assert_eq!(session.state(), StreamState::Pending);
        assert!(session.history().is_empty());
        assert!(session.chunks().is_empty());

        input.clear();
        for ch in "hello".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let offline_blocked = input.readiness_with_gate(&session, &offline_gate);

        assert_eq!(offline_blocked.enter_action, InputActionKind::Blocked);
        assert!(offline_blocked.enter_is_blocked);
        assert_eq!(offline_blocked.primary_action_label, "repair_gate");
        assert!(!offline_blocked.primary_action_enabled);
        assert_eq!(
            offline_blocked.primary_action_disabled_reason.as_deref(),
            Some("backend is offline")
        );
        assert_eq!(offline_blocked.block_state, Some(StreamState::Failed));
        assert_eq!(
            offline_blocked.block_reason.as_deref(),
            Some("backend is offline")
        );
        assert!(offline_blocked.preserves_buffer_on_enter);
        assert!(!offline_blocked.clears_buffer_on_enter);

        input.clear();
        for ch in "/model reviewer fast".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let route = input.readiness_with_gate(&session, &offline_gate);

        assert_eq!(route.buffer_kind, InputBufferKind::RoutingCommand);
        assert_eq!(route.enter_action, InputActionKind::RoutingChanged);
        assert_eq!(route.enter_action_label, "routing_changed");
        assert!(route.enter_runs_local_command);
        assert!(!route.enter_submits_prompt);
        assert!(!route.enter_is_blocked);
        assert_eq!(route.primary_action_label, "apply_route");
        assert!(route.primary_action_enabled);
        assert_eq!(route.block_state, None);
        assert_eq!(route.request_preview, None);
        assert_eq!(
            route
                .command_preview
                .as_ref()
                .and_then(|preview| preview.local_status.as_deref()),
            Some("role=reviewer preference=prefer_fast endpoint=auto pinned=false")
        );

        input.clear();
        for ch in "/max-tokens auto".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let config = input.readiness_with_gate(&session, &offline_gate);

        assert_eq!(config.buffer_kind, InputBufferKind::SessionConfigCommand);
        assert_eq!(config.enter_action, InputActionKind::SessionConfigChanged);
        assert_eq!(config.enter_action_label, "session_config_changed");
        assert!(config.enter_runs_local_command);
        assert!(!config.enter_submits_prompt);
        assert!(!config.enter_is_blocked);
        assert_eq!(config.primary_action_label, "apply_config");
        assert!(config.primary_action_enabled);
        assert_eq!(config.block_state, None);
        assert_eq!(config.request_preview, None);
        assert_eq!(
            config
                .command_preview
                .as_ref()
                .and_then(|preview| preview.session_config_update.clone()),
            Some(SessionConfigUpdate::DefaultMaxTokens(None))
        );

        input.clear();
        for ch in "/history-limit 2".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let history_config = input.readiness_with_gate(&session, &offline_gate);
        let history_preview = history_config
            .command_preview
            .as_ref()
            .expect("history-limit should preview a local config update");
        let history_update = history_preview
            .session_config_update_detail
            .as_ref()
            .expect("history-limit should expose structured update metadata");

        assert_eq!(
            history_config.buffer_kind,
            InputBufferKind::SessionConfigCommand
        );
        assert_eq!(
            history_config.enter_action,
            InputActionKind::SessionConfigChanged
        );
        assert_eq!(history_config.enter_action_label, "session_config_changed");
        assert!(history_config.enter_runs_local_command);
        assert!(!history_config.enter_submits_prompt);
        assert!(!history_config.enter_is_blocked);
        assert_eq!(history_config.primary_action_label, "apply_config");
        assert!(history_config.primary_action_enabled);
        assert_eq!(history_config.block_state, None);
        assert_eq!(history_config.request_preview, None);
        assert_eq!(history_config.prompt_submit_control, None);
        assert_eq!(
            history_preview.session_config_update,
            Some(SessionConfigUpdate::HistoryLimit(2))
        );
        assert_eq!(history_update.kind_label, "history_limit");
        assert_eq!(history_update.summary, "history_limit=2");
        assert!(!history_update.changes_max_tokens);
        assert!(history_update.changes_history_limit);
        assert_eq!(history_update.max_tokens, None);
        assert_eq!(history_update.max_tokens_label, None);
        assert!(!history_update.max_tokens_backend_default);
        assert_eq!(history_update.history_limit, Some(2));
        assert_eq!(
            history_preview.local_status.as_deref(),
            Some("history_limit=2")
        );

        input.clear();
        for ch in "/model reviewer speedy".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let invalid_route = input.readiness_with_gate(&session, &offline_gate);
        let invalid_route_preview = invalid_route
            .command_preview
            .as_ref()
            .expect("invalid route command should still preview a command error");

        assert_eq!(invalid_route.buffer_kind, InputBufferKind::InvalidCommand);
        assert_eq!(invalid_route.enter_action, InputActionKind::InputError);
        assert_eq!(invalid_route.enter_action_label, "input_error");
        assert!(!invalid_route.enter_submits_prompt);
        assert!(!invalid_route.enter_runs_local_command);
        assert!(!invalid_route.enter_is_blocked);
        assert_eq!(invalid_route.primary_action_label, "fix_command");
        assert!(!invalid_route.primary_action_enabled);
        assert_eq!(
            invalid_route.primary_action_disabled_reason.as_deref(),
            Some("unknown routing preference: speedy")
        );
        assert_eq!(invalid_route.block_state, None);
        assert_eq!(invalid_route.block_reason, None);
        assert_eq!(invalid_route.request_preview, None);
        assert_eq!(
            invalid_route_preview.error.as_deref(),
            Some("unknown routing preference: speedy")
        );
        assert_eq!(invalid_route_preview.local_status, None);
        assert_eq!(invalid_route_preview.routing_intent, None);

        input.clear();
        for ch in "/max-tokens many".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }
        let invalid_config = input.readiness_with_gate(&session, &offline_gate);
        let invalid_config_preview = invalid_config
            .command_preview
            .as_ref()
            .expect("invalid config command should still preview a command error");

        assert_eq!(invalid_config.buffer_kind, InputBufferKind::InvalidCommand);
        assert_eq!(invalid_config.enter_action, InputActionKind::InputError);
        assert_eq!(invalid_config.enter_action_label, "input_error");
        assert!(!invalid_config.enter_submits_prompt);
        assert!(!invalid_config.enter_runs_local_command);
        assert!(!invalid_config.enter_is_blocked);
        assert_eq!(invalid_config.primary_action_label, "fix_command");
        assert!(!invalid_config.primary_action_enabled);
        assert_eq!(
            invalid_config.primary_action_disabled_reason.as_deref(),
            Some("max token budget must be a positive integer")
        );
        assert_eq!(invalid_config.block_state, None);
        assert_eq!(invalid_config.block_reason, None);
        assert_eq!(invalid_config.request_preview, None);
        assert_eq!(
            invalid_config_preview.error.as_deref(),
            Some("max token budget must be a positive integer")
        );
        assert_eq!(invalid_config_preview.local_status, None);
        assert_eq!(invalid_config_preview.session_config_update, None);
    }

    #[test]
    fn config_command_aliases_stay_local_under_combined_frontend_gate() {
        let gate = FrontendGateSnapshot {
            backend_online: false,
            engine_busy: true,
            safe_device_ok: false,
            experience_hygiene_ok: false,
            queued_requests: 3,
            queue_limit: 3,
            active_request: Some("#55 chat-stream".to_owned()),
        };

        for (command, expected_update, expected_status) in [
            (
                "/tokens auto",
                SessionConfigUpdate::DefaultMaxTokens(None),
                "max_tokens=backend-default",
            ),
            (
                "/tokens off",
                SessionConfigUpdate::DefaultMaxTokens(None),
                "max_tokens=backend-default",
            ),
            (
                "/tokens 2048",
                SessionConfigUpdate::DefaultMaxTokens(Some(2048)),
                "max_tokens=2048",
            ),
            (
                "/history 3",
                SessionConfigUpdate::HistoryLimit(3),
                "history_limit=3",
            ),
            (
                "/history-limit 4",
                SessionConfigUpdate::HistoryLimit(4),
                "history_limit=4",
            ),
        ] {
            let mut session = ChatSession::new("cli", ChatSessionConfig::default());
            let mut input = CliInput::default();
            for ch in command.chars() {
                input.handle_key_with_gate_and_start(KeyInput::Char(ch), &mut session, &gate);
            }

            let readiness = input.readiness_with_gate_and_start(&session, &gate);

            assert_eq!(
                readiness.buffer_kind,
                InputBufferKind::SessionConfigCommand,
                "{command}"
            );
            assert_eq!(
                readiness.enter_action,
                InputActionKind::SessionConfigChanged,
                "{command}"
            );
            assert_eq!(
                readiness.enter_action_label, "session_config_changed",
                "{command}"
            );
            assert!(readiness.enter_runs_local_command, "{command}");
            assert!(!readiness.enter_submits_prompt, "{command}");
            assert!(!readiness.enter_is_blocked, "{command}");
            assert_eq!(readiness.primary_action_label, "apply_config", "{command}");
            assert!(readiness.primary_action_enabled, "{command}");
            assert_eq!(readiness.primary_action_disabled_reason, None, "{command}");
            assert_eq!(readiness.block_state, None, "{command}");
            assert_eq!(readiness.block_reason, None, "{command}");
            assert_eq!(readiness.request_preview, None, "{command}");
            assert_eq!(readiness.prompt_submit_control, None, "{command}");
            assert!(!readiness.records_user_on_enter, "{command}");
            assert!(!readiness.starts_stream_on_enter, "{command}");

            let preview = readiness
                .command_preview
                .as_ref()
                .expect("config alias should preview a local config update");
            assert_eq!(
                preview.session_config_update,
                Some(expected_update.clone()),
                "{command}"
            );
            assert_eq!(
                preview.local_status.as_deref(),
                Some(expected_status),
                "{command}"
            );
            let detail = preview
                .session_config_update_detail
                .as_ref()
                .expect("config alias should expose structured update metadata");
            assert_eq!(detail.summary, expected_status, "{command}");

            let action = input.handle_key_with_gate_and_start(KeyInput::Enter, &mut session, &gate);
            let snapshot = input.action_snapshot(&action);

            assert_eq!(
                action,
                InputAction::SessionConfigChanged {
                    update: expected_update.clone(),
                    summary: expected_status.to_owned()
                },
                "{command}"
            );
            assert_eq!(
                snapshot.kind,
                InputActionKind::SessionConfigChanged,
                "{command}"
            );
            assert_eq!(
                snapshot.session_config_update,
                Some(expected_update),
                "{command}"
            );
            assert_eq!(
                snapshot.local_status.as_deref(),
                Some(expected_status),
                "{command}"
            );
            assert_eq!(snapshot.request, None, "{command}");
            assert!(input.buffer().is_empty(), "{command}");
            assert!(session.history().is_empty(), "{command}");
            assert_eq!(session.state(), StreamState::Pending, "{command}");
            assert!(session.chunks().is_empty(), "{command}");
        }
    }

    #[test]
    fn status_command_aliases_stay_local_read_only_under_gates() {
        let gates = [
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#44 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            FrontendGateSnapshot {
                backend_online: false,
                ..FrontendGateSnapshot::default()
            },
        ];

        for gate in gates {
            for (command, buffer_kind, primary_action_label) in [
                ("/status", InputBufferKind::StatusCommand, "show_status"),
                ("/state", InputBufferKind::StatusCommand, "show_status"),
                (
                    "/workers",
                    InputBufferKind::WorkerStatusCommand,
                    "show_workers",
                ),
                (
                    "/worker-status",
                    InputBufferKind::WorkerStatusCommand,
                    "show_workers",
                ),
                (
                    "/endpoints",
                    InputBufferKind::WorkerStatusCommand,
                    "show_workers",
                ),
            ] {
                let mut session = ChatSession::new("cli", ChatSessionConfig::default());
                let mut input = CliInput::default();
                for ch in command.chars() {
                    input.handle_key_with_gate_and_start(KeyInput::Char(ch), &mut session, &gate);
                }

                let readiness = input.readiness_with_gate(&session, &gate);

                assert_eq!(readiness.buffer_kind, buffer_kind, "{command}");
                assert_eq!(readiness.enter_action, InputActionKind::Status, "{command}");
                assert_eq!(readiness.enter_action_label, "status", "{command}");
                assert!(readiness.enter_runs_local_command, "{command}");
                assert!(!readiness.enter_submits_prompt, "{command}");
                assert!(!readiness.enter_is_blocked, "{command}");
                assert_eq!(
                    readiness.primary_action_label, primary_action_label,
                    "{command}"
                );
                assert!(readiness.primary_action_enabled, "{command}");
                assert_eq!(readiness.primary_action_disabled_reason, None, "{command}");
                assert_eq!(readiness.block_state, None, "{command}");
                assert_eq!(readiness.block_reason, None, "{command}");
                assert_eq!(readiness.request_preview, None, "{command}");
                assert_eq!(readiness.prompt_submit_control, None, "{command}");
                assert!(!readiness.preserves_buffer_on_enter, "{command}");
                assert!(readiness.clears_buffer_on_enter, "{command}");
                assert!(!readiness.records_user_on_enter, "{command}");
                assert!(!readiness.starts_stream_on_enter, "{command}");

                let action =
                    input.handle_key_with_gate_and_start(KeyInput::Enter, &mut session, &gate);
                let snapshot = input.action_snapshot(&action);

                assert!(
                    matches!(action, InputAction::Status(_)),
                    "{command} must stay a local status action"
                );
                assert_eq!(snapshot.kind, InputActionKind::Status, "{command}");
                assert_eq!(snapshot.kind_label, "status", "{command}");
                assert!(snapshot.local_status.is_some(), "{command}");
                assert_eq!(snapshot.request, None, "{command}");
                assert_eq!(snapshot.session_config_update, None, "{command}");
                assert_eq!(snapshot.stream_state, None, "{command}");
                assert_eq!(snapshot.stream_chunk, None, "{command}");
                assert_eq!(snapshot.reason, None, "{command}");
                assert_eq!(snapshot.start_sequence, None, "{command}");
                assert_eq!(snapshot.start_state, None, "{command}");
                assert_eq!(snapshot.start_chunk, None, "{command}");
                assert_eq!(input.buffer(), "", "{command}");
                assert_eq!(session.state(), StreamState::Pending, "{command}");
                assert!(session.history().is_empty(), "{command}");
                assert!(session.chunks().is_empty(), "{command}");
                assert_eq!(session.partial_answer(), "", "{command}");
                assert_eq!(session.last_error(), None, "{command}");
            }
        }
    }

    #[test]
    fn model_pool_status_and_workers_stay_read_only_under_engine_busy_and_health_preflight() {
        let gates = [
            (
                "engine_busy",
                StreamState::Busy,
                "backend engine is busy: #287 chat-stream",
                ModelPoolGateSnapshot::new(
                    FrontendGateSnapshot {
                        engine_busy: true,
                        active_request: Some("#287 chat-stream".to_owned()),
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
                "health_preflight",
                StreamState::Failed,
                "backend is offline",
                ModelPoolGateSnapshot::new(
                    FrontendGateSnapshot {
                        backend_online: false,
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

        for (gate_label, expected_block_state, expected_reason, gate) in gates {
            for (command, buffer_kind, primary_action_label) in [
                ("/status", InputBufferKind::StatusCommand, "show_status"),
                ("/state", InputBufferKind::StatusCommand, "show_status"),
                (
                    "/workers",
                    InputBufferKind::WorkerStatusCommand,
                    "show_workers",
                ),
                (
                    "/worker-status",
                    InputBufferKind::WorkerStatusCommand,
                    "show_workers",
                ),
                (
                    "/endpoints",
                    InputBufferKind::WorkerStatusCommand,
                    "show_workers",
                ),
            ] {
                let config = CliInputConfig::default()
                    .with_model_role(ModelRole::Reviewer)
                    .with_routing_preference(RoutingPreference::PreferFast);
                let mut session = ChatSession::new("cli", ChatSessionConfig::default());
                let expected_status =
                    CliStatusSnapshot::from_model_pool_gate(&config, &session, &gate);
                let mut input = CliInput::new(config);
                for ch in command.chars() {
                    input.handle_key_with_model_pool_gate_and_start(
                        KeyInput::Char(ch),
                        &mut session,
                        &gate,
                    );
                }

                let control = input.control_snapshot_with_model_pool_gate(
                    &session,
                    &gate,
                    InputSubmitMode::StartStream,
                );

                assert_eq!(
                    control.readiness.buffer_kind, buffer_kind,
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.readiness.enter_action,
                    InputActionKind::Status,
                    "{gate_label} {command}"
                );
                assert!(control.enter_runs_local_command, "{gate_label} {command}");
                assert!(!control.enter_submits_prompt, "{gate_label} {command}");
                assert!(!control.enter_is_blocked, "{gate_label} {command}");
                assert_eq!(
                    control.primary_action_label, primary_action_label,
                    "{gate_label} {command}"
                );
                assert!(control.primary_action_enabled, "{gate_label} {command}");
                assert_eq!(
                    control.primary_action_disabled_reason, None,
                    "{gate_label} {command}"
                );
                assert_eq!(control.request_preview, None, "{gate_label} {command}");
                assert_eq!(
                    control.prompt_submit_control, None,
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.route_send_allowed,
                    Some(false),
                    "{gate_label} {command}"
                );
                assert_eq!(control.status, expected_status, "{gate_label} {command}");
                assert_eq!(
                    control.block_state,
                    Some(expected_block_state),
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.block_reason.as_deref(),
                    Some(expected_reason),
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.route_send_block_state,
                    Some(expected_block_state),
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.route_send_block_reason.as_deref(),
                    Some(expected_reason),
                    "{gate_label} {command}"
                );

                let action = input.handle_key_with_model_pool_gate_and_start(
                    KeyInput::Enter,
                    &mut session,
                    &gate,
                );
                let snapshot = input.action_snapshot(&action);

                let InputAction::Status(line) = action else {
                    panic!("{gate_label} {command} should stay a local status action");
                };
                assert!(line.contains(expected_reason), "{gate_label} {command}");
                assert_eq!(
                    snapshot.kind,
                    InputActionKind::Status,
                    "{gate_label} {command}"
                );
                assert_eq!(snapshot.kind_label, "status", "{gate_label} {command}");
                assert_eq!(
                    snapshot.local_status.as_deref(),
                    Some(line.as_str()),
                    "{gate_label} {command}"
                );
                assert_eq!(snapshot.request, None, "{gate_label} {command}");
                assert_eq!(
                    snapshot.session_config_update, None,
                    "{gate_label} {command}"
                );
                assert_eq!(snapshot.stream_state, None, "{gate_label} {command}");
                assert_eq!(snapshot.stream_chunk, None, "{gate_label} {command}");
                assert_eq!(snapshot.reason, None, "{gate_label} {command}");
                assert_eq!(snapshot.start_sequence, None, "{gate_label} {command}");
                assert_eq!(snapshot.start_state, None, "{gate_label} {command}");
                assert_eq!(snapshot.start_chunk, None, "{gate_label} {command}");
                assert!(input.buffer().is_empty(), "{gate_label} {command}");
                assert_eq!(
                    session.state(),
                    StreamState::Pending,
                    "{gate_label} {command}"
                );
                assert!(session.history().is_empty(), "{gate_label} {command}");
                assert!(session.chunks().is_empty(), "{gate_label} {command}");
                assert_eq!(session.partial_answer(), "", "{gate_label} {command}");
                assert_eq!(session.last_error(), None, "{gate_label} {command}");
            }
        }
    }

    #[test]
    fn model_pool_status_commands_stay_read_only_when_route_is_backpressured() {
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

        for (command, buffer_kind, primary_action_label) in [
            ("/status", InputBufferKind::StatusCommand, "show_status"),
            ("/state", InputBufferKind::StatusCommand, "show_status"),
            (
                "/workers",
                InputBufferKind::WorkerStatusCommand,
                "show_workers",
            ),
            (
                "/worker-status",
                InputBufferKind::WorkerStatusCommand,
                "show_workers",
            ),
            (
                "/endpoints",
                InputBufferKind::WorkerStatusCommand,
                "show_workers",
            ),
        ] {
            let config = CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast);
            let mut session = ChatSession::new("cli", ChatSessionConfig::default());
            let expected_status = CliStatusSnapshot::from_model_pool_gate(&config, &session, &gate);
            let mut input = CliInput::new(config);
            for ch in command.chars() {
                input.handle_key_with_model_pool_gate_and_start(
                    KeyInput::Char(ch),
                    &mut session,
                    &gate,
                );
            }

            let control = input.control_snapshot_with_model_pool_gate(
                &session,
                &gate,
                InputSubmitMode::StartStream,
            );

            assert_eq!(control.readiness.buffer_kind, buffer_kind, "{command}");
            assert_eq!(
                control.readiness.enter_action,
                InputActionKind::Status,
                "{command}"
            );
            assert!(control.enter_runs_local_command, "{command}");
            assert!(!control.enter_submits_prompt, "{command}");
            assert!(!control.readiness.records_user_on_enter, "{command}");
            assert!(!control.readiness.starts_stream_on_enter, "{command}");
            assert_eq!(control.request_preview, None, "{command}");
            assert_eq!(control.prompt_submit_control, None, "{command}");
            assert_eq!(
                control.primary_action_label, primary_action_label,
                "{command}"
            );
            assert!(control.primary_action_enabled, "{command}");
            assert_eq!(control.primary_action_disabled_reason, None, "{command}");
            assert_eq!(control.route_send_allowed, Some(false), "{command}");
            assert_eq!(control.status, expected_status, "{command}");
            assert_eq!(
                control.block_state,
                Some(StreamState::Backpressure),
                "{command}"
            );
            assert_eq!(
                control.block_reason.as_deref(),
                Some("matching model workers are saturated: 1 workers"),
                "{command}"
            );
            assert_eq!(
                control.route_send_block_state,
                Some(StreamState::Backpressure),
                "{command}"
            );
            assert_eq!(
                control.route_send_block_reason.as_deref(),
                Some("matching model workers are saturated: 1 workers"),
                "{command}"
            );
            assert_eq!(
                control.route_pool_status.as_deref(),
                Some("matching total=1 available=0 busy=0 saturated=1"),
                "{command}"
            );
            assert_eq!(
                control.route_pool_has_matching_available_workers,
                Some(false),
                "{command}"
            );
            assert_eq!(control.pool_has_available_workers, Some(true), "{command}");
            let block_chunk = control
                .block_chunk
                .as_ref()
                .expect("route backpressure local status should expose block chunk");
            assert_eq!(block_chunk.output_label, "backpressure", "{command}");
            assert_eq!(
                block_chunk.appended,
                "[backpressure] matching model workers are saturated: 1 workers",
                "{command}"
            );
            assert_eq!(control.workers, expected_status.workers, "{command}");
            assert_eq!(
                control.route_workers, expected_status.route_workers,
                "{command}"
            );

            let workers = control
                .workers
                .as_ref()
                .expect("route backpressure local status should keep worker health rows");
            assert_eq!(workers.len(), 2, "{command}");
            assert_eq!(workers[0].endpoint_label(), "quality-12b", "{command}");
            assert_eq!(workers[0].status_label(), "available", "{command}");
            assert_eq!(workers[1].endpoint_label(), "fast-reviewer", "{command}");
            assert_eq!(workers[1].status_label(), "backpressure", "{command}");

            let route_workers = control
                .route_workers
                .as_ref()
                .expect("route backpressure local status should keep route worker rows");
            assert_eq!(route_workers.len(), 2, "{command}");
            assert_eq!(
                route_workers[0].endpoint_label(),
                "quality-12b",
                "{command}"
            );
            assert!(!route_workers[0].route_match, "{command}");
            assert!(!route_workers[0].selectable, "{command}");
            assert_eq!(
                route_workers[0].picker_action,
                ModelRouteWorkerPickerAction::Unavailable,
                "{command}"
            );
            assert_eq!(
                route_workers[0].picker_action_label, "unavailable",
                "{command}"
            );
            assert_eq!(
                route_workers[0].worker_status_label(),
                "available",
                "{command}"
            );
            assert_eq!(
                route_workers[1].endpoint_label(),
                "fast-reviewer",
                "{command}"
            );
            assert!(route_workers[1].route_match, "{command}");
            assert!(!route_workers[1].selectable, "{command}");
            assert_eq!(
                route_workers[1].picker_action,
                ModelRouteWorkerPickerAction::Wait,
                "{command}"
            );
            assert_eq!(route_workers[1].picker_action_label, "wait", "{command}");
            assert_eq!(
                route_workers[1].worker_status_label(),
                "backpressure",
                "{command}"
            );
            assert_eq!(
                route_workers[1].decision_action_label(),
                "retry_later",
                "{command}"
            );
            assert_eq!(
                route_workers[1].decision_state_label(),
                "backpressure",
                "{command}"
            );

            let action = input.handle_key_with_model_pool_gate_and_start(
                KeyInput::Enter,
                &mut session,
                &gate,
            );
            let snapshot = input.action_snapshot(&action);

            assert!(
                matches!(action, InputAction::Status(_)),
                "{command} must remain a local read-only status action"
            );
            assert_eq!(snapshot.kind, InputActionKind::Status, "{command}");
            if let InputAction::Status(line) = &action {
                assert!(
                    line.contains("matching model workers are saturated: 1 workers"),
                    "{command}"
                );
            }
            assert_eq!(snapshot.request, None, "{command}");
            assert_eq!(snapshot.start_state, None, "{command}");
            assert_eq!(snapshot.start_chunk, None, "{command}");
            assert!(input.buffer().is_empty(), "{command}");
            assert_eq!(session.state(), StreamState::Pending, "{command}");
            assert!(session.history().is_empty(), "{command}");
            assert!(session.chunks().is_empty(), "{command}");
            assert_eq!(session.partial_answer(), "", "{command}");
        }
    }

    #[test]
    fn model_pool_status_commands_preserve_structured_host_snapshot_under_gate_pressure() {
        let cases = [
            (
                "engine_busy",
                ModelPoolGateSnapshot::new(
                    FrontendGateSnapshot {
                        engine_busy: true,
                        active_request: Some("#287 chat-stream".to_owned()),
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
                ),
                StreamState::Busy,
                "backend engine is busy: #287 chat-stream",
                "busy",
                "[busy] backend engine is busy: #287 chat-stream",
                ModelRouteWorkerPickerAction::Wait,
                "wait",
            ),
            (
                "repair_gate",
                ModelPoolGateSnapshot::new(
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
                ),
                StreamState::Failed,
                "safe-device gate failed",
                "error",
                "[error] safe-device gate failed",
                ModelRouteWorkerPickerAction::RepairGate,
                "repair_gate",
            ),
        ];

        for (
            gate_label,
            gate,
            expected_block_state,
            expected_reason,
            expected_output_label,
            expected_block_line,
            expected_picker_action,
            expected_picker_label,
        ) in cases
        {
            for (command, buffer_kind, primary_action_label) in [
                ("/status", InputBufferKind::StatusCommand, "show_status"),
                (
                    "/workers",
                    InputBufferKind::WorkerStatusCommand,
                    "show_workers",
                ),
            ] {
                let config = CliInputConfig::default()
                    .with_model_role(ModelRole::Reviewer)
                    .with_routing_preference(RoutingPreference::PreferFast);
                let mut session = ChatSession::new("cli", ChatSessionConfig::default());
                let expected_status =
                    CliStatusSnapshot::from_model_pool_gate(&config, &session, &gate);
                let mut input = CliInput::new(config);
                for ch in command.chars() {
                    input.handle_key_with_model_pool_gate_and_start(
                        KeyInput::Char(ch),
                        &mut session,
                        &gate,
                    );
                }

                let control = input.control_snapshot_with_model_pool_gate(
                    &session,
                    &gate,
                    InputSubmitMode::StartStream,
                );

                assert_eq!(
                    control.readiness.buffer_kind, buffer_kind,
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.primary_action_label, primary_action_label,
                    "{gate_label} {command}"
                );
                assert!(!control.enter_submits_prompt, "{gate_label} {command}");
                assert!(control.enter_runs_local_command, "{gate_label} {command}");
                assert_eq!(control.request_preview, None, "{gate_label} {command}");
                assert_eq!(
                    control.prompt_submit_control, None,
                    "{gate_label} {command}"
                );
                assert_eq!(control.status, expected_status, "{gate_label} {command}");
                assert_eq!(
                    control.block_state,
                    Some(expected_block_state),
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.send_block_reason.as_deref(),
                    Some(expected_reason),
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.route_send_block_state,
                    Some(expected_block_state),
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.route_send_block_reason.as_deref(),
                    Some(expected_reason),
                    "{gate_label} {command}"
                );
                let block_chunk = control
                    .block_chunk
                    .as_ref()
                    .expect("local status command should still expose gate block chunk");
                assert_eq!(
                    block_chunk.output_label, expected_output_label,
                    "{gate_label} {command}"
                );
                assert_eq!(
                    block_chunk.appended, expected_block_line,
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.workers, expected_status.workers,
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.route_workers, expected_status.route_workers,
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.pool_status, expected_status.pool_status,
                    "{gate_label} {command}"
                );
                assert_eq!(
                    control.route_pool_status, expected_status.route_pool_status,
                    "{gate_label} {command}"
                );

                let workers = control
                    .workers
                    .as_ref()
                    .expect("status command control should keep worker health rows");
                assert_eq!(workers.len(), 2, "{gate_label} {command}");
                assert_eq!(
                    workers[0].status_label(),
                    "available",
                    "{gate_label} {command}"
                );
                assert_eq!(workers[1].status_label(), "busy", "{gate_label} {command}");

                let route_workers = control
                    .route_workers
                    .as_ref()
                    .expect("status command control should keep route worker rows");
                assert_eq!(route_workers.len(), 2, "{gate_label} {command}");
                assert!(
                    route_workers.iter().all(|worker| worker.route_match),
                    "{gate_label} {command}"
                );
                assert!(
                    route_workers.iter().all(|worker| !worker.selectable),
                    "{gate_label} {command}"
                );
                assert!(
                    route_workers
                        .iter()
                        .all(|worker| worker.picker_action == expected_picker_action),
                    "{gate_label} {command}"
                );
                assert!(
                    route_workers
                        .iter()
                        .all(|worker| worker.picker_action_label == expected_picker_label),
                    "{gate_label} {command}"
                );
            }
        }
    }

    #[test]
    fn readiness_snapshot_uses_model_pool_auto_route_and_pinned_worker_pressure() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("quality".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer),
            ],
        );
        let mut auto_input = CliInput::new(
            CliInputConfig::default()
                .with_model_role(ModelRole::Reviewer)
                .with_routing_preference(RoutingPreference::PreferFast),
        );
        for ch in "quick review".chars() {
            auto_input.handle_key(KeyInput::Char(ch), &session);
        }

        let auto_ready = auto_input.readiness_with_model_pool_gate(&session, &gate);
        let auto_start = auto_input.readiness_with_model_pool_gate_and_start(&session, &gate);

        assert_eq!(auto_ready.enter_action, InputActionKind::Send);
        assert!(auto_ready.enter_submits_prompt);
        assert_eq!(auto_ready.primary_action_label, "send");
        assert!(auto_ready.primary_action_enabled);
        assert_eq!(auto_ready.primary_action_disabled_reason, None);
        assert!(auto_ready.send_allowed);
        assert!(!auto_ready.routing_intent.endpoint_pinned);
        assert_eq!(auto_ready.model_role_label, "reviewer");
        assert_eq!(auto_ready.routing_preference_label, "prefer_fast");
        assert_eq!(auto_ready.endpoint_label, "auto");
        assert!(!auto_ready.endpoint_pinned);
        assert_eq!(auto_ready.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(auto_ready.endpoint_kind_label, "auto");
        assert!(auto_ready.endpoint_auto);
        assert!(!auto_ready.endpoint_built_in);
        assert!(!auto_ready.endpoint_custom);
        assert_eq!(auto_ready.wire_model_role_label, "reviewer");
        assert_eq!(auto_ready.wire_routing_preference_label, "prefer_fast");
        assert!(auto_ready.wire_prefer_fast);
        assert!(!auto_ready.wire_prefer_quality);
        assert!(!auto_ready.wire_endpoint_pinned);
        assert_eq!(auto_ready.wire_endpoint_kind_label, "auto");
        assert!(!auto_ready.wire_sends_model_endpoint);
        assert_eq!(auto_ready.wire_model_endpoint_label, None);
        assert_eq!(auto_start.enter_action, InputActionKind::StartStream);
        assert!(!auto_start.preserves_buffer_on_enter);
        assert!(auto_start.clears_buffer_on_enter);
        assert!(auto_start.records_user_on_enter);
        assert!(auto_start.starts_stream_on_enter);

        let mut pinned_input = CliInput::new(
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::Quality12B)),
        );
        for ch in "deep answer".chars() {
            pinned_input.handle_key(KeyInput::Char(ch), &session);
        }

        let pinned_blocked = pinned_input.readiness_with_model_pool_gate(&session, &gate);

        assert_eq!(pinned_blocked.enter_action, InputActionKind::Blocked);
        assert_eq!(pinned_blocked.block_state, Some(StreamState::Busy));
        assert!(pinned_blocked.enter_is_blocked);
        let pinned_block_chunk = pinned_blocked
            .block_chunk
            .as_ref()
            .expect("pinned worker pressure should expose service chunk display snapshot");
        assert_eq!(pinned_block_chunk.output_label, "busy");
        assert_eq!(
            pinned_block_chunk.appended,
            "[busy] worker quality-12b is busy: quality"
        );
        assert_eq!(
            pinned_blocked.primary_action_label,
            "wait_for_current_stream"
        );
        assert!(!pinned_blocked.primary_action_enabled);
        assert_eq!(
            pinned_blocked.primary_action_disabled_reason.as_deref(),
            Some("worker quality-12b is busy: quality")
        );
        assert!(pinned_blocked.preserves_buffer_on_enter);
        assert!(!pinned_blocked.clears_buffer_on_enter);
        assert_eq!(
            pinned_blocked.advice_action,
            Some(GateAdviceAction::WaitForCurrentStream)
        );
        assert_eq!(
            pinned_blocked.block_reason.as_deref(),
            Some("worker quality-12b is busy: quality")
        );
        assert!(!pinned_blocked.records_user_on_enter);
        assert!(!pinned_blocked.starts_stream_on_enter);
        assert!(pinned_blocked.routing_intent.endpoint_pinned);
        assert_eq!(pinned_blocked.endpoint_label, "quality-12b");
        assert!(pinned_blocked.endpoint_pinned);
        assert_eq!(
            pinned_blocked.endpoint_kind,
            ModelEndpointSelectionKind::BuiltIn
        );
        assert_eq!(pinned_blocked.endpoint_kind_label, "built_in");
        assert!(!pinned_blocked.endpoint_auto);
        assert!(pinned_blocked.endpoint_built_in);
        assert!(!pinned_blocked.endpoint_custom);
        assert!(pinned_blocked.wire_endpoint_pinned);
        assert_eq!(pinned_blocked.wire_endpoint_kind_label, "built_in");
        assert!(pinned_blocked.wire_sends_model_endpoint);
        assert_eq!(
            pinned_blocked.wire_model_endpoint_label.as_deref(),
            Some("quality-12b")
        );

        let offline_gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                backend_online: false,
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        auto_input.clear();
        for ch in "quick review".chars() {
            auto_input.handle_key(KeyInput::Char(ch), &session);
        }
        let offline_prompt = auto_input.readiness_with_model_pool_gate(&session, &offline_gate);

        assert_eq!(offline_prompt.enter_action, InputActionKind::Blocked);
        assert!(offline_prompt.enter_is_blocked);
        assert_eq!(offline_prompt.primary_action_label, "repair_gate");
        assert_eq!(offline_prompt.block_state, Some(StreamState::Failed));
        assert_eq!(
            offline_prompt.block_reason.as_deref(),
            Some("backend is offline")
        );
        assert!(offline_prompt.request_preview.is_some());

        auto_input.clear();
        for ch in "/workers".chars() {
            auto_input.handle_key(KeyInput::Char(ch), &session);
        }
        let workers = auto_input.readiness_with_model_pool_gate(&session, &offline_gate);

        assert_eq!(workers.buffer_kind, InputBufferKind::WorkerStatusCommand);
        assert_eq!(workers.buffer_kind_label, "worker_status_command");
        assert_eq!(workers.enter_action, InputActionKind::Status);
        assert_eq!(workers.enter_action_label, "status");
        assert!(workers.enter_runs_local_command);
        assert!(!workers.enter_submits_prompt);
        assert!(!workers.enter_is_blocked);
        assert_eq!(workers.primary_action_label, "show_workers");
        assert!(workers.primary_action_enabled);
        assert_eq!(workers.block_state, None);
        assert_eq!(workers.request_preview, None);

        auto_input.clear();
        for ch in "/model reviewer fast".chars() {
            auto_input.handle_key(KeyInput::Char(ch), &session);
        }
        let route = auto_input.readiness_with_model_pool_gate(&session, &offline_gate);

        assert_eq!(route.buffer_kind, InputBufferKind::RoutingCommand);
        assert_eq!(route.enter_action, InputActionKind::RoutingChanged);
        assert_eq!(route.primary_action_label, "apply_route");
        assert!(route.enter_runs_local_command);
        assert!(!route.enter_submits_prompt);
        assert!(!route.enter_is_blocked);
        assert_eq!(route.block_state, None);
        assert_eq!(route.request_preview, None);
        assert_eq!(
            route
                .command_preview
                .as_ref()
                .and_then(|preview| preview.local_status.as_deref()),
            Some("role=reviewer preference=prefer_fast endpoint=auto pinned=false")
        );
    }

    #[test]
    fn readiness_advice_action_maps_pressure_and_repair_buttons() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let mut input = CliInput::default();
        for ch in "hello".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let queued = input.readiness_with_gate(
            &session,
            &FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#7 queued-ish".to_owned()),
                ..FrontendGateSnapshot::default()
            },
        );

        assert_eq!(queued.block_state, Some(StreamState::Busy));
        assert_eq!(queued.block_state_label.as_deref(), Some("busy"));
        assert_eq!(
            queued.advice_action,
            Some(GateAdviceAction::WaitForCurrentStream)
        );
        assert_eq!(
            queued.advice_action_label.as_deref(),
            Some("wait_for_current_stream")
        );

        let mut queued_session = ChatSession::new("cli", ChatSessionConfig::default());
        queued_session.queued("waiting for worker");
        let queued_local = input.readiness(&queued_session);

        assert_eq!(queued_local.block_state, Some(StreamState::Queued));
        assert_eq!(queued_local.block_state_label.as_deref(), Some("queued"));
        assert!(queued_local.preserves_buffer_on_enter);
        assert!(!queued_local.clears_buffer_on_enter);
        assert_eq!(
            queued_local.advice_action,
            Some(GateAdviceAction::WaitForWorker)
        );
        assert_eq!(
            queued_local.advice_action_label.as_deref(),
            Some("wait_for_worker")
        );
        assert_eq!(queued_local.model_role_label, "assistant");
        assert_eq!(queued_local.routing_preference_label, "balanced");
        assert_eq!(queued_local.endpoint_label, "auto");
        assert!(!queued_local.endpoint_pinned);
        assert_eq!(queued_local.endpoint_kind_label, "auto");
        assert!(!queued_local.wire_endpoint_pinned);
        assert!(!queued_local.wire_sends_model_endpoint);
        assert_eq!(queued_local.wire_model_endpoint_label, None);

        let backpressure = input.readiness_with_gate(
            &session,
            &FrontendGateSnapshot {
                queued_requests: 2,
                queue_limit: 2,
                ..FrontendGateSnapshot::default()
            },
        );

        assert_eq!(backpressure.block_state, Some(StreamState::Backpressure));
        assert_eq!(
            backpressure.block_state_label.as_deref(),
            Some("backpressure")
        );
        assert!(backpressure.preserves_buffer_on_enter);
        assert!(!backpressure.clears_buffer_on_enter);
        assert_eq!(
            backpressure.advice_action,
            Some(GateAdviceAction::RetryLater)
        );
        assert_eq!(
            backpressure.advice_action_label.as_deref(),
            Some("retry_later")
        );
        assert_eq!(backpressure.model_role_label, "assistant");
        assert_eq!(backpressure.routing_preference_label, "balanced");
        assert_eq!(backpressure.endpoint_label, "auto");
        assert!(!backpressure.endpoint_pinned);
        assert_eq!(backpressure.endpoint_kind_label, "auto");
        assert!(!backpressure.wire_endpoint_pinned);
        assert!(!backpressure.wire_sends_model_endpoint);
        assert_eq!(backpressure.wire_model_endpoint_label, None);

        let repair = input.readiness_with_gate(
            &session,
            &FrontendGateSnapshot {
                safe_device_ok: false,
                ..FrontendGateSnapshot::default()
            },
        );

        assert_eq!(repair.block_state, Some(StreamState::Failed));
        assert_eq!(repair.block_state_label.as_deref(), Some("failed"));
        assert!(repair.preserves_buffer_on_enter);
        assert!(!repair.clears_buffer_on_enter);
        assert_eq!(repair.advice_action, Some(GateAdviceAction::RepairGate));
        assert_eq!(repair.advice_action_label.as_deref(), Some("repair_gate"));

        let hygiene = input.readiness_with_gate_and_start(
            &session,
            &FrontendGateSnapshot {
                experience_hygiene_ok: false,
                ..FrontendGateSnapshot::default()
            },
        );

        assert_eq!(hygiene.enter_action, InputActionKind::Blocked);
        assert!(hygiene.enter_is_blocked);
        assert_eq!(hygiene.block_state, Some(StreamState::Failed));
        assert_eq!(hygiene.block_state_label.as_deref(), Some("failed"));
        assert_eq!(
            hygiene.block_reason.as_deref(),
            Some("experience hygiene gate failed")
        );
        assert!(hygiene.preserves_buffer_on_enter);
        assert!(!hygiene.clears_buffer_on_enter);
        assert_eq!(hygiene.advice_action, Some(GateAdviceAction::RepairGate));
        assert_eq!(hygiene.advice_action_label.as_deref(), Some("repair_gate"));
        assert_eq!(hygiene.primary_action_label, "repair_gate");
        assert!(!hygiene.primary_action_enabled);
        assert_eq!(
            hygiene.primary_action_disabled_reason.as_deref(),
            Some("experience hygiene gate failed")
        );
        assert!(!hygiene.records_user_on_enter);
        assert!(!hygiene.starts_stream_on_enter);
        assert_eq!(session.history().len(), 0);
        assert_eq!(session.chunks().len(), 0);
    }
}
