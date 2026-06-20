#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedLiveStatusBundleFacts {
    pub evidence_ids: Vec<String>,
    pub transition_kind: String,
    pub active_round: Option<u64>,
    pub ledger_latest_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub round_in_progress: bool,
    pub read_only: bool,
    pub starts_process: bool,
    pub sends_prompt: bool,
    pub report_gate_ready: bool,
    pub remote_pool_healthy: bool,
    pub evidence_current: bool,
    pub polluted_evidence_present: bool,
    pub polluted_evidence_non_actionable: bool,
    pub raw_payloads_present: bool,
}

impl SanitizedLiveStatusBundleFacts {
    pub fn active_round(
        evidence_id: impl Into<String>,
        active_round: u64,
        latest_done_round: u64,
    ) -> Self {
        Self {
            evidence_ids: vec![evidence_id.into()],
            transition_kind: "normal_in_progress".to_owned(),
            active_round: Some(active_round),
            ledger_latest_round: Some(latest_done_round),
            latest_done_round: Some(latest_done_round),
            round_in_progress: true,
            read_only: true,
            starts_process: false,
            sends_prompt: false,
            report_gate_ready: true,
            remote_pool_healthy: true,
            evidence_current: true,
            polluted_evidence_present: false,
            polluted_evidence_non_actionable: true,
            raw_payloads_present: false,
        }
    }

    pub fn ready_after_round(evidence_id: impl Into<String>, latest_done_round: u64) -> Self {
        Self {
            evidence_ids: vec![evidence_id.into()],
            transition_kind: "round_done_waiting_ledger_commit".to_owned(),
            active_round: None,
            ledger_latest_round: Some(latest_done_round),
            latest_done_round: Some(latest_done_round),
            round_in_progress: false,
            read_only: true,
            starts_process: false,
            sends_prompt: false,
            report_gate_ready: true,
            remote_pool_healthy: true,
            evidence_current: true,
            polluted_evidence_present: false,
            polluted_evidence_non_actionable: true,
            raw_payloads_present: false,
        }
    }

    pub fn with_evidence_ids(mut self, evidence_ids: Vec<String>) -> Self {
        self.evidence_ids = evidence_ids;
        self
    }

    pub fn with_polluted_evidence(
        mut self,
        present: bool,
        completed_window_evidence_non_actionable: bool,
    ) -> Self {
        self.polluted_evidence_present = present;
        self.polluted_evidence_non_actionable = completed_window_evidence_non_actionable;
        self
    }

    pub fn with_evidence_current(mut self, evidence_current: bool) -> Self {
        self.evidence_current = evidence_current;
        self
    }

    pub fn with_raw_payloads_present(mut self, raw_payloads_present: bool) -> Self {
        self.raw_payloads_present = raw_payloads_present;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanRoomAssignmentDecisionKind {
    WaitCurrentRoundActive,
    CreateFreshCleanRoomAssignmentAfterCurrentRound,
    BlockStaleOrPollutedEvidence,
}

impl CleanRoomAssignmentDecisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WaitCurrentRoundActive => "wait_current_round_active",
            Self::CreateFreshCleanRoomAssignmentAfterCurrentRound => {
                "create_fresh_clean_room_assignment_after_current_round"
            }
            Self::BlockStaleOrPollutedEvidence => "block_stale_or_polluted_evidence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanRoomAssignmentReport {
    pub decision: CleanRoomAssignmentDecisionKind,
    pub business_task_ids: Vec<String>,
    pub assignment_task_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub transition_kind: String,
    pub active_round: Option<u64>,
    pub target_after_round: Option<u64>,
    pub read_only: bool,
    pub report_only: bool,
    pub starts_process: bool,
    pub sends_prompt: bool,
    pub dispatch_side_effects_allowed: bool,
    pub creates_thread: bool,
    pub raw_payloads_retained: bool,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl CleanRoomAssignmentReport {
    pub fn can_create_assignment_report(&self) -> bool {
        self.decision
            == CleanRoomAssignmentDecisionKind::CreateFreshCleanRoomAssignmentAfterCurrentRound
            && !self.assignment_task_ids.is_empty()
            && self.report_only
            && !self.dispatch_side_effects_allowed
            && !self.creates_thread
    }

    pub fn is_blocked(&self) -> bool {
        self.decision == CleanRoomAssignmentDecisionKind::BlockStaleOrPollutedEvidence
    }
}

#[derive(Debug, Clone, Default)]
pub struct CleanRoomAssignmentDecider;

impl CleanRoomAssignmentDecider {
    pub fn new() -> Self {
        Self
    }

    pub fn decide(
        &self,
        facts: &SanitizedLiveStatusBundleFacts,
        candidate_task_ids: Vec<String>,
    ) -> CleanRoomAssignmentReport {
        let mut reasons = Vec::new();

        if facts.evidence_ids.is_empty() {
            reasons.push("live_status_bundle_evidence_ids_empty".to_owned());
        }
        if !facts.evidence_current {
            reasons.push("live_status_bundle_evidence_stale".to_owned());
        }
        if !facts.read_only {
            reasons.push("live_status_bundle_not_read_only".to_owned());
        }
        if facts.starts_process {
            reasons.push("live_status_bundle_starts_process".to_owned());
        }
        if facts.sends_prompt {
            reasons.push("live_status_bundle_sends_prompt".to_owned());
        }
        if !facts.report_gate_ready {
            reasons.push("live_status_bundle_report_gate_not_ready".to_owned());
        }
        if !facts.remote_pool_healthy {
            reasons.push("live_status_bundle_remote_pool_not_healthy".to_owned());
        }
        if facts.raw_payloads_present {
            reasons.push("live_status_bundle_raw_payloads_present".to_owned());
        }
        if facts.polluted_evidence_present && !facts.polluted_evidence_non_actionable {
            reasons.push("live_status_bundle_polluted_evidence_actionable".to_owned());
        }
        if candidate_task_ids.is_empty() {
            reasons.push("clean_room_candidate_task_ids_empty".to_owned());
        }

        let stale_or_polluted = !reasons.is_empty();
        let decision = if stale_or_polluted {
            CleanRoomAssignmentDecisionKind::BlockStaleOrPollutedEvidence
        } else if facts.round_in_progress {
            reasons.push("current_daemon_round_active".to_owned());
            CleanRoomAssignmentDecisionKind::WaitCurrentRoundActive
        } else {
            reasons.push("current_round_complete_clean_room_assignment_report_ready".to_owned());
            CleanRoomAssignmentDecisionKind::CreateFreshCleanRoomAssignmentAfterCurrentRound
        };

        let assignment_task_ids = if decision
            == CleanRoomAssignmentDecisionKind::CreateFreshCleanRoomAssignmentAfterCurrentRound
        {
            candidate_task_ids.clone()
        } else {
            Vec::new()
        };
        let target_after_round = if decision
            == CleanRoomAssignmentDecisionKind::CreateFreshCleanRoomAssignmentAfterCurrentRound
        {
            facts.latest_done_round.or(facts.ledger_latest_round)
        } else {
            None
        };
        let telemetry = clean_room_assignment_telemetry(
            decision,
            facts,
            candidate_task_ids.len(),
            assignment_task_ids.len(),
            reasons.len(),
        );

        CleanRoomAssignmentReport {
            decision,
            business_task_ids: candidate_task_ids,
            assignment_task_ids,
            evidence_ids: facts.evidence_ids.clone(),
            transition_kind: facts.transition_kind.clone(),
            active_round: facts.active_round,
            target_after_round,
            read_only: true,
            report_only: true,
            starts_process: false,
            sends_prompt: false,
            dispatch_side_effects_allowed: false,
            creates_thread: false,
            raw_payloads_retained: false,
            reasons,
            telemetry,
        }
    }
}

pub fn decide_clean_room_assignment(
    facts: &SanitizedLiveStatusBundleFacts,
    candidate_task_ids: Vec<String>,
) -> CleanRoomAssignmentReport {
    CleanRoomAssignmentDecider::new().decide(facts, candidate_task_ids)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SanitizedNextRoundDecisionStatus {
    SafeToWaitCurrentRoundActive,
    SafeToContinueAfterCurrentRound,
    OperatorAttentionBlocked,
}

impl SanitizedNextRoundDecisionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SafeToWaitCurrentRoundActive => "safe_to_wait_current_round_active",
            Self::SafeToContinueAfterCurrentRound => "safe_to_continue_after_current_round",
            Self::OperatorAttentionBlocked => "operator_attention_blocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedRoundIdEvidence {
    pub source_schema: String,
    pub active_round: Option<u64>,
    pub ledger_latest_round: Option<u64>,
    pub latest_done_round: Option<u64>,
}

impl SanitizedRoundIdEvidence {
    pub fn daemon_transition(
        active_round: Option<u64>,
        ledger_latest_round: Option<u64>,
        latest_done_round: Option<u64>,
    ) -> Self {
        Self {
            source_schema: "daemon_round_transition_status_v1".to_owned(),
            active_round,
            ledger_latest_round,
            latest_done_round,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedNextRoundDecisionFacts {
    pub evidence_ids: Vec<String>,
    pub round_id_evidence: Option<SanitizedRoundIdEvidence>,
    pub decision_status: SanitizedNextRoundDecisionStatus,
    pub current_round_active: bool,
    pub live_status_display_state: String,
    pub transition_kind: String,
    pub active_round: Option<u64>,
    pub ledger_latest_round: Option<u64>,
    pub latest_done_round: Option<u64>,
    pub readiness_can_schedule_next_round: bool,
    pub report_gate_ready: bool,
    pub context_hygiene_passed: bool,
    pub read_only: bool,
    pub report_only: bool,
    pub no_side_effects: bool,
    pub dispatch_work_allowed: bool,
    pub prompt_replay_allowed: bool,
    pub process_start_allowed: bool,
    pub memory_write_allowed: bool,
    pub ndkv_write_allowed: bool,
    pub operator_attention_required: bool,
    pub failure_reasons: Vec<String>,
    pub raw_payloads_present: bool,
}

impl SanitizedNextRoundDecisionFacts {
    pub fn safe_to_wait(
        evidence_id: impl Into<String>,
        active_round: u64,
        latest_done_round: u64,
    ) -> Self {
        Self {
            evidence_ids: vec![evidence_id.into()],
            round_id_evidence: Some(SanitizedRoundIdEvidence::daemon_transition(
                Some(active_round),
                Some(latest_done_round),
                Some(latest_done_round),
            )),
            decision_status: SanitizedNextRoundDecisionStatus::SafeToWaitCurrentRoundActive,
            current_round_active: true,
            live_status_display_state: "active_busy".to_owned(),
            transition_kind: "normal_in_progress".to_owned(),
            active_round: Some(active_round),
            ledger_latest_round: Some(latest_done_round),
            latest_done_round: Some(latest_done_round),
            readiness_can_schedule_next_round: true,
            report_gate_ready: true,
            context_hygiene_passed: true,
            read_only: true,
            report_only: true,
            no_side_effects: true,
            dispatch_work_allowed: false,
            prompt_replay_allowed: false,
            process_start_allowed: false,
            memory_write_allowed: false,
            ndkv_write_allowed: false,
            operator_attention_required: false,
            failure_reasons: Vec::new(),
            raw_payloads_present: false,
        }
    }

    pub fn safe_to_continue(evidence_id: impl Into<String>, latest_done_round: u64) -> Self {
        Self {
            evidence_ids: vec![evidence_id.into()],
            round_id_evidence: Some(SanitizedRoundIdEvidence::daemon_transition(
                None,
                Some(latest_done_round),
                Some(latest_done_round),
            )),
            decision_status: SanitizedNextRoundDecisionStatus::SafeToContinueAfterCurrentRound,
            current_round_active: false,
            live_status_display_state: "ledger_synced".to_owned(),
            transition_kind: "round_done_waiting_ledger_commit".to_owned(),
            active_round: None,
            ledger_latest_round: Some(latest_done_round),
            latest_done_round: Some(latest_done_round),
            readiness_can_schedule_next_round: true,
            report_gate_ready: true,
            context_hygiene_passed: true,
            read_only: true,
            report_only: true,
            no_side_effects: true,
            dispatch_work_allowed: false,
            prompt_replay_allowed: false,
            process_start_allowed: false,
            memory_write_allowed: false,
            ndkv_write_allowed: false,
            operator_attention_required: false,
            failure_reasons: Vec::new(),
            raw_payloads_present: false,
        }
    }

    pub fn operator_attention(
        evidence_id: impl Into<String>,
        failure_reasons: Vec<String>,
    ) -> Self {
        Self {
            evidence_ids: vec![evidence_id.into()],
            round_id_evidence: None,
            decision_status: SanitizedNextRoundDecisionStatus::OperatorAttentionBlocked,
            current_round_active: false,
            live_status_display_state: "blocked".to_owned(),
            transition_kind: "operator_attention_required".to_owned(),
            active_round: None,
            ledger_latest_round: None,
            latest_done_round: None,
            readiness_can_schedule_next_round: false,
            report_gate_ready: false,
            context_hygiene_passed: false,
            read_only: true,
            report_only: true,
            no_side_effects: true,
            dispatch_work_allowed: false,
            prompt_replay_allowed: false,
            process_start_allowed: false,
            memory_write_allowed: false,
            ndkv_write_allowed: false,
            operator_attention_required: true,
            failure_reasons,
            raw_payloads_present: false,
        }
    }

    pub fn with_evidence_ids(mut self, evidence_ids: Vec<String>) -> Self {
        self.evidence_ids = evidence_ids;
        self
    }

    pub fn with_round_id_evidence(
        mut self,
        round_id_evidence: Option<SanitizedRoundIdEvidence>,
    ) -> Self {
        self.round_id_evidence = round_id_evidence;
        self
    }

    pub fn with_raw_payloads_present(mut self, raw_payloads_present: bool) -> Self {
        self.raw_payloads_present = raw_payloads_present;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanRoomAssignmentPlanningDecisionKind {
    WaitCurrentRoundActive,
    AllowFreshCleanRoomAssignmentAfterSyncedCompletion,
    BlockOperatorAttention,
}

impl CleanRoomAssignmentPlanningDecisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WaitCurrentRoundActive => "wait_current_round_active",
            Self::AllowFreshCleanRoomAssignmentAfterSyncedCompletion => {
                "allow_fresh_clean_room_assignment_after_synced_completion"
            }
            Self::BlockOperatorAttention => "block_operator_attention",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanRoomAssignmentPlanningEvidence {
    pub decision: CleanRoomAssignmentPlanningDecisionKind,
    pub next_round_decision_status: String,
    pub round_id_evidence: Option<SanitizedRoundIdEvidence>,
    pub business_task_ids: Vec<String>,
    pub assignment_task_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub next_round_evidence_ids: Vec<String>,
    pub assignment_evidence_ids: Vec<String>,
    pub transition_kind: String,
    pub active_round: Option<u64>,
    pub target_after_round: Option<u64>,
    pub read_only: bool,
    pub report_only: bool,
    pub starts_process: bool,
    pub sends_prompt: bool,
    pub dispatch_side_effects_allowed: bool,
    pub creates_thread: bool,
    pub raw_payloads_retained: bool,
    pub operator_attention_required: bool,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl CleanRoomAssignmentPlanningEvidence {
    pub fn can_prepare_fresh_assignment(&self) -> bool {
        self.decision
            == CleanRoomAssignmentPlanningDecisionKind::AllowFreshCleanRoomAssignmentAfterSyncedCompletion
            && !self.assignment_task_ids.is_empty()
            && self.read_only
            && self.report_only
            && !self.dispatch_side_effects_allowed
            && !self.creates_thread
            && !self.raw_payloads_retained
    }

    pub fn requires_operator_attention(&self) -> bool {
        self.decision == CleanRoomAssignmentPlanningDecisionKind::BlockOperatorAttention
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanRoomAssignmentAcceptanceDecisionKind {
    AcceptFreshAssignmentPlanningEvidence,
    WaitCurrentRoundPlanningEvidence,
    RejectPlanningEvidence,
}

impl CleanRoomAssignmentAcceptanceDecisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcceptFreshAssignmentPlanningEvidence => {
                "accept_fresh_assignment_planning_evidence"
            }
            Self::WaitCurrentRoundPlanningEvidence => "wait_current_round_planning_evidence",
            Self::RejectPlanningEvidence => "reject_planning_evidence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanRoomAssignmentAcceptance {
    pub decision: CleanRoomAssignmentAcceptanceDecisionKind,
    pub planning_decision: CleanRoomAssignmentPlanningDecisionKind,
    pub next_round_decision_status: String,
    pub round_id_evidence: Option<SanitizedRoundIdEvidence>,
    pub business_task_ids: Vec<String>,
    pub assignment_task_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub next_round_evidence_ids: Vec<String>,
    pub assignment_evidence_ids: Vec<String>,
    pub transition_kind: String,
    pub active_round: Option<u64>,
    pub target_after_round: Option<u64>,
    pub read_only: bool,
    pub report_only: bool,
    pub starts_process: bool,
    pub sends_prompt: bool,
    pub dispatch_side_effects_allowed: bool,
    pub creates_thread: bool,
    pub process_start_allowed: bool,
    pub memory_write_allowed: bool,
    pub ndkv_write_allowed: bool,
    pub raw_payloads_retained: bool,
    pub operator_attention_required: bool,
    pub accepted_for_agent_planning: bool,
    pub wait_only: bool,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl CleanRoomAssignmentAcceptance {
    pub fn can_surface_assignment_plan(&self) -> bool {
        self.decision
            == CleanRoomAssignmentAcceptanceDecisionKind::AcceptFreshAssignmentPlanningEvidence
            && self.accepted_for_agent_planning
            && !self.assignment_task_ids.is_empty()
            && self.read_only
            && self.report_only
            && !self.starts_process
            && !self.sends_prompt
            && !self.dispatch_side_effects_allowed
            && !self.creates_thread
            && !self.process_start_allowed
            && !self.memory_write_allowed
            && !self.ndkv_write_allowed
            && !self.raw_payloads_retained
    }

    pub fn is_wait_only(&self) -> bool {
        self.decision == CleanRoomAssignmentAcceptanceDecisionKind::WaitCurrentRoundPlanningEvidence
            && self.wait_only
    }
}

pub fn accept_clean_room_assignment_planning_evidence(
    planning: &CleanRoomAssignmentPlanningEvidence,
) -> CleanRoomAssignmentAcceptance {
    let mut reasons = planning.reasons.clone();

    if planning.evidence_ids.is_empty() {
        reasons.push("assignment_acceptance_evidence_ids_empty".to_owned());
    }
    if planning.next_round_evidence_ids.is_empty() {
        reasons.push("assignment_acceptance_next_round_evidence_ids_empty".to_owned());
    }
    if planning.assignment_evidence_ids.is_empty() {
        reasons.push("assignment_acceptance_assignment_evidence_ids_empty".to_owned());
    }
    if !planning.read_only || !planning.report_only {
        reasons.push("assignment_acceptance_requires_read_only_report_only".to_owned());
    }
    if planning.starts_process
        || planning.sends_prompt
        || planning.dispatch_side_effects_allowed
        || planning.creates_thread
    {
        reasons.push("assignment_acceptance_rejects_runtime_side_effects".to_owned());
    }
    if planning.raw_payloads_retained {
        reasons.push("assignment_acceptance_rejects_raw_payloads".to_owned());
    }
    if planning.operator_attention_required || planning.requires_operator_attention() {
        reasons.push("assignment_acceptance_operator_attention_required".to_owned());
    }
    if planning.decision
        == CleanRoomAssignmentPlanningDecisionKind::AllowFreshCleanRoomAssignmentAfterSyncedCompletion
        && planning.assignment_task_ids.is_empty()
    {
        reasons.push("assignment_acceptance_assignment_task_ids_empty".to_owned());
    }

    let bridge_inputs_clean = planning.read_only
        && planning.report_only
        && !planning.starts_process
        && !planning.sends_prompt
        && !planning.dispatch_side_effects_allowed
        && !planning.creates_thread
        && !planning.raw_payloads_retained
        && !planning.operator_attention_required
        && !planning.evidence_ids.is_empty()
        && !planning.next_round_evidence_ids.is_empty()
        && !planning.assignment_evidence_ids.is_empty();

    let decision = if bridge_inputs_clean && planning.can_prepare_fresh_assignment() {
        CleanRoomAssignmentAcceptanceDecisionKind::AcceptFreshAssignmentPlanningEvidence
    } else if bridge_inputs_clean
        && planning.decision == CleanRoomAssignmentPlanningDecisionKind::WaitCurrentRoundActive
    {
        CleanRoomAssignmentAcceptanceDecisionKind::WaitCurrentRoundPlanningEvidence
    } else {
        CleanRoomAssignmentAcceptanceDecisionKind::RejectPlanningEvidence
    };

    match decision {
        CleanRoomAssignmentAcceptanceDecisionKind::AcceptFreshAssignmentPlanningEvidence => {
            reasons.push("assignment_acceptance_fresh_plan_evidence_accepted".to_owned());
        }
        CleanRoomAssignmentAcceptanceDecisionKind::WaitCurrentRoundPlanningEvidence => {
            reasons.push("assignment_acceptance_waits_for_current_round".to_owned());
        }
        CleanRoomAssignmentAcceptanceDecisionKind::RejectPlanningEvidence => {
            reasons.push("assignment_acceptance_rejected_without_side_effects".to_owned());
        }
    }

    let assignment_task_ids = if decision
        == CleanRoomAssignmentAcceptanceDecisionKind::AcceptFreshAssignmentPlanningEvidence
    {
        planning.assignment_task_ids.clone()
    } else {
        Vec::new()
    };
    let target_after_round = if decision
        == CleanRoomAssignmentAcceptanceDecisionKind::AcceptFreshAssignmentPlanningEvidence
    {
        planning.target_after_round
    } else {
        None
    };
    let accepted_for_agent_planning = decision
        == CleanRoomAssignmentAcceptanceDecisionKind::AcceptFreshAssignmentPlanningEvidence;
    let wait_only =
        decision == CleanRoomAssignmentAcceptanceDecisionKind::WaitCurrentRoundPlanningEvidence;
    let operator_attention_required =
        decision == CleanRoomAssignmentAcceptanceDecisionKind::RejectPlanningEvidence;
    let telemetry = clean_room_assignment_acceptance_telemetry(
        decision,
        planning,
        assignment_task_ids.len(),
        reasons.len(),
    );

    CleanRoomAssignmentAcceptance {
        decision,
        planning_decision: planning.decision,
        next_round_decision_status: planning.next_round_decision_status.clone(),
        round_id_evidence: planning.round_id_evidence.clone(),
        business_task_ids: planning.business_task_ids.clone(),
        assignment_task_ids,
        evidence_ids: planning.evidence_ids.clone(),
        next_round_evidence_ids: planning.next_round_evidence_ids.clone(),
        assignment_evidence_ids: planning.assignment_evidence_ids.clone(),
        transition_kind: planning.transition_kind.clone(),
        active_round: planning.active_round,
        target_after_round,
        read_only: true,
        report_only: true,
        starts_process: false,
        sends_prompt: false,
        dispatch_side_effects_allowed: false,
        creates_thread: false,
        process_start_allowed: false,
        memory_write_allowed: false,
        ndkv_write_allowed: false,
        raw_payloads_retained: false,
        operator_attention_required,
        accepted_for_agent_planning,
        wait_only,
        reasons,
        telemetry,
    }
}

pub fn plan_clean_room_assignment_from_next_round_decision(
    decision: &SanitizedNextRoundDecisionFacts,
    candidates: &CleanRoomAssignmentReport,
) -> CleanRoomAssignmentPlanningEvidence {
    let mut reasons = Vec::new();

    if decision.evidence_ids.is_empty() {
        reasons.push("next_round_decision_evidence_ids_empty".to_owned());
    }
    if candidates.evidence_ids.is_empty() {
        reasons.push("clean_room_assignment_evidence_ids_empty".to_owned());
    }
    if candidates.business_task_ids.is_empty() {
        reasons.push("clean_room_assignment_candidate_task_ids_empty".to_owned());
    }
    if !decision.read_only || !candidates.read_only {
        reasons.push("assignment_planning_requires_read_only_inputs".to_owned());
    }
    if !decision.report_only || !candidates.report_only {
        reasons.push("assignment_planning_requires_report_only_inputs".to_owned());
    }
    if !decision.no_side_effects
        || decision.dispatch_work_allowed
        || decision.prompt_replay_allowed
        || decision.process_start_allowed
        || decision.memory_write_allowed
        || decision.ndkv_write_allowed
        || candidates.starts_process
        || candidates.sends_prompt
        || candidates.dispatch_side_effects_allowed
        || candidates.creates_thread
    {
        reasons.push("assignment_planning_rejects_runtime_side_effects".to_owned());
    }
    if decision.raw_payloads_present || candidates.raw_payloads_retained {
        reasons.push("assignment_planning_drops_raw_payloads".to_owned());
    }
    if !decision.readiness_can_schedule_next_round {
        reasons.push("next_round_readiness_blocked_scheduling".to_owned());
    }
    if !decision.report_gate_ready {
        reasons.push("next_round_report_gate_not_ready".to_owned());
    }
    if !decision.context_hygiene_passed {
        reasons.push("next_round_context_hygiene_not_passed".to_owned());
    }
    if decision.operator_attention_required
        || decision.decision_status == SanitizedNextRoundDecisionStatus::OperatorAttentionBlocked
    {
        reasons.push("next_round_decision_operator_attention_required".to_owned());
    }
    reasons.extend(
        decision
            .failure_reasons
            .iter()
            .map(|reason| format!("next_round_decision_failure:{reason}")),
    );

    let decision_kind = if !reasons.is_empty() {
        CleanRoomAssignmentPlanningDecisionKind::BlockOperatorAttention
    } else if decision.decision_status
        == SanitizedNextRoundDecisionStatus::SafeToWaitCurrentRoundActive
        || decision.current_round_active
    {
        reasons.push("next_round_decision_wait_current_round_active".to_owned());
        CleanRoomAssignmentPlanningDecisionKind::WaitCurrentRoundActive
    } else if decision.decision_status
        == SanitizedNextRoundDecisionStatus::SafeToContinueAfterCurrentRound
        && decision.live_status_display_state == "ledger_synced"
    {
        reasons.push("next_round_decision_synced_completion_allows_assignment".to_owned());
        CleanRoomAssignmentPlanningDecisionKind::AllowFreshCleanRoomAssignmentAfterSyncedCompletion
    } else {
        reasons.push("next_round_decision_continue_requires_synced_completion".to_owned());
        CleanRoomAssignmentPlanningDecisionKind::BlockOperatorAttention
    };

    let assignment_task_ids = if decision_kind
        == CleanRoomAssignmentPlanningDecisionKind::AllowFreshCleanRoomAssignmentAfterSyncedCompletion
    {
        if candidates.assignment_task_ids.is_empty() {
            candidates.business_task_ids.clone()
        } else {
            candidates.assignment_task_ids.clone()
        }
    } else {
        Vec::new()
    };
    let target_after_round = if decision_kind
        == CleanRoomAssignmentPlanningDecisionKind::AllowFreshCleanRoomAssignmentAfterSyncedCompletion
    {
        decision
            .latest_done_round
            .or(decision.ledger_latest_round)
            .or(candidates.target_after_round)
    } else {
        None
    };
    let mut evidence_ids = decision.evidence_ids.clone();
    evidence_ids.extend(candidates.evidence_ids.clone());
    let telemetry = clean_room_assignment_planning_telemetry(
        decision_kind,
        decision,
        candidates.business_task_ids.len(),
        assignment_task_ids.len(),
        reasons.len(),
    );

    CleanRoomAssignmentPlanningEvidence {
        decision: decision_kind,
        next_round_decision_status: decision.decision_status.as_str().to_owned(),
        round_id_evidence: decision.round_id_evidence.clone(),
        business_task_ids: candidates.business_task_ids.clone(),
        assignment_task_ids,
        evidence_ids,
        next_round_evidence_ids: decision.evidence_ids.clone(),
        assignment_evidence_ids: candidates.evidence_ids.clone(),
        transition_kind: decision.transition_kind.clone(),
        active_round: decision.active_round,
        target_after_round,
        read_only: true,
        report_only: true,
        starts_process: false,
        sends_prompt: false,
        dispatch_side_effects_allowed: false,
        creates_thread: false,
        raw_payloads_retained: false,
        operator_attention_required: decision_kind
            == CleanRoomAssignmentPlanningDecisionKind::BlockOperatorAttention,
        reasons,
        telemetry,
    }
}

fn clean_room_assignment_telemetry(
    decision: CleanRoomAssignmentDecisionKind,
    facts: &SanitizedLiveStatusBundleFacts,
    business_task_ids: usize,
    assignment_task_ids: usize,
    reasons: usize,
) -> Vec<String> {
    vec![
        format!("clean_room_assignment_decision={}", decision.as_str()),
        format!("clean_room_assignment_business_task_ids={business_task_ids}"),
        format!("clean_room_assignment_assignment_task_ids={assignment_task_ids}"),
        format!(
            "clean_room_assignment_evidence_ids={}",
            facts.evidence_ids.len()
        ),
        format!(
            "clean_room_assignment_round_in_progress={}",
            facts.round_in_progress
        ),
        format!("clean_room_assignment_read_only={}", true),
        format!("clean_room_assignment_report_only={}", true),
        format!("clean_room_assignment_starts_process={}", false),
        format!("clean_room_assignment_sends_prompt={}", false),
        format!("clean_room_assignment_creates_thread={}", false),
        format!(
            "clean_room_assignment_dispatch_side_effects_allowed={}",
            false
        ),
        format!("clean_room_assignment_raw_payloads_retained={}", false),
        format!("clean_room_assignment_reasons={reasons}"),
    ]
}

fn clean_room_assignment_planning_telemetry(
    decision: CleanRoomAssignmentPlanningDecisionKind,
    facts: &SanitizedNextRoundDecisionFacts,
    business_task_ids: usize,
    assignment_task_ids: usize,
    reasons: usize,
) -> Vec<String> {
    vec![
        format!(
            "clean_room_assignment_planning_decision={}",
            decision.as_str()
        ),
        format!(
            "clean_room_assignment_planning_next_round_status={}",
            facts.decision_status.as_str()
        ),
        format!("clean_room_assignment_planning_business_task_ids={business_task_ids}"),
        format!("clean_room_assignment_planning_assignment_task_ids={assignment_task_ids}"),
        format!(
            "clean_room_assignment_planning_next_round_evidence_ids={}",
            facts.evidence_ids.len()
        ),
        format!("clean_room_assignment_planning_read_only={}", true),
        format!("clean_room_assignment_planning_report_only={}", true),
        format!("clean_room_assignment_planning_starts_process={}", false),
        format!("clean_room_assignment_planning_sends_prompt={}", false),
        format!("clean_room_assignment_planning_creates_thread={}", false),
        format!(
            "clean_room_assignment_planning_dispatch_side_effects_allowed={}",
            false
        ),
        format!(
            "clean_room_assignment_planning_raw_payloads_retained={}",
            false
        ),
        format!("clean_room_assignment_planning_reasons={reasons}"),
    ]
}

fn clean_room_assignment_acceptance_telemetry(
    decision: CleanRoomAssignmentAcceptanceDecisionKind,
    planning: &CleanRoomAssignmentPlanningEvidence,
    assignment_task_ids: usize,
    reasons: usize,
) -> Vec<String> {
    vec![
        format!(
            "clean_room_assignment_acceptance_decision={}",
            decision.as_str()
        ),
        format!(
            "clean_room_assignment_acceptance_planning_decision={}",
            planning.decision.as_str()
        ),
        format!(
            "clean_room_assignment_acceptance_next_round_status={}",
            planning.next_round_decision_status
        ),
        format!(
            "clean_room_assignment_acceptance_business_task_ids={}",
            planning.business_task_ids.len()
        ),
        format!("clean_room_assignment_acceptance_assignment_task_ids={assignment_task_ids}"),
        format!(
            "clean_room_assignment_acceptance_evidence_ids={}",
            planning.evidence_ids.len()
        ),
        format!(
            "clean_room_assignment_acceptance_next_round_evidence_ids={}",
            planning.next_round_evidence_ids.len()
        ),
        format!(
            "clean_room_assignment_acceptance_assignment_evidence_ids={}",
            planning.assignment_evidence_ids.len()
        ),
        format!("clean_room_assignment_acceptance_read_only={}", true),
        format!("clean_room_assignment_acceptance_report_only={}", true),
        format!("clean_room_assignment_acceptance_starts_process={}", false),
        format!("clean_room_assignment_acceptance_sends_prompt={}", false),
        format!("clean_room_assignment_acceptance_creates_thread={}", false),
        format!(
            "clean_room_assignment_acceptance_dispatch_side_effects_allowed={}",
            false
        ),
        format!(
            "clean_room_assignment_acceptance_process_start_allowed={}",
            false
        ),
        format!(
            "clean_room_assignment_acceptance_memory_write_allowed={}",
            false
        ),
        format!(
            "clean_room_assignment_acceptance_ndkv_write_allowed={}",
            false
        ),
        format!(
            "clean_room_assignment_acceptance_raw_payloads_retained={}",
            false
        ),
        format!("clean_room_assignment_acceptance_reasons={reasons}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_current_round_waits_without_assignment_side_effects() {
        let facts = SanitizedLiveStatusBundleFacts::active_round(
            "daemon_round_transition_status_v1:365",
            365,
            364,
        );

        let report = decide_clean_room_assignment(
            &facts,
            vec![
                "R34-clean-room-worker-B".to_owned(),
                "business-42".to_owned(),
            ],
        );

        assert_eq!(
            report.decision,
            CleanRoomAssignmentDecisionKind::WaitCurrentRoundActive
        );
        assert_eq!(
            report.business_task_ids,
            vec!["R34-clean-room-worker-B", "business-42"]
        );
        assert!(report.assignment_task_ids.is_empty());
        assert_eq!(
            report.evidence_ids,
            vec!["daemon_round_transition_status_v1:365"]
        );
        assert_eq!(report.active_round, Some(365));
        assert_eq!(report.target_after_round, None);
        assert!(report.read_only);
        assert!(report.report_only);
        assert!(!report.starts_process);
        assert!(!report.sends_prompt);
        assert!(!report.dispatch_side_effects_allowed);
        assert!(!report.creates_thread);
        assert!(!report.raw_payloads_retained);
        assert_eq!(report.reasons, vec!["current_daemon_round_active"]);
    }

    #[test]
    fn completed_current_round_reports_fresh_clean_room_assignment() {
        let facts = SanitizedLiveStatusBundleFacts::ready_after_round(
            "live_status_bundle_report_v1:round-365",
            365,
        )
        .with_evidence_ids(vec![
            "live_status_bundle_report_v1:round-365".to_owned(),
            "report_gate:pass".to_owned(),
        ]);

        let report = decide_clean_room_assignment(
            &facts,
            vec!["business-a".to_owned(), "business-b".to_owned()],
        );

        assert_eq!(
            report.decision,
            CleanRoomAssignmentDecisionKind::CreateFreshCleanRoomAssignmentAfterCurrentRound
        );
        assert!(report.can_create_assignment_report());
        assert_eq!(report.business_task_ids, vec!["business-a", "business-b"]);
        assert_eq!(report.assignment_task_ids, report.business_task_ids);
        assert_eq!(
            report.evidence_ids,
            vec!["live_status_bundle_report_v1:round-365", "report_gate:pass"]
        );
        assert_eq!(report.target_after_round, Some(365));
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| line == "clean_room_assignment_creates_thread=false")
        );
    }

    #[test]
    fn stale_or_polluted_evidence_blocks_and_drops_payloads() {
        let mut facts = SanitizedLiveStatusBundleFacts::ready_after_round(
            "live_status_bundle_report_v1:round-364",
            364,
        )
        .with_evidence_current(false)
        .with_polluted_evidence(true, false)
        .with_raw_payloads_present(true);
        facts.read_only = false;
        facts.starts_process = true;
        facts.sends_prompt = true;

        let report = decide_clean_room_assignment(&facts, vec!["business-kept".to_owned()]);

        assert_eq!(
            report.decision,
            CleanRoomAssignmentDecisionKind::BlockStaleOrPollutedEvidence
        );
        assert!(report.is_blocked());
        assert_eq!(report.business_task_ids, vec!["business-kept"]);
        assert!(report.assignment_task_ids.is_empty());
        assert_eq!(
            report.evidence_ids,
            vec!["live_status_bundle_report_v1:round-364"]
        );
        assert!(!report.starts_process);
        assert!(!report.sends_prompt);
        assert!(!report.dispatch_side_effects_allowed);
        assert!(!report.creates_thread);
        assert!(!report.raw_payloads_retained);
        assert_eq!(
            report.reasons,
            vec![
                "live_status_bundle_evidence_stale",
                "live_status_bundle_not_read_only",
                "live_status_bundle_starts_process",
                "live_status_bundle_sends_prompt",
                "live_status_bundle_raw_payloads_present",
                "live_status_bundle_polluted_evidence_actionable",
            ]
        );
    }

    #[test]
    fn non_actionable_polluted_window_evidence_remains_evidence_only() {
        let facts = SanitizedLiveStatusBundleFacts::ready_after_round(
            "live_status_bundle_report_v1:round-365",
            365,
        )
        .with_polluted_evidence(true, true);

        let report = decide_clean_room_assignment(&facts, vec!["business-a".to_owned()]);

        assert_eq!(
            report.decision,
            CleanRoomAssignmentDecisionKind::CreateFreshCleanRoomAssignmentAfterCurrentRound
        );
        assert_eq!(report.assignment_task_ids, vec!["business-a"]);
        assert!(!report.dispatch_side_effects_allowed);
        assert!(!report.creates_thread);
    }

    #[test]
    fn next_round_wait_status_keeps_candidates_as_evidence_only() {
        let assignment = decide_clean_room_assignment(
            &SanitizedLiveStatusBundleFacts::active_round(
                "daemon_round_transition_status_v1:367",
                367,
                366,
            ),
            vec![
                "R36-clean-room-worker-C".to_owned(),
                "business-77".to_owned(),
            ],
        );
        let next_round = SanitizedNextRoundDecisionFacts::safe_to_wait(
            "next_round_decision_report_v1:round-367",
            367,
            366,
        );

        let plan = plan_clean_room_assignment_from_next_round_decision(&next_round, &assignment);

        assert_eq!(
            plan.decision,
            CleanRoomAssignmentPlanningDecisionKind::WaitCurrentRoundActive
        );
        assert_eq!(
            plan.next_round_decision_status,
            "safe_to_wait_current_round_active"
        );
        assert_eq!(
            plan.business_task_ids,
            vec!["R36-clean-room-worker-C", "business-77"]
        );
        assert!(plan.assignment_task_ids.is_empty());
        assert_eq!(
            plan.evidence_ids,
            vec![
                "next_round_decision_report_v1:round-367",
                "daemon_round_transition_status_v1:367"
            ]
        );
        assert_eq!(plan.active_round, Some(367));
        assert_eq!(plan.target_after_round, None);
        assert!(plan.read_only);
        assert!(plan.report_only);
        assert!(!plan.dispatch_side_effects_allowed);
        assert!(!plan.creates_thread);
        assert!(!plan.raw_payloads_retained);
        assert!(!plan.operator_attention_required);
        assert_eq!(
            plan.reasons,
            vec!["next_round_decision_wait_current_round_active"]
        );
    }

    #[test]
    fn next_round_synced_completion_allows_fresh_assignment_evidence() {
        let assignment = decide_clean_room_assignment(
            &SanitizedLiveStatusBundleFacts::ready_after_round(
                "live_status_bundle_report_v1:round-366",
                366,
            ),
            vec!["business-a".to_owned(), "business-b".to_owned()],
        );
        let next_round = SanitizedNextRoundDecisionFacts::safe_to_continue(
            "next_round_decision_report_v1:round-366",
            366,
        )
        .with_evidence_ids(vec![
            "next_round_decision_report_v1:round-366".to_owned(),
            "readiness_next_round_v1:pass".to_owned(),
        ]);

        let plan = plan_clean_room_assignment_from_next_round_decision(&next_round, &assignment);

        assert_eq!(
            plan.decision,
            CleanRoomAssignmentPlanningDecisionKind::AllowFreshCleanRoomAssignmentAfterSyncedCompletion
        );
        assert!(plan.can_prepare_fresh_assignment());
        assert_eq!(plan.assignment_task_ids, vec!["business-a", "business-b"]);
        assert_eq!(plan.business_task_ids, plan.assignment_task_ids);
        assert_eq!(plan.target_after_round, Some(366));
        assert_eq!(
            plan.next_round_evidence_ids,
            vec![
                "next_round_decision_report_v1:round-366",
                "readiness_next_round_v1:pass"
            ]
        );
        assert_eq!(
            plan.assignment_evidence_ids,
            vec!["live_status_bundle_report_v1:round-366"]
        );
        assert!(plan.telemetry.iter().any(|line| {
            line == "clean_room_assignment_planning_dispatch_side_effects_allowed=false"
        }));
    }

    #[test]
    fn next_round_operator_attention_blocks_assignment_planning() {
        let assignment = decide_clean_room_assignment(
            &SanitizedLiveStatusBundleFacts::ready_after_round(
                "live_status_bundle_report_v1:round-366",
                366,
            ),
            vec!["business-a".to_owned()],
        );
        let next_round = SanitizedNextRoundDecisionFacts::operator_attention(
            "next_round_decision_report_v1:blocked",
            vec!["readiness_next_round_v1 blocked scheduling: validation gate failed".to_owned()],
        );

        let plan = plan_clean_room_assignment_from_next_round_decision(&next_round, &assignment);

        assert_eq!(
            plan.decision,
            CleanRoomAssignmentPlanningDecisionKind::BlockOperatorAttention
        );
        assert!(plan.requires_operator_attention());
        assert_eq!(plan.business_task_ids, vec!["business-a"]);
        assert!(plan.assignment_task_ids.is_empty());
        assert!(!plan.starts_process);
        assert!(!plan.sends_prompt);
        assert!(!plan.dispatch_side_effects_allowed);
        assert!(!plan.creates_thread);
        assert!(!plan.raw_payloads_retained);
        assert!(
            plan.reasons
                .contains(&"next_round_decision_operator_attention_required".to_owned())
        );
        assert!(
            plan.reasons
                .iter()
                .any(|reason| { reason.contains("readiness_next_round_v1 blocked scheduling") })
        );
    }

    #[test]
    fn next_round_planner_rejects_raw_payloads_and_side_effect_markers() {
        let mut assignment = decide_clean_room_assignment(
            &SanitizedLiveStatusBundleFacts::ready_after_round(
                "live_status_bundle_report_v1:round-366",
                366,
            ),
            vec!["business-a".to_owned()],
        );
        assignment.dispatch_side_effects_allowed = true;
        assignment.creates_thread = true;
        let mut next_round = SanitizedNextRoundDecisionFacts::safe_to_continue(
            "next_round_decision_report_v1:round-366",
            366,
        )
        .with_raw_payloads_present(true);
        next_round.no_side_effects = false;
        next_round.process_start_allowed = true;

        let plan = plan_clean_room_assignment_from_next_round_decision(&next_round, &assignment);

        assert_eq!(
            plan.decision,
            CleanRoomAssignmentPlanningDecisionKind::BlockOperatorAttention
        );
        assert_eq!(plan.business_task_ids, vec!["business-a"]);
        assert!(plan.assignment_task_ids.is_empty());
        assert!(!plan.starts_process);
        assert!(!plan.dispatch_side_effects_allowed);
        assert!(!plan.creates_thread);
        assert!(!plan.raw_payloads_retained);
        assert!(
            plan.reasons
                .contains(&"assignment_planning_rejects_runtime_side_effects".to_owned())
        );
        assert!(
            plan.reasons
                .contains(&"assignment_planning_drops_raw_payloads".to_owned())
        );
    }

    #[test]
    fn assignment_acceptance_surfaces_synced_plan_without_dispatch() {
        let assignment = decide_clean_room_assignment(
            &SanitizedLiveStatusBundleFacts::ready_after_round(
                "live_status_bundle_report_v1:round-368",
                368,
            ),
            vec!["R37-clean-room-worker-C".to_owned()],
        );
        let next_round = SanitizedNextRoundDecisionFacts::safe_to_continue(
            "next_round_decision_report_v1:round-368",
            368,
        )
        .with_evidence_ids(vec![
            "next_round_decision_report_v1:round-368".to_owned(),
            "readiness_next_round_v1:pass".to_owned(),
        ]);
        let plan = plan_clean_room_assignment_from_next_round_decision(&next_round, &assignment);

        let acceptance = accept_clean_room_assignment_planning_evidence(&plan);

        assert_eq!(
            acceptance.decision,
            CleanRoomAssignmentAcceptanceDecisionKind::AcceptFreshAssignmentPlanningEvidence
        );
        assert!(acceptance.can_surface_assignment_plan());
        assert_eq!(
            acceptance.assignment_task_ids,
            vec!["R37-clean-room-worker-C"]
        );
        assert_eq!(
            acceptance.evidence_ids,
            vec![
                "next_round_decision_report_v1:round-368",
                "readiness_next_round_v1:pass",
                "live_status_bundle_report_v1:round-368"
            ]
        );
        assert_eq!(
            acceptance.next_round_evidence_ids,
            vec![
                "next_round_decision_report_v1:round-368",
                "readiness_next_round_v1:pass"
            ]
        );
        assert_eq!(
            acceptance.assignment_evidence_ids,
            vec!["live_status_bundle_report_v1:round-368"]
        );
        assert_eq!(acceptance.target_after_round, Some(368));
        assert!(acceptance.read_only);
        assert!(acceptance.report_only);
        assert!(!acceptance.starts_process);
        assert!(!acceptance.sends_prompt);
        assert!(!acceptance.dispatch_side_effects_allowed);
        assert!(!acceptance.creates_thread);
        assert!(!acceptance.process_start_allowed);
        assert!(!acceptance.memory_write_allowed);
        assert!(!acceptance.ndkv_write_allowed);
        assert!(!acceptance.raw_payloads_retained);
        assert!(acceptance.telemetry.iter().any(|line| {
            line == "clean_room_assignment_acceptance_dispatch_side_effects_allowed=false"
        }));
    }

    #[test]
    fn assignment_acceptance_preserves_round_id_evidence_without_opening_side_effects() {
        let assignment = decide_clean_room_assignment(
            &SanitizedLiveStatusBundleFacts::ready_after_round(
                "live_status_bundle_report_v1:round-377",
                377,
            ),
            vec!["R44-clean-room-worker-B".to_owned()],
        );
        let next_round = SanitizedNextRoundDecisionFacts::safe_to_continue(
            "next_round_downstream_status_consumers_v1:round-377",
            377,
        )
        .with_evidence_ids(vec![
            "next_round_downstream_status_consumers_v1:round-377".to_owned(),
            "daemon_round_transition_status_v1:377".to_owned(),
        ])
        .with_round_id_evidence(Some(SanitizedRoundIdEvidence::daemon_transition(
            Some(378),
            Some(377),
            Some(377),
        )));
        let plan = plan_clean_room_assignment_from_next_round_decision(&next_round, &assignment);

        let acceptance = accept_clean_room_assignment_planning_evidence(&plan);

        assert_eq!(
            acceptance.decision,
            CleanRoomAssignmentAcceptanceDecisionKind::AcceptFreshAssignmentPlanningEvidence
        );
        assert!(acceptance.can_surface_assignment_plan());
        assert_eq!(
            acceptance.round_id_evidence,
            Some(SanitizedRoundIdEvidence {
                source_schema: "daemon_round_transition_status_v1".to_owned(),
                active_round: Some(378),
                ledger_latest_round: Some(377),
                latest_done_round: Some(377),
            })
        );
        assert_eq!(
            acceptance.next_round_evidence_ids,
            vec![
                "next_round_downstream_status_consumers_v1:round-377",
                "daemon_round_transition_status_v1:377",
            ]
        );
        assert_eq!(
            acceptance.assignment_task_ids,
            vec!["R44-clean-room-worker-B"]
        );
        assert!(!acceptance.starts_process);
        assert!(!acceptance.sends_prompt);
        assert!(!acceptance.dispatch_side_effects_allowed);
        assert!(!acceptance.creates_thread);
        assert!(!acceptance.process_start_allowed);
        assert!(!acceptance.memory_write_allowed);
        assert!(!acceptance.ndkv_write_allowed);
        assert!(!acceptance.raw_payloads_retained);
    }

    #[test]
    fn assignment_acceptance_waits_without_promoting_task_ids() {
        let assignment = decide_clean_room_assignment(
            &SanitizedLiveStatusBundleFacts::active_round(
                "daemon_round_transition_status_v1:369",
                369,
                368,
            ),
            vec!["R37-clean-room-worker-C".to_owned()],
        );
        let next_round = SanitizedNextRoundDecisionFacts::safe_to_wait(
            "next_round_decision_report_v1:round-369",
            369,
            368,
        );
        let plan = plan_clean_room_assignment_from_next_round_decision(&next_round, &assignment);

        let acceptance = accept_clean_room_assignment_planning_evidence(&plan);

        assert_eq!(
            acceptance.decision,
            CleanRoomAssignmentAcceptanceDecisionKind::WaitCurrentRoundPlanningEvidence
        );
        assert!(acceptance.is_wait_only());
        assert!(!acceptance.accepted_for_agent_planning);
        assert!(acceptance.assignment_task_ids.is_empty());
        assert_eq!(acceptance.active_round, Some(369));
        assert_eq!(acceptance.target_after_round, None);
        assert_eq!(
            acceptance.evidence_ids,
            vec![
                "next_round_decision_report_v1:round-369",
                "daemon_round_transition_status_v1:369"
            ]
        );
        assert!(!acceptance.operator_attention_required);
        assert!(!acceptance.starts_process);
        assert!(!acceptance.creates_thread);
        assert!(
            acceptance
                .reasons
                .contains(&"assignment_acceptance_waits_for_current_round".to_owned())
        );
    }

    #[test]
    fn assignment_acceptance_rejects_raw_payloads_and_side_effect_markers() {
        let assignment = decide_clean_room_assignment(
            &SanitizedLiveStatusBundleFacts::ready_after_round(
                "live_status_bundle_report_v1:round-368",
                368,
            ),
            vec!["business-a".to_owned()],
        );
        let next_round = SanitizedNextRoundDecisionFacts::safe_to_continue(
            "next_round_decision_report_v1:round-368",
            368,
        );
        let mut plan =
            plan_clean_room_assignment_from_next_round_decision(&next_round, &assignment);
        plan.starts_process = true;
        plan.creates_thread = true;
        plan.raw_payloads_retained = true;

        let acceptance = accept_clean_room_assignment_planning_evidence(&plan);

        assert_eq!(
            acceptance.decision,
            CleanRoomAssignmentAcceptanceDecisionKind::RejectPlanningEvidence
        );
        assert!(!acceptance.can_surface_assignment_plan());
        assert!(acceptance.assignment_task_ids.is_empty());
        assert_eq!(acceptance.business_task_ids, vec!["business-a"]);
        assert_eq!(
            acceptance.evidence_ids,
            vec![
                "next_round_decision_report_v1:round-368",
                "live_status_bundle_report_v1:round-368"
            ]
        );
        assert!(!acceptance.starts_process);
        assert!(!acceptance.creates_thread);
        assert!(!acceptance.dispatch_side_effects_allowed);
        assert!(!acceptance.raw_payloads_retained);
        assert!(acceptance.operator_attention_required);
        assert!(
            acceptance
                .reasons
                .contains(&"assignment_acceptance_rejects_runtime_side_effects".to_owned())
        );
        assert!(
            acceptance
                .reasons
                .contains(&"assignment_acceptance_rejects_raw_payloads".to_owned())
        );
    }
}
