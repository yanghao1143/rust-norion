use norion_service::{
    ChatChunkDisplaySnapshot, ChatSession, GateAdvice, GateDecision, ModelEndpointSelectionKind,
    ModelPoolGateSnapshot, ModelPoolRouteStatus, ModelPoolStatus, ModelRouteWorkerSnapshot,
    ModelWorkerSnapshot, RoutingIntent, SmartSteamCleanRoomHandoffStatusSnapshot,
    SmartSteamContextHygieneStatusSnapshot, SmartSteamDaemonRoundTransitionStatusSnapshot,
    SmartSteamHelperStageRepairStatusSnapshot, SmartSteamMemoryStartupAdmissionStatusSnapshot,
    SmartSteamNextRoundDecisionStatusSnapshot, SmartSteamSelfImproveProposalStatusSnapshot,
    SmartSteamStatusSnapshot, SmartSteamWorkerWindowStatusSnapshot, StreamState,
};

use crate::input::CliInputConfig;
use crate::output::gate_advice_status;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliStatusSnapshot {
    pub route: String,
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
    pub state: StreamState,
    pub state_label: String,
    pub state_is_terminal: bool,
    pub state_is_pressure: bool,
    pub state_blocks_prompt_submit: bool,
    pub history_messages: usize,
    pub history_limit: usize,
    pub max_tokens: Option<usize>,
    pub max_tokens_label: String,
    pub partial_chars: usize,
    pub last_error: Option<String>,
    pub gate_advice: Option<String>,
    pub gate_advice_detail: Option<GateAdvice>,
    pub gate_advice_action_label: Option<String>,
    pub gate_advice_state_label: Option<String>,
    pub gate_advice_reason: Option<String>,
    pub send_allowed: bool,
    pub send_block_state: Option<StreamState>,
    pub send_block_state_label: Option<String>,
    pub send_block_state_is_terminal: bool,
    pub send_block_state_is_pressure: bool,
    pub send_block_state_blocks_prompt_submit: bool,
    pub send_block_chunk: Option<ChatChunkDisplaySnapshot>,
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
    pub pool: Option<ModelPoolStatus>,
    pub route_pool: Option<ModelPoolRouteStatus>,
    pub workers: Option<Vec<ModelWorkerSnapshot>>,
    pub route_workers: Option<Vec<ModelRouteWorkerSnapshot>>,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliWorkersHostSnapshot {
    pub read_only: bool,
    pub launches_process: bool,
    pub sends_prompt: bool,
    pub starts_stream: bool,
    pub carries_request_preview: bool,
    pub carries_stream_chunk: bool,
    pub carries_input_action_snapshot: bool,
    pub route: String,
    pub model_role_label: String,
    pub routing_preference_label: String,
    pub endpoint_label: String,
    pub endpoint_pinned: bool,
    pub endpoint_kind_label: String,
    pub wire_model_role_label: String,
    pub wire_routing_preference_label: String,
    pub wire_prefer_fast: bool,
    pub wire_prefer_quality: bool,
    pub wire_endpoint_pinned: bool,
    pub wire_endpoint_kind_label: String,
    pub wire_sends_model_endpoint: bool,
    pub wire_model_endpoint_label: Option<String>,
    pub send_allowed: bool,
    pub send_block_state_label: Option<String>,
    pub send_block_reason: Option<String>,
    pub route_send_allowed: Option<bool>,
    pub route_send_block_state_label: Option<String>,
    pub route_send_block_reason: Option<String>,
    pub gate_advice_action_label: Option<String>,
    pub route_gate_advice_action_label: Option<String>,
    pub pool_status: Option<String>,
    pub route_pool_status: Option<String>,
    pub history_messages: usize,
    pub partial_chars: usize,
    pub workers: Vec<CliWorkerHostSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliSmartSteamStatusHostSnapshot {
    pub read_only: bool,
    pub launches_process: bool,
    pub starts_daemon: bool,
    pub stops_daemon: bool,
    pub touches_remote: bool,
    pub downloads_model: bool,
    pub warms_model_cache: bool,
    pub sends_prompt: bool,
    pub starts_stream: bool,
    pub replays_prompt: bool,
    pub mutates_busy: bool,
    pub mutates_readiness: bool,
    pub mutates_active_round: bool,
    pub starts_clean_room_replacement: bool,
    pub mutates_worker_window_status: bool,
    pub carries_request_preview: bool,
    pub carries_stream_chunk: bool,
    pub carries_input_action_snapshot: bool,
    pub history_messages: usize,
    pub partial_chars: usize,
    pub clean_room_replacement_required: bool,
    pub worker_window_status: String,
    pub worker_windows: Vec<SmartSteamWorkerWindowStatusSnapshot>,
    pub context_hygiene_status: SmartSteamContextHygieneStatusSnapshot,
    pub context_hygiene_summary: String,
    pub memory_startup_admission_status: Option<SmartSteamMemoryStartupAdmissionStatusSnapshot>,
    pub memory_startup_admission_summary: Option<String>,
    pub clean_room_handoff_status: Option<SmartSteamCleanRoomHandoffStatusSnapshot>,
    pub clean_room_handoff_summary: Option<String>,
    pub helper_stage_repair_status: Option<SmartSteamHelperStageRepairStatusSnapshot>,
    pub helper_stage_repair_summary: Option<String>,
    pub self_improve_proposal_status: Option<SmartSteamSelfImproveProposalStatusSnapshot>,
    pub self_improve_proposal_summary: Option<String>,
    pub daemon_round_transition_status: Option<SmartSteamDaemonRoundTransitionStatusSnapshot>,
    pub daemon_round_transition_summary: Option<String>,
    pub next_round_decision_status: Option<SmartSteamNextRoundDecisionStatusSnapshot>,
    pub next_round_decision_summary: Option<String>,
    pub service_status: SmartSteamStatusSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliWorkerHostSnapshot {
    pub endpoint_label: String,
    pub role_labels: Vec<String>,
    pub preference_labels: Vec<String>,
    pub worker_status_label: String,
    pub worker_status_state_label: String,
    pub worker_status_is_available: bool,
    pub worker_status_is_pressure: bool,
    pub worker_status_blocks_prompt_submit: bool,
    pub worker_status_display_snapshot: Option<ChatChunkDisplaySnapshot>,
    pub endpoint_selected: bool,
    pub route_match: bool,
    pub selectable: bool,
    pub picker_action_label: String,
    pub decision_action_label: String,
    pub decision_state_label: String,
    pub decision_reason: String,
    pub decision_display_snapshot: Option<ChatChunkDisplaySnapshot>,
    pub selection_summary: String,
    pub selection_model_role_label: String,
    pub selection_routing_preference_label: String,
    pub selection_endpoint_label: String,
    pub selection_endpoint_kind_label: String,
    pub selection_wire_model_role_label: String,
    pub selection_wire_routing_preference_label: String,
    pub selection_wire_prefer_fast: bool,
    pub selection_wire_prefer_quality: bool,
    pub selection_wire_endpoint_pinned: bool,
    pub selection_wire_endpoint_kind_label: String,
    pub selection_wire_sends_model_endpoint: bool,
    pub selection_wire_model_endpoint_label: Option<String>,
}

impl CliStatusSnapshot {
    pub fn new(input: &CliInputConfig, session: &ChatSession, gate: Option<&GateDecision>) -> Self {
        let display_decision = status_display_decision(session, gate);
        let gate_advice_detail = display_decision.as_ref().map(GateDecision::advice);
        let gate_advice_action_label = gate_advice_detail
            .as_ref()
            .map(|advice| advice.action.as_str().to_owned());
        let gate_advice_state_label = gate_advice_detail
            .as_ref()
            .map(|advice| advice.state.as_str().to_owned());
        let gate_advice_reason = gate_advice_detail
            .as_ref()
            .map(|advice| advice.reason.clone());
        let gate_advice = display_decision.as_ref().map(gate_advice_status);
        let send_block_state = display_decision.as_ref().and_then(decision_block_state);
        let send_block_state_label = send_block_state.map(|state| state.as_str().to_owned());
        let send_block_state_is_terminal = send_block_state.is_some_and(StreamState::is_terminal);
        let send_block_state_is_pressure = send_block_state.is_some_and(StreamState::is_pressure);
        let send_block_state_blocks_prompt_submit =
            send_block_state.is_some_and(StreamState::blocks_prompt_submit);
        let send_block_reason = send_block_state.and_then(|_| gate_advice_reason.clone());
        let send_block_chunk = display_decision
            .as_ref()
            .and_then(|decision| decision.display_snapshot(0));
        let routing_intent = input.routing_intent();
        let endpoint_kind = routing_intent.endpoint_kind();
        let wire = routing_intent.wire_snapshot();
        let state = session.state();
        let max_tokens = session.config().default_max_tokens;
        Self {
            route: routing_intent.summary(),
            model_role_label: routing_intent.model_role_label().to_owned(),
            routing_preference_label: routing_intent.routing_preference_label().to_owned(),
            endpoint_label: routing_intent.endpoint_label().to_owned(),
            endpoint_pinned: routing_intent.endpoint_pinned,
            endpoint_kind,
            endpoint_kind_label: routing_intent.endpoint_kind_label().to_owned(),
            endpoint_auto: routing_intent.endpoint_auto(),
            endpoint_built_in: routing_intent.endpoint_built_in(),
            endpoint_custom: routing_intent.endpoint_custom(),
            wire_model_role_label: wire.model_role_label,
            wire_routing_preference_label: wire.routing_preference_label,
            wire_prefer_fast: wire.prefer_fast,
            wire_prefer_quality: wire.prefer_quality,
            wire_endpoint_pinned: wire.endpoint_pinned,
            wire_endpoint_kind_label: wire.endpoint_kind_label,
            wire_sends_model_endpoint: wire.sends_model_endpoint,
            wire_model_endpoint_label: wire.model_endpoint_label,
            routing_intent,
            state,
            state_label: state.as_str().to_owned(),
            state_is_terminal: state.is_terminal(),
            state_is_pressure: state.is_pressure(),
            state_blocks_prompt_submit: state.blocks_prompt_submit(),
            history_messages: session.history().len(),
            history_limit: session.config().history_limit,
            max_tokens,
            max_tokens_label: max_tokens
                .map(|value| value.to_string())
                .unwrap_or_else(|| "backend-default".to_owned()),
            partial_chars: session.partial_answer().chars().count(),
            last_error: session.last_error().map(str::to_owned),
            gate_advice,
            gate_advice_detail,
            gate_advice_action_label,
            gate_advice_state_label,
            gate_advice_reason,
            send_allowed: send_block_state.is_none(),
            send_block_state,
            send_block_state_label,
            send_block_state_is_terminal,
            send_block_state_is_pressure,
            send_block_state_blocks_prompt_submit,
            send_block_chunk,
            send_block_reason,
            route_gate_advice: None,
            route_gate_advice_detail: None,
            route_gate_advice_action_label: None,
            route_gate_advice_state_label: None,
            route_gate_advice_reason: None,
            route_send_allowed: None,
            route_send_block_state: None,
            route_send_block_state_label: None,
            route_send_block_state_is_terminal: None,
            route_send_block_state_is_pressure: None,
            route_send_block_state_blocks_prompt_submit: None,
            route_send_block_chunk: None,
            route_send_block_reason: None,
            pool: None,
            route_pool: None,
            workers: None,
            route_workers: None,
            pool_status: None,
            pool_queue_label: None,
            pool_capacity_state: None,
            pool_capacity_state_label: None,
            pool_capacity_state_is_pressure: None,
            pool_capacity_state_blocks_prompt_submit: None,
            pool_has_workers: None,
            pool_has_available_workers: None,
            pool_has_busy_workers: None,
            pool_has_saturated_workers: None,
            pool_has_queued_requests: None,
            pool_queue_is_saturated: None,
            route_pool_status: None,
            route_pool_queue_label: None,
            route_pool_capacity_state: None,
            route_pool_capacity_state_label: None,
            route_pool_capacity_state_is_pressure: None,
            route_pool_capacity_state_blocks_prompt_submit: None,
            route_pool_has_matching_workers: None,
            route_pool_has_matching_available_workers: None,
            route_pool_has_matching_busy_workers: None,
            route_pool_has_matching_saturated_workers: None,
            route_pool_has_matching_queued_requests: None,
            route_pool_queue_is_saturated: None,
        }
    }

    pub fn from_model_pool_gate(
        input: &CliInputConfig,
        session: &ChatSession,
        gate: &ModelPoolGateSnapshot,
    ) -> Self {
        let route = gate.route_snapshot(&input.routing_intent());
        let decision = session_display_decision(session, route.decision.clone());
        let mut status = Self::new(input, session, Some(&decision));
        status.route = route.route.clone();
        status.routing_intent = route.intent.clone();
        status.model_role_label = route.model_role_label.clone();
        status.routing_preference_label = route.routing_preference_label.clone();
        status.endpoint_label = route.endpoint_label.clone();
        status.endpoint_pinned = route.endpoint_pinned;
        status.endpoint_kind = route.endpoint_kind;
        status.endpoint_kind_label = route.endpoint_kind_label.clone();
        status.endpoint_auto = route.endpoint_auto;
        status.endpoint_built_in = route.endpoint_built_in;
        status.endpoint_custom = route.endpoint_custom;
        status.wire_model_role_label = route.wire_model_role_label.clone();
        status.wire_routing_preference_label = route.wire_routing_preference_label.clone();
        status.wire_prefer_fast = route.wire_prefer_fast;
        status.wire_prefer_quality = route.wire_prefer_quality;
        status.wire_endpoint_pinned = route.wire_endpoint_pinned;
        status.wire_endpoint_kind_label = route.wire_endpoint_kind_label.clone();
        status.wire_sends_model_endpoint = route.wire_sends_model_endpoint;
        status.wire_model_endpoint_label = route.wire_model_endpoint_label.clone();
        let route_send_block_reason = route
            .send_block_state
            .and(Some(route.decision_reason.clone()));
        status.route_gate_advice = Some(route.decision_advice.status_line());
        status.route_gate_advice_action_label = Some(route.decision_action_label);
        status.route_gate_advice_state_label = Some(route.decision_state_label);
        status.route_gate_advice_reason = Some(route.decision_reason);
        status.route_gate_advice_detail = Some(route.decision_advice);
        status.route_send_allowed = Some(route.send_allowed);
        status.route_send_block_state = route.send_block_state;
        status.route_send_block_state_label = route.send_block_state_label;
        status.route_send_block_state_is_terminal =
            Some(route.send_block_state.is_some_and(StreamState::is_terminal));
        status.route_send_block_state_is_pressure =
            Some(route.send_block_state.is_some_and(StreamState::is_pressure));
        status.route_send_block_state_blocks_prompt_submit = Some(
            route
                .send_block_state
                .is_some_and(StreamState::blocks_prompt_submit),
        );
        status.route_send_block_chunk = route.send_block_chunk;
        status.route_send_block_reason = route_send_block_reason;
        status.pool_queue_label = Some(route.pool_queue_label);
        status.pool_status = Some(route.pool_status);
        status.pool_capacity_state = Some(route.pool_capacity_state);
        status.pool_capacity_state_label = Some(route.pool_capacity_state_label);
        status.pool_capacity_state_is_pressure = Some(route.pool_capacity_state_is_pressure);
        status.pool_capacity_state_blocks_prompt_submit =
            Some(route.pool_capacity_state_blocks_prompt_submit);
        status.pool_has_workers = Some(route.pool.has_workers());
        status.pool_has_available_workers = Some(route.pool.has_available_workers());
        status.pool_has_busy_workers = Some(route.pool.has_busy_workers());
        status.pool_has_saturated_workers = Some(route.pool.has_saturated_workers());
        status.pool_has_queued_requests = Some(route.pool.has_queued_requests());
        status.pool_queue_is_saturated = Some(route.pool.queue_is_saturated());
        status.pool = Some(route.pool);
        status.workers = Some(gate.workers.clone());
        status.route_workers = Some(route.workers);
        if gate.has_capability_declarations() {
            status.route_pool_queue_label = Some(route.route_pool_queue_label);
            status.route_pool_status = Some(route.route_pool_status);
            status.route_pool_capacity_state = Some(route.route_pool_capacity_state);
            status.route_pool_capacity_state_label = Some(route.route_pool_capacity_state_label);
            status.route_pool_capacity_state_is_pressure =
                Some(route.route_pool_capacity_state_is_pressure);
            status.route_pool_capacity_state_blocks_prompt_submit =
                Some(route.route_pool_capacity_state_blocks_prompt_submit);
            status.route_pool_has_matching_workers = Some(route.route_pool.has_matching_workers());
            status.route_pool_has_matching_available_workers =
                Some(route.route_pool.has_matching_available_workers());
            status.route_pool_has_matching_busy_workers =
                Some(route.route_pool.has_matching_busy_workers());
            status.route_pool_has_matching_saturated_workers =
                Some(route.route_pool.has_matching_saturated_workers());
            status.route_pool_has_matching_queued_requests =
                Some(route.route_pool.has_matching_queued_requests());
            status.route_pool_queue_is_saturated =
                Some(route.route_pool.matching_queue_is_saturated());
            status.route_pool = Some(route.route_pool);
        }
        status
    }

    pub fn line(&self) -> String {
        let mut parts = vec![
            self.route.clone(),
            format!("state={}", self.state.as_str()),
            format!("history={}", self.history_messages),
            format!("max_tokens={}", self.max_tokens_label),
            format!("partial_chars={}", self.partial_chars),
        ];
        if let Some(last_error) = self.last_error.as_ref() {
            parts.push(format!("last_error={last_error}"));
        }
        if let Some(advice) = self.gate_advice.as_ref() {
            parts.push(format!("advice={advice}"));
        }
        if let Some(pool_status) = self.pool_status.as_ref() {
            parts.push(format!("pool={pool_status}"));
        }
        if let Some(route_pool_status) = self.route_pool_status.as_ref() {
            parts.push(format!("route_pool={route_pool_status}"));
        }
        parts.join(" ")
    }

    pub fn workers_line(&self) -> Option<String> {
        let workers = self.workers.as_ref()?;
        let advice = self.gate_advice.as_ref()?;
        let pool_status = self.pool_status.as_ref()?;
        let route_pool = self
            .route_pool_status
            .as_ref()
            .map(|status| format!(" route_pool={status}"))
            .unwrap_or_default();
        let worker_lines = if workers.is_empty() {
            "workers none registered".to_owned()
        } else {
            workers
                .iter()
                .map(ModelWorkerSnapshot::summary)
                .collect::<Vec<_>>()
                .join(" | ")
        };
        Some(format!(
            "{} advice={} pool={}{} workers=[{}]",
            self.route, advice, pool_status, route_pool, worker_lines
        ))
    }

    pub fn workers_host_snapshot(&self) -> Option<CliWorkersHostSnapshot> {
        let route_workers = self.route_workers.as_ref()?;
        Some(CliWorkersHostSnapshot {
            read_only: true,
            launches_process: false,
            sends_prompt: false,
            starts_stream: false,
            carries_request_preview: false,
            carries_stream_chunk: false,
            carries_input_action_snapshot: false,
            route: self.route.clone(),
            model_role_label: self.model_role_label.clone(),
            routing_preference_label: self.routing_preference_label.clone(),
            endpoint_label: self.endpoint_label.clone(),
            endpoint_pinned: self.endpoint_pinned,
            endpoint_kind_label: self.endpoint_kind_label.clone(),
            wire_model_role_label: self.wire_model_role_label.clone(),
            wire_routing_preference_label: self.wire_routing_preference_label.clone(),
            wire_prefer_fast: self.wire_prefer_fast,
            wire_prefer_quality: self.wire_prefer_quality,
            wire_endpoint_pinned: self.wire_endpoint_pinned,
            wire_endpoint_kind_label: self.wire_endpoint_kind_label.clone(),
            wire_sends_model_endpoint: self.wire_sends_model_endpoint,
            wire_model_endpoint_label: self.wire_model_endpoint_label.clone(),
            send_allowed: self.send_allowed,
            send_block_state_label: self.send_block_state_label.clone(),
            send_block_reason: self.send_block_reason.clone(),
            route_send_allowed: self.route_send_allowed,
            route_send_block_state_label: self.route_send_block_state_label.clone(),
            route_send_block_reason: self.route_send_block_reason.clone(),
            gate_advice_action_label: self.gate_advice_action_label.clone(),
            route_gate_advice_action_label: self.route_gate_advice_action_label.clone(),
            pool_status: self.pool_status.clone(),
            route_pool_status: self.route_pool_status.clone(),
            history_messages: self.history_messages,
            partial_chars: self.partial_chars,
            workers: route_workers
                .iter()
                .map(|worker| CliWorkerHostSnapshot {
                    endpoint_label: worker.endpoint_label().to_owned(),
                    role_labels: worker.worker.role_labels(),
                    preference_labels: worker.worker.preference_labels(),
                    worker_status_label: worker.worker_status_label().to_owned(),
                    worker_status_state_label: worker.worker_status_state_label().to_owned(),
                    worker_status_is_available: worker.worker_status_is_available(),
                    worker_status_is_pressure: worker.worker_status_is_pressure(),
                    worker_status_blocks_prompt_submit: worker.worker_status_blocks_prompt_submit(),
                    worker_status_display_snapshot: worker.worker_status_display_snapshot(),
                    endpoint_selected: worker.endpoint_selected,
                    route_match: worker.route_match,
                    selectable: worker.selectable,
                    picker_action_label: worker.picker_action_label.clone(),
                    decision_action_label: worker.decision_action_label().to_owned(),
                    decision_state_label: worker.decision_state_label().to_owned(),
                    decision_reason: worker.decision_reason(),
                    decision_display_snapshot: worker.decision_display_snapshot(),
                    selection_summary: worker.selection_summary.clone(),
                    selection_model_role_label: worker.selection_model_role_label.clone(),
                    selection_routing_preference_label: worker
                        .selection_routing_preference_label
                        .clone(),
                    selection_endpoint_label: worker.selection_endpoint_label.clone(),
                    selection_endpoint_kind_label: worker.selection_endpoint_kind_label.clone(),
                    selection_wire_model_role_label: worker.selection_wire_model_role_label.clone(),
                    selection_wire_routing_preference_label: worker
                        .selection_wire_routing_preference_label
                        .clone(),
                    selection_wire_prefer_fast: worker.selection_wire_prefer_fast,
                    selection_wire_prefer_quality: worker.selection_wire_prefer_quality,
                    selection_wire_endpoint_pinned: worker.selection_wire_endpoint_pinned,
                    selection_wire_endpoint_kind_label: worker
                        .selection_wire_endpoint_kind_label
                        .clone(),
                    selection_wire_sends_model_endpoint: worker.selection_wire_sends_model_endpoint,
                    selection_wire_model_endpoint_label: worker
                        .selection_wire_model_endpoint_label
                        .clone(),
                })
                .collect(),
        })
    }

    pub fn smartsteam_status_host_snapshot(
        &self,
        service_status: SmartSteamStatusSnapshot,
    ) -> CliSmartSteamStatusHostSnapshot {
        let worker_window_status = service_status.worker_window_status.clone();
        let worker_windows = service_status.worker_windows.clone();
        let context_hygiene_status = service_status.context_hygiene_status.clone();
        let context_hygiene_summary = service_status.context_hygiene_summary.clone();
        let memory_startup_admission_status =
            service_status.memory_startup_admission_status.clone();
        let memory_startup_admission_summary =
            service_status.memory_startup_admission_summary.clone();
        let clean_room_handoff_status = service_status.clean_room_handoff_status.clone();
        let clean_room_handoff_summary = service_status.clean_room_handoff_summary.clone();
        let helper_stage_repair_status = service_status.helper_stage_repair_status.clone();
        let helper_stage_repair_summary = service_status.helper_stage_repair_summary.clone();
        let self_improve_proposal_status = service_status.self_improve_proposal_status.clone();
        let self_improve_proposal_summary = service_status.self_improve_proposal_summary.clone();
        let daemon_round_transition_status = service_status.daemon_round_transition_status.clone();
        let daemon_round_transition_summary =
            service_status.daemon_round_transition_summary.clone();
        let next_round_decision_status = service_status.next_round_decision_status.clone();
        let next_round_decision_summary = service_status.next_round_decision_summary.clone();
        CliSmartSteamStatusHostSnapshot {
            read_only: service_status.read_only,
            launches_process: service_status.launches_process,
            starts_daemon: service_status.starts_daemon,
            stops_daemon: service_status.stops_daemon,
            touches_remote: service_status.touches_remote,
            downloads_model: service_status.downloads_model,
            warms_model_cache: service_status.warms_model_cache,
            sends_prompt: service_status.sends_prompt,
            starts_stream: service_status.starts_stream,
            replays_prompt: service_status.replays_prompt,
            mutates_busy: service_status.mutates_busy,
            mutates_readiness: service_status.mutates_readiness,
            mutates_active_round: service_status.mutates_active_round,
            starts_clean_room_replacement: service_status.starts_clean_room_replacement,
            mutates_worker_window_status: service_status.mutates_worker_window_status,
            carries_request_preview: false,
            carries_stream_chunk: false,
            carries_input_action_snapshot: false,
            history_messages: self.history_messages,
            partial_chars: self.partial_chars,
            clean_room_replacement_required: service_status.clean_room_replacement_required,
            worker_window_status,
            worker_windows,
            context_hygiene_status,
            context_hygiene_summary,
            memory_startup_admission_status,
            memory_startup_admission_summary,
            clean_room_handoff_status,
            clean_room_handoff_summary,
            helper_stage_repair_status,
            helper_stage_repair_summary,
            self_improve_proposal_status,
            self_improve_proposal_summary,
            daemon_round_transition_status,
            daemon_round_transition_summary,
            next_round_decision_status,
            next_round_decision_summary,
            service_status,
        }
    }
}

pub fn cli_status_line(
    input: &CliInputConfig,
    session: &ChatSession,
    gate: Option<&GateDecision>,
) -> String {
    CliStatusSnapshot::new(input, session, gate).line()
}

pub fn cli_model_pool_status_line(
    input: &CliInputConfig,
    session: &ChatSession,
    gate: &ModelPoolGateSnapshot,
) -> String {
    CliStatusSnapshot::from_model_pool_gate(input, session, gate).line()
}

pub fn cli_workers_unavailable_line(input: &CliInputConfig) -> String {
    format!(
        "{} workers=unavailable reason=model-pool-gate-not-attached",
        input.routing_summary()
    )
}

pub fn cli_model_pool_workers_line(
    input: &CliInputConfig,
    session: &ChatSession,
    gate: &ModelPoolGateSnapshot,
) -> String {
    CliStatusSnapshot::from_model_pool_gate(input, session, gate)
        .workers_line()
        .expect("model pool status snapshots always carry worker details")
}

fn session_display_decision(session: &ChatSession, decision: GateDecision) -> GateDecision {
    if matches!(
        decision,
        GateDecision::Blocked {
            state: StreamState::Failed,
            ..
        }
    ) {
        return decision;
    }

    session
        .prompt_blocked_chunk()
        .map(|chunk| GateDecision::blocked(chunk.state, chunk.content))
        .unwrap_or(decision)
}

fn status_display_decision(
    session: &ChatSession,
    gate: Option<&GateDecision>,
) -> Option<GateDecision> {
    gate.cloned()
        .map(|decision| session_display_decision(session, decision))
        .or_else(|| {
            session
                .prompt_blocked_chunk()
                .map(|chunk| GateDecision::blocked(chunk.state, chunk.content))
        })
}

fn decision_block_state(decision: &GateDecision) -> Option<StreamState> {
    match decision {
        GateDecision::Allowed => None,
        GateDecision::Blocked { state, .. } => Some(*state),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CliInputConfig;
    use norion_service::{
        ChatSessionConfig, FrontendGateSnapshot, GateAdviceAction, GateDecision, ModelEndpoint,
        ModelPoolGateSnapshot, ModelRole, ModelRouteWorkerPickerAction, ModelWorkerSnapshot,
        RoutingPreference,
    };

    #[test]
    fn status_line_summarizes_route_session_policy_and_partial() {
        let mut session = ChatSession::new(
            "cli",
            ChatSessionConfig::default().with_default_max_tokens(Some(8192)),
        );
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected stream");
        session.push_delta("partial");
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);

        let line = cli_status_line(&input, &session, None);

        assert_eq!(
            line,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=streaming history=1 max_tokens=8192 partial_chars=7 advice=wait_for_current_stream busy: session stream is already active"
        );
    }

    #[test]
    fn status_snapshot_exposes_structured_routing_intent_for_ui() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);

        let status = CliStatusSnapshot::new(&input, &session, None);

        assert_eq!(status.routing_intent.model_role, ModelRole::Reviewer);
        assert_eq!(status.model_role_label, "reviewer");
        assert_eq!(
            status.routing_intent.routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(status.routing_preference_label, "prefer_fast");
        assert_eq!(status.history_messages, 0);
        assert_eq!(status.history_limit, 64);
        assert_eq!(status.max_tokens_label, "backend-default");
        assert_eq!(status.state, StreamState::Pending);
        assert_eq!(status.state_label, "pending");
        assert!(!status.state_is_terminal);
        assert!(!status.state_is_pressure);
        assert!(!status.state_blocks_prompt_submit);
        assert_eq!(status.routing_intent.endpoint_label(), "auto");
        assert_eq!(status.endpoint_label, "auto");
        assert!(!status.routing_intent.endpoint_pinned);
        assert!(!status.endpoint_pinned);
        assert_eq!(status.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(status.endpoint_kind_label, "auto");
        assert!(status.endpoint_auto);
        assert!(!status.endpoint_built_in);
        assert!(!status.endpoint_custom);
        assert_eq!(status.wire_model_role_label, "reviewer");
        assert_eq!(status.wire_routing_preference_label, "prefer_fast");
        assert!(status.wire_prefer_fast);
        assert!(!status.wire_prefer_quality);
        assert!(!status.wire_endpoint_pinned);
        assert_eq!(status.wire_endpoint_kind_label, "auto");
        assert!(!status.wire_sends_model_endpoint);
        assert_eq!(status.wire_model_endpoint_label, None);
        assert_eq!(
            status.route,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );

        let pinned = CliStatusSnapshot::new(
            &input
                .clone()
                .with_model_endpoint(Some(ModelEndpoint::FastReviewer)),
            &session,
            None,
        );

        assert_eq!(pinned.routing_intent.endpoint_label(), "fast-reviewer");
        assert_eq!(pinned.model_role_label, "reviewer");
        assert_eq!(pinned.routing_preference_label, "prefer_fast");
        assert_eq!(pinned.endpoint_label, "fast-reviewer");
        assert!(pinned.routing_intent.endpoint_pinned);
        assert!(pinned.endpoint_pinned);
        assert_eq!(pinned.endpoint_kind, ModelEndpointSelectionKind::BuiltIn);
        assert_eq!(pinned.endpoint_kind_label, "built_in");
        assert!(!pinned.endpoint_auto);
        assert!(pinned.endpoint_built_in);
        assert!(!pinned.endpoint_custom);
        assert_eq!(pinned.wire_model_role_label, "reviewer");
        assert_eq!(pinned.wire_routing_preference_label, "prefer_fast");
        assert!(pinned.wire_prefer_fast);
        assert!(!pinned.wire_prefer_quality);
        assert!(pinned.wire_endpoint_pinned);
        assert_eq!(pinned.wire_endpoint_kind_label, "built_in");
        assert!(pinned.wire_sends_model_endpoint);
        assert_eq!(
            pinned.wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );
        assert_eq!(
            pinned.route,
            "role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true"
        );

        let custom = CliStatusSnapshot::new(
            &input.with_model_endpoint(Some(ModelEndpoint::Worker("mlx-reviewer-8b".to_owned()))),
            &session,
            None,
        );

        assert_eq!(custom.routing_intent.endpoint_label(), "mlx-reviewer-8b");
        assert_eq!(custom.endpoint_label, "mlx-reviewer-8b");
        assert!(custom.endpoint_pinned);
        assert_eq!(custom.endpoint_kind, ModelEndpointSelectionKind::Custom);
        assert_eq!(custom.endpoint_kind_label, "custom");
        assert!(!custom.endpoint_auto);
        assert!(!custom.endpoint_built_in);
        assert!(custom.endpoint_custom);
        assert!(custom.wire_endpoint_pinned);
        assert_eq!(custom.wire_endpoint_kind_label, "custom");
        assert!(custom.wire_sends_model_endpoint);
        assert_eq!(
            custom.wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-8b")
        );
    }

    #[test]
    fn status_snapshot_exposes_history_limit_without_changing_terminal_line() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::new(4));
        session.record_user("one");
        session.record_assistant("two");
        let input = CliInputConfig::default();

        let status = CliStatusSnapshot::new(&input, &session, None);

        assert_eq!(status.history_messages, 2);
        assert_eq!(status.history_limit, 4);
        assert_eq!(status.max_tokens_label, "backend-default");
        assert_eq!(status.pool_has_workers, None);
        assert_eq!(status.pool_has_available_workers, None);
        assert_eq!(status.pool_has_busy_workers, None);
        assert_eq!(status.pool_has_saturated_workers, None);
        assert_eq!(status.pool_has_queued_requests, None);
        assert_eq!(status.pool_queue_is_saturated, None);
        assert_eq!(status.pool_capacity_state, None);
        assert_eq!(status.pool_capacity_state_label, None);
        assert_eq!(status.pool_capacity_state_is_pressure, None);
        assert_eq!(status.pool_capacity_state_blocks_prompt_submit, None);
        assert_eq!(status.route_pool_has_matching_workers, None);
        assert_eq!(status.route_pool_has_matching_available_workers, None);
        assert_eq!(status.route_pool_has_matching_busy_workers, None);
        assert_eq!(status.route_pool_has_matching_saturated_workers, None);
        assert_eq!(status.route_pool_has_matching_queued_requests, None);
        assert_eq!(status.route_pool_queue_is_saturated, None);
        assert_eq!(status.route_pool_capacity_state, None);
        assert_eq!(status.route_pool_capacity_state_label, None);
        assert_eq!(status.route_pool_capacity_state_is_pressure, None);
        assert_eq!(status.route_pool_capacity_state_blocks_prompt_submit, None);
        assert_eq!(
            status.line(),
            "role=assistant preference=balanced endpoint=auto pinned=false state=pending history=2 max_tokens=backend-default partial_chars=0"
        );
    }

    #[test]
    fn status_line_includes_gate_advice_when_available() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input =
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let gate = GateDecision::blocked(StreamState::Busy, "worker fast-reviewer is busy");

        let status = CliStatusSnapshot::new(&input, &session, Some(&gate));

        assert_eq!(status.state, StreamState::Pending);
        assert_eq!(status.state_label, "pending");
        assert!(!status.state_is_terminal);
        assert!(!status.state_is_pressure);
        assert!(!status.state_blocks_prompt_submit);
        assert_eq!(status.history_messages, 0);
        assert_eq!(status.max_tokens, None);
        assert_eq!(status.last_error, None);
        assert_eq!(
            status.gate_advice.as_deref(),
            Some("wait_for_current_stream busy: worker fast-reviewer is busy")
        );
        let advice = status
            .gate_advice_detail
            .as_ref()
            .expect("structured advice should be available");
        assert_eq!(advice.action, GateAdviceAction::WaitForCurrentStream);
        assert_eq!(advice.state, StreamState::Busy);
        assert_eq!(advice.reason.as_str(), "worker fast-reviewer is busy");
        assert_eq!(
            status.gate_advice_action_label.as_deref(),
            Some("wait_for_current_stream")
        );
        assert_eq!(status.gate_advice_state_label.as_deref(), Some("busy"));
        assert_eq!(
            status.gate_advice_reason.as_deref(),
            Some("worker fast-reviewer is busy")
        );
        assert_eq!(status.send_block_state, Some(StreamState::Busy));
        assert_eq!(status.send_block_state_label.as_deref(), Some("busy"));
        assert!(!status.send_block_state_is_terminal);
        assert!(status.send_block_state_is_pressure);
        assert!(status.send_block_state_blocks_prompt_submit);
        let send_block_chunk = status
            .send_block_chunk
            .as_ref()
            .expect("status pressure should expose a display chunk");
        assert_eq!(send_block_chunk.output_label, "busy");
        assert_eq!(
            send_block_chunk.appended,
            "[busy] worker fast-reviewer is busy"
        );
        assert!(send_block_chunk.state_blocks_prompt_submit);
        assert_eq!(
            status.send_block_reason.as_deref(),
            Some("worker fast-reviewer is busy")
        );
        assert_eq!(
            status.line(),
            "role=assistant preference=balanced endpoint=fast-reviewer pinned=true state=pending history=0 max_tokens=backend-default partial_chars=0 advice=wait_for_current_stream busy: worker fast-reviewer is busy"
        );
    }

    #[test]
    fn status_line_prefers_active_session_over_allowed_frontend_gate() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        session.push_delta("partial");
        let input = CliInputConfig::default();

        let status = CliStatusSnapshot::new(&input, &session, Some(&GateDecision::Allowed));

        assert_eq!(
            status.gate_advice.as_deref(),
            Some("wait_for_current_stream busy: session stream is already active")
        );
        assert_eq!(status.state, StreamState::Streaming);
        assert!(!status.state_is_terminal);
        assert!(!status.state_is_pressure);
        assert!(status.state_blocks_prompt_submit);
        assert_eq!(
            status.line(),
            "role=assistant preference=balanced endpoint=auto pinned=false state=streaming history=1 max_tokens=backend-default partial_chars=7 advice=wait_for_current_stream busy: session stream is already active"
        );
    }

    #[test]
    fn status_line_preserves_session_pressure_reason() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session.backpressure("pool queue full");
        let input = CliInputConfig::default();

        let status = CliStatusSnapshot::new(&input, &session, None);

        assert_eq!(status.state, StreamState::Backpressure);
        assert_eq!(status.state_label, "backpressure");
        assert!(!status.state_is_terminal);
        assert!(status.state_is_pressure);
        assert!(status.state_blocks_prompt_submit);
        assert_eq!(
            status.gate_advice.as_deref(),
            Some("retry_later backpressure: pool queue full")
        );
        assert_eq!(
            status.gate_advice_action_label.as_deref(),
            Some("retry_later")
        );
        assert_eq!(
            status.send_block_state_label.as_deref(),
            Some("backpressure")
        );
        let send_block_chunk = status
            .send_block_chunk
            .as_ref()
            .expect("session pressure should expose a display chunk");
        assert_eq!(send_block_chunk.output_label, "backpressure");
        assert_eq!(send_block_chunk.appended, "[backpressure] pool queue full");
        assert_eq!(status.send_block_reason.as_deref(), Some("pool queue full"));
        assert_eq!(
            status.line(),
            "role=assistant preference=balanced endpoint=auto pinned=false state=backpressure history=0 max_tokens=backend-default partial_chars=0 advice=retry_later backpressure: pool queue full"
        );
    }

    #[test]
    fn status_line_keeps_blocked_governance_gate_over_active_session() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        let input = CliInputConfig::default();
        let gate = GateDecision::blocked(StreamState::Failed, "safe-device gate failed");

        let status = CliStatusSnapshot::new(&input, &session, Some(&gate));

        assert_eq!(
            status.gate_advice.as_deref(),
            Some("repair_gate failed: safe-device gate failed")
        );
        let advice = status
            .gate_advice_detail
            .as_ref()
            .expect("structured repair advice should be available");
        assert_eq!(advice.action, GateAdviceAction::RepairGate);
        assert_eq!(advice.state, StreamState::Failed);
        assert_eq!(advice.reason.as_str(), "safe-device gate failed");
        assert_eq!(
            status.gate_advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(status.send_block_state_label.as_deref(), Some("failed"));
        assert_eq!(
            status.send_block_reason.as_deref(),
            Some("safe-device gate failed")
        );
        assert_eq!(
            status.line(),
            "role=assistant preference=balanced endpoint=auto pinned=false state=streaming history=1 max_tokens=backend-default partial_chars=0 advice=repair_gate failed: safe-device gate failed"
        );
    }

    #[test]
    fn status_line_includes_last_error_after_interrupted_partial_stream() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected stream");
        session.push_delta("partial");
        session.interrupt("backend stream closed");
        let input = CliInputConfig::default();

        let status = CliStatusSnapshot::new(&input, &session, None);

        assert_eq!(status.state, StreamState::Interrupted);
        assert_eq!(status.partial_chars, 7);
        assert_eq!(status.last_error.as_deref(), Some("backend stream closed"));
        assert_eq!(
            status.line(),
            "role=assistant preference=balanced endpoint=auto pinned=false state=interrupted history=1 max_tokens=backend-default partial_chars=7 last_error=backend stream closed"
        );
    }

    #[test]
    fn status_snapshot_marks_interrupted_partial_as_terminal_but_sendable() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected stream");
        session.push_delta("partial");
        session.cancel_stream().expect("expected cancel chunk");
        let input = CliInputConfig::default();

        let status = CliStatusSnapshot::new(&input, &session, None);

        assert_eq!(status.state, StreamState::Interrupted);
        assert_eq!(status.state_label, "interrupted");
        assert!(status.state_is_terminal);
        assert!(!status.state_is_pressure);
        assert!(!status.state_blocks_prompt_submit);
        assert_eq!(status.partial_chars, 7);
        assert_eq!(
            status.last_error.as_deref(),
            Some("stream cancelled by user")
        );
        assert!(status.send_allowed);
        assert_eq!(status.send_block_state, None);
        assert_eq!(status.send_block_state_label, None);
        assert_eq!(status.send_block_reason, None);
        assert!(!status.send_block_state_is_terminal);
        assert!(!status.send_block_state_is_pressure);
        assert!(!status.send_block_state_blocks_prompt_submit);
        assert_eq!(status.send_block_chunk, None);
        assert_eq!(
            status.line(),
            "role=assistant preference=balanced endpoint=auto pinned=false state=interrupted history=1 max_tokens=backend-default partial_chars=7 last_error=stream cancelled by user"
        );
    }

    #[test]
    fn model_pool_status_keeps_cancelled_session_sendable_until_external_gate_blocks() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected stream");
        session.push_delta("partial");
        session.cancel_stream().expect("expected cancel chunk");
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let ready_gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );

        let ready = CliStatusSnapshot::from_model_pool_gate(&input, &session, &ready_gate);

        assert_eq!(ready.state, StreamState::Interrupted);
        assert_eq!(ready.state_label, "interrupted");
        assert!(ready.state_is_terminal);
        assert!(!ready.state_blocks_prompt_submit);
        assert_eq!(ready.partial_chars, 7);
        assert_eq!(
            ready.last_error.as_deref(),
            Some("stream cancelled by user")
        );
        assert_eq!(
            ready.gate_advice.as_deref(),
            Some("send_now pending: ready to send")
        );
        assert!(ready.send_allowed);
        assert_eq!(ready.send_block_state, None);
        assert_eq!(ready.route_send_allowed, Some(true));
        assert_eq!(ready.route_send_block_state, None);
        assert_eq!(
            ready.line(),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=interrupted history=1 max_tokens=backend-default partial_chars=7 last_error=stream cancelled by user advice=send_now pending: ready to send pool=workers total=1 available=1 busy=0 saturated=0 route_pool=matching total=1 available=1 busy=0 saturated=0"
        );

        let repair_gate = ModelPoolGateSnapshot::new(
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

        let repair = CliStatusSnapshot::from_model_pool_gate(&input, &session, &repair_gate);

        assert_eq!(repair.state, StreamState::Interrupted);
        assert!(repair.state_is_terminal);
        assert!(!repair.state_blocks_prompt_submit);
        assert_eq!(
            repair.last_error.as_deref(),
            Some("stream cancelled by user")
        );
        assert_eq!(
            repair.gate_advice.as_deref(),
            Some("repair_gate failed: safe-device gate failed")
        );
        assert!(!repair.send_allowed);
        assert_eq!(repair.send_block_state, Some(StreamState::Failed));
        assert_eq!(repair.send_block_state_label.as_deref(), Some("failed"));
        assert!(!repair.send_block_state_is_pressure);
        assert!(!repair.send_block_state_blocks_prompt_submit);
        assert_eq!(repair.route_send_allowed, Some(false));
        assert_eq!(repair.route_send_block_state, Some(StreamState::Failed));
        assert_eq!(
            repair.route_send_block_reason.as_deref(),
            Some("safe-device gate failed")
        );
    }

    #[test]
    fn status_line_clears_last_error_after_recovered_stream() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected stream");
        session.push_delta("partial");
        session.interrupt("backend stream closed");
        let input = CliInputConfig::default();

        let interrupted = cli_status_line(&input, &session, None);
        assert!(interrupted.contains("state=interrupted"));
        assert!(interrupted.contains("partial_chars=7"));
        assert!(interrupted.contains("last_error=backend stream closed"));

        session
            .try_submit_and_begin_stream("next")
            .expect("interrupted stream should recover");
        let recovered = cli_status_line(&input, &session, None);

        assert_eq!(
            recovered,
            "role=assistant preference=balanced endpoint=auto pinned=false state=streaming history=2 max_tokens=backend-default partial_chars=0 advice=wait_for_current_stream busy: session stream is already active"
        );
        assert!(!recovered.contains("last_error="));
    }

    #[test]
    fn status_line_includes_last_error_after_failed_empty_stream() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session.begin_stream();
        session.fail("safe-device gate failed");
        let input = CliInputConfig::default();

        assert_eq!(
            cli_status_line(&input, &session, None),
            "role=assistant preference=balanced endpoint=auto pinned=false state=failed history=0 max_tokens=backend-default partial_chars=0 last_error=safe-device gate failed"
        );
    }

    #[test]
    fn model_pool_status_line_includes_pool_capacity_and_route_decision() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("quality".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer),
                ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester).with_queue(2, 2),
            ],
        );

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);

        assert_eq!(status.wire_model_role_label, "reviewer");
        assert_eq!(status.wire_routing_preference_label, "prefer_fast");
        assert!(status.wire_prefer_fast);
        assert!(!status.wire_prefer_quality);
        assert!(!status.wire_endpoint_pinned);
        assert_eq!(status.wire_endpoint_kind_label, "auto");
        assert!(!status.wire_sends_model_endpoint);
        assert_eq!(status.wire_model_endpoint_label, None);
        assert_eq!(
            status.pool_status.as_deref(),
            Some("workers total=3 available=1 busy=1 saturated=1")
        );
        assert_eq!(status.pool_queue_label.as_deref(), Some("2/4"));
        assert_eq!(status.pool_has_workers, Some(true));
        assert_eq!(status.pool_has_available_workers, Some(true));
        assert_eq!(status.pool_has_busy_workers, Some(true));
        assert_eq!(status.pool_has_saturated_workers, Some(true));
        assert_eq!(status.pool_has_queued_requests, Some(true));
        assert_eq!(status.pool_queue_is_saturated, Some(false));
        assert_eq!(status.pool_capacity_state, Some(StreamState::Queued));
        assert_eq!(status.pool_capacity_state_label.as_deref(), Some("queued"));
        assert_eq!(status.pool_capacity_state_is_pressure, Some(true));
        assert_eq!(status.pool_capacity_state_blocks_prompt_submit, Some(true));
        let pool = status
            .pool
            .as_ref()
            .expect("pool status should be structured");
        assert_eq!(pool.total_workers, 3);
        assert_eq!(pool.available_workers, 1);
        assert_eq!(pool.busy_workers, 1);
        assert_eq!(pool.saturated_workers, 1);
        assert_eq!(pool.queued_requests, 2);
        assert_eq!(pool.queue_limit, 4);
        let workers = status
            .workers
            .as_ref()
            .expect("workers should be structured");
        let route_workers = status
            .route_workers
            .as_ref()
            .expect("route workers should be structured");
        assert_eq!(workers.len(), 3);
        assert_eq!(workers[0].endpoint.label(), "quality-12b");
        assert_eq!(workers[0].endpoint_label(), "quality-12b");
        assert_eq!(workers[0].status_label(), "busy");
        assert_eq!(workers[0].queue_label(), "0/1");
        assert_eq!(workers[0].active_request_label(), "quality");
        assert_eq!(workers[0].active_request.as_deref(), Some("quality"));
        assert_eq!(workers[1].endpoint.label(), "fast-reviewer");
        assert_eq!(workers[1].status_label(), "available");
        assert_eq!(workers[1].active_request_label(), "none");
        assert_eq!(workers[2].endpoint.label(), "summary-tester");
        assert_eq!(workers[2].status_label(), "backpressure");
        assert_eq!(route_workers.len(), 3);
        assert!(route_workers[0].route_match);
        assert!(!route_workers[0].selectable);
        assert_eq!(
            route_workers[0].picker_action,
            ModelRouteWorkerPickerAction::Wait
        );
        assert_eq!(route_workers[0].picker_action_label, "wait");
        assert_eq!(route_workers[0].endpoint_label(), "quality-12b");
        assert_eq!(route_workers[0].worker_status_label(), "busy");
        assert_eq!(
            route_workers[0].decision_action_label(),
            "wait_for_current_stream"
        );
        assert_eq!(route_workers[0].decision_state_label(), "busy");
        assert!(route_workers[1].route_match);
        assert!(route_workers[1].selectable);
        assert_eq!(
            route_workers[1].picker_action,
            ModelRouteWorkerPickerAction::Select
        );
        assert_eq!(route_workers[1].picker_action_label, "select");
        assert_eq!(route_workers[1].decision_action_label(), "send_now");
        assert_eq!(route_workers[1].decision_state_label(), "pending");
        assert!(route_workers[2].route_match);
        assert!(!route_workers[2].selectable);
        assert_eq!(
            route_workers[2].picker_action,
            ModelRouteWorkerPickerAction::Wait
        );
        assert_eq!(route_workers[2].picker_action_label, "wait");
        assert_eq!(route_workers[2].decision_action_label(), "retry_later");
        assert_eq!(route_workers[2].decision_state_label(), "backpressure");
        assert_eq!(status.route_pool_status, None);
        assert_eq!(status.route_pool_queue_label, None);
        assert_eq!(status.route_pool, None);
        assert_eq!(status.route_pool_capacity_state, None);
        assert_eq!(status.route_pool_capacity_state_label, None);
        assert_eq!(status.route_pool_capacity_state_is_pressure, None);
        assert_eq!(status.route_pool_capacity_state_blocks_prompt_submit, None);
        assert_eq!(status.route_pool_has_matching_workers, None);
        assert_eq!(status.route_pool_has_matching_available_workers, None);
        assert_eq!(status.route_pool_has_matching_busy_workers, None);
        assert_eq!(status.route_pool_has_matching_saturated_workers, None);
        assert_eq!(status.route_pool_has_matching_queued_requests, None);
        assert_eq!(status.route_pool_queue_is_saturated, None);
        assert_eq!(
            status.gate_advice.as_deref(),
            Some("send_now pending: ready to send")
        );
        let advice = status
            .gate_advice_detail
            .as_ref()
            .expect("structured send advice should be available");
        assert_eq!(advice.action, GateAdviceAction::SendNow);
        assert_eq!(advice.state, StreamState::Pending);
        assert_eq!(advice.reason.as_str(), "ready to send");
        assert_eq!(status.gate_advice_action_label.as_deref(), Some("send_now"));
        assert!(status.send_allowed);
        assert_eq!(status.send_block_state, None);
        assert_eq!(status.send_block_state_label, None);
        assert_eq!(status.send_block_reason, None);
        assert!(!status.send_block_state_is_terminal);
        assert!(!status.send_block_state_is_pressure);
        assert!(!status.send_block_state_blocks_prompt_submit);
        assert_eq!(
            status.route_gate_advice.as_deref(),
            Some("send_now pending: ready to send")
        );
        assert_eq!(
            status.route_gate_advice_action_label.as_deref(),
            Some("send_now")
        );
        let route_advice = status
            .route_gate_advice_detail
            .as_ref()
            .expect("route advice should be structured");
        assert_eq!(route_advice.action, GateAdviceAction::SendNow);
        assert_eq!(route_advice.state, StreamState::Pending);
        assert_eq!(status.route_send_allowed, Some(true));
        assert_eq!(status.route_send_block_state, None);
        assert_eq!(status.route_send_block_state_label, None);
        assert_eq!(status.route_send_block_reason, None);
        assert_eq!(status.route_send_block_state_is_terminal, Some(false));
        assert_eq!(status.route_send_block_state_is_pressure, Some(false));
        assert_eq!(
            status.route_send_block_state_blocks_prompt_submit,
            Some(false)
        );
        assert_eq!(
            cli_model_pool_status_line(&input, &session, &gate),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending history=0 max_tokens=backend-default partial_chars=0 advice=send_now pending: ready to send pool=workers total=3 available=1 busy=1 saturated=1"
        );
    }

    #[test]
    fn model_pool_status_line_prefers_active_session_over_ready_worker_pool() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        session.push_delta("partial");
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);

        assert_eq!(
            status.gate_advice.as_deref(),
            Some("wait_for_current_stream busy: session stream is already active")
        );
        assert!(!status.send_allowed);
        assert_eq!(status.send_block_state, Some(StreamState::Busy));
        assert_eq!(
            status.send_block_reason.as_deref(),
            Some("session stream is already active")
        );
        assert!(!status.send_block_state_is_terminal);
        assert!(status.send_block_state_is_pressure);
        assert!(status.send_block_state_blocks_prompt_submit);
        assert_eq!(
            status.line(),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=streaming history=1 max_tokens=backend-default partial_chars=7 advice=wait_for_current_stream busy: session stream is already active pool=workers total=1 available=1 busy=0 saturated=0"
        );
    }

    #[test]
    fn model_pool_status_line_prefers_active_session_over_route_queue() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        session.push_delta("partial");
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
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

        assert_eq!(
            status.gate_advice.as_deref(),
            Some("wait_for_current_stream busy: session stream is already active")
        );
        assert!(!status.send_allowed);
        assert_eq!(status.send_block_state, Some(StreamState::Busy));
        assert_eq!(
            status.send_block_reason.as_deref(),
            Some("session stream is already active")
        );
        let send_block_chunk = status
            .send_block_chunk
            .as_ref()
            .expect("local active session should expose a display chunk");
        assert_eq!(send_block_chunk.output_label, "busy");
        assert_eq!(
            send_block_chunk.appended,
            "[busy] session stream is already active"
        );
        assert!(send_block_chunk.state_is_pressure);
        assert!(send_block_chunk.state_blocks_prompt_submit);
        assert_eq!(
            status.route_gate_advice.as_deref(),
            Some(
                "wait_for_worker queued: all model workers are busy; waiting for scheduler across 1 workers"
            )
        );
        assert_eq!(
            status.route_gate_advice_action_label.as_deref(),
            Some("wait_for_worker")
        );
        assert_eq!(
            status.route_gate_advice_state_label.as_deref(),
            Some("queued")
        );
        assert_eq!(
            status.route_gate_advice_reason.as_deref(),
            Some("all model workers are busy; waiting for scheduler across 1 workers")
        );
        assert_eq!(status.route_send_allowed, Some(false));
        assert_eq!(status.route_send_block_state, Some(StreamState::Queued));
        assert_eq!(
            status.route_send_block_state_label.as_deref(),
            Some("queued")
        );
        assert_eq!(
            status.route_send_block_reason.as_deref(),
            Some("all model workers are busy; waiting for scheduler across 1 workers")
        );
        assert_eq!(status.route_send_block_state_is_terminal, Some(false));
        assert_eq!(status.route_send_block_state_is_pressure, Some(true));
        assert_eq!(
            status.route_send_block_state_blocks_prompt_submit,
            Some(true)
        );
        let route_send_block_chunk = status
            .route_send_block_chunk
            .as_ref()
            .expect("route pressure should expose a display chunk");
        assert_eq!(route_send_block_chunk.output_label, "queued");
        assert_eq!(
            route_send_block_chunk.appended,
            "[queued] all model workers are busy; waiting for scheduler across 1 workers"
        );
        assert!(route_send_block_chunk.state_blocks_prompt_submit);
        assert_eq!(
            status.route_pool_status.as_deref(),
            Some("matching total=1 available=0 busy=1 saturated=0")
        );
        assert_eq!(status.pool_capacity_state, Some(StreamState::Busy));
        assert_eq!(status.pool_capacity_state_label.as_deref(), Some("busy"));
        assert_eq!(status.pool_capacity_state_is_pressure, Some(true));
        assert_eq!(status.pool_capacity_state_blocks_prompt_submit, Some(true));
        assert_eq!(status.route_pool_capacity_state, Some(StreamState::Busy));
        assert_eq!(
            status.route_pool_capacity_state_label.as_deref(),
            Some("busy")
        );
        assert_eq!(status.route_pool_capacity_state_is_pressure, Some(true));
        assert_eq!(
            status.route_pool_capacity_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(
            status.line(),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=streaming history=1 max_tokens=backend-default partial_chars=7 advice=wait_for_current_stream busy: session stream is already active pool=workers total=1 available=0 busy=1 saturated=0 route_pool=matching total=1 available=0 busy=1 saturated=0"
        );
    }

    #[test]
    fn model_pool_status_line_keeps_repair_gate_over_active_session() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                safe_device_ok: false,
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);

        assert_eq!(
            status.gate_advice.as_deref(),
            Some("repair_gate failed: safe-device gate failed")
        );
        assert!(!status.send_allowed);
        assert_eq!(status.send_block_state, Some(StreamState::Failed));
        assert_eq!(
            status.route_gate_advice.as_deref(),
            Some("repair_gate failed: safe-device gate failed")
        );
        assert_eq!(
            status.route_gate_advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(
            status.route_gate_advice_state_label.as_deref(),
            Some("failed")
        );
        assert_eq!(
            status.route_gate_advice_reason.as_deref(),
            Some("safe-device gate failed")
        );
        assert_eq!(status.route_send_allowed, Some(false));
        assert_eq!(status.route_send_block_state, Some(StreamState::Failed));
        assert_eq!(
            status.route_send_block_state_label.as_deref(),
            Some("failed")
        );
        assert_eq!(
            status.line(),
            "role=assistant preference=balanced endpoint=auto pinned=false state=streaming history=1 max_tokens=backend-default partial_chars=0 advice=repair_gate failed: safe-device gate failed pool=workers total=1 available=1 busy=0 saturated=0"
        );

        let hygiene_gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                experience_hygiene_ok: false,
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );

        let hygiene_status =
            CliStatusSnapshot::from_model_pool_gate(&input, &session, &hygiene_gate);

        assert_eq!(
            hygiene_status.gate_advice.as_deref(),
            Some("repair_gate failed: experience hygiene gate failed")
        );
        assert!(!hygiene_status.send_allowed);
        assert_eq!(hygiene_status.send_block_state, Some(StreamState::Failed));
        assert_eq!(
            hygiene_status.route_gate_advice.as_deref(),
            Some("repair_gate failed: experience hygiene gate failed")
        );
        assert_eq!(
            hygiene_status.route_gate_advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(
            hygiene_status.route_gate_advice_reason.as_deref(),
            Some("experience hygiene gate failed")
        );
        assert_eq!(
            hygiene_status.route_send_block_state,
            Some(StreamState::Failed)
        );
        assert_eq!(
            hygiene_status.route_send_block_reason.as_deref(),
            Some("experience hygiene gate failed")
        );
        let route_block = hygiene_status
            .route_send_block_chunk
            .as_ref()
            .expect("experience hygiene gate should expose a route block chunk");
        assert_eq!(route_block.output_label, "error");
        assert_eq!(
            route_block.appended,
            "[error] experience hygiene gate failed"
        );
        assert_eq!(
            hygiene_status.line(),
            "role=assistant preference=balanced endpoint=auto pinned=false state=streaming history=1 max_tokens=backend-default partial_chars=0 advice=repair_gate failed: experience hygiene gate failed pool=workers total=1 available=1 busy=0 saturated=0"
        );
    }

    #[test]
    fn workers_host_snapshot_projects_read_only_dto_for_web_and_forge() {
        let plain_status = CliStatusSnapshot::new(
            &CliInputConfig::default(),
            &ChatSession::new("cli", ChatSessionConfig::default()),
            None,
        );
        assert_eq!(plain_status.workers_host_snapshot(), None);

        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
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
        let dto = status
            .workers_host_snapshot()
            .expect("model-pool status should expose a read-only workers host DTO");
        let service_dto = gate
            .route_snapshot(&input.routing_intent())
            .workers_host_snapshot();

        assert!(dto.read_only);
        assert!(!dto.launches_process);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.carries_request_preview);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert_eq!(
            (
                dto.read_only,
                dto.launches_process,
                dto.sends_prompt,
                dto.starts_stream,
                dto.carries_request_preview,
                false,
                dto.carries_stream_chunk,
                dto.carries_input_action_snapshot,
            ),
            (
                service_dto.read_only,
                service_dto.launches_process,
                service_dto.sends_prompt,
                service_dto.starts_stream,
                service_dto.carries_request_preview,
                service_dto.mutates_history,
                service_dto.carries_stream_chunk,
                service_dto.carries_input_action_snapshot,
            )
        );
        assert_eq!(dto.history_messages, 0);
        assert_eq!(dto.partial_chars, 0);
        assert_eq!(dto.model_role_label, "reviewer");
        assert_eq!(dto.routing_preference_label, "prefer_fast");
        assert_eq!(dto.endpoint_label, "auto");
        assert!(!dto.endpoint_pinned);
        assert_eq!(dto.endpoint_kind_label, "auto");
        assert_eq!(dto.wire_model_role_label, "reviewer");
        assert_eq!(dto.wire_routing_preference_label, "prefer_fast");
        assert!(dto.wire_prefer_fast);
        assert!(!dto.wire_prefer_quality);
        assert!(!dto.wire_endpoint_pinned);
        assert_eq!(dto.wire_endpoint_kind_label, "auto");
        assert!(!dto.wire_sends_model_endpoint);
        assert_eq!(dto.wire_model_endpoint_label, None);
        assert!(!dto.send_allowed);
        assert_eq!(dto.send_block_state_label.as_deref(), Some("failed"));
        assert_eq!(
            dto.send_block_reason.as_deref(),
            Some("safe-device gate failed")
        );
        assert_eq!(dto.route_send_allowed, Some(false));
        assert_eq!(dto.route_send_block_state_label.as_deref(), Some("failed"));
        assert_eq!(
            dto.route_send_block_reason.as_deref(),
            Some("safe-device gate failed")
        );
        assert_eq!(dto.gate_advice_action_label.as_deref(), Some("repair_gate"));
        assert_eq!(
            dto.route_gate_advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(
            dto.pool_status.as_deref(),
            Some("workers total=3 available=1 busy=1 saturated=1")
        );
        assert_eq!(
            dto.route_pool_status.as_deref(),
            Some("matching total=3 available=1 busy=1 saturated=1")
        );

        assert_eq!(dto.workers.len(), 3);
        let ready = &dto.workers[0];
        assert_eq!(ready.endpoint_label, "fast-reviewer");
        assert_eq!(ready.role_labels, vec!["reviewer"]);
        assert_eq!(ready.preference_labels, vec!["prefer_fast"]);
        assert_eq!(ready.worker_status_label, "available");
        assert_eq!(ready.worker_status_state_label, "pending");
        assert!(ready.worker_status_is_available);
        assert_eq!(ready.worker_status_display_snapshot, None);

        let busy = &dto.workers[1];
        assert_eq!(busy.endpoint_label, "quality-12b");
        assert_eq!(busy.worker_status_label, "busy");
        assert!(busy.worker_status_is_pressure);
        assert!(busy.worker_status_blocks_prompt_submit);
        let busy_display = busy
            .worker_status_display_snapshot
            .as_ref()
            .expect("busy worker health should remain visible");
        assert_eq!(busy_display.output_label, "busy");
        assert_eq!(
            busy_display.appended,
            "[busy] worker quality-12b is busy: #41 review"
        );

        let saturated = &dto.workers[2];
        assert_eq!(saturated.endpoint_label, "summary-tester");
        assert_eq!(saturated.worker_status_label, "backpressure");
        assert!(saturated.worker_status_is_pressure);
        let saturated_display = saturated
            .worker_status_display_snapshot
            .as_ref()
            .expect("backpressure worker health should remain visible");
        assert_eq!(saturated_display.output_label, "backpressure");
        assert_eq!(
            saturated_display.appended,
            "[backpressure] worker summary-tester queue is saturated: 1/1"
        );

        for worker in &dto.workers {
            assert!(worker.route_match);
            assert!(!worker.selectable);
            assert_eq!(worker.picker_action_label, "repair_gate");
            assert_eq!(worker.decision_action_label, "repair_gate");
            assert_eq!(worker.decision_state_label, "failed");
            assert_eq!(worker.decision_reason, "safe-device gate failed");
            assert_eq!(worker.selection_model_role_label, "reviewer");
            assert_eq!(worker.selection_routing_preference_label, "prefer_fast");
            assert!(worker.selection_wire_prefer_fast);
            assert!(!worker.selection_wire_prefer_quality);
            assert!(worker.selection_wire_endpoint_pinned);
            assert!(worker.selection_wire_sends_model_endpoint);
            let decision = worker
                .decision_display_snapshot
                .as_ref()
                .expect("repair gate row should carry decision display");
            assert_eq!(decision.output_label, "error");
            assert_eq!(decision.appended, "[error] safe-device gate failed");
        }
        assert_eq!(
            dto.workers[0]
                .selection_wire_model_endpoint_label
                .as_deref(),
            Some("fast-reviewer")
        );
        assert_eq!(
            dto.workers[2]
                .selection_wire_model_endpoint_label
                .as_deref(),
            Some("summary-tester")
        );
    }

    #[test]
    fn workers_host_snapshot_projects_8686_8690_readiness_for_web_lab_and_forge() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(endpoint(8686))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(endpoint(8687))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8687 active review".to_owned())),
                ModelWorkerSnapshot::new(endpoint(8688))
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(endpoint(8689))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(1, 1),
                ModelWorkerSnapshot::new(endpoint(8690))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status
            .workers_host_snapshot()
            .expect("model-pool status should expose a read-only workers host DTO");

        assert!(dto.read_only);
        assert!(!dto.launches_process);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.carries_request_preview);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert!(dto.send_allowed);
        assert_eq!(dto.route_send_allowed, Some(true));
        assert_eq!(dto.gate_advice_action_label.as_deref(), Some("send_now"));
        assert_eq!(
            dto.route_gate_advice_action_label.as_deref(),
            Some("send_now")
        );
        assert_eq!(
            dto.pool_status.as_deref(),
            Some("workers total=5 available=3 busy=1 saturated=1")
        );
        assert_eq!(
            dto.route_pool_status.as_deref(),
            Some("matching total=4 available=2 busy=1 saturated=1")
        );
        assert_eq!(dto.history_messages, 0);
        assert_eq!(dto.partial_chars, 0);
        assert_eq!(dto.workers.len(), 5);

        let ready = &dto.workers[0];
        assert_eq!(ready.endpoint_label, "127.0.0.1:8686");
        assert_eq!(ready.worker_status_label, "available");
        assert_eq!(ready.worker_status_state_label, "pending");
        assert!(ready.worker_status_is_available);
        assert!(ready.route_match);
        assert!(ready.selectable);
        assert_eq!(ready.picker_action_label, "select");
        assert_eq!(
            ready.selection_wire_model_endpoint_label.as_deref(),
            Some("127.0.0.1:8686")
        );

        let busy = &dto.workers[1];
        assert_eq!(busy.endpoint_label, "127.0.0.1:8687");
        assert_eq!(busy.worker_status_label, "busy");
        assert!(busy.worker_status_is_pressure);
        assert!(busy.worker_status_blocks_prompt_submit);
        assert!(busy.route_match);
        assert!(!busy.selectable);
        assert_eq!(busy.picker_action_label, "wait");
        assert_eq!(busy.decision_action_label, "wait_for_current_stream");
        assert_eq!(busy.decision_state_label, "busy");

        let unavailable = &dto.workers[2];
        assert_eq!(unavailable.endpoint_label, "127.0.0.1:8688");
        assert_eq!(unavailable.worker_status_label, "available");
        assert!(unavailable.worker_status_is_available);
        assert!(!unavailable.route_match);
        assert!(!unavailable.selectable);
        assert_eq!(unavailable.picker_action_label, "unavailable");
        assert_eq!(unavailable.decision_action_label, "wait_for_worker");
        assert_eq!(unavailable.decision_state_label, "queued");

        let saturated = &dto.workers[3];
        assert_eq!(saturated.endpoint_label, "127.0.0.1:8689");
        assert_eq!(saturated.worker_status_label, "backpressure");
        assert_eq!(saturated.worker_status_state_label, "backpressure");
        assert!(saturated.worker_status_is_pressure);
        assert!(saturated.route_match);
        assert!(!saturated.selectable);
        assert_eq!(saturated.picker_action_label, "wait");
        assert_eq!(saturated.decision_action_label, "retry_later");

        let ready_tail = &dto.workers[4];
        assert_eq!(ready_tail.endpoint_label, "127.0.0.1:8690");
        assert_eq!(ready_tail.worker_status_label, "available");
        assert!(ready_tail.worker_status_is_available);
        assert!(ready_tail.route_match);
        assert!(ready_tail.selectable);
        assert_eq!(ready_tail.picker_action_label, "select");
    }

    #[test]
    fn workers_host_snapshot_json_field_names_are_stable_for_8686_8690() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(endpoint(8686))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(endpoint(8687))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8687 active review".to_owned())),
                ModelWorkerSnapshot::new(endpoint(8688))
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(endpoint(8689))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(1, 1),
                ModelWorkerSnapshot::new(endpoint(8690))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status
            .workers_host_snapshot()
            .expect("model-pool status should expose a read-only workers host DTO");

        assert_eq!(
            cli_workers_host_json_fields(&dto),
            vec![
                "read_only",
                "launches_process",
                "sends_prompt",
                "starts_stream",
                "carries_request_preview",
                "carries_stream_chunk",
                "carries_input_action_snapshot",
                "route",
                "model_role_label",
                "routing_preference_label",
                "endpoint_label",
                "endpoint_pinned",
                "endpoint_kind_label",
                "wire_model_role_label",
                "wire_routing_preference_label",
                "wire_prefer_fast",
                "wire_prefer_quality",
                "wire_endpoint_pinned",
                "wire_endpoint_kind_label",
                "wire_sends_model_endpoint",
                "wire_model_endpoint_label",
                "send_allowed",
                "send_block_state_label",
                "send_block_reason",
                "route_send_allowed",
                "route_send_block_state_label",
                "route_send_block_reason",
                "gate_advice_action_label",
                "route_gate_advice_action_label",
                "pool_status",
                "route_pool_status",
                "history_messages",
                "partial_chars",
                "workers",
            ]
        );
        assert_eq!(
            cli_worker_host_json_fields(&dto.workers[0]),
            vec![
                "endpoint_label",
                "role_labels",
                "preference_labels",
                "worker_status_label",
                "worker_status_state_label",
                "worker_status_is_available",
                "worker_status_is_pressure",
                "worker_status_blocks_prompt_submit",
                "worker_status_display_snapshot",
                "endpoint_selected",
                "route_match",
                "selectable",
                "picker_action_label",
                "decision_action_label",
                "decision_state_label",
                "decision_reason",
                "decision_display_snapshot",
                "selection_summary",
                "selection_model_role_label",
                "selection_routing_preference_label",
                "selection_endpoint_label",
                "selection_endpoint_kind_label",
                "selection_wire_model_role_label",
                "selection_wire_routing_preference_label",
                "selection_wire_prefer_fast",
                "selection_wire_prefer_quality",
                "selection_wire_endpoint_pinned",
                "selection_wire_endpoint_kind_label",
                "selection_wire_sends_model_endpoint",
                "selection_wire_model_endpoint_label",
            ]
        );
        assert!(dto.read_only);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert_eq!(dto.workers[0].endpoint_label, "127.0.0.1:8686");
        assert_eq!(dto.workers[1].worker_status_label, "busy");
        assert_eq!(dto.workers[2].picker_action_label, "unavailable");
        assert_eq!(dto.workers[3].worker_status_label, "backpressure");
        assert_eq!(dto.workers[4].endpoint_label, "127.0.0.1:8690");
    }

    #[test]
    fn status_consumer_reads_8686_8690_without_replaying_input_or_changing_session() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("active prompt")
            .expect("setup should create a local active stream");
        session.push_delta("partial answer");
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#8687 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(endpoint(8686))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(endpoint(8687))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8687 active review".to_owned())),
                ModelWorkerSnapshot::new(endpoint(8688))
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(endpoint(8689))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(1, 1),
                ModelWorkerSnapshot::new(endpoint(8690))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let before_state = session.state();
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();
        let before_chunks = session.chunks().len();

        let first_status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let first_dto = first_status
            .workers_host_snapshot()
            .expect("model-pool status should expose a read-only workers host DTO");
        let second_status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let second_dto = second_status
            .workers_host_snapshot()
            .expect("model-pool status should expose a read-only workers host DTO");

        assert_eq!(first_status, second_status);
        assert_eq!(first_dto, second_dto);
        assert_eq!(session.state(), before_state);
        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert_eq!(session.chunks().len(), before_chunks);
        assert!(first_dto.read_only);
        assert!(!first_dto.launches_process);
        assert!(!first_dto.sends_prompt);
        assert!(!first_dto.starts_stream);
        assert!(!first_dto.carries_request_preview);
        assert!(!first_dto.carries_stream_chunk);
        assert!(!first_dto.carries_input_action_snapshot);
        assert_eq!(first_dto.history_messages, before_history);
        assert_eq!(first_dto.partial_chars, before_partial.chars().count());
        assert!(!first_dto.send_allowed);
        assert_eq!(
            first_dto.gate_advice_action_label.as_deref(),
            Some("wait_for_current_stream")
        );
        assert_eq!(
            first_dto.route_gate_advice_action_label.as_deref(),
            Some("wait_for_current_stream")
        );
        assert_eq!(
            first_dto.pool_status.as_deref(),
            Some("workers total=5 available=3 busy=1 saturated=1")
        );
        assert_eq!(
            first_dto.route_pool_status.as_deref(),
            Some("matching total=4 available=2 busy=1 saturated=1")
        );
        assert_eq!(
            first_dto
                .workers
                .iter()
                .map(|worker| (
                    worker.endpoint_label.as_str(),
                    worker.worker_status_label.as_str(),
                    worker.worker_status_state_label.as_str(),
                    worker.worker_status_is_available,
                    worker.route_match,
                    worker.selectable,
                    worker.picker_action_label.as_str(),
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "127.0.0.1:8686",
                    "available",
                    "pending",
                    true,
                    true,
                    false,
                    "wait"
                ),
                ("127.0.0.1:8687", "busy", "busy", false, true, false, "wait"),
                (
                    "127.0.0.1:8688",
                    "available",
                    "pending",
                    true,
                    false,
                    false,
                    "unavailable",
                ),
                (
                    "127.0.0.1:8689",
                    "backpressure",
                    "backpressure",
                    false,
                    true,
                    false,
                    "wait",
                ),
                (
                    "127.0.0.1:8690",
                    "available",
                    "pending",
                    true,
                    true,
                    false,
                    "wait"
                ),
            ]
        );
    }

    #[test]
    fn smartsteam_status_host_snapshot_reads_daemon_supervisor_pool_without_replay() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("active prompt")
            .expect("setup should create a local active stream");
        session.push_delta("partial answer");
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("round=314 generate:start".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(endpoint(8686))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(endpoint(8687))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8687 active review".to_owned())),
                ModelWorkerSnapshot::new(endpoint(8688))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(1, 1),
            ],
        );
        let source = norion_service::SmartSteamStatusSource::new()
            .with_daemon(true, Some(224392), Some(314), Some(313))
            .with_supervisor(true, true)
            .with_readiness(false, true)
            .with_model_cache_label("5/5 external diagnostic");
        let before_state = session.state();
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();
        let before_chunks = session.chunks().len();

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            source.clone(),
            &gate,
            Some(&input.routing_intent()),
        );
        let first = status.smartsteam_status_host_snapshot(service_status);
        let second_status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let second_service_status =
            SmartSteamStatusSnapshot::from_model_pool(source, &gate, Some(&input.routing_intent()));
        let second = second_status.smartsteam_status_host_snapshot(second_service_status);

        assert_eq!(first, second);
        assert_eq!(session.state(), before_state);
        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert_eq!(session.chunks().len(), before_chunks);
        assert!(first.read_only);
        assert!(!first.launches_process);
        assert!(!first.starts_daemon);
        assert!(!first.stops_daemon);
        assert!(!first.touches_remote);
        assert!(!first.downloads_model);
        assert!(!first.warms_model_cache);
        assert!(!first.sends_prompt);
        assert!(!first.starts_stream);
        assert!(!first.replays_prompt);
        assert!(!first.mutates_busy);
        assert!(!first.mutates_readiness);
        assert!(!first.mutates_active_round);
        assert!(!first.carries_request_preview);
        assert!(!first.carries_stream_chunk);
        assert!(!first.carries_input_action_snapshot);
        assert_eq!(first.history_messages, before_history);
        assert_eq!(first.partial_chars, before_partial.chars().count());
        assert!(first.service_status.daemon_running);
        assert_eq!(first.service_status.daemon_pid, Some(224392));
        assert_eq!(first.service_status.active_round, Some(314));
        assert_eq!(first.service_status.ledger_round, Some(313));
        assert!(!first.service_status.readiness_ok);
        assert!(first.service_status.engine_busy);
        assert_eq!(
            first.service_status.active_request.as_deref(),
            Some("round=314 generate:start")
        );
        assert_eq!(
            first.service_status.pool_status,
            "workers total=3 available=1 busy=1 saturated=1"
        );
        assert_eq!(
            first.service_status.route_pool_status.as_deref(),
            Some("matching total=3 available=1 busy=1 saturated=1")
        );
        assert_eq!(
            first.service_status.route_send_block_reason.as_deref(),
            Some("backend engine is busy: round=314 generate:start")
        );
    }

    #[test]
    fn smartsteam_status_host_snapshot_surfaces_round_done_ledger_pending_without_replay() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                active_request: Some("round=333 done [DONE]".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(192756), Some(333), Some(332))
                .with_supervisor(true, true)
                .with_readiness(true, true)
                .with_daemon_round_transition(
                    norion_service::SmartSteamDaemonRoundTransitionStatusSource::round_done_ledger_pending(
                        333,
                        Some(332),
                    )
                    .with_evidence_ids(["stdout:round-333:done", "ledger:latest-round-332"]),
                ),
            &gate,
            Some(&input.routing_intent()),
        );
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);
        let transition = dto
            .daemon_round_transition_status
            .as_ref()
            .expect("CLI host should expose daemon round transition status");

        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_active_round);
        assert_eq!(dto.history_messages, before_history);
        assert_eq!(dto.partial_chars, 0);
        assert_eq!(
            dto.daemon_round_transition_summary,
            dto.service_status.daemon_round_transition_summary
        );
        assert_eq!(transition.status_label, "round-done-ledger-commit-pending");
        assert_eq!(transition.done_round, Some(333));
        assert_eq!(transition.latest_done_round, Some(333));
        assert!(!transition.round_in_progress);
        assert_eq!(transition.ledger_round, Some(332));
        assert_eq!(transition.ledger_lag_rounds, Some(1));
        assert!(transition.ledger_commit_pending);
        assert!(!transition.starts_daemon);
        assert!(!transition.sends_prompt);
        assert!(!transition.starts_stream);
        assert!(!transition.writes_ndkv);
    }

    #[test]
    fn smartsteam_status_host_snapshot_preserves_absent_next_round_decision_compatibility() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(197412), Some(367), Some(366))
                .with_daemon_round_progress(Some(366), true)
                .with_supervisor(true, true)
                .with_readiness(true, true),
            &gate,
            Some(&input.routing_intent()),
        );

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);

        assert_eq!(dto.next_round_decision_status, None);
        assert_eq!(dto.next_round_decision_summary, None);
        assert_eq!(dto.service_status.next_round_decision_status, None);
        assert_eq!(dto.service_status.next_round_decision_summary, None);
    }

    #[test]
    fn smartsteam_status_host_snapshot_surfaces_next_round_decision_without_replay() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("round=367 generate:start".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("round=367 quality worker".to_owned())),
            ],
        );
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(197412), Some(367), Some(366))
                .with_daemon_round_progress(Some(366), true)
                .with_supervisor(true, true)
                .with_readiness(true, true)
                .with_next_round_decision(
                    norion_service::SmartSteamNextRoundDecisionStatusSource::safe_to_continue_after_current_round(
                        367,
                        366,
                    )
                    .with_evidence_ids(["next-round:round-367:continue"])
                    .with_reason_codes(["current_round_will_complete", "safe_to_continue"]),
                ),
            &gate,
            Some(&input.routing_intent()),
        );
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();
        let before_chunks = session.chunks().len();

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);
        let decision = dto
            .next_round_decision_status
            .as_ref()
            .expect("CLI host should expose next-round decision status");

        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert_eq!(session.chunks().len(), before_chunks);
        assert!(dto.read_only);
        assert!(!dto.launches_process);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_active_round);
        assert!(!dto.mutates_worker_window_status);
        assert!(!dto.carries_request_preview);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert_eq!(dto.history_messages, before_history);
        assert_eq!(dto.partial_chars, 0);
        assert_eq!(
            dto.next_round_decision_status,
            dto.service_status.next_round_decision_status
        );
        assert_eq!(
            dto.next_round_decision_summary,
            dto.service_status.next_round_decision_summary
        );
        assert_eq!(decision.report_version, "next_round_decision_report_v1");
        assert_eq!(
            decision.status_label,
            "safe-to-continue-after-current-round"
        );
        assert!(!decision.safe_to_wait_current_round_active);
        assert!(decision.safe_to_continue_after_current_round);
        assert!(!decision.operator_attention_blocked);
        assert_eq!(decision.current_round, Some(367));
        assert_eq!(decision.latest_done_round, Some(366));
        assert_eq!(decision.evidence_ids, vec!["next-round:round-367:continue"]);
        assert!(!decision.starts_daemon);
        assert!(!decision.sends_prompt);
        assert!(!decision.starts_stream);
        assert!(!decision.writes_ndkv);
        assert!(!decision.creates_thread);
    }

    #[test]
    fn smartsteam_status_host_snapshot_consumes_report_shaped_next_round_decision_without_replay() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("round=369 generate:start".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("round=369 quality worker".to_owned())),
            ],
        );
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(199264), Some(369), Some(368))
                .with_daemon_round_progress(Some(368), true)
                .with_supervisor(true, true)
                .with_readiness(true, true)
                .with_next_round_decision_report(
                    norion_service::SmartSteamNextRoundDecisionReportStatusSource {
                        decision_status: Some("safe_to_wait_current_round_active".to_owned()),
                        display_state: Some("safe-to-wait".to_owned()),
                        live_status_display_state: Some("safe-to-wait".to_owned()),
                        current_round_active: Some(true),
                        readiness_can_schedule_next_round: Some(false),
                        report_gate_ready: Some(true),
                        context_hygiene_passed: Some(true),
                        read_only: Some(true),
                        report_only: Some(true),
                        no_side_effects: Some(true),
                        dispatch_work_allowed: Some(false),
                        prompt_replay_allowed: Some(false),
                        process_start_allowed: Some(false),
                        memory_write_allowed: Some(false),
                        ndkv_write_allowed: Some(false),
                        current_round: Some(369),
                        latest_done_round: Some(368),
                        evidence_ids: vec![
                            "live_status_bundle:next_round_decision:round-369".to_owned(),
                        ],
                        reason_codes: vec!["current_round_active".to_owned()],
                        ..norion_service::SmartSteamNextRoundDecisionReportStatusSource::default()
                    },
                ),
            &gate,
            Some(&input.routing_intent()),
        );
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();
        let before_chunks = session.chunks().len();

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);
        let decision = dto
            .next_round_decision_status
            .as_ref()
            .expect("CLI host should expose report-shaped next-round decision status");

        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert_eq!(session.chunks().len(), before_chunks);
        assert!(dto.read_only);
        assert!(!dto.launches_process);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_active_round);
        assert!(!dto.mutates_worker_window_status);
        assert!(!dto.carries_request_preview);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert_eq!(
            dto.next_round_decision_status,
            dto.service_status.next_round_decision_status
        );
        assert_eq!(
            dto.next_round_decision_summary,
            dto.service_status.next_round_decision_summary
        );
        assert_eq!(decision.report_version, "next_round_decision_report_v1");
        assert_eq!(decision.status_label, "safe-to-wait-current-round-active");
        assert!(decision.safe_to_wait_current_round_active);
        assert!(!decision.safe_to_continue_after_current_round);
        assert!(!decision.operator_attention_blocked);
        assert_eq!(decision.current_round, Some(369));
        assert_eq!(decision.latest_done_round, Some(368));
        assert_eq!(
            decision.evidence_ids,
            vec!["live_status_bundle:next_round_decision:round-369"]
        );
        assert!(!decision.starts_daemon);
        assert!(!decision.sends_prompt);
        assert!(!decision.starts_stream);
        assert!(!decision.replays_prompt);
        assert!(!decision.writes_ndkv);
        assert!(!decision.creates_thread);
    }

    #[test]
    fn smartsteam_status_host_snapshot_surfaces_captured_current_status_json_next_round_decision() {
        let captured_json = captured_current_status_next_round_decision_json_fixture();
        assert_captured_current_status_next_round_decision_shape(captured_json);
        let report = captured_current_status_next_round_decision_report_from_json(captured_json)
            .expect("captured current-status JSON should expose a next-round decision report");
        let downstream_report = report
            .downstream_status_consumers
            .as_ref()
            .expect("captured current-status JSON should carry optional downstream facts");
        assert_eq!(
            downstream_report.service_cli_display_status.as_deref(),
            Some("display_safe_to_wait_current_round")
        );
        assert_eq!(downstream_report.read_only, Some(true));
        assert_eq!(downstream_report.report_only, Some(true));
        assert_eq!(downstream_report.no_side_effects, Some(true));
        assert_eq!(downstream_report.process_start_allowed, Some(false));
        let round_id_evidence = downstream_report
            .round_id_evidence
            .as_ref()
            .expect("captured downstream status should carry round-id evidence");
        assert_eq!(
            round_id_evidence.source_schema.as_deref(),
            Some("daemon_round_transition_status_v1")
        );
        assert_eq!(round_id_evidence.active_round, Some(370));
        assert_eq!(round_id_evidence.ledger_latest_round, Some(369));
        assert_eq!(round_id_evidence.latest_done_round, Some(369));
        assert_eq!(
            round_id_evidence.transition_kind.as_deref(),
            Some("normal_in_progress")
        );

        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("round=370 generate:start".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("round=370 quality worker".to_owned())),
            ],
        );
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(199264), Some(370), Some(369))
                .with_daemon_round_progress(Some(369), true)
                .with_supervisor(true, true)
                .with_readiness(true, true)
                .with_next_round_decision_report(report),
            &gate,
            Some(&input.routing_intent()),
        );
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();
        let before_chunks = session.chunks().len();

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);
        let decision = dto
            .next_round_decision_status
            .as_ref()
            .expect("CLI host should copy captured next-round decision evidence");

        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert_eq!(session.chunks().len(), before_chunks);
        assert!(dto.read_only);
        assert!(!dto.launches_process);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.downloads_model);
        assert!(!dto.warms_model_cache);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_active_round);
        assert!(!dto.starts_clean_room_replacement);
        assert!(!dto.mutates_worker_window_status);
        assert!(!dto.carries_request_preview);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert_eq!(dto.history_messages, before_history);
        assert_eq!(dto.partial_chars, 0);
        assert_eq!(
            dto.next_round_decision_status,
            dto.service_status.next_round_decision_status
        );
        assert_eq!(
            dto.next_round_decision_summary,
            dto.service_status.next_round_decision_summary
        );
        assert_eq!(decision.report_version, "next_round_decision_report_v1");
        assert!(decision.read_only);
        assert!(decision.report_only);
        assert_eq!(decision.status_label, "safe-to-wait-current-round-active");
        assert!(decision.safe_to_wait_current_round_active);
        assert!(!decision.safe_to_continue_after_current_round);
        assert!(!decision.operator_attention_blocked);
        assert_eq!(decision.current_round, Some(370));
        assert_eq!(decision.latest_done_round, Some(369));
        assert_eq!(
            decision.evidence_ids,
            vec![
                "live_status_bundle:next_round_decision:round-370",
                "next_round_decision:display_state:safe-to-wait"
            ]
        );
        assert_eq!(
            decision.reason_codes,
            vec!["current_round_active", "safe_to_wait"]
        );
        let downstream = decision
            .downstream_status_consumers
            .as_ref()
            .expect("CLI host should copy downstream display facts");
        assert_eq!(
            downstream.schema_version,
            "next_round_downstream_status_consumers_v1"
        );
        assert_eq!(
            downstream.service_cli_display_status,
            "display_safe_to_wait_current_round"
        );
        assert_eq!(
            downstream.forge_operator_display_status,
            "forge_safe_to_wait"
        );
        assert_eq!(
            downstream.agent_assignment_acceptance,
            "defer_until_current_round_completes"
        );
        assert_eq!(
            downstream.memory_self_improve_admission_visibility,
            "visible_admission_waiting"
        );
        assert!(!downstream.operator_attention_required);
        assert!(downstream.read_only);
        assert!(downstream.report_only);
        assert!(downstream.no_side_effects);
        assert!(!downstream.dispatch_work_allowed);
        assert!(!downstream.prompt_replay_allowed);
        assert!(!downstream.process_start_allowed);
        assert!(!downstream.memory_write_allowed);
        assert!(!downstream.ndkv_write_allowed);
        let round_id_evidence = downstream
            .round_id_evidence
            .as_ref()
            .expect("CLI host should copy downstream round-id evidence");
        assert_eq!(
            round_id_evidence.source_schema.as_deref(),
            Some("daemon_round_transition_status_v1")
        );
        assert_eq!(round_id_evidence.active_round, Some(370));
        assert_eq!(round_id_evidence.ledger_latest_round, Some(369));
        assert_eq!(round_id_evidence.latest_done_round, Some(369));
        assert_eq!(
            round_id_evidence.transition_status_label.as_deref(),
            Some("round-in-progress")
        );
        assert_eq!(round_id_evidence.ledger_commit_pending, Some(false));
        assert_eq!(round_id_evidence.round_in_progress, Some(true));
        assert_eq!(
            round_id_evidence.evidence_ids,
            vec![
                "daemon_transition:active-round-370",
                "ledger:latest-round-369"
            ]
        );
        assert!(!decision.starts_daemon);
        assert!(!decision.stops_daemon);
        assert!(!decision.touches_remote);
        assert!(!decision.sends_prompt);
        assert!(!decision.starts_stream);
        assert!(!decision.replays_prompt);
        assert!(!decision.mutates_active_round);
        assert!(!decision.mutates_worker_window_status);
        assert!(!decision.writes_ndkv);
        assert!(!decision.creates_thread);
        assert!(!decision.operator_action_required);
    }

    #[test]
    fn captured_current_status_json_downstream_consumers_accept_root_and_nested_round_evidence() {
        let nested_report = captured_current_status_next_round_decision_report_from_json(
            r#"{
  "active_round": 371,
  "ledger_latest_round": 370,
  "latest_done_round": 371,
  "live_status_bundle": {
    "next_round_decision": {
      "decision_status": "safe_to_wait_current_round_active",
      "display_state": "safe-to-wait",
      "current_round_active": true,
      "read_only": true,
      "report_only": true,
      "no_side_effects": true,
      "dispatch_work_allowed": false,
      "prompt_replay_allowed": false,
      "process_start_allowed": false,
      "memory_write_allowed": false,
      "ndkv_write_allowed": false,
      "current_round": 371,
      "latest_done_round": 371
    },
    "next_round_downstream_status_consumers_v1": {
      "next_round_downstream": {
        "source_decision_status": "safe_to_wait_current_round_active",
        "effective_decision_status": "safe_to_wait_current_round_active",
        "service_cli_display_status": "display_safe_to_wait_current_round",
        "forge_operator_display_status": "forge_safe_to_wait",
        "agent_assignment_acceptance": "defer_until_current_round_completes",
        "memory_self_improve_admission_visibility": "visible_admission_waiting",
        "read_only": true,
        "report_only": true,
        "no_side_effects": true,
        "dispatch_work_allowed": false,
        "prompt_replay_allowed": false,
        "process_start_allowed": false,
        "memory_write_allowed": false,
        "ndkv_write_allowed": false,
        "active_round": 371,
        "ledger_latest_round": 370,
        "latest_done_round": 371,
        "round_id_evidence": {
          "active_round": 371,
          "ledger_latest_round": 370,
          "latest_done_round": 371,
          "transition_kind": "round_done_waiting_ledger_commit",
          "transition_status_label": "round-done-ledger-commit-pending",
          "ledger_commit_pending": true,
          "round_in_progress": false,
          "evidence_ids": ["stdout:round-371:done"],
          "reason_codes": ["round_done_before_ledger_commit"]
        }
      }
    }
  }
}"#,
        )
        .expect("nested downstream consumer status should parse");
        let nested_downstream = nested_report
            .into_status_source()
            .expect("nested report should map to next-round status")
            .downstream_status_consumers
            .expect("nested downstream status should be retained");
        let nested_evidence = nested_downstream
            .round_id_evidence
            .expect("nested downstream status should carry round evidence");
        assert_eq!(nested_evidence.active_round, Some(371));
        assert_eq!(nested_evidence.ledger_latest_round, Some(370));
        assert_eq!(nested_evidence.latest_done_round, Some(371));
        assert_eq!(
            nested_evidence.transition_kind.as_deref(),
            Some("round_done_waiting_ledger_commit")
        );
        assert_eq!(nested_evidence.ledger_commit_pending, Some(true));
        assert_eq!(nested_evidence.round_in_progress, Some(false));

        let root_report = captured_current_status_next_round_decision_report_from_json(
            r#"{
  "next_round_decision_report_v1": {
    "decision_status": "safe_to_continue_after_current_round",
    "display_state": "safe-to-continue-after-current-round",
    "readiness_can_schedule_next_round": true,
    "read_only": true,
    "report_only": true,
    "no_side_effects": true,
    "dispatch_work_allowed": false,
    "prompt_replay_allowed": false,
    "process_start_allowed": false,
    "memory_write_allowed": false,
    "ndkv_write_allowed": false,
    "current_round": 371,
    "latest_done_round": 371
  },
  "next_round_downstream_status_consumers_v1": {
    "source_decision_status": "safe_to_continue_after_current_round",
    "effective_decision_status": "safe_to_continue_after_current_round",
    "service_cli_display_status": "display_safe_to_continue",
    "forge_operator_display_status": "forge_safe_to_continue",
    "agent_assignment_acceptance": "accept_next_round_assignment",
    "memory_self_improve_admission_visibility": "visible_admission_safe",
    "read_only": true,
    "report_only": true,
    "no_side_effects": true,
    "dispatch_work_allowed": false,
    "prompt_replay_allowed": false,
    "process_start_allowed": false,
    "memory_write_allowed": false,
    "ndkv_write_allowed": false,
    "active_round": 371,
    "ledger_latest_round": 371,
    "latest_done_round": 371,
    "round_id_evidence": {
      "active_round": 371,
      "ledger_latest_round": 371,
      "latest_done_round": 371,
      "transition_kind": "normal_in_progress",
      "transition_status_label": "safe-to-continue-after-current-round",
      "ledger_commit_pending": false,
      "round_in_progress": false,
      "evidence_ids": ["ledger:latest-round-371"],
      "reason_codes": ["round_ids_consistent"]
    }
  }
}"#,
        )
        .expect("root flat downstream consumer status should parse");
        let root_downstream = root_report
            .into_status_source()
            .expect("root flat report should map to next-round status")
            .downstream_status_consumers
            .expect("root flat downstream status should be retained");
        let root_evidence = root_downstream
            .round_id_evidence
            .expect("root flat downstream status should carry round evidence");
        assert_eq!(root_evidence.active_round, Some(371));
        assert_eq!(root_evidence.ledger_latest_round, Some(371));
        assert_eq!(root_evidence.latest_done_round, Some(371));
        assert_eq!(root_evidence.ledger_commit_pending, Some(false));
        assert_eq!(root_evidence.evidence_ids, vec!["ledger:latest-round-371"]);
    }

    #[test]
    fn smartsteam_status_host_replays_post_r44_safe_to_wait_root_and_live_bundle_downstream_status()
    {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("round=380 generate:start".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("round=380 quality worker".to_owned())),
            ],
        );

        for (label, captured_json) in [
            (
                "root",
                r#"{
  "daemon_running": true,
  "active_round": 380,
  "ledger_latest_round": 379,
  "latest_done_round": 379,
  "round_in_progress": true,
  "live_status_bundle": {
    "display_state": "safe-to-wait",
    "next_round_decision": {
      "decision_status": "safe_to_wait_current_round_active",
      "display_state": "safe-to-wait",
      "live_status_display_state": "safe-to-wait",
      "current_round_active": true,
      "readiness_can_schedule_next_round": false,
      "report_gate_ready": true,
      "context_hygiene_passed": true,
      "read_only": true,
      "report_only": true,
      "no_side_effects": true,
      "dispatch_work_allowed": false,
      "prompt_replay_allowed": false,
      "process_start_allowed": false,
      "memory_write_allowed": false,
      "ndkv_write_allowed": false,
      "current_round": 380,
      "latest_done_round": 379,
      "evidence_ids": ["live_status_bundle:next_round_decision:round-380"],
      "reason_codes": ["current_round_active", "safe_to_wait"]
    }
  },
  "next_round_downstream_status_consumers_v1": {
    "next_round_downstream": {
      "source_decision_status": "safe_to_wait_current_round_active",
      "effective_decision_status": "safe_to_wait_current_round_active",
      "service_cli_display_status": "display_safe_to_wait_current_round",
      "forge_operator_display_status": "forge_safe_to_wait",
      "agent_assignment_acceptance": "defer_until_current_round_completes",
      "memory_self_improve_admission_visibility": "visible_admission_waiting",
      "operator_attention_required": false,
      "read_only": true,
      "report_only": true,
      "no_side_effects": true,
      "dispatch_work_allowed": false,
      "prompt_replay_allowed": false,
      "process_start_allowed": false,
      "memory_write_allowed": false,
      "ndkv_write_allowed": false,
      "current_round_active": true,
      "live_status_display_state": "safe-to-wait",
      "active_round": 380,
      "ledger_latest_round": 379,
      "latest_done_round": 379,
      "readiness_can_schedule_next_round": false,
      "round_id_evidence": {
        "source_schema": "daemon_round_transition_status_v1",
        "active_round": 380,
        "ledger_latest_round": 379,
        "latest_done_round": 379,
        "transition_kind": "normal_in_progress",
        "transition_status_label": "round-in-progress",
        "ledger_commit_pending": false,
        "round_in_progress": true,
        "evidence_ids": ["daemon_transition:active-round-380", "ledger:latest-round-379"],
        "reason_codes": ["current_round_active", "safe_to_wait"]
      },
      "failure_reasons": []
    }
  }
}"#,
            ),
            (
                "live_status_bundle",
                r#"{
  "daemon_running": true,
  "active_round": 380,
  "ledger_latest_round": 379,
  "latest_done_round": 379,
  "round_in_progress": true,
  "live_status_bundle": {
    "display_state": "safe-to-wait",
    "next_round_decision": {
      "decision_status": "safe_to_wait_current_round_active",
      "display_state": "safe-to-wait",
      "live_status_display_state": "safe-to-wait",
      "current_round_active": true,
      "readiness_can_schedule_next_round": false,
      "report_gate_ready": true,
      "context_hygiene_passed": true,
      "read_only": true,
      "report_only": true,
      "no_side_effects": true,
      "dispatch_work_allowed": false,
      "prompt_replay_allowed": false,
      "process_start_allowed": false,
      "memory_write_allowed": false,
      "ndkv_write_allowed": false,
      "current_round": 380,
      "latest_done_round": 379,
      "evidence_ids": ["live_status_bundle:next_round_decision:round-380"],
      "reason_codes": ["current_round_active", "safe_to_wait"]
    },
    "next_round_downstream_status_consumers_v1": {
      "next_round_downstream": {
        "source_decision_status": "safe_to_wait_current_round_active",
        "effective_decision_status": "safe_to_wait_current_round_active",
        "service_cli_display_status": "display_safe_to_wait_current_round",
        "forge_operator_display_status": "forge_safe_to_wait",
        "agent_assignment_acceptance": "defer_until_current_round_completes",
        "memory_self_improve_admission_visibility": "visible_admission_waiting",
        "operator_attention_required": false,
        "read_only": true,
        "report_only": true,
        "no_side_effects": true,
        "dispatch_work_allowed": false,
        "prompt_replay_allowed": false,
        "process_start_allowed": false,
        "memory_write_allowed": false,
        "ndkv_write_allowed": false,
        "current_round_active": true,
        "live_status_display_state": "safe-to-wait",
        "active_round": 380,
        "ledger_latest_round": 379,
        "latest_done_round": 379,
        "readiness_can_schedule_next_round": false,
        "round_id_evidence": {
          "source_schema": "daemon_round_transition_status_v1",
          "active_round": 380,
          "ledger_latest_round": 379,
          "latest_done_round": 379,
          "transition_kind": "normal_in_progress",
          "transition_status_label": "round-in-progress",
          "ledger_commit_pending": false,
          "round_in_progress": true,
          "evidence_ids": ["daemon_transition:active-round-380", "ledger:latest-round-379"],
          "reason_codes": ["current_round_active", "safe_to_wait"]
        },
        "failure_reasons": []
      }
    }
  }
}"#,
            ),
        ] {
            let report =
                captured_current_status_next_round_decision_report_from_json(captured_json)
                    .unwrap_or_else(|| panic!("{label} post-R44 status should parse"));
            let service_status = SmartSteamStatusSnapshot::from_model_pool(
                norion_service::SmartSteamStatusSource::new()
                    .with_daemon(true, Some(209816), Some(380), Some(379))
                    .with_daemon_round_progress(Some(379), true)
                    .with_supervisor(true, true)
                    .with_readiness(true, true)
                    .with_next_round_decision_report(report),
                &gate,
                Some(&input.routing_intent()),
            );
            let before_history = session.history().len();
            let before_partial = session.partial_answer().to_owned();
            let before_chunks = session.chunks().len();

            let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
            let dto = status.smartsteam_status_host_snapshot(service_status);
            let decision = dto
                .next_round_decision_status
                .as_ref()
                .unwrap_or_else(|| panic!("{label} CLI status should surface decision"));
            let downstream = decision
                .downstream_status_consumers
                .as_ref()
                .unwrap_or_else(|| panic!("{label} CLI status should surface downstream"));
            let round_id_evidence = downstream
                .round_id_evidence
                .as_ref()
                .unwrap_or_else(|| panic!("{label} CLI status should surface round ids"));

            assert_eq!(session.history().len(), before_history, "{label}");
            assert_eq!(session.partial_answer(), before_partial, "{label}");
            assert_eq!(session.chunks().len(), before_chunks, "{label}");
            assert!(dto.read_only, "{label}");
            assert!(!dto.launches_process, "{label}");
            assert!(!dto.starts_daemon, "{label}");
            assert!(!dto.stops_daemon, "{label}");
            assert!(!dto.touches_remote, "{label}");
            assert!(!dto.downloads_model, "{label}");
            assert!(!dto.warms_model_cache, "{label}");
            assert!(!dto.sends_prompt, "{label}");
            assert!(!dto.starts_stream, "{label}");
            assert!(!dto.replays_prompt, "{label}");
            assert!(!dto.mutates_active_round, "{label}");
            assert!(!dto.carries_request_preview, "{label}");
            assert!(!dto.carries_stream_chunk, "{label}");
            assert!(!dto.carries_input_action_snapshot, "{label}");
            assert_eq!(decision.status_label, "safe-to-wait-current-round-active");
            assert!(decision.safe_to_wait_current_round_active);
            assert!(!decision.safe_to_continue_after_current_round);
            assert!(!decision.operator_attention_blocked);
            assert!(!decision.operator_action_required);
            assert_eq!(
                downstream.service_cli_display_status,
                "display_safe_to_wait_current_round"
            );
            assert_eq!(
                downstream.agent_assignment_acceptance,
                "defer_until_current_round_completes"
            );
            assert!(!downstream.operator_attention_required);
            assert!(downstream.read_only);
            assert!(downstream.report_only);
            assert!(downstream.no_side_effects);
            assert!(!downstream.dispatch_work_allowed);
            assert!(!downstream.prompt_replay_allowed);
            assert!(!downstream.process_start_allowed);
            assert!(!downstream.memory_write_allowed);
            assert!(!downstream.ndkv_write_allowed);
            assert_eq!(
                round_id_evidence.source_schema.as_deref(),
                Some("daemon_round_transition_status_v1")
            );
            assert_eq!(round_id_evidence.active_round, Some(380));
            assert_eq!(round_id_evidence.ledger_latest_round, Some(379));
            assert_eq!(round_id_evidence.latest_done_round, Some(379));
            assert_eq!(
                round_id_evidence.transition_kind.as_deref(),
                Some("normal_in_progress")
            );
        }
    }

    #[test]
    fn smartsteam_status_host_next_round_report_v1_field_names_are_stable() {
        let report = captured_current_status_next_round_decision_report_from_json(
            captured_current_status_next_round_decision_json_fixture(),
        )
        .expect("fixture should parse next-round report");
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(199264), Some(370), Some(369))
                .with_daemon_round_progress(Some(369), true)
                .with_supervisor(true, true)
                .with_readiness(true, true)
                .with_next_round_decision_report(report),
            &gate,
            Some(&input.routing_intent()),
        );

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);
        let decision = dto
            .next_round_decision_status
            .as_ref()
            .expect("CLI host should expose next-round decision");
        let downstream = decision
            .downstream_status_consumers
            .as_ref()
            .expect("CLI host should expose downstream consumer status");
        let round_id_evidence = downstream
            .round_id_evidence
            .as_ref()
            .expect("CLI host should expose downstream round-id evidence");

        assert_eq!(
            smartsteam_next_round_decision_json_fields(decision),
            vec![
                "report_version",
                "read_only",
                "report_only",
                "safe_to_wait_current_round_active",
                "safe_to_continue_after_current_round",
                "operator_attention_blocked",
                "current_round",
                "latest_done_round",
                "status_label",
                "evidence_ids",
                "reason_codes",
                "starts_daemon",
                "stops_daemon",
                "touches_remote",
                "sends_prompt",
                "starts_stream",
                "replays_prompt",
                "mutates_active_round",
                "mutates_worker_window_status",
                "writes_ndkv",
                "creates_thread",
                "operator_action_required",
                "downstream_status_consumers",
            ]
        );
        assert_eq!(
            smartsteam_next_round_downstream_consumer_json_fields(downstream),
            vec![
                "schema_version",
                "source_decision_status",
                "effective_decision_status",
                "service_cli_display_status",
                "forge_operator_display_status",
                "agent_assignment_acceptance",
                "memory_self_improve_admission_visibility",
                "operator_attention_required",
                "read_only",
                "report_only",
                "no_side_effects",
                "dispatch_work_allowed",
                "prompt_replay_allowed",
                "process_start_allowed",
                "memory_write_allowed",
                "ndkv_write_allowed",
                "current_round_active",
                "live_status_display_state",
                "active_round",
                "ledger_latest_round",
                "latest_done_round",
                "readiness_can_schedule_next_round",
                "round_id_evidence",
                "failure_reasons",
            ]
        );
        assert_eq!(
            smartsteam_next_round_round_id_evidence_json_fields(round_id_evidence),
            vec![
                "source_schema",
                "active_round",
                "ledger_latest_round",
                "latest_done_round",
                "transition_kind",
                "transition_status_label",
                "ledger_commit_pending",
                "round_in_progress",
                "evidence_ids",
                "reason_codes",
            ]
        );
        assert!(dto.read_only);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!decision.starts_daemon);
        assert!(!decision.sends_prompt);
        assert!(!decision.starts_stream);
        assert!(!decision.writes_ndkv);
        assert!(!decision.creates_thread);
        assert!(downstream.no_side_effects);
        assert!(!downstream.dispatch_work_allowed);
        assert!(!downstream.prompt_replay_allowed);
        assert!(!downstream.process_start_allowed);
        assert!(!downstream.memory_write_allowed);
        assert!(!downstream.ndkv_write_allowed);
    }

    #[test]
    fn smartsteam_status_host_snapshot_maps_captured_daemon_json_fixture_read_only() {
        let (captured_json, captured) = captured_daemon_json_status_fixture();
        assert_captured_daemon_json_status_shape(captured_json);
        assert!(captured.daemon_round_transition_status_v1.read_only);
        assert!(!captured.daemon_round_transition_status_v1.starts_process);
        assert!(!captured.daemon_round_transition_status_v1.sends_prompt);

        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("round=337 generate:start".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Worker("127.0.0.1:8686".to_owned()))
                    .with_roles([ModelRole::Reviewer])
                    .with_busy(true, Some("round=337 generate:start".to_owned())),
            ],
        );
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            service_source_from_captured_daemon_json_status(captured),
            &gate,
            Some(&input.routing_intent()),
        );
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();
        let before_chunks = session.chunks().len();

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);
        let transition = dto
            .daemon_round_transition_status
            .as_ref()
            .expect("CLI host should copy daemon round transition status");

        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert_eq!(session.chunks().len(), before_chunks);
        assert!(dto.read_only);
        assert!(!dto.launches_process);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.downloads_model);
        assert!(!dto.warms_model_cache);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_active_round);
        assert!(!dto.starts_clean_room_replacement);
        assert!(!dto.mutates_worker_window_status);
        assert!(!dto.carries_request_preview);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert_eq!(dto.history_messages, before_history);
        assert_eq!(dto.partial_chars, 0);
        assert_eq!(dto.service_status.latest_done_round, Some(336));
        assert!(dto.service_status.round_in_progress);
        assert_eq!(dto.service_status.active_round, Some(337));
        assert_eq!(dto.service_status.ledger_round, Some(336));
        assert!(transition.read_only);
        assert!(transition.report_only);
        assert_eq!(transition.status_label, "observing");
        assert_eq!(transition.latest_done_round, Some(336));
        assert!(transition.round_in_progress);
        assert_eq!(transition.ledger_round, Some(336));
        assert_eq!(
            transition.reason_codes,
            vec!["normal_in_progress", "active_round_after_latest_done"]
        );
        assert!(!transition.starts_daemon);
        assert!(!transition.sends_prompt);
        assert!(!transition.starts_stream);
        assert!(!transition.writes_ndkv);
        assert!(
            dto.context_hygiene_status
                .completed_window_evidence_non_actionable
        );
        assert!(!dto.context_hygiene_status.reads_old_window_payload);
        assert_eq!(
            dto.context_hygiene_status,
            dto.service_status.context_hygiene_status
        );
        assert_eq!(
            dto.daemon_round_transition_status,
            dto.service_status.daemon_round_transition_status
        );
        assert_eq!(
            dto.daemon_round_transition_summary,
            dto.service_status.daemon_round_transition_summary
        );
    }

    #[test]
    fn smartsteam_status_host_snapshot_field_bundle_keeps_cli_boundary_read_only() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(224392), Some(314), Some(314))
                .with_supervisor(true, true)
                .with_readiness(true, true)
                .with_model_cache_label("5/5 external diagnostic"),
            &gate,
            None,
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);

        assert_eq!(
            [
                dto.read_only,
                dto.launches_process,
                dto.starts_daemon,
                dto.stops_daemon,
                dto.touches_remote,
                dto.downloads_model,
                dto.warms_model_cache,
                dto.sends_prompt,
                dto.starts_stream,
                dto.replays_prompt,
                dto.mutates_busy,
                dto.mutates_readiness,
                dto.mutates_active_round,
                dto.starts_clean_room_replacement,
                dto.mutates_worker_window_status,
                dto.carries_request_preview,
                dto.carries_stream_chunk,
                dto.carries_input_action_snapshot,
            ],
            [
                true, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false, false, false,
            ]
        );
        assert_eq!(
            cli_smartsteam_status_host_json_fields(&dto),
            vec![
                "read_only",
                "launches_process",
                "starts_daemon",
                "stops_daemon",
                "touches_remote",
                "downloads_model",
                "warms_model_cache",
                "sends_prompt",
                "starts_stream",
                "replays_prompt",
                "mutates_busy",
                "mutates_readiness",
                "mutates_active_round",
                "starts_clean_room_replacement",
                "mutates_worker_window_status",
                "carries_request_preview",
                "carries_stream_chunk",
                "carries_input_action_snapshot",
                "history_messages",
                "partial_chars",
                "clean_room_replacement_required",
                "worker_window_status",
                "worker_windows",
                "context_hygiene_status",
                "context_hygiene_summary",
                "memory_startup_admission_status",
                "memory_startup_admission_summary",
                "clean_room_handoff_status",
                "clean_room_handoff_summary",
                "helper_stage_repair_status",
                "helper_stage_repair_summary",
                "self_improve_proposal_status",
                "self_improve_proposal_summary",
                "daemon_round_transition_status",
                "daemon_round_transition_summary",
                "next_round_decision_status",
                "next_round_decision_summary",
                "service_status",
            ]
        );
        assert_eq!(dto.history_messages, 0);
        assert_eq!(dto.partial_chars, 0);
        assert_eq!(
            dto.service_status.status_line(),
            "daemon_running=true active_round=314 latest_done_round=314 round_in_progress=false ledger_round=314 readiness_ok=true engine_busy=false remote_chain_ready=true pool=workers total=1 available=1 busy=0 saturated=0"
        );
    }

    #[test]
    fn smartsteam_status_host_snapshot_exposes_clean_room_window_replacement_without_replay() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("active prompt")
            .expect("setup should create a local active stream");
        session.push_delta("partial answer");
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(endpoint(8686))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(endpoint(8687))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(224392), Some(315), Some(315))
                .with_supervisor(true, false)
                .with_readiness(true, true)
                .with_worker_window(
                    norion_service::SmartSteamWorkerWindowStatusSource::new(
                        "019ee199-ae19-7660-8a5a-ff672c3080e0",
                        "service-cli-status",
                    )
                    .with_polluted("old context saw previous worker instructions")
                    .with_replacement_window("019f0000-clean-room-service-cli"),
                )
                .with_worker_window(
                    norion_service::SmartSteamWorkerWindowStatusSource::new(
                        "019ee201-855f-7591-aeda-84f17e171d92",
                        "service-cli-forge-r29",
                    )
                    .with_completed_evidence_only("completed evidence only; open a fresh window"),
                ),
            &gate,
            Some(&input.routing_intent()),
        );
        let before_state = session.state();
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();
        let before_chunks = session.chunks().len();

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);

        assert_eq!(session.state(), before_state);
        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert_eq!(session.chunks().len(), before_chunks);
        assert!(dto.read_only);
        assert!(!dto.starts_clean_room_replacement);
        assert!(!dto.mutates_worker_window_status);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.carries_request_preview);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert_eq!(dto.history_messages, before_history);
        assert_eq!(dto.partial_chars, before_partial.chars().count());
        assert!(dto.service_status.daemon_running);
        assert!(dto.service_status.supervisor_running);
        assert!(dto.service_status.remote_chain_ready);
        assert_eq!(dto.service_status.workers_total, 2);
        assert_eq!(dto.service_status.workers_available, 2);
        assert!(dto.clean_room_replacement_required);
        assert_eq!(
            dto.worker_window_status,
            "windows total=2 running=0 paused=0 polluted=1 clean_room_replacements_required=2"
        );
        assert_eq!(dto.worker_windows.len(), 2);
        assert_eq!(dto.worker_windows[0].status_label, "polluted");
        assert!(!dto.worker_windows[0].assignment_allowed);
        assert!(dto.worker_windows[0].original_window_blocks_assignment);
        assert!(dto.worker_windows[0].future_work_requires_fresh_clean_room);
        assert_eq!(
            dto.worker_windows[0].replacement_window_id.as_deref(),
            Some("019f0000-clean-room-service-cli")
        );
        assert_eq!(
            dto.worker_windows[1].status_label,
            "completed-evidence-only"
        );
        assert!(dto.worker_windows[1].completed_evidence_only);
        assert!(!dto.worker_windows[1].assignment_allowed);
        assert!(dto.worker_windows[1].future_work_requires_fresh_clean_room);
        assert!(
            dto.context_hygiene_status
                .completed_window_evidence_non_actionable
        );
        assert!(
            dto.context_hygiene_status
                .future_work_requires_fresh_clean_room
        );
        assert!(!dto.context_hygiene_status.reads_old_window_payload);
        assert_eq!(
            dto.context_hygiene_status,
            dto.service_status.context_hygiene_status
        );
        assert_eq!(dto.service_status.worker_windows, dto.worker_windows);
    }

    #[test]
    fn smartsteam_status_host_snapshot_surfaces_memory_admission_without_replay() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("active prompt")
            .expect("setup should create a local active stream");
        session.push_delta("partial answer");
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let evidence = norion_service::MemoryStartupAdmissionEvidence {
            read_only_review_required: false,
            index_quality_blocker_count: 0,
            index_quality_warning_count: 1,
            index_operation_count: 2,
            index_refresh_count: 1,
            index_detail_codes: vec!["quality_warning".to_owned()],
            context_rot_risk_count: 1,
            context_rot_blocker_reason_codes: vec!["stale_window".to_owned()],
            admission_decision_count: 3,
            admission_accepted_count: 2,
            admission_risk_rejection_count: 1,
            migration_live_store_targeted_count: 0,
            adapter_live_write_count: 0,
            live_write_phase_request_count: 0,
            store_mutation_count: 0,
            helper_prose_line_count: 1,
            non_contract_line_count: 2,
        };
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(224392), Some(317), Some(317))
                .with_supervisor(true, true)
                .with_readiness(true, true)
                .with_memory_startup_admission(evidence),
            &gate,
            None,
        );
        let before_state = session.state();
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();
        let before_chunks = session.chunks().len();

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);

        assert_eq!(session.state(), before_state);
        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert_eq!(session.chunks().len(), before_chunks);
        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.carries_request_preview);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert_eq!(dto.history_messages, before_history);
        assert_eq!(dto.partial_chars, before_partial.chars().count());
        let memory = dto
            .memory_startup_admission_status
            .as_ref()
            .expect("CLI host should expose memory admission status");
        assert!(memory.read_only_contract);
        assert_eq!(memory.index_quality_warning_count, 1);
        assert_eq!(memory.index_operation_count, 2);
        assert_eq!(memory.context_rot_risk_count, 1);
        assert_eq!(memory.admission_decision_count, 3);
        assert_eq!(memory.admission_accepted_count, 2);
        assert_eq!(memory.helper_prose_line_count, 1);
        assert_eq!(memory.non_contract_line_count, 2);
        assert!(!memory.live_store_mutation_requested);
        assert_eq!(memory.store_mutation_count, 0);
        assert!(!memory.ndkv_write_allowed);
        assert!(!memory.admission_expanded_by_non_contract_evidence);
        assert_eq!(
            dto.memory_startup_admission_summary,
            dto.service_status.memory_startup_admission_summary
        );
    }

    #[test]
    fn smartsteam_status_host_snapshot_surfaces_clean_room_handoff_without_replay() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("active prompt")
            .expect("setup should create a local active stream");
        session.push_delta("partial answer");
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let evidence = norion_service::MemoryStartupAdmissionEvidence {
            read_only_review_required: false,
            index_quality_blocker_count: 0,
            index_quality_warning_count: 1,
            index_operation_count: 2,
            index_refresh_count: 1,
            index_detail_codes: vec!["quality_warning".to_owned()],
            context_rot_risk_count: 0,
            context_rot_blocker_reason_codes: Vec::new(),
            admission_decision_count: 3,
            admission_accepted_count: 3,
            admission_risk_rejection_count: 0,
            migration_live_store_targeted_count: 0,
            adapter_live_write_count: 0,
            live_write_phase_request_count: 0,
            store_mutation_count: 0,
            helper_prose_line_count: 4,
            non_contract_line_count: 2,
        };
        let handoff = norion_service::SmartSteamCleanRoomHandoffStatusSource::new()
            .with_agent_replacement_plan(true, true, true)
            .with_original_window_follow_up_assignment_allowed(false)
            .with_old_window_payload_read(false)
            .with_thread_side_effects(false, false)
            .with_evidence_result_ids(["handoff-summary:r24-agent"])
            .with_reason_codes(["window_context_polluted", "paused_by_main_window"]);
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(230076), Some(322), Some(321))
                .with_supervisor(true, true)
                .with_readiness(true, true)
                .with_memory_startup_admission(evidence)
                .with_worker_window(
                    norion_service::SmartSteamWorkerWindowStatusSource::new(
                        "019ee1c4-3b94-7cb0-a870-b1cb0e7b11e4",
                        "agent-clean-room-assignment",
                    )
                    .with_polluted("old window pollution blocks follow-up assignment"),
                )
                .with_clean_room_handoff(handoff),
            &gate,
            None,
        );
        let before_state = session.state();
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();
        let before_chunks = session.chunks().len();

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);
        let handoff = dto
            .clean_room_handoff_status
            .as_ref()
            .expect("CLI host should expose clean-room handoff status");

        assert_eq!(session.state(), before_state);
        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert_eq!(session.chunks().len(), before_chunks);
        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.carries_request_preview);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert_eq!(dto.history_messages, before_history);
        assert_eq!(dto.partial_chars, before_partial.chars().count());
        assert!(handoff.memory_admission_safe);
        assert!(handoff.agent_replacement_plan_required);
        assert!(handoff.agent_replacement_plan_available);
        assert!(handoff.replacement_prompt_ready);
        assert!(handoff.original_window_follow_up_blocked);
        assert!(!handoff.original_window_follow_up_assignment_allowed);
        assert!(!handoff.reads_old_window_payload);
        assert!(handoff.old_window_payload_ignored);
        assert!(!handoff.live_write_allowed);
        assert!(!handoff.live_store_mutation_allowed);
        assert!(!handoff.ndkv_write_allowed);
        assert!(!handoff.runtime_side_effects_allowed);
        assert_eq!(
            dto.clean_room_handoff_summary,
            dto.service_status.clean_room_handoff_summary
        );
    }

    #[test]
    fn smartsteam_status_host_snapshot_surfaces_helper_stage_repair_without_replay() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("active prompt")
            .expect("setup should create a local active stream");
        session.push_delta("partial answer");
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(230076), Some(324), Some(323))
                .with_supervisor(true, true)
                .with_readiness(true, true)
                .with_helper_stage_repair(
                    norion_service::SmartSteamHelperStageRepairStatusSource::new(
                        "review",
                        norion_service::SmartSteamHelperStageRepairState::RepairRequired,
                    )
                    .with_source_round(324)
                    .with_evidence_ids(["round-324:helper-stage:review"])
                    .with_reason_codes([
                        "helper_stage_contract_incomplete",
                        "missing_required_contract_fields",
                    ])
                    .with_missing_helper_role_repair_proposals([
                        norion_service::SmartSteamMissingHelperRoleRepairProposalStatusSource::new(
                            "helper-stage-repair-r324-router",
                            "router",
                        )
                        .with_reason_codes(["required_helper_role_missing"]),
                    ]),
                ),
            &gate,
            None,
        );
        let before_state = session.state();
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();
        let before_chunks = session.chunks().len();

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);
        let repair = dto
            .helper_stage_repair_status
            .as_ref()
            .expect("CLI host should expose helper-stage repair status");

        assert_eq!(session.state(), before_state);
        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert_eq!(session.chunks().len(), before_chunks);
        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.carries_request_preview);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert_eq!(dto.history_messages, before_history);
        assert_eq!(dto.partial_chars, before_partial.chars().count());
        assert!(repair.read_only);
        assert!(repair.report_only);
        assert!(repair.pure_data_only);
        assert_eq!(repair.stage_label, "review");
        assert_eq!(repair.state_label, "repair-required");
        assert!(!repair.helper_stage_contract_complete);
        assert!(repair.helper_stage_repair_required);
        assert_eq!(repair.source_round, Some(324));
        assert_eq!(repair.evidence_ids, vec!["round-324:helper-stage:review"]);
        assert!(repair.missing_helper_role_repair_required);
        assert_eq!(repair.missing_helper_role_repair_proposal_count, 1);
        assert_eq!(repair.missing_helper_roles, vec!["router"]);
        let missing_role_proposal = repair
            .missing_helper_role_repair_proposals
            .first()
            .expect("CLI host should preserve missing helper-role proposal");
        assert_eq!(
            missing_role_proposal.proposal_id,
            "helper-stage-repair-r324-router"
        );
        assert_eq!(missing_role_proposal.role_label, "router");
        assert!(missing_role_proposal.repair_required);
        assert!(!missing_role_proposal.replays_prompt);
        assert!(!missing_role_proposal.calls_model);
        assert!(!missing_role_proposal.sends_prompt);
        assert!(!missing_role_proposal.starts_stream);
        assert!(!missing_role_proposal.writes_memory);
        assert!(!missing_role_proposal.writes_ndkv);
        assert!(!missing_role_proposal.runtime_side_effects_allowed);
        assert!(!repair.parses_helper_prose);
        assert!(!repair.replays_prompt);
        assert!(!repair.calls_model);
        assert!(!repair.sends_prompt);
        assert!(!repair.starts_stream);
        assert!(!repair.writes_memory);
        assert!(!repair.writes_ndkv);
        assert!(!repair.mutates_live_store);
        assert!(!repair.starts_clean_room_replacement);
        assert!(!repair.mutates_worker_window_status);
        assert!(!repair.runtime_side_effects_allowed);
        assert_eq!(
            dto.helper_stage_repair_summary,
            dto.service_status.helper_stage_repair_summary
        );
    }

    #[test]
    fn smartsteam_status_host_snapshot_surfaces_self_improve_proposals_without_replay() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("active prompt")
            .expect("setup should create a local active stream");
        session.push_delta("partial answer");
        let input = CliInputConfig::default();
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let proposal = |id: &str,
                        lifecycle: norion_service::SmartSteamSelfImproveProposalLifecycle,
                        round: u64,
                        validation_checked: bool,
                        validation_passed: bool,
                        memory_checked: bool,
                        memory_admitted: bool,
                        memory_quarantined: bool| {
            norion_service::SmartSteamSelfImproveProposalStatusSource::new(id, lifecycle)
                .with_source_round(round)
                .with_evidence_ids([format!("round-{round}:proposal:{id}")])
                .with_validation_status(
                    norion_service::SmartSteamSelfImproveProposalValidationStatusSource::new(
                        validation_checked,
                        validation_passed,
                    )
                    .with_status_code(if validation_passed { 0 } else { 1 })
                    .with_evidence_ids([format!("round-{round}:validation:{id}")]),
                )
                .with_memory_admission_status(
                    norion_service::SmartSteamSelfImproveProposalMemoryAdmissionStatusSource::new(
                        memory_checked,
                        memory_admitted,
                        memory_quarantined,
                    )
                    .with_evidence_ids([format!("round-{round}:memory:{id}")]),
                )
        };
        let service_status = SmartSteamStatusSnapshot::from_model_pool(
            norion_service::SmartSteamStatusSource::new()
                .with_daemon(true, Some(230076), Some(324), Some(323))
                .with_supervisor(true, true)
                .with_readiness(true, true)
                .with_self_improve_proposals([
                    proposal(
                        "candidate-001",
                        norion_service::SmartSteamSelfImproveProposalLifecycle::Candidate,
                        324,
                        false,
                        false,
                        false,
                        false,
                        false,
                    ),
                    proposal(
                        "validated-001",
                        norion_service::SmartSteamSelfImproveProposalLifecycle::Validated,
                        323,
                        true,
                        true,
                        false,
                        false,
                        false,
                    ),
                    proposal(
                        "admitted-001",
                        norion_service::SmartSteamSelfImproveProposalLifecycle::Admitted,
                        322,
                        true,
                        true,
                        true,
                        true,
                        false,
                    ),
                    proposal(
                        "quarantined-001",
                        norion_service::SmartSteamSelfImproveProposalLifecycle::Quarantined,
                        321,
                        true,
                        false,
                        true,
                        false,
                        true,
                    ),
                    proposal(
                        "promoted-001",
                        norion_service::SmartSteamSelfImproveProposalLifecycle::Promoted,
                        320,
                        true,
                        true,
                        true,
                        true,
                        false,
                    ),
                    proposal(
                        "repair-001",
                        norion_service::SmartSteamSelfImproveProposalLifecycle::RepairRequired,
                        319,
                        true,
                        false,
                        true,
                        false,
                        false,
                    ),
                ])
                .with_self_improve_proposal_prompt_guidance(
                    norion_service::SmartSteamSelfImproveProposalPromptGuidanceSource::new(
                        true, false, true,
                    )
                    .with_evidence_ids(["round-324:self-improve-proposal-guidance"]),
                ),
            &gate,
            None,
        );
        let before_state = session.state();
        let before_history = session.history().len();
        let before_partial = session.partial_answer().to_owned();
        let before_chunks = session.chunks().len();

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status.smartsteam_status_host_snapshot(service_status);
        let proposals = dto
            .self_improve_proposal_status
            .as_ref()
            .expect("CLI host should expose self-improve proposal status");

        assert_eq!(session.state(), before_state);
        assert_eq!(session.history().len(), before_history);
        assert_eq!(session.partial_answer(), before_partial);
        assert_eq!(session.chunks().len(), before_chunks);
        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.carries_request_preview);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert_eq!(dto.history_messages, before_history);
        assert_eq!(dto.partial_chars, before_partial.chars().count());
        assert_eq!(proposals.proposal_count, 6);
        assert_eq!(proposals.candidate_count, 1);
        assert_eq!(proposals.validated_count, 1);
        assert_eq!(proposals.admitted_count, 1);
        assert_eq!(proposals.quarantined_count, 1);
        assert_eq!(proposals.promoted_count, 1);
        assert_eq!(proposals.repair_required_count, 1);
        assert!(!proposals.parses_helper_prose);
        assert!(!proposals.replays_prompt);
        assert!(!proposals.calls_model);
        assert!(!proposals.sends_prompt);
        assert!(!proposals.starts_stream);
        assert!(!proposals.writes_memory);
        assert!(!proposals.writes_ndkv);
        assert!(!proposals.mutates_live_store);
        assert!(!proposals.runtime_side_effects_allowed);
        let guidance = proposals
            .prompt_guidance
            .as_ref()
            .expect("CLI host should expose self-improve proposal guidance");
        assert!(guidance.convert_advisory_to_business_evidence);
        assert!(!guidance.repair_unvalidated_or_unaccepted);
        assert!(guidance.requires_validation_and_memory_admission);
        assert_eq!(
            guidance.evidence_ids,
            vec!["round-324:self-improve-proposal-guidance"]
        );
        assert!(!guidance.parses_helper_prose);
        assert!(!guidance.replays_prompt);
        assert!(!guidance.calls_model);
        assert!(!guidance.sends_prompt);
        assert!(!guidance.starts_stream);
        assert!(!guidance.writes_memory);
        assert!(!guidance.writes_ndkv);
        assert!(!guidance.mutates_live_store);
        assert!(!guidance.runtime_side_effects_allowed);
        assert_eq!(
            proposals
                .proposals
                .iter()
                .map(|proposal| proposal.lifecycle_label.as_str())
                .collect::<Vec<_>>(),
            vec![
                "candidate",
                "validated",
                "admitted",
                "quarantined",
                "promoted",
                "repair-required"
            ]
        );
        assert_eq!(proposals.proposals[0].source_round, Some(324));
        assert_eq!(
            proposals.proposals[1].validation_status.status_label,
            "passed"
        );
        assert_eq!(
            proposals.proposals[2].memory_admission_status.status_label,
            "admitted"
        );
        assert_eq!(
            proposals.proposals[3].memory_admission_status.evidence_ids,
            vec!["round-321:memory:quarantined-001"]
        );
        assert!(
            proposals
                .proposals
                .iter()
                .all(|proposal| !proposal.parses_helper_prose
                    && !proposal.replays_prompt
                    && !proposal.calls_model
                    && !proposal.sends_prompt
                    && !proposal.starts_stream
                    && !proposal.writes_memory
                    && !proposal.writes_ndkv
                    && !proposal.mutates_live_store
                    && !proposal.promotes_runtime
                    && !proposal.quarantines_runtime
                    && !proposal.runtime_side_effects_allowed)
        );
        assert_eq!(
            dto.self_improve_proposal_summary,
            dto.service_status.self_improve_proposal_summary
        );
    }

    #[test]
    fn status_consumer_field_bundle_covers_8686_8690_worker_states_and_read_only_flags() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(endpoint(8686))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(endpoint(8687))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8687 active review".to_owned())),
                ModelWorkerSnapshot::new(endpoint(8688))
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(endpoint(8689))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(1, 1),
                ModelWorkerSnapshot::new(endpoint(8690))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status
            .workers_host_snapshot()
            .expect("model-pool status should expose a read-only workers host DTO");

        assert_eq!(
            (
                dto.read_only,
                dto.launches_process,
                dto.sends_prompt,
                dto.starts_stream,
                dto.carries_request_preview,
                dto.carries_stream_chunk,
                dto.carries_input_action_snapshot,
            ),
            (true, false, false, false, false, false, false)
        );
        assert_eq!(
            dto.pool_status.as_deref(),
            Some("workers total=5 available=3 busy=1 saturated=1")
        );
        assert_eq!(
            dto.route_pool_status.as_deref(),
            Some("matching total=4 available=2 busy=1 saturated=1")
        );
        assert_eq!(
            dto.workers
                .iter()
                .map(|worker| (
                    worker.endpoint_label.as_str(),
                    worker.worker_status_label.as_str(),
                    worker.worker_status_state_label.as_str(),
                    worker.worker_status_is_available,
                    worker.worker_status_is_pressure,
                    worker.worker_status_blocks_prompt_submit,
                    worker.route_match,
                    worker.selectable,
                    worker.picker_action_label.as_str(),
                    worker.decision_action_label.as_str(),
                    worker.decision_state_label.as_str(),
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "127.0.0.1:8686",
                    "available",
                    "pending",
                    true,
                    false,
                    false,
                    true,
                    true,
                    "select",
                    "send_now",
                    "pending",
                ),
                (
                    "127.0.0.1:8687",
                    "busy",
                    "busy",
                    false,
                    true,
                    true,
                    true,
                    false,
                    "wait",
                    "wait_for_current_stream",
                    "busy",
                ),
                (
                    "127.0.0.1:8688",
                    "available",
                    "pending",
                    true,
                    false,
                    false,
                    false,
                    false,
                    "unavailable",
                    "wait_for_worker",
                    "queued",
                ),
                (
                    "127.0.0.1:8689",
                    "backpressure",
                    "backpressure",
                    false,
                    true,
                    true,
                    true,
                    false,
                    "wait",
                    "retry_later",
                    "backpressure",
                ),
                (
                    "127.0.0.1:8690",
                    "available",
                    "pending",
                    true,
                    false,
                    false,
                    true,
                    true,
                    "select",
                    "send_now",
                    "pending",
                ),
            ]
        );
    }

    #[test]
    fn workers_host_snapshot_keeps_external_model_cache_diagnostics_out_of_cli_dto() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(endpoint(8686))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(endpoint(8687))
                    .with_roles([ModelRole::Summarizer])
                    .with_preferences([RoutingPreference::Balanced]),
            ],
        );
        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);
        let dto = status
            .workers_host_snapshot()
            .expect("model-pool status should expose a read-only workers host DTO");

        assert!(dto.read_only);
        assert!(!dto.launches_process);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert_eq!(
            dto.pool_status.as_deref(),
            Some("workers total=2 available=2 busy=0 saturated=0")
        );

        let host_fields = cli_workers_host_json_fields(&dto);
        let worker_fields = cli_worker_host_json_fields(&dto.workers[0]);
        for external_field in [
            "model_cache",
            "model_cache_label",
            "model_cache_all_ok",
            "cache_warmup",
            "downloads_model",
            "writes_model_weights",
            "quality_port",
            "helper_ports",
            "readiness_ok",
        ] {
            assert!(
                !host_fields.contains(&external_field),
                "{external_field} must remain an external diagnostic, not a CLI host DTO field"
            );
            assert!(
                !worker_fields.contains(&external_field),
                "{external_field} must remain an external diagnostic, not a CLI worker DTO field"
            );
        }
    }

    fn cli_smartsteam_status_host_json_fields(
        dto: &CliSmartSteamStatusHostSnapshot,
    ) -> Vec<&'static str> {
        let CliSmartSteamStatusHostSnapshot {
            read_only: _read_only,
            launches_process: _launches_process,
            starts_daemon: _starts_daemon,
            stops_daemon: _stops_daemon,
            touches_remote: _touches_remote,
            downloads_model: _downloads_model,
            warms_model_cache: _warms_model_cache,
            sends_prompt: _sends_prompt,
            starts_stream: _starts_stream,
            replays_prompt: _replays_prompt,
            mutates_busy: _mutates_busy,
            mutates_readiness: _mutates_readiness,
            mutates_active_round: _mutates_active_round,
            starts_clean_room_replacement: _starts_clean_room_replacement,
            mutates_worker_window_status: _mutates_worker_window_status,
            carries_request_preview: _carries_request_preview,
            carries_stream_chunk: _carries_stream_chunk,
            carries_input_action_snapshot: _carries_input_action_snapshot,
            history_messages: _history_messages,
            partial_chars: _partial_chars,
            clean_room_replacement_required: _clean_room_replacement_required,
            worker_window_status: _worker_window_status,
            worker_windows: _worker_windows,
            context_hygiene_status: _context_hygiene_status,
            context_hygiene_summary: _context_hygiene_summary,
            memory_startup_admission_status: _memory_startup_admission_status,
            memory_startup_admission_summary: _memory_startup_admission_summary,
            clean_room_handoff_status: _clean_room_handoff_status,
            clean_room_handoff_summary: _clean_room_handoff_summary,
            helper_stage_repair_status: _helper_stage_repair_status,
            helper_stage_repair_summary: _helper_stage_repair_summary,
            self_improve_proposal_status: _self_improve_proposal_status,
            self_improve_proposal_summary: _self_improve_proposal_summary,
            daemon_round_transition_status: _daemon_round_transition_status,
            daemon_round_transition_summary: _daemon_round_transition_summary,
            next_round_decision_status: _next_round_decision_status,
            next_round_decision_summary: _next_round_decision_summary,
            service_status: _service_status,
        } = dto;
        vec![
            "read_only",
            "launches_process",
            "starts_daemon",
            "stops_daemon",
            "touches_remote",
            "downloads_model",
            "warms_model_cache",
            "sends_prompt",
            "starts_stream",
            "replays_prompt",
            "mutates_busy",
            "mutates_readiness",
            "mutates_active_round",
            "starts_clean_room_replacement",
            "mutates_worker_window_status",
            "carries_request_preview",
            "carries_stream_chunk",
            "carries_input_action_snapshot",
            "history_messages",
            "partial_chars",
            "clean_room_replacement_required",
            "worker_window_status",
            "worker_windows",
            "context_hygiene_status",
            "context_hygiene_summary",
            "memory_startup_admission_status",
            "memory_startup_admission_summary",
            "clean_room_handoff_status",
            "clean_room_handoff_summary",
            "helper_stage_repair_status",
            "helper_stage_repair_summary",
            "self_improve_proposal_status",
            "self_improve_proposal_summary",
            "daemon_round_transition_status",
            "daemon_round_transition_summary",
            "next_round_decision_status",
            "next_round_decision_summary",
            "service_status",
        ]
    }

    fn smartsteam_next_round_decision_json_fields(
        dto: &SmartSteamNextRoundDecisionStatusSnapshot,
    ) -> Vec<&'static str> {
        let SmartSteamNextRoundDecisionStatusSnapshot {
            report_version: _report_version,
            read_only: _read_only,
            report_only: _report_only,
            safe_to_wait_current_round_active: _safe_to_wait_current_round_active,
            safe_to_continue_after_current_round: _safe_to_continue_after_current_round,
            operator_attention_blocked: _operator_attention_blocked,
            current_round: _current_round,
            latest_done_round: _latest_done_round,
            status_label: _status_label,
            evidence_ids: _evidence_ids,
            reason_codes: _reason_codes,
            starts_daemon: _starts_daemon,
            stops_daemon: _stops_daemon,
            touches_remote: _touches_remote,
            sends_prompt: _sends_prompt,
            starts_stream: _starts_stream,
            replays_prompt: _replays_prompt,
            mutates_active_round: _mutates_active_round,
            mutates_worker_window_status: _mutates_worker_window_status,
            writes_ndkv: _writes_ndkv,
            creates_thread: _creates_thread,
            operator_action_required: _operator_action_required,
            downstream_status_consumers: _downstream_status_consumers,
        } = dto;
        vec![
            "report_version",
            "read_only",
            "report_only",
            "safe_to_wait_current_round_active",
            "safe_to_continue_after_current_round",
            "operator_attention_blocked",
            "current_round",
            "latest_done_round",
            "status_label",
            "evidence_ids",
            "reason_codes",
            "starts_daemon",
            "stops_daemon",
            "touches_remote",
            "sends_prompt",
            "starts_stream",
            "replays_prompt",
            "mutates_active_round",
            "mutates_worker_window_status",
            "writes_ndkv",
            "creates_thread",
            "operator_action_required",
            "downstream_status_consumers",
        ]
    }

    fn smartsteam_next_round_downstream_consumer_json_fields(
        dto: &norion_service::SmartSteamNextRoundDownstreamConsumerStatusSnapshot,
    ) -> Vec<&'static str> {
        let norion_service::SmartSteamNextRoundDownstreamConsumerStatusSnapshot {
            schema_version: _schema_version,
            source_decision_status: _source_decision_status,
            effective_decision_status: _effective_decision_status,
            service_cli_display_status: _service_cli_display_status,
            forge_operator_display_status: _forge_operator_display_status,
            agent_assignment_acceptance: _agent_assignment_acceptance,
            memory_self_improve_admission_visibility: _memory_self_improve_admission_visibility,
            operator_attention_required: _operator_attention_required,
            read_only: _read_only,
            report_only: _report_only,
            no_side_effects: _no_side_effects,
            dispatch_work_allowed: _dispatch_work_allowed,
            prompt_replay_allowed: _prompt_replay_allowed,
            process_start_allowed: _process_start_allowed,
            memory_write_allowed: _memory_write_allowed,
            ndkv_write_allowed: _ndkv_write_allowed,
            current_round_active: _current_round_active,
            live_status_display_state: _live_status_display_state,
            active_round: _active_round,
            ledger_latest_round: _ledger_latest_round,
            latest_done_round: _latest_done_round,
            readiness_can_schedule_next_round: _readiness_can_schedule_next_round,
            round_id_evidence: _round_id_evidence,
            failure_reasons: _failure_reasons,
        } = dto;
        vec![
            "schema_version",
            "source_decision_status",
            "effective_decision_status",
            "service_cli_display_status",
            "forge_operator_display_status",
            "agent_assignment_acceptance",
            "memory_self_improve_admission_visibility",
            "operator_attention_required",
            "read_only",
            "report_only",
            "no_side_effects",
            "dispatch_work_allowed",
            "prompt_replay_allowed",
            "process_start_allowed",
            "memory_write_allowed",
            "ndkv_write_allowed",
            "current_round_active",
            "live_status_display_state",
            "active_round",
            "ledger_latest_round",
            "latest_done_round",
            "readiness_can_schedule_next_round",
            "round_id_evidence",
            "failure_reasons",
        ]
    }

    fn smartsteam_next_round_round_id_evidence_json_fields(
        dto: &norion_service::SmartSteamNextRoundRoundIdEvidenceSnapshot,
    ) -> Vec<&'static str> {
        let norion_service::SmartSteamNextRoundRoundIdEvidenceSnapshot {
            source_schema: _source_schema,
            active_round: _active_round,
            ledger_latest_round: _ledger_latest_round,
            latest_done_round: _latest_done_round,
            transition_kind: _transition_kind,
            transition_status_label: _transition_status_label,
            ledger_commit_pending: _ledger_commit_pending,
            round_in_progress: _round_in_progress,
            evidence_ids: _evidence_ids,
            reason_codes: _reason_codes,
        } = dto;
        vec![
            "source_schema",
            "active_round",
            "ledger_latest_round",
            "latest_done_round",
            "transition_kind",
            "transition_status_label",
            "ledger_commit_pending",
            "round_in_progress",
            "evidence_ids",
            "reason_codes",
        ]
    }

    fn cli_workers_host_json_fields(dto: &CliWorkersHostSnapshot) -> Vec<&'static str> {
        let CliWorkersHostSnapshot {
            read_only: _read_only,
            launches_process: _launches_process,
            sends_prompt: _sends_prompt,
            starts_stream: _starts_stream,
            carries_request_preview: _carries_request_preview,
            carries_stream_chunk: _carries_stream_chunk,
            carries_input_action_snapshot: _carries_input_action_snapshot,
            route: _route,
            model_role_label: _model_role_label,
            routing_preference_label: _routing_preference_label,
            endpoint_label: _endpoint_label,
            endpoint_pinned: _endpoint_pinned,
            endpoint_kind_label: _endpoint_kind_label,
            wire_model_role_label: _wire_model_role_label,
            wire_routing_preference_label: _wire_routing_preference_label,
            wire_prefer_fast: _wire_prefer_fast,
            wire_prefer_quality: _wire_prefer_quality,
            wire_endpoint_pinned: _wire_endpoint_pinned,
            wire_endpoint_kind_label: _wire_endpoint_kind_label,
            wire_sends_model_endpoint: _wire_sends_model_endpoint,
            wire_model_endpoint_label: _wire_model_endpoint_label,
            send_allowed: _send_allowed,
            send_block_state_label: _send_block_state_label,
            send_block_reason: _send_block_reason,
            route_send_allowed: _route_send_allowed,
            route_send_block_state_label: _route_send_block_state_label,
            route_send_block_reason: _route_send_block_reason,
            gate_advice_action_label: _gate_advice_action_label,
            route_gate_advice_action_label: _route_gate_advice_action_label,
            pool_status: _pool_status,
            route_pool_status: _route_pool_status,
            history_messages: _history_messages,
            partial_chars: _partial_chars,
            workers: _workers,
        } = dto;

        vec![
            "read_only",
            "launches_process",
            "sends_prompt",
            "starts_stream",
            "carries_request_preview",
            "carries_stream_chunk",
            "carries_input_action_snapshot",
            "route",
            "model_role_label",
            "routing_preference_label",
            "endpoint_label",
            "endpoint_pinned",
            "endpoint_kind_label",
            "wire_model_role_label",
            "wire_routing_preference_label",
            "wire_prefer_fast",
            "wire_prefer_quality",
            "wire_endpoint_pinned",
            "wire_endpoint_kind_label",
            "wire_sends_model_endpoint",
            "wire_model_endpoint_label",
            "send_allowed",
            "send_block_state_label",
            "send_block_reason",
            "route_send_allowed",
            "route_send_block_state_label",
            "route_send_block_reason",
            "gate_advice_action_label",
            "route_gate_advice_action_label",
            "pool_status",
            "route_pool_status",
            "history_messages",
            "partial_chars",
            "workers",
        ]
    }

    fn cli_worker_host_json_fields(dto: &CliWorkerHostSnapshot) -> Vec<&'static str> {
        let CliWorkerHostSnapshot {
            endpoint_label: _endpoint_label,
            role_labels: _role_labels,
            preference_labels: _preference_labels,
            worker_status_label: _worker_status_label,
            worker_status_state_label: _worker_status_state_label,
            worker_status_is_available: _worker_status_is_available,
            worker_status_is_pressure: _worker_status_is_pressure,
            worker_status_blocks_prompt_submit: _worker_status_blocks_prompt_submit,
            worker_status_display_snapshot: _worker_status_display_snapshot,
            endpoint_selected: _endpoint_selected,
            route_match: _route_match,
            selectable: _selectable,
            picker_action_label: _picker_action_label,
            decision_action_label: _decision_action_label,
            decision_state_label: _decision_state_label,
            decision_reason: _decision_reason,
            decision_display_snapshot: _decision_display_snapshot,
            selection_summary: _selection_summary,
            selection_model_role_label: _selection_model_role_label,
            selection_routing_preference_label: _selection_routing_preference_label,
            selection_endpoint_label: _selection_endpoint_label,
            selection_endpoint_kind_label: _selection_endpoint_kind_label,
            selection_wire_model_role_label: _selection_wire_model_role_label,
            selection_wire_routing_preference_label: _selection_wire_routing_preference_label,
            selection_wire_prefer_fast: _selection_wire_prefer_fast,
            selection_wire_prefer_quality: _selection_wire_prefer_quality,
            selection_wire_endpoint_pinned: _selection_wire_endpoint_pinned,
            selection_wire_endpoint_kind_label: _selection_wire_endpoint_kind_label,
            selection_wire_sends_model_endpoint: _selection_wire_sends_model_endpoint,
            selection_wire_model_endpoint_label: _selection_wire_model_endpoint_label,
        } = dto;

        vec![
            "endpoint_label",
            "role_labels",
            "preference_labels",
            "worker_status_label",
            "worker_status_state_label",
            "worker_status_is_available",
            "worker_status_is_pressure",
            "worker_status_blocks_prompt_submit",
            "worker_status_display_snapshot",
            "endpoint_selected",
            "route_match",
            "selectable",
            "picker_action_label",
            "decision_action_label",
            "decision_state_label",
            "decision_reason",
            "decision_display_snapshot",
            "selection_summary",
            "selection_model_role_label",
            "selection_routing_preference_label",
            "selection_endpoint_label",
            "selection_endpoint_kind_label",
            "selection_wire_model_role_label",
            "selection_wire_routing_preference_label",
            "selection_wire_prefer_fast",
            "selection_wire_prefer_quality",
            "selection_wire_endpoint_pinned",
            "selection_wire_endpoint_kind_label",
            "selection_wire_sends_model_endpoint",
            "selection_wire_model_endpoint_label",
        ]
    }

    #[test]
    fn model_pool_status_snapshot_keeps_backend_offline_over_available_worker() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                backend_online: false,
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);

        assert_eq!(
            status.gate_advice.as_deref(),
            Some("repair_gate failed: backend is offline")
        );
        assert_eq!(
            status.gate_advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(status.gate_advice_state_label.as_deref(), Some("failed"));
        assert_eq!(
            status.gate_advice_reason.as_deref(),
            Some("backend is offline")
        );
        assert!(!status.send_allowed);
        assert_eq!(status.send_block_state, Some(StreamState::Failed));
        assert_eq!(status.send_block_state_label.as_deref(), Some("failed"));
        assert!(status.send_block_state_is_terminal);
        assert!(!status.send_block_state_is_pressure);
        assert!(!status.send_block_state_blocks_prompt_submit);
        assert_eq!(
            status.send_block_reason.as_deref(),
            Some("backend is offline")
        );
        let send_block = status
            .send_block_chunk
            .as_ref()
            .expect("backend offline should expose a send block chunk");
        assert_eq!(send_block.output_label, "error");
        assert_eq!(send_block.appended, "[error] backend is offline");

        assert_eq!(
            status.route_gate_advice.as_deref(),
            Some("repair_gate failed: backend is offline")
        );
        assert_eq!(
            status.route_gate_advice_action_label.as_deref(),
            Some("repair_gate")
        );
        assert_eq!(
            status.route_gate_advice_reason.as_deref(),
            Some("backend is offline")
        );
        assert_eq!(status.route_send_allowed, Some(false));
        assert_eq!(status.route_send_block_state, Some(StreamState::Failed));
        assert_eq!(
            status.route_send_block_state_label.as_deref(),
            Some("failed")
        );
        assert_eq!(status.route_send_block_state_is_terminal, Some(true));
        assert_eq!(status.route_send_block_state_is_pressure, Some(false));
        assert_eq!(
            status.route_send_block_state_blocks_prompt_submit,
            Some(false)
        );
        assert_eq!(
            status.route_send_block_reason.as_deref(),
            Some("backend is offline")
        );
        let route_block = status
            .route_send_block_chunk
            .as_ref()
            .expect("backend offline should expose a route block chunk");
        assert_eq!(route_block.output_label, "error");
        assert_eq!(route_block.appended, "[error] backend is offline");

        assert_eq!(
            status.pool_status.as_deref(),
            Some("workers total=1 available=1 busy=0 saturated=0")
        );
        assert_eq!(status.pool_has_available_workers, Some(true));
        assert_eq!(
            status.route_pool_status.as_deref(),
            Some("matching total=1 available=1 busy=0 saturated=0")
        );
        assert_eq!(status.route_pool_has_matching_available_workers, Some(true));
        let workers = status
            .workers
            .as_ref()
            .expect("status should keep worker rows for read-only UI");
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].endpoint_label(), "fast-reviewer");
        assert_eq!(workers[0].status_label(), "available");
        let route_workers = status
            .route_workers
            .as_ref()
            .expect("status should keep route worker rows for picker UI");
        assert_eq!(route_workers.len(), 1);
        assert!(route_workers[0].route_match);
        assert!(!route_workers[0].selectable);
        assert_eq!(
            route_workers[0].picker_action,
            ModelRouteWorkerPickerAction::RepairGate
        );
        assert_eq!(route_workers[0].worker_status_label(), "available");
        assert_eq!(
            route_workers[0].decision,
            GateDecision::blocked(StreamState::Failed, "backend is offline")
        );
        assert_eq!(
            status.line(),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending history=0 max_tokens=backend-default partial_chars=0 advice=repair_gate failed: backend is offline pool=workers total=1 available=1 busy=0 saturated=0 route_pool=matching total=1 available=1 busy=0 saturated=0"
        );
    }

    #[test]
    fn model_pool_status_snapshot_keeps_engine_busy_over_available_worker() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
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

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);

        assert_eq!(
            status.gate_advice.as_deref(),
            Some("wait_for_current_stream busy: backend engine is busy: #77 chat-stream")
        );
        assert_eq!(
            status.gate_advice_action_label.as_deref(),
            Some("wait_for_current_stream")
        );
        assert_eq!(status.gate_advice_state_label.as_deref(), Some("busy"));
        assert_eq!(
            status.gate_advice_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        assert!(!status.send_allowed);
        assert_eq!(status.send_block_state, Some(StreamState::Busy));
        assert_eq!(status.send_block_state_label.as_deref(), Some("busy"));
        assert!(!status.send_block_state_is_terminal);
        assert!(status.send_block_state_is_pressure);
        assert!(status.send_block_state_blocks_prompt_submit);
        assert_eq!(
            status.send_block_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        let send_block = status
            .send_block_chunk
            .as_ref()
            .expect("engine busy should expose a send block chunk");
        assert_eq!(send_block.output_label, "busy");
        assert_eq!(
            send_block.appended,
            "[busy] backend engine is busy: #77 chat-stream"
        );

        assert_eq!(
            status.route_gate_advice.as_deref(),
            Some("wait_for_current_stream busy: backend engine is busy: #77 chat-stream")
        );
        assert_eq!(
            status.route_gate_advice_action_label.as_deref(),
            Some("wait_for_current_stream")
        );
        assert_eq!(
            status.route_gate_advice_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        assert_eq!(status.route_send_allowed, Some(false));
        assert_eq!(status.route_send_block_state, Some(StreamState::Busy));
        assert_eq!(status.route_send_block_state_label.as_deref(), Some("busy"));
        assert_eq!(status.route_send_block_state_is_terminal, Some(false));
        assert_eq!(status.route_send_block_state_is_pressure, Some(true));
        assert_eq!(
            status.route_send_block_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(
            status.route_send_block_reason.as_deref(),
            Some("backend engine is busy: #77 chat-stream")
        );
        let route_block = status
            .route_send_block_chunk
            .as_ref()
            .expect("engine busy should expose a route block chunk");
        assert_eq!(route_block.output_label, "busy");
        assert_eq!(
            route_block.appended,
            "[busy] backend engine is busy: #77 chat-stream"
        );

        assert_eq!(
            status.pool_status.as_deref(),
            Some("workers total=1 available=1 busy=0 saturated=0")
        );
        assert_eq!(status.pool_has_available_workers, Some(true));
        assert_eq!(
            status.route_pool_status.as_deref(),
            Some("matching total=1 available=1 busy=0 saturated=0")
        );
        assert_eq!(status.route_pool_has_matching_available_workers, Some(true));
        let route_workers = status
            .route_workers
            .as_ref()
            .expect("status should keep route worker rows for picker UI");
        assert_eq!(route_workers.len(), 1);
        assert!(route_workers[0].route_match);
        assert!(!route_workers[0].selectable);
        assert_eq!(
            route_workers[0].picker_action,
            ModelRouteWorkerPickerAction::Wait
        );
        assert_eq!(route_workers[0].worker_status_label(), "available");
        assert_eq!(
            route_workers[0].decision,
            GateDecision::blocked(StreamState::Busy, "backend engine is busy: #77 chat-stream",)
        );
        assert_eq!(
            status.line(),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending history=0 max_tokens=backend-default partial_chars=0 advice=wait_for_current_stream busy: backend engine is busy: #77 chat-stream pool=workers total=1 available=1 busy=0 saturated=0 route_pool=matching total=1 available=1 busy=0 saturated=0"
        );
    }

    #[test]
    fn worker_status_line_lists_pool_workers_without_pinning_auto_route() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("quality".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer),
            ],
        );

        assert_eq!(
            cli_model_pool_workers_line(
                &input,
                &ChatSession::new("cli", ChatSessionConfig::default()),
                &gate
            ),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false advice=send_now pending: ready to send pool=workers total=2 available=1 busy=1 saturated=0 workers=[endpoint=quality-12b status=busy queue=0/1 active=quality | endpoint=fast-reviewer status=available queue=0/1 active=none]"
        );
    }

    #[test]
    fn worker_status_line_prefers_active_session_over_ready_worker_pool() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );

        assert_eq!(
            cli_model_pool_workers_line(&input, &session, &gate),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false advice=wait_for_current_stream busy: session stream is already active pool=workers total=1 available=1 busy=0 saturated=0 workers=[endpoint=fast-reviewer status=available queue=0/1 active=none]"
        );
    }

    #[test]
    fn worker_status_line_prefers_active_session_over_route_queue() {
        let mut session = ChatSession::new("cli", ChatSessionConfig::default());
        session
            .try_submit_and_begin_stream("hello")
            .expect("expected active stream");
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
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
        let route_workers = status
            .route_workers
            .as_ref()
            .expect("worker status should expose structured picker rows");
        let worker_status_chunk = route_workers[0]
            .worker_status_display_snapshot()
            .expect("busy worker should expose a display chunk");
        assert_eq!(worker_status_chunk.output_label, "busy");
        assert_eq!(
            worker_status_chunk.appended,
            "[busy] worker fast-reviewer is busy: #18 review"
        );
        assert!(worker_status_chunk.state_blocks_prompt_submit);
        let route_decision_chunk = route_workers[0]
            .decision_display_snapshot()
            .expect("busy route worker should expose a route decision chunk");
        assert_eq!(route_decision_chunk.output_label, "busy");
        assert_eq!(
            route_decision_chunk.appended,
            "[busy] worker fast-reviewer is busy: #18 review"
        );
        assert!(route_decision_chunk.state_blocks_prompt_submit);

        assert_eq!(
            cli_model_pool_workers_line(&input, &session, &gate),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false advice=wait_for_current_stream busy: session stream is already active pool=workers total=1 available=0 busy=1 saturated=0 route_pool=matching total=1 available=0 busy=1 saturated=0 workers=[endpoint=fast-reviewer status=busy queue=0/1 active=#18 review roles=reviewer preferences=prefer_fast]"
        );
    }

    #[test]
    fn worker_status_line_explains_pinned_busy_worker() {
        let input =
            CliInputConfig::default().with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_busy(true, Some("#9 review".to_owned())),
            ],
        );

        assert_eq!(
            cli_model_pool_workers_line(
                &input,
                &ChatSession::new("cli", ChatSessionConfig::default()),
                &gate
            ),
            "role=assistant preference=balanced endpoint=fast-reviewer pinned=true advice=wait_for_current_stream busy: worker fast-reviewer is busy: #9 review pool=workers total=2 available=1 busy=1 saturated=0 workers=[endpoint=quality-12b status=available queue=0/1 active=none | endpoint=fast-reviewer status=busy queue=0/1 active=#9 review]"
        );
    }

    #[test]
    fn worker_status_line_exposes_worker_capabilities_and_no_match_advice() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Tester)
            .with_routing_preference(RoutingPreference::PreferFast);
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

        assert_eq!(
            cli_model_pool_workers_line(
                &input,
                &ChatSession::new("cli", ChatSessionConfig::default()),
                &gate
            ),
            "role=tester preference=prefer_fast endpoint=auto pinned=false advice=wait_for_worker queued: no model worker matches role=tester preference=prefer_fast pool=workers total=2 available=2 busy=0 saturated=0 route_pool=matching total=0 available=0 busy=0 saturated=0 workers=[endpoint=quality-12b status=available queue=0/1 active=none roles=assistant preferences=prefer_quality | endpoint=fast-reviewer status=available queue=0/1 active=none roles=reviewer preferences=prefer_fast]"
        );
    }

    #[test]
    fn worker_status_line_scopes_route_pool_to_pinned_endpoint() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Assistant)
            .with_routing_preference(RoutingPreference::PreferQuality)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality])
                    .with_busy(true, Some("#13 pinned".to_owned())),
            ],
        );

        assert_eq!(
            cli_model_pool_workers_line(
                &input,
                &ChatSession::new("cli", ChatSessionConfig::default()),
                &gate
            ),
            "role=assistant preference=prefer_quality endpoint=fast-reviewer pinned=true advice=wait_for_current_stream busy: worker fast-reviewer is busy: #13 pinned pool=workers total=2 available=1 busy=1 saturated=0 route_pool=matching total=1 available=0 busy=1 saturated=0 workers=[endpoint=quality-12b status=available queue=0/1 active=none roles=assistant preferences=prefer_quality | endpoint=fast-reviewer status=busy queue=0/1 active=#13 pinned roles=assistant preferences=prefer_quality]"
        );
    }

    #[test]
    fn worker_status_line_shows_pinned_capability_mismatch_as_zero_route_pool() {
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Assistant)
            .with_routing_preference(RoutingPreference::PreferQuality)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));
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

        assert_eq!(
            cli_model_pool_workers_line(
                &input,
                &ChatSession::new("cli", ChatSessionConfig::default()),
                &gate
            ),
            "role=assistant preference=prefer_quality endpoint=fast-reviewer pinned=true advice=wait_for_worker queued: worker fast-reviewer does not match role=assistant preference=prefer_quality pool=workers total=2 available=2 busy=0 saturated=0 route_pool=matching total=0 available=0 busy=0 saturated=0 workers=[endpoint=quality-12b status=available queue=0/1 active=none roles=assistant preferences=prefer_quality | endpoint=fast-reviewer status=available queue=0/1 active=none roles=reviewer preferences=prefer_fast]"
        );
    }

    #[test]
    fn model_pool_status_line_includes_route_pool_capacity_for_capability_pools() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8 review".to_owned())),
            ],
        );

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);

        assert_eq!(status.wire_model_role_label, "reviewer");
        assert_eq!(status.wire_routing_preference_label, "prefer_fast");
        assert!(status.wire_prefer_fast);
        assert!(!status.wire_prefer_quality);
        assert!(!status.wire_endpoint_pinned);
        assert_eq!(status.wire_endpoint_kind_label, "auto");
        assert!(!status.wire_sends_model_endpoint);
        assert_eq!(status.wire_model_endpoint_label, None);
        assert_eq!(
            status.route_pool_status.as_deref(),
            Some("matching total=1 available=0 busy=1 saturated=0")
        );
        assert_eq!(status.pool_queue_label.as_deref(), Some("0/2"));
        assert_eq!(status.route_pool_queue_label.as_deref(), Some("0/1"));
        assert_eq!(status.route_pool_has_matching_workers, Some(true));
        assert_eq!(
            status.route_pool_has_matching_available_workers,
            Some(false)
        );
        assert_eq!(status.route_pool_has_matching_busy_workers, Some(true));
        assert_eq!(
            status.route_pool_has_matching_saturated_workers,
            Some(false)
        );
        assert_eq!(status.route_pool_has_matching_queued_requests, Some(false));
        assert_eq!(status.route_pool_queue_is_saturated, Some(false));
        assert_eq!(
            status.route_gate_advice_state_label.as_deref(),
            Some("queued")
        );
        assert_eq!(
            status.route_gate_advice_reason.as_deref(),
            Some("all matching model workers are busy; waiting for scheduler across 1 workers")
        );
        let route_pool = status
            .route_pool
            .as_ref()
            .expect("route pool should be structured");
        assert_eq!(route_pool.matching_workers, 1);
        assert_eq!(route_pool.matching_available_workers, 0);
        assert_eq!(route_pool.matching_busy_workers, 1);
        assert_eq!(route_pool.matching_saturated_workers, 0);
        assert_eq!(route_pool.matching_queued_requests, 0);
        assert_eq!(route_pool.matching_queue_limit, 1);
        assert_eq!(
            status.line(),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending history=0 max_tokens=backend-default partial_chars=0 advice=wait_for_worker queued: all matching model workers are busy; waiting for scheduler across 1 workers pool=workers total=2 available=1 busy=1 saturated=0 route_pool=matching total=1 available=0 busy=1 saturated=0"
        );
    }

    #[test]
    fn model_pool_status_snapshot_exposes_route_pool_backpressure_for_saturated_match() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
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

        assert_eq!(
            status.gate_advice.as_deref(),
            Some("retry_later backpressure: matching model workers are saturated: 1 workers")
        );
        assert_eq!(
            status.gate_advice_action_label.as_deref(),
            Some("retry_later")
        );
        assert_eq!(
            status.gate_advice_state_label.as_deref(),
            Some("backpressure")
        );
        assert!(!status.send_allowed);
        assert_eq!(status.send_block_state, Some(StreamState::Backpressure));
        assert_eq!(
            status.send_block_state_label.as_deref(),
            Some("backpressure")
        );
        assert!(!status.send_block_state_is_terminal);
        assert!(status.send_block_state_is_pressure);
        assert!(status.send_block_state_blocks_prompt_submit);
        assert_eq!(
            status.send_block_reason.as_deref(),
            Some("matching model workers are saturated: 1 workers")
        );
        assert_eq!(status.route_send_allowed, Some(false));
        assert_eq!(
            status.route_send_block_state,
            Some(StreamState::Backpressure)
        );
        assert_eq!(
            status.route_send_block_state_label.as_deref(),
            Some("backpressure")
        );
        assert_eq!(status.route_send_block_state_is_terminal, Some(false));
        assert_eq!(status.route_send_block_state_is_pressure, Some(true));
        assert_eq!(
            status.route_send_block_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(
            status.route_send_block_reason.as_deref(),
            Some("matching model workers are saturated: 1 workers")
        );
        let route_block = status
            .route_send_block_chunk
            .as_ref()
            .expect("route backpressure should expose a block chunk");
        assert_eq!(route_block.output_label, "backpressure");
        assert_eq!(
            route_block.appended,
            "[backpressure] matching model workers are saturated: 1 workers"
        );

        assert_eq!(
            status.pool_status.as_deref(),
            Some("workers total=2 available=1 busy=0 saturated=1")
        );
        assert_eq!(status.pool_has_available_workers, Some(true));
        assert_eq!(status.pool_has_saturated_workers, Some(true));
        assert_eq!(status.pool_queue_is_saturated, Some(false));
        assert_eq!(
            status.route_pool_status.as_deref(),
            Some("matching total=1 available=0 busy=0 saturated=1")
        );
        assert_eq!(status.route_pool_queue_label.as_deref(), Some("1/1"));
        assert_eq!(
            status.route_pool_capacity_state,
            Some(StreamState::Backpressure)
        );
        assert_eq!(
            status.route_pool_capacity_state_label.as_deref(),
            Some("backpressure")
        );
        assert_eq!(status.route_pool_capacity_state_is_pressure, Some(true));
        assert_eq!(
            status.route_pool_capacity_state_blocks_prompt_submit,
            Some(true)
        );
        assert_eq!(status.route_pool_has_matching_workers, Some(true));
        assert_eq!(
            status.route_pool_has_matching_available_workers,
            Some(false)
        );
        assert_eq!(status.route_pool_has_matching_saturated_workers, Some(true));
        assert_eq!(status.route_pool_has_matching_queued_requests, Some(true));
        assert_eq!(status.route_pool_queue_is_saturated, Some(true));

        let route_workers = status
            .route_workers
            .as_ref()
            .expect("route workers should be structured");
        assert_eq!(route_workers.len(), 2);
        assert!(!route_workers[0].route_match);
        assert!(!route_workers[0].selectable);
        assert_eq!(
            route_workers[0].picker_action,
            ModelRouteWorkerPickerAction::Unavailable
        );
        assert!(route_workers[1].route_match);
        assert!(!route_workers[1].selectable);
        assert_eq!(
            route_workers[1].picker_action,
            ModelRouteWorkerPickerAction::Wait
        );
        assert_eq!(route_workers[1].worker_status_label(), "backpressure");
        assert_eq!(route_workers[1].decision_action_label(), "retry_later");
        assert_eq!(route_workers[1].decision_state_label(), "backpressure");
        assert_eq!(
            status.line(),
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false state=pending history=0 max_tokens=backend-default partial_chars=0 advice=retry_later backpressure: matching model workers are saturated: 1 workers pool=workers total=2 available=1 busy=0 saturated=1 route_pool=matching total=1 available=0 busy=0 saturated=1"
        );
    }

    #[test]
    fn model_pool_status_line_scopes_route_pool_to_pinned_endpoint() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Assistant)
            .with_routing_preference(RoutingPreference::PreferQuality)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality])
                    .with_busy(true, Some("#13 pinned".to_owned())),
            ],
        );

        let status = CliStatusSnapshot::from_model_pool_gate(&input, &session, &gate);

        assert_eq!(
            status.route_pool_status.as_deref(),
            Some("matching total=1 available=0 busy=1 saturated=0")
        );
        let route_workers = status
            .route_workers
            .as_ref()
            .expect("route workers should be structured");
        assert_eq!(route_workers.len(), 2);
        assert_eq!(route_workers[0].worker.endpoint.label(), "quality-12b");
        assert!(!route_workers[0].endpoint_selected);
        assert!(!route_workers[0].route_match);
        assert!(route_workers[0].selectable);
        assert_eq!(route_workers[1].worker.endpoint.label(), "fast-reviewer");
        assert!(route_workers[1].endpoint_selected);
        assert!(route_workers[1].route_match);
        assert!(!route_workers[1].selectable);
        assert_eq!(
            status.line(),
            "role=assistant preference=prefer_quality endpoint=fast-reviewer pinned=true state=pending history=0 max_tokens=backend-default partial_chars=0 advice=wait_for_current_stream busy: worker fast-reviewer is busy: #13 pinned pool=workers total=2 available=1 busy=1 saturated=0 route_pool=matching total=1 available=0 busy=1 saturated=0"
        );
    }

    #[test]
    fn model_pool_status_line_shows_pinned_capability_mismatch_as_zero_route_pool() {
        let session = ChatSession::new("cli", ChatSessionConfig::default());
        let input = CliInputConfig::default()
            .with_model_role(ModelRole::Assistant)
            .with_routing_preference(RoutingPreference::PreferQuality)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));
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

        assert_eq!(
            status.route_pool_status.as_deref(),
            Some("matching total=0 available=0 busy=0 saturated=0")
        );
        assert_eq!(
            status.line(),
            "role=assistant preference=prefer_quality endpoint=fast-reviewer pinned=true state=pending history=0 max_tokens=backend-default partial_chars=0 advice=wait_for_worker queued: worker fast-reviewer does not match role=assistant preference=prefer_quality pool=workers total=2 available=2 busy=0 saturated=0 route_pool=matching total=0 available=0 busy=0 saturated=0"
        );
    }

    #[test]
    fn worker_status_line_without_pool_gate_is_local_status() {
        let input = CliInputConfig::default();

        assert_eq!(
            cli_workers_unavailable_line(&input),
            "role=assistant preference=balanced endpoint=auto pinned=false workers=unavailable reason=model-pool-gate-not-attached"
        );
    }

    fn captured_current_status_next_round_decision_json_fixture() -> &'static str {
        r#"{
  "daemon_running": true,
  "daemon_pid": 199264,
  "active_round": 370,
  "ledger_latest_round": 369,
  "latest_done_round": 369,
  "round_in_progress": true,
  "live_status_bundle": {
    "display_state": "safe-to-wait",
    "current_round": 370,
    "latest_done_round": 369,
    "next_round_decision": {
      "decision_status": "safe_to_wait_current_round_active",
      "display_state": "safe-to-wait",
      "live_status_display_state": "safe-to-wait",
      "current_round_active": true,
      "readiness_can_schedule_next_round": false,
      "report_gate_ready": true,
      "context_hygiene_passed": true,
      "read_only": true,
      "report_only": true,
      "no_side_effects": true,
      "dispatch_work_allowed": false,
      "prompt_replay_allowed": false,
      "process_start_allowed": false,
      "memory_write_allowed": false,
      "ndkv_write_allowed": false,
      "current_round": 370,
      "latest_done_round": 369,
      "evidence_ids": [
        "live_status_bundle:next_round_decision:round-370",
        "next_round_decision:display_state:safe-to-wait"
      ],
      "reason_codes": [
        "current_round_active",
        "safe_to_wait"
      ],
      "failure_reasons": []
    }
  },
  "next_round_decision": {
    "decision_status": "safe_to_wait_current_round_active",
    "display_state": "safe-to-wait"
  },
  "next_round_decision_report_v1": {
    "decision_status": "safe_to_wait_current_round_active",
    "display_state": "safe-to-wait",
    "read_only": true,
    "report_only": true,
    "no_side_effects": true
  },
  "next_round_downstream_status_consumers_v1": {
    "next_round_downstream": {
      "source_decision_status": "safe_to_wait_current_round_active",
      "effective_decision_status": "safe_to_wait_current_round_active",
      "service_cli_display_status": "display_safe_to_wait_current_round",
      "forge_operator_display_status": "forge_safe_to_wait",
      "agent_assignment_acceptance": "defer_until_current_round_completes",
      "memory_self_improve_admission_visibility": "visible_admission_waiting",
      "operator_attention_required": false,
      "read_only": true,
      "report_only": true,
      "no_side_effects": true,
      "dispatch_work_allowed": false,
      "prompt_replay_allowed": false,
      "process_start_allowed": false,
      "memory_write_allowed": false,
      "ndkv_write_allowed": false,
      "current_round_active": true,
      "live_status_display_state": "safe-to-wait",
      "active_round": 370,
      "ledger_latest_round": 369,
      "latest_done_round": 369,
      "readiness_can_schedule_next_round": false,
      "round_id_evidence": {
        "source_schema": "daemon_round_transition_status_v1",
        "active_round": 370,
        "ledger_latest_round": 369,
        "latest_done_round": 369,
        "transition_kind": "normal_in_progress",
        "transition_status_label": "round-in-progress",
        "ledger_commit_pending": false,
        "round_in_progress": true,
        "evidence_ids": [
          "daemon_transition:active-round-370",
          "ledger:latest-round-369"
        ],
        "reason_codes": [
          "current_round_active"
        ]
      },
      "failure_reasons": []
    }
  }
}"#
    }

    fn assert_captured_current_status_next_round_decision_shape(json: &str) {
        for field in [
            "\"live_status_bundle\"",
            "\"next_round_decision\"",
            "\"next_round_decision_report_v1\"",
            "\"next_round_downstream_status_consumers_v1\"",
            "\"next_round_downstream\"",
            "\"decision_status\"",
            "\"display_state\"",
            "\"current_round_active\"",
            "\"readiness_can_schedule_next_round\"",
            "\"service_cli_display_status\"",
            "\"forge_operator_display_status\"",
            "\"agent_assignment_acceptance\"",
            "\"memory_self_improve_admission_visibility\"",
            "\"dispatch_work_allowed\"",
            "\"process_start_allowed\"",
            "\"memory_write_allowed\"",
            "\"ndkv_write_allowed\"",
            "\"round_id_evidence\"",
            "\"source_schema\"",
            "\"evidence_ids\"",
            "\"reason_codes\"",
        ] {
            assert!(json.contains(field), "fixture missing field {field}");
        }
    }

    fn captured_current_status_next_round_decision_report_from_json(
        json: &str,
    ) -> Option<norion_service::SmartSteamNextRoundDecisionReportStatusSource> {
        let live_status_bundle = json_object_after_key(json, "live_status_bundle");
        let report_json = live_status_bundle
            .and_then(|bundle| json_object_after_key(bundle, "next_round_decision"))
            .or_else(|| json_object_after_key(json, "next_round_decision_report_v1"))
            .or_else(|| json_object_after_key(json, "next_round_decision"))?;

        let current_round = json_u64_value(report_json, "current_round")
            .or_else(|| {
                live_status_bundle.and_then(|bundle| json_u64_value(bundle, "current_round"))
            })
            .or_else(|| json_u64_value(json, "active_round"));
        let latest_done_round = json_u64_value(report_json, "latest_done_round")
            .or_else(|| {
                live_status_bundle.and_then(|bundle| json_u64_value(bundle, "latest_done_round"))
            })
            .or_else(|| json_u64_value(json, "latest_done_round"));

        Some(
            norion_service::SmartSteamNextRoundDecisionReportStatusSource {
                decision_status: json_string_value(report_json, "decision_status"),
                display_state: json_string_value(report_json, "display_state"),
                live_status_display_state: json_string_value(
                    report_json,
                    "live_status_display_state",
                )
                .or_else(|| {
                    live_status_bundle.and_then(|bundle| json_string_value(bundle, "display_state"))
                }),
                current_round_active: json_bool_value(report_json, "current_round_active"),
                readiness_can_schedule_next_round: json_bool_value(
                    report_json,
                    "readiness_can_schedule_next_round",
                ),
                report_gate_ready: json_bool_value(report_json, "report_gate_ready"),
                context_hygiene_passed: json_bool_value(report_json, "context_hygiene_passed"),
                read_only: json_bool_value(report_json, "read_only"),
                report_only: json_bool_value(report_json, "report_only"),
                no_side_effects: json_bool_value(report_json, "no_side_effects"),
                dispatch_work_allowed: json_bool_value(report_json, "dispatch_work_allowed"),
                prompt_replay_allowed: json_bool_value(report_json, "prompt_replay_allowed"),
                process_start_allowed: json_bool_value(report_json, "process_start_allowed"),
                memory_write_allowed: json_bool_value(report_json, "memory_write_allowed"),
                ndkv_write_allowed: json_bool_value(report_json, "ndkv_write_allowed"),
                operator_attention_required: json_bool_value(
                    report_json,
                    "operator_attention_required",
                ),
                current_round,
                latest_done_round,
                evidence_ids: json_string_array_value(report_json, "evidence_ids"),
                reason_codes: json_string_array_value(report_json, "reason_codes"),
                failure_reasons: json_string_array_value(report_json, "failure_reasons"),
                downstream_status_consumers:
                    captured_current_status_next_round_downstream_status_from_json(json),
            },
        )
    }

    fn captured_current_status_next_round_downstream_status_from_json(
        json: &str,
    ) -> Option<norion_service::SmartSteamNextRoundDownstreamConsumerStatusSource> {
        let live_status_bundle = json_object_after_key(json, "live_status_bundle");
        let container = live_status_bundle
            .and_then(|bundle| {
                json_object_after_key(bundle, "next_round_downstream_status_consumers_v1")
            })
            .or_else(|| json_object_after_key(json, "next_round_downstream_status_consumers_v1"))?;
        let downstream_json =
            json_object_after_key(container, "next_round_downstream").unwrap_or(container);

        Some(
            norion_service::SmartSteamNextRoundDownstreamConsumerStatusSource {
                source_decision_status: json_string_value(
                    downstream_json,
                    "source_decision_status",
                ),
                effective_decision_status: json_string_value(
                    downstream_json,
                    "effective_decision_status",
                ),
                service_cli_display_status: json_string_value(
                    downstream_json,
                    "service_cli_display_status",
                ),
                forge_operator_display_status: json_string_value(
                    downstream_json,
                    "forge_operator_display_status",
                ),
                agent_assignment_acceptance: json_string_value(
                    downstream_json,
                    "agent_assignment_acceptance",
                ),
                memory_self_improve_admission_visibility: json_string_value(
                    downstream_json,
                    "memory_self_improve_admission_visibility",
                ),
                operator_attention_required: json_bool_value(
                    downstream_json,
                    "operator_attention_required",
                ),
                read_only: json_bool_value(downstream_json, "read_only"),
                report_only: json_bool_value(downstream_json, "report_only"),
                no_side_effects: json_bool_value(downstream_json, "no_side_effects"),
                dispatch_work_allowed: json_bool_value(downstream_json, "dispatch_work_allowed"),
                prompt_replay_allowed: json_bool_value(downstream_json, "prompt_replay_allowed"),
                process_start_allowed: json_bool_value(downstream_json, "process_start_allowed"),
                memory_write_allowed: json_bool_value(downstream_json, "memory_write_allowed"),
                ndkv_write_allowed: json_bool_value(downstream_json, "ndkv_write_allowed"),
                current_round_active: json_bool_value(downstream_json, "current_round_active"),
                live_status_display_state: json_string_value(
                    downstream_json,
                    "live_status_display_state",
                ),
                active_round: json_u64_value(downstream_json, "active_round"),
                ledger_latest_round: json_u64_value(downstream_json, "ledger_latest_round"),
                latest_done_round: json_u64_value(downstream_json, "latest_done_round"),
                readiness_can_schedule_next_round: json_bool_value(
                    downstream_json,
                    "readiness_can_schedule_next_round",
                ),
                round_id_evidence: captured_current_status_next_round_round_id_evidence_from_json(
                    downstream_json,
                ),
                failure_reasons: json_string_array_value(downstream_json, "failure_reasons"),
            },
        )
    }

    fn captured_current_status_next_round_round_id_evidence_from_json(
        json: &str,
    ) -> Option<norion_service::SmartSteamNextRoundRoundIdEvidenceSource> {
        let round_id_evidence = json_object_after_key(json, "round_id_evidence")?;
        Some(norion_service::SmartSteamNextRoundRoundIdEvidenceSource {
            source_schema: json_string_value(round_id_evidence, "source_schema"),
            active_round: json_u64_value(round_id_evidence, "active_round"),
            ledger_latest_round: json_u64_value(round_id_evidence, "ledger_latest_round"),
            latest_done_round: json_u64_value(round_id_evidence, "latest_done_round"),
            transition_kind: json_string_value(round_id_evidence, "transition_kind"),
            transition_status_label: json_string_value(
                round_id_evidence,
                "transition_status_label",
            ),
            ledger_commit_pending: json_bool_value(round_id_evidence, "ledger_commit_pending"),
            round_in_progress: json_bool_value(round_id_evidence, "round_in_progress"),
            evidence_ids: json_string_array_value(round_id_evidence, "evidence_ids"),
            reason_codes: json_string_array_value(round_id_evidence, "reason_codes"),
        })
    }

    fn json_object_after_key<'a>(json: &'a str, key: &str) -> Option<&'a str> {
        json_delimited_value_after_key(json, key, '{', '}')
    }

    fn json_array_after_key<'a>(json: &'a str, key: &str) -> Option<&'a str> {
        json_delimited_value_after_key(json, key, '[', ']')
    }

    fn json_delimited_value_after_key<'a>(
        json: &'a str,
        key: &str,
        open: char,
        close: char,
    ) -> Option<&'a str> {
        let value = json_value_after_key(json, key)?;
        let start = value.find(open)?;
        if !value[..start].trim().is_empty() {
            return None;
        }

        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        for (offset, ch) in value[start..].char_indices() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
                continue;
            }

            if ch == '"' {
                in_string = true;
            } else if ch == open {
                depth += 1;
            } else if ch == close {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    let end = start + offset + ch.len_utf8();
                    return Some(&value[start..end]);
                }
            }
        }
        None
    }

    fn json_value_after_key<'a>(json: &'a str, key: &str) -> Option<&'a str> {
        let marker = format!("\"{key}\"");
        let key_at = json.find(&marker)?;
        let after_key = &json[key_at + marker.len()..];
        let colon_at = after_key.find(':')?;
        Some(after_key[colon_at + 1..].trim_start())
    }

    fn json_string_value(json: &str, key: &str) -> Option<String> {
        let value = json_value_after_key(json, key)?;
        if !value.starts_with('"') {
            return None;
        }

        let mut parsed = String::new();
        let mut escaped = false;
        for ch in value[1..].chars() {
            if escaped {
                parsed.push(ch);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                return Some(parsed);
            } else {
                parsed.push(ch);
            }
        }
        None
    }

    fn json_bool_value(json: &str, key: &str) -> Option<bool> {
        let value = json_value_after_key(json, key)?;
        if value.starts_with("true") {
            Some(true)
        } else if value.starts_with("false") {
            Some(false)
        } else {
            None
        }
    }

    fn json_u64_value(json: &str, key: &str) -> Option<u64> {
        let value = json_value_after_key(json, key)?;
        let digits: String = value.chars().take_while(|ch| ch.is_ascii_digit()).collect();
        if digits.is_empty() {
            None
        } else {
            digits.parse().ok()
        }
    }

    fn json_string_array_value(json: &str, key: &str) -> Vec<String> {
        let Some(array) = json_array_after_key(json, key) else {
            return Vec::new();
        };
        let mut values = Vec::new();
        let mut in_string = false;
        let mut escaped = false;
        let mut parsed = String::new();

        for ch in array[1..array.len() - 1].chars() {
            if in_string {
                if escaped {
                    parsed.push(ch);
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    values.push(std::mem::take(&mut parsed));
                    in_string = false;
                } else {
                    parsed.push(ch);
                }
            } else if ch == '"' {
                in_string = true;
            }
        }

        values
    }

    #[derive(Debug, Clone, Copy)]
    struct CapturedDaemonJsonStatusFixture {
        daemon_running: bool,
        daemon_pid: Option<u32>,
        active_round: Option<u64>,
        ledger_latest_round: Option<u64>,
        latest_done_round: Option<u64>,
        round_in_progress: bool,
        readiness_ok: bool,
        remote_chain_ready: bool,
        model_cache_label: &'static str,
        daemon_round_transition_status_v1: CapturedDaemonRoundTransitionJsonFixture,
        worker_windows: &'static [CapturedWorkerWindowJsonFixture],
    }

    #[derive(Debug, Clone, Copy)]
    struct CapturedDaemonRoundTransitionJsonFixture {
        read_only: bool,
        starts_process: bool,
        sends_prompt: bool,
        transition_kind: &'static str,
        active_round: Option<u64>,
        ledger_latest_round: Option<u64>,
        latest_done_round: Option<u64>,
        round_in_progress: bool,
        evidence_ids: &'static [&'static str],
        reason_codes: &'static [&'static str],
    }

    #[derive(Debug, Clone, Copy)]
    struct CapturedWorkerWindowJsonFixture {
        window_id: &'static str,
        lane_label: &'static str,
        status_label: &'static str,
        reason: &'static str,
    }

    fn captured_daemon_json_status_fixture() -> (&'static str, CapturedDaemonJsonStatusFixture) {
        const CAPTURED_JSON: &str = r#"{
  "daemon_running": true,
  "daemon_pid": 235440,
  "active_round": 337,
  "ledger_latest_round": 336,
  "latest_done_round": 336,
  "round_in_progress": true,
  "readiness_ok": true,
  "remote_chain_ready": true,
  "model_cache_label": "5/5 OK",
  "daemon_round_transition_status_v1": {
    "read_only": true,
    "starts_process": false,
    "sends_prompt": false,
    "transition_kind": "normal_in_progress",
    "active_round": 337,
    "ledger_latest_round": 336,
    "latest_done_round": 336,
    "round_in_progress": true,
    "evidence_ids": [
      "daemon:status:active-round-337",
      "ledger:latest-done-round-336"
    ],
    "reason_codes": [
      "normal_in_progress",
      "active_round_after_latest_done"
    ]
  },
  "worker_windows": [
    {
      "window_id": "019ee225-a80f-7a10-aa76-ee225fbe96aa",
      "lane_label": "service-cli-live-status",
      "status_label": "completed-evidence-only",
      "reason": "completed worker evidence only; non-actionable context hygiene"
    }
  ]
}"#;
        const WORKER_WINDOWS: &[CapturedWorkerWindowJsonFixture] =
            &[CapturedWorkerWindowJsonFixture {
                window_id: "019ee225-a80f-7a10-aa76-ee225fbe96aa",
                lane_label: "service-cli-live-status",
                status_label: "completed-evidence-only",
                reason: "completed worker evidence only; non-actionable context hygiene",
            }];
        (
            CAPTURED_JSON,
            CapturedDaemonJsonStatusFixture {
                daemon_running: true,
                daemon_pid: Some(235440),
                active_round: Some(337),
                ledger_latest_round: Some(336),
                latest_done_round: Some(336),
                round_in_progress: true,
                readiness_ok: true,
                remote_chain_ready: true,
                model_cache_label: "5/5 OK",
                daemon_round_transition_status_v1: CapturedDaemonRoundTransitionJsonFixture {
                    read_only: true,
                    starts_process: false,
                    sends_prompt: false,
                    transition_kind: "normal_in_progress",
                    active_round: Some(337),
                    ledger_latest_round: Some(336),
                    latest_done_round: Some(336),
                    round_in_progress: true,
                    evidence_ids: &[
                        "daemon:status:active-round-337",
                        "ledger:latest-done-round-336",
                    ],
                    reason_codes: &["normal_in_progress", "active_round_after_latest_done"],
                },
                worker_windows: WORKER_WINDOWS,
            },
        )
    }

    fn assert_captured_daemon_json_status_shape(json: &str) {
        for field in [
            "\"daemon_round_transition_status_v1\"",
            "\"latest_done_round\"",
            "\"round_in_progress\"",
            "\"read_only\"",
            "\"starts_process\"",
            "\"sends_prompt\"",
            "\"normal_in_progress\"",
            "\"completed-evidence-only\"",
        ] {
            assert!(json.contains(field), "fixture missing field {field}");
        }
    }

    fn service_source_from_captured_daemon_json_status(
        captured: CapturedDaemonJsonStatusFixture,
    ) -> norion_service::SmartSteamStatusSource {
        let transition = captured.daemon_round_transition_status_v1;
        assert_eq!(transition.transition_kind, "normal_in_progress");
        assert_eq!(transition.active_round, captured.active_round);
        assert_eq!(transition.latest_done_round, captured.latest_done_round);
        assert_eq!(transition.ledger_latest_round, captured.ledger_latest_round);
        assert!(transition.read_only);
        assert!(!transition.starts_process);
        assert!(!transition.sends_prompt);

        norion_service::SmartSteamStatusSource::new()
            .with_daemon(
                captured.daemon_running,
                captured.daemon_pid,
                captured.active_round,
                captured.ledger_latest_round,
            )
            .with_daemon_round_progress(captured.latest_done_round, captured.round_in_progress)
            .with_supervisor(true, true)
            .with_readiness(captured.readiness_ok, captured.remote_chain_ready)
            .with_model_cache_label(captured.model_cache_label)
            .with_daemon_round_transition(
                norion_service::SmartSteamDaemonRoundTransitionStatusSource {
                    observed_round_done: false,
                    done_round: None,
                    latest_done_round: transition.latest_done_round,
                    round_in_progress: transition.round_in_progress,
                    ledger_round: transition.ledger_latest_round,
                    ledger_commit_pending: false,
                    evidence_ids: transition
                        .evidence_ids
                        .iter()
                        .map(|id| (*id).to_owned())
                        .collect(),
                    reason_codes: transition
                        .reason_codes
                        .iter()
                        .map(|code| (*code).to_owned())
                        .collect(),
                },
            )
            .with_worker_windows(
                captured
                    .worker_windows
                    .iter()
                    .map(worker_window_source_from_captured_json),
            )
    }

    fn worker_window_source_from_captured_json(
        captured: &CapturedWorkerWindowJsonFixture,
    ) -> norion_service::SmartSteamWorkerWindowStatusSource {
        match captured.status_label {
            "completed-evidence-only" => norion_service::SmartSteamWorkerWindowStatusSource::new(
                captured.window_id,
                captured.lane_label,
            )
            .with_completed_evidence_only(captured.reason),
            status => panic!("unsupported captured worker window status {status}"),
        }
    }
}
