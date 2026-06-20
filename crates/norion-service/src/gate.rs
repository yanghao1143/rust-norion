use crate::protocol::{
    ChatChunk, ChatChunkDisplaySnapshot, ModelEndpoint, ModelEndpointSelectionKind, ModelRole,
    RoutingIntent, RoutingPreference, StreamState,
};
use norion_memory::MemoryStartupAdmissionEvidence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontendGateSnapshot {
    pub backend_online: bool,
    pub engine_busy: bool,
    pub safe_device_ok: bool,
    pub experience_hygiene_ok: bool,
    pub queued_requests: usize,
    pub queue_limit: usize,
    pub active_request: Option<String>,
}

impl Default for FrontendGateSnapshot {
    fn default() -> Self {
        Self {
            backend_online: true,
            engine_busy: false,
            safe_device_ok: true,
            experience_hygiene_ok: true,
            queued_requests: 0,
            queue_limit: 1,
            active_request: None,
        }
    }
}

impl FrontendGateSnapshot {
    pub fn decision(&self) -> GateDecision {
        if !self.backend_online {
            return GateDecision::blocked(StreamState::Failed, "backend is offline");
        }

        if !self.safe_device_ok {
            return GateDecision::blocked(StreamState::Failed, "safe-device gate failed");
        }

        if !self.experience_hygiene_ok {
            return GateDecision::blocked(StreamState::Failed, "experience hygiene gate failed");
        }

        if self.engine_busy {
            let reason = self
                .active_request
                .as_deref()
                .filter(|request| !request.is_empty())
                .map(|request| format!("backend engine is busy: {request}"))
                .unwrap_or_else(|| "backend engine is busy".to_owned());
            return GateDecision::blocked(StreamState::Busy, reason);
        }

        if self.queue_limit > 0 && self.queued_requests >= self.queue_limit {
            return GateDecision::blocked(
                StreamState::Backpressure,
                format!(
                    "model queue is saturated: {}/{}",
                    self.queued_requests, self.queue_limit
                ),
            );
        }

        GateDecision::Allowed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelWorkerSnapshot {
    pub endpoint: ModelEndpoint,
    pub roles: Vec<ModelRole>,
    pub preferences: Vec<RoutingPreference>,
    pub busy: bool,
    pub queued_requests: usize,
    pub queue_limit: usize,
    pub active_request: Option<String>,
}

impl ModelWorkerSnapshot {
    pub fn new(endpoint: ModelEndpoint) -> Self {
        Self {
            endpoint,
            roles: Vec::new(),
            preferences: Vec::new(),
            busy: false,
            queued_requests: 0,
            queue_limit: 1,
            active_request: None,
        }
    }

    pub fn with_busy(mut self, busy: bool, active_request: Option<String>) -> Self {
        self.busy = busy;
        self.active_request = active_request;
        self
    }

    pub fn with_queue(mut self, queued_requests: usize, queue_limit: usize) -> Self {
        self.queued_requests = queued_requests;
        self.queue_limit = queue_limit;
        self
    }

    pub fn with_roles(mut self, roles: impl IntoIterator<Item = ModelRole>) -> Self {
        self.roles = roles.into_iter().collect();
        self
    }

    pub fn with_preferences(
        mut self,
        preferences: impl IntoIterator<Item = RoutingPreference>,
    ) -> Self {
        self.preferences = preferences.into_iter().collect();
        self
    }

    pub fn is_saturated(&self) -> bool {
        self.queue_limit > 0 && self.queued_requests >= self.queue_limit
    }

    pub fn accepts_intent(&self, intent: &RoutingIntent) -> bool {
        let role_matches = self.roles.is_empty() || self.roles.contains(&intent.model_role);
        let preference_matches = self.preferences.is_empty()
            || intent.routing_preference == RoutingPreference::Balanced
            || self.preferences.contains(&intent.routing_preference);
        role_matches && preference_matches
    }

    pub fn status_label(&self) -> &'static str {
        if self.is_available() {
            "available"
        } else {
            self.status_state().as_str()
        }
    }

    pub fn status_state(&self) -> StreamState {
        if self.is_saturated() {
            StreamState::Backpressure
        } else if self.busy {
            StreamState::Busy
        } else {
            StreamState::Pending
        }
    }

    pub fn status_state_label(&self) -> &'static str {
        self.status_state().as_str()
    }

    pub fn is_available(&self) -> bool {
        !self.busy && !self.is_saturated()
    }

    pub fn status_is_pressure(&self) -> bool {
        self.status_state().is_pressure()
    }

    pub fn status_blocks_prompt_submit(&self) -> bool {
        self.status_state().blocks_prompt_submit()
    }

    pub fn status_display_snapshot(&self) -> Option<ChatChunkDisplaySnapshot> {
        self.decision().display_snapshot(0)
    }

    pub fn endpoint_label(&self) -> &str {
        self.endpoint.label()
    }

    pub fn role_labels(&self) -> Vec<String> {
        self.roles
            .iter()
            .map(|role| role.as_str().to_owned())
            .collect()
    }

    pub fn preference_labels(&self) -> Vec<String> {
        self.preferences
            .iter()
            .map(|preference| preference.as_str().to_owned())
            .collect()
    }

    pub fn accepts_any_role(&self) -> bool {
        self.roles.is_empty()
    }

    pub fn accepts_any_preference(&self) -> bool {
        self.preferences.is_empty()
    }

    pub fn queue_label(&self) -> String {
        format!("{}/{}", self.queued_requests, self.queue_limit)
    }

    pub fn active_request_label(&self) -> &str {
        self.active_request
            .as_deref()
            .filter(|request| !request.is_empty())
            .unwrap_or("none")
    }

    pub fn summary(&self) -> String {
        let mut parts = vec![format!(
            "endpoint={} status={} queue={}/{} active={}",
            self.endpoint_label(),
            self.status_label(),
            self.queued_requests,
            self.queue_limit,
            self.active_request_label()
        )];
        if !self.roles.is_empty() {
            parts.push(format!(
                "roles={}",
                labels(self.roles.iter().map(|role| role.as_str()))
            ));
        }
        if !self.preferences.is_empty() {
            parts.push(format!(
                "preferences={}",
                labels(
                    self.preferences
                        .iter()
                        .map(|preference| preference.as_str())
                )
            ));
        }
        parts.join(" ")
    }

    fn decision(&self) -> GateDecision {
        if self.is_saturated() {
            return GateDecision::blocked(
                StreamState::Backpressure,
                format!(
                    "worker {} queue is saturated: {}/{}",
                    self.endpoint.label(),
                    self.queued_requests,
                    self.queue_limit
                ),
            );
        }

        if self.busy {
            let reason = self
                .active_request
                .as_deref()
                .filter(|request| !request.is_empty())
                .map(|request| format!("worker {} is busy: {request}", self.endpoint.label()))
                .unwrap_or_else(|| format!("worker {} is busy", self.endpoint.label()));
            return GateDecision::blocked(StreamState::Busy, reason);
        }

        GateDecision::Allowed
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelPoolGateSnapshot {
    pub frontend: FrontendGateSnapshot,
    pub workers: Vec<ModelWorkerSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPoolStatus {
    pub total_workers: usize,
    pub available_workers: usize,
    pub busy_workers: usize,
    pub saturated_workers: usize,
    pub queued_requests: usize,
    pub queue_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPoolRouteStatus {
    pub matching_workers: usize,
    pub matching_available_workers: usize,
    pub matching_busy_workers: usize,
    pub matching_saturated_workers: usize,
    pub matching_queued_requests: usize,
    pub matching_queue_limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRouteWorkerPickerAction {
    Current,
    Select,
    Wait,
    RepairGate,
    Unavailable,
}

impl ModelRouteWorkerPickerAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Select => "select",
            Self::Wait => "wait",
            Self::RepairGate => "repair_gate",
            Self::Unavailable => "unavailable",
        }
    }

    fn for_row(
        endpoint_selected: bool,
        route_match: bool,
        selectable: bool,
        decision: &GateDecision,
    ) -> Self {
        if endpoint_selected {
            return Self::Current;
        }

        if selectable {
            return Self::Select;
        }

        let advice = decision.advice();
        if matches!(advice.action, GateAdviceAction::RepairGate) {
            return Self::RepairGate;
        }

        if route_match && advice.state.is_pressure() {
            return Self::Wait;
        }

        Self::Unavailable
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRouteWorkerSnapshot {
    pub worker: ModelWorkerSnapshot,
    pub endpoint_selected: bool,
    pub route_match: bool,
    pub selectable: bool,
    pub picker_action: ModelRouteWorkerPickerAction,
    pub picker_action_label: String,
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
    pub decision: GateDecision,
}

impl ModelRouteWorkerSnapshot {
    pub fn endpoint_label(&self) -> &str {
        self.worker.endpoint_label()
    }

    pub fn worker_status_label(&self) -> &'static str {
        self.worker.status_label()
    }

    pub fn worker_status_state(&self) -> StreamState {
        self.worker.status_state()
    }

    pub fn worker_status_state_label(&self) -> &'static str {
        self.worker.status_state_label()
    }

    pub fn worker_status_is_available(&self) -> bool {
        self.worker.is_available()
    }

    pub fn worker_status_is_pressure(&self) -> bool {
        self.worker.status_is_pressure()
    }

    pub fn worker_status_blocks_prompt_submit(&self) -> bool {
        self.worker.status_blocks_prompt_submit()
    }

    pub fn worker_status_display_snapshot(&self) -> Option<ChatChunkDisplaySnapshot> {
        self.worker.status_display_snapshot()
    }

    pub fn decision_advice(&self) -> GateAdvice {
        self.decision.advice()
    }

    pub fn decision_action_label(&self) -> &'static str {
        self.decision.advice().action.as_str()
    }

    pub fn decision_state_label(&self) -> &'static str {
        self.decision.advice().state.as_str()
    }

    pub fn decision_state_is_terminal(&self) -> bool {
        self.decision.advice().state.is_terminal()
    }

    pub fn decision_state_is_pressure(&self) -> bool {
        self.decision.advice().state.is_pressure()
    }

    pub fn decision_state_blocks_prompt_submit(&self) -> bool {
        self.decision.advice().state.blocks_prompt_submit()
    }

    pub fn decision_display_snapshot(&self) -> Option<ChatChunkDisplaySnapshot> {
        self.decision.display_snapshot(0)
    }

    pub fn decision_reason(&self) -> String {
        self.decision.advice().reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPoolRouteSnapshot {
    pub intent: RoutingIntent,
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
    pub decision: GateDecision,
    pub decision_advice: GateAdvice,
    pub decision_action_label: String,
    pub decision_state_label: String,
    pub decision_state_is_terminal: bool,
    pub decision_state_is_pressure: bool,
    pub decision_state_blocks_prompt_submit: bool,
    pub decision_reason: String,
    pub send_allowed: bool,
    pub send_block_state: Option<StreamState>,
    pub send_block_state_label: Option<String>,
    pub send_block_state_is_terminal: bool,
    pub send_block_state_is_pressure: bool,
    pub send_block_state_blocks_prompt_submit: bool,
    pub send_block_chunk: Option<ChatChunkDisplaySnapshot>,
    pub pool: ModelPoolStatus,
    pub pool_status: String,
    pub pool_queue_label: String,
    pub pool_capacity_state: StreamState,
    pub pool_capacity_state_label: String,
    pub pool_capacity_state_is_pressure: bool,
    pub pool_capacity_state_blocks_prompt_submit: bool,
    pub route_pool: ModelPoolRouteStatus,
    pub route_pool_status: String,
    pub route_pool_queue_label: String,
    pub route_pool_capacity_state: StreamState,
    pub route_pool_capacity_state_label: String,
    pub route_pool_capacity_state_is_pressure: bool,
    pub route_pool_capacity_state_blocks_prompt_submit: bool,
    pub workers: Vec<ModelRouteWorkerSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPoolWorkersHostSnapshot {
    pub read_only: bool,
    pub launches_process: bool,
    pub sends_prompt: bool,
    pub starts_stream: bool,
    pub carries_request_preview: bool,
    pub mutates_history: bool,
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
    pub decision_action_label: String,
    pub decision_state_label: String,
    pub decision_reason: String,
    pub pool_status: String,
    pub route_pool_status: String,
    pub workers: Vec<ModelWorkerHostSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelWorkerHostSnapshot {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamWorkerWindowStatusSource {
    pub window_id: String,
    pub lane_label: String,
    pub paused: bool,
    pub polluted: bool,
    pub archived: bool,
    pub completed_evidence_only: bool,
    pub clean_room_replacement: bool,
    pub replacement_window_id: Option<String>,
    pub reason: Option<String>,
}

impl SmartSteamWorkerWindowStatusSource {
    pub fn new(window_id: impl Into<String>, lane_label: impl Into<String>) -> Self {
        Self {
            window_id: window_id.into(),
            lane_label: lane_label.into(),
            paused: false,
            polluted: false,
            archived: false,
            completed_evidence_only: false,
            clean_room_replacement: false,
            replacement_window_id: None,
            reason: None,
        }
    }

    pub fn with_paused(mut self, reason: impl Into<String>) -> Self {
        self.paused = true;
        self.reason = Some(reason.into());
        self
    }

    pub fn with_polluted(mut self, reason: impl Into<String>) -> Self {
        self.polluted = true;
        self.reason = Some(reason.into());
        self
    }

    pub fn with_archived(mut self, reason: impl Into<String>) -> Self {
        self.archived = true;
        self.reason = Some(reason.into());
        self
    }

    pub fn with_completed_evidence_only(mut self, reason: impl Into<String>) -> Self {
        self.completed_evidence_only = true;
        self.reason = Some(reason.into());
        self
    }

    pub fn with_clean_room_replacement(mut self) -> Self {
        self.clean_room_replacement = true;
        self
    }

    pub fn with_replacement_window(mut self, window_id: impl Into<String>) -> Self {
        self.replacement_window_id = Some(window_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamWorkerWindowStatusSnapshot {
    pub window_id: String,
    pub lane_label: String,
    pub status_label: String,
    pub paused: bool,
    pub polluted: bool,
    pub archived: bool,
    pub completed_evidence_only: bool,
    pub clean_room_replacement: bool,
    pub assignment_allowed: bool,
    pub original_window_blocks_assignment: bool,
    pub clean_room_replacement_required: bool,
    pub future_work_requires_fresh_clean_room: bool,
    pub replacement_window_id: Option<String>,
    pub reason: Option<String>,
}

impl SmartSteamWorkerWindowStatusSnapshot {
    fn from_source(source: SmartSteamWorkerWindowStatusSource) -> Self {
        let blocks_assignment =
            source.paused || source.polluted || source.archived || source.completed_evidence_only;
        let clean_room_replacement_required = blocks_assignment && !source.clean_room_replacement;
        let assignment_allowed = !blocks_assignment || source.clean_room_replacement;
        let future_work_requires_fresh_clean_room =
            blocks_assignment && !source.clean_room_replacement;
        let status_label = if source.clean_room_replacement {
            "clean-room-replacement"
        } else if source.archived {
            "archived"
        } else if source.completed_evidence_only {
            "completed-evidence-only"
        } else if source.polluted {
            "polluted"
        } else if source.paused {
            "paused"
        } else {
            "running"
        };

        Self {
            window_id: source.window_id,
            lane_label: source.lane_label,
            status_label: status_label.to_owned(),
            paused: source.paused,
            polluted: source.polluted,
            archived: source.archived,
            completed_evidence_only: source.completed_evidence_only,
            clean_room_replacement: source.clean_room_replacement,
            assignment_allowed,
            original_window_blocks_assignment: blocks_assignment,
            clean_room_replacement_required,
            future_work_requires_fresh_clean_room,
            replacement_window_id: source.replacement_window_id,
            reason: source.reason,
        }
    }

    pub fn summary(&self) -> String {
        let mut parts = vec![
            format!("window={}", self.window_id),
            format!("lane={}", self.lane_label),
            format!("status={}", self.status_label),
            format!("assignment_allowed={}", self.assignment_allowed),
            format!(
                "original_window_blocks_assignment={}",
                self.original_window_blocks_assignment
            ),
            format!(
                "clean_room_replacement_required={}",
                self.clean_room_replacement_required
            ),
            format!(
                "future_work_requires_fresh_clean_room={}",
                self.future_work_requires_fresh_clean_room
            ),
        ];
        if let Some(window_id) = self.replacement_window_id.as_deref() {
            parts.push(format!("replacement={window_id}"));
        }
        if let Some(reason) = self.reason.as_deref().filter(|reason| !reason.is_empty()) {
            parts.push(format!("reason={reason}"));
        }
        parts.join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamContextHygieneStatusSnapshot {
    pub read_only: bool,
    pub report_only: bool,
    pub completed_window_evidence_non_actionable: bool,
    pub future_work_requires_fresh_clean_room: bool,
    pub reads_old_window_payload: bool,
    pub reason_codes: Vec<String>,
}

impl SmartSteamContextHygieneStatusSnapshot {
    fn from_worker_windows(windows: &[SmartSteamWorkerWindowStatusSnapshot]) -> Self {
        let completed_window_evidence_non_actionable =
            windows.iter().any(|window| window.completed_evidence_only);
        let future_work_requires_fresh_clean_room = windows
            .iter()
            .any(|window| window.future_work_requires_fresh_clean_room);
        let mut reason_codes = Vec::new();
        if completed_window_evidence_non_actionable {
            reason_codes.push("completed_worker_evidence_only".to_owned());
        }
        if future_work_requires_fresh_clean_room {
            reason_codes.push("fresh_clean_room_required".to_owned());
        }

        Self {
            read_only: true,
            report_only: true,
            completed_window_evidence_non_actionable,
            future_work_requires_fresh_clean_room,
            reads_old_window_payload: false,
            reason_codes,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "context_hygiene completed_window_evidence_non_actionable={} future_work_requires_fresh_clean_room={} reads_old_window_payload={} reason_codes={}",
            self.completed_window_evidence_non_actionable,
            self.future_work_requires_fresh_clean_room,
            self.reads_old_window_payload,
            list_or_none(&self.reason_codes),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamMemoryStartupAdmissionStatusSnapshot {
    pub read_only_contract: bool,
    pub read_only_review_required: bool,
    pub index_quality_blocker_count: usize,
    pub index_quality_warning_count: usize,
    pub index_operation_count: usize,
    pub index_refresh_count: usize,
    pub index_detail_codes: Vec<String>,
    pub context_rot_risk_count: usize,
    pub context_rot_blocker_reason_codes: Vec<String>,
    pub admission_decision_count: usize,
    pub admission_accepted_count: usize,
    pub admission_risk_rejection_count: usize,
    pub migration_live_store_targeted_count: usize,
    pub adapter_live_write_count: usize,
    pub live_write_phase_request_count: usize,
    pub live_store_mutation_requested: bool,
    pub store_mutation_count: usize,
    pub ndkv_write_allowed: bool,
    pub helper_prose_line_count: usize,
    pub non_contract_line_count: usize,
    pub admission_expanded_by_non_contract_evidence: bool,
}

impl SmartSteamMemoryStartupAdmissionStatusSnapshot {
    pub fn from_evidence(evidence: &MemoryStartupAdmissionEvidence) -> Self {
        Self {
            read_only_contract: evidence.read_only_contract_holds(),
            read_only_review_required: evidence.read_only_review_required,
            index_quality_blocker_count: evidence.index_quality_blocker_count,
            index_quality_warning_count: evidence.index_quality_warning_count,
            index_operation_count: evidence.index_operation_count,
            index_refresh_count: evidence.index_refresh_count,
            index_detail_codes: evidence.index_detail_codes.clone(),
            context_rot_risk_count: evidence.context_rot_risk_count,
            context_rot_blocker_reason_codes: evidence.context_rot_blocker_reason_codes.clone(),
            admission_decision_count: evidence.admission_decision_count,
            admission_accepted_count: evidence.admission_accepted_count,
            admission_risk_rejection_count: evidence.admission_risk_rejection_count,
            migration_live_store_targeted_count: evidence.migration_live_store_targeted_count,
            adapter_live_write_count: evidence.adapter_live_write_count,
            live_write_phase_request_count: evidence.live_write_phase_request_count,
            live_store_mutation_requested: evidence.live_store_mutation_requested(),
            store_mutation_count: evidence.store_mutation_count,
            ndkv_write_allowed: evidence.ndkv_write_allowed(),
            helper_prose_line_count: evidence.helper_prose_line_count,
            non_contract_line_count: evidence.non_contract_line_count,
            admission_expanded_by_non_contract_evidence: evidence
                .admission_expanded_by_non_contract_evidence(),
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "memory_startup_admission read_only_contract={} review={} index_blockers={} index_warnings={} index_ops={} index_refresh={} context_rot_risks={} admission_decisions={} admission_accepted={} admission_risk_rejections={} live_store_mutation_requested={} store_mutations={} ndkv_write_allowed={} helper_prose_lines={} non_contract_lines={} admission_expanded_by_non_contract={}",
            self.read_only_contract,
            self.read_only_review_required,
            self.index_quality_blocker_count,
            self.index_quality_warning_count,
            self.index_operation_count,
            self.index_refresh_count,
            self.context_rot_risk_count,
            self.admission_decision_count,
            self.admission_accepted_count,
            self.admission_risk_rejection_count,
            self.live_store_mutation_requested,
            self.store_mutation_count,
            self.ndkv_write_allowed,
            self.helper_prose_line_count,
            self.non_contract_line_count,
            self.admission_expanded_by_non_contract_evidence,
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmartSteamCleanRoomHandoffStatusSource {
    pub agent_replacement_plan_required: bool,
    pub agent_replacement_plan_available: bool,
    pub replacement_prompt_ready: bool,
    pub original_window_follow_up_assignment_allowed: bool,
    pub reads_old_window_payload: bool,
    pub starts_thread: bool,
    pub sends_message: bool,
    pub evidence_result_ids: Vec<String>,
    pub reason_codes: Vec<String>,
}

impl SmartSteamCleanRoomHandoffStatusSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_agent_replacement_plan(
        mut self,
        required: bool,
        available: bool,
        prompt_ready: bool,
    ) -> Self {
        self.agent_replacement_plan_required = required;
        self.agent_replacement_plan_available = available;
        self.replacement_prompt_ready = prompt_ready;
        self
    }

    pub fn with_original_window_follow_up_assignment_allowed(mut self, allowed: bool) -> Self {
        self.original_window_follow_up_assignment_allowed = allowed;
        self
    }

    pub fn with_old_window_payload_read(mut self, reads_old_window_payload: bool) -> Self {
        self.reads_old_window_payload = reads_old_window_payload;
        self
    }

    pub fn with_thread_side_effects(mut self, starts_thread: bool, sends_message: bool) -> Self {
        self.starts_thread = starts_thread;
        self.sends_message = sends_message;
        self
    }

    pub fn with_evidence_result_ids(
        mut self,
        ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.evidence_result_ids = ids.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_reason_codes(mut self, codes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.reason_codes = codes.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamCleanRoomHandoffStatusSnapshot {
    pub read_only: bool,
    pub report_only: bool,
    pub pure_data_only: bool,
    pub memory_admission_safe: bool,
    pub agent_replacement_plan_required: bool,
    pub agent_replacement_plan_available: bool,
    pub replacement_prompt_ready: bool,
    pub original_window_follow_up_assignment_allowed: bool,
    pub original_window_follow_up_blocked: bool,
    pub reads_old_window_payload: bool,
    pub old_window_payload_ignored: bool,
    pub starts_thread: bool,
    pub sends_message: bool,
    pub starts_clean_room_replacement: bool,
    pub mutates_worker_window_status: bool,
    pub starts_daemon: bool,
    pub stops_daemon: bool,
    pub touches_remote: bool,
    pub downloads_model: bool,
    pub warms_model_cache: bool,
    pub sends_prompt: bool,
    pub starts_stream: bool,
    pub replays_prompt: bool,
    pub live_write_allowed: bool,
    pub live_store_mutation_allowed: bool,
    pub ndkv_write_allowed: bool,
    pub runtime_side_effects_allowed: bool,
    pub evidence_result_ids: Vec<String>,
    pub reason_codes: Vec<String>,
}

impl SmartSteamCleanRoomHandoffStatusSnapshot {
    fn from_source(
        source: SmartSteamCleanRoomHandoffStatusSource,
        memory: Option<&SmartSteamMemoryStartupAdmissionStatusSnapshot>,
    ) -> Self {
        let memory_admission_safe = memory.is_some_and(|memory| {
            memory.read_only_contract
                && !memory.live_store_mutation_requested
                && memory.store_mutation_count == 0
                && !memory.ndkv_write_allowed
                && !memory.admission_expanded_by_non_contract_evidence
        });

        Self {
            read_only: true,
            report_only: true,
            pure_data_only: true,
            memory_admission_safe,
            agent_replacement_plan_required: source.agent_replacement_plan_required,
            agent_replacement_plan_available: source.agent_replacement_plan_available,
            replacement_prompt_ready: source.replacement_prompt_ready,
            original_window_follow_up_assignment_allowed: source
                .original_window_follow_up_assignment_allowed,
            original_window_follow_up_blocked: !source.original_window_follow_up_assignment_allowed,
            reads_old_window_payload: source.reads_old_window_payload,
            old_window_payload_ignored: !source.reads_old_window_payload,
            starts_thread: source.starts_thread,
            sends_message: source.sends_message,
            starts_clean_room_replacement: false,
            mutates_worker_window_status: false,
            starts_daemon: false,
            stops_daemon: false,
            touches_remote: false,
            downloads_model: false,
            warms_model_cache: false,
            sends_prompt: false,
            starts_stream: false,
            replays_prompt: false,
            live_write_allowed: false,
            live_store_mutation_allowed: false,
            ndkv_write_allowed: false,
            runtime_side_effects_allowed: false,
            evidence_result_ids: source.evidence_result_ids,
            reason_codes: source.reason_codes,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "clean_room_handoff memory_admission_safe={} agent_replacement_plan_required={} agent_replacement_plan_available={} replacement_prompt_ready={} original_window_follow_up_blocked={} reads_old_window_payload={} live_write_allowed={} live_store_mutation_allowed={} ndkv_write_allowed={} runtime_side_effects_allowed={}",
            self.memory_admission_safe,
            self.agent_replacement_plan_required,
            self.agent_replacement_plan_available,
            self.replacement_prompt_ready,
            self.original_window_follow_up_blocked,
            self.reads_old_window_payload,
            self.live_write_allowed,
            self.live_store_mutation_allowed,
            self.ndkv_write_allowed,
            self.runtime_side_effects_allowed,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartSteamHelperStageRepairState {
    Complete,
    RepairRequired,
}

impl SmartSteamHelperStageRepairState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::RepairRequired => "repair-required",
        }
    }

    pub fn contract_complete(self) -> bool {
        matches!(self, Self::Complete)
    }

    pub fn repair_required(self) -> bool {
        matches!(self, Self::RepairRequired)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamHelperStageRepairStatusSource {
    pub stage_label: String,
    pub state: SmartSteamHelperStageRepairState,
    pub source_round: Option<u64>,
    pub evidence_ids: Vec<String>,
    pub reason_codes: Vec<String>,
    pub missing_helper_role_repair_proposals:
        Vec<SmartSteamMissingHelperRoleRepairProposalStatusSource>,
}

impl SmartSteamHelperStageRepairStatusSource {
    pub fn new(stage_label: impl Into<String>, state: SmartSteamHelperStageRepairState) -> Self {
        Self {
            stage_label: stage_label.into(),
            state,
            source_round: None,
            evidence_ids: Vec::new(),
            reason_codes: Vec::new(),
            missing_helper_role_repair_proposals: Vec::new(),
        }
    }

    pub fn with_source_round(mut self, source_round: u64) -> Self {
        self.source_round = Some(source_round);
        self
    }

    pub fn with_evidence_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.evidence_ids = ids.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_reason_codes(mut self, codes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.reason_codes = codes.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_missing_helper_role_repair_proposals(
        mut self,
        proposals: impl IntoIterator<Item = SmartSteamMissingHelperRoleRepairProposalStatusSource>,
    ) -> Self {
        self.missing_helper_role_repair_proposals = proposals.into_iter().collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamMissingHelperRoleRepairProposalStatusSource {
    pub proposal_id: String,
    pub role_label: String,
    pub reason_codes: Vec<String>,
}

impl SmartSteamMissingHelperRoleRepairProposalStatusSource {
    pub fn new(proposal_id: impl Into<String>, role_label: impl Into<String>) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            role_label: role_label.into(),
            reason_codes: Vec::new(),
        }
    }

    pub fn with_reason_codes(mut self, codes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.reason_codes = codes.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamMissingHelperRoleRepairProposalStatusSnapshot {
    pub read_only: bool,
    pub report_only: bool,
    pub pure_data_only: bool,
    pub proposal_id: String,
    pub role_label: String,
    pub repair_required: bool,
    pub reason_codes: Vec<String>,
    pub parses_helper_prose: bool,
    pub replays_prompt: bool,
    pub calls_model: bool,
    pub sends_prompt: bool,
    pub starts_stream: bool,
    pub writes_memory: bool,
    pub writes_ndkv: bool,
    pub mutates_live_store: bool,
    pub starts_clean_room_replacement: bool,
    pub mutates_worker_window_status: bool,
    pub runtime_side_effects_allowed: bool,
}

impl SmartSteamMissingHelperRoleRepairProposalStatusSnapshot {
    fn from_source(source: SmartSteamMissingHelperRoleRepairProposalStatusSource) -> Self {
        Self {
            read_only: true,
            report_only: true,
            pure_data_only: true,
            proposal_id: source.proposal_id,
            role_label: source.role_label,
            repair_required: true,
            reason_codes: source.reason_codes,
            parses_helper_prose: false,
            replays_prompt: false,
            calls_model: false,
            sends_prompt: false,
            starts_stream: false,
            writes_memory: false,
            writes_ndkv: false,
            mutates_live_store: false,
            starts_clean_room_replacement: false,
            mutates_worker_window_status: false,
            runtime_side_effects_allowed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamHelperStageRepairStatusSnapshot {
    pub read_only: bool,
    pub report_only: bool,
    pub pure_data_only: bool,
    pub stage_label: String,
    pub state: SmartSteamHelperStageRepairState,
    pub state_label: String,
    pub helper_stage_contract_complete: bool,
    pub helper_stage_repair_required: bool,
    pub source_round: Option<u64>,
    pub evidence_ids: Vec<String>,
    pub reason_codes: Vec<String>,
    pub missing_helper_role_repair_required: bool,
    pub missing_helper_role_repair_proposal_count: usize,
    pub missing_helper_roles: Vec<String>,
    pub missing_helper_role_repair_proposals:
        Vec<SmartSteamMissingHelperRoleRepairProposalStatusSnapshot>,
    pub parses_helper_prose: bool,
    pub replays_prompt: bool,
    pub calls_model: bool,
    pub sends_prompt: bool,
    pub starts_stream: bool,
    pub writes_memory: bool,
    pub writes_ndkv: bool,
    pub mutates_live_store: bool,
    pub starts_clean_room_replacement: bool,
    pub mutates_worker_window_status: bool,
    pub runtime_side_effects_allowed: bool,
}

impl SmartSteamHelperStageRepairStatusSnapshot {
    fn from_source(source: SmartSteamHelperStageRepairStatusSource) -> Self {
        let missing_helper_role_repair_proposals = source
            .missing_helper_role_repair_proposals
            .into_iter()
            .map(SmartSteamMissingHelperRoleRepairProposalStatusSnapshot::from_source)
            .collect::<Vec<_>>();
        let missing_helper_roles =
            missing_helper_role_labels(&missing_helper_role_repair_proposals);
        let missing_helper_role_repair_proposal_count = missing_helper_role_repair_proposals.len();
        Self {
            read_only: true,
            report_only: true,
            pure_data_only: true,
            stage_label: source.stage_label,
            state: source.state,
            state_label: source.state.as_str().to_owned(),
            helper_stage_contract_complete: source.state.contract_complete(),
            helper_stage_repair_required: source.state.repair_required(),
            source_round: source.source_round,
            evidence_ids: source.evidence_ids,
            reason_codes: source.reason_codes,
            missing_helper_role_repair_required: missing_helper_role_repair_proposal_count > 0,
            missing_helper_role_repair_proposal_count,
            missing_helper_roles,
            missing_helper_role_repair_proposals,
            parses_helper_prose: false,
            replays_prompt: false,
            calls_model: false,
            sends_prompt: false,
            starts_stream: false,
            writes_memory: false,
            writes_ndkv: false,
            mutates_live_store: false,
            starts_clean_room_replacement: false,
            mutates_worker_window_status: false,
            runtime_side_effects_allowed: false,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "helper_stage_repair stage={} state={} contract_complete={} repair_required={} missing_helper_role_repair_required={} missing_helper_role_repair_proposals={} missing_helper_roles={} reasons={} evidence={} parses_helper_prose={} writes_memory={} writes_ndkv={} runtime_side_effects_allowed={}",
            self.stage_label,
            self.state_label,
            self.helper_stage_contract_complete,
            self.helper_stage_repair_required,
            self.missing_helper_role_repair_required,
            self.missing_helper_role_repair_proposal_count,
            list_or_none(&self.missing_helper_roles),
            self.reason_codes.len(),
            self.evidence_ids.len(),
            self.parses_helper_prose,
            self.writes_memory,
            self.writes_ndkv,
            self.runtime_side_effects_allowed,
        )
    }
}

fn missing_helper_role_labels(
    proposals: &[SmartSteamMissingHelperRoleRepairProposalStatusSnapshot],
) -> Vec<String> {
    let mut labels = Vec::new();
    for proposal in proposals {
        if !labels
            .iter()
            .any(|existing| existing == &proposal.role_label)
        {
            labels.push(proposal.role_label.clone());
        }
    }
    labels
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartSteamSelfImproveProposalLifecycle {
    Candidate,
    Validated,
    Admitted,
    Quarantined,
    Promoted,
    RepairRequired,
}

impl SmartSteamSelfImproveProposalLifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Validated => "validated",
            Self::Admitted => "admitted",
            Self::Quarantined => "quarantined",
            Self::Promoted => "promoted",
            Self::RepairRequired => "repair-required",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamSelfImproveProposalValidationStatusSource {
    pub checked: bool,
    pub passed: bool,
    pub status_code: Option<i32>,
    pub evidence_ids: Vec<String>,
}

impl SmartSteamSelfImproveProposalValidationStatusSource {
    pub fn new(checked: bool, passed: bool) -> Self {
        Self {
            checked,
            passed,
            status_code: None,
            evidence_ids: Vec::new(),
        }
    }

    pub fn with_status_code(mut self, status_code: i32) -> Self {
        self.status_code = Some(status_code);
        self
    }

    pub fn with_evidence_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.evidence_ids = ids.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamSelfImproveProposalMemoryAdmissionStatusSource {
    pub checked: bool,
    pub admitted: bool,
    pub quarantined: bool,
    pub evidence_ids: Vec<String>,
}

impl SmartSteamSelfImproveProposalMemoryAdmissionStatusSource {
    pub fn new(checked: bool, admitted: bool, quarantined: bool) -> Self {
        Self {
            checked,
            admitted,
            quarantined,
            evidence_ids: Vec::new(),
        }
    }

    pub fn with_evidence_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.evidence_ids = ids.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamSelfImproveProposalStatusSource {
    pub proposal_id: String,
    pub lifecycle: SmartSteamSelfImproveProposalLifecycle,
    pub source_round: Option<u64>,
    pub evidence_ids: Vec<String>,
    pub validation_status: SmartSteamSelfImproveProposalValidationStatusSource,
    pub memory_admission_status: SmartSteamSelfImproveProposalMemoryAdmissionStatusSource,
}

impl SmartSteamSelfImproveProposalStatusSource {
    pub fn new(
        proposal_id: impl Into<String>,
        lifecycle: SmartSteamSelfImproveProposalLifecycle,
    ) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            lifecycle,
            source_round: None,
            evidence_ids: Vec::new(),
            validation_status: SmartSteamSelfImproveProposalValidationStatusSource::new(
                false, false,
            ),
            memory_admission_status: SmartSteamSelfImproveProposalMemoryAdmissionStatusSource::new(
                false, false, false,
            ),
        }
    }

    pub fn with_source_round(mut self, source_round: u64) -> Self {
        self.source_round = Some(source_round);
        self
    }

    pub fn with_evidence_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.evidence_ids = ids.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_validation_status(
        mut self,
        status: SmartSteamSelfImproveProposalValidationStatusSource,
    ) -> Self {
        self.validation_status = status;
        self
    }

    pub fn with_memory_admission_status(
        mut self,
        status: SmartSteamSelfImproveProposalMemoryAdmissionStatusSource,
    ) -> Self {
        self.memory_admission_status = status;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamSelfImproveProposalPromptGuidanceSource {
    pub convert_advisory_to_business_evidence: bool,
    pub repair_unvalidated_or_unaccepted: bool,
    pub requires_validation_and_memory_admission: bool,
    pub evidence_ids: Vec<String>,
}

impl SmartSteamSelfImproveProposalPromptGuidanceSource {
    pub fn new(
        convert_advisory_to_business_evidence: bool,
        repair_unvalidated_or_unaccepted: bool,
        requires_validation_and_memory_admission: bool,
    ) -> Self {
        Self {
            convert_advisory_to_business_evidence,
            repair_unvalidated_or_unaccepted,
            requires_validation_and_memory_admission,
            evidence_ids: Vec::new(),
        }
    }

    pub fn with_evidence_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.evidence_ids = ids.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamSelfImproveProposalValidationStatusSnapshot {
    pub checked: bool,
    pub passed: bool,
    pub status_label: String,
    pub status_code: Option<i32>,
    pub evidence_ids: Vec<String>,
}

impl SmartSteamSelfImproveProposalValidationStatusSnapshot {
    fn from_source(source: SmartSteamSelfImproveProposalValidationStatusSource) -> Self {
        let status_label = if source.checked {
            if source.passed { "passed" } else { "failed" }
        } else {
            "not-checked"
        };
        Self {
            checked: source.checked,
            passed: source.passed,
            status_label: status_label.to_owned(),
            status_code: source.status_code,
            evidence_ids: source.evidence_ids,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamSelfImproveProposalMemoryAdmissionStatusSnapshot {
    pub checked: bool,
    pub admitted: bool,
    pub quarantined: bool,
    pub status_label: String,
    pub evidence_ids: Vec<String>,
}

impl SmartSteamSelfImproveProposalMemoryAdmissionStatusSnapshot {
    fn from_source(source: SmartSteamSelfImproveProposalMemoryAdmissionStatusSource) -> Self {
        let status_label = if source.quarantined {
            "quarantined"
        } else if source.admitted {
            "admitted"
        } else if source.checked {
            "rejected"
        } else {
            "not-checked"
        };
        Self {
            checked: source.checked,
            admitted: source.admitted,
            quarantined: source.quarantined,
            status_label: status_label.to_owned(),
            evidence_ids: source.evidence_ids,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamSelfImproveProposalSnapshot {
    pub proposal_id: String,
    pub lifecycle: SmartSteamSelfImproveProposalLifecycle,
    pub lifecycle_label: String,
    pub source_round: Option<u64>,
    pub evidence_ids: Vec<String>,
    pub validation_status: SmartSteamSelfImproveProposalValidationStatusSnapshot,
    pub memory_admission_status: SmartSteamSelfImproveProposalMemoryAdmissionStatusSnapshot,
    pub read_only: bool,
    pub report_only: bool,
    pub pure_data_only: bool,
    pub parses_helper_prose: bool,
    pub replays_prompt: bool,
    pub calls_model: bool,
    pub sends_prompt: bool,
    pub starts_stream: bool,
    pub writes_memory: bool,
    pub writes_ndkv: bool,
    pub mutates_live_store: bool,
    pub promotes_runtime: bool,
    pub quarantines_runtime: bool,
    pub runtime_side_effects_allowed: bool,
}

impl SmartSteamSelfImproveProposalSnapshot {
    fn from_source(source: SmartSteamSelfImproveProposalStatusSource) -> Self {
        Self {
            proposal_id: source.proposal_id,
            lifecycle: source.lifecycle,
            lifecycle_label: source.lifecycle.as_str().to_owned(),
            source_round: source.source_round,
            evidence_ids: source.evidence_ids,
            validation_status: SmartSteamSelfImproveProposalValidationStatusSnapshot::from_source(
                source.validation_status,
            ),
            memory_admission_status:
                SmartSteamSelfImproveProposalMemoryAdmissionStatusSnapshot::from_source(
                    source.memory_admission_status,
                ),
            read_only: true,
            report_only: true,
            pure_data_only: true,
            parses_helper_prose: false,
            replays_prompt: false,
            calls_model: false,
            sends_prompt: false,
            starts_stream: false,
            writes_memory: false,
            writes_ndkv: false,
            mutates_live_store: false,
            promotes_runtime: false,
            quarantines_runtime: false,
            runtime_side_effects_allowed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamSelfImproveProposalPromptGuidanceSnapshot {
    pub read_only: bool,
    pub report_only: bool,
    pub pure_data_only: bool,
    pub convert_advisory_to_business_evidence: bool,
    pub repair_unvalidated_or_unaccepted: bool,
    pub requires_validation_and_memory_admission: bool,
    pub evidence_ids: Vec<String>,
    pub parses_helper_prose: bool,
    pub replays_prompt: bool,
    pub calls_model: bool,
    pub sends_prompt: bool,
    pub starts_stream: bool,
    pub writes_memory: bool,
    pub writes_ndkv: bool,
    pub mutates_live_store: bool,
    pub runtime_side_effects_allowed: bool,
}

impl SmartSteamSelfImproveProposalPromptGuidanceSnapshot {
    fn from_source(source: SmartSteamSelfImproveProposalPromptGuidanceSource) -> Self {
        Self {
            read_only: true,
            report_only: true,
            pure_data_only: true,
            convert_advisory_to_business_evidence: source.convert_advisory_to_business_evidence,
            repair_unvalidated_or_unaccepted: source.repair_unvalidated_or_unaccepted,
            requires_validation_and_memory_admission: source
                .requires_validation_and_memory_admission,
            evidence_ids: source.evidence_ids,
            parses_helper_prose: false,
            replays_prompt: false,
            calls_model: false,
            sends_prompt: false,
            starts_stream: false,
            writes_memory: false,
            writes_ndkv: false,
            mutates_live_store: false,
            runtime_side_effects_allowed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamSelfImproveProposalStatusSnapshot {
    pub read_only: bool,
    pub report_only: bool,
    pub pure_data_only: bool,
    pub proposal_count: usize,
    pub candidate_count: usize,
    pub validated_count: usize,
    pub admitted_count: usize,
    pub quarantined_count: usize,
    pub promoted_count: usize,
    pub repair_required_count: usize,
    pub parses_helper_prose: bool,
    pub replays_prompt: bool,
    pub calls_model: bool,
    pub sends_prompt: bool,
    pub starts_stream: bool,
    pub writes_memory: bool,
    pub writes_ndkv: bool,
    pub mutates_live_store: bool,
    pub runtime_side_effects_allowed: bool,
    pub prompt_guidance: Option<SmartSteamSelfImproveProposalPromptGuidanceSnapshot>,
    pub proposals: Vec<SmartSteamSelfImproveProposalSnapshot>,
}

impl SmartSteamSelfImproveProposalStatusSnapshot {
    fn from_sources_and_guidance(
        sources: impl IntoIterator<Item = SmartSteamSelfImproveProposalStatusSource>,
        prompt_guidance: Option<SmartSteamSelfImproveProposalPromptGuidanceSource>,
    ) -> Self {
        let proposals = sources
            .into_iter()
            .map(SmartSteamSelfImproveProposalSnapshot::from_source)
            .collect::<Vec<_>>();
        let prompt_guidance =
            prompt_guidance.map(SmartSteamSelfImproveProposalPromptGuidanceSnapshot::from_source);
        Self {
            read_only: true,
            report_only: true,
            pure_data_only: true,
            proposal_count: proposals.len(),
            candidate_count: lifecycle_count(
                &proposals,
                SmartSteamSelfImproveProposalLifecycle::Candidate,
            ),
            validated_count: lifecycle_count(
                &proposals,
                SmartSteamSelfImproveProposalLifecycle::Validated,
            ),
            admitted_count: lifecycle_count(
                &proposals,
                SmartSteamSelfImproveProposalLifecycle::Admitted,
            ),
            quarantined_count: lifecycle_count(
                &proposals,
                SmartSteamSelfImproveProposalLifecycle::Quarantined,
            ),
            promoted_count: lifecycle_count(
                &proposals,
                SmartSteamSelfImproveProposalLifecycle::Promoted,
            ),
            repair_required_count: lifecycle_count(
                &proposals,
                SmartSteamSelfImproveProposalLifecycle::RepairRequired,
            ),
            parses_helper_prose: false,
            replays_prompt: false,
            calls_model: false,
            sends_prompt: false,
            starts_stream: false,
            writes_memory: false,
            writes_ndkv: false,
            mutates_live_store: false,
            runtime_side_effects_allowed: false,
            prompt_guidance,
            proposals,
        }
    }

    pub fn summary(&self) -> String {
        let mut summary = format!(
            "self_improve_proposals total={} candidate={} validated={} admitted={} quarantined={} promoted={} repair-required={} writes_memory={} writes_ndkv={} runtime_side_effects_allowed={}",
            self.proposal_count,
            self.candidate_count,
            self.validated_count,
            self.admitted_count,
            self.quarantined_count,
            self.promoted_count,
            self.repair_required_count,
            self.writes_memory,
            self.writes_ndkv,
            self.runtime_side_effects_allowed,
        );
        if let Some(guidance) = self.prompt_guidance.as_ref() {
            summary.push_str(&format!(
                " convert_advisory_to_business_evidence={} repair_unvalidated_or_unaccepted={} requires_validation_and_memory_admission={}",
                guidance.convert_advisory_to_business_evidence,
                guidance.repair_unvalidated_or_unaccepted,
                guidance.requires_validation_and_memory_admission,
            ));
        }
        summary
    }
}

fn lifecycle_count(
    proposals: &[SmartSteamSelfImproveProposalSnapshot],
    lifecycle: SmartSteamSelfImproveProposalLifecycle,
) -> usize {
    proposals
        .iter()
        .filter(|proposal| proposal.lifecycle == lifecycle)
        .count()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamDaemonRoundTransitionStatusSource {
    pub observed_round_done: bool,
    pub done_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub round_in_progress: bool,
    pub ledger_round: Option<u64>,
    pub ledger_commit_pending: bool,
    pub evidence_ids: Vec<String>,
    pub reason_codes: Vec<String>,
}

impl SmartSteamDaemonRoundTransitionStatusSource {
    pub fn round_done_ledger_pending(done_round: u64, ledger_round: Option<u64>) -> Self {
        Self {
            observed_round_done: true,
            done_round: Some(done_round),
            latest_done_round: Some(done_round),
            round_in_progress: false,
            ledger_round,
            ledger_commit_pending: true,
            evidence_ids: Vec::new(),
            reason_codes: vec!["round_done_ledger_commit_pending".to_owned()],
        }
    }

    pub fn with_evidence_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.evidence_ids = ids.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_reason_codes(mut self, codes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.reason_codes = codes.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_round_in_progress(mut self, round_in_progress: bool) -> Self {
        self.round_in_progress = round_in_progress;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamDaemonRoundTransitionStatusSnapshot {
    pub read_only: bool,
    pub report_only: bool,
    pub observed_round_done: bool,
    pub done_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub round_in_progress: bool,
    pub ledger_round: Option<u64>,
    pub ledger_commit_pending: bool,
    pub ledger_lag_rounds: Option<u64>,
    pub status_label: String,
    pub evidence_ids: Vec<String>,
    pub reason_codes: Vec<String>,
    pub starts_daemon: bool,
    pub stops_daemon: bool,
    pub touches_remote: bool,
    pub sends_prompt: bool,
    pub starts_stream: bool,
    pub replays_prompt: bool,
    pub mutates_active_round: bool,
    pub writes_ndkv: bool,
}

impl SmartSteamDaemonRoundTransitionStatusSnapshot {
    fn from_source(source: SmartSteamDaemonRoundTransitionStatusSource) -> Self {
        let latest_done_round = source.latest_done_round.or(source.done_round);
        let ledger_lag_rounds = match (latest_done_round, source.ledger_round) {
            (Some(done), Some(ledger)) => done.checked_sub(ledger),
            _ => None,
        };
        let status_label = if source.ledger_commit_pending {
            "round-done-ledger-commit-pending"
        } else if source.observed_round_done {
            "round-done"
        } else {
            "observing"
        };

        Self {
            read_only: true,
            report_only: true,
            observed_round_done: source.observed_round_done,
            done_round: source.done_round,
            latest_done_round,
            round_in_progress: source.round_in_progress,
            ledger_round: source.ledger_round,
            ledger_commit_pending: source.ledger_commit_pending,
            ledger_lag_rounds,
            status_label: status_label.to_owned(),
            evidence_ids: source.evidence_ids,
            reason_codes: source.reason_codes,
            starts_daemon: false,
            stops_daemon: false,
            touches_remote: false,
            sends_prompt: false,
            starts_stream: false,
            replays_prompt: false,
            mutates_active_round: false,
            writes_ndkv: false,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "daemon_round_transition status={} observed_round_done={} done_round={} latest_done_round={} round_in_progress={} ledger_round={} ledger_commit_pending={} ledger_lag_rounds={} starts_daemon={} stops_daemon={} sends_prompt={} starts_stream={} writes_ndkv={}",
            self.status_label,
            self.observed_round_done,
            optional_u64(self.done_round),
            optional_u64(self.latest_done_round),
            self.round_in_progress,
            optional_u64(self.ledger_round),
            self.ledger_commit_pending,
            optional_u64(self.ledger_lag_rounds),
            self.starts_daemon,
            self.stops_daemon,
            self.sends_prompt,
            self.starts_stream,
            self.writes_ndkv,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamNextRoundDecisionStatusSource {
    pub safe_to_wait_current_round_active: bool,
    pub safe_to_continue_after_current_round: bool,
    pub operator_attention_blocked: bool,
    pub current_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub evidence_ids: Vec<String>,
    pub reason_codes: Vec<String>,
    pub downstream_status_consumers: Option<SmartSteamNextRoundDownstreamConsumerStatusSource>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmartSteamNextRoundDownstreamConsumerStatusSource {
    pub source_decision_status: Option<String>,
    pub effective_decision_status: Option<String>,
    pub service_cli_display_status: Option<String>,
    pub forge_operator_display_status: Option<String>,
    pub agent_assignment_acceptance: Option<String>,
    pub memory_self_improve_admission_visibility: Option<String>,
    pub operator_attention_required: Option<bool>,
    pub read_only: Option<bool>,
    pub report_only: Option<bool>,
    pub no_side_effects: Option<bool>,
    pub dispatch_work_allowed: Option<bool>,
    pub prompt_replay_allowed: Option<bool>,
    pub process_start_allowed: Option<bool>,
    pub memory_write_allowed: Option<bool>,
    pub ndkv_write_allowed: Option<bool>,
    pub current_round_active: Option<bool>,
    pub live_status_display_state: Option<String>,
    pub active_round: Option<u64>,
    pub ledger_latest_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub readiness_can_schedule_next_round: Option<bool>,
    pub round_id_evidence: Option<SmartSteamNextRoundRoundIdEvidenceSource>,
    pub failure_reasons: Vec<String>,
}

impl SmartSteamNextRoundDownstreamConsumerStatusSource {
    fn is_display_only_contract(&self) -> bool {
        self.read_only == Some(true)
            && self.report_only == Some(true)
            && self.no_side_effects == Some(true)
            && self.dispatch_work_allowed == Some(false)
            && self.prompt_replay_allowed == Some(false)
            && self.process_start_allowed == Some(false)
            && self.memory_write_allowed == Some(false)
            && self.ndkv_write_allowed == Some(false)
            && self.source_decision_status.is_some()
            && self.effective_decision_status.is_some()
            && self.service_cli_display_status.is_some()
            && self.forge_operator_display_status.is_some()
            && self.agent_assignment_acceptance.is_some()
            && self.memory_self_improve_admission_visibility.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmartSteamNextRoundRoundIdEvidenceSource {
    pub source_schema: Option<String>,
    pub active_round: Option<u64>,
    pub ledger_latest_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub transition_kind: Option<String>,
    pub transition_status_label: Option<String>,
    pub ledger_commit_pending: Option<bool>,
    pub round_in_progress: Option<bool>,
    pub evidence_ids: Vec<String>,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmartSteamNextRoundDecisionReportStatusSource {
    pub decision_status: Option<String>,
    pub display_state: Option<String>,
    pub live_status_display_state: Option<String>,
    pub current_round_active: Option<bool>,
    pub readiness_can_schedule_next_round: Option<bool>,
    pub report_gate_ready: Option<bool>,
    pub context_hygiene_passed: Option<bool>,
    pub read_only: Option<bool>,
    pub report_only: Option<bool>,
    pub no_side_effects: Option<bool>,
    pub dispatch_work_allowed: Option<bool>,
    pub prompt_replay_allowed: Option<bool>,
    pub process_start_allowed: Option<bool>,
    pub memory_write_allowed: Option<bool>,
    pub ndkv_write_allowed: Option<bool>,
    pub operator_attention_required: Option<bool>,
    pub current_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub evidence_ids: Vec<String>,
    pub reason_codes: Vec<String>,
    pub failure_reasons: Vec<String>,
    pub downstream_status_consumers: Option<SmartSteamNextRoundDownstreamConsumerStatusSource>,
}

impl SmartSteamNextRoundDecisionReportStatusSource {
    pub fn into_status_source(self) -> Option<SmartSteamNextRoundDecisionStatusSource> {
        if !self.is_display_only_contract() {
            return None;
        }

        let labels = [
            self.decision_status.as_deref(),
            self.display_state.as_deref(),
            self.live_status_display_state.as_deref(),
        ];
        let label_matches = |needles: &[&str]| {
            labels.iter().flatten().any(|label| {
                let normalized = normalize_decision_label(label);
                needles.iter().any(|needle| normalized.contains(needle))
            })
        };

        let operator_attention_blocked = self.operator_attention_required.unwrap_or(false)
            || label_matches(&[
                "operator-attention",
                "operator_attention",
                "blocked",
                "attention-required",
                "attention_required",
            ]);
        let explicitly_safe_to_continue = label_matches(&["safe-to-continue", "safe_to_continue"]);
        let safe_to_continue_after_current_round = !operator_attention_blocked
            && (explicitly_safe_to_continue
                || (self.readiness_can_schedule_next_round == Some(true)
                    && self.report_gate_ready.unwrap_or(true)
                    && self.context_hygiene_passed.unwrap_or(true)));
        let safe_to_wait_current_round_active = !operator_attention_blocked
            && !safe_to_continue_after_current_round
            && (label_matches(&["safe-to-wait", "safe_to_wait"])
                || self.current_round_active == Some(true));

        if !operator_attention_blocked
            && !safe_to_wait_current_round_active
            && !safe_to_continue_after_current_round
        {
            return None;
        }

        let default_reason = if operator_attention_blocked {
            "operator_attention_blocked"
        } else if safe_to_continue_after_current_round {
            "safe_to_continue_after_current_round"
        } else {
            "safe_to_wait_current_round_active"
        };
        let mut reason_codes = self.reason_codes;
        reason_codes.extend(self.failure_reasons);
        if reason_codes.is_empty() {
            reason_codes.push(default_reason.to_owned());
        }

        Some(SmartSteamNextRoundDecisionStatusSource {
            safe_to_wait_current_round_active,
            safe_to_continue_after_current_round,
            operator_attention_blocked,
            current_round: self.current_round,
            latest_done_round: self.latest_done_round,
            evidence_ids: self.evidence_ids,
            reason_codes,
            downstream_status_consumers: self.downstream_status_consumers.filter(
                SmartSteamNextRoundDownstreamConsumerStatusSource::is_display_only_contract,
            ),
        })
    }

    fn is_display_only_contract(&self) -> bool {
        self.read_only != Some(false)
            && self.report_only != Some(false)
            && self.no_side_effects != Some(false)
            && self.dispatch_work_allowed != Some(true)
            && self.prompt_replay_allowed != Some(true)
            && self.process_start_allowed != Some(true)
            && self.memory_write_allowed != Some(true)
            && self.ndkv_write_allowed != Some(true)
    }
}

impl SmartSteamNextRoundDecisionStatusSource {
    pub fn safe_to_wait_current_round_active(current_round: u64, latest_done_round: u64) -> Self {
        Self {
            safe_to_wait_current_round_active: true,
            safe_to_continue_after_current_round: false,
            operator_attention_blocked: false,
            current_round: Some(current_round),
            latest_done_round: Some(latest_done_round),
            evidence_ids: Vec::new(),
            reason_codes: vec!["safe_to_wait_current_round_active".to_owned()],
            downstream_status_consumers: None,
        }
    }

    pub fn safe_to_continue_after_current_round(
        current_round: u64,
        latest_done_round: u64,
    ) -> Self {
        Self {
            safe_to_wait_current_round_active: false,
            safe_to_continue_after_current_round: true,
            operator_attention_blocked: false,
            current_round: Some(current_round),
            latest_done_round: Some(latest_done_round),
            evidence_ids: Vec::new(),
            reason_codes: vec!["safe_to_continue_after_current_round".to_owned()],
            downstream_status_consumers: None,
        }
    }

    pub fn operator_attention_blocked(
        current_round: Option<u64>,
        latest_done_round: Option<u64>,
    ) -> Self {
        Self {
            safe_to_wait_current_round_active: false,
            safe_to_continue_after_current_round: false,
            operator_attention_blocked: true,
            current_round,
            latest_done_round,
            evidence_ids: Vec::new(),
            reason_codes: vec!["operator_attention_blocked".to_owned()],
            downstream_status_consumers: None,
        }
    }

    pub fn with_evidence_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.evidence_ids = ids.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_reason_codes(mut self, codes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.reason_codes = codes.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamNextRoundDownstreamConsumerStatusSnapshot {
    pub schema_version: String,
    pub source_decision_status: String,
    pub effective_decision_status: String,
    pub service_cli_display_status: String,
    pub forge_operator_display_status: String,
    pub agent_assignment_acceptance: String,
    pub memory_self_improve_admission_visibility: String,
    pub operator_attention_required: bool,
    pub read_only: bool,
    pub report_only: bool,
    pub no_side_effects: bool,
    pub dispatch_work_allowed: bool,
    pub prompt_replay_allowed: bool,
    pub process_start_allowed: bool,
    pub memory_write_allowed: bool,
    pub ndkv_write_allowed: bool,
    pub current_round_active: Option<bool>,
    pub live_status_display_state: Option<String>,
    pub active_round: Option<u64>,
    pub ledger_latest_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub readiness_can_schedule_next_round: Option<bool>,
    pub round_id_evidence: Option<SmartSteamNextRoundRoundIdEvidenceSnapshot>,
    pub failure_reasons: Vec<String>,
}

impl SmartSteamNextRoundDownstreamConsumerStatusSnapshot {
    fn from_source(source: SmartSteamNextRoundDownstreamConsumerStatusSource) -> Option<Self> {
        if !source.is_display_only_contract() {
            return None;
        }

        Some(Self {
            schema_version: "next_round_downstream_status_consumers_v1".to_owned(),
            source_decision_status: source.source_decision_status?,
            effective_decision_status: source.effective_decision_status?,
            service_cli_display_status: source.service_cli_display_status?,
            forge_operator_display_status: source.forge_operator_display_status?,
            agent_assignment_acceptance: source.agent_assignment_acceptance?,
            memory_self_improve_admission_visibility: source
                .memory_self_improve_admission_visibility?,
            operator_attention_required: source.operator_attention_required.unwrap_or(false),
            read_only: true,
            report_only: true,
            no_side_effects: true,
            dispatch_work_allowed: false,
            prompt_replay_allowed: false,
            process_start_allowed: false,
            memory_write_allowed: false,
            ndkv_write_allowed: false,
            current_round_active: source.current_round_active,
            live_status_display_state: source.live_status_display_state,
            active_round: source.active_round,
            ledger_latest_round: source.ledger_latest_round,
            latest_done_round: source.latest_done_round,
            readiness_can_schedule_next_round: source.readiness_can_schedule_next_round,
            round_id_evidence: source
                .round_id_evidence
                .and_then(SmartSteamNextRoundRoundIdEvidenceSnapshot::from_source),
            failure_reasons: source.failure_reasons,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamNextRoundRoundIdEvidenceSnapshot {
    pub source_schema: Option<String>,
    pub active_round: Option<u64>,
    pub ledger_latest_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub transition_kind: Option<String>,
    pub transition_status_label: Option<String>,
    pub ledger_commit_pending: Option<bool>,
    pub round_in_progress: Option<bool>,
    pub evidence_ids: Vec<String>,
    pub reason_codes: Vec<String>,
}

impl SmartSteamNextRoundRoundIdEvidenceSnapshot {
    fn from_source(source: SmartSteamNextRoundRoundIdEvidenceSource) -> Option<Self> {
        if source.source_schema.is_none()
            && source.active_round.is_none()
            && source.ledger_latest_round.is_none()
            && source.latest_done_round.is_none()
            && source.transition_kind.is_none()
            && source.transition_status_label.is_none()
            && source.ledger_commit_pending.is_none()
            && source.round_in_progress.is_none()
            && source.evidence_ids.is_empty()
            && source.reason_codes.is_empty()
        {
            return None;
        }

        Some(Self {
            source_schema: source.source_schema,
            active_round: source.active_round,
            ledger_latest_round: source.ledger_latest_round,
            latest_done_round: source.latest_done_round,
            transition_kind: source.transition_kind,
            transition_status_label: source.transition_status_label,
            ledger_commit_pending: source.ledger_commit_pending,
            round_in_progress: source.round_in_progress,
            evidence_ids: source.evidence_ids,
            reason_codes: source.reason_codes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamNextRoundDecisionStatusSnapshot {
    pub report_version: String,
    pub read_only: bool,
    pub report_only: bool,
    pub safe_to_wait_current_round_active: bool,
    pub safe_to_continue_after_current_round: bool,
    pub operator_attention_blocked: bool,
    pub current_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub status_label: String,
    pub evidence_ids: Vec<String>,
    pub reason_codes: Vec<String>,
    pub starts_daemon: bool,
    pub stops_daemon: bool,
    pub touches_remote: bool,
    pub sends_prompt: bool,
    pub starts_stream: bool,
    pub replays_prompt: bool,
    pub mutates_active_round: bool,
    pub mutates_worker_window_status: bool,
    pub writes_ndkv: bool,
    pub creates_thread: bool,
    pub operator_action_required: bool,
    pub downstream_status_consumers: Option<SmartSteamNextRoundDownstreamConsumerStatusSnapshot>,
}

impl SmartSteamNextRoundDecisionStatusSnapshot {
    fn from_source(source: SmartSteamNextRoundDecisionStatusSource) -> Self {
        let status_label = if source.operator_attention_blocked {
            "operator-attention-blocked"
        } else if source.safe_to_continue_after_current_round {
            "safe-to-continue-after-current-round"
        } else if source.safe_to_wait_current_round_active {
            "safe-to-wait-current-round-active"
        } else {
            "unknown"
        };

        Self {
            report_version: "next_round_decision_report_v1".to_owned(),
            read_only: true,
            report_only: true,
            safe_to_wait_current_round_active: source.safe_to_wait_current_round_active,
            safe_to_continue_after_current_round: source.safe_to_continue_after_current_round,
            operator_attention_blocked: source.operator_attention_blocked,
            current_round: source.current_round,
            latest_done_round: source.latest_done_round,
            status_label: status_label.to_owned(),
            evidence_ids: source.evidence_ids,
            reason_codes: source.reason_codes,
            starts_daemon: false,
            stops_daemon: false,
            touches_remote: false,
            sends_prompt: false,
            starts_stream: false,
            replays_prompt: false,
            mutates_active_round: false,
            mutates_worker_window_status: false,
            writes_ndkv: false,
            creates_thread: false,
            operator_action_required: source.operator_attention_blocked,
            downstream_status_consumers: source
                .downstream_status_consumers
                .and_then(SmartSteamNextRoundDownstreamConsumerStatusSnapshot::from_source),
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "next_round_decision status={} safe_to_wait_current_round_active={} safe_to_continue_after_current_round={} operator_attention_blocked={} current_round={} latest_done_round={} starts_daemon={} sends_prompt={} starts_stream={} writes_ndkv={} creates_thread={}",
            self.status_label,
            self.safe_to_wait_current_round_active,
            self.safe_to_continue_after_current_round,
            self.operator_attention_blocked,
            optional_u64(self.current_round),
            optional_u64(self.latest_done_round),
            self.starts_daemon,
            self.sends_prompt,
            self.starts_stream,
            self.writes_ndkv,
            self.creates_thread,
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmartSteamStatusSource {
    pub daemon_running: bool,
    pub daemon_pid: Option<u32>,
    pub supervisor_running: bool,
    pub supervisor_check_only: bool,
    pub active_round: Option<u64>,
    pub ledger_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub round_in_progress: Option<bool>,
    pub readiness_ok: bool,
    pub remote_chain_ready: bool,
    pub model_cache_label: Option<String>,
    pub worker_windows: Vec<SmartSteamWorkerWindowStatusSource>,
    pub memory_startup_admission: Option<MemoryStartupAdmissionEvidence>,
    pub clean_room_handoff: Option<SmartSteamCleanRoomHandoffStatusSource>,
    pub helper_stage_repair: Option<SmartSteamHelperStageRepairStatusSource>,
    pub self_improve_proposals: Vec<SmartSteamSelfImproveProposalStatusSource>,
    pub self_improve_proposal_prompt_guidance:
        Option<SmartSteamSelfImproveProposalPromptGuidanceSource>,
    pub daemon_round_transition: Option<SmartSteamDaemonRoundTransitionStatusSource>,
    pub next_round_decision: Option<SmartSteamNextRoundDecisionStatusSource>,
}

impl SmartSteamStatusSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_daemon(
        mut self,
        running: bool,
        pid: Option<u32>,
        active_round: Option<u64>,
        ledger_round: Option<u64>,
    ) -> Self {
        self.daemon_running = running;
        self.daemon_pid = pid;
        self.active_round = active_round;
        self.ledger_round = ledger_round;
        self
    }

    pub fn with_daemon_round_progress(
        mut self,
        latest_done_round: Option<u64>,
        round_in_progress: bool,
    ) -> Self {
        self.latest_done_round = latest_done_round;
        self.round_in_progress = Some(round_in_progress);
        self
    }

    pub fn with_supervisor(mut self, running: bool, check_only: bool) -> Self {
        self.supervisor_running = running;
        self.supervisor_check_only = check_only;
        self
    }

    pub fn with_readiness(mut self, readiness_ok: bool, remote_chain_ready: bool) -> Self {
        self.readiness_ok = readiness_ok;
        self.remote_chain_ready = remote_chain_ready;
        self
    }

    pub fn with_model_cache_label(mut self, label: impl Into<String>) -> Self {
        self.model_cache_label = Some(label.into());
        self
    }

    pub fn with_worker_window(mut self, window: SmartSteamWorkerWindowStatusSource) -> Self {
        self.worker_windows.push(window);
        self
    }

    pub fn with_worker_windows(
        mut self,
        windows: impl IntoIterator<Item = SmartSteamWorkerWindowStatusSource>,
    ) -> Self {
        self.worker_windows.extend(windows);
        self
    }

    pub fn with_memory_startup_admission(
        mut self,
        evidence: MemoryStartupAdmissionEvidence,
    ) -> Self {
        self.memory_startup_admission = Some(evidence);
        self
    }

    pub fn with_clean_room_handoff(
        mut self,
        handoff: SmartSteamCleanRoomHandoffStatusSource,
    ) -> Self {
        self.clean_room_handoff = Some(handoff);
        self
    }

    pub fn with_helper_stage_repair(
        mut self,
        repair: SmartSteamHelperStageRepairStatusSource,
    ) -> Self {
        self.helper_stage_repair = Some(repair);
        self
    }

    pub fn with_self_improve_proposal(
        mut self,
        proposal: SmartSteamSelfImproveProposalStatusSource,
    ) -> Self {
        self.self_improve_proposals.push(proposal);
        self
    }

    pub fn with_self_improve_proposals(
        mut self,
        proposals: impl IntoIterator<Item = SmartSteamSelfImproveProposalStatusSource>,
    ) -> Self {
        self.self_improve_proposals.extend(proposals);
        self
    }

    pub fn with_self_improve_proposal_prompt_guidance(
        mut self,
        guidance: SmartSteamSelfImproveProposalPromptGuidanceSource,
    ) -> Self {
        self.self_improve_proposal_prompt_guidance = Some(guidance);
        self
    }

    pub fn with_daemon_round_transition(
        mut self,
        transition: SmartSteamDaemonRoundTransitionStatusSource,
    ) -> Self {
        self.daemon_round_transition = Some(transition);
        self
    }

    pub fn with_next_round_decision(
        mut self,
        decision: SmartSteamNextRoundDecisionStatusSource,
    ) -> Self {
        self.next_round_decision = Some(decision);
        self
    }

    pub fn with_next_round_decision_report(
        mut self,
        report: SmartSteamNextRoundDecisionReportStatusSource,
    ) -> Self {
        self.next_round_decision = report.into_status_source();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamStatusSnapshot {
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
    pub daemon_running: bool,
    pub daemon_pid: Option<u32>,
    pub supervisor_running: bool,
    pub supervisor_check_only: bool,
    pub active_round: Option<u64>,
    pub ledger_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub round_in_progress: bool,
    pub readiness_ok: bool,
    pub engine_busy: bool,
    pub active_request: Option<String>,
    pub remote_chain_ready: bool,
    pub model_cache_label: Option<String>,
    pub worker_windows_total: usize,
    pub worker_windows_paused: usize,
    pub worker_windows_polluted: usize,
    pub worker_windows_clean_room_replacements_required: usize,
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
    pub workers_total: usize,
    pub workers_available: usize,
    pub workers_busy: usize,
    pub workers_saturated: usize,
    pub pool_status: String,
    pub route_pool_status: Option<String>,
    pub route_send_allowed: Option<bool>,
    pub route_send_block_reason: Option<String>,
}

impl SmartSteamStatusSnapshot {
    pub fn from_model_pool(
        source: SmartSteamStatusSource,
        gate: &ModelPoolGateSnapshot,
        intent: Option<&RoutingIntent>,
    ) -> Self {
        let pool = gate.status();
        let route = intent.map(|intent| gate.route_snapshot(intent));
        let route_send_block_reason = route.as_ref().and_then(|route| {
            route
                .send_block_state
                .map(|_| route.decision_reason.clone())
        });
        let worker_windows = source
            .worker_windows
            .into_iter()
            .map(SmartSteamWorkerWindowStatusSnapshot::from_source)
            .collect::<Vec<_>>();
        let worker_windows_total = worker_windows.len();
        let worker_windows_paused = worker_windows.iter().filter(|window| window.paused).count();
        let worker_windows_polluted = worker_windows
            .iter()
            .filter(|window| window.polluted)
            .count();
        let worker_windows_clean_room_replacements_required = worker_windows
            .iter()
            .filter(|window| window.clean_room_replacement_required)
            .count();
        let clean_room_replacement_required = worker_windows_clean_room_replacements_required > 0;
        let worker_window_status = smartsteam_worker_window_status(
            worker_windows_total,
            worker_windows_paused,
            worker_windows_polluted,
            worker_windows_clean_room_replacements_required,
        );
        let context_hygiene_status =
            SmartSteamContextHygieneStatusSnapshot::from_worker_windows(&worker_windows);
        let context_hygiene_summary = context_hygiene_status.summary();
        let memory_startup_admission_status = source
            .memory_startup_admission
            .as_ref()
            .map(SmartSteamMemoryStartupAdmissionStatusSnapshot::from_evidence);
        let memory_startup_admission_summary = memory_startup_admission_status
            .as_ref()
            .map(SmartSteamMemoryStartupAdmissionStatusSnapshot::summary);
        let clean_room_handoff_status = source.clean_room_handoff.map(|handoff| {
            SmartSteamCleanRoomHandoffStatusSnapshot::from_source(
                handoff,
                memory_startup_admission_status.as_ref(),
            )
        });
        let clean_room_handoff_summary = clean_room_handoff_status
            .as_ref()
            .map(SmartSteamCleanRoomHandoffStatusSnapshot::summary);
        let helper_stage_repair_status = source
            .helper_stage_repair
            .map(SmartSteamHelperStageRepairStatusSnapshot::from_source);
        let helper_stage_repair_summary = helper_stage_repair_status
            .as_ref()
            .map(SmartSteamHelperStageRepairStatusSnapshot::summary);
        let self_improve_proposal_status = (!source.self_improve_proposals.is_empty()
            || source.self_improve_proposal_prompt_guidance.is_some())
        .then(|| {
            SmartSteamSelfImproveProposalStatusSnapshot::from_sources_and_guidance(
                source.self_improve_proposals,
                source.self_improve_proposal_prompt_guidance,
            )
        });
        let self_improve_proposal_summary = self_improve_proposal_status
            .as_ref()
            .map(SmartSteamSelfImproveProposalStatusSnapshot::summary);
        let daemon_round_transition_status = source
            .daemon_round_transition
            .map(SmartSteamDaemonRoundTransitionStatusSnapshot::from_source);
        let daemon_round_transition_summary = daemon_round_transition_status
            .as_ref()
            .map(SmartSteamDaemonRoundTransitionStatusSnapshot::summary);
        let next_round_decision_status = source
            .next_round_decision
            .map(SmartSteamNextRoundDecisionStatusSnapshot::from_source);
        let next_round_decision_summary = next_round_decision_status
            .as_ref()
            .map(SmartSteamNextRoundDecisionStatusSnapshot::summary);
        let latest_done_round = daemon_round_transition_status
            .as_ref()
            .and_then(|transition| transition.latest_done_round)
            .or(source.latest_done_round)
            .or(source.ledger_round);
        let transition_marks_round_done = daemon_round_transition_status
            .as_ref()
            .is_some_and(|transition| transition.observed_round_done);
        let round_in_progress = source.round_in_progress.unwrap_or_else(|| {
            !transition_marks_round_done
                && match (source.active_round, latest_done_round) {
                    (Some(active), Some(done)) => active > done,
                    (Some(_), None) => true,
                    _ => false,
                }
        });

        Self {
            read_only: true,
            launches_process: false,
            starts_daemon: false,
            stops_daemon: false,
            touches_remote: false,
            downloads_model: false,
            warms_model_cache: false,
            sends_prompt: false,
            starts_stream: false,
            replays_prompt: false,
            mutates_busy: false,
            mutates_readiness: false,
            mutates_active_round: false,
            starts_clean_room_replacement: false,
            mutates_worker_window_status: false,
            daemon_running: source.daemon_running,
            daemon_pid: source.daemon_pid,
            supervisor_running: source.supervisor_running,
            supervisor_check_only: source.supervisor_check_only,
            active_round: source.active_round,
            ledger_round: source.ledger_round,
            latest_done_round,
            round_in_progress,
            readiness_ok: source.readiness_ok,
            engine_busy: gate.frontend.engine_busy,
            active_request: gate.frontend.active_request.clone(),
            remote_chain_ready: source.remote_chain_ready,
            model_cache_label: source.model_cache_label,
            worker_windows_total,
            worker_windows_paused,
            worker_windows_polluted,
            worker_windows_clean_room_replacements_required,
            clean_room_replacement_required,
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
            workers_total: pool.total_workers,
            workers_available: pool.available_workers,
            workers_busy: pool.busy_workers,
            workers_saturated: pool.saturated_workers,
            pool_status: pool.summary(),
            route_pool_status: route.as_ref().map(|route| route.route_pool_status.clone()),
            route_send_allowed: route.as_ref().map(|route| route.send_allowed),
            route_send_block_reason,
        }
    }

    pub fn status_line(&self) -> String {
        let mut line = format!(
            "daemon_running={} active_round={} latest_done_round={} round_in_progress={} ledger_round={} readiness_ok={} engine_busy={} remote_chain_ready={} pool={}",
            self.daemon_running,
            optional_u64(self.active_round),
            optional_u64(self.latest_done_round),
            self.round_in_progress,
            optional_u64(self.ledger_round),
            self.readiness_ok,
            self.engine_busy,
            self.remote_chain_ready,
            self.pool_status
        );
        if self.clean_room_replacement_required {
            line.push_str(" worker_windows=");
            line.push_str(&self.worker_window_status);
        }
        if let Some(memory_summary) = self.memory_startup_admission_summary.as_deref() {
            line.push(' ');
            line.push_str(memory_summary);
        }
        if let Some(handoff_summary) = self.clean_room_handoff_summary.as_deref() {
            line.push(' ');
            line.push_str(handoff_summary);
        }
        if let Some(helper_stage_summary) = self.helper_stage_repair_summary.as_deref() {
            line.push(' ');
            line.push_str(helper_stage_summary);
        }
        if let Some(proposal_summary) = self.self_improve_proposal_summary.as_deref() {
            line.push(' ');
            line.push_str(proposal_summary);
        }
        if let Some(transition_summary) = self.daemon_round_transition_summary.as_deref() {
            line.push(' ');
            line.push_str(transition_summary);
        }
        if let Some(next_round_summary) = self.next_round_decision_summary.as_deref() {
            line.push(' ');
            line.push_str(next_round_summary);
        }
        line
    }
}

fn smartsteam_worker_window_status(
    total: usize,
    paused: usize,
    polluted: usize,
    clean_room_replacements_required: usize,
) -> String {
    format!(
        "windows total={total} running={} paused={paused} polluted={polluted} clean_room_replacements_required={clean_room_replacements_required}",
        total.saturating_sub(clean_room_replacements_required)
    )
}

fn normalize_decision_label(label: &str) -> String {
    label.trim().to_ascii_lowercase()
}

impl ModelPoolRouteSnapshot {
    pub fn workers_host_snapshot(&self) -> ModelPoolWorkersHostSnapshot {
        ModelPoolWorkersHostSnapshot {
            read_only: true,
            launches_process: false,
            sends_prompt: false,
            starts_stream: false,
            carries_request_preview: false,
            mutates_history: false,
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
            send_block_reason: self.send_block_state.map(|_| self.decision_reason.clone()),
            decision_action_label: self.decision_action_label.clone(),
            decision_state_label: self.decision_state_label.clone(),
            decision_reason: self.decision_reason.clone(),
            pool_status: self.pool_status.clone(),
            route_pool_status: self.route_pool_status.clone(),
            workers: self
                .workers
                .iter()
                .map(|worker| ModelWorkerHostSnapshot {
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
        }
    }
}

impl ModelPoolStatus {
    pub fn has_workers(&self) -> bool {
        self.total_workers > 0
    }

    pub fn has_available_workers(&self) -> bool {
        self.available_workers > 0
    }

    pub fn has_busy_workers(&self) -> bool {
        self.busy_workers > 0
    }

    pub fn has_saturated_workers(&self) -> bool {
        self.saturated_workers > 0
    }

    pub fn has_queued_requests(&self) -> bool {
        self.queued_requests > 0
    }

    pub fn queue_is_saturated(&self) -> bool {
        self.queue_limit > 0 && self.queued_requests >= self.queue_limit
    }

    pub fn queue_label(&self) -> String {
        format!("{}/{}", self.queued_requests, self.queue_limit)
    }

    pub fn capacity_state(&self) -> StreamState {
        if self.queue_is_saturated()
            || (self.has_workers() && !self.has_available_workers() && self.has_saturated_workers())
        {
            StreamState::Backpressure
        } else if self.has_queued_requests() {
            StreamState::Queued
        } else if self.has_workers() && !self.has_available_workers() && self.has_busy_workers() {
            StreamState::Busy
        } else {
            StreamState::Pending
        }
    }

    pub fn capacity_state_label(&self) -> &'static str {
        self.capacity_state().as_str()
    }

    pub fn capacity_state_is_pressure(&self) -> bool {
        self.capacity_state().is_pressure()
    }

    pub fn capacity_state_blocks_prompt_submit(&self) -> bool {
        self.capacity_state().blocks_prompt_submit()
    }

    pub fn summary(&self) -> String {
        format!(
            "workers total={} available={} busy={} saturated={}",
            self.total_workers, self.available_workers, self.busy_workers, self.saturated_workers
        )
    }
}

impl ModelPoolRouteStatus {
    pub fn has_matching_workers(&self) -> bool {
        self.matching_workers > 0
    }

    pub fn has_matching_available_workers(&self) -> bool {
        self.matching_available_workers > 0
    }

    pub fn has_matching_busy_workers(&self) -> bool {
        self.matching_busy_workers > 0
    }

    pub fn has_matching_saturated_workers(&self) -> bool {
        self.matching_saturated_workers > 0
    }

    pub fn has_matching_queued_requests(&self) -> bool {
        self.matching_queued_requests > 0
    }

    pub fn matching_queue_is_saturated(&self) -> bool {
        self.matching_queue_limit > 0 && self.matching_queued_requests >= self.matching_queue_limit
    }

    pub fn queue_label(&self) -> String {
        format!(
            "{}/{}",
            self.matching_queued_requests, self.matching_queue_limit
        )
    }

    pub fn capacity_state(&self) -> StreamState {
        if !self.has_matching_workers() {
            StreamState::Queued
        } else if self.matching_queue_is_saturated()
            || (!self.has_matching_available_workers() && self.has_matching_saturated_workers())
        {
            StreamState::Backpressure
        } else if self.has_matching_queued_requests() {
            StreamState::Queued
        } else if !self.has_matching_available_workers() && self.has_matching_busy_workers() {
            StreamState::Busy
        } else {
            StreamState::Pending
        }
    }

    pub fn capacity_state_label(&self) -> &'static str {
        self.capacity_state().as_str()
    }

    pub fn capacity_state_is_pressure(&self) -> bool {
        self.capacity_state().is_pressure()
    }

    pub fn capacity_state_blocks_prompt_submit(&self) -> bool {
        self.capacity_state().blocks_prompt_submit()
    }

    pub fn summary(&self) -> String {
        format!(
            "matching total={} available={} busy={} saturated={}",
            self.matching_workers,
            self.matching_available_workers,
            self.matching_busy_workers,
            self.matching_saturated_workers
        )
    }
}

impl ModelPoolGateSnapshot {
    pub fn new(frontend: FrontendGateSnapshot, workers: Vec<ModelWorkerSnapshot>) -> Self {
        Self { frontend, workers }
    }

    pub fn decision_for_intent(&self, intent: &RoutingIntent) -> GateDecision {
        let frontend_decision = self.frontend.decision();
        if !frontend_decision.is_allowed() {
            return frontend_decision;
        }

        let Some(endpoint) = pinned_endpoint(intent) else {
            return self.auto_route_decision(intent);
        };

        self.worker_for(endpoint)
            .map(|worker| {
                if worker.accepts_intent(intent) {
                    worker.decision()
                } else {
                    GateDecision::blocked(
                        StreamState::Queued,
                        format!(
                            "worker {} does not match role={} preference={}",
                            worker.endpoint.label(),
                            intent.model_role.as_str(),
                            intent.routing_preference.as_str()
                        ),
                    )
                }
            })
            .unwrap_or_else(|| {
                GateDecision::blocked(
                    StreamState::Queued,
                    format!("worker {} is not registered", endpoint.label()),
                )
            })
    }

    pub fn worker_for(&self, endpoint: &ModelEndpoint) -> Option<&ModelWorkerSnapshot> {
        self.workers.iter().find(|worker| {
            worker
                .endpoint
                .label()
                .eq_ignore_ascii_case(endpoint.label())
        })
    }

    pub fn status(&self) -> ModelPoolStatus {
        let total_workers = self.workers.len();
        let busy_workers = self.workers.iter().filter(|worker| worker.busy).count();
        let saturated_workers = self
            .workers
            .iter()
            .filter(|worker| worker.is_saturated())
            .count();
        let available_workers = self
            .workers
            .iter()
            .filter(|worker| !worker.busy && !worker.is_saturated())
            .count();
        let queued_requests = self
            .workers
            .iter()
            .map(|worker| worker.queued_requests)
            .sum();
        let queue_limit = self.workers.iter().map(|worker| worker.queue_limit).sum();

        ModelPoolStatus {
            total_workers,
            available_workers,
            busy_workers,
            saturated_workers,
            queued_requests,
            queue_limit,
        }
    }

    pub fn route_status(&self, intent: &RoutingIntent) -> ModelPoolRouteStatus {
        let matching_workers = self
            .workers
            .iter()
            .filter(|worker| {
                pinned_endpoint(intent).is_none_or(|endpoint| {
                    worker
                        .endpoint
                        .label()
                        .eq_ignore_ascii_case(endpoint.label())
                })
            })
            .filter(|worker| worker.accepts_intent(intent))
            .collect::<Vec<_>>();
        let matching_available_workers = matching_workers
            .iter()
            .filter(|worker| !worker.busy && !worker.is_saturated())
            .count();
        let matching_busy_workers = matching_workers.iter().filter(|worker| worker.busy).count();
        let matching_saturated_workers = matching_workers
            .iter()
            .filter(|worker| worker.is_saturated())
            .count();
        let matching_queued_requests = matching_workers
            .iter()
            .map(|worker| worker.queued_requests)
            .sum();
        let matching_queue_limit = matching_workers
            .iter()
            .map(|worker| worker.queue_limit)
            .sum();

        ModelPoolRouteStatus {
            matching_workers: matching_workers.len(),
            matching_available_workers,
            matching_busy_workers,
            matching_saturated_workers,
            matching_queued_requests,
            matching_queue_limit,
        }
    }

    pub fn route_workers(&self, intent: &RoutingIntent) -> Vec<ModelRouteWorkerSnapshot> {
        let frontend_decision = self.frontend.decision();
        let frontend_allows_send = frontend_decision.is_allowed();
        self.workers
            .iter()
            .map(|worker| {
                let endpoint_selected = pinned_endpoint(intent).is_some_and(|endpoint| {
                    worker
                        .endpoint
                        .label()
                        .eq_ignore_ascii_case(endpoint.label())
                });
                let endpoint_matches = !intent.endpoint_pinned || endpoint_selected;
                let capability_matches = worker.accepts_intent(intent);
                let route_match = endpoint_matches && capability_matches;
                let worker_decision = if capability_matches {
                    worker.decision()
                } else {
                    GateDecision::blocked(
                        StreamState::Queued,
                        format!(
                            "worker {} does not match role={} preference={}",
                            worker.endpoint.label(),
                            intent.model_role.as_str(),
                            intent.routing_preference.as_str()
                        ),
                    )
                };
                let decision = if frontend_allows_send {
                    worker_decision
                } else {
                    frontend_decision.clone()
                };
                let selectable = frontend_allows_send
                    && capability_matches
                    && !worker.busy
                    && !worker.is_saturated();
                let picker_action = ModelRouteWorkerPickerAction::for_row(
                    endpoint_selected,
                    route_match,
                    selectable,
                    &decision,
                );
                let selection_intent = RoutingIntent::operator_pinned(
                    intent.model_role,
                    intent.routing_preference,
                    worker.endpoint.clone(),
                );
                let selection_endpoint_kind = selection_intent.endpoint_kind();
                let selection_wire = selection_intent.wire_snapshot();

                ModelRouteWorkerSnapshot {
                    worker: worker.clone(),
                    endpoint_selected,
                    route_match,
                    selectable,
                    picker_action,
                    picker_action_label: picker_action.as_str().to_owned(),
                    selection_summary: selection_intent.summary(),
                    selection_model_role_label: selection_intent.model_role_label().to_owned(),
                    selection_routing_preference_label: selection_intent
                        .routing_preference_label()
                        .to_owned(),
                    selection_endpoint_label: selection_intent.endpoint_label().to_owned(),
                    selection_endpoint_kind,
                    selection_endpoint_kind_label: selection_intent
                        .endpoint_kind_label()
                        .to_owned(),
                    selection_endpoint_auto: selection_intent.endpoint_auto(),
                    selection_endpoint_built_in: selection_intent.endpoint_built_in(),
                    selection_endpoint_custom: selection_intent.endpoint_custom(),
                    selection_wire_model_role_label: selection_wire.model_role_label,
                    selection_wire_routing_preference_label: selection_wire
                        .routing_preference_label,
                    selection_wire_prefer_fast: selection_wire.prefer_fast,
                    selection_wire_prefer_quality: selection_wire.prefer_quality,
                    selection_wire_endpoint_pinned: selection_wire.endpoint_pinned,
                    selection_wire_endpoint_kind_label: selection_wire.endpoint_kind_label,
                    selection_wire_sends_model_endpoint: selection_wire.sends_model_endpoint,
                    selection_wire_model_endpoint_label: selection_wire.model_endpoint_label,
                    selection_intent,
                    decision,
                }
            })
            .collect()
    }

    pub fn route_snapshot(&self, intent: &RoutingIntent) -> ModelPoolRouteSnapshot {
        let decision = self.decision_for_intent(intent);
        let send_block_state = match &decision {
            GateDecision::Allowed => None,
            GateDecision::Blocked { state, .. } => Some(*state),
        };
        let decision_advice = decision.advice();
        let pool = self.status();
        let pool_status = pool.summary();
        let pool_queue_label = pool.queue_label();
        let pool_capacity_state = pool.capacity_state();
        let route_pool = self.route_status(intent);
        let route_pool_status = route_pool.summary();
        let route_pool_queue_label = route_pool.queue_label();
        let route_pool_capacity_state = route_pool.capacity_state();
        let endpoint_kind = intent.endpoint_kind();
        let wire = intent.wire_snapshot();
        let send_block_chunk = decision.display_snapshot(0);
        ModelPoolRouteSnapshot {
            intent: intent.clone(),
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
            decision,
            decision_action_label: decision_advice.action.as_str().to_owned(),
            decision_state_label: decision_advice.state.as_str().to_owned(),
            decision_state_is_terminal: decision_advice.state.is_terminal(),
            decision_state_is_pressure: decision_advice.state.is_pressure(),
            decision_state_blocks_prompt_submit: decision_advice.state.blocks_prompt_submit(),
            decision_reason: decision_advice.reason.clone(),
            send_allowed: send_block_state.is_none(),
            send_block_state,
            send_block_state_label: send_block_state.map(|state| state.as_str().to_owned()),
            send_block_state_is_terminal: send_block_state.is_some_and(StreamState::is_terminal),
            send_block_state_is_pressure: send_block_state.is_some_and(StreamState::is_pressure),
            send_block_state_blocks_prompt_submit: send_block_state
                .is_some_and(StreamState::blocks_prompt_submit),
            send_block_chunk,
            decision_advice,
            pool,
            pool_status,
            pool_queue_label,
            pool_capacity_state,
            pool_capacity_state_label: pool_capacity_state.as_str().to_owned(),
            pool_capacity_state_is_pressure: pool_capacity_state.is_pressure(),
            pool_capacity_state_blocks_prompt_submit: pool_capacity_state.blocks_prompt_submit(),
            route_pool,
            route_pool_status,
            route_pool_queue_label,
            route_pool_capacity_state,
            route_pool_capacity_state_label: route_pool_capacity_state.as_str().to_owned(),
            route_pool_capacity_state_is_pressure: route_pool_capacity_state.is_pressure(),
            route_pool_capacity_state_blocks_prompt_submit: route_pool_capacity_state
                .blocks_prompt_submit(),
            workers: self.route_workers(intent),
        }
    }

    pub fn has_capability_declarations(&self) -> bool {
        self.workers
            .iter()
            .any(|worker| !worker.roles.is_empty() || !worker.preferences.is_empty())
    }

    pub fn worker_status_lines(&self) -> Vec<String> {
        if self.workers.is_empty() {
            return vec!["workers none registered".to_owned()];
        }

        self.workers
            .iter()
            .map(ModelWorkerSnapshot::summary)
            .collect()
    }

    fn auto_route_decision(&self, intent: &RoutingIntent) -> GateDecision {
        if self.workers.is_empty() {
            return GateDecision::Allowed;
        }

        let matching_workers = self
            .workers
            .iter()
            .filter(|worker| worker.accepts_intent(intent))
            .collect::<Vec<_>>();
        if matching_workers.is_empty() {
            return GateDecision::blocked(
                StreamState::Queued,
                format!(
                    "no model worker matches role={} preference={}",
                    intent.model_role.as_str(),
                    intent.routing_preference.as_str()
                ),
            );
        }

        self.auto_route_decision_for_matching_workers(matching_workers)
    }

    fn auto_route_decision_for_matching_workers(
        &self,
        matching_workers: Vec<&ModelWorkerSnapshot>,
    ) -> GateDecision {
        if matching_workers.is_empty() {
            return GateDecision::Allowed;
        }

        if matching_workers
            .iter()
            .any(|worker| !worker.busy && !worker.is_saturated())
        {
            return GateDecision::Allowed;
        }

        if matching_workers.iter().all(|worker| worker.is_saturated()) {
            return GateDecision::blocked(
                StreamState::Backpressure,
                if matching_workers.len() == self.workers.len() {
                    format!("model pool is saturated: {} workers", self.workers.len())
                } else {
                    format!(
                        "matching model workers are saturated: {} workers",
                        matching_workers.len()
                    )
                },
            );
        }

        GateDecision::blocked(
            StreamState::Queued,
            if matching_workers.len() == self.workers.len() {
                format!(
                    "all model workers are busy; waiting for scheduler across {} workers",
                    self.workers.len()
                )
            } else {
                format!(
                    "all matching model workers are busy; waiting for scheduler across {} workers",
                    matching_workers.len()
                )
            },
        )
    }
}

fn pinned_endpoint(intent: &RoutingIntent) -> Option<&ModelEndpoint> {
    intent
        .endpoint_pinned
        .then_some(intent.model_endpoint.as_ref())
        .flatten()
}

fn labels<'a>(labels: impl Iterator<Item = &'a str>) -> String {
    labels.collect::<Vec<_>>().join("|")
}

fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    Allowed,
    Blocked { state: StreamState, reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateAdviceAction {
    SendNow,
    WaitForWorker,
    WaitForCurrentStream,
    RetryLater,
    RepairGate,
}

impl GateAdviceAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SendNow => "send_now",
            Self::WaitForWorker => "wait_for_worker",
            Self::WaitForCurrentStream => "wait_for_current_stream",
            Self::RetryLater => "retry_later",
            Self::RepairGate => "repair_gate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateAdvice {
    pub action: GateAdviceAction,
    pub state: StreamState,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateSendControl {
    pub decision: GateDecision,
    pub advice: GateAdvice,
    pub action: GateAdviceAction,
    pub action_label: String,
    pub state: StreamState,
    pub state_label: String,
    pub state_is_terminal: bool,
    pub state_is_pressure: bool,
    pub state_blocks_prompt_submit: bool,
    pub block_chunk: Option<ChatChunkDisplaySnapshot>,
    pub reason: String,
    pub prompt_present: bool,
    pub send_allowed: bool,
    pub primary_action_label: String,
    pub primary_action_enabled: bool,
    pub primary_action_disabled_reason: Option<String>,
    pub preserves_prompt: bool,
    pub clears_prompt: bool,
}

impl GateAdvice {
    pub fn action_label(&self) -> &'static str {
        self.action.as_str()
    }

    pub fn state_label(&self) -> &'static str {
        self.state.as_str()
    }

    pub fn status_line(&self) -> String {
        format!(
            "{} {}: {}",
            self.action_label(),
            self.state_label(),
            self.reason
        )
    }
}

impl GateDecision {
    pub fn blocked(state: StreamState, reason: impl Into<String>) -> Self {
        Self::Blocked {
            state,
            reason: reason.into(),
        }
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }

    pub fn advice(&self) -> GateAdvice {
        match self {
            Self::Allowed => GateAdvice {
                action: GateAdviceAction::SendNow,
                state: StreamState::Pending,
                reason: "ready to send".to_owned(),
            },
            Self::Blocked { state, reason } => GateAdvice {
                action: advice_action_for_state(*state),
                state: *state,
                reason: reason.clone(),
            },
        }
    }

    pub fn send_control(&self, prompt_present: bool) -> GateSendControl {
        let advice = self.advice();
        let send_allowed = prompt_present && self.is_allowed();
        let block_chunk = (prompt_present && !send_allowed)
            .then(|| self.display_snapshot(0))
            .flatten();
        let primary_action_label = if !prompt_present {
            "type_prompt".to_owned()
        } else if send_allowed {
            "send".to_owned()
        } else {
            advice.action.as_str().to_owned()
        };
        let primary_action_enabled = send_allowed;
        let primary_action_disabled_reason = if !prompt_present {
            Some("empty input".to_owned())
        } else if send_allowed {
            None
        } else {
            Some(advice.reason.clone())
        };

        GateSendControl {
            decision: self.clone(),
            action: advice.action,
            action_label: advice.action.as_str().to_owned(),
            state: advice.state,
            state_label: advice.state.as_str().to_owned(),
            state_is_terminal: advice.state.is_terminal(),
            state_is_pressure: advice.state.is_pressure(),
            state_blocks_prompt_submit: advice.state.blocks_prompt_submit(),
            block_chunk,
            reason: advice.reason.clone(),
            prompt_present,
            send_allowed,
            primary_action_label,
            primary_action_enabled,
            primary_action_disabled_reason,
            preserves_prompt: prompt_present && !send_allowed,
            clears_prompt: send_allowed,
            advice,
        }
    }

    pub fn to_chunk(&self, sequence: u64) -> Option<ChatChunk> {
        let Self::Blocked { state, reason } = self else {
            return None;
        };
        Some(match state {
            StreamState::Queued => ChatChunk::queued(sequence, reason),
            StreamState::Busy => ChatChunk::busy(sequence, reason),
            StreamState::Backpressure => ChatChunk::backpressure(sequence, reason),
            StreamState::Failed => ChatChunk::failed(sequence, reason),
            _ => ChatChunk::status(sequence, reason),
        })
    }

    pub fn display_snapshot(&self, sequence: u64) -> Option<ChatChunkDisplaySnapshot> {
        self.to_chunk(sequence)
            .map(|chunk| chunk.display_snapshot())
    }
}

fn advice_action_for_state(state: StreamState) -> GateAdviceAction {
    match state {
        StreamState::Queued => GateAdviceAction::WaitForWorker,
        StreamState::Busy => GateAdviceAction::WaitForCurrentStream,
        StreamState::Backpressure => GateAdviceAction::RetryLater,
        StreamState::Failed => GateAdviceAction::RepairGate,
        StreamState::Pending
        | StreamState::Streaming
        | StreamState::Completed
        | StreamState::Interrupted => GateAdviceAction::RetryLater,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChatMessage, ChatRequest, ModelRole, RoutingPreference};

    #[test]
    fn default_gate_snapshot_allows_send() {
        assert_eq!(
            FrontendGateSnapshot::default().decision(),
            GateDecision::Allowed
        );
        assert!(FrontendGateSnapshot::default().decision().is_allowed());
    }

    #[test]
    fn engine_busy_maps_to_busy_stream_state() {
        let snapshot = FrontendGateSnapshot {
            engine_busy: true,
            active_request: Some("#42 chat-stream 1200ms".to_owned()),
            ..FrontendGateSnapshot::default()
        };

        assert_eq!(
            snapshot.decision(),
            GateDecision::blocked(
                StreamState::Busy,
                "backend engine is busy: #42 chat-stream 1200ms"
            )
        );
    }

    #[test]
    fn saturated_queue_maps_to_backpressure() {
        let snapshot = FrontendGateSnapshot {
            queued_requests: 4,
            queue_limit: 4,
            ..FrontendGateSnapshot::default()
        };

        assert_eq!(
            snapshot.decision(),
            GateDecision::blocked(StreamState::Backpressure, "model queue is saturated: 4/4")
        );
    }

    #[test]
    fn safety_and_experience_gates_block_send() {
        let safe_device = FrontendGateSnapshot {
            safe_device_ok: false,
            ..FrontendGateSnapshot::default()
        };
        let hygiene = FrontendGateSnapshot {
            experience_hygiene_ok: false,
            ..FrontendGateSnapshot::default()
        };

        assert_eq!(
            safe_device.decision(),
            GateDecision::blocked(StreamState::Failed, "safe-device gate failed")
        );
        assert_eq!(
            hygiene.decision(),
            GateDecision::blocked(StreamState::Failed, "experience hygiene gate failed")
        );
    }

    #[test]
    fn safety_and_experience_gates_take_precedence_over_pressure_states() {
        let unsafe_busy = FrontendGateSnapshot {
            engine_busy: true,
            safe_device_ok: false,
            active_request: Some("#42 chat-stream".to_owned()),
            ..FrontendGateSnapshot::default()
        };
        let unhygienic_backpressure = FrontendGateSnapshot {
            experience_hygiene_ok: false,
            queued_requests: 8,
            queue_limit: 8,
            ..FrontendGateSnapshot::default()
        };

        assert_eq!(
            unsafe_busy.decision(),
            GateDecision::blocked(StreamState::Failed, "safe-device gate failed")
        );
        let unsafe_busy_control = unsafe_busy.decision().send_control(true);
        assert_eq!(unsafe_busy_control.action, GateAdviceAction::RepairGate);
        assert_eq!(unsafe_busy_control.action_label, "repair_gate");
        assert_eq!(unsafe_busy_control.state, StreamState::Failed);
        assert_eq!(unsafe_busy_control.state_label, "failed");
        assert!(unsafe_busy_control.state_is_terminal);
        assert!(!unsafe_busy_control.state_is_pressure);
        assert!(!unsafe_busy_control.state_blocks_prompt_submit);
        assert_eq!(unsafe_busy_control.reason, "safe-device gate failed");
        assert!(!unsafe_busy_control.send_allowed);
        assert_eq!(unsafe_busy_control.primary_action_label, "repair_gate");
        assert_eq!(
            unsafe_busy_control
                .primary_action_disabled_reason
                .as_deref(),
            Some("safe-device gate failed")
        );
        assert!(unsafe_busy_control.preserves_prompt);
        assert!(!unsafe_busy_control.clears_prompt);
        let unsafe_busy_chunk = unsafe_busy_control
            .block_chunk
            .as_ref()
            .expect("safe-device repair gate should expose a display chunk");
        assert_eq!(unsafe_busy_chunk.output_label, "error");
        assert_eq!(
            unsafe_busy_chunk.appended,
            "[error] safe-device gate failed"
        );
        assert_eq!(
            unhygienic_backpressure.decision(),
            GateDecision::blocked(StreamState::Failed, "experience hygiene gate failed")
        );
        let hygiene_control = unhygienic_backpressure.decision().send_control(true);
        assert_eq!(hygiene_control.action, GateAdviceAction::RepairGate);
        assert_eq!(hygiene_control.action_label, "repair_gate");
        assert_eq!(hygiene_control.state, StreamState::Failed);
        assert_eq!(hygiene_control.state_label, "failed");
        assert!(hygiene_control.state_is_terminal);
        assert!(!hygiene_control.state_is_pressure);
        assert!(!hygiene_control.state_blocks_prompt_submit);
        assert_eq!(hygiene_control.reason, "experience hygiene gate failed");
        assert!(!hygiene_control.send_allowed);
        assert_eq!(hygiene_control.primary_action_label, "repair_gate");
        assert_eq!(
            hygiene_control.primary_action_disabled_reason.as_deref(),
            Some("experience hygiene gate failed")
        );
        assert!(hygiene_control.preserves_prompt);
        assert!(!hygiene_control.clears_prompt);
        let hygiene_chunk = hygiene_control
            .block_chunk
            .as_ref()
            .expect("experience repair gate should expose a display chunk");
        assert_eq!(hygiene_chunk.output_label, "error");
        assert_eq!(
            hygiene_chunk.appended,
            "[error] experience hygiene gate failed"
        );
    }

    #[test]
    fn backend_offline_takes_precedence_over_all_other_frontend_gate_failures() {
        let snapshot = FrontendGateSnapshot {
            backend_online: false,
            engine_busy: true,
            safe_device_ok: false,
            experience_hygiene_ok: false,
            queued_requests: 8,
            queue_limit: 8,
            active_request: Some("#42 chat-stream".to_owned()),
        };
        let decision = snapshot.decision();
        let advice = decision.advice();
        let control = decision.send_control(true);
        let chunk = decision
            .to_chunk(0)
            .expect("offline gate should produce a display chunk");

        assert_eq!(
            decision,
            GateDecision::blocked(StreamState::Failed, "backend is offline")
        );
        assert_eq!(advice.action, GateAdviceAction::RepairGate);
        assert_eq!(advice.action_label(), "repair_gate");
        assert_eq!(advice.state, StreamState::Failed);
        assert_eq!(advice.reason, "backend is offline");
        assert!(!control.send_allowed);
        assert_eq!(control.action, GateAdviceAction::RepairGate);
        assert_eq!(control.action_label, "repair_gate");
        assert_eq!(control.state, StreamState::Failed);
        assert_eq!(control.state_label, "failed");
        assert!(control.state_is_terminal);
        assert!(!control.state_is_pressure);
        assert!(!control.state_blocks_prompt_submit);
        assert_eq!(control.reason, "backend is offline");
        assert_eq!(control.primary_action_label, "repair_gate");
        assert_eq!(
            control.primary_action_disabled_reason.as_deref(),
            Some("backend is offline")
        );
        assert!(control.preserves_prompt);
        assert!(!control.clears_prompt);
        let control_chunk = control
            .block_chunk
            .as_ref()
            .expect("offline send control should expose a display chunk");
        assert_eq!(control_chunk.output_label, "error");
        assert_eq!(control_chunk.appended, "[error] backend is offline");
        assert_eq!(chunk.state, StreamState::Failed);
        assert_eq!(chunk.content, "backend is offline");
    }

    #[test]
    fn frontend_gate_decision_display_snapshot_is_stable_for_hosts() {
        assert_eq!(
            FrontendGateSnapshot::default()
                .decision()
                .display_snapshot(0),
            None
        );

        let busy = FrontendGateSnapshot {
            engine_busy: true,
            active_request: Some("#42 chat-stream".to_owned()),
            ..FrontendGateSnapshot::default()
        };
        let backpressure = FrontendGateSnapshot {
            queued_requests: 4,
            queue_limit: 4,
            ..FrontendGateSnapshot::default()
        };
        let repair = FrontendGateSnapshot {
            safe_device_ok: false,
            engine_busy: true,
            ..FrontendGateSnapshot::default()
        };
        let offline = FrontendGateSnapshot {
            backend_online: false,
            engine_busy: true,
            safe_device_ok: false,
            experience_hygiene_ok: false,
            queued_requests: 8,
            queue_limit: 8,
            active_request: Some("#77 chat-stream".to_owned()),
        };

        let busy_chunk = busy
            .decision()
            .display_snapshot(0)
            .expect("busy frontend gate should expose a display snapshot");
        assert_eq!(busy_chunk.output_label, "busy");
        assert_eq!(busy_chunk.state, StreamState::Busy);
        assert_eq!(busy_chunk.state_label, "busy");
        assert_eq!(
            busy_chunk.appended,
            "[busy] backend engine is busy: #42 chat-stream"
        );
        assert!(!busy_chunk.state_is_terminal);
        assert!(busy_chunk.state_is_pressure);
        assert!(busy_chunk.state_blocks_prompt_submit);

        let backpressure_chunk = backpressure
            .decision()
            .display_snapshot(0)
            .expect("backpressure frontend gate should expose a display snapshot");
        assert_eq!(backpressure_chunk.output_label, "backpressure");
        assert_eq!(backpressure_chunk.state, StreamState::Backpressure);
        assert_eq!(backpressure_chunk.state_label, "backpressure");
        assert_eq!(
            backpressure_chunk.appended,
            "[backpressure] model queue is saturated: 4/4"
        );
        assert!(!backpressure_chunk.state_is_terminal);
        assert!(backpressure_chunk.state_is_pressure);
        assert!(backpressure_chunk.state_blocks_prompt_submit);

        let repair_chunk = repair
            .decision()
            .display_snapshot(0)
            .expect("repair frontend gate should expose a display snapshot");
        assert_eq!(repair_chunk.output_label, "error");
        assert_eq!(repair_chunk.state, StreamState::Failed);
        assert_eq!(repair_chunk.state_label, "failed");
        assert_eq!(repair_chunk.appended, "[error] safe-device gate failed");
        assert!(repair_chunk.state_is_terminal);
        assert!(!repair_chunk.state_is_pressure);
        assert!(!repair_chunk.state_blocks_prompt_submit);

        let offline_chunk = offline
            .decision()
            .display_snapshot(0)
            .expect("offline frontend gate should expose a display snapshot");
        assert_eq!(offline_chunk.output_label, "error");
        assert_eq!(offline_chunk.state, StreamState::Failed);
        assert_eq!(offline_chunk.state_label, "failed");
        assert_eq!(offline_chunk.appended, "[error] backend is offline");
        assert!(offline_chunk.state_is_terminal);
        assert!(!offline_chunk.state_is_pressure);
        assert!(!offline_chunk.state_blocks_prompt_submit);
    }

    #[test]
    fn blocked_decision_can_be_rendered_as_stream_chunk() {
        let busy = GateDecision::blocked(StreamState::Busy, "backend engine is busy");
        let backpressure = GateDecision::blocked(StreamState::Backpressure, "queue full");

        let busy_chunk = busy.to_chunk(7).unwrap();
        let backpressure_chunk = backpressure.to_chunk(8).unwrap();

        assert_eq!(busy_chunk.sequence, 7);
        assert_eq!(busy_chunk.state, StreamState::Busy);
        assert_eq!(busy_chunk.content, "backend engine is busy");
        assert_eq!(backpressure_chunk.state, StreamState::Backpressure);
    }

    #[test]
    fn gate_decisions_expose_wait_and_retry_advice() {
        let allowed = GateDecision::Allowed.advice();
        let queued = GateDecision::blocked(StreamState::Queued, "waiting for worker").advice();
        let busy = GateDecision::blocked(StreamState::Busy, "worker busy").advice();
        let backpressure = GateDecision::blocked(StreamState::Backpressure, "queue full").advice();
        let failed = GateDecision::blocked(StreamState::Failed, "safe-device gate failed").advice();

        assert_eq!(allowed.action, GateAdviceAction::SendNow);
        assert_eq!(allowed.action_label(), "send_now");
        assert_eq!(allowed.state_label(), "pending");
        assert_eq!(allowed.status_line(), "send_now pending: ready to send");
        assert_eq!(queued.action, GateAdviceAction::WaitForWorker);
        assert_eq!(queued.action_label(), "wait_for_worker");
        assert_eq!(queued.state_label(), "queued");
        assert_eq!(busy.action, GateAdviceAction::WaitForCurrentStream);
        assert_eq!(busy.action_label(), "wait_for_current_stream");
        assert_eq!(busy.state_label(), "busy");
        assert_eq!(backpressure.action, GateAdviceAction::RetryLater);
        assert_eq!(backpressure.action_label(), "retry_later");
        assert_eq!(backpressure.state_label(), "backpressure");
        assert_eq!(failed.action, GateAdviceAction::RepairGate);
        assert_eq!(failed.action_label(), "repair_gate");
        assert_eq!(failed.state_label(), "failed");
        assert_eq!(
            failed.status_line(),
            "repair_gate failed: safe-device gate failed"
        );
    }

    #[test]
    fn gate_send_control_exposes_prompt_button_and_draft_policy() {
        let empty = GateDecision::Allowed.send_control(false);
        assert!(!empty.prompt_present);
        assert!(!empty.send_allowed);
        assert_eq!(empty.primary_action_label, "type_prompt");
        assert!(!empty.primary_action_enabled);
        assert_eq!(
            empty.primary_action_disabled_reason.as_deref(),
            Some("empty input")
        );
        assert_eq!(empty.block_chunk, None);
        assert!(!empty.preserves_prompt);
        assert!(!empty.clears_prompt);

        let empty_blocked =
            GateDecision::blocked(StreamState::Busy, "backend engine is busy").send_control(false);
        assert!(!empty_blocked.prompt_present);
        assert!(!empty_blocked.send_allowed);
        assert_eq!(empty_blocked.action, GateAdviceAction::WaitForCurrentStream);
        assert_eq!(empty_blocked.action_label, "wait_for_current_stream");
        assert_eq!(empty_blocked.state, StreamState::Busy);
        assert_eq!(empty_blocked.state_label, "busy");
        assert_eq!(empty_blocked.reason, "backend engine is busy");
        assert_eq!(empty_blocked.primary_action_label, "type_prompt");
        assert!(!empty_blocked.primary_action_enabled);
        assert_eq!(
            empty_blocked.primary_action_disabled_reason.as_deref(),
            Some("empty input")
        );
        assert_eq!(empty_blocked.block_chunk, None);
        assert!(!empty_blocked.preserves_prompt);
        assert!(!empty_blocked.clears_prompt);

        let allowed = GateDecision::Allowed.send_control(true);
        assert!(allowed.prompt_present);
        assert!(allowed.send_allowed);
        assert_eq!(allowed.action, GateAdviceAction::SendNow);
        assert_eq!(allowed.action_label, "send_now");
        assert_eq!(allowed.state, StreamState::Pending);
        assert_eq!(allowed.state_label, "pending");
        assert!(!allowed.state_is_pressure);
        assert!(!allowed.state_blocks_prompt_submit);
        assert_eq!(allowed.reason, "ready to send");
        assert_eq!(allowed.primary_action_label, "send");
        assert!(allowed.primary_action_enabled);
        assert_eq!(allowed.primary_action_disabled_reason, None);
        assert_eq!(allowed.block_chunk, None);
        assert!(!allowed.preserves_prompt);
        assert!(allowed.clears_prompt);

        let queued =
            GateDecision::blocked(StreamState::Queued, "waiting for reviewer").send_control(true);
        assert!(!queued.send_allowed);
        assert_eq!(queued.action, GateAdviceAction::WaitForWorker);
        assert_eq!(queued.action_label, "wait_for_worker");
        assert_eq!(queued.state, StreamState::Queued);
        assert_eq!(queued.state_label, "queued");
        assert!(queued.state_is_pressure);
        assert!(queued.state_blocks_prompt_submit);
        assert_eq!(queued.primary_action_label, "wait_for_worker");
        assert_eq!(
            queued.primary_action_disabled_reason.as_deref(),
            Some("waiting for reviewer")
        );
        let block_chunk = queued
            .block_chunk
            .as_ref()
            .expect("queued send control should expose display chunk");
        assert_eq!(block_chunk.output_label, "queued");
        assert_eq!(block_chunk.appended, "[queued] waiting for reviewer");
        assert!(block_chunk.state_blocks_prompt_submit);
        assert!(queued.preserves_prompt);
        assert!(!queued.clears_prompt);

        let busy =
            GateDecision::blocked(StreamState::Busy, "backend engine is busy").send_control(true);
        assert!(!busy.send_allowed);
        assert_eq!(busy.action, GateAdviceAction::WaitForCurrentStream);
        assert_eq!(busy.action_label, "wait_for_current_stream");
        assert_eq!(busy.state, StreamState::Busy);
        assert_eq!(busy.state_label, "busy");
        assert!(busy.state_is_pressure);
        assert!(busy.state_blocks_prompt_submit);
        assert_eq!(busy.primary_action_label, "wait_for_current_stream");
        assert_eq!(
            busy.primary_action_disabled_reason.as_deref(),
            Some("backend engine is busy")
        );
        let block_chunk = busy
            .block_chunk
            .as_ref()
            .expect("busy send control should expose display chunk");
        assert_eq!(block_chunk.output_label, "busy");
        assert_eq!(block_chunk.appended, "[busy] backend engine is busy");
        assert!(block_chunk.state_blocks_prompt_submit);
        assert!(busy.preserves_prompt);
        assert!(!busy.clears_prompt);

        let backpressure =
            GateDecision::blocked(StreamState::Backpressure, "pool queue full").send_control(true);
        assert!(!backpressure.send_allowed);
        assert_eq!(backpressure.action, GateAdviceAction::RetryLater);
        assert_eq!(backpressure.action_label, "retry_later");
        assert_eq!(backpressure.state_label, "backpressure");
        assert!(backpressure.state_is_pressure);
        assert!(backpressure.state_blocks_prompt_submit);
        assert_eq!(backpressure.primary_action_label, "retry_later");
        assert_eq!(
            backpressure.primary_action_disabled_reason.as_deref(),
            Some("pool queue full")
        );
        let block_chunk = backpressure
            .block_chunk
            .as_ref()
            .expect("blocked send control should expose display chunk");
        assert_eq!(block_chunk.output_label, "backpressure");
        assert_eq!(block_chunk.appended, "[backpressure] pool queue full");
        assert!(block_chunk.state_blocks_prompt_submit);
        assert!(backpressure.preserves_prompt);
        assert!(!backpressure.clears_prompt);

        let repair = GateDecision::blocked(StreamState::Failed, "safe-device gate failed")
            .send_control(true);
        assert!(!repair.send_allowed);
        assert_eq!(repair.action, GateAdviceAction::RepairGate);
        assert_eq!(repair.primary_action_label, "repair_gate");
        assert!(repair.state_is_terminal);
        assert!(!repair.state_blocks_prompt_submit);
        let block_chunk = repair
            .block_chunk
            .as_ref()
            .expect("repair send control should expose display chunk");
        assert_eq!(block_chunk.kind_label, "error");
        assert_eq!(block_chunk.state_label, "failed");
        assert_eq!(block_chunk.output_label, "error");
        assert_eq!(block_chunk.appended, "[error] safe-device gate failed");
        assert!(repair.preserves_prompt);
        assert!(!repair.clears_prompt);
    }

    #[test]
    fn model_pool_global_engine_busy_wins_before_worker_routing() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#1 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));

        assert_eq!(
            snapshot.decision_for_intent(&request.routing_intent()),
            GateDecision::blocked(StreamState::Busy, "backend engine is busy: #1 chat-stream")
        );
    }

    #[test]
    fn route_snapshot_keeps_engine_busy_gate_over_worker_availability() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("#1 chat-stream".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8 review".to_owned())),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);

        let route = snapshot.route_snapshot(&request.routing_intent());

        assert_eq!(
            route.decision,
            GateDecision::blocked(StreamState::Busy, "backend engine is busy: #1 chat-stream")
        );
        assert_eq!(route.decision_action_label, "wait_for_current_stream");
        assert_eq!(route.decision_state_label, "busy");
        assert!(!route.decision_state_is_terminal);
        assert!(route.decision_state_is_pressure);
        assert!(route.decision_state_blocks_prompt_submit);
        assert!(!route.send_allowed);
        assert_eq!(route.send_block_state, Some(StreamState::Busy));
        assert_eq!(route.send_block_state_label.as_deref(), Some("busy"));
        assert!(!route.send_block_state_is_terminal);
        assert!(route.send_block_state_is_pressure);
        assert!(route.send_block_state_blocks_prompt_submit);
        let send_block_chunk = route
            .send_block_chunk
            .as_ref()
            .expect("engine busy should expose a display chunk");
        assert_eq!(send_block_chunk.output_label, "busy");
        assert_eq!(
            send_block_chunk.appended,
            "[busy] backend engine is busy: #1 chat-stream"
        );

        assert_eq!(
            route.pool_status,
            "workers total=2 available=1 busy=1 saturated=0"
        );
        assert_eq!(
            route.route_pool_status,
            "matching total=2 available=1 busy=1 saturated=0"
        );
        assert_eq!(route.workers.len(), 2);
        assert!(route.workers.iter().all(|worker| worker.route_match));
        assert!(route.workers.iter().all(|worker| !worker.selectable));
        assert!(route.workers.iter().all(|worker| {
            worker.decision
                == GateDecision::blocked(
                    StreamState::Busy,
                    "backend engine is busy: #1 chat-stream",
                )
        }));
        assert!(
            route
                .workers
                .iter()
                .all(|worker| worker.picker_action_label == "wait")
        );
        assert_eq!(route.workers[0].worker_status_label(), "available");
        assert_eq!(route.workers[1].worker_status_label(), "busy");
    }

    #[test]
    fn auto_route_allows_scheduler_when_one_worker_is_available() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("deep answer".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("hello")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);

        assert_eq!(
            snapshot.decision_for_intent(&request.routing_intent()),
            GateDecision::Allowed
        );
        assert!(!request.endpoint_pinned());

        let route = snapshot.route_snapshot(&request.routing_intent());
        assert_eq!(route.decision, GateDecision::Allowed);
        assert_eq!(route.decision_action_label, "send_now");
        assert_eq!(route.decision_state_label, "pending");
        assert!(!route.decision_state_is_terminal);
        assert!(!route.decision_state_is_pressure);
        assert!(!route.decision_state_blocks_prompt_submit);
        assert!(route.send_allowed);
        assert_eq!(route.send_block_state, None);
        assert_eq!(route.send_block_state_label, None);
        assert!(!route.send_block_state_is_terminal);
        assert!(!route.send_block_state_is_pressure);
        assert!(!route.send_block_state_blocks_prompt_submit);
        assert_eq!(route.send_block_chunk, None);
        assert_eq!(route.endpoint_label, "auto");
        assert!(!route.endpoint_pinned);
        assert_eq!(route.wire_endpoint_kind_label, "auto");
        assert!(!route.wire_sends_model_endpoint);
    }

    #[test]
    fn auto_route_uses_worker_capabilities_when_declared() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8 review".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester)
                    .with_roles([ModelRole::Summarizer, ModelRole::Tester]),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);

        assert_eq!(
            snapshot.decision_for_intent(&request.routing_intent()),
            GateDecision::blocked(
                StreamState::Queued,
                "all matching model workers are busy; waiting for scheduler across 1 workers"
            )
        );
    }

    #[test]
    fn unpinned_endpoint_hint_is_treated_as_auto_route() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#9 review".to_owned())),
            ],
        );
        let intent = RoutingIntent {
            model_role: ModelRole::Assistant,
            routing_preference: RoutingPreference::PreferQuality,
            model_endpoint: Some(ModelEndpoint::FastReviewer),
            endpoint_pinned: false,
        };

        assert_eq!(intent.endpoint_label(), "auto");
        assert_eq!(snapshot.decision_for_intent(&intent), GateDecision::Allowed);

        let route = snapshot.route_snapshot(&intent);
        assert_eq!(route.endpoint_label, "auto");
        assert!(!route.endpoint_pinned);
        assert_eq!(route.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(route.endpoint_kind_label, "auto");
        assert!(route.endpoint_auto);
        assert!(!route.endpoint_built_in);
        assert!(!route.endpoint_custom);

        let status = snapshot.route_status(&intent);
        assert_eq!(status.matching_workers, 1);
        assert_eq!(status.matching_available_workers, 1);
        assert_eq!(status.matching_busy_workers, 0);
        assert_eq!(
            status.summary(),
            "matching total=1 available=1 busy=0 saturated=0"
        );

        let workers = snapshot.route_workers(&intent);
        assert!(!workers[0].endpoint_selected);
        assert!(workers[0].route_match);
        assert!(workers[0].selectable);
        assert!(!workers[1].endpoint_selected);
        assert!(!workers[1].route_match);
    }

    #[test]
    fn auto_route_reports_when_no_worker_matches_declared_capabilities() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Assistant]),
                ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester)
                    .with_roles([ModelRole::Summarizer]),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("test")])
            .with_model_role(ModelRole::Tester)
            .with_routing_preference(RoutingPreference::PreferFast);

        assert_eq!(
            snapshot.decision_for_intent(&request.routing_intent()),
            GateDecision::blocked(
                StreamState::Queued,
                "no model worker matches role=tester preference=prefer_fast"
            )
        );
    }

    #[test]
    fn pinned_busy_worker_maps_to_busy_without_falling_back_to_auto() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_busy(true, Some("#7 reviewer".to_owned())),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));

        assert_eq!(
            snapshot.decision_for_intent(&request.routing_intent()),
            GateDecision::blocked(
                StreamState::Busy,
                "worker fast-reviewer is busy: #7 reviewer"
            )
        );
        assert!(request.endpoint_pinned());
    }

    #[test]
    fn pinned_worker_capability_mismatch_queues_without_falling_back_to_auto() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("deep answer")])
            .with_model_role(ModelRole::Assistant)
            .with_routing_preference(RoutingPreference::PreferQuality)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));

        assert_eq!(
            snapshot.decision_for_intent(&request.routing_intent()),
            GateDecision::blocked(
                StreamState::Queued,
                "worker fast-reviewer does not match role=assistant preference=prefer_quality"
            )
        );
    }

    #[test]
    fn pinned_saturated_worker_maps_to_backpressure() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer).with_queue(3, 3),
                ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));

        assert_eq!(
            snapshot.decision_for_intent(&request.routing_intent()),
            GateDecision::blocked(
                StreamState::Backpressure,
                "worker fast-reviewer queue is saturated: 3/3"
            )
        );
    }

    #[test]
    fn auto_route_reports_queued_when_every_worker_is_busy_but_pool_has_room() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_busy(true, Some("quality".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_busy(true, Some("reviewer".to_owned())),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("hello")]);

        assert_eq!(
            snapshot.decision_for_intent(&request.routing_intent()),
            GateDecision::blocked(
                StreamState::Queued,
                "all model workers are busy; waiting for scheduler across 2 workers"
            )
        );
    }

    #[test]
    fn unknown_pinned_worker_reports_queued_until_registered() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("hello")])
            .with_model_endpoint(Some(ModelEndpoint::Worker("operator-debug".to_owned())));

        assert_eq!(
            snapshot.decision_for_intent(&request.routing_intent()),
            GateDecision::blocked(
                StreamState::Queued,
                "worker operator-debug is not registered"
            )
        );
    }

    #[test]
    fn model_pool_status_summarizes_available_busy_and_saturated_workers() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_busy(true, Some("review".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester).with_queue(2, 2),
            ],
        );

        let status = snapshot.status();

        assert_eq!(status.total_workers, 3);
        assert_eq!(status.available_workers, 1);
        assert_eq!(status.busy_workers, 1);
        assert_eq!(status.saturated_workers, 1);
        assert_eq!(status.queued_requests, 2);
        assert_eq!(status.queue_limit, 4);
        assert!(status.has_workers());
        assert!(status.has_available_workers());
        assert!(status.has_busy_workers());
        assert!(status.has_saturated_workers());
        assert!(status.has_queued_requests());
        assert!(!status.queue_is_saturated());
        assert_eq!(status.queue_label(), "2/4");
        assert_eq!(status.capacity_state(), StreamState::Queued);
        assert_eq!(status.capacity_state_label(), "queued");
        assert!(status.capacity_state_is_pressure());
        assert!(status.capacity_state_blocks_prompt_submit());
        assert_eq!(
            status.summary(),
            "workers total=3 available=1 busy=1 saturated=1"
        );

        let saturated = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer).with_queue(1, 1)],
        )
        .status();
        assert!(saturated.has_workers());
        assert!(!saturated.has_available_workers());
        assert!(!saturated.has_busy_workers());
        assert!(saturated.has_saturated_workers());
        assert!(saturated.has_queued_requests());
        assert!(saturated.queue_is_saturated());
        assert_eq!(saturated.capacity_state(), StreamState::Backpressure);
        assert_eq!(saturated.capacity_state_label(), "backpressure");
        assert!(saturated.capacity_state_is_pressure());
        assert!(saturated.capacity_state_blocks_prompt_submit());

        let empty = ModelPoolGateSnapshot::default().status();
        assert!(!empty.has_workers());
        assert!(!empty.has_available_workers());
        assert!(!empty.has_busy_workers());
        assert!(!empty.has_saturated_workers());
        assert!(!empty.has_queued_requests());
        assert!(!empty.queue_is_saturated());
        assert_eq!(empty.capacity_state(), StreamState::Pending);
        assert_eq!(empty.capacity_state_label(), "pending");
        assert!(!empty.capacity_state_is_pressure());
        assert!(!empty.capacity_state_blocks_prompt_submit());
    }

    #[test]
    fn route_status_counts_only_workers_matching_current_intent() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8 review".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester)
                    .with_roles([ModelRole::Tester])
                    .with_queue(2, 2),
            ],
        );
        let reviewer = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);
        let tester = ChatRequest::new("s1", vec![ChatMessage::user("test")])
            .with_model_role(ModelRole::Tester);

        let reviewer_status = snapshot.route_status(&reviewer.routing_intent());
        let tester_status = snapshot.route_status(&tester.routing_intent());

        assert!(snapshot.has_capability_declarations());
        assert_eq!(reviewer_status.matching_workers, 1);
        assert_eq!(reviewer_status.matching_available_workers, 0);
        assert_eq!(reviewer_status.matching_busy_workers, 1);
        assert_eq!(reviewer_status.matching_saturated_workers, 0);
        assert_eq!(reviewer_status.matching_queued_requests, 0);
        assert_eq!(reviewer_status.matching_queue_limit, 1);
        assert!(reviewer_status.has_matching_workers());
        assert!(!reviewer_status.has_matching_available_workers());
        assert!(reviewer_status.has_matching_busy_workers());
        assert!(!reviewer_status.has_matching_saturated_workers());
        assert!(!reviewer_status.has_matching_queued_requests());
        assert!(!reviewer_status.matching_queue_is_saturated());
        assert_eq!(reviewer_status.queue_label(), "0/1");
        assert_eq!(reviewer_status.capacity_state(), StreamState::Busy);
        assert_eq!(reviewer_status.capacity_state_label(), "busy");
        assert!(reviewer_status.capacity_state_is_pressure());
        assert!(reviewer_status.capacity_state_blocks_prompt_submit());
        assert_eq!(
            reviewer_status.summary(),
            "matching total=1 available=0 busy=1 saturated=0"
        );
        assert_eq!(tester_status.matching_workers, 1);
        assert_eq!(tester_status.matching_available_workers, 0);
        assert_eq!(tester_status.matching_saturated_workers, 1);
        assert_eq!(tester_status.matching_queued_requests, 2);
        assert_eq!(tester_status.matching_queue_limit, 2);
        assert!(tester_status.has_matching_workers());
        assert!(!tester_status.has_matching_available_workers());
        assert!(!tester_status.has_matching_busy_workers());
        assert!(tester_status.has_matching_saturated_workers());
        assert!(tester_status.has_matching_queued_requests());
        assert!(tester_status.matching_queue_is_saturated());
        assert_eq!(tester_status.queue_label(), "2/2");
        assert_eq!(tester_status.capacity_state(), StreamState::Backpressure);
        assert_eq!(tester_status.capacity_state_label(), "backpressure");
        assert!(tester_status.capacity_state_is_pressure());
        assert!(tester_status.capacity_state_blocks_prompt_submit());
    }

    #[test]
    fn route_workers_exposes_worker_picker_state_without_pinning_auto_route() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Assistant])
                    .with_preferences([RoutingPreference::PreferQuality]),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(1, 1),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);

        let workers = snapshot.route_workers(&request.routing_intent());

        assert!(!request.endpoint_pinned());
        assert_eq!(workers.len(), 3);
        assert_eq!(workers[0].worker.endpoint.label(), "quality-12b");
        assert_eq!(workers[0].endpoint_label(), "quality-12b");
        assert_eq!(workers[0].worker.role_labels(), vec!["assistant"]);
        assert_eq!(
            workers[0].worker.preference_labels(),
            vec!["prefer_quality"]
        );
        assert!(!workers[0].endpoint_selected);
        assert!(!workers[0].route_match);
        assert!(!workers[0].selectable);
        assert_eq!(
            workers[0].picker_action,
            ModelRouteWorkerPickerAction::Unavailable
        );
        assert_eq!(workers[0].picker_action_label, "unavailable");
        assert_eq!(workers[0].decision_action_label(), "wait_for_worker");
        assert_eq!(workers[0].decision_state_label(), "queued");
        assert!(!workers[0].decision_state_is_terminal());
        assert!(workers[0].decision_state_is_pressure());
        assert!(workers[0].decision_state_blocks_prompt_submit());
        assert_eq!(
            workers[0].decision_reason(),
            "worker quality-12b does not match role=reviewer preference=prefer_fast"
        );
        let decision_chunk = workers[0]
            .decision_display_snapshot()
            .expect("capability mismatch should expose a decision display chunk");
        assert_eq!(decision_chunk.output_label, "queued");
        assert_eq!(
            decision_chunk.appended,
            "[queued] worker quality-12b does not match role=reviewer preference=prefer_fast"
        );
        assert!(decision_chunk.state_blocks_prompt_submit);
        assert_eq!(
            workers[0].decision,
            GateDecision::blocked(
                StreamState::Queued,
                "worker quality-12b does not match role=reviewer preference=prefer_fast"
            )
        );
        assert_eq!(workers[1].worker.endpoint.label(), "fast-reviewer");
        assert_eq!(workers[1].worker_status_label(), "available");
        assert_eq!(workers[1].worker_status_state(), StreamState::Pending);
        assert_eq!(workers[1].worker_status_state_label(), "pending");
        assert!(workers[1].worker_status_is_available());
        assert!(!workers[1].worker_status_is_pressure());
        assert!(!workers[1].worker_status_blocks_prompt_submit());
        assert_eq!(workers[1].worker_status_display_snapshot(), None);
        assert!(!workers[1].endpoint_selected);
        assert!(workers[1].route_match);
        assert!(workers[1].selectable);
        assert_eq!(
            workers[1].picker_action,
            ModelRouteWorkerPickerAction::Select
        );
        assert_eq!(workers[1].picker_action_label, "select");
        assert_eq!(workers[1].decision, GateDecision::Allowed);
        assert_eq!(workers[1].decision_action_label(), "send_now");
        assert_eq!(workers[1].decision_state_label(), "pending");
        assert!(!workers[1].decision_state_is_terminal());
        assert!(!workers[1].decision_state_is_pressure());
        assert!(!workers[1].decision_state_blocks_prompt_submit());
        assert_eq!(workers[1].decision_display_snapshot(), None);
        assert_eq!(workers[1].decision_reason(), "ready to send");
        assert_eq!(
            workers[1].selection_summary,
            "role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true"
        );
        assert_eq!(workers[1].selection_intent.model_role, ModelRole::Reviewer);
        assert_eq!(
            workers[1].selection_intent.routing_preference,
            RoutingPreference::PreferFast
        );
        assert!(workers[1].selection_intent.endpoint_pinned);
        assert_eq!(workers[1].selection_model_role_label, "reviewer");
        assert_eq!(workers[1].selection_routing_preference_label, "prefer_fast");
        assert_eq!(workers[1].selection_endpoint_label, "fast-reviewer");
        assert_eq!(
            workers[1].selection_endpoint_kind,
            ModelEndpointSelectionKind::BuiltIn
        );
        assert_eq!(workers[1].selection_endpoint_kind_label, "built_in");
        assert!(!workers[1].selection_endpoint_auto);
        assert!(workers[1].selection_endpoint_built_in);
        assert!(!workers[1].selection_endpoint_custom);
        assert_eq!(workers[1].selection_wire_model_role_label, "reviewer");
        assert_eq!(
            workers[1].selection_wire_routing_preference_label,
            "prefer_fast"
        );
        assert!(workers[1].selection_wire_prefer_fast);
        assert!(!workers[1].selection_wire_prefer_quality);
        assert!(workers[1].selection_wire_endpoint_pinned);
        assert_eq!(workers[1].selection_wire_endpoint_kind_label, "built_in");
        assert!(workers[1].selection_wire_sends_model_endpoint);
        assert_eq!(
            workers[1].selection_wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );
        assert_eq!(workers[2].worker.endpoint.label(), "summary-tester");
        assert_eq!(workers[2].worker_status_label(), "backpressure");
        assert_eq!(workers[2].worker_status_state(), StreamState::Backpressure);
        assert_eq!(workers[2].worker_status_state_label(), "backpressure");
        assert!(!workers[2].worker_status_is_available());
        assert!(workers[2].worker_status_is_pressure());
        assert!(workers[2].worker_status_blocks_prompt_submit());
        let worker_status_chunk = workers[2]
            .worker_status_display_snapshot()
            .expect("saturated worker should expose a status display chunk");
        assert_eq!(worker_status_chunk.output_label, "backpressure");
        assert_eq!(
            worker_status_chunk.appended,
            "[backpressure] worker summary-tester queue is saturated: 1/1"
        );
        assert!(workers[2].route_match);
        assert!(!workers[2].selectable);
        assert_eq!(workers[2].picker_action, ModelRouteWorkerPickerAction::Wait);
        assert_eq!(workers[2].picker_action_label, "wait");
        assert_eq!(workers[2].decision_action_label(), "retry_later");
        assert_eq!(workers[2].decision_state_label(), "backpressure");
        assert!(!workers[2].decision_state_is_terminal());
        assert!(workers[2].decision_state_is_pressure());
        assert!(workers[2].decision_state_blocks_prompt_submit());
        let decision_chunk = workers[2]
            .decision_display_snapshot()
            .expect("saturated route worker should expose a decision display chunk");
        assert_eq!(decision_chunk.output_label, "backpressure");
        assert_eq!(
            decision_chunk.appended,
            "[backpressure] worker summary-tester queue is saturated: 1/1"
        );
        assert_eq!(
            workers[2].decision,
            GateDecision::blocked(
                StreamState::Backpressure,
                "worker summary-tester queue is saturated: 1/1"
            )
        );
    }

    #[test]
    fn route_workers_expose_custom_worker_selection_contract() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Worker("mlx-reviewer-8b".to_owned()))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);

        let workers = snapshot.route_workers(&request.routing_intent());

        assert!(!request.endpoint_pinned());
        assert_eq!(workers.len(), 1);
        assert!(workers[0].route_match);
        assert!(workers[0].selectable);
        assert_eq!(
            workers[0].picker_action,
            ModelRouteWorkerPickerAction::Select
        );
        assert_eq!(workers[0].picker_action_label, "select");
        assert_eq!(
            workers[0].selection_summary,
            "role=reviewer preference=prefer_fast endpoint=mlx-reviewer-8b pinned=true"
        );
        assert!(workers[0].selection_intent.endpoint_pinned);
        assert_eq!(
            workers[0]
                .selection_intent
                .model_endpoint
                .as_ref()
                .map(ModelEndpoint::label),
            Some("mlx-reviewer-8b")
        );
        assert_eq!(
            workers[0].selection_endpoint_kind,
            ModelEndpointSelectionKind::Custom
        );
        assert_eq!(workers[0].selection_endpoint_kind_label, "custom");
        assert!(!workers[0].selection_endpoint_auto);
        assert!(!workers[0].selection_endpoint_built_in);
        assert!(workers[0].selection_endpoint_custom);
        assert!(workers[0].selection_wire_endpoint_pinned);
        assert_eq!(workers[0].selection_wire_endpoint_kind_label, "custom");
        assert!(workers[0].selection_wire_sends_model_endpoint);
        assert_eq!(
            workers[0].selection_wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-8b")
        );
    }

    #[test]
    fn route_workers_keep_frontend_repair_gate_over_worker_availability() {
        let snapshot = ModelPoolGateSnapshot::new(
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
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);

        let route = snapshot.route_snapshot(&request.routing_intent());
        let workers = snapshot.route_workers(&request.routing_intent());

        assert_eq!(route.decision_action_label, "repair_gate");
        assert_eq!(route.decision_state_label, "failed");
        assert!(!route.send_allowed);
        assert_eq!(workers.len(), 1);
        assert!(workers[0].route_match);
        assert!(!workers[0].selectable);
        assert_eq!(
            workers[0].picker_action,
            ModelRouteWorkerPickerAction::RepairGate
        );
        assert_eq!(workers[0].picker_action_label, "repair_gate");
        assert_eq!(
            workers[0].decision,
            GateDecision::blocked(StreamState::Failed, "safe-device gate failed")
        );
        assert_eq!(workers[0].decision_action_label(), "repair_gate");
        assert_eq!(workers[0].decision_state_label(), "failed");
        assert!(workers[0].decision_state_is_terminal());
        assert!(!workers[0].decision_state_is_pressure());
        assert!(!workers[0].decision_state_blocks_prompt_submit());
        assert_eq!(workers[0].decision_reason(), "safe-device gate failed");
        let decision_chunk = workers[0]
            .decision_display_snapshot()
            .expect("repair gate picker row should carry a display chunk");
        assert_eq!(decision_chunk.output_label, "error");
        assert_eq!(decision_chunk.appended, "[error] safe-device gate failed");
        assert_eq!(workers[0].worker_status_label(), "available");
        assert_eq!(workers[0].worker.role_labels(), vec!["reviewer"]);
        assert_eq!(workers[0].worker.preference_labels(), vec!["prefer_fast"]);
        assert_eq!(workers[0].worker_status_display_snapshot(), None);
        assert_eq!(
            workers[0].selection_summary,
            "role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true"
        );
        assert_eq!(workers[0].selection_model_role_label, "reviewer");
        assert_eq!(workers[0].selection_routing_preference_label, "prefer_fast");
        assert_eq!(workers[0].selection_endpoint_label, "fast-reviewer");
        assert_eq!(
            workers[0].selection_endpoint_kind,
            ModelEndpointSelectionKind::BuiltIn
        );
        assert_eq!(workers[0].selection_endpoint_kind_label, "built_in");
        assert!(!workers[0].selection_endpoint_auto);
        assert!(workers[0].selection_endpoint_built_in);
        assert!(!workers[0].selection_endpoint_custom);
        assert_eq!(workers[0].selection_wire_model_role_label, "reviewer");
        assert_eq!(
            workers[0].selection_wire_routing_preference_label,
            "prefer_fast"
        );
        assert!(workers[0].selection_wire_prefer_fast);
        assert!(!workers[0].selection_wire_prefer_quality);
        assert!(workers[0].selection_wire_endpoint_pinned);
        assert_eq!(workers[0].selection_wire_endpoint_kind_label, "built_in");
        assert!(workers[0].selection_wire_sends_model_endpoint);
        assert_eq!(
            workers[0].selection_wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );
    }

    #[test]
    fn workers_host_snapshot_projects_service_dto_under_repair_gate_without_side_effects() {
        let snapshot = ModelPoolGateSnapshot::new(
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
                    .with_preferences([RoutingPreference::PreferQuality])
                    .with_busy(true, Some("#41 review".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester)
                    .with_roles([ModelRole::Tester])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(1, 1),
            ],
        );
        let intent = RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast);

        let dto = snapshot.route_snapshot(&intent).workers_host_snapshot();

        assert!(dto.read_only);
        assert!(!dto.launches_process);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.carries_request_preview);
        assert!(!dto.mutates_history);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert_eq!(
            dto.route,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
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
        assert_eq!(dto.decision_action_label, "repair_gate");
        assert_eq!(dto.decision_state_label, "failed");
        assert_eq!(dto.decision_reason, "safe-device gate failed");
        assert_eq!(
            dto.pool_status,
            "workers total=3 available=1 busy=1 saturated=1"
        );
        assert_eq!(
            dto.route_pool_status,
            "matching total=1 available=1 busy=0 saturated=0"
        );
        assert_eq!(dto.workers.len(), 3);

        let ready = &dto.workers[0];
        assert_eq!(ready.endpoint_label, "fast-reviewer");
        assert_eq!(ready.role_labels, vec!["reviewer"]);
        assert_eq!(ready.preference_labels, vec!["prefer_fast"]);
        assert_eq!(ready.worker_status_label, "available");
        assert_eq!(ready.worker_status_state_label, "pending");
        assert!(ready.worker_status_is_available);
        assert!(!ready.worker_status_is_pressure);
        assert!(!ready.worker_status_blocks_prompt_submit);
        assert_eq!(ready.worker_status_display_snapshot, None);
        assert!(ready.route_match);
        assert!(!ready.endpoint_selected);
        assert!(!ready.selectable);
        assert_eq!(ready.picker_action_label, "repair_gate");
        assert_eq!(ready.decision_action_label, "repair_gate");
        assert_eq!(ready.decision_state_label, "failed");
        assert_eq!(ready.decision_reason, "safe-device gate failed");
        let ready_decision = ready
            .decision_display_snapshot
            .as_ref()
            .expect("repair gate row should keep a display chunk");
        assert_eq!(ready_decision.output_label, "error");
        assert_eq!(ready_decision.appended, "[error] safe-device gate failed");
        assert_eq!(
            ready.selection_summary,
            "role=reviewer preference=prefer_fast endpoint=fast-reviewer pinned=true"
        );
        assert_eq!(ready.selection_model_role_label, "reviewer");
        assert_eq!(ready.selection_routing_preference_label, "prefer_fast");
        assert_eq!(ready.selection_endpoint_label, "fast-reviewer");
        assert_eq!(ready.selection_endpoint_kind_label, "built_in");
        assert_eq!(ready.selection_wire_model_role_label, "reviewer");
        assert_eq!(ready.selection_wire_routing_preference_label, "prefer_fast");
        assert!(ready.selection_wire_prefer_fast);
        assert!(!ready.selection_wire_prefer_quality);
        assert!(ready.selection_wire_endpoint_pinned);
        assert_eq!(ready.selection_wire_endpoint_kind_label, "built_in");
        assert!(ready.selection_wire_sends_model_endpoint);
        assert_eq!(
            ready.selection_wire_model_endpoint_label.as_deref(),
            Some("fast-reviewer")
        );

        let busy = &dto.workers[1];
        assert_eq!(busy.endpoint_label, "quality-12b");
        assert_eq!(busy.role_labels, vec!["reviewer"]);
        assert_eq!(busy.preference_labels, vec!["prefer_quality"]);
        assert_eq!(busy.worker_status_label, "busy");
        assert_eq!(busy.worker_status_state_label, "busy");
        assert!(!busy.worker_status_is_available);
        assert!(busy.worker_status_is_pressure);
        assert!(busy.worker_status_blocks_prompt_submit);
        let busy_status = busy
            .worker_status_display_snapshot
            .as_ref()
            .expect("busy worker should keep a local health chunk");
        assert_eq!(busy_status.output_label, "busy");
        assert_eq!(
            busy_status.appended,
            "[busy] worker quality-12b is busy: #41 review"
        );
        assert!(!busy.route_match);
        assert!(!busy.selectable);
        assert_eq!(busy.picker_action_label, "repair_gate");
        assert_eq!(busy.decision_reason, "safe-device gate failed");
        assert_eq!(
            busy.selection_wire_model_endpoint_label.as_deref(),
            Some("quality-12b")
        );

        let saturated = &dto.workers[2];
        assert_eq!(saturated.endpoint_label, "summary-tester");
        assert_eq!(saturated.role_labels, vec!["tester"]);
        assert_eq!(saturated.preference_labels, vec!["prefer_fast"]);
        assert_eq!(saturated.worker_status_label, "backpressure");
        assert_eq!(saturated.worker_status_state_label, "backpressure");
        assert!(!saturated.worker_status_is_available);
        assert!(saturated.worker_status_is_pressure);
        assert!(saturated.worker_status_blocks_prompt_submit);
        let saturated_status = saturated
            .worker_status_display_snapshot
            .as_ref()
            .expect("saturated worker should keep a local health chunk");
        assert_eq!(saturated_status.output_label, "backpressure");
        assert_eq!(
            saturated_status.appended,
            "[backpressure] worker summary-tester queue is saturated: 1/1"
        );
        assert!(!saturated.route_match);
        assert!(!saturated.selectable);
        assert_eq!(saturated.picker_action_label, "repair_gate");
        assert_eq!(saturated.decision_reason, "safe-device gate failed");
        assert_eq!(
            saturated.selection_wire_model_endpoint_label.as_deref(),
            Some("summary-tester")
        );
    }

    #[test]
    fn workers_host_snapshot_keeps_web_lab_forge_boundary_read_only() {
        let intent = RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast);
        let scenarios = [
            (
                "allowed",
                FrontendGateSnapshot::default(),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ),
            (
                "engine_busy",
                FrontendGateSnapshot {
                    engine_busy: true,
                    active_request: Some("#9 chat-stream".to_owned()),
                    ..FrontendGateSnapshot::default()
                },
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ),
            (
                "repair_gate",
                FrontendGateSnapshot {
                    safe_device_ok: false,
                    ..FrontendGateSnapshot::default()
                },
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ),
            (
                "route_backpressure",
                FrontendGateSnapshot::default(),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(1, 1),
            ),
        ];

        for (label, frontend, worker) in scenarios {
            let dto = ModelPoolGateSnapshot::new(frontend, vec![worker])
                .route_snapshot(&intent)
                .workers_host_snapshot();

            assert!(dto.read_only, "{label} should stay read-only");
            assert!(!dto.launches_process, "{label} must not launch a process");
            assert!(!dto.sends_prompt, "{label} must not send prompts");
            assert!(!dto.starts_stream, "{label} must not enter StartStream");
            assert!(
                !dto.carries_request_preview,
                "{label} must not expose request_preview"
            );
            assert!(
                !dto.mutates_history,
                "{label} must not carry replayable history side effects"
            );
            assert!(
                !dto.carries_stream_chunk,
                "{label} must not expose stream chunks"
            );
            assert!(
                !dto.carries_input_action_snapshot,
                "{label} must not expose input actions"
            );
            assert_eq!(
                dto.workers.len(),
                1,
                "{label} should still expose worker rows"
            );
            assert_eq!(dto.workers[0].selection_wire_model_role_label, "reviewer");
            assert_eq!(
                dto.workers[0].selection_wire_routing_preference_label,
                "prefer_fast"
            );
            assert!(
                dto.workers[0].selection_wire_sends_model_endpoint,
                "{label} keeps selection wire as data, not as a send action"
            );
        }
    }

    #[test]
    fn workers_host_snapshot_marks_8686_8690_readiness_without_side_effects() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let intent = RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast);
        let dto = ModelPoolGateSnapshot::new(
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
        )
        .route_snapshot(&intent)
        .workers_host_snapshot();

        assert!(dto.read_only);
        assert!(!dto.launches_process);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.carries_request_preview);
        assert!(!dto.mutates_history);
        assert!(!dto.carries_stream_chunk);
        assert!(!dto.carries_input_action_snapshot);
        assert!(dto.send_allowed);
        assert_eq!(dto.decision_action_label, "send_now");
        assert_eq!(
            dto.pool_status,
            "workers total=5 available=3 busy=1 saturated=1"
        );
        assert_eq!(
            dto.route_pool_status,
            "matching total=4 available=2 busy=1 saturated=1"
        );
        assert_eq!(dto.workers.len(), 5);

        let ready = &dto.workers[0];
        assert_eq!(ready.endpoint_label, "127.0.0.1:8686");
        assert_eq!(ready.worker_status_label, "available");
        assert_eq!(ready.worker_status_state_label, "pending");
        assert!(ready.worker_status_is_available);
        assert!(!ready.worker_status_is_pressure);
        assert!(ready.route_match);
        assert!(ready.selectable);
        assert_eq!(ready.picker_action_label, "select");
        assert_eq!(ready.decision_action_label, "send_now");
        assert_eq!(ready.selection_endpoint_kind_label, "custom");
        assert_eq!(
            ready.selection_wire_model_endpoint_label.as_deref(),
            Some("127.0.0.1:8686")
        );

        let busy = &dto.workers[1];
        assert_eq!(busy.endpoint_label, "127.0.0.1:8687");
        assert_eq!(busy.worker_status_label, "busy");
        assert_eq!(busy.worker_status_state_label, "busy");
        assert!(!busy.worker_status_is_available);
        assert!(busy.worker_status_is_pressure);
        assert!(busy.worker_status_blocks_prompt_submit);
        assert!(busy.route_match);
        assert!(!busy.selectable);
        assert_eq!(busy.picker_action_label, "wait");
        assert_eq!(busy.decision_action_label, "wait_for_current_stream");
        assert_eq!(busy.decision_state_label, "busy");
        assert_eq!(
            busy.decision_reason,
            "worker 127.0.0.1:8687 is busy: #8687 active review"
        );
        let busy_status = busy
            .worker_status_display_snapshot
            .as_ref()
            .expect("busy port should keep a local health chunk");
        assert_eq!(busy_status.output_label, "busy");
        assert_eq!(
            busy_status.appended,
            "[busy] worker 127.0.0.1:8687 is busy: #8687 active review"
        );

        let unavailable = &dto.workers[2];
        assert_eq!(unavailable.endpoint_label, "127.0.0.1:8688");
        assert_eq!(unavailable.worker_status_label, "available");
        assert!(unavailable.worker_status_is_available);
        assert!(!unavailable.route_match);
        assert!(!unavailable.selectable);
        assert_eq!(unavailable.picker_action_label, "unavailable");
        assert_eq!(unavailable.decision_action_label, "wait_for_worker");
        assert_eq!(unavailable.decision_state_label, "queued");
        assert_eq!(
            unavailable.decision_reason,
            "worker 127.0.0.1:8688 does not match role=reviewer preference=prefer_fast"
        );

        let saturated = &dto.workers[3];
        assert_eq!(saturated.endpoint_label, "127.0.0.1:8689");
        assert_eq!(saturated.worker_status_label, "backpressure");
        assert_eq!(saturated.worker_status_state_label, "backpressure");
        assert!(!saturated.worker_status_is_available);
        assert!(saturated.worker_status_is_pressure);
        assert!(saturated.route_match);
        assert!(!saturated.selectable);
        assert_eq!(saturated.picker_action_label, "wait");
        assert_eq!(saturated.decision_action_label, "retry_later");
        assert_eq!(saturated.decision_state_label, "backpressure");

        let ready_tail = &dto.workers[4];
        assert_eq!(ready_tail.endpoint_label, "127.0.0.1:8690");
        assert_eq!(ready_tail.worker_status_label, "available");
        assert!(ready_tail.worker_status_is_available);
        assert!(ready_tail.route_match);
        assert!(ready_tail.selectable);
        assert_eq!(ready_tail.picker_action_label, "select");
    }

    #[test]
    fn status_consumer_reads_8686_8690_without_changing_busy_or_readiness() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let intent = RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast);
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
        let before_pool = gate.status();
        let before_worker_lines = gate.worker_status_lines();

        let first = gate.route_snapshot(&intent).workers_host_snapshot();
        let second = gate.route_snapshot(&intent).workers_host_snapshot();

        assert_eq!(first, second);
        assert_eq!(gate.status(), before_pool);
        assert_eq!(gate.worker_status_lines(), before_worker_lines);
        assert!(first.read_only);
        assert!(!first.launches_process);
        assert!(!first.sends_prompt);
        assert!(!first.starts_stream);
        assert!(!first.carries_request_preview);
        assert!(!first.mutates_history);
        assert!(!first.carries_stream_chunk);
        assert!(!first.carries_input_action_snapshot);
        assert!(!first.send_allowed);
        assert_eq!(first.decision_action_label, "wait_for_current_stream");
        assert_eq!(first.decision_state_label, "busy");
        assert_eq!(
            first.decision_reason,
            "backend engine is busy: #8687 chat-stream"
        );
        assert_eq!(
            first.pool_status,
            "workers total=5 available=3 busy=1 saturated=1"
        );
        assert_eq!(
            first.route_pool_status,
            "matching total=4 available=2 busy=1 saturated=1"
        );
        assert_eq!(
            first
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
    fn status_consumer_field_bundle_covers_8686_8690_worker_states_and_read_only_flags() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let intent = RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast);
        let dto = ModelPoolGateSnapshot::new(
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
        )
        .route_snapshot(&intent)
        .workers_host_snapshot();

        assert_eq!(
            (
                dto.read_only,
                dto.launches_process,
                dto.sends_prompt,
                dto.starts_stream,
                dto.carries_request_preview,
                dto.mutates_history,
                dto.carries_stream_chunk,
                dto.carries_input_action_snapshot,
            ),
            (true, false, false, false, false, false, false, false)
        );
        assert_eq!(
            dto.pool_status,
            "workers total=5 available=3 busy=1 saturated=1"
        );
        assert_eq!(
            dto.route_pool_status,
            "matching total=4 available=2 busy=1 saturated=1"
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
    fn smartsteam_status_snapshot_reads_daemon_supervisor_pool_without_side_effects() {
        let intent = RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast);
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: true,
                active_request: Some("round=314 generate:start".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Worker("127.0.0.1:8686".to_owned()))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(ModelEndpoint::Worker("127.0.0.1:8687".to_owned()))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8687 active review".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::Worker("127.0.0.1:8688".to_owned()))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(1, 1),
            ],
        );
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(224392), Some(314), Some(313))
            .with_supervisor(true, true)
            .with_readiness(false, true)
            .with_model_cache_label("5/5 external diagnostic");
        let before_pool = gate.status();
        let before_worker_lines = gate.worker_status_lines();

        let first = SmartSteamStatusSnapshot::from_model_pool(source.clone(), &gate, Some(&intent));
        let second = SmartSteamStatusSnapshot::from_model_pool(source, &gate, Some(&intent));

        assert_eq!(first, second);
        assert_eq!(gate.status(), before_pool);
        assert_eq!(gate.worker_status_lines(), before_worker_lines);
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
        assert!(first.daemon_running);
        assert_eq!(first.daemon_pid, Some(224392));
        assert!(first.supervisor_running);
        assert!(first.supervisor_check_only);
        assert_eq!(first.active_round, Some(314));
        assert_eq!(first.ledger_round, Some(313));
        assert_eq!(first.latest_done_round, Some(313));
        assert!(first.round_in_progress);
        assert!(!first.readiness_ok);
        assert!(first.engine_busy);
        assert_eq!(
            first.active_request.as_deref(),
            Some("round=314 generate:start")
        );
        assert!(first.remote_chain_ready);
        assert_eq!(
            first.model_cache_label.as_deref(),
            Some("5/5 external diagnostic")
        );
        assert_eq!(first.workers_total, 3);
        assert_eq!(first.workers_available, 1);
        assert_eq!(first.workers_busy, 1);
        assert_eq!(first.workers_saturated, 1);
        assert_eq!(
            first.pool_status,
            "workers total=3 available=1 busy=1 saturated=1"
        );
        assert_eq!(
            first.route_pool_status.as_deref(),
            Some("matching total=3 available=1 busy=1 saturated=1")
        );
        assert_eq!(first.route_send_allowed, Some(false));
        assert_eq!(
            first.route_send_block_reason.as_deref(),
            Some("backend engine is busy: round=314 generate:start")
        );
        assert_eq!(
            first.status_line(),
            "daemon_running=true active_round=314 latest_done_round=313 round_in_progress=true ledger_round=313 readiness_ok=false engine_busy=true remote_chain_ready=true pool=workers total=3 available=1 busy=1 saturated=1"
        );
    }

    #[test]
    fn smartsteam_status_snapshot_field_bundle_is_read_only_for_ui_consumers() {
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(224392), Some(314), Some(314))
            .with_supervisor(true, true)
            .with_readiness(true, true)
            .with_model_cache_label("5/5 external diagnostic");
        let dto = SmartSteamStatusSnapshot::from_model_pool(source, &gate, None);

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
            ],
            [
                true, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false,
            ]
        );
        assert_eq!(
            smartsteam_status_json_fields(&dto),
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
                "daemon_running",
                "daemon_pid",
                "supervisor_running",
                "supervisor_check_only",
                "active_round",
                "ledger_round",
                "latest_done_round",
                "round_in_progress",
                "readiness_ok",
                "engine_busy",
                "active_request",
                "remote_chain_ready",
                "model_cache_label",
                "worker_windows_total",
                "worker_windows_paused",
                "worker_windows_polluted",
                "worker_windows_clean_room_replacements_required",
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
                "workers_total",
                "workers_available",
                "workers_busy",
                "workers_saturated",
                "pool_status",
                "route_pool_status",
                "route_send_allowed",
                "route_send_block_reason",
            ]
        );
        assert_eq!(
            dto.model_cache_label.as_deref(),
            Some("5/5 external diagnostic")
        );
    }

    #[test]
    fn smartsteam_status_snapshot_surfaces_round_done_ledger_pending_without_side_effects() {
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                engine_busy: false,
                active_request: Some("round=333 done [DONE]".to_owned()),
                ..FrontendGateSnapshot::default()
            },
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(192756), Some(333), Some(332))
            .with_supervisor(true, true)
            .with_readiness(true, true)
            .with_daemon_round_transition(
                SmartSteamDaemonRoundTransitionStatusSource::round_done_ledger_pending(
                    333,
                    Some(332),
                )
                .with_evidence_ids(["stdout:round-333:done", "ledger:latest-round-332"])
                .with_reason_codes(["round_done_before_ledger_commit"]),
            );

        let dto = SmartSteamStatusSnapshot::from_model_pool(source, &gate, None);
        let transition = dto
            .daemon_round_transition_status
            .as_ref()
            .expect("round-done transition status should be surfaced");

        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_active_round);
        assert_eq!(dto.latest_done_round, Some(333));
        assert!(!dto.round_in_progress);
        assert!(transition.read_only);
        assert!(transition.report_only);
        assert!(transition.observed_round_done);
        assert_eq!(transition.done_round, Some(333));
        assert_eq!(transition.latest_done_round, Some(333));
        assert!(!transition.round_in_progress);
        assert_eq!(transition.ledger_round, Some(332));
        assert!(transition.ledger_commit_pending);
        assert_eq!(transition.ledger_lag_rounds, Some(1));
        assert_eq!(transition.status_label, "round-done-ledger-commit-pending");
        assert_eq!(
            transition.evidence_ids,
            vec!["stdout:round-333:done", "ledger:latest-round-332"]
        );
        assert_eq!(
            transition.reason_codes,
            vec!["round_done_before_ledger_commit"]
        );
        assert!(!transition.starts_daemon);
        assert!(!transition.stops_daemon);
        assert!(!transition.touches_remote);
        assert!(!transition.sends_prompt);
        assert!(!transition.starts_stream);
        assert!(!transition.replays_prompt);
        assert!(!transition.mutates_active_round);
        assert!(!transition.writes_ndkv);
        assert_eq!(
            dto.daemon_round_transition_summary.as_deref(),
            Some(
                "daemon_round_transition status=round-done-ledger-commit-pending observed_round_done=true done_round=333 latest_done_round=333 round_in_progress=false ledger_round=332 ledger_commit_pending=true ledger_lag_rounds=1 starts_daemon=false stops_daemon=false sends_prompt=false starts_stream=false writes_ndkv=false"
            )
        );
        assert!(
            dto.status_line()
                .contains("daemon_round_transition status=round-done-ledger-commit-pending")
        );
    }

    #[test]
    fn smartsteam_status_snapshot_preserves_absent_next_round_decision_compatibility() {
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );

        let dto = SmartSteamStatusSnapshot::from_model_pool(
            SmartSteamStatusSource::new()
                .with_daemon(true, Some(197412), Some(367), Some(366))
                .with_daemon_round_progress(Some(366), true)
                .with_supervisor(true, true)
                .with_readiness(true, true),
            &gate,
            None,
        );

        assert_eq!(dto.next_round_decision_status, None);
        assert_eq!(dto.next_round_decision_summary, None);
        assert!(!dto.status_line().contains("next_round_decision"));
    }

    #[test]
    fn smartsteam_status_snapshot_surfaces_next_round_decision_without_side_effects() {
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
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(197412), Some(367), Some(366))
            .with_daemon_round_progress(Some(366), true)
            .with_supervisor(true, true)
            .with_readiness(true, true)
            .with_next_round_decision(
                SmartSteamNextRoundDecisionStatusSource::safe_to_wait_current_round_active(
                    367, 366,
                )
                .with_evidence_ids(["next-round:round-367:live-status"])
                .with_reason_codes(["current_round_active", "safe_to_wait"]),
            );

        let dto = SmartSteamStatusSnapshot::from_model_pool(source, &gate, None);
        let decision = dto
            .next_round_decision_status
            .as_ref()
            .expect("next-round decision status should be surfaced");

        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_active_round);
        assert!(!dto.mutates_worker_window_status);
        assert_eq!(decision.report_version, "next_round_decision_report_v1");
        assert!(decision.read_only);
        assert!(decision.report_only);
        assert_eq!(decision.status_label, "safe-to-wait-current-round-active");
        assert!(decision.safe_to_wait_current_round_active);
        assert!(!decision.safe_to_continue_after_current_round);
        assert!(!decision.operator_attention_blocked);
        assert_eq!(decision.current_round, Some(367));
        assert_eq!(decision.latest_done_round, Some(366));
        assert_eq!(
            decision.evidence_ids,
            vec!["next-round:round-367:live-status"]
        );
        assert_eq!(
            decision.reason_codes,
            vec!["current_round_active", "safe_to_wait"]
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
        assert_eq!(
            dto.next_round_decision_summary.as_deref(),
            Some(
                "next_round_decision status=safe-to-wait-current-round-active safe_to_wait_current_round_active=true safe_to_continue_after_current_round=false operator_attention_blocked=false current_round=367 latest_done_round=366 starts_daemon=false sends_prompt=false starts_stream=false writes_ndkv=false creates_thread=false"
            )
        );
        assert!(
            dto.status_line()
                .contains("next_round_decision status=safe-to-wait-current-round-active")
        );
    }

    #[test]
    fn smartsteam_status_snapshot_surfaces_operator_attention_blocked_next_round_decision() {
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(197412), Some(368), Some(367))
            .with_supervisor(true, true)
            .with_readiness(true, true)
            .with_next_round_decision(
                SmartSteamNextRoundDecisionStatusSource::operator_attention_blocked(
                    Some(368),
                    Some(367),
                )
                .with_evidence_ids(["next-round:operator-attention"])
                .with_reason_codes(["operator_attention_blocked"]),
            );

        let dto = SmartSteamStatusSnapshot::from_model_pool(source, &gate, None);
        let decision = dto
            .next_round_decision_status
            .as_ref()
            .expect("operator-attention status should be surfaced");

        assert_eq!(decision.status_label, "operator-attention-blocked");
        assert!(decision.operator_attention_blocked);
        assert!(decision.operator_action_required);
        assert!(!decision.safe_to_wait_current_round_active);
        assert!(!decision.safe_to_continue_after_current_round);
        assert!(!decision.starts_daemon);
        assert!(!decision.sends_prompt);
        assert!(!decision.starts_stream);
        assert!(!decision.writes_ndkv);
        assert!(!decision.creates_thread);
    }

    #[test]
    fn smartsteam_status_snapshot_maps_current_next_round_decision_report_fields() {
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
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(199264), Some(369), Some(368))
            .with_daemon_round_progress(Some(368), true)
            .with_supervisor(true, true)
            .with_readiness(true, true)
            .with_next_round_decision_report(SmartSteamNextRoundDecisionReportStatusSource {
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
                    "next_round_decision:display_state:safe-to-wait".to_owned(),
                ],
                reason_codes: vec!["current_round_active".to_owned(), "safe_to_wait".to_owned()],
                ..SmartSteamNextRoundDecisionReportStatusSource::default()
            });

        let dto = SmartSteamStatusSnapshot::from_model_pool(source, &gate, None);
        let decision = dto
            .next_round_decision_status
            .as_ref()
            .expect("current next-round decision report should populate status");

        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_active_round);
        assert!(!dto.mutates_worker_window_status);
        assert_eq!(decision.report_version, "next_round_decision_report_v1");
        assert!(decision.read_only);
        assert!(decision.report_only);
        assert_eq!(decision.status_label, "safe-to-wait-current-round-active");
        assert!(decision.safe_to_wait_current_round_active);
        assert!(!decision.safe_to_continue_after_current_round);
        assert!(!decision.operator_attention_blocked);
        assert_eq!(decision.current_round, Some(369));
        assert_eq!(decision.latest_done_round, Some(368));
        assert_eq!(
            decision.evidence_ids,
            vec![
                "live_status_bundle:next_round_decision:round-369",
                "next_round_decision:display_state:safe-to-wait"
            ]
        );
        assert_eq!(
            decision.reason_codes,
            vec!["current_round_active", "safe_to_wait"]
        );
        assert!(!decision.starts_daemon);
        assert!(!decision.sends_prompt);
        assert!(!decision.starts_stream);
        assert!(!decision.replays_prompt);
        assert!(!decision.writes_ndkv);
        assert!(!decision.creates_thread);
        assert!(!decision.operator_action_required);
        assert!(
            dto.status_line()
                .contains("next_round_decision status=safe-to-wait-current-round-active")
        );
    }

    #[test]
    fn smartsteam_status_snapshot_maps_captured_current_status_json_next_round_decision_fixture() {
        let captured_json = captured_current_status_next_round_decision_json_fixture();
        assert_captured_current_status_next_round_decision_shape(captured_json);
        let report = captured_current_status_next_round_decision_report_from_json(captured_json)
            .expect("captured current-status JSON should expose a next-round decision report");

        assert_eq!(
            report.decision_status.as_deref(),
            Some("safe_to_wait_current_round_active")
        );
        assert_eq!(report.display_state.as_deref(), Some("safe-to-wait"));
        assert_eq!(
            report.live_status_display_state.as_deref(),
            Some("safe-to-wait")
        );
        assert_eq!(report.current_round_active, Some(true));
        assert_eq!(report.current_round, Some(370));
        assert_eq!(report.latest_done_round, Some(369));
        assert_eq!(
            report.evidence_ids,
            vec![
                "live_status_bundle:next_round_decision:round-370",
                "next_round_decision:display_state:safe-to-wait"
            ]
        );
        let downstream_report = report
            .downstream_status_consumers
            .as_ref()
            .expect("captured current-status JSON should carry optional downstream facts");
        assert_eq!(
            downstream_report.service_cli_display_status.as_deref(),
            Some("display_safe_to_wait_current_round")
        );
        assert_eq!(
            downstream_report.forge_operator_display_status.as_deref(),
            Some("forge_safe_to_wait")
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
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(199264), Some(370), Some(369))
            .with_daemon_round_progress(Some(369), true)
            .with_supervisor(true, true)
            .with_readiness(true, true)
            .with_next_round_decision_report(report);

        let dto = SmartSteamStatusSnapshot::from_model_pool(source, &gate, None);
        let decision = dto
            .next_round_decision_status
            .as_ref()
            .expect("captured current-status JSON should surface display-only evidence");

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
            decision.reason_codes,
            vec!["current_round_active", "safe_to_wait"]
        );
        let downstream = decision
            .downstream_status_consumers
            .as_ref()
            .expect("captured current-status JSON should surface downstream display facts");
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
        assert_eq!(downstream.active_round, Some(370));
        assert_eq!(downstream.ledger_latest_round, Some(369));
        let round_id_evidence = downstream
            .round_id_evidence
            .as_ref()
            .expect("captured downstream snapshot should keep round-id evidence");
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
        assert_eq!(
            dto.next_round_decision_summary.as_deref(),
            Some(
                "next_round_decision status=safe-to-wait-current-round-active safe_to_wait_current_round_active=true safe_to_continue_after_current_round=false operator_attention_blocked=false current_round=370 latest_done_round=369 starts_daemon=false sends_prompt=false starts_stream=false writes_ndkv=false creates_thread=false"
            )
        );
        assert!(
            dto.status_line()
                .contains("next_round_decision status=safe-to-wait-current-round-active")
        );
    }

    #[test]
    fn smartsteam_status_snapshot_ignores_next_round_decision_report_side_effect_markers() {
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(199264), Some(369), Some(368))
            .with_daemon_round_progress(Some(368), true)
            .with_supervisor(true, true)
            .with_readiness(true, true)
            .with_next_round_decision_report(SmartSteamNextRoundDecisionReportStatusSource {
                decision_status: Some("safe_to_continue_after_current_round".to_owned()),
                readiness_can_schedule_next_round: Some(true),
                read_only: Some(true),
                report_only: Some(true),
                no_side_effects: Some(true),
                process_start_allowed: Some(true),
                current_round: Some(369),
                latest_done_round: Some(368),
                evidence_ids: vec!["next_round_decision:unsafe-side-effect-marker".to_owned()],
                ..SmartSteamNextRoundDecisionReportStatusSource::default()
            });

        let dto = SmartSteamStatusSnapshot::from_model_pool(source, &gate, None);

        assert_eq!(dto.next_round_decision_status, None);
        assert_eq!(dto.next_round_decision_summary, None);
        assert!(!dto.status_line().contains("next_round_decision"));
        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
    }

    #[test]
    fn captured_current_status_json_next_round_decision_preserves_absence_and_rejects_side_effects()
    {
        assert_eq!(
            captured_current_status_next_round_decision_report_from_json(
                r#"{"daemon_running":true,"live_status_bundle":{"display_state":"safe-to-wait"}}"#
            ),
            None
        );

        let continue_report = captured_current_status_next_round_decision_report_from_json(
            r#"{
  "next_round_decision": {
    "decision_status": "safe_to_continue_after_current_round",
    "display_state": "safe-to-continue-after-current-round",
    "current_round_active": false,
    "readiness_can_schedule_next_round": true,
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
    "latest_done_round": 370,
    "evidence_ids": ["next_round_decision:top-level-safe-continue"],
    "reason_codes": ["safe_to_continue"]
  }
}"#,
        )
        .expect("top-level next_round_decision should parse");
        let continue_decision = continue_report
            .into_status_source()
            .expect("display-only continue report should map");
        assert!(continue_decision.safe_to_continue_after_current_round);
        assert!(!continue_decision.safe_to_wait_current_round_active);
        assert_eq!(
            continue_decision.evidence_ids,
            vec!["next_round_decision:top-level-safe-continue"]
        );
        assert_eq!(continue_decision.downstream_status_consumers, None);

        let unsafe_report = captured_current_status_next_round_decision_report_from_json(
            r#"{
  "next_round_decision_report_v1": {
    "decision_status": "safe_to_continue_after_current_round",
    "readiness_can_schedule_next_round": true,
    "read_only": true,
    "report_only": true,
    "no_side_effects": true,
    "process_start_allowed": true,
    "current_round": 370,
    "latest_done_round": 369,
    "evidence_ids": ["next_round_decision_report_v1:process-start-marker"]
  }
}"#,
        )
        .expect("report-v1 shape should parse before side-effect rejection");

        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let dto = SmartSteamStatusSnapshot::from_model_pool(
            SmartSteamStatusSource::new()
                .with_daemon(true, Some(199264), Some(370), Some(369))
                .with_daemon_round_progress(Some(369), true)
                .with_supervisor(true, true)
                .with_readiness(true, true)
                .with_next_round_decision_report(unsafe_report),
            &gate,
            None,
        );

        assert_eq!(dto.next_round_decision_status, None);
        assert_eq!(dto.next_round_decision_summary, None);
        assert!(!dto.status_line().contains("next_round_decision"));
        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
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
    fn post_r44_safe_to_wait_status_replay_accepts_root_and_live_bundle_downstream_status() {
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
            let dto = SmartSteamStatusSnapshot::from_model_pool(
                SmartSteamStatusSource::new()
                    .with_daemon(true, Some(209816), Some(380), Some(379))
                    .with_daemon_round_progress(Some(379), true)
                    .with_supervisor(true, true)
                    .with_readiness(true, true)
                    .with_next_round_decision_report(report),
                &gate,
                None,
            );
            let decision = dto
                .next_round_decision_status
                .as_ref()
                .unwrap_or_else(|| panic!("{label} post-R44 status should surface decision"));
            let downstream = decision
                .downstream_status_consumers
                .as_ref()
                .unwrap_or_else(|| panic!("{label} post-R44 status should surface downstream"));
            let round_id_evidence = downstream
                .round_id_evidence
                .as_ref()
                .unwrap_or_else(|| panic!("{label} post-R44 status should surface round ids"));

            assert!(dto.read_only, "{label} dto should stay read-only");
            assert!(!dto.launches_process, "{label} dto must not launch");
            assert!(!dto.starts_daemon, "{label} dto must not start daemon");
            assert!(!dto.stops_daemon, "{label} dto must not stop daemon");
            assert!(!dto.touches_remote, "{label} dto must not touch remote");
            assert!(!dto.sends_prompt, "{label} dto must not send prompt");
            assert!(!dto.starts_stream, "{label} dto must not start stream");
            assert!(!dto.replays_prompt, "{label} dto must not replay prompt");
            assert!(
                !dto.mutates_active_round,
                "{label} dto must not mutate rounds"
            );
            assert_eq!(decision.status_label, "safe-to-wait-current-round-active");
            assert!(decision.safe_to_wait_current_round_active);
            assert!(!decision.safe_to_continue_after_current_round);
            assert!(!decision.operator_attention_blocked);
            assert!(!decision.operator_action_required);
            assert_eq!(decision.current_round, Some(380));
            assert_eq!(decision.latest_done_round, Some(379));
            assert_eq!(
                downstream.effective_decision_status,
                "safe_to_wait_current_round_active"
            );
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
            assert_eq!(round_id_evidence.round_in_progress, Some(true));
        }
    }

    #[test]
    fn next_round_decision_report_drops_downstream_side_effect_markers_only() {
        let decision = SmartSteamNextRoundDecisionReportStatusSource {
            decision_status: Some("safe_to_continue_after_current_round".to_owned()),
            display_state: Some("safe-to-continue-after-current-round".to_owned()),
            readiness_can_schedule_next_round: Some(true),
            read_only: Some(true),
            report_only: Some(true),
            no_side_effects: Some(true),
            dispatch_work_allowed: Some(false),
            prompt_replay_allowed: Some(false),
            process_start_allowed: Some(false),
            memory_write_allowed: Some(false),
            ndkv_write_allowed: Some(false),
            downstream_status_consumers: Some(SmartSteamNextRoundDownstreamConsumerStatusSource {
                source_decision_status: Some("safe_to_continue_after_current_round".to_owned()),
                effective_decision_status: Some("safe_to_continue_after_current_round".to_owned()),
                service_cli_display_status: Some("display_safe_to_continue".to_owned()),
                forge_operator_display_status: Some("forge_safe_to_continue".to_owned()),
                agent_assignment_acceptance: Some("accept_next_round_assignment".to_owned()),
                memory_self_improve_admission_visibility: Some("visible_admission_safe".to_owned()),
                read_only: Some(true),
                report_only: Some(true),
                no_side_effects: Some(true),
                dispatch_work_allowed: Some(false),
                prompt_replay_allowed: Some(false),
                process_start_allowed: Some(true),
                memory_write_allowed: Some(false),
                ndkv_write_allowed: Some(false),
                ..SmartSteamNextRoundDownstreamConsumerStatusSource::default()
            }),
            ..SmartSteamNextRoundDecisionReportStatusSource::default()
        }
        .into_status_source()
        .expect("safe core report should still map");

        assert!(decision.safe_to_continue_after_current_round);
        assert_eq!(decision.downstream_status_consumers, None);
    }

    #[test]
    fn next_round_decision_report_prefers_explicit_continue_label_over_active_round_flag() {
        let decision = SmartSteamNextRoundDecisionReportStatusSource {
            decision_status: Some("safe_to_continue_after_current_round".to_owned()),
            display_state: Some("safe-to-continue-after-current-round".to_owned()),
            current_round_active: Some(true),
            readiness_can_schedule_next_round: Some(true),
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
            ..SmartSteamNextRoundDecisionReportStatusSource::default()
        }
        .into_status_source()
        .expect("explicit continue report should map");

        assert!(decision.safe_to_continue_after_current_round);
        assert!(!decision.safe_to_wait_current_round_active);
        assert!(!decision.operator_attention_blocked);
        assert_eq!(decision.current_round, Some(369));
        assert_eq!(decision.latest_done_round, Some(368));
    }

    #[test]
    fn smartsteam_status_snapshot_maps_captured_daemon_json_status_fixture_read_only() {
        let (captured_json, captured) = captured_daemon_json_status_fixture();
        assert_captured_daemon_json_status_shape(captured_json);
        assert!(captured.daemon_round_transition_status_v1.read_only);
        assert!(!captured.daemon_round_transition_status_v1.starts_process);
        assert!(!captured.daemon_round_transition_status_v1.sends_prompt);

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
        let dto = SmartSteamStatusSnapshot::from_model_pool(
            service_source_from_captured_daemon_json_status(captured),
            &gate,
            None,
        );
        let transition = dto
            .daemon_round_transition_status
            .as_ref()
            .expect("captured daemon status should map transition v1");

        assert!(dto.read_only);
        assert!(!dto.launches_process);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_active_round);
        assert!(dto.daemon_running);
        assert_eq!(dto.daemon_pid, Some(235440));
        assert_eq!(dto.active_round, Some(337));
        assert_eq!(dto.ledger_round, Some(336));
        assert_eq!(dto.latest_done_round, Some(336));
        assert!(dto.round_in_progress);
        assert!(dto.engine_busy);
        assert!(dto.readiness_ok);
        assert!(dto.remote_chain_ready);
        assert_eq!(dto.model_cache_label.as_deref(), Some("5/5 OK"));
        assert!(transition.read_only);
        assert!(transition.report_only);
        assert!(!transition.observed_round_done);
        assert_eq!(transition.done_round, None);
        assert_eq!(transition.latest_done_round, Some(336));
        assert!(transition.round_in_progress);
        assert_eq!(transition.ledger_round, Some(336));
        assert!(!transition.ledger_commit_pending);
        assert_eq!(transition.ledger_lag_rounds, Some(0));
        assert_eq!(transition.status_label, "observing");
        assert_eq!(
            transition.reason_codes,
            vec!["normal_in_progress", "active_round_after_latest_done"]
        );
        assert_eq!(
            transition.evidence_ids,
            vec![
                "daemon:status:active-round-337",
                "ledger:latest-done-round-336"
            ]
        );
        assert!(!transition.starts_daemon);
        assert!(!transition.stops_daemon);
        assert!(!transition.touches_remote);
        assert!(!transition.sends_prompt);
        assert!(!transition.starts_stream);
        assert!(!transition.replays_prompt);
        assert!(!transition.mutates_active_round);
        assert!(!transition.writes_ndkv);
        assert!(dto.context_hygiene_status.read_only);
        assert!(dto.context_hygiene_status.report_only);
        assert!(
            dto.context_hygiene_status
                .completed_window_evidence_non_actionable
        );
        assert!(!dto.context_hygiene_status.reads_old_window_payload);
        assert_eq!(
            dto.context_hygiene_status.reason_codes,
            vec![
                "completed_worker_evidence_only",
                "fresh_clean_room_required"
            ]
        );
        assert_eq!(
            dto.daemon_round_transition_summary.as_deref(),
            Some(
                "daemon_round_transition status=observing observed_round_done=false done_round=none latest_done_round=336 round_in_progress=true ledger_round=336 ledger_commit_pending=false ledger_lag_rounds=0 starts_daemon=false stops_daemon=false sends_prompt=false starts_stream=false writes_ndkv=false"
            )
        );
        assert_eq!(
            dto.status_line(),
            "daemon_running=true active_round=337 latest_done_round=336 round_in_progress=true ledger_round=336 readiness_ok=true engine_busy=true remote_chain_ready=true pool=workers total=1 available=0 busy=1 saturated=0 worker_windows=windows total=1 running=0 paused=0 polluted=0 clean_room_replacements_required=1 daemon_round_transition status=observing observed_round_done=false done_round=none latest_done_round=336 round_in_progress=true ledger_round=336 ledger_commit_pending=false ledger_lag_rounds=0 starts_daemon=false stops_daemon=false sends_prompt=false starts_stream=false writes_ndkv=false"
        );
    }

    #[test]
    fn smartsteam_status_snapshot_marks_polluted_windows_for_clean_room_without_side_effects() {
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Worker("127.0.0.1:8686".to_owned()))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(ModelEndpoint::Worker("127.0.0.1:8687".to_owned()))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(224392), Some(315), Some(315))
            .with_supervisor(true, false)
            .with_readiness(true, true)
            .with_worker_windows([
                SmartSteamWorkerWindowStatusSource::new(
                    "019ee199-ae19-7660-8a5a-ff672c3080e0",
                    "service-cli-status",
                )
                .with_polluted("old context saw previous worker instructions")
                .with_replacement_window("019f0000-clean-room-service-cli"),
                SmartSteamWorkerWindowStatusSource::new(
                    "019ee199-7ecc-7e63-b210-63b838c283b4",
                    "eval-test-gates",
                )
                .with_paused("waiting for clean-room replacement prompt"),
                SmartSteamWorkerWindowStatusSource::new(
                    "019ee201-855f-7591-aeda-84f17e171d92",
                    "service-cli-forge-r29",
                )
                .with_completed_evidence_only("completed worker evidence only; do not reassign"),
                SmartSteamWorkerWindowStatusSource::new(
                    "019ee201-333e-7a72-a831-13f74abc6a4d",
                    "eval-test-r29",
                )
                .with_archived("archived clean-room window; future work needs a fresh window"),
                SmartSteamWorkerWindowStatusSource::new(
                    "019ee199-5b56-73c0-9a8c-d0a6beadd869",
                    "agent-coordination",
                ),
                SmartSteamWorkerWindowStatusSource::new(
                    "019f0000-clean-room-service-cli",
                    "service-cli-status",
                )
                .with_clean_room_replacement(),
            ]);

        let dto = SmartSteamStatusSnapshot::from_model_pool(
            source,
            &gate,
            Some(&RoutingIntent::auto_route(
                ModelRole::Reviewer,
                RoutingPreference::PreferFast,
            )),
        );

        assert!(dto.read_only);
        assert!(!dto.starts_clean_room_replacement);
        assert!(!dto.mutates_worker_window_status);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_busy);
        assert!(!dto.mutates_readiness);
        assert!(!dto.mutates_active_round);
        assert!(dto.daemon_running);
        assert!(dto.supervisor_running);
        assert!(dto.readiness_ok);
        assert!(dto.remote_chain_ready);
        assert_eq!(dto.workers_total, 2);
        assert_eq!(dto.workers_available, 2);
        assert_eq!(dto.worker_windows_total, 6);
        assert_eq!(dto.worker_windows_paused, 1);
        assert_eq!(dto.worker_windows_polluted, 1);
        assert_eq!(dto.worker_windows_clean_room_replacements_required, 4);
        assert!(dto.clean_room_replacement_required);
        assert_eq!(
            dto.worker_window_status,
            "windows total=6 running=2 paused=1 polluted=1 clean_room_replacements_required=4"
        );
        assert_eq!(dto.worker_windows[0].status_label, "polluted");
        assert!(!dto.worker_windows[0].assignment_allowed);
        assert!(dto.worker_windows[0].original_window_blocks_assignment);
        assert!(dto.worker_windows[0].future_work_requires_fresh_clean_room);
        assert!(dto.worker_windows[0].clean_room_replacement_required);
        assert_eq!(
            dto.worker_windows[0].replacement_window_id.as_deref(),
            Some("019f0000-clean-room-service-cli")
        );
        assert_eq!(
            dto.worker_windows[0].summary(),
            "window=019ee199-ae19-7660-8a5a-ff672c3080e0 lane=service-cli-status status=polluted assignment_allowed=false original_window_blocks_assignment=true clean_room_replacement_required=true future_work_requires_fresh_clean_room=true replacement=019f0000-clean-room-service-cli reason=old context saw previous worker instructions"
        );
        assert_eq!(dto.worker_windows[1].status_label, "paused");
        assert!(dto.worker_windows[1].clean_room_replacement_required);
        assert_eq!(
            dto.worker_windows[2].status_label,
            "completed-evidence-only"
        );
        assert!(dto.worker_windows[2].completed_evidence_only);
        assert!(!dto.worker_windows[2].assignment_allowed);
        assert!(dto.worker_windows[2].original_window_blocks_assignment);
        assert!(dto.worker_windows[2].future_work_requires_fresh_clean_room);
        assert_eq!(dto.worker_windows[3].status_label, "archived");
        assert!(dto.worker_windows[3].archived);
        assert!(!dto.worker_windows[3].assignment_allowed);
        assert!(dto.worker_windows[3].original_window_blocks_assignment);
        assert!(dto.worker_windows[3].future_work_requires_fresh_clean_room);
        assert_eq!(dto.worker_windows[4].status_label, "running");
        assert!(!dto.worker_windows[4].clean_room_replacement_required);
        assert!(dto.worker_windows[5].clean_room_replacement);
        assert!(dto.worker_windows[5].assignment_allowed);
        assert!(!dto.worker_windows[5].original_window_blocks_assignment);
        assert!(!dto.worker_windows[5].future_work_requires_fresh_clean_room);
        assert!(dto.context_hygiene_status.read_only);
        assert!(dto.context_hygiene_status.report_only);
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
            dto.context_hygiene_status.reason_codes,
            vec![
                "completed_worker_evidence_only",
                "fresh_clean_room_required"
            ]
        );
        assert_eq!(
            dto.status_line(),
            "daemon_running=true active_round=315 latest_done_round=315 round_in_progress=false ledger_round=315 readiness_ok=true engine_busy=false remote_chain_ready=true pool=workers total=2 available=2 busy=0 saturated=0 worker_windows=windows total=6 running=2 paused=1 polluted=1 clean_room_replacements_required=4"
        );
    }

    #[test]
    fn smartsteam_status_snapshot_surfaces_memory_startup_admission_without_side_effects() {
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let evidence = MemoryStartupAdmissionEvidence {
            read_only_review_required: true,
            index_quality_blocker_count: 1,
            index_quality_warning_count: 2,
            index_operation_count: 3,
            index_refresh_count: 1,
            index_detail_codes: vec!["stale_index".to_owned(), "refresh_planned".to_owned()],
            context_rot_risk_count: 2,
            context_rot_blocker_reason_codes: vec!["old_window_payload".to_owned()],
            admission_decision_count: 4,
            admission_accepted_count: 2,
            admission_risk_rejection_count: 1,
            migration_live_store_targeted_count: 0,
            adapter_live_write_count: 0,
            live_write_phase_request_count: 0,
            store_mutation_count: 0,
            helper_prose_line_count: 2,
            non_contract_line_count: 3,
        };
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(224392), Some(316), Some(316))
            .with_supervisor(true, true)
            .with_readiness(true, true)
            .with_memory_startup_admission(evidence);

        let dto = SmartSteamStatusSnapshot::from_model_pool(source, &gate, None);
        let memory = dto
            .memory_startup_admission_status
            .as_ref()
            .expect("memory startup admission status should be surfaced");

        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_busy);
        assert!(!dto.mutates_readiness);
        assert!(!dto.mutates_active_round);
        assert!(memory.read_only_contract);
        assert!(memory.read_only_review_required);
        assert_eq!(memory.index_quality_blocker_count, 1);
        assert_eq!(memory.index_quality_warning_count, 2);
        assert_eq!(memory.index_operation_count, 3);
        assert_eq!(memory.index_refresh_count, 1);
        assert_eq!(
            memory.index_detail_codes,
            vec!["stale_index", "refresh_planned"]
        );
        assert_eq!(memory.context_rot_risk_count, 2);
        assert_eq!(
            memory.context_rot_blocker_reason_codes,
            vec!["old_window_payload"]
        );
        assert_eq!(memory.admission_decision_count, 4);
        assert_eq!(memory.admission_accepted_count, 2);
        assert_eq!(memory.admission_risk_rejection_count, 1);
        assert!(!memory.live_store_mutation_requested);
        assert_eq!(memory.store_mutation_count, 0);
        assert!(!memory.ndkv_write_allowed);
        assert_eq!(memory.helper_prose_line_count, 2);
        assert_eq!(memory.non_contract_line_count, 3);
        assert!(!memory.admission_expanded_by_non_contract_evidence);
        assert_eq!(
            dto.memory_startup_admission_summary.as_deref(),
            Some(
                "memory_startup_admission read_only_contract=true review=true index_blockers=1 index_warnings=2 index_ops=3 index_refresh=1 context_rot_risks=2 admission_decisions=4 admission_accepted=2 admission_risk_rejections=1 live_store_mutation_requested=false store_mutations=0 ndkv_write_allowed=false helper_prose_lines=2 non_contract_lines=3 admission_expanded_by_non_contract=false"
            )
        );
        assert!(
            dto.status_line()
                .contains("memory_startup_admission read_only_contract=true")
        );
    }

    #[test]
    fn smartsteam_status_snapshot_surfaces_clean_room_handoff_without_side_effects() {
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let evidence = MemoryStartupAdmissionEvidence {
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
        let handoff = SmartSteamCleanRoomHandoffStatusSource::new()
            .with_agent_replacement_plan(true, true, true)
            .with_original_window_follow_up_assignment_allowed(false)
            .with_old_window_payload_read(false)
            .with_thread_side_effects(false, false)
            .with_evidence_result_ids(["handoff-summary:r24-agent"])
            .with_reason_codes(["window_context_polluted", "paused_by_main_window"]);
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(230076), Some(322), Some(321))
            .with_supervisor(true, true)
            .with_readiness(true, true)
            .with_memory_startup_admission(evidence)
            .with_worker_window(
                SmartSteamWorkerWindowStatusSource::new(
                    "019ee1c4-3b94-7cb0-a870-b1cb0e7b11e4",
                    "agent-clean-room-assignment",
                )
                .with_polluted("old window pollution blocks follow-up assignment"),
            )
            .with_clean_room_handoff(handoff);

        let dto = SmartSteamStatusSnapshot::from_model_pool(source, &gate, None);
        let handoff = dto
            .clean_room_handoff_status
            .as_ref()
            .expect("clean-room handoff status should be surfaced");

        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_busy);
        assert!(!dto.mutates_readiness);
        assert!(!dto.mutates_active_round);
        assert!(handoff.read_only);
        assert!(handoff.report_only);
        assert!(handoff.pure_data_only);
        assert!(handoff.memory_admission_safe);
        assert!(handoff.agent_replacement_plan_required);
        assert!(handoff.agent_replacement_plan_available);
        assert!(handoff.replacement_prompt_ready);
        assert!(!handoff.original_window_follow_up_assignment_allowed);
        assert!(handoff.original_window_follow_up_blocked);
        assert!(!handoff.reads_old_window_payload);
        assert!(handoff.old_window_payload_ignored);
        assert!(!handoff.starts_thread);
        assert!(!handoff.sends_message);
        assert!(!handoff.starts_clean_room_replacement);
        assert!(!handoff.mutates_worker_window_status);
        assert!(!handoff.live_write_allowed);
        assert!(!handoff.live_store_mutation_allowed);
        assert!(!handoff.ndkv_write_allowed);
        assert!(!handoff.runtime_side_effects_allowed);
        assert_eq!(
            handoff.evidence_result_ids,
            vec!["handoff-summary:r24-agent"]
        );
        assert_eq!(
            handoff.reason_codes,
            vec!["window_context_polluted", "paused_by_main_window"]
        );
        assert_eq!(
            dto.clean_room_handoff_summary.as_deref(),
            Some(
                "clean_room_handoff memory_admission_safe=true agent_replacement_plan_required=true agent_replacement_plan_available=true replacement_prompt_ready=true original_window_follow_up_blocked=true reads_old_window_payload=false live_write_allowed=false live_store_mutation_allowed=false ndkv_write_allowed=false runtime_side_effects_allowed=false"
            )
        );
        assert!(
            dto.status_line()
                .contains("clean_room_handoff memory_admission_safe=true")
        );
    }

    #[test]
    fn smartsteam_status_snapshot_surfaces_helper_stage_repair_required_without_side_effects() {
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(230076), Some(324), Some(323))
            .with_supervisor(true, true)
            .with_readiness(true, true)
            .with_helper_stage_repair(
                SmartSteamHelperStageRepairStatusSource::new(
                    "review",
                    SmartSteamHelperStageRepairState::RepairRequired,
                )
                .with_source_round(324)
                .with_evidence_ids(["round-324:helper-stage:review"])
                .with_reason_codes([
                    "helper_stage_contract_incomplete",
                    "missing_required_contract_fields",
                ])
                .with_missing_helper_role_repair_proposals([
                    SmartSteamMissingHelperRoleRepairProposalStatusSource::new(
                        "helper-stage-repair-r324-router",
                        "router",
                    )
                    .with_reason_codes(["required_helper_role_missing"]),
                ]),
            );

        let dto = SmartSteamStatusSnapshot::from_model_pool(source, &gate, None);
        let repair = dto
            .helper_stage_repair_status
            .as_ref()
            .expect("helper-stage repair status should be surfaced");

        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_busy);
        assert!(!dto.mutates_readiness);
        assert!(!dto.mutates_active_round);
        assert!(repair.read_only);
        assert!(repair.report_only);
        assert!(repair.pure_data_only);
        assert_eq!(repair.stage_label, "review");
        assert_eq!(
            repair.state,
            SmartSteamHelperStageRepairState::RepairRequired
        );
        assert_eq!(repair.state_label, "repair-required");
        assert!(!repair.helper_stage_contract_complete);
        assert!(repair.helper_stage_repair_required);
        assert_eq!(repair.source_round, Some(324));
        assert_eq!(repair.evidence_ids, vec!["round-324:helper-stage:review"]);
        assert_eq!(
            repair.reason_codes,
            vec![
                "helper_stage_contract_incomplete",
                "missing_required_contract_fields"
            ]
        );
        assert!(repair.missing_helper_role_repair_required);
        assert_eq!(repair.missing_helper_role_repair_proposal_count, 1);
        assert_eq!(repair.missing_helper_roles, vec!["router"]);
        let missing_role_proposal = repair
            .missing_helper_role_repair_proposals
            .first()
            .expect("missing helper-role proposal should be visible");
        assert!(missing_role_proposal.read_only);
        assert!(missing_role_proposal.report_only);
        assert!(missing_role_proposal.pure_data_only);
        assert_eq!(
            missing_role_proposal.proposal_id,
            "helper-stage-repair-r324-router"
        );
        assert_eq!(missing_role_proposal.role_label, "router");
        assert!(missing_role_proposal.repair_required);
        assert_eq!(
            missing_role_proposal.reason_codes,
            vec!["required_helper_role_missing"]
        );
        assert!(!missing_role_proposal.parses_helper_prose);
        assert!(!missing_role_proposal.replays_prompt);
        assert!(!missing_role_proposal.calls_model);
        assert!(!missing_role_proposal.sends_prompt);
        assert!(!missing_role_proposal.starts_stream);
        assert!(!missing_role_proposal.writes_memory);
        assert!(!missing_role_proposal.writes_ndkv);
        assert!(!missing_role_proposal.mutates_live_store);
        assert!(!missing_role_proposal.starts_clean_room_replacement);
        assert!(!missing_role_proposal.mutates_worker_window_status);
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
            dto.helper_stage_repair_summary.as_deref(),
            Some(
                "helper_stage_repair stage=review state=repair-required contract_complete=false repair_required=true missing_helper_role_repair_required=true missing_helper_role_repair_proposals=1 missing_helper_roles=router reasons=2 evidence=1 parses_helper_prose=false writes_memory=false writes_ndkv=false runtime_side_effects_allowed=false"
            )
        );
        assert!(
            dto.status_line()
                .contains("helper_stage_repair stage=review state=repair-required")
        );
    }

    #[test]
    fn smartsteam_status_snapshot_surfaces_self_improve_proposals_without_side_effects() {
        let gate = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)],
        );
        let proposal = |id: &str,
                        lifecycle: SmartSteamSelfImproveProposalLifecycle,
                        round: u64,
                        validation_checked: bool,
                        validation_passed: bool,
                        memory_checked: bool,
                        memory_admitted: bool,
                        memory_quarantined: bool| {
            SmartSteamSelfImproveProposalStatusSource::new(id, lifecycle)
                .with_source_round(round)
                .with_evidence_ids([format!("round-{round}:proposal:{id}")])
                .with_validation_status(
                    SmartSteamSelfImproveProposalValidationStatusSource::new(
                        validation_checked,
                        validation_passed,
                    )
                    .with_status_code(if validation_passed { 0 } else { 1 })
                    .with_evidence_ids([format!("round-{round}:validation:{id}")]),
                )
                .with_memory_admission_status(
                    SmartSteamSelfImproveProposalMemoryAdmissionStatusSource::new(
                        memory_checked,
                        memory_admitted,
                        memory_quarantined,
                    )
                    .with_evidence_ids([format!("round-{round}:memory:{id}")]),
                )
        };
        let source = SmartSteamStatusSource::new()
            .with_daemon(true, Some(230076), Some(324), Some(323))
            .with_supervisor(true, true)
            .with_readiness(true, true)
            .with_self_improve_proposals([
                proposal(
                    "candidate-001",
                    SmartSteamSelfImproveProposalLifecycle::Candidate,
                    324,
                    false,
                    false,
                    false,
                    false,
                    false,
                ),
                proposal(
                    "validated-001",
                    SmartSteamSelfImproveProposalLifecycle::Validated,
                    323,
                    true,
                    true,
                    false,
                    false,
                    false,
                ),
                proposal(
                    "admitted-001",
                    SmartSteamSelfImproveProposalLifecycle::Admitted,
                    322,
                    true,
                    true,
                    true,
                    true,
                    false,
                ),
                proposal(
                    "quarantined-001",
                    SmartSteamSelfImproveProposalLifecycle::Quarantined,
                    321,
                    true,
                    false,
                    true,
                    false,
                    true,
                ),
                proposal(
                    "promoted-001",
                    SmartSteamSelfImproveProposalLifecycle::Promoted,
                    320,
                    true,
                    true,
                    true,
                    true,
                    false,
                ),
                proposal(
                    "repair-001",
                    SmartSteamSelfImproveProposalLifecycle::RepairRequired,
                    319,
                    true,
                    false,
                    true,
                    false,
                    false,
                ),
            ])
            .with_self_improve_proposal_prompt_guidance(
                SmartSteamSelfImproveProposalPromptGuidanceSource::new(true, false, true)
                    .with_evidence_ids(["round-324:self-improve-proposal-guidance"]),
            );

        let dto = SmartSteamStatusSnapshot::from_model_pool(source, &gate, None);
        let proposals = dto
            .self_improve_proposal_status
            .as_ref()
            .expect("self-improve proposal status should be surfaced");

        assert!(dto.read_only);
        assert!(!dto.starts_daemon);
        assert!(!dto.stops_daemon);
        assert!(!dto.touches_remote);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert!(!dto.replays_prompt);
        assert!(!dto.mutates_busy);
        assert!(!dto.mutates_readiness);
        assert!(!dto.mutates_active_round);
        assert!(proposals.read_only);
        assert!(proposals.report_only);
        assert!(proposals.pure_data_only);
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
            .expect("self-improve proposal prompt guidance should be surfaced");
        assert!(guidance.read_only);
        assert!(guidance.report_only);
        assert!(guidance.pure_data_only);
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
            proposals.proposals[0].evidence_ids,
            vec!["round-324:proposal:candidate-001"]
        );
        assert_eq!(
            proposals.proposals[0].validation_status.status_label,
            "not-checked"
        );
        assert_eq!(
            proposals.proposals[1].validation_status.evidence_ids,
            vec!["round-323:validation:validated-001"]
        );
        assert_eq!(
            proposals.proposals[2].memory_admission_status.status_label,
            "admitted"
        );
        assert_eq!(
            proposals.proposals[3].memory_admission_status.status_label,
            "quarantined"
        );
        assert_eq!(
            proposals.proposals[5].validation_status.status_label,
            "failed"
        );
        assert!(
            proposals
                .proposals
                .iter()
                .all(|proposal| proposal.read_only)
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
            dto.self_improve_proposal_summary.as_deref(),
            Some(
                "self_improve_proposals total=6 candidate=1 validated=1 admitted=1 quarantined=1 promoted=1 repair-required=1 writes_memory=false writes_ndkv=false runtime_side_effects_allowed=false convert_advisory_to_business_evidence=true repair_unvalidated_or_unaccepted=false requires_validation_and_memory_admission=true"
            )
        );
        assert!(
            dto.status_line()
                .contains("self_improve_proposals total=6 candidate=1")
        );
        assert!(
            dto.status_line()
                .contains("convert_advisory_to_business_evidence=true")
        );
    }

    #[test]
    fn workers_host_snapshot_keeps_external_model_cache_diagnostics_out_of_service_dto() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let intent = RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast);
        let dto = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(endpoint(8686))
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(endpoint(8687))
                    .with_roles([ModelRole::Summarizer])
                    .with_preferences([RoutingPreference::Balanced]),
            ],
        )
        .route_snapshot(&intent)
        .workers_host_snapshot();

        assert!(dto.read_only);
        assert!(!dto.launches_process);
        assert!(!dto.sends_prompt);
        assert!(!dto.starts_stream);
        assert_eq!(
            dto.pool_status,
            "workers total=2 available=2 busy=0 saturated=0"
        );

        let host_fields = model_pool_workers_host_json_fields(&dto);
        let worker_fields = model_worker_host_json_fields(&dto.workers[0]);
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
                "{external_field} must remain an external diagnostic, not a service host DTO field"
            );
            assert!(
                !worker_fields.contains(&external_field),
                "{external_field} must remain an external diagnostic, not a service worker DTO field"
            );
        }
    }

    #[test]
    fn workers_host_snapshot_json_field_names_are_stable_for_8686_8690() {
        let endpoint = |port: u16| ModelEndpoint::Worker(format!("127.0.0.1:{port}"));
        let intent = RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast);
        let dto = ModelPoolGateSnapshot::new(
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
        )
        .route_snapshot(&intent)
        .workers_host_snapshot();

        assert_eq!(
            model_pool_workers_host_json_fields(&dto),
            vec![
                "read_only",
                "launches_process",
                "sends_prompt",
                "starts_stream",
                "carries_request_preview",
                "mutates_history",
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
                "decision_action_label",
                "decision_state_label",
                "decision_reason",
                "pool_status",
                "route_pool_status",
                "workers",
            ]
        );
        assert_eq!(
            model_worker_host_json_fields(&dto.workers[0]),
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
        assert_eq!(dto.workers[0].endpoint_label, "127.0.0.1:8686");
        assert_eq!(dto.workers[1].worker_status_label, "busy");
        assert_eq!(dto.workers[2].picker_action_label, "unavailable");
        assert_eq!(dto.workers[3].worker_status_label, "backpressure");
        assert_eq!(dto.workers[4].endpoint_label, "127.0.0.1:8690");
    }

    #[test]
    fn next_round_decision_report_v1_json_field_names_are_stable_for_consumers() {
        let report = captured_current_status_next_round_decision_report_from_json(
            captured_current_status_next_round_decision_json_fixture(),
        )
        .expect("fixture should parse next-round report");
        let decision = report
            .into_status_source()
            .expect("fixture report should map to service display status");
        let snapshot = SmartSteamNextRoundDecisionStatusSnapshot::from_source(decision);
        let downstream = snapshot
            .downstream_status_consumers
            .as_ref()
            .expect("fixture should expose downstream consumer display facts");
        let round_id_evidence = downstream
            .round_id_evidence
            .as_ref()
            .expect("fixture should expose downstream round-id evidence");

        assert_eq!(
            smartsteam_next_round_decision_json_fields(&snapshot),
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
        assert!(snapshot.read_only);
        assert!(snapshot.report_only);
        assert!(!snapshot.starts_daemon);
        assert!(!snapshot.sends_prompt);
        assert!(!snapshot.starts_stream);
        assert!(!snapshot.writes_ndkv);
        assert!(!snapshot.creates_thread);
        assert!(downstream.read_only);
        assert!(downstream.report_only);
        assert!(downstream.no_side_effects);
        assert!(!downstream.dispatch_work_allowed);
        assert!(!downstream.prompt_replay_allowed);
        assert!(!downstream.process_start_allowed);
        assert!(!downstream.memory_write_allowed);
        assert!(!downstream.ndkv_write_allowed);
    }

    fn smartsteam_status_json_fields(dto: &SmartSteamStatusSnapshot) -> Vec<&'static str> {
        let SmartSteamStatusSnapshot {
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
            daemon_running: _daemon_running,
            daemon_pid: _daemon_pid,
            supervisor_running: _supervisor_running,
            supervisor_check_only: _supervisor_check_only,
            active_round: _active_round,
            ledger_round: _ledger_round,
            latest_done_round: _latest_done_round,
            round_in_progress: _round_in_progress,
            readiness_ok: _readiness_ok,
            engine_busy: _engine_busy,
            active_request: _active_request,
            remote_chain_ready: _remote_chain_ready,
            model_cache_label: _model_cache_label,
            worker_windows_total: _worker_windows_total,
            worker_windows_paused: _worker_windows_paused,
            worker_windows_polluted: _worker_windows_polluted,
            worker_windows_clean_room_replacements_required:
                _worker_windows_clean_room_replacements_required,
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
            workers_total: _workers_total,
            workers_available: _workers_available,
            workers_busy: _workers_busy,
            workers_saturated: _workers_saturated,
            pool_status: _pool_status,
            route_pool_status: _route_pool_status,
            route_send_allowed: _route_send_allowed,
            route_send_block_reason: _route_send_block_reason,
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
            "daemon_running",
            "daemon_pid",
            "supervisor_running",
            "supervisor_check_only",
            "active_round",
            "ledger_round",
            "latest_done_round",
            "round_in_progress",
            "readiness_ok",
            "engine_busy",
            "active_request",
            "remote_chain_ready",
            "model_cache_label",
            "worker_windows_total",
            "worker_windows_paused",
            "worker_windows_polluted",
            "worker_windows_clean_room_replacements_required",
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
            "workers_total",
            "workers_available",
            "workers_busy",
            "workers_saturated",
            "pool_status",
            "route_pool_status",
            "route_send_allowed",
            "route_send_block_reason",
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
        dto: &SmartSteamNextRoundDownstreamConsumerStatusSnapshot,
    ) -> Vec<&'static str> {
        let SmartSteamNextRoundDownstreamConsumerStatusSnapshot {
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
        dto: &SmartSteamNextRoundRoundIdEvidenceSnapshot,
    ) -> Vec<&'static str> {
        let SmartSteamNextRoundRoundIdEvidenceSnapshot {
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

    fn model_pool_workers_host_json_fields(
        dto: &ModelPoolWorkersHostSnapshot,
    ) -> Vec<&'static str> {
        let ModelPoolWorkersHostSnapshot {
            read_only: _read_only,
            launches_process: _launches_process,
            sends_prompt: _sends_prompt,
            starts_stream: _starts_stream,
            carries_request_preview: _carries_request_preview,
            mutates_history: _mutates_history,
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
            decision_action_label: _decision_action_label,
            decision_state_label: _decision_state_label,
            decision_reason: _decision_reason,
            pool_status: _pool_status,
            route_pool_status: _route_pool_status,
            workers: _workers,
        } = dto;

        vec![
            "read_only",
            "launches_process",
            "sends_prompt",
            "starts_stream",
            "carries_request_preview",
            "mutates_history",
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
            "decision_action_label",
            "decision_state_label",
            "decision_reason",
            "pool_status",
            "route_pool_status",
            "workers",
        ]
    }

    fn model_worker_host_json_fields(dto: &ModelWorkerHostSnapshot) -> Vec<&'static str> {
        let ModelWorkerHostSnapshot {
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
    fn route_snapshot_keeps_backend_offline_gate_over_worker_availability() {
        let snapshot = ModelPoolGateSnapshot::new(
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
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);

        let route = snapshot.route_snapshot(&request.routing_intent());

        assert_eq!(
            route.decision,
            GateDecision::blocked(StreamState::Failed, "backend is offline")
        );
        assert_eq!(route.decision_action_label, "repair_gate");
        assert_eq!(route.decision_state_label, "failed");
        assert!(route.decision_state_is_terminal);
        assert!(!route.decision_state_is_pressure);
        assert!(!route.decision_state_blocks_prompt_submit);
        assert!(!route.send_allowed);
        assert_eq!(route.send_block_state, Some(StreamState::Failed));
        assert_eq!(route.send_block_state_label.as_deref(), Some("failed"));
        assert!(route.send_block_state_is_terminal);
        assert!(!route.send_block_state_is_pressure);
        assert!(!route.send_block_state_blocks_prompt_submit);
        let send_block_chunk = route
            .send_block_chunk
            .as_ref()
            .expect("offline gate should expose a display chunk");
        assert_eq!(send_block_chunk.output_label, "error");
        assert_eq!(send_block_chunk.appended, "[error] backend is offline");

        assert_eq!(
            route.pool_status,
            "workers total=1 available=1 busy=0 saturated=0"
        );
        assert_eq!(
            route.route_pool_status,
            "matching total=1 available=1 busy=0 saturated=0"
        );
        assert_eq!(route.workers.len(), 1);
        assert!(route.workers[0].route_match);
        assert!(!route.workers[0].selectable);
        assert_eq!(
            route.workers[0].picker_action,
            ModelRouteWorkerPickerAction::RepairGate
        );
        assert_eq!(route.workers[0].picker_action_label, "repair_gate");
        assert_eq!(
            route.workers[0].decision,
            GateDecision::blocked(StreamState::Failed, "backend is offline")
        );
        assert_eq!(route.workers[0].decision_reason(), "backend is offline");
        assert_eq!(route.workers[0].worker_status_label(), "available");

        let blocked_snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                backend_online: false,
                engine_busy: true,
                safe_device_ok: false,
                experience_hygiene_ok: false,
                queued_requests: 4,
                queue_limit: 4,
                active_request: Some("#9 chat-stream".to_owned()),
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8 review".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(2, 2),
            ],
        );

        let blocked_route = blocked_snapshot.route_snapshot(&request.routing_intent());

        assert_eq!(
            blocked_route.decision,
            GateDecision::blocked(StreamState::Failed, "backend is offline")
        );
        assert_eq!(blocked_route.decision_action_label, "repair_gate");
        assert_eq!(blocked_route.decision_state_label, "failed");
        assert!(blocked_route.decision_state_is_terminal);
        assert!(!blocked_route.decision_state_is_pressure);
        assert!(!blocked_route.decision_state_blocks_prompt_submit);
        assert!(!blocked_route.send_allowed);
        assert_eq!(blocked_route.send_block_state, Some(StreamState::Failed));
        assert_eq!(
            blocked_route.send_block_state_label.as_deref(),
            Some("failed")
        );
        assert!(blocked_route.send_block_state_is_terminal);
        assert!(!blocked_route.send_block_state_is_pressure);
        assert!(!blocked_route.send_block_state_blocks_prompt_submit);
        let blocked_chunk = blocked_route
            .send_block_chunk
            .as_ref()
            .expect("offline gate should expose a display chunk");
        assert_eq!(blocked_chunk.appended, "[error] backend is offline");
        assert_eq!(
            blocked_route.pool_status,
            "workers total=2 available=0 busy=1 saturated=1"
        );
        assert_eq!(
            blocked_route.route_pool_status,
            "matching total=2 available=0 busy=1 saturated=1"
        );
        assert_eq!(blocked_route.workers.len(), 2);
        assert!(
            blocked_route
                .workers
                .iter()
                .all(|worker| worker.route_match)
        );
        assert!(
            blocked_route
                .workers
                .iter()
                .all(|worker| !worker.selectable)
        );
        assert!(blocked_route.workers.iter().all(|worker| {
            worker.decision == GateDecision::blocked(StreamState::Failed, "backend is offline")
        }));
        assert_eq!(blocked_route.workers[0].picker_action_label, "repair_gate");
        assert_eq!(blocked_route.workers[1].picker_action_label, "repair_gate");
        assert_eq!(blocked_route.workers[0].worker_status_label(), "busy");
        assert_eq!(
            blocked_route.workers[1].worker_status_label(),
            "backpressure"
        );
    }

    #[test]
    fn route_snapshot_keeps_experience_repair_gate_over_worker_pressure() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot {
                experience_hygiene_ok: false,
                ..FrontendGateSnapshot::default()
            },
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_busy(true, Some("#8 review".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast])
                    .with_queue(2, 2),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);

        let route = snapshot.route_snapshot(&request.routing_intent());

        assert_eq!(
            route.decision,
            GateDecision::blocked(StreamState::Failed, "experience hygiene gate failed")
        );
        assert_eq!(route.decision_action_label, "repair_gate");
        assert_eq!(route.decision_state_label, "failed");
        assert!(route.decision_state_is_terminal);
        assert!(!route.decision_state_is_pressure);
        assert!(!route.decision_state_blocks_prompt_submit);
        assert!(!route.send_allowed);
        assert_eq!(route.send_block_state, Some(StreamState::Failed));
        assert_eq!(route.send_block_state_label.as_deref(), Some("failed"));
        assert!(route.send_block_state_is_terminal);
        assert!(!route.send_block_state_is_pressure);
        assert!(!route.send_block_state_blocks_prompt_submit);
        let send_block_chunk = route
            .send_block_chunk
            .as_ref()
            .expect("experience repair gate should expose a display chunk");
        assert_eq!(send_block_chunk.output_label, "error");
        assert_eq!(
            send_block_chunk.appended,
            "[error] experience hygiene gate failed"
        );

        assert_eq!(
            route.pool_status,
            "workers total=2 available=0 busy=1 saturated=1"
        );
        assert_eq!(route.pool_queue_label, "2/3");
        assert_eq!(route.pool_capacity_state, StreamState::Backpressure);
        assert_eq!(route.pool_capacity_state_label, "backpressure");
        assert!(route.pool_capacity_state_is_pressure);
        assert!(route.pool_capacity_state_blocks_prompt_submit);
        assert_eq!(
            route.route_pool_status,
            "matching total=2 available=0 busy=1 saturated=1"
        );
        assert_eq!(route.route_pool_capacity_state, StreamState::Backpressure);
        assert_eq!(route.workers.len(), 2);
        assert!(route.workers.iter().all(|worker| worker.route_match));
        assert!(route.workers.iter().all(|worker| !worker.selectable));
        assert!(route.workers.iter().all(|worker| {
            worker.decision
                == GateDecision::blocked(StreamState::Failed, "experience hygiene gate failed")
        }));
        assert_eq!(route.workers[0].worker_status_label(), "busy");
        assert_eq!(route.workers[1].worker_status_label(), "backpressure");
        assert_eq!(route.workers[0].picker_action_label, "repair_gate");
        assert_eq!(route.workers[1].picker_action_label, "repair_gate");
    }

    #[test]
    fn route_workers_scopes_route_match_to_operator_pinned_endpoint() {
        let snapshot = ModelPoolGateSnapshot::new(
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
        let request = ChatRequest::new("s1", vec![ChatMessage::user("deep answer")])
            .with_model_role(ModelRole::Assistant)
            .with_routing_preference(RoutingPreference::PreferQuality)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));

        let workers = snapshot.route_workers(&request.routing_intent());

        assert_eq!(workers.len(), 2);
        assert!(!workers[0].endpoint_selected);
        assert!(!workers[0].route_match);
        assert!(workers[0].selectable);
        assert_eq!(
            workers[0].picker_action,
            ModelRouteWorkerPickerAction::Select
        );
        assert_eq!(workers[0].picker_action_label, "select");
        assert_eq!(workers[0].decision, GateDecision::Allowed);
        assert!(workers[1].endpoint_selected);
        assert!(workers[1].route_match);
        assert!(!workers[1].selectable);
        assert_eq!(
            workers[1].picker_action,
            ModelRouteWorkerPickerAction::Current
        );
        assert_eq!(workers[1].picker_action_label, "current");
        assert_eq!(
            workers[1].decision,
            GateDecision::blocked(
                StreamState::Busy,
                "worker fast-reviewer is busy: #13 pinned"
            )
        );
    }

    #[test]
    fn route_snapshot_packages_decision_capacity_and_worker_picker_state() {
        let snapshot = ModelPoolGateSnapshot::new(
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
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_role(ModelRole::Reviewer)
            .with_routing_preference(RoutingPreference::PreferFast);

        let route = snapshot.route_snapshot(&request.routing_intent());

        assert_eq!(route.intent.model_role, ModelRole::Reviewer);
        assert_eq!(
            route.route,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
        assert_eq!(route.model_role_label, "reviewer");
        assert_eq!(route.routing_preference_label, "prefer_fast");
        assert_eq!(route.endpoint_label, "auto");
        assert!(!route.endpoint_pinned);
        assert_eq!(route.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(route.endpoint_kind_label, "auto");
        assert!(route.endpoint_auto);
        assert!(!route.endpoint_built_in);
        assert!(!route.endpoint_custom);
        assert_eq!(route.wire_model_role_label, "reviewer");
        assert_eq!(route.wire_routing_preference_label, "prefer_fast");
        assert!(route.wire_prefer_fast);
        assert!(!route.wire_prefer_quality);
        assert!(!route.wire_endpoint_pinned);
        assert_eq!(route.wire_endpoint_kind_label, "auto");
        assert!(!route.wire_sends_model_endpoint);
        assert_eq!(route.wire_model_endpoint_label, None);
        assert_eq!(route.pool.total_workers, 2);
        assert_eq!(route.pool.busy_workers, 1);
        assert_eq!(
            route.pool_status,
            "workers total=2 available=1 busy=1 saturated=0"
        );
        assert_eq!(route.pool_queue_label, "0/2");
        assert_eq!(route.pool_capacity_state, StreamState::Pending);
        assert_eq!(route.pool_capacity_state_label, "pending");
        assert!(!route.pool_capacity_state_is_pressure);
        assert!(!route.pool_capacity_state_blocks_prompt_submit);
        assert_eq!(route.route_pool.matching_workers, 1);
        assert_eq!(route.route_pool.matching_busy_workers, 1);
        assert_eq!(
            route.route_pool_status,
            "matching total=1 available=0 busy=1 saturated=0"
        );
        assert_eq!(route.route_pool_queue_label, "0/1");
        assert_eq!(route.route_pool_capacity_state, StreamState::Busy);
        assert_eq!(route.route_pool_capacity_state_label, "busy");
        assert!(route.route_pool_capacity_state_is_pressure);
        assert!(route.route_pool_capacity_state_blocks_prompt_submit);
        assert_eq!(
            route.decision,
            GateDecision::blocked(
                StreamState::Queued,
                "all matching model workers are busy; waiting for scheduler across 1 workers"
            )
        );
        assert_eq!(
            route.decision_advice.action,
            GateAdviceAction::WaitForWorker
        );
        assert_eq!(route.decision_advice.state, StreamState::Queued);
        assert_eq!(route.decision_action_label, "wait_for_worker");
        assert_eq!(route.decision_state_label, "queued");
        assert!(!route.decision_state_is_terminal);
        assert!(route.decision_state_is_pressure);
        assert!(route.decision_state_blocks_prompt_submit);
        assert_eq!(
            route.decision_reason,
            "all matching model workers are busy; waiting for scheduler across 1 workers"
        );
        assert!(!route.send_allowed);
        assert_eq!(route.send_block_state, Some(StreamState::Queued));
        assert_eq!(route.send_block_state_label.as_deref(), Some("queued"));
        assert!(!route.send_block_state_is_terminal);
        assert!(route.send_block_state_is_pressure);
        assert!(route.send_block_state_blocks_prompt_submit);
        let send_block_chunk = route
            .send_block_chunk
            .as_ref()
            .expect("route pressure should expose a display chunk");
        assert_eq!(send_block_chunk.output_label, "queued");
        assert_eq!(
            send_block_chunk.appended,
            "[queued] all matching model workers are busy; waiting for scheduler across 1 workers"
        );
        assert!(send_block_chunk.state_blocks_prompt_submit);
        assert_eq!(route.workers.len(), 2);
        assert!(!route.workers[0].route_match);
        assert!(route.workers[1].route_match);
        assert!(!route.workers[1].selectable);

        let pinned = snapshot.route_snapshot(&RoutingIntent::operator_pinned(
            ModelRole::Reviewer,
            RoutingPreference::PreferFast,
            ModelEndpoint::FastReviewer,
        ));
        assert_eq!(pinned.endpoint_label, "fast-reviewer");
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
        let pinned_block_chunk = pinned
            .send_block_chunk
            .as_ref()
            .expect("pinned busy worker should expose a display chunk");
        assert_eq!(pinned_block_chunk.output_label, "busy");
        assert_eq!(
            pinned_block_chunk.appended,
            "[busy] worker fast-reviewer is busy: #8 review"
        );

        let custom = snapshot.route_snapshot(&RoutingIntent::operator_pinned(
            ModelRole::Reviewer,
            RoutingPreference::PreferFast,
            ModelEndpoint::Worker("mlx-reviewer-8b".to_owned()),
        ));
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
    fn route_snapshot_maps_auto_route_saturated_match_to_backpressure() {
        let snapshot = ModelPoolGateSnapshot::new(
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
        let intent = RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast);

        let route = snapshot.route_snapshot(&intent);

        assert_eq!(
            route.route,
            "role=reviewer preference=prefer_fast endpoint=auto pinned=false"
        );
        assert_eq!(route.model_role_label, "reviewer");
        assert_eq!(route.routing_preference_label, "prefer_fast");
        assert_eq!(route.endpoint_label, "auto");
        assert!(!route.endpoint_pinned);
        assert!(!route.wire_endpoint_pinned);
        assert!(!route.wire_sends_model_endpoint);
        assert_eq!(
            route.pool_status,
            "workers total=2 available=1 busy=0 saturated=1"
        );
        assert_eq!(route.pool_queue_label, "1/2");
        assert_eq!(route.pool_capacity_state, StreamState::Queued);
        assert_eq!(route.pool_capacity_state_label, "queued");
        assert!(route.pool_capacity_state_is_pressure);
        assert!(route.pool_capacity_state_blocks_prompt_submit);
        assert_eq!(
            route.route_pool_status,
            "matching total=1 available=0 busy=0 saturated=1"
        );
        assert_eq!(route.route_pool_queue_label, "1/1");
        assert_eq!(route.route_pool_capacity_state, StreamState::Backpressure);
        assert_eq!(route.route_pool_capacity_state_label, "backpressure");
        assert!(route.route_pool_capacity_state_is_pressure);
        assert!(route.route_pool_capacity_state_blocks_prompt_submit);
        assert_eq!(
            route.decision,
            GateDecision::blocked(
                StreamState::Backpressure,
                "matching model workers are saturated: 1 workers"
            )
        );
        assert_eq!(route.decision_advice.action, GateAdviceAction::RetryLater);
        assert_eq!(route.decision_advice.state, StreamState::Backpressure);
        assert_eq!(route.decision_action_label, "retry_later");
        assert_eq!(route.decision_state_label, "backpressure");
        assert!(!route.decision_state_is_terminal);
        assert!(route.decision_state_is_pressure);
        assert!(route.decision_state_blocks_prompt_submit);
        assert_eq!(
            route.decision_reason,
            "matching model workers are saturated: 1 workers"
        );
        assert!(!route.send_allowed);
        assert_eq!(route.send_block_state, Some(StreamState::Backpressure));
        assert_eq!(
            route.send_block_state_label.as_deref(),
            Some("backpressure")
        );
        let send_block_chunk = route
            .send_block_chunk
            .as_ref()
            .expect("route backpressure should expose a display chunk");
        assert_eq!(send_block_chunk.output_label, "backpressure");
        assert_eq!(
            send_block_chunk.appended,
            "[backpressure] matching model workers are saturated: 1 workers"
        );
        assert!(send_block_chunk.state_is_pressure);
        assert!(send_block_chunk.state_blocks_prompt_submit);

        assert_eq!(route.workers.len(), 2);
        assert_eq!(route.workers[0].endpoint_label(), "quality-12b");
        assert!(!route.workers[0].route_match);
        assert!(!route.workers[0].selectable);
        assert_eq!(
            route.workers[0].picker_action,
            ModelRouteWorkerPickerAction::Unavailable
        );
        assert_eq!(route.workers[0].worker_status_label(), "available");
        assert_eq!(route.workers[1].endpoint_label(), "fast-reviewer");
        assert!(route.workers[1].route_match);
        assert!(!route.workers[1].selectable);
        assert_eq!(
            route.workers[1].picker_action,
            ModelRouteWorkerPickerAction::Wait
        );
        assert_eq!(route.workers[1].worker_status_label(), "backpressure");
        assert_eq!(route.workers[1].decision_action_label(), "retry_later");
        assert_eq!(route.workers[1].decision_state_label(), "backpressure");
    }

    #[test]
    fn route_snapshot_keeps_same_worker_rows_as_route_workers_helper() {
        let cases = [
            (
                "engine_busy",
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
                        ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                            .with_roles([ModelRole::Reviewer])
                            .with_preferences([RoutingPreference::PreferFast])
                            .with_busy(true, Some("#18 review".to_owned())),
                    ],
                ),
                RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast),
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
                            .with_queue(1, 1),
                    ],
                ),
                RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast),
            ),
            (
                "route_backpressure",
                ModelPoolGateSnapshot::new(
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
                ),
                RoutingIntent::auto_route(ModelRole::Reviewer, RoutingPreference::PreferFast),
            ),
        ];

        for (case, snapshot, intent) in cases {
            let route = snapshot.route_snapshot(&intent);
            let workers = snapshot.route_workers(&intent);

            assert_eq!(route.workers, workers, "{case}");
        }
    }

    #[test]
    fn balanced_preference_keeps_role_matching_workers_in_scheduler_pool() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferFast]),
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B)
                    .with_roles([ModelRole::Reviewer])
                    .with_preferences([RoutingPreference::PreferQuality])
                    .with_busy(true, Some("#15 deep review".to_owned())),
                ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester)
                    .with_roles([ModelRole::Tester])
                    .with_preferences([RoutingPreference::PreferFast]),
            ],
        );
        let request = ChatRequest::new("s1", vec![ChatMessage::user("review")])
            .with_model_role(ModelRole::Reviewer);

        let status = snapshot.route_status(&request.routing_intent());

        assert_eq!(
            snapshot.decision_for_intent(&request.routing_intent()),
            GateDecision::Allowed
        );
        assert_eq!(status.matching_workers, 2);
        assert_eq!(status.matching_available_workers, 1);
        assert_eq!(status.matching_busy_workers, 1);
        assert_eq!(status.matching_saturated_workers, 0);
        assert_eq!(status.matching_queued_requests, 0);
        assert_eq!(status.matching_queue_limit, 2);
        assert!(!request.endpoint_pinned());
    }

    #[test]
    fn route_status_counts_only_pinned_endpoint_when_operator_selected() {
        let snapshot = ModelPoolGateSnapshot::new(
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
        let request = ChatRequest::new("s1", vec![ChatMessage::user("deep answer")])
            .with_model_role(ModelRole::Assistant)
            .with_routing_preference(RoutingPreference::PreferQuality)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));

        let status = snapshot.route_status(&request.routing_intent());

        assert_eq!(status.matching_workers, 1);
        assert_eq!(status.matching_available_workers, 0);
        assert_eq!(status.matching_busy_workers, 1);
        assert_eq!(status.matching_saturated_workers, 0);
        assert_eq!(status.matching_queued_requests, 0);
        assert_eq!(status.matching_queue_limit, 1);
        assert_eq!(
            status.summary(),
            "matching total=1 available=0 busy=1 saturated=0"
        );
    }

    #[test]
    fn route_status_reports_zero_when_pinned_endpoint_mismatches_capabilities() {
        let snapshot = ModelPoolGateSnapshot::new(
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
        let request = ChatRequest::new("s1", vec![ChatMessage::user("deep answer")])
            .with_model_role(ModelRole::Assistant)
            .with_routing_preference(RoutingPreference::PreferQuality)
            .with_model_endpoint(Some(ModelEndpoint::FastReviewer));

        let status = snapshot.route_status(&request.routing_intent());

        assert_eq!(status.matching_workers, 0);
        assert_eq!(status.matching_available_workers, 0);
        assert_eq!(status.matching_busy_workers, 0);
        assert_eq!(status.matching_saturated_workers, 0);
        assert_eq!(status.matching_queued_requests, 0);
        assert_eq!(status.matching_queue_limit, 0);
        assert!(!status.has_matching_workers());
        assert!(!status.has_matching_available_workers());
        assert!(!status.has_matching_busy_workers());
        assert!(!status.has_matching_saturated_workers());
        assert!(!status.has_matching_queued_requests());
        assert!(!status.matching_queue_is_saturated());
        assert_eq!(
            status.summary(),
            "matching total=0 available=0 busy=0 saturated=0"
        );
    }

    #[test]
    fn worker_snapshot_summary_exposes_operator_endpoint_health() {
        let available = ModelWorkerSnapshot::new(ModelEndpoint::Quality12B);
        let busy = ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
            .with_busy(true, Some("#7 review".to_owned()));
        let saturated = ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester).with_queue(2, 2);

        assert_eq!(available.status_label(), "available");
        assert_eq!(available.status_state(), StreamState::Pending);
        assert_eq!(available.status_state_label(), "pending");
        assert!(available.is_available());
        assert!(!available.status_is_pressure());
        assert!(!available.status_blocks_prompt_submit());
        assert_eq!(
            available.summary(),
            "endpoint=quality-12b status=available queue=0/1 active=none"
        );
        assert_eq!(busy.status_label(), "busy");
        assert_eq!(busy.status_state(), StreamState::Busy);
        assert_eq!(busy.status_state_label(), "busy");
        assert!(!busy.is_available());
        assert!(busy.status_is_pressure());
        assert!(busy.status_blocks_prompt_submit());
        assert_eq!(
            busy.summary(),
            "endpoint=fast-reviewer status=busy queue=0/1 active=#7 review"
        );
        assert_eq!(saturated.status_label(), "backpressure");
        assert_eq!(saturated.status_state(), StreamState::Backpressure);
        assert_eq!(saturated.status_state_label(), "backpressure");
        assert!(!saturated.is_available());
        assert!(saturated.status_is_pressure());
        assert!(saturated.status_blocks_prompt_submit());
        assert_eq!(
            saturated.summary(),
            "endpoint=summary-tester status=backpressure queue=2/2 active=none"
        );

        let reviewer = ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
            .with_roles([ModelRole::Reviewer])
            .with_preferences([RoutingPreference::PreferFast]);
        assert_eq!(available.endpoint_label(), "quality-12b");
        assert_eq!(available.queue_label(), "0/1");
        assert_eq!(available.active_request_label(), "none");
        assert!(available.accepts_any_role());
        assert!(available.accepts_any_preference());
        assert_eq!(reviewer.role_labels(), vec!["reviewer"]);
        assert_eq!(reviewer.preference_labels(), vec!["prefer_fast"]);
        assert!(!reviewer.accepts_any_role());
        assert!(!reviewer.accepts_any_preference());
        assert_eq!(
            reviewer.summary(),
            "endpoint=fast-reviewer status=available queue=0/1 active=none roles=reviewer preferences=prefer_fast"
        );
    }

    #[test]
    fn worker_status_display_snapshot_is_stable_for_busy_and_backpressure_hosts() {
        let available = ModelWorkerSnapshot::new(ModelEndpoint::Quality12B);
        let busy = ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
            .with_busy(true, Some("#7 review".to_owned()));
        let saturated = ModelWorkerSnapshot::new(ModelEndpoint::SummaryTester).with_queue(2, 2);

        assert_eq!(available.status_display_snapshot(), None);

        let busy_chunk = busy
            .status_display_snapshot()
            .expect("busy worker should expose a status display snapshot");
        assert_eq!(busy_chunk.output_label, "busy");
        assert_eq!(busy_chunk.state, StreamState::Busy);
        assert_eq!(busy_chunk.state_label, "busy");
        assert_eq!(
            busy_chunk.appended,
            "[busy] worker fast-reviewer is busy: #7 review"
        );
        assert!(!busy_chunk.state_is_terminal);
        assert!(busy_chunk.state_is_pressure);
        assert!(busy_chunk.state_blocks_prompt_submit);

        let saturated_chunk = saturated
            .status_display_snapshot()
            .expect("saturated worker should expose a status display snapshot");
        assert_eq!(saturated_chunk.output_label, "backpressure");
        assert_eq!(saturated_chunk.state, StreamState::Backpressure);
        assert_eq!(saturated_chunk.state_label, "backpressure");
        assert_eq!(
            saturated_chunk.appended,
            "[backpressure] worker summary-tester queue is saturated: 2/2"
        );
        assert!(!saturated_chunk.state_is_terminal);
        assert!(saturated_chunk.state_is_pressure);
        assert!(saturated_chunk.state_blocks_prompt_submit);
    }

    #[test]
    fn model_pool_worker_status_lines_are_stable_for_ui_lists() {
        let snapshot = ModelPoolGateSnapshot::new(
            FrontendGateSnapshot::default(),
            vec![
                ModelWorkerSnapshot::new(ModelEndpoint::Quality12B),
                ModelWorkerSnapshot::new(ModelEndpoint::FastReviewer)
                    .with_busy(true, Some("review".to_owned())),
            ],
        );

        assert_eq!(
            snapshot.worker_status_lines(),
            vec![
                "endpoint=quality-12b status=available queue=0/1 active=none".to_owned(),
                "endpoint=fast-reviewer status=busy queue=0/1 active=review".to_owned(),
            ]
        );
        assert_eq!(
            ModelPoolGateSnapshot::default().worker_status_lines(),
            vec!["workers none registered".to_owned()]
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
    ) -> Option<SmartSteamNextRoundDecisionReportStatusSource> {
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

        Some(SmartSteamNextRoundDecisionReportStatusSource {
            decision_status: json_string_value(report_json, "decision_status"),
            display_state: json_string_value(report_json, "display_state"),
            live_status_display_state: json_string_value(report_json, "live_status_display_state")
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
        })
    }

    fn captured_current_status_next_round_downstream_status_from_json(
        json: &str,
    ) -> Option<SmartSteamNextRoundDownstreamConsumerStatusSource> {
        let live_status_bundle = json_object_after_key(json, "live_status_bundle");
        let container = live_status_bundle
            .and_then(|bundle| {
                json_object_after_key(bundle, "next_round_downstream_status_consumers_v1")
            })
            .or_else(|| json_object_after_key(json, "next_round_downstream_status_consumers_v1"))?;
        let downstream_json =
            json_object_after_key(container, "next_round_downstream").unwrap_or(container);

        Some(SmartSteamNextRoundDownstreamConsumerStatusSource {
            source_decision_status: json_string_value(downstream_json, "source_decision_status"),
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
        })
    }

    fn captured_current_status_next_round_round_id_evidence_from_json(
        json: &str,
    ) -> Option<SmartSteamNextRoundRoundIdEvidenceSource> {
        let round_id_evidence = json_object_after_key(json, "round_id_evidence")?;
        Some(SmartSteamNextRoundRoundIdEvidenceSource {
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
    ) -> SmartSteamStatusSource {
        let transition = captured.daemon_round_transition_status_v1;
        assert_eq!(transition.transition_kind, "normal_in_progress");
        assert_eq!(transition.active_round, captured.active_round);
        assert_eq!(transition.latest_done_round, captured.latest_done_round);
        assert_eq!(transition.ledger_latest_round, captured.ledger_latest_round);
        assert!(transition.read_only);
        assert!(!transition.starts_process);
        assert!(!transition.sends_prompt);

        SmartSteamStatusSource::new()
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
            .with_daemon_round_transition(SmartSteamDaemonRoundTransitionStatusSource {
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
            })
            .with_worker_windows(
                captured
                    .worker_windows
                    .iter()
                    .map(worker_window_source_from_captured_json),
            )
    }

    fn worker_window_source_from_captured_json(
        captured: &CapturedWorkerWindowJsonFixture,
    ) -> SmartSteamWorkerWindowStatusSource {
        match captured.status_label {
            "completed-evidence-only" => {
                SmartSteamWorkerWindowStatusSource::new(captured.window_id, captured.lane_label)
                    .with_completed_evidence_only(captured.reason)
            }
            status => panic!("unsupported captured worker window status {status}"),
        }
    }
}
