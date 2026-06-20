use std::collections::{BTreeMap, BTreeSet};

use crate::{MemoryScope, clamp01};

#[derive(Debug, Clone, PartialEq)]
pub struct ExperienceEnvelope {
    pub id: String,
    pub prompt: String,
    pub lesson: String,
    pub clean_gist: Option<String>,
    pub quality: f32,
    pub tags: Vec<String>,
    pub scope: MemoryScope,
}

impl ExperienceEnvelope {
    pub fn new(
        id: impl Into<String>,
        prompt: impl Into<String>,
        lesson: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            prompt: prompt.into(),
            lesson: lesson.into(),
            clean_gist: None,
            quality: 0.5,
            tags: Vec::new(),
            scope: MemoryScope::default(),
        }
    }

    pub fn with_scope(mut self, scope: MemoryScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_clean_gist(mut self, clean_gist: impl Into<String>) -> Self {
        self.clean_gist = Some(clean_gist.into());
        self
    }

    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = clamp01(quality);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfImproveProposalSource {
    CleanRoomWorker,
    ReportOnlyContract,
    OldWindow,
    LegacyThread,
    Unknown,
}

impl SelfImproveProposalSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CleanRoomWorker => "clean_room_worker",
            Self::ReportOnlyContract => "report_only_contract",
            Self::OldWindow => "old_window",
            Self::LegacyThread => "legacy_thread",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_clean_source(self) -> bool {
        matches!(self, Self::CleanRoomWorker | Self::ReportOnlyContract)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfImproveProposalRepairState {
    NotRequired,
    RepairRequired,
    RepairApplied,
}

impl SelfImproveProposalRepairState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotRequired => "not_required",
            Self::RepairRequired => "repair_required",
            Self::RepairApplied => "repair_applied",
        }
    }

    pub fn blocks_admission(self) -> bool {
        matches!(self, Self::RepairRequired)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfImproveNextRoundDecision {
    Unknown,
    SafeToWaitCurrentRoundActive,
    SafeToContinueAfterCurrentRound,
    OperatorAttentionBlocked,
}

impl SelfImproveNextRoundDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::SafeToWaitCurrentRoundActive => "safe-to-wait-current-round-active",
            Self::SafeToContinueAfterCurrentRound => "safe-to-continue-after-current-round",
            Self::OperatorAttentionBlocked => "operator-attention-blocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfImproveRoundIdEvidence {
    pub source_schema: String,
    pub active_round: Option<u64>,
    pub ledger_latest_round: Option<u64>,
    pub latest_done_round: Option<u64>,
}

impl SelfImproveRoundIdEvidence {
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

    pub fn is_sanitized_downstream_status_evidence(&self) -> bool {
        self.source_schema == "daemon_round_transition_status_v1"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfImproveLearningEvidence {
    pub live_status_healthy: bool,
    pub report_gate_passed: bool,
    pub validation_gate_passed: bool,
    pub test_gate_passed: bool,
    pub helper_stage_contract_complete: bool,
    pub evidence_ids: Vec<String>,
    pub source_window_polluted: bool,
    pub source_window_actionable: bool,
    pub next_round_decision: SelfImproveNextRoundDecision,
    pub next_round_round_id_evidence: Option<SelfImproveRoundIdEvidence>,
    pub next_round_live_status_synced: bool,
    pub next_round_current_round_active: bool,
    pub next_round_side_effect_markers: Vec<String>,
    pub next_round_raw_window_markers: Vec<String>,
}

impl Default for SelfImproveLearningEvidence {
    fn default() -> Self {
        Self {
            live_status_healthy: false,
            report_gate_passed: false,
            validation_gate_passed: false,
            test_gate_passed: false,
            helper_stage_contract_complete: false,
            evidence_ids: Vec::new(),
            source_window_polluted: true,
            source_window_actionable: true,
            next_round_decision: SelfImproveNextRoundDecision::Unknown,
            next_round_round_id_evidence: None,
            next_round_live_status_synced: false,
            next_round_current_round_active: false,
            next_round_side_effect_markers: Vec::new(),
            next_round_raw_window_markers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfImproveLearningProposal {
    pub proposal_id: String,
    pub source_round: u64,
    pub source: SelfImproveProposalSource,
    pub repair_state: SelfImproveProposalRepairState,
    pub validation_passed: bool,
    pub feedback_applied: bool,
    pub clean_gist: Option<String>,
    pub payload: Option<String>,
    pub payload_clean: bool,
    pub scope: MemoryScope,
    pub tags: Vec<String>,
    pub evidence: SelfImproveLearningEvidence,
}

impl SelfImproveLearningProposal {
    pub fn new(proposal_id: impl Into<String>, source_round: u64) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            source_round,
            source: SelfImproveProposalSource::CleanRoomWorker,
            repair_state: SelfImproveProposalRepairState::NotRequired,
            validation_passed: false,
            feedback_applied: false,
            clean_gist: None,
            payload: None,
            payload_clean: true,
            scope: MemoryScope::default(),
            tags: Vec::new(),
            evidence: SelfImproveLearningEvidence::default(),
        }
    }

    pub fn with_source(mut self, source: SelfImproveProposalSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_repair_state(mut self, repair_state: SelfImproveProposalRepairState) -> Self {
        self.repair_state = repair_state;
        self
    }

    pub fn with_validation_passed(mut self, passed: bool) -> Self {
        self.validation_passed = passed;
        self
    }

    pub fn with_feedback_applied(mut self, applied: bool) -> Self {
        self.feedback_applied = applied;
        self
    }

    pub fn with_clean_gist(mut self, clean_gist: impl Into<String>) -> Self {
        self.clean_gist = Some(clean_gist.into());
        self
    }

    pub fn with_payload(mut self, payload: impl Into<String>) -> Self {
        self.payload = Some(payload.into());
        self
    }

    pub fn with_payload_clean(mut self, clean: bool) -> Self {
        self.payload_clean = clean;
        self
    }

    pub fn with_scope(mut self, scope: MemoryScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_live_status_report_gate_evidence(
        mut self,
        live_status_healthy: bool,
        report_gate_passed: bool,
    ) -> Self {
        self.evidence.live_status_healthy = live_status_healthy;
        self.evidence.report_gate_passed = report_gate_passed;
        self
    }

    pub fn with_validation_test_gate_evidence(
        mut self,
        validation_gate_passed: bool,
        test_gate_passed: bool,
    ) -> Self {
        self.evidence.validation_gate_passed = validation_gate_passed;
        self.evidence.test_gate_passed = test_gate_passed;
        self
    }

    pub fn with_helper_stage_contract_complete(mut self, complete: bool) -> Self {
        self.evidence.helper_stage_contract_complete = complete;
        self
    }

    pub fn with_evidence_ids(mut self, evidence_ids: Vec<String>) -> Self {
        self.evidence.evidence_ids = evidence_ids;
        self
    }

    pub fn with_source_window_status(mut self, polluted: bool, actionable: bool) -> Self {
        self.evidence.source_window_polluted = polluted;
        self.evidence.source_window_actionable = actionable;
        self
    }

    pub fn with_clean_source_windows(self) -> Self {
        self.with_source_window_status(false, false)
    }

    pub fn with_next_round_decision_evidence(
        mut self,
        decision: SelfImproveNextRoundDecision,
        live_status_synced: bool,
        current_round_active: bool,
    ) -> Self {
        self.evidence.next_round_decision = decision;
        self.evidence.next_round_live_status_synced = live_status_synced;
        self.evidence.next_round_current_round_active = current_round_active;
        self
    }

    pub fn with_next_round_round_id_evidence(
        mut self,
        evidence: Option<SelfImproveRoundIdEvidence>,
    ) -> Self {
        self.evidence.next_round_round_id_evidence = evidence;
        self
    }

    pub fn with_next_round_side_effect_markers(mut self, markers: Vec<String>) -> Self {
        self.evidence.next_round_side_effect_markers = markers;
        self
    }

    pub fn with_next_round_raw_window_markers(mut self, markers: Vec<String>) -> Self {
        self.evidence.next_round_raw_window_markers = markers;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfImproveAdmissionDecision {
    AcceptEnvelope,
    QuarantineCandidate,
    Reject,
}

impl SelfImproveAdmissionDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcceptEnvelope => "accept_envelope",
            Self::QuarantineCandidate => "quarantine_candidate",
            Self::Reject => "reject",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfImproveAdmissionWriteMode {
    ReadOnly,
    IsolatedWrite,
}

impl SelfImproveAdmissionWriteMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::IsolatedWrite => "isolated_write",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfImproveLearningAdmissionPlan {
    pub candidate_id: String,
    pub proposal_id: String,
    pub source_round: u64,
    pub source: SelfImproveProposalSource,
    pub repair_state: SelfImproveProposalRepairState,
    pub decision: SelfImproveAdmissionDecision,
    pub write_mode: SelfImproveAdmissionWriteMode,
    pub envelope: Option<ExperienceEnvelope>,
    pub reason_codes: Vec<String>,
    pub detail_codes: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub live_status_healthy: bool,
    pub report_gate_passed: bool,
    pub validation_gate_passed: bool,
    pub test_gate_passed: bool,
    pub helper_stage_contract_complete: bool,
    pub source_window_polluted: bool,
    pub source_window_actionable: bool,
    pub next_round_decision: SelfImproveNextRoundDecision,
    pub next_round_round_id_evidence: Option<SelfImproveRoundIdEvidence>,
    pub next_round_live_status_synced: bool,
    pub next_round_current_round_active: bool,
    pub next_round_side_effect_marker_codes: Vec<String>,
    pub next_round_raw_window_marker_codes: Vec<String>,
}

impl SelfImproveLearningAdmissionPlan {
    pub fn live_store_mutation_allowed(&self) -> bool {
        false
    }

    pub fn ndkv_write_allowed(&self) -> bool {
        false
    }

    pub fn accepted_envelope(&self) -> Option<&ExperienceEnvelope> {
        self.envelope.as_ref()
    }

    pub fn memory_candidate_ready(&self) -> bool {
        self.decision == SelfImproveAdmissionDecision::AcceptEnvelope
            && self.envelope.is_some()
            && self.reason_codes.is_empty()
            && self.live_status_healthy
            && self.report_gate_passed
            && self.validation_gate_passed
            && self.test_gate_passed
            && self.helper_stage_contract_complete
            && !self.evidence_ids.is_empty()
            && !self.source_window_polluted
            && !self.source_window_actionable
            && self.next_round_decision
                == SelfImproveNextRoundDecision::SafeToContinueAfterCurrentRound
            && self
                .next_round_round_id_evidence
                .as_ref()
                .is_none_or(SelfImproveRoundIdEvidence::is_sanitized_downstream_status_evidence)
            && self.next_round_live_status_synced
            && !self.next_round_current_round_active
            && self.next_round_side_effect_marker_codes.is_empty()
            && self.next_round_raw_window_marker_codes.is_empty()
            && self.read_only_or_isolated_contract_holds()
            && self.write_mode == SelfImproveAdmissionWriteMode::ReadOnly
    }

    pub fn read_only_or_isolated_contract_holds(&self) -> bool {
        !self.live_store_mutation_allowed()
            && !self.ndkv_write_allowed()
            && matches!(
                self.write_mode,
                SelfImproveAdmissionWriteMode::ReadOnly
                    | SelfImproveAdmissionWriteMode::IsolatedWrite
            )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_improve_learning_admission candidate={} proposal={} source_round={} source={} repair_state={} decision={} write_mode={} memory_candidate_ready={} envelope_ready={} live_status_healthy={} report_gate_passed={} validation_gate_passed={} test_gate_passed={} helper_stage_contract_complete={} evidence_ids={} source_window_polluted={} source_window_actionable={} next_round_decision={} next_round_live_status_synced={} next_round_current_round_active={} next_round_round_id_evidence={} next_round_side_effect_markers={} next_round_raw_window_markers={} live_store_mutation_allowed={} ndkv_write_allowed={} reason_codes={} detail_codes={}",
            stable_detail_part(&self.candidate_id),
            stable_detail_part(&self.proposal_id),
            self.source_round,
            self.source.as_str(),
            self.repair_state.as_str(),
            self.decision.as_str(),
            self.write_mode.as_str(),
            self.memory_candidate_ready(),
            self.envelope.is_some(),
            self.live_status_healthy,
            self.report_gate_passed,
            self.validation_gate_passed,
            self.test_gate_passed,
            self.helper_stage_contract_complete,
            join_reason_codes(
                self.evidence_ids
                    .iter()
                    .map(|id| stable_detail_part(id))
                    .collect()
            ),
            self.source_window_polluted,
            self.source_window_actionable,
            self.next_round_decision.as_str(),
            self.next_round_live_status_synced,
            self.next_round_current_round_active,
            self.next_round_round_id_evidence
                .as_ref()
                .map(round_id_evidence_summary)
                .unwrap_or_else(|| "none".to_owned()),
            join_reason_codes(self.next_round_side_effect_marker_codes.clone()),
            join_reason_codes(self.next_round_raw_window_marker_codes.clone()),
            self.live_store_mutation_allowed(),
            self.ndkv_write_allowed(),
            join_reason_codes(self.reason_codes.clone()),
            join_reason_codes(self.detail_codes.clone()),
        )
    }
}

pub fn admit_self_improve_learning_candidate(
    proposal: SelfImproveLearningProposal,
) -> SelfImproveLearningAdmissionPlan {
    let proposal_id = proposal.proposal_id.trim().to_owned();
    let candidate_id = if proposal_id.is_empty() {
        format!("self_improve_round_{}", proposal.source_round)
    } else {
        format!(
            "self_improve_round_{}_{}",
            proposal.source_round,
            stable_detail_part(&proposal_id)
        )
    };
    let mut reason_codes = Vec::new();

    if proposal_id.is_empty() {
        reason_codes.push("missing_proposal_id".to_owned());
    }
    if proposal.source_round == 0 {
        reason_codes.push("missing_source_round".to_owned());
    }
    if !proposal.validation_passed {
        reason_codes.push("validation_not_passed".to_owned());
    }
    if !proposal.evidence.live_status_healthy {
        reason_codes.push("live_status_not_healthy".to_owned());
    }
    if !proposal.evidence.report_gate_passed {
        reason_codes.push("report_gate_not_passed".to_owned());
    }
    if !proposal.evidence.validation_gate_passed {
        reason_codes.push("validation_gate_not_passed".to_owned());
    }
    if !proposal.evidence.test_gate_passed {
        reason_codes.push("test_gate_not_passed".to_owned());
    }
    if !proposal.evidence.helper_stage_contract_complete {
        reason_codes.push("helper_stage_contract_incomplete".to_owned());
    }
    if proposal.repair_state.blocks_admission() {
        reason_codes.push("repair_required_not_applied".to_owned());
    }
    if !proposal.feedback_applied {
        reason_codes.push("feedback_not_applied".to_owned());
    }
    if !proposal.source.is_clean_source() {
        reason_codes.push(
            match proposal.source {
                SelfImproveProposalSource::OldWindow | SelfImproveProposalSource::LegacyThread => {
                    "old_window_source"
                }
                SelfImproveProposalSource::Unknown => "unknown_source",
                SelfImproveProposalSource::CleanRoomWorker
                | SelfImproveProposalSource::ReportOnlyContract => "source_not_clean_room",
            }
            .to_owned(),
        );
    }
    if proposal.scope.task_id.as_deref().is_none_or(str::is_empty) {
        reason_codes.push("missing_scope".to_owned());
    }
    if proposal.tags.iter().all(|tag| tag.trim().is_empty()) {
        reason_codes.push("missing_tags".to_owned());
    }
    let clean_evidence_ids = clean_evidence_ids(&proposal.evidence.evidence_ids);
    if clean_evidence_ids.is_empty() {
        reason_codes.push("missing_evidence_ids".to_owned());
    }
    if clean_evidence_ids.len() != proposal.evidence.evidence_ids.len() {
        reason_codes.push("dirty_evidence_id".to_owned());
    }
    if proposal.evidence.source_window_polluted {
        reason_codes.push("source_window_polluted".to_owned());
    }
    if proposal.evidence.source_window_actionable {
        reason_codes.push("source_window_actionable".to_owned());
    }
    let next_round_side_effect_marker_codes =
        clean_next_round_marker_codes(&proposal.evidence.next_round_side_effect_markers);
    let next_round_raw_window_marker_codes =
        clean_next_round_marker_codes(&proposal.evidence.next_round_raw_window_markers);
    let next_round_round_id_evidence = proposal.evidence.next_round_round_id_evidence.clone();
    if next_round_round_id_evidence
        .as_ref()
        .is_some_and(|evidence| !evidence.is_sanitized_downstream_status_evidence())
    {
        reason_codes.push("next_round_round_id_evidence_source_untrusted".to_owned());
    }
    match proposal.evidence.next_round_decision {
        SelfImproveNextRoundDecision::SafeToContinueAfterCurrentRound => {
            if !proposal.evidence.next_round_live_status_synced {
                reason_codes.push("next_round_evidence_not_synced".to_owned());
            }
            if proposal.evidence.next_round_current_round_active {
                reason_codes.push("next_round_current_round_active".to_owned());
            }
        }
        SelfImproveNextRoundDecision::SafeToWaitCurrentRoundActive => {
            reason_codes.push("next_round_wait_current_round_active".to_owned());
            if !proposal.evidence.next_round_current_round_active {
                reason_codes.push("next_round_wait_missing_active_round".to_owned());
            }
        }
        SelfImproveNextRoundDecision::OperatorAttentionBlocked => {
            reason_codes.push("next_round_operator_attention".to_owned());
        }
        SelfImproveNextRoundDecision::Unknown => {
            reason_codes.push("next_round_decision_missing".to_owned());
        }
    }
    if !next_round_side_effect_marker_codes.is_empty() {
        reason_codes.push("next_round_side_effect_marker".to_owned());
    }
    if !next_round_raw_window_marker_codes.is_empty() {
        reason_codes.push("next_round_raw_window_marker".to_owned());
    }

    let clean_gist = proposal.clean_gist.as_deref().map(str::trim);
    match clean_gist {
        None | Some("") => reason_codes.push("missing_clean_gist".to_owned()),
        Some(gist) if !is_clean_gist(gist) => {
            reason_codes.push("dirty_clean_gist".to_owned());
        }
        Some(_) => {}
    }

    if !proposal.payload_clean {
        reason_codes.push("payload_marked_dirty".to_owned());
    }
    if let Some(payload) = proposal.payload.as_deref() {
        if payload_declares_repair_required(payload) {
            reason_codes.push("payload_repair_required_marker".to_owned());
        }
        reason_codes.extend(dirty_self_improve_payload_reason_codes(payload));
    }
    sort_dedup(&mut reason_codes);

    let decision = if reason_codes.iter().any(|code| {
        matches!(
            code.as_str(),
            "missing_proposal_id"
                | "missing_source_round"
                | "validation_not_passed"
                | "live_status_not_healthy"
                | "report_gate_not_passed"
                | "validation_gate_not_passed"
                | "test_gate_not_passed"
                | "helper_stage_contract_incomplete"
                | "missing_evidence_ids"
                | "source_window_polluted"
                | "source_window_actionable"
                | "next_round_decision_missing"
                | "next_round_evidence_not_synced"
                | "next_round_current_round_active"
                | "next_round_wait_current_round_active"
                | "next_round_wait_missing_active_round"
                | "next_round_operator_attention"
                | "missing_clean_gist"
                | "missing_scope"
                | "missing_tags"
        )
    }) {
        SelfImproveAdmissionDecision::Reject
    } else if reason_codes.is_empty() {
        SelfImproveAdmissionDecision::AcceptEnvelope
    } else {
        SelfImproveAdmissionDecision::QuarantineCandidate
    };
    let write_mode = SelfImproveAdmissionWriteMode::ReadOnly;
    let envelope = if decision == SelfImproveAdmissionDecision::AcceptEnvelope {
        clean_gist.map(|gist| {
            let mut tags = proposal
                .tags
                .iter()
                .map(|tag| tag.trim().to_owned())
                .filter(|tag| !tag.is_empty())
                .chain([
                    "self-improve".to_owned(),
                    "learning-candidate".to_owned(),
                    format!("source-round:{}", proposal.source_round),
                ])
                .collect::<Vec<_>>();
            sort_dedup(&mut tags);
            ExperienceEnvelope::new(
                candidate_id.clone(),
                format!(
                    "validated self-improve proposal {} from round {}",
                    stable_detail_part(&proposal_id),
                    proposal.source_round
                ),
                gist,
            )
            .with_clean_gist(gist)
            .with_quality(0.86)
            .with_scope(proposal.scope.clone())
            .with_tags(tags)
        })
    } else {
        None
    };
    let detail_codes = reason_codes
        .iter()
        .map(|reason| {
            format!(
                "self_improve_learning:{}:{}",
                stable_detail_part(&candidate_id),
                stable_detail_part(reason)
            )
        })
        .collect();

    SelfImproveLearningAdmissionPlan {
        candidate_id,
        proposal_id,
        source_round: proposal.source_round,
        source: proposal.source,
        repair_state: proposal.repair_state,
        decision,
        write_mode,
        envelope,
        reason_codes,
        detail_codes,
        evidence_ids: clean_evidence_ids,
        live_status_healthy: proposal.evidence.live_status_healthy,
        report_gate_passed: proposal.evidence.report_gate_passed,
        validation_gate_passed: proposal.evidence.validation_gate_passed,
        test_gate_passed: proposal.evidence.test_gate_passed,
        helper_stage_contract_complete: proposal.evidence.helper_stage_contract_complete,
        source_window_polluted: proposal.evidence.source_window_polluted,
        source_window_actionable: proposal.evidence.source_window_actionable,
        next_round_decision: proposal.evidence.next_round_decision,
        next_round_round_id_evidence,
        next_round_live_status_synced: proposal.evidence.next_round_live_status_synced,
        next_round_current_round_active: proposal.evidence.next_round_current_round_active,
        next_round_side_effect_marker_codes,
        next_round_raw_window_marker_codes,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GistStatus {
    Clean,
    Missing,
    Dirty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateGroup {
    pub canonical_id: String,
    pub duplicate_ids: Vec<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeduplicationReport {
    pub total_records: usize,
    pub duplicate_group_count: usize,
    pub duplicate_record_count: usize,
    pub groups: Vec<DuplicateGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoiseAssessment {
    pub experience_id: String,
    pub score: f32,
    pub gist_status: GistStatus,
    pub reasons: Vec<String>,
}

impl NoiseAssessment {
    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = self.reasons.clone();
        sort_dedup(&mut codes);
        codes
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let experience_id = stable_detail_part(&self.experience_id);
        self.reason_codes()
            .into_iter()
            .map(|reason| format!("noise:{experience_id}:{}", stable_detail_part(&reason)))
            .chain(std::iter::once(format!(
                "noise:{experience_id}:gist_{}",
                self.gist_status.as_str()
            )))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn summary_line(&self) -> String {
        let reason_codes = self.reason_codes();
        let detail_codes = self.detail_codes();
        format!(
            "noise_assessment experience={} score={:.3} gist_status={} reasons={} reason_codes={} detail_codes={}",
            stable_detail_part(&self.experience_id),
            self.score,
            self.gist_status.as_str(),
            reason_codes.len(),
            join_reason_codes(reason_codes),
            join_reason_codes(detail_codes),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextRotRisk {
    pub experience_id: String,
    pub score: f32,
    pub reasons: Vec<String>,
}

impl ContextRotRisk {
    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = self.reasons.clone();
        sort_dedup(&mut codes);
        codes
    }

    pub fn context_injection_blocker_reason_codes(&self) -> Vec<String> {
        let mut codes = self
            .reasons
            .iter()
            .filter(|reason| is_context_rot_blocker_reason(reason))
            .cloned()
            .collect::<Vec<_>>();
        sort_dedup(&mut codes);
        codes
    }

    pub fn requires_context_injection_blocker(&self) -> bool {
        !self.context_injection_blocker_reason_codes().is_empty()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let experience_id = stable_detail_part(&self.experience_id);
        self.reason_codes()
            .into_iter()
            .map(|reason| {
                format!(
                    "context_rot:{experience_id}:{}",
                    stable_detail_part(&reason)
                )
            })
            .collect()
    }

    pub fn summary_line(&self) -> String {
        let reason_codes = self.reason_codes();
        let detail_codes = self.detail_codes();
        format!(
            "context_rot_risk experience={} score={:.3} reasons={} reason_codes={} detail_codes={}",
            stable_detail_part(&self.experience_id),
            self.score,
            reason_codes.len(),
            join_reason_codes(reason_codes),
            join_reason_codes(detail_codes),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GovernanceReport {
    pub total_records: usize,
    pub deduplication: DeduplicationReport,
    pub noisy_records: Vec<NoiseAssessment>,
    pub context_rot_risks: Vec<ContextRotRisk>,
}

impl GovernanceReport {
    pub fn context_rot_blocker_count(&self) -> usize {
        self.context_rot_risks
            .iter()
            .filter(|risk| risk.requires_context_injection_blocker())
            .count()
    }

    pub fn context_rot_blocker_reason_codes(&self) -> Vec<String> {
        self.context_rot_risks
            .iter()
            .flat_map(|risk| risk.context_injection_blocker_reason_codes())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = self
            .noisy_records
            .iter()
            .flat_map(|noise| noise.reason_codes())
            .chain(
                self.context_rot_risks
                    .iter()
                    .flat_map(|risk| risk.reason_codes()),
            )
            .collect::<Vec<_>>();
        sort_dedup(&mut codes);
        codes
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        for group in &self.deduplication.groups {
            let canonical_id = stable_detail_part(&group.canonical_id);
            for duplicate_id in &group.duplicate_ids {
                codes.insert(format!(
                    "duplicate:{canonical_id}:{}",
                    stable_detail_part(duplicate_id)
                ));
            }
        }
        for noise in &self.noisy_records {
            codes.extend(noise.detail_codes());
        }
        for risk in &self.context_rot_risks {
            codes.extend(risk.detail_codes());
        }
        codes.into_iter().collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_governance records={} duplicate_groups={} duplicate_records={} noisy={} context_rot={} reason_codes={} detail_codes={}",
            self.total_records,
            self.deduplication.duplicate_group_count,
            self.deduplication.duplicate_record_count,
            self.noisy_records.len(),
            self.context_rot_risks.len(),
            join_reason_codes(self.reason_codes()),
            join_reason_codes(self.detail_codes()),
        )
    }

    pub fn quality_gate(&self, rebuild: &IndexRebuildPlan) -> ExperienceIndexQualityGate {
        ExperienceIndexQualityGate::from_report_and_plan(self, rebuild)
    }
}

impl GistStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Missing => "missing",
            Self::Dirty => "dirty",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct IndexRebuildPlan {
    pub rebuild_required: bool,
    pub deduplicate_groups: Vec<DuplicateGroup>,
    pub refresh_embedding_ids: Vec<String>,
    pub compact_ids: Vec<String>,
    pub quarantine_candidate_ids: Vec<String>,
    pub missing_clean_gist_ids: Vec<String>,
    pub dirty_clean_gist_ids: Vec<String>,
    pub dirty_gist_ids: Vec<String>,
    pub reasons: Vec<String>,
}

impl IndexRebuildPlan {
    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = self.reasons.clone();
        sort_dedup(&mut codes);
        codes
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        for group in &self.deduplicate_groups {
            let canonical_id = stable_detail_part(&group.canonical_id);
            for duplicate_id in &group.duplicate_ids {
                codes.insert(format!(
                    "deduplicate:{canonical_id}:{}",
                    stable_detail_part(duplicate_id)
                ));
            }
        }
        for id in &self.refresh_embedding_ids {
            codes.insert(format!("refresh:{}", stable_detail_part(id)));
        }
        for id in &self.compact_ids {
            codes.insert(format!("compact:{}", stable_detail_part(id)));
        }
        for id in &self.quarantine_candidate_ids {
            codes.insert(format!("quarantine:{}", stable_detail_part(id)));
        }
        codes.extend(self.clean_gist_repair_detail_codes());
        codes.into_iter().collect()
    }

    pub fn clean_gist_repair_detail_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        for id in &self.missing_clean_gist_ids {
            codes.insert(format!("missing_clean_gist:{}", stable_detail_part(id)));
        }
        for id in &self.dirty_clean_gist_ids {
            codes.insert(format!("dirty_clean_gist:{}", stable_detail_part(id)));
        }
        for id in &self.dirty_gist_ids {
            codes.insert(format!("dirty_gist:{}", stable_detail_part(id)));
        }
        codes.into_iter().collect()
    }

    pub fn clean_gist_repair_summary_line(&self) -> String {
        format!(
            "clean_gist_repair missing_clean_gist={} dirty_clean_gist={} dirty_gist={} detail_codes={}",
            self.missing_clean_gist_ids.len(),
            self.dirty_clean_gist_ids.len(),
            self.dirty_gist_ids.len(),
            join_reason_codes(self.clean_gist_repair_detail_codes()),
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_rebuild required={} duplicate_groups={} refresh={} compact={} quarantine={} missing_clean_gist={} dirty_clean_gist={} dirty_gist={} reasons={} reason_codes={} detail_codes={}",
            self.rebuild_required,
            self.deduplicate_groups.len(),
            self.refresh_embedding_ids.len(),
            self.compact_ids.len(),
            self.quarantine_candidate_ids.len(),
            self.missing_clean_gist_ids.len(),
            self.dirty_clean_gist_ids.len(),
            self.dirty_gist_ids.len(),
            self.reasons.len(),
            join_reason_codes(self.reason_codes()),
            join_reason_codes(self.detail_codes()),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExperienceIndexQualityGate {
    pub ready_for_context_injection: bool,
    pub total_records: usize,
    pub blocker_count: usize,
    pub warning_count: usize,
    pub duplicate_record_count: usize,
    pub refresh_count: usize,
    pub compact_count: usize,
    pub quarantine_count: usize,
    pub missing_clean_gist_count: usize,
    pub dirty_clean_gist_count: usize,
    pub dirty_gist_count: usize,
    pub context_rot_blocker_count: usize,
    pub context_rot_blocker_reason_codes: Vec<String>,
    pub reasons: Vec<String>,
    pub details: Vec<String>,
}

impl ExperienceIndexQualityGate {
    pub fn from_report_and_plan(report: &GovernanceReport, plan: &IndexRebuildPlan) -> Self {
        let duplicate_record_count = report.deduplication.duplicate_record_count;
        let compact_count = plan.compact_ids.len();
        let quarantine_count = plan.quarantine_candidate_ids.len();
        let dirty_gist_count = plan.dirty_gist_ids.len();
        let refresh_count = plan.refresh_embedding_ids.len();
        let missing_clean_gist_count = plan.missing_clean_gist_ids.len();
        let dirty_clean_gist_count = plan.dirty_clean_gist_ids.len();
        let context_rot_blocker_count = report.context_rot_blocker_count();
        let context_rot_blocker_reason_codes = report.context_rot_blocker_reason_codes();
        let blocker_count = duplicate_record_count + compact_count + quarantine_count;
        let warning_count = refresh_count + missing_clean_gist_count + dirty_clean_gist_count;
        let ready_for_context_injection = blocker_count == 0 && dirty_gist_count == 0;
        let mut reasons = Vec::new();

        if duplicate_record_count > 0 {
            reasons.push("duplicate_experience".to_owned());
        }
        if refresh_count > 0 {
            reasons.push("refresh_noisy_or_rotting_index".to_owned());
        }
        if compact_count > 0 {
            reasons.push("compact_context_rot".to_owned());
        }
        if quarantine_count > 0 {
            reasons.push("quarantine_context_rot".to_owned());
        }
        if missing_clean_gist_count > 0 {
            reasons.push("missing_clean_gist".to_owned());
        }
        if dirty_clean_gist_count > 0 {
            reasons.push("dirty_clean_gist".to_owned());
        }
        if dirty_gist_count > 0 {
            reasons.push("dirty_gist".to_owned());
        }
        sort_dedup(&mut reasons);

        Self {
            ready_for_context_injection,
            total_records: report.total_records,
            blocker_count,
            warning_count,
            duplicate_record_count,
            refresh_count,
            compact_count,
            quarantine_count,
            missing_clean_gist_count,
            dirty_clean_gist_count,
            dirty_gist_count,
            context_rot_blocker_count,
            context_rot_blocker_reason_codes,
            reasons,
            details: quality_gate_detail_codes(report, plan),
        }
    }

    pub fn reason_codes(&self) -> Vec<String> {
        self.reasons.clone()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.details.clone()
    }

    pub fn checklist_detail(&self) -> String {
        format!(
            "quality_gate_blockers={} quality_gate_warnings={} quality_gate_context_rot_blockers={} quality_gate_reason_codes={} quality_gate_context_rot_blocker_reason_codes={} quality_gate_detail_codes={}",
            self.blocker_count,
            self.warning_count,
            self.context_rot_blocker_count,
            join_reason_codes(self.reason_codes()),
            join_reason_codes(self.context_rot_blocker_reason_codes.clone()),
            join_reason_codes(self.detail_codes())
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "experience_index_quality_gate ready_for_context_injection={} records={} blockers={} warnings={} duplicates={} refresh={} compact={} quarantine={} missing_clean_gist={} dirty_clean_gist={} dirty_gist={} context_rot_blockers={} reason_codes={} context_rot_blocker_reason_codes={} detail_codes={}",
            self.ready_for_context_injection,
            self.total_records,
            self.blocker_count,
            self.warning_count,
            self.duplicate_record_count,
            self.refresh_count,
            self.compact_count,
            self.quarantine_count,
            self.missing_clean_gist_count,
            self.dirty_clean_gist_count,
            self.dirty_gist_count,
            self.context_rot_blocker_count,
            join_reason_codes(self.reason_codes()),
            join_reason_codes(self.context_rot_blocker_reason_codes.clone()),
            join_reason_codes(self.detail_codes()),
        )
    }
}

pub trait ExperienceGovernance {
    fn deduplicate(&self, records: &[ExperienceEnvelope]) -> DeduplicationReport;
    fn assess(&self, records: &[ExperienceEnvelope]) -> GovernanceReport;
    fn assess_for_scope(
        &self,
        records: &[ExperienceEnvelope],
        _scope: &MemoryScope,
    ) -> GovernanceReport {
        self.assess(records)
    }
    fn rebuild_plan(&self, records: &[ExperienceEnvelope]) -> IndexRebuildPlan;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefaultExperienceGovernance {
    pub noise_threshold: f32,
    pub context_rot_threshold: f32,
    pub long_record_chars: usize,
}

impl Default for DefaultExperienceGovernance {
    fn default() -> Self {
        Self {
            noise_threshold: 0.42,
            context_rot_threshold: 0.50,
            long_record_chars: 2_400,
        }
    }
}

impl ExperienceGovernance for DefaultExperienceGovernance {
    fn deduplicate(&self, records: &[ExperienceEnvelope]) -> DeduplicationReport {
        let mut by_fingerprint = BTreeMap::<String, Vec<String>>::new();
        for record in records {
            by_fingerprint
                .entry(fingerprint(record))
                .or_default()
                .push(record.id.clone());
        }

        let groups = by_fingerprint
            .into_iter()
            .filter_map(|(fingerprint, ids)| {
                if ids.len() <= 1 {
                    return None;
                }
                let canonical_id = ids[0].clone();
                Some(DuplicateGroup {
                    canonical_id,
                    duplicate_ids: ids[1..].to_vec(),
                    fingerprint,
                })
            })
            .collect::<Vec<_>>();
        let duplicate_record_count = groups
            .iter()
            .map(|group| group.duplicate_ids.len())
            .sum::<usize>();
        DeduplicationReport {
            total_records: records.len(),
            duplicate_group_count: groups.len(),
            duplicate_record_count,
            groups,
        }
    }

    fn assess(&self, records: &[ExperienceEnvelope]) -> GovernanceReport {
        self.assess_internal(records, None)
    }

    fn assess_for_scope(
        &self,
        records: &[ExperienceEnvelope],
        scope: &MemoryScope,
    ) -> GovernanceReport {
        self.assess_internal(records, Some(scope))
    }

    fn rebuild_plan(&self, records: &[ExperienceEnvelope]) -> IndexRebuildPlan {
        let report = self.assess(records);
        self.plan_from_report(&report)
    }
}

impl DefaultExperienceGovernance {
    pub fn rebuild_plan_for_scope(
        &self,
        records: &[ExperienceEnvelope],
        scope: &MemoryScope,
    ) -> IndexRebuildPlan {
        let report = self.assess_for_scope(records, scope);
        self.plan_from_report(&report)
    }

    fn assess_internal(
        &self,
        records: &[ExperienceEnvelope],
        scope: Option<&MemoryScope>,
    ) -> GovernanceReport {
        let deduplication = self.deduplicate(records);
        let duplicate_ids = deduplication
            .groups
            .iter()
            .flat_map(|group| group.duplicate_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        let mut noisy_records = Vec::new();
        let mut context_rot_risks = Vec::new();

        for record in records {
            let noise = assess_noise(record, self.long_record_chars, scope);
            let rot = assess_context_rot(record, &noise, duplicate_ids.contains(&record.id));
            if noise.score >= self.noise_threshold || !noise.reasons.is_empty() {
                noisy_records.push(noise);
            }
            if rot.score >= self.context_rot_threshold || !rot.reasons.is_empty() {
                context_rot_risks.push(rot);
            }
        }

        noisy_records.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.experience_id.cmp(&right.experience_id))
        });
        context_rot_risks.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.experience_id.cmp(&right.experience_id))
        });

        GovernanceReport {
            total_records: records.len(),
            deduplication,
            noisy_records,
            context_rot_risks,
        }
    }

    fn plan_from_report(&self, report: &GovernanceReport) -> IndexRebuildPlan {
        let mut plan = IndexRebuildPlan {
            deduplicate_groups: report.deduplication.groups.clone(),
            ..IndexRebuildPlan::default()
        };

        for noise in &report.noisy_records {
            if noise.score >= 0.68 {
                plan.quarantine_candidate_ids
                    .push(noise.experience_id.clone());
            } else {
                plan.refresh_embedding_ids.push(noise.experience_id.clone());
            }
            match noise.gist_status {
                GistStatus::Missing => {
                    plan.missing_clean_gist_ids
                        .push(noise.experience_id.clone());
                    plan.dirty_gist_ids.push(noise.experience_id.clone());
                }
                GistStatus::Dirty => {
                    plan.dirty_clean_gist_ids.push(noise.experience_id.clone());
                    plan.dirty_gist_ids.push(noise.experience_id.clone());
                }
                GistStatus::Clean => {}
            }
        }

        for risk in &report.context_rot_risks {
            if risk
                .reasons
                .iter()
                .any(|reason| reason == "long_without_clean_gist")
            {
                plan.compact_ids.push(risk.experience_id.clone());
            }
            if risk.score >= self.context_rot_threshold {
                plan.refresh_embedding_ids.push(risk.experience_id.clone());
            }
        }

        sort_dedup(&mut plan.refresh_embedding_ids);
        sort_dedup(&mut plan.compact_ids);
        sort_dedup(&mut plan.quarantine_candidate_ids);
        sort_dedup(&mut plan.missing_clean_gist_ids);
        sort_dedup(&mut plan.dirty_clean_gist_ids);
        sort_dedup(&mut plan.dirty_gist_ids);

        if !plan.deduplicate_groups.is_empty() {
            plan.reasons
                .push("deduplicate_exact_fingerprints".to_owned());
        }
        if !plan.refresh_embedding_ids.is_empty() {
            plan.reasons
                .push("refresh_noisy_or_rotting_index".to_owned());
        }
        if !plan.compact_ids.is_empty() {
            plan.reasons
                .push("compact_long_context_without_gist".to_owned());
        }
        if !plan.quarantine_candidate_ids.is_empty() {
            plan.reasons
                .push("quarantine_high_noise_records".to_owned());
        }
        if !plan.dirty_gist_ids.is_empty() {
            plan.reasons
                .push("repair_missing_or_dirty_clean_gist".to_owned());
        }
        plan.rebuild_required = !plan.reasons.is_empty();
        plan
    }
}

fn assess_noise(
    record: &ExperienceEnvelope,
    long_record_chars: usize,
    current_scope: Option<&MemoryScope>,
) -> NoiseAssessment {
    let mut score: f32 = 0.0;
    let mut reasons = Vec::new();
    let gist_status = clean_gist_status(record);
    let combined = format!("{}\n{}", record.prompt, record.lesson);
    let transcript_shape =
        has_transcript_shape(&record.prompt) || has_transcript_shape(&record.lesson);
    let metadata_lesson_shape = has_metadata_lesson_shape(&record.lesson);
    let shell_markers = matched_shell_markers(&combined);
    let has_shell_markers = !shell_markers.is_empty();

    if transcript_shape {
        score += 0.30;
        reasons.push("transcript_shape".to_owned());
    }
    if metadata_lesson_shape {
        score += 0.32;
        reasons.push("metadata_lesson".to_owned());
    }
    if has_shell_markers {
        score += 0.38;
        reasons.extend(shell_markers);
    }
    if gist_status == GistStatus::Missing
        && (transcript_shape || metadata_lesson_shape || has_shell_markers)
    {
        score += 0.16;
        reasons.push("missing_clean_gist".to_owned());
    }
    if current_scope
        .and_then(|scope| scope.same_task_as(&record.scope))
        .is_some_and(|same_task| !same_task)
        && transcript_shape
        && has_shell_markers
    {
        score += 0.28;
        reasons.push("cross_task_transcript_pollution".to_owned());
    }
    if record.prompt.chars().count() + record.lesson.chars().count() > long_record_chars
        && gist_status != GistStatus::Clean
    {
        score += 0.18;
        reasons.push("long_without_clean_gist".to_owned());
    }
    if gist_status == GistStatus::Dirty {
        score += 0.14;
        reasons.push("dirty_clean_gist".to_owned());
    }
    if record.quality < 0.12 {
        score += 0.08;
        reasons.push("very_low_quality".to_owned());
    }

    NoiseAssessment {
        experience_id: record.id.clone(),
        score: clamp01(score),
        gist_status,
        reasons,
    }
}

fn assess_context_rot(
    record: &ExperienceEnvelope,
    noise: &NoiseAssessment,
    duplicate: bool,
) -> ContextRotRisk {
    let mut score = noise.score * 0.56;
    let mut reasons = Vec::new();
    let chars = record.prompt.chars().count() + record.lesson.chars().count();

    if chars > 2_400 {
        score += ((chars as f32 - 2_400.0) / 12_000.0).min(0.18);
        reasons.push("long_record".to_owned());
    }
    if noise.gist_status != GistStatus::Clean {
        score += 0.16;
        reasons.push("missing_clean_gist".to_owned());
    }
    if duplicate {
        score += 0.14;
        reasons.push("duplicate_experience".to_owned());
    }
    if noise
        .reasons
        .iter()
        .any(|reason| reason == "transcript_shape")
    {
        score += 0.12;
        reasons.push("transcript_anchor_risk".to_owned());
    }
    if noise
        .reasons
        .iter()
        .any(|reason| reason == "cross_task_transcript_pollution")
    {
        score += 0.18;
        reasons.push("cross_task_transcript_pollution".to_owned());
    }
    if noise
        .reasons
        .iter()
        .any(|reason| reason == "long_without_clean_gist")
    {
        reasons.push("long_without_clean_gist".to_owned());
    }

    ContextRotRisk {
        experience_id: record.id.clone(),
        score: clamp01(score),
        reasons,
    }
}

fn is_context_rot_blocker_reason(reason: &str) -> bool {
    matches!(
        reason,
        "cross_task_transcript_pollution" | "duplicate_experience" | "long_without_clean_gist"
    )
}

fn clean_gist_status(record: &ExperienceEnvelope) -> GistStatus {
    let Some(gist) = record.clean_gist.as_deref() else {
        return GistStatus::Missing;
    };
    if is_clean_gist(gist) {
        GistStatus::Clean
    } else {
        GistStatus::Dirty
    }
}

fn is_clean_gist(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= 420
        && !has_transcript_shape(trimmed)
        && !has_metadata_lesson_shape(trimmed)
        && trimmed
            .chars()
            .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
            .take(12)
            .count()
            >= 12
}

fn has_transcript_shape(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("conversation transcript:")
        || (value.contains("user:") && value.contains("assistant:"))
}

fn has_metadata_lesson_shape(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.starts_with("accepted_pattern ")
        || value.starts_with("rejected_pattern ")
        || ((value.contains("quality=") || value.contains("overlap="))
            && value.contains("max_severity="))
}

fn matched_shell_markers(value: &str) -> Vec<String> {
    const MARKERS: &[(&str, &str)] = &[
        ("ssh_connect_timeout", "ssh -o connecttimeout"),
        ("product_automation_token", "product_automation_token"),
        ("owner_bot_merge_token", "owner_bot_merge_token"),
        ("gitlab_local", "gitlab.local"),
        ("merge_requests", "merge requests"),
        ("bash_command", "bash command"),
        ("remote_script", "<<'remote'"),
    ];
    let lower = value.to_ascii_lowercase();
    MARKERS
        .iter()
        .filter_map(|(name, marker)| lower.contains(marker).then_some((*name).to_owned()))
        .collect()
}

fn dirty_self_improve_payload_reason_codes(value: &str) -> Vec<String> {
    let mut reason_codes = Vec::new();
    let lower = value.to_ascii_lowercase();

    if has_transcript_shape(value) {
        reason_codes.push("payload_transcript_shape".to_owned());
    }
    if has_metadata_lesson_shape(value) {
        reason_codes.push("payload_metadata_lesson".to_owned());
    }
    for marker in matched_shell_markers(value) {
        reason_codes.push(format!("payload_{marker}"));
    }
    if lower.contains("old_window_payload")
        || lower.contains("polluted_context_payload")
        || lower.contains("legacy_chat")
        || lower.contains("old thread")
        || lower.contains("old window")
    {
        reason_codes.push("payload_old_window_reference".to_owned());
    }
    if lower.contains(".ndkv")
        || lower.contains("live_write")
        || lower.contains("store_mutations")
        || lower.contains("live_store_targeted")
    {
        reason_codes.push("payload_live_store_write_request".to_owned());
    }
    if lower.contains("chat-stream") {
        reason_codes.push("payload_chat_stream_reference".to_owned());
    }

    sort_dedup(&mut reason_codes);
    reason_codes
}

fn clean_evidence_ids(values: &[String]) -> Vec<String> {
    let mut ids = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| is_clean_evidence_id(value))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    sort_dedup(&mut ids);
    ids
}

fn clean_next_round_marker_codes(values: &[String]) -> Vec<String> {
    let mut marker_codes = values
        .iter()
        .map(|value| stable_detail_part(value.trim()))
        .filter(|value| value != "unknown")
        .collect::<Vec<_>>();
    sort_dedup(&mut marker_codes);
    marker_codes
}

fn round_id_evidence_summary(evidence: &SelfImproveRoundIdEvidence) -> String {
    format!(
        "source_schema:{}:active_round:{}:ledger_latest_round:{}:latest_done_round:{}",
        stable_detail_part(&evidence.source_schema),
        evidence
            .active_round
            .map(|round| round.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        evidence
            .ledger_latest_round
            .map(|round| round.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        evidence
            .latest_done_round
            .map(|round| round.to_string())
            .unwrap_or_else(|| "none".to_owned()),
    )
}

fn is_clean_evidence_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'))
        && dirty_self_improve_payload_reason_codes(value).is_empty()
        && !has_transcript_shape(value)
        && !has_metadata_lesson_shape(value)
}

fn payload_declares_repair_required(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase().replace('-', "_");
    [
        "repair_required=true",
        "repair_required: true",
        "\"repair_required\":true",
        "\"repair_required\": true",
        "admission_status=repair_required",
        "helper_status=repair_required",
        "lifecycle=repair_required",
        "proposal_status=repair_required",
        "state=repair_required",
        "status=repair_required",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn fingerprint(record: &ExperienceEnvelope) -> String {
    normalize_fingerprint_text(&format!("{}\n{}", record.prompt, record.lesson))
}

fn normalize_fingerprint_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_ascii_lowercase()
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn join_reason_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn quality_gate_detail_codes(report: &GovernanceReport, plan: &IndexRebuildPlan) -> Vec<String> {
    let mut codes = BTreeSet::new();

    for group in &report.deduplication.groups {
        let canonical_id = stable_detail_part(&group.canonical_id);
        for duplicate_id in &group.duplicate_ids {
            codes.insert(format!(
                "blocker:duplicate:{canonical_id}:{}",
                stable_detail_part(duplicate_id)
            ));
        }
    }
    for id in &plan.compact_ids {
        codes.insert(format!("blocker:compact:{}", stable_detail_part(id)));
    }
    for id in &plan.quarantine_candidate_ids {
        codes.insert(format!("blocker:quarantine:{}", stable_detail_part(id)));
    }
    for id in &plan.refresh_embedding_ids {
        codes.insert(format!("warning:refresh:{}", stable_detail_part(id)));
    }
    for id in &plan.missing_clean_gist_ids {
        codes.insert(format!(
            "warning:missing_clean_gist:{}",
            stable_detail_part(id)
        ));
    }
    for id in &plan.dirty_clean_gist_ids {
        codes.insert(format!(
            "warning:dirty_clean_gist:{}",
            stable_detail_part(id)
        ));
    }
    for id in &plan.dirty_gist_ids {
        codes.insert(format!("warning:dirty_gist:{}", stable_detail_part(id)));
    }

    codes.into_iter().collect()
}

fn stable_detail_part(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_separator = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            out.push('_');
            last_was_separator = true;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "unknown".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r34_ready(proposal: SelfImproveLearningProposal) -> SelfImproveLearningProposal {
        proposal
            .with_live_status_report_gate_evidence(true, true)
            .with_validation_test_gate_evidence(true, true)
            .with_helper_stage_contract_complete(true)
            .with_evidence_ids(vec![
                "live-status-round-364".to_owned(),
                "report-gate-round-364".to_owned(),
                "test-gate-pass-round-364".to_owned(),
            ])
            .with_clean_source_windows()
            .with_next_round_decision_evidence(
                SelfImproveNextRoundDecision::SafeToContinueAfterCurrentRound,
                true,
                false,
            )
    }

    #[test]
    fn duplicate_experiences_are_grouped_by_normalized_content() {
        let records = vec![
            ExperienceEnvelope::new("1", "Prompt A", "Lesson A"),
            ExperienceEnvelope::new("2", " prompt   a ", " lesson a "),
            ExperienceEnvelope::new("3", "Prompt B", "Lesson B"),
        ];
        let report = DefaultExperienceGovernance::default().deduplicate(&records);
        assert_eq!(report.duplicate_group_count, 1);
        assert_eq!(report.duplicate_record_count, 1);
        assert_eq!(report.groups[0].canonical_id, "1");
        assert_eq!(report.groups[0].duplicate_ids, vec!["2".to_owned()]);
    }

    #[test]
    fn validated_self_improve_proposal_becomes_isolated_learning_envelope() {
        let plan = admit_self_improve_learning_candidate(r34_ready(
            SelfImproveLearningProposal::new("R26-C", 324)
                .with_source(SelfImproveProposalSource::CleanRoomWorker)
                .with_validation_passed(true)
                .with_feedback_applied(true)
                .with_clean_gist(
                    "Validated memory learning candidates must stay isolated until admission.",
                )
                .with_payload("proposal_payload_id=abc123 clean_summary_ref=round324")
                .with_scope(MemoryScope::for_task("memory-admission"))
                .with_tags(vec![
                    "memory".to_owned(),
                    "admission".to_owned(),
                    "memory".to_owned(),
                ]),
        ));

        assert_eq!(plan.decision, SelfImproveAdmissionDecision::AcceptEnvelope);
        assert_eq!(
            plan.repair_state,
            SelfImproveProposalRepairState::NotRequired
        );
        assert_eq!(plan.write_mode, SelfImproveAdmissionWriteMode::ReadOnly);
        assert!(plan.read_only_or_isolated_contract_holds());
        assert!(plan.memory_candidate_ready());
        assert!(!plan.live_store_mutation_allowed());
        assert!(!plan.ndkv_write_allowed());
        assert_eq!(plan.reason_codes, Vec::<String>::new());

        let envelope = plan.accepted_envelope().unwrap();
        assert_eq!(envelope.id, "self_improve_round_324_r26_c");
        assert_eq!(
            envelope.clean_gist.as_deref(),
            Some("Validated memory learning candidates must stay isolated until admission.")
        );
        assert_eq!(
            envelope.lesson,
            "Validated memory learning candidates must stay isolated until admission."
        );
        assert_eq!(envelope.scope.task_id.as_deref(), Some("memory-admission"));
        assert_eq!(
            envelope.tags,
            vec![
                "admission".to_owned(),
                "learning-candidate".to_owned(),
                "memory".to_owned(),
                "self-improve".to_owned(),
                "source-round:324".to_owned(),
            ]
        );
        assert!(plan.summary_line().contains(
            "repair_state=not_required decision=accept_envelope write_mode=read_only memory_candidate_ready=true"
        ));
        assert!(plan.summary_line().contains(
            "next_round_decision=safe-to-continue-after-current-round next_round_live_status_synced=true next_round_current_round_active=false"
        ));
    }

    #[test]
    fn self_improve_admission_refuses_unvalidated_or_missing_gist_inputs() {
        let unvalidated = admit_self_improve_learning_candidate(r34_ready(
            SelfImproveLearningProposal::new("R26-C", 324)
                .with_feedback_applied(true)
                .with_clean_gist(
                    "Validated memory learning candidates must stay isolated until admission.",
                )
                .with_scope(MemoryScope::for_task("memory-admission"))
                .with_tags(vec!["memory".to_owned()]),
        ));
        let missing_gist = admit_self_improve_learning_candidate(r34_ready(
            SelfImproveLearningProposal::new("R26-C", 324)
                .with_validation_passed(true)
                .with_feedback_applied(true)
                .with_scope(MemoryScope::for_task("memory-admission"))
                .with_tags(vec!["memory".to_owned()]),
        ));

        assert_eq!(unvalidated.decision, SelfImproveAdmissionDecision::Reject);
        assert_eq!(
            unvalidated.write_mode,
            SelfImproveAdmissionWriteMode::ReadOnly
        );
        assert_eq!(unvalidated.accepted_envelope(), None);
        assert!(
            unvalidated
                .reason_codes
                .contains(&"validation_not_passed".to_owned())
        );
        assert!(!unvalidated.ndkv_write_allowed());

        assert_eq!(missing_gist.decision, SelfImproveAdmissionDecision::Reject);
        assert_eq!(
            missing_gist.write_mode,
            SelfImproveAdmissionWriteMode::ReadOnly
        );
        assert_eq!(missing_gist.accepted_envelope(), None);
        assert!(
            missing_gist
                .reason_codes
                .contains(&"missing_clean_gist".to_owned())
        );
    }

    #[test]
    fn self_improve_admission_requires_healthy_gate_evidence_and_clean_windows() {
        let ready = || {
            r34_ready(
                SelfImproveLearningProposal::new("R35-B", 365)
                    .with_source(SelfImproveProposalSource::CleanRoomWorker)
                    .with_validation_passed(true)
                    .with_feedback_applied(true)
                    .with_clean_gist(
                        "Healthy gate evidence is required before learning candidates can appear.",
                    )
                    .with_payload("proposal_payload_id=r35b clean_summary_ref=round365")
                    .with_scope(MemoryScope::for_task("memory-admission"))
                    .with_tags(vec!["memory".to_owned(), "self-improve".to_owned()]),
            )
        };

        let unhealthy_live_status = admit_self_improve_learning_candidate(
            ready().with_live_status_report_gate_evidence(false, true),
        );
        let failed_report_gate = admit_self_improve_learning_candidate(
            ready().with_live_status_report_gate_evidence(true, false),
        );
        let failed_validation_gate = admit_self_improve_learning_candidate(
            ready().with_validation_test_gate_evidence(false, true),
        );
        let failed_test_gate = admit_self_improve_learning_candidate(
            ready().with_validation_test_gate_evidence(true, false),
        );
        let incomplete_helper_contract = admit_self_improve_learning_candidate(
            ready().with_helper_stage_contract_complete(false),
        );
        let missing_evidence =
            admit_self_improve_learning_candidate(ready().with_evidence_ids(Vec::new()));
        let dirty_evidence =
            admit_self_improve_learning_candidate(ready().with_evidence_ids(vec![
                "live-status-round-365".to_owned(),
                "Conversation Transcript: User: stale Assistant: stale".to_owned(),
            ]));
        let polluted_window =
            admit_self_improve_learning_candidate(ready().with_source_window_status(true, false));
        let actionable_window =
            admit_self_improve_learning_candidate(ready().with_source_window_status(false, true));

        for (plan, reason) in [
            (&unhealthy_live_status, "live_status_not_healthy"),
            (&failed_report_gate, "report_gate_not_passed"),
            (&failed_validation_gate, "validation_gate_not_passed"),
            (&failed_test_gate, "test_gate_not_passed"),
            (
                &incomplete_helper_contract,
                "helper_stage_contract_incomplete",
            ),
            (&missing_evidence, "missing_evidence_ids"),
            (&dirty_evidence, "dirty_evidence_id"),
            (&polluted_window, "source_window_polluted"),
            (&actionable_window, "source_window_actionable"),
        ] {
            assert!(plan.reason_codes.contains(&reason.to_owned()));
            assert_eq!(plan.accepted_envelope(), None);
            assert!(!plan.memory_candidate_ready());
            assert_eq!(plan.write_mode, SelfImproveAdmissionWriteMode::ReadOnly);
            assert!(!plan.live_store_mutation_allowed());
            assert!(!plan.ndkv_write_allowed());
        }

        for rejected in [
            &unhealthy_live_status,
            &failed_report_gate,
            &failed_validation_gate,
            &failed_test_gate,
            &incomplete_helper_contract,
            &missing_evidence,
            &polluted_window,
            &actionable_window,
        ] {
            assert_eq!(rejected.decision, SelfImproveAdmissionDecision::Reject);
        }
        assert_eq!(
            dirty_evidence.decision,
            SelfImproveAdmissionDecision::QuarantineCandidate
        );
    }

    #[test]
    fn self_improve_next_round_wait_state_is_report_only() {
        let plan = admit_self_improve_learning_candidate(
            r34_ready(
                SelfImproveLearningProposal::new("R37-D", 369)
                    .with_source(SelfImproveProposalSource::CleanRoomWorker)
                    .with_validation_passed(true)
                    .with_feedback_applied(true)
                    .with_clean_gist(
                        "Wait-state evidence is visible but must not admit memory candidates.",
                    )
                    .with_payload("proposal_payload_id=r37d clean_summary_ref=round369")
                    .with_scope(MemoryScope::for_task("memory-admission"))
                    .with_tags(vec!["memory".to_owned(), "next-round".to_owned()]),
            )
            .with_next_round_decision_evidence(
                SelfImproveNextRoundDecision::SafeToWaitCurrentRoundActive,
                true,
                true,
            ),
        );

        assert_eq!(plan.decision, SelfImproveAdmissionDecision::Reject);
        assert!(plan.accepted_envelope().is_none());
        assert!(!plan.memory_candidate_ready());
        assert_eq!(plan.write_mode, SelfImproveAdmissionWriteMode::ReadOnly);
        assert!(!plan.live_store_mutation_allowed());
        assert!(!plan.ndkv_write_allowed());
        assert!(
            plan.reason_codes
                .contains(&"next_round_wait_current_round_active".to_owned())
        );
        assert!(plan.summary_line().contains(
            "next_round_decision=safe-to-wait-current-round-active next_round_live_status_synced=true next_round_current_round_active=true"
        ));
        assert!(
            plan.summary_line()
                .contains("memory_candidate_ready=false envelope_ready=false")
        );
    }

    #[test]
    fn self_improve_next_round_continue_requires_synced_evidence() {
        let ready = || {
            r34_ready(
                SelfImproveLearningProposal::new("R37-E", 370)
                    .with_source(SelfImproveProposalSource::CleanRoomWorker)
                    .with_validation_passed(true)
                    .with_feedback_applied(true)
                    .with_clean_gist(
                        "Synced continue evidence may expose a candidate without live writes.",
                    )
                    .with_payload("proposal_payload_id=r37e clean_summary_ref=round370")
                    .with_scope(MemoryScope::for_task("memory-admission"))
                    .with_tags(vec!["memory".to_owned(), "next-round".to_owned()]),
            )
        };
        let synced_continue = admit_self_improve_learning_candidate(ready());
        let unsynced_continue =
            admit_self_improve_learning_candidate(ready().with_next_round_decision_evidence(
                SelfImproveNextRoundDecision::SafeToContinueAfterCurrentRound,
                false,
                false,
            ));

        assert_eq!(
            synced_continue.decision,
            SelfImproveAdmissionDecision::AcceptEnvelope
        );
        assert!(synced_continue.memory_candidate_ready());
        assert!(synced_continue.accepted_envelope().is_some());
        assert!(!synced_continue.live_store_mutation_allowed());
        assert!(!synced_continue.ndkv_write_allowed());
        assert_eq!(
            synced_continue.next_round_decision,
            SelfImproveNextRoundDecision::SafeToContinueAfterCurrentRound
        );

        assert_eq!(
            unsynced_continue.decision,
            SelfImproveAdmissionDecision::Reject
        );
        assert!(unsynced_continue.accepted_envelope().is_none());
        assert!(!unsynced_continue.memory_candidate_ready());
        assert!(
            unsynced_continue
                .reason_codes
                .contains(&"next_round_evidence_not_synced".to_owned())
        );
        assert!(!unsynced_continue.ndkv_write_allowed());
    }

    #[test]
    fn self_improve_admission_exposes_round_id_evidence_without_live_writes() {
        let ready = || {
            r34_ready(
                SelfImproveLearningProposal::new("R44-B", 377)
                    .with_source(SelfImproveProposalSource::CleanRoomWorker)
                    .with_validation_passed(true)
                    .with_feedback_applied(true)
                    .with_clean_gist(
                        "Daemon round-id evidence can be learned only as read-only provenance.",
                    )
                    .with_payload("proposal_payload_id=r44b clean_summary_ref=round377")
                    .with_scope(MemoryScope::for_task("memory-admission"))
                    .with_tags(vec!["memory".to_owned(), "next-round".to_owned()]),
            )
            .with_evidence_ids(vec![
                "next_round_downstream_status_consumers_v1:round-377".to_owned(),
                "daemon_round_transition_status_v1:377".to_owned(),
            ])
            .with_next_round_round_id_evidence(Some(
                SelfImproveRoundIdEvidence::daemon_transition(Some(378), Some(377), Some(377)),
            ))
        };
        let admitted = admit_self_improve_learning_candidate(ready());
        let untrusted_source = admit_self_improve_learning_candidate(
            ready().with_next_round_round_id_evidence(Some(SelfImproveRoundIdEvidence {
                source_schema: "old_window_payload".to_owned(),
                active_round: Some(378),
                ledger_latest_round: Some(377),
                latest_done_round: Some(377),
            })),
        );

        assert_eq!(
            admitted.decision,
            SelfImproveAdmissionDecision::AcceptEnvelope
        );
        assert!(admitted.memory_candidate_ready());
        assert!(admitted.accepted_envelope().is_some());
        assert_eq!(
            admitted.next_round_round_id_evidence,
            Some(SelfImproveRoundIdEvidence {
                source_schema: "daemon_round_transition_status_v1".to_owned(),
                active_round: Some(378),
                ledger_latest_round: Some(377),
                latest_done_round: Some(377),
            })
        );
        assert_eq!(admitted.write_mode, SelfImproveAdmissionWriteMode::ReadOnly);
        assert!(!admitted.live_store_mutation_allowed());
        assert!(!admitted.ndkv_write_allowed());
        assert!(admitted.summary_line().contains(
            "next_round_round_id_evidence=source_schema:daemon_round_transition_status_v1:active_round:378:ledger_latest_round:377:latest_done_round:377"
        ));

        assert_eq!(
            untrusted_source.decision,
            SelfImproveAdmissionDecision::QuarantineCandidate
        );
        assert!(untrusted_source.accepted_envelope().is_none());
        assert!(!untrusted_source.memory_candidate_ready());
        assert!(
            untrusted_source
                .reason_codes
                .contains(&"next_round_round_id_evidence_source_untrusted".to_owned())
        );
        assert!(!untrusted_source.live_store_mutation_allowed());
        assert!(!untrusted_source.ndkv_write_allowed());
    }

    #[test]
    fn self_improve_next_round_attention_and_dirty_markers_do_not_admit_candidates() {
        let ready = || {
            r34_ready(
                SelfImproveLearningProposal::new("R37-F", 371)
                    .with_source(SelfImproveProposalSource::CleanRoomWorker)
                    .with_validation_passed(true)
                    .with_feedback_applied(true)
                    .with_clean_gist(
                        "Operator attention and side-effect markers stay outside memory admission.",
                    )
                    .with_payload("proposal_payload_id=r37f clean_summary_ref=round371")
                    .with_scope(MemoryScope::for_task("memory-admission"))
                    .with_tags(vec!["memory".to_owned(), "next-round".to_owned()]),
            )
        };
        let operator_attention =
            admit_self_improve_learning_candidate(ready().with_next_round_decision_evidence(
                SelfImproveNextRoundDecision::OperatorAttentionBlocked,
                true,
                false,
            ));
        let side_effect_marker = admit_self_improve_learning_candidate(
            ready().with_next_round_side_effect_markers(vec![
                "store_mutations=true".to_owned(),
                "ndkv_write_allowed=true".to_owned(),
            ]),
        );
        let raw_window_marker = admit_self_improve_learning_candidate(
            ready().with_next_round_raw_window_markers(vec![
                "old_window_payload thread=stale".to_owned(),
                "raw_window_source=true".to_owned(),
            ]),
        );

        assert_eq!(
            operator_attention.decision,
            SelfImproveAdmissionDecision::Reject
        );
        assert!(
            operator_attention
                .reason_codes
                .contains(&"next_round_operator_attention".to_owned())
        );
        assert!(operator_attention.accepted_envelope().is_none());
        assert!(!operator_attention.live_store_mutation_allowed());
        assert!(!operator_attention.ndkv_write_allowed());

        for (plan, reason) in [
            (&side_effect_marker, "next_round_side_effect_marker"),
            (&raw_window_marker, "next_round_raw_window_marker"),
        ] {
            assert_eq!(
                plan.decision,
                SelfImproveAdmissionDecision::QuarantineCandidate
            );
            assert!(plan.reason_codes.contains(&reason.to_owned()));
            assert!(plan.accepted_envelope().is_none());
            assert!(!plan.memory_candidate_ready());
            assert!(!plan.live_store_mutation_allowed());
            assert!(!plan.ndkv_write_allowed());
        }
        assert_eq!(
            side_effect_marker.next_round_side_effect_marker_codes,
            vec![
                "ndkv_write_allowed_true".to_owned(),
                "store_mutations_true".to_owned(),
            ]
        );
        assert_eq!(
            raw_window_marker.next_round_raw_window_marker_codes,
            vec![
                "old_window_payload_thread_stale".to_owned(),
                "raw_window_source_true".to_owned(),
            ]
        );
        assert!(side_effect_marker.summary_line().contains(
            "next_round_side_effect_markers=ndkv_write_allowed_true|store_mutations_true"
        ));
        assert!(raw_window_marker.summary_line().contains(
            "next_round_raw_window_markers=old_window_payload_thread_stale|raw_window_source_true"
        ));
    }

    #[test]
    fn repair_required_self_improve_proposal_is_quarantined_until_repaired() {
        let blocked = admit_self_improve_learning_candidate(r34_ready(
            SelfImproveLearningProposal::new("R28-F", 328)
                .with_source(SelfImproveProposalSource::CleanRoomWorker)
                .with_repair_state(SelfImproveProposalRepairState::RepairRequired)
                .with_validation_passed(true)
                .with_feedback_applied(true)
                .with_clean_gist(
                    "Repair helper proposals stay isolated until their patch is verified.",
                )
                .with_payload("proposal_payload_id=r28f clean_summary_ref=round328")
                .with_scope(MemoryScope::for_task("memory-repair-admission"))
                .with_tags(vec!["memory".to_owned(), "repair".to_owned()]),
        ));
        let marker_blocked = admit_self_improve_learning_candidate(r34_ready(
            SelfImproveLearningProposal::new("R28-F", 328)
                .with_source(SelfImproveProposalSource::CleanRoomWorker)
                .with_validation_passed(true)
                .with_feedback_applied(true)
                .with_clean_gist(
                    "Repair helper proposals stay isolated until their patch is verified.",
                )
                .with_payload("proposal_status=repair-required clean_summary_ref=round328")
                .with_scope(MemoryScope::for_task("memory-repair-admission"))
                .with_tags(vec!["memory".to_owned(), "repair".to_owned()]),
        ));
        let repaired = admit_self_improve_learning_candidate(r34_ready(
            SelfImproveLearningProposal::new("R28-F", 328)
                .with_source(SelfImproveProposalSource::CleanRoomWorker)
                .with_repair_state(SelfImproveProposalRepairState::RepairApplied)
                .with_validation_passed(true)
                .with_feedback_applied(true)
                .with_clean_gist(
                    "Repair helper proposals stay isolated until their patch is verified.",
                )
                .with_payload("proposal_payload_id=r28f clean_summary_ref=round328")
                .with_scope(MemoryScope::for_task("memory-repair-admission"))
                .with_tags(vec!["memory".to_owned(), "repair".to_owned()]),
        ));
        let repaired_but_unvalidated = admit_self_improve_learning_candidate(r34_ready(
            SelfImproveLearningProposal::new("R28-F", 328)
                .with_source(SelfImproveProposalSource::CleanRoomWorker)
                .with_repair_state(SelfImproveProposalRepairState::RepairApplied)
                .with_feedback_applied(true)
                .with_clean_gist(
                    "Repair helper proposals stay isolated until their patch is verified.",
                )
                .with_scope(MemoryScope::for_task("memory-repair-admission"))
                .with_tags(vec!["memory".to_owned(), "repair".to_owned()]),
        ));

        assert_eq!(
            blocked.decision,
            SelfImproveAdmissionDecision::QuarantineCandidate
        );
        assert_eq!(blocked.write_mode, SelfImproveAdmissionWriteMode::ReadOnly);
        assert!(blocked.accepted_envelope().is_none());
        assert!(!blocked.live_store_mutation_allowed());
        assert!(!blocked.ndkv_write_allowed());
        assert!(
            blocked
                .reason_codes
                .contains(&"repair_required_not_applied".to_owned())
        );
        assert!(
            blocked
                .summary_line()
                .contains("repair_state=repair_required decision=quarantine_candidate")
        );

        assert_eq!(
            marker_blocked.decision,
            SelfImproveAdmissionDecision::QuarantineCandidate
        );
        assert!(
            marker_blocked
                .reason_codes
                .contains(&"payload_repair_required_marker".to_owned())
        );
        assert!(marker_blocked.accepted_envelope().is_none());
        assert!(!marker_blocked.live_store_mutation_allowed());
        assert!(!marker_blocked.ndkv_write_allowed());

        assert_eq!(
            repaired.decision,
            SelfImproveAdmissionDecision::AcceptEnvelope
        );
        assert_eq!(
            repaired.repair_state,
            SelfImproveProposalRepairState::RepairApplied
        );
        assert_eq!(repaired.reason_codes, Vec::<String>::new());
        assert!(repaired.memory_candidate_ready());
        assert!(repaired.accepted_envelope().is_some());
        assert!(!repaired.live_store_mutation_allowed());
        assert!(!repaired.ndkv_write_allowed());

        assert_eq!(
            repaired_but_unvalidated.decision,
            SelfImproveAdmissionDecision::Reject
        );
        assert!(
            repaired_but_unvalidated
                .reason_codes
                .contains(&"validation_not_passed".to_owned())
        );
        assert!(repaired_but_unvalidated.accepted_envelope().is_none());
        assert!(!repaired_but_unvalidated.ndkv_write_allowed());
    }

    #[test]
    fn self_improve_admission_quarantines_dirty_payload_and_old_window_source() {
        let forbidden_payload = "SECRET_OLD_WINDOW_PAYLOAD";
        let dirty_payload =
            admit_self_improve_learning_candidate(r34_ready(SelfImproveLearningProposal::new("R26-C", 324)
                .with_source(SelfImproveProposalSource::CleanRoomWorker)
                .with_validation_passed(true)
                .with_feedback_applied(true)
                .with_clean_gist(
                    "Validated memory learning candidates must stay isolated until admission.",
                )
                .with_payload(format!(
                    "old_window_payload Conversation Transcript:\nUser: {forbidden_payload} live_write prod.ndkv\nAssistant: ok"
                ))
                .with_scope(MemoryScope::for_task("memory-admission"))
                .with_tags(vec!["memory".to_owned()])));
        let old_window = admit_self_improve_learning_candidate(r34_ready(
            SelfImproveLearningProposal::new("R26-C", 324)
                .with_source(SelfImproveProposalSource::OldWindow)
                .with_validation_passed(true)
                .with_feedback_applied(true)
                .with_clean_gist(
                    "Validated memory learning candidates must stay isolated until admission.",
                )
                .with_scope(MemoryScope::for_task("memory-admission"))
                .with_tags(vec!["memory".to_owned()]),
        ));
        let pending_feedback = admit_self_improve_learning_candidate(r34_ready(
            SelfImproveLearningProposal::new("R26-C", 324)
                .with_source(SelfImproveProposalSource::ReportOnlyContract)
                .with_validation_passed(true)
                .with_feedback_applied(false)
                .with_clean_gist(
                    "Validated memory learning candidates must stay isolated until admission.",
                )
                .with_scope(MemoryScope::for_task("memory-admission"))
                .with_tags(vec!["memory".to_owned()]),
        ));

        assert_eq!(
            dirty_payload.decision,
            SelfImproveAdmissionDecision::QuarantineCandidate
        );
        assert_eq!(
            dirty_payload.write_mode,
            SelfImproveAdmissionWriteMode::ReadOnly
        );
        assert!(dirty_payload.accepted_envelope().is_none());
        assert!(
            dirty_payload
                .reason_codes
                .contains(&"payload_transcript_shape".to_owned())
        );
        assert!(
            dirty_payload
                .reason_codes
                .contains(&"payload_live_store_write_request".to_owned())
        );
        assert!(
            dirty_payload
                .reason_codes
                .contains(&"payload_old_window_reference".to_owned())
        );
        assert!(!dirty_payload.summary_line().contains(forbidden_payload));
        assert!(!dirty_payload.summary_line().contains("prod.ndkv"));
        assert!(
            dirty_payload
                .detail_codes
                .iter()
                .all(|code| !code.contains(forbidden_payload))
        );
        assert!(!dirty_payload.ndkv_write_allowed());

        assert_eq!(
            old_window.decision,
            SelfImproveAdmissionDecision::QuarantineCandidate
        );
        assert!(old_window.accepted_envelope().is_none());
        assert!(
            old_window
                .reason_codes
                .contains(&"old_window_source".to_owned())
        );
        assert!(!old_window.live_store_mutation_allowed());

        assert_eq!(
            pending_feedback.decision,
            SelfImproveAdmissionDecision::QuarantineCandidate
        );
        assert!(
            pending_feedback
                .reason_codes
                .contains(&"feedback_not_applied".to_owned())
        );
        assert!(pending_feedback.accepted_envelope().is_none());
    }

    #[test]
    fn noisy_transcript_records_are_identified() {
        let mut record = ExperienceEnvelope::new(
            "noise",
            "Conversation Transcript:\nUser: run ssh -o ConnectTimeout=1\nAssistant: ok",
            "accepted_pattern quality=0.1 max_severity=critical",
        );
        record.clean_gist =
            Some("Conversation Transcript: User: stale Assistant: stale".to_owned());
        let report = DefaultExperienceGovernance::default().assess(&[record]);
        assert_eq!(report.noisy_records.len(), 1);
        assert_eq!(
            report.summary_line(),
            "memory_governance records=1 duplicate_groups=0 duplicate_records=0 noisy=1 context_rot=1 reason_codes=dirty_clean_gist|metadata_lesson|missing_clean_gist|ssh_connect_timeout|transcript_anchor_risk|transcript_shape detail_codes=context_rot:noise:missing_clean_gist|context_rot:noise:transcript_anchor_risk|noise:noise:dirty_clean_gist|noise:noise:gist_dirty|noise:noise:metadata_lesson|noise:noise:ssh_connect_timeout|noise:noise:transcript_shape"
        );
        assert_eq!(
            report.reason_codes(),
            vec![
                "dirty_clean_gist".to_owned(),
                "metadata_lesson".to_owned(),
                "missing_clean_gist".to_owned(),
                "ssh_connect_timeout".to_owned(),
                "transcript_anchor_risk".to_owned(),
                "transcript_shape".to_owned(),
            ]
        );
        assert!(
            report
                .detail_codes()
                .contains(&"noise:noise:gist_dirty".to_owned())
        );
        assert!(
            report
                .detail_codes()
                .contains(&"context_rot:noise:transcript_anchor_risk".to_owned())
        );
        let noise = &report.noisy_records[0];
        assert!(noise.score >= 0.68);
        assert!(
            noise
                .reasons
                .iter()
                .any(|reason| reason == "transcript_shape")
        );
        assert!(
            noise
                .reasons
                .iter()
                .any(|reason| reason == "metadata_lesson")
        );
        assert_eq!(noise.gist_status, GistStatus::Dirty);
    }

    #[test]
    fn noise_assessment_exposes_clean_gist_evidence_without_payloads() {
        let forbidden = "DIRTY_GIST_PAYLOAD_SECRET_DO_NOT_LOG";
        let noise = NoiseAssessment {
            experience_id: "Dirty Gist/42".to_owned(),
            score: 0.7312,
            gist_status: GistStatus::Dirty,
            reasons: vec![
                "dirty_clean_gist".to_owned(),
                "metadata_lesson".to_owned(),
                "dirty_clean_gist".to_owned(),
            ],
        };

        assert_eq!(
            noise.reason_codes(),
            vec!["dirty_clean_gist".to_owned(), "metadata_lesson".to_owned()]
        );
        assert_eq!(
            noise.detail_codes(),
            vec![
                "noise:dirty_gist_42:dirty_clean_gist".to_owned(),
                "noise:dirty_gist_42:gist_dirty".to_owned(),
                "noise:dirty_gist_42:metadata_lesson".to_owned(),
            ]
        );
        assert_eq!(
            noise.summary_line(),
            "noise_assessment experience=dirty_gist_42 score=0.731 gist_status=dirty reasons=2 reason_codes=dirty_clean_gist|metadata_lesson detail_codes=noise:dirty_gist_42:dirty_clean_gist|noise:dirty_gist_42:gist_dirty|noise:dirty_gist_42:metadata_lesson"
        );

        let record = ExperienceEnvelope::new(
            "derived",
            "A normal Rust runtime prompt",
            "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
        )
        .with_clean_gist(format!(
            "Conversation Transcript: User: {forbidden} Assistant: stale"
        ));
        let report = DefaultExperienceGovernance::default().assess(&[record]);
        let derived = &report.noisy_records[0];

        assert_eq!(derived.gist_status, GistStatus::Dirty);
        assert!(
            derived
                .reason_codes()
                .contains(&"dirty_clean_gist".to_owned())
        );
        assert!(!derived.summary_line().contains(forbidden));
        assert!(
            !derived
                .detail_codes()
                .iter()
                .any(|code| code.contains(forbidden))
        );
    }

    #[test]
    fn context_rot_risk_exposes_stable_payload_safe_evidence() {
        let forbidden = "PROMPT_PAYLOAD_SECRET_DO_NOT_LOG";
        let risk = ContextRotRisk {
            experience_id: "Rot Case/42".to_owned(),
            score: 0.8126,
            reasons: vec![
                "transcript_anchor_risk".to_owned(),
                "missing_clean_gist".to_owned(),
                "cross_task_transcript_pollution".to_owned(),
                "missing_clean_gist".to_owned(),
            ],
        };

        assert_eq!(
            risk.reason_codes(),
            vec![
                "cross_task_transcript_pollution".to_owned(),
                "missing_clean_gist".to_owned(),
                "transcript_anchor_risk".to_owned(),
            ]
        );
        assert_eq!(
            risk.detail_codes(),
            vec![
                "context_rot:rot_case_42:cross_task_transcript_pollution".to_owned(),
                "context_rot:rot_case_42:missing_clean_gist".to_owned(),
                "context_rot:rot_case_42:transcript_anchor_risk".to_owned(),
            ]
        );
        assert_eq!(
            risk.summary_line(),
            "context_rot_risk experience=rot_case_42 score=0.813 reasons=3 reason_codes=cross_task_transcript_pollution|missing_clean_gist|transcript_anchor_risk detail_codes=context_rot:rot_case_42:cross_task_transcript_pollution|context_rot:rot_case_42:missing_clean_gist|context_rot:rot_case_42:transcript_anchor_risk"
        );
        assert!(!risk.summary_line().contains(forbidden));
        assert!(
            !risk
                .detail_codes()
                .iter()
                .any(|code| code.contains(forbidden))
        );

        let report = DefaultExperienceGovernance::default().assess(&[ExperienceEnvelope::new(
            "derived",
            format!("Conversation Transcript:\nUser: {forbidden}\nAssistant: ok"),
            "accepted_pattern quality=0.1 max_severity=critical",
        )]);
        let derived = &report.context_rot_risks[0];
        assert!(!derived.summary_line().contains(forbidden));
        assert!(
            !derived
                .detail_codes()
                .iter()
                .any(|code| code.contains(forbidden))
        );
    }

    #[test]
    fn context_rot_risk_marks_only_hard_injection_blockers() {
        let repairable = ContextRotRisk {
            experience_id: "legacy".to_owned(),
            score: 0.44,
            reasons: vec![
                "missing_clean_gist".to_owned(),
                "transcript_anchor_risk".to_owned(),
            ],
        };
        let blocking = ContextRotRisk {
            experience_id: "polluted".to_owned(),
            score: 0.91,
            reasons: vec![
                "missing_clean_gist".to_owned(),
                "long_without_clean_gist".to_owned(),
                "duplicate_experience".to_owned(),
                "cross_task_transcript_pollution".to_owned(),
                "duplicate_experience".to_owned(),
            ],
        };

        assert!(!repairable.requires_context_injection_blocker());
        assert_eq!(
            repairable.context_injection_blocker_reason_codes(),
            Vec::<String>::new()
        );
        assert!(blocking.requires_context_injection_blocker());
        assert_eq!(
            blocking.context_injection_blocker_reason_codes(),
            vec![
                "cross_task_transcript_pollution".to_owned(),
                "duplicate_experience".to_owned(),
                "long_without_clean_gist".to_owned(),
            ]
        );
        assert_eq!(
            blocking.reason_codes(),
            vec![
                "cross_task_transcript_pollution".to_owned(),
                "duplicate_experience".to_owned(),
                "long_without_clean_gist".to_owned(),
                "missing_clean_gist".to_owned(),
            ]
        );
    }

    #[test]
    fn rebuild_plan_covers_duplicates_noise_and_compaction() {
        let mut long = ExperienceEnvelope::new(
            "long",
            format!(
                "Conversation Transcript:\nUser: bash command for merge requests {}\nAssistant: ok",
                "x".repeat(2_600)
            ),
            "accepted_pattern quality=0.1 max_severity=critical",
        );
        long.clean_gist = None;
        let records = vec![
            ExperienceEnvelope::new("1", "Prompt A", "Lesson A"),
            ExperienceEnvelope::new("2", "prompt a", "lesson a"),
            long,
        ];
        let plan = DefaultExperienceGovernance::default().rebuild_plan(&records);
        assert!(plan.rebuild_required);
        assert_eq!(plan.deduplicate_groups.len(), 1);
        assert!(plan.compact_ids.iter().any(|id| id == "long"));
        assert!(plan.quarantine_candidate_ids.iter().any(|id| id == "long"));
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason == "repair_missing_or_dirty_clean_gist")
        );
        assert_eq!(plan.missing_clean_gist_ids, vec!["long"]);
        assert!(plan.dirty_clean_gist_ids.is_empty());
        assert_eq!(
            plan.summary_line(),
            "memory_rebuild required=true duplicate_groups=1 refresh=1 compact=1 quarantine=1 missing_clean_gist=1 dirty_clean_gist=0 dirty_gist=1 reasons=5 reason_codes=compact_long_context_without_gist|deduplicate_exact_fingerprints|quarantine_high_noise_records|refresh_noisy_or_rotting_index|repair_missing_or_dirty_clean_gist detail_codes=compact:long|deduplicate:1:2|dirty_gist:long|missing_clean_gist:long|quarantine:long|refresh:long"
        );
        assert_eq!(
            plan.reason_codes(),
            vec![
                "compact_long_context_without_gist".to_owned(),
                "deduplicate_exact_fingerprints".to_owned(),
                "quarantine_high_noise_records".to_owned(),
                "refresh_noisy_or_rotting_index".to_owned(),
                "repair_missing_or_dirty_clean_gist".to_owned(),
            ]
        );
    }

    #[test]
    fn scoped_governance_batch_preserves_rot_and_rebuild_evidence() {
        let current = MemoryScope::for_task("agent-memory");
        let records = vec![
            ExperienceEnvelope::new("clean", "Stable prompt", "Stable lesson")
                .with_clean_gist("A clean durable summary with enough useful signal.")
                .with_scope(current.clone()),
            ExperienceEnvelope::new("same_a", "Prompt A", "Lesson A")
                .with_clean_gist("A clean duplicate summary with enough signal.")
                .with_scope(current.clone()),
            ExperienceEnvelope::new("same_b", " prompt   a ", " lesson a ")
                .with_clean_gist("A clean duplicate summary with enough signal.")
                .with_scope(current.clone()),
            ExperienceEnvelope::new(
                "legacy",
                "A normal Rust runtime prompt",
                "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
            )
            .with_scope(current.clone()),
            ExperienceEnvelope::new(
                "polluted",
                format!(
                    "Conversation Transcript:\nUser: bash command for merge requests {}\nAssistant: ok",
                    "x".repeat(2_700)
                ),
                "accepted_pattern quality=0.2 max_severity=critical",
            )
            .with_scope(MemoryScope::for_task("ops-debug")),
        ];
        let governance = DefaultExperienceGovernance::default();

        let report = governance.assess_for_scope(&records, &current);
        let plan = governance.rebuild_plan_for_scope(&records, &current);

        assert_eq!(report.total_records, 5);
        assert_eq!(report.deduplication.duplicate_group_count, 1);
        assert_eq!(report.noisy_records.len(), 2);
        assert_eq!(report.context_rot_risks.len(), 3);
        assert!(
            report
                .reason_codes()
                .contains(&"cross_task_transcript_pollution".to_owned())
        );
        assert!(
            report
                .detail_codes()
                .contains(&"duplicate:same_a:same_b".to_owned())
        );
        assert!(
            report
                .detail_codes()
                .contains(&"noise:legacy:missing_clean_gist".to_owned())
        );
        assert!(
            report
                .detail_codes()
                .contains(&"noise:polluted:cross_task_transcript_pollution".to_owned())
        );
        assert!(
            report
                .detail_codes()
                .contains(&"context_rot:polluted:long_without_clean_gist".to_owned())
        );
        assert!(
            report
                .detail_codes()
                .contains(&"context_rot:same_b:duplicate_experience".to_owned())
        );
        assert!(report.summary_line().contains("records=5"));
        assert!(report.summary_line().contains("context_rot=3"));

        assert!(plan.rebuild_required);
        assert_eq!(plan.deduplicate_groups.len(), 1);
        assert_eq!(plan.refresh_embedding_ids, vec!["legacy", "polluted"]);
        assert_eq!(plan.compact_ids, vec!["polluted"]);
        assert_eq!(plan.quarantine_candidate_ids, vec!["polluted"]);
        assert_eq!(plan.missing_clean_gist_ids, vec!["legacy", "polluted"]);
        assert!(plan.dirty_clean_gist_ids.is_empty());
        assert_eq!(plan.dirty_gist_ids, vec!["legacy", "polluted"]);
        assert_eq!(
            plan.detail_codes(),
            vec![
                "compact:polluted".to_owned(),
                "deduplicate:same_a:same_b".to_owned(),
                "dirty_gist:legacy".to_owned(),
                "dirty_gist:polluted".to_owned(),
                "missing_clean_gist:legacy".to_owned(),
                "missing_clean_gist:polluted".to_owned(),
                "quarantine:polluted".to_owned(),
                "refresh:legacy".to_owned(),
                "refresh:polluted".to_owned(),
            ]
        );
        assert_eq!(
            plan.reason_codes(),
            vec![
                "compact_long_context_without_gist".to_owned(),
                "deduplicate_exact_fingerprints".to_owned(),
                "quarantine_high_noise_records".to_owned(),
                "refresh_noisy_or_rotting_index".to_owned(),
                "repair_missing_or_dirty_clean_gist".to_owned(),
            ]
        );
        assert!(plan.summary_line().contains("dirty_gist=2"));
        assert!(plan.summary_line().contains("missing_clean_gist=2"));
        assert!(plan.summary_line().contains("dirty_clean_gist=0"));
    }

    #[test]
    fn missing_clean_gist_on_legacy_metadata_is_repairable_noise() {
        let record = ExperienceEnvelope::new(
            "legacy",
            "A normal Rust runtime prompt",
            "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
        );

        let report = DefaultExperienceGovernance::default().assess(&[record]);
        let noise = &report.noisy_records[0];
        assert_eq!(noise.gist_status, GistStatus::Missing);
        assert!(
            noise
                .reasons
                .iter()
                .any(|reason| reason == "missing_clean_gist")
        );

        let plan = DefaultExperienceGovernance::default().rebuild_plan(&[ExperienceEnvelope::new(
            "legacy",
            "A normal Rust runtime prompt",
            "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
        )]);
        assert!(plan.dirty_gist_ids.iter().any(|id| id == "legacy"));
        assert!(plan.missing_clean_gist_ids.iter().any(|id| id == "legacy"));
        assert!(plan.dirty_clean_gist_ids.is_empty());
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason == "repair_missing_or_dirty_clean_gist")
        );
    }

    #[test]
    fn rebuild_plan_distinguishes_missing_and_dirty_clean_gist_repair() {
        let missing = ExperienceEnvelope::new(
            "missing",
            "A normal Rust runtime prompt",
            "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
        );
        let dirty = ExperienceEnvelope::new(
            "dirty",
            "A normal Rust runtime prompt",
            "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
        )
        .with_clean_gist("Conversation Transcript: User: stale Assistant: stale");

        let plan = DefaultExperienceGovernance::default().rebuild_plan(&[missing, dirty]);

        assert_eq!(plan.missing_clean_gist_ids, vec!["missing"]);
        assert_eq!(plan.dirty_clean_gist_ids, vec!["dirty"]);
        assert_eq!(plan.dirty_gist_ids, vec!["dirty", "missing"]);
        assert!(
            plan.detail_codes()
                .contains(&"missing_clean_gist:missing".to_owned())
        );
        assert!(
            plan.detail_codes()
                .contains(&"dirty_clean_gist:dirty".to_owned())
        );
        assert!(plan.summary_line().contains("missing_clean_gist=1"));
        assert!(plan.summary_line().contains("dirty_clean_gist=1"));
    }

    #[test]
    fn rebuild_plan_projects_clean_gist_repair_evidence_locally() {
        let forbidden = "REBUILD_GIST_SECRET_DO_NOT_LOG";
        let plan = IndexRebuildPlan {
            missing_clean_gist_ids: vec!["Legacy/No Gist".to_owned()],
            dirty_clean_gist_ids: vec!["Dirty Gist".to_owned()],
            dirty_gist_ids: vec!["Dirty Gist".to_owned(), "Legacy/No Gist".to_owned()],
            reasons: vec!["repair_missing_or_dirty_clean_gist".to_owned()],
            ..IndexRebuildPlan::default()
        };

        assert_eq!(
            plan.clean_gist_repair_detail_codes(),
            vec![
                "dirty_clean_gist:dirty_gist".to_owned(),
                "dirty_gist:dirty_gist".to_owned(),
                "dirty_gist:legacy_no_gist".to_owned(),
                "missing_clean_gist:legacy_no_gist".to_owned(),
            ]
        );
        assert_eq!(
            plan.clean_gist_repair_summary_line(),
            "clean_gist_repair missing_clean_gist=1 dirty_clean_gist=1 dirty_gist=2 detail_codes=dirty_clean_gist:dirty_gist|dirty_gist:dirty_gist|dirty_gist:legacy_no_gist|missing_clean_gist:legacy_no_gist"
        );
        assert!(
            plan.clean_gist_repair_detail_codes()
                .into_iter()
                .all(|code| plan.detail_codes().contains(&code))
        );
        assert!(!plan.clean_gist_repair_summary_line().contains(forbidden));

        let derived = DefaultExperienceGovernance::default().rebuild_plan(&[
            ExperienceEnvelope::new(
                "missing",
                "A normal Rust runtime prompt",
                "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
            ),
            ExperienceEnvelope::new(
                "dirty",
                "A normal Rust runtime prompt",
                "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
            )
            .with_clean_gist(format!(
                "Conversation Transcript: User: {forbidden} Assistant: stale"
            )),
        ]);

        assert_eq!(
            derived.clean_gist_repair_detail_codes(),
            vec![
                "dirty_clean_gist:dirty".to_owned(),
                "dirty_gist:dirty".to_owned(),
                "dirty_gist:missing".to_owned(),
                "missing_clean_gist:missing".to_owned(),
            ]
        );
        assert!(!derived.clean_gist_repair_summary_line().contains(forbidden));
    }

    #[test]
    fn quality_gate_allows_clean_experience_index_batch() {
        let records = vec![
            ExperienceEnvelope::new("clean", "Stable prompt", "Stable lesson")
                .with_clean_gist("A clean durable summary with enough useful signal."),
        ];
        let governance = DefaultExperienceGovernance::default();
        let report = governance.assess(&records);
        let plan = governance.rebuild_plan(&records);
        let gate = report.quality_gate(&plan);

        assert!(gate.ready_for_context_injection);
        assert_eq!(gate.blocker_count, 0);
        assert_eq!(gate.warning_count, 0);
        assert_eq!(gate.context_rot_blocker_count, 0);
        assert_eq!(gate.context_rot_blocker_reason_codes, Vec::<String>::new());
        assert_eq!(gate.reason_codes(), Vec::<String>::new());
        assert_eq!(gate.detail_codes(), Vec::<String>::new());
        assert_eq!(
            gate.checklist_detail(),
            "quality_gate_blockers=0 quality_gate_warnings=0 quality_gate_context_rot_blockers=0 quality_gate_reason_codes=none quality_gate_context_rot_blocker_reason_codes=none quality_gate_detail_codes=none"
        );
        assert_eq!(
            gate.summary_line(),
            "experience_index_quality_gate ready_for_context_injection=true records=1 blockers=0 warnings=0 duplicates=0 refresh=0 compact=0 quarantine=0 missing_clean_gist=0 dirty_clean_gist=0 dirty_gist=0 context_rot_blockers=0 reason_codes=none context_rot_blocker_reason_codes=none detail_codes=none"
        );
    }

    #[test]
    fn quality_gate_splits_context_rot_blockers_from_gist_warnings() {
        let records = vec![
            ExperienceEnvelope::new("same-a", "Prompt A", "Lesson A")
                .with_clean_gist("A clean duplicate summary with enough signal."),
            ExperienceEnvelope::new("same-b", " prompt   a ", " lesson a ")
                .with_clean_gist("A clean duplicate summary with enough signal."),
            ExperienceEnvelope::new(
                "legacy",
                "A normal Rust runtime prompt",
                "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
            ),
            ExperienceEnvelope::new(
                "dirty",
                "A normal Rust runtime prompt",
                "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
            )
            .with_clean_gist("Conversation Transcript: User: stale Assistant: stale"),
            ExperienceEnvelope::new(
                "polluted",
                format!(
                    "Conversation Transcript:\nUser: bash command for merge requests {}\nAssistant: ok",
                    "x".repeat(2_700)
                ),
                "accepted_pattern quality=0.2 max_severity=critical",
            ),
        ];
        let governance = DefaultExperienceGovernance::default();
        let report = governance.assess(&records);
        let plan = governance.rebuild_plan(&records);
        let gate = ExperienceIndexQualityGate::from_report_and_plan(&report, &plan);

        assert!(!gate.ready_for_context_injection);
        assert_eq!(gate.duplicate_record_count, 2);
        assert_eq!(gate.blocker_count, 4);
        assert_eq!(gate.warning_count, 6);
        assert_eq!(gate.refresh_count, 3);
        assert_eq!(gate.compact_count, 1);
        assert_eq!(gate.quarantine_count, 1);
        assert_eq!(gate.missing_clean_gist_count, 2);
        assert_eq!(gate.dirty_clean_gist_count, 1);
        assert_eq!(gate.dirty_gist_count, 3);
        assert_eq!(report.context_rot_blocker_count(), 3);
        assert_eq!(
            report.context_rot_blocker_reason_codes(),
            vec![
                "duplicate_experience".to_owned(),
                "long_without_clean_gist".to_owned(),
            ]
        );
        assert_eq!(gate.context_rot_blocker_count, 3);
        assert_eq!(
            gate.context_rot_blocker_reason_codes,
            vec![
                "duplicate_experience".to_owned(),
                "long_without_clean_gist".to_owned(),
            ]
        );
        assert_eq!(
            gate.reason_codes(),
            vec![
                "compact_context_rot".to_owned(),
                "dirty_clean_gist".to_owned(),
                "dirty_gist".to_owned(),
                "duplicate_experience".to_owned(),
                "missing_clean_gist".to_owned(),
                "quarantine_context_rot".to_owned(),
                "refresh_noisy_or_rotting_index".to_owned(),
            ]
        );
        assert_eq!(
            gate.detail_codes(),
            vec![
                "blocker:compact:polluted".to_owned(),
                "blocker:duplicate:legacy:dirty".to_owned(),
                "blocker:duplicate:same_a:same_b".to_owned(),
                "blocker:quarantine:polluted".to_owned(),
                "warning:dirty_clean_gist:dirty".to_owned(),
                "warning:dirty_gist:dirty".to_owned(),
                "warning:dirty_gist:legacy".to_owned(),
                "warning:dirty_gist:polluted".to_owned(),
                "warning:missing_clean_gist:legacy".to_owned(),
                "warning:missing_clean_gist:polluted".to_owned(),
                "warning:refresh:dirty".to_owned(),
                "warning:refresh:legacy".to_owned(),
                "warning:refresh:polluted".to_owned(),
            ]
        );
        assert!(gate.summary_line().contains("blockers=4 warnings=6"));
        assert_eq!(
            gate.checklist_detail(),
            "quality_gate_blockers=4 quality_gate_warnings=6 quality_gate_context_rot_blockers=3 quality_gate_reason_codes=compact_context_rot|dirty_clean_gist|dirty_gist|duplicate_experience|missing_clean_gist|quarantine_context_rot|refresh_noisy_or_rotting_index quality_gate_context_rot_blocker_reason_codes=duplicate_experience|long_without_clean_gist quality_gate_detail_codes=blocker:compact:polluted|blocker:duplicate:legacy:dirty|blocker:duplicate:same_a:same_b|blocker:quarantine:polluted|warning:dirty_clean_gist:dirty|warning:dirty_gist:dirty|warning:dirty_gist:legacy|warning:dirty_gist:polluted|warning:missing_clean_gist:legacy|warning:missing_clean_gist:polluted|warning:refresh:dirty|warning:refresh:legacy|warning:refresh:polluted"
        );
        assert!(
            gate.summary_line()
                .contains("reason_codes=compact_context_rot|dirty_clean_gist")
        );
        assert!(gate.summary_line().contains(
            "context_rot_blocker_reason_codes=duplicate_experience|long_without_clean_gist"
        ));
    }

    #[test]
    fn envelope_builders_preserve_adapter_projection_fields() {
        let envelope = ExperienceEnvelope::new("42", "prompt", "lesson")
            .with_clean_gist("A clean adapter summary with enough signal.")
            .with_quality(8.0)
            .with_tags(vec!["runtime".to_owned(), "adapter".to_owned()])
            .with_scope(MemoryScope::for_task("task"));

        assert_eq!(
            envelope.clean_gist.as_deref(),
            Some("A clean adapter summary with enough signal.")
        );
        assert_eq!(envelope.quality, 1.0);
        assert_eq!(
            envelope.tags,
            vec!["runtime".to_owned(), "adapter".to_owned()]
        );
        assert_eq!(envelope.scope.task_id.as_deref(), Some("task"));
    }

    #[test]
    fn scoped_assessment_flags_cross_task_transcript_pollution() {
        let current = MemoryScope::for_task("rust-runtime");
        let record = ExperienceEnvelope::new(
            "shell",
            "Conversation Transcript:\nUser: run bash command for merge requests\nAssistant: ok",
            "Reuse only when the GitLab merge task is active.",
        )
        .with_scope(MemoryScope::for_task("gitlab-merge"));

        let report = DefaultExperienceGovernance::default().assess_for_scope(&[record], &current);
        let noise = &report.noisy_records[0];
        assert!(
            noise
                .reasons
                .iter()
                .any(|reason| reason == "cross_task_transcript_pollution")
        );
        let risk = &report.context_rot_risks[0];
        assert!(
            risk.reasons
                .iter()
                .any(|reason| reason == "cross_task_transcript_pollution")
        );
        assert!(
            report
                .detail_codes()
                .contains(&"noise:shell:cross_task_transcript_pollution".to_owned())
        );
        assert!(
            report
                .detail_codes()
                .contains(&"context_rot:shell:cross_task_transcript_pollution".to_owned())
        );
    }

    #[test]
    fn scoped_rebuild_plan_quarantines_cross_task_context_rot() {
        let current = MemoryScope::for_task("agent-memory");
        let record = ExperienceEnvelope::new(
            "polluted",
            "Conversation Transcript:\nUser: ssh -o ConnectTimeout=1 gitlab.local\nAssistant: ok",
            "accepted_pattern quality=0.2 max_severity=critical",
        )
        .with_scope(MemoryScope::for_task("ops-debug"));

        let plan =
            DefaultExperienceGovernance::default().rebuild_plan_for_scope(&[record], &current);
        assert!(plan.rebuild_required);
        assert!(
            plan.quarantine_candidate_ids
                .iter()
                .any(|id| id == "polluted")
        );
        assert!(plan.dirty_gist_ids.iter().any(|id| id == "polluted"));
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason == "refresh_noisy_or_rotting_index")
        );
    }
}
