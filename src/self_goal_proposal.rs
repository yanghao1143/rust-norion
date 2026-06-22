use crate::evolution_goal::{
    EvolutionGoal, EvolutionGoalApprovalGate, EvolutionGoalBudgetCap, EvolutionGoalEvidenceKind,
    EvolutionGoalQueue, EvolutionGoalRollbackCondition, EvolutionGoalRunEvidence,
    EvolutionGoalStatus, EvolutionGoalStopCondition, EvolutionGoalSuccessGate,
    default_noiron_pursuit_goal_queue,
};
use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};
use crate::writer_gate::{
    UnifiedWriterGate, UnifiedWriterGateCandidate, UnifiedWriterGateDecision,
    UnifiedWriterGateDomain, UnifiedWriterGateRecord, UnifiedWriterGateReport,
    UnifiedWriterGateWriteScope,
};

pub const SELF_GOAL_PROPOSAL_SCHEMA_VERSION: &str = "self_goal_proposal_v1";
pub const SELF_GOAL_PROPOSAL_TRACE_SCHEMA: &str = "rust-norion-self-goal-proposal-preview-v1";
pub const SELF_GOAL_ADMISSION_SCHEMA_VERSION: &str = "self_goal_admission_v1";
pub const SELF_GOAL_ADMISSION_TRACE_SCHEMA: &str = "rust-norion-self-goal-admission-preview-v1";
pub const SELF_GOAL_QUEUE_PREVIEW_SCHEMA_VERSION: &str = "self_goal_queue_preview_v1";
pub const SELF_GOAL_QUEUE_PREVIEW_TRACE_SCHEMA: &str = "rust-norion-self-goal-queue-preview-v1";
pub const SELF_GOAL_QUEUE_APPLY_PLAN_SCHEMA_VERSION: &str = "self_goal_queue_apply_plan_v1";
pub const SELF_GOAL_QUEUE_APPLY_PLAN_TRACE_SCHEMA: &str =
    "rust-norion-self-goal-queue-apply-plan-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelfGoalProposalSource {
    ActiveQueueGap,
    EvidenceGap,
    RoadmapSuccessor,
    GovernanceGate,
}

impl SelfGoalProposalSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ActiveQueueGap => "active_queue_gap",
            Self::EvidenceGap => "evidence_gap",
            Self::RoadmapSuccessor => "roadmap_successor",
            Self::GovernanceGate => "governance_gate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfGoalProposalPolicy {
    pub max_candidates: usize,
    pub require_success_gate: bool,
    pub require_stop_condition: bool,
    pub require_rollback_condition: bool,
    pub require_budget_cap: bool,
    pub require_approval_gate: bool,
    pub require_conflict_isolation: bool,
    pub require_digest_only_evidence: bool,
    pub allow_write: bool,
}

impl Default for SelfGoalProposalPolicy {
    fn default() -> Self {
        Self {
            max_candidates: 4,
            require_success_gate: true,
            require_stop_condition: true,
            require_rollback_condition: true,
            require_budget_cap: true,
            require_approval_gate: true,
            require_conflict_isolation: true,
            require_digest_only_evidence: true,
            allow_write: false,
        }
    }
}

impl SelfGoalProposalPolicy {
    pub fn is_preview_safe(self) -> bool {
        self.max_candidates > 0
            && self.require_success_gate
            && self.require_stop_condition
            && self.require_rollback_condition
            && self.require_budget_cap
            && self.require_approval_gate
            && self.require_conflict_isolation
            && self.require_digest_only_evidence
            && !self.allow_write
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfGoalProposalCandidate {
    pub schema_version: &'static str,
    pub stable_id: String,
    pub source: SelfGoalProposalSource,
    pub target_release: String,
    pub proposed_goal: EvolutionGoal,
    pub rationale: String,
    pub conflict_isolation_note: String,
    pub provenance_digest: String,
    pub evidence_digest: String,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SelfGoalProposalCandidate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: SelfGoalProposalSource,
        target_release: impl Into<String>,
        priority: u32,
        objective: impl Into<String>,
        required_evidence: impl IntoIterator<Item = EvolutionGoalEvidenceKind>,
        budget_cap: EvolutionGoalBudgetCap,
        provenance_parts: impl IntoIterator<Item = impl AsRef<str>>,
        rationale: impl Into<String>,
        conflict_isolation_note: impl Into<String>,
    ) -> Self {
        let target_release = safe_text(target_release.into());
        let objective = safe_text(objective.into());
        let rationale = safe_text(rationale.into());
        let conflict_isolation_note = safe_text(conflict_isolation_note.into());
        let provenance = provenance_parts
            .into_iter()
            .map(|part| safe_text(part.as_ref().to_owned()))
            .collect::<Vec<_>>();
        let provenance_refs = provenance.iter().map(String::as_str).collect::<Vec<_>>();

        let proposed_goal = EvolutionGoal::with_policy(
            priority,
            objective,
            EvolutionGoalSuccessGate::new(required_evidence),
            EvolutionGoalStopCondition::default(),
            EvolutionGoalRollbackCondition::default(),
            budget_cap,
            EvolutionGoalApprovalGate::default(),
            provenance_refs.iter().copied(),
        );
        let evidence_digest = stable_redaction_digest([
            SELF_GOAL_PROPOSAL_SCHEMA_VERSION,
            source.as_str(),
            target_release.as_str(),
            proposed_goal.stable_id.as_str(),
            proposed_goal.provenance_digest.as_str(),
            rationale.as_str(),
            conflict_isolation_note.as_str(),
        ]);
        let stable_id = stable_redaction_digest([
            SELF_GOAL_PROPOSAL_SCHEMA_VERSION,
            source.as_str(),
            target_release.as_str(),
            proposed_goal.stable_id.as_str(),
            evidence_digest.as_str(),
        ]);

        Self {
            schema_version: SELF_GOAL_PROPOSAL_SCHEMA_VERSION,
            stable_id,
            source,
            target_release,
            provenance_digest: proposed_goal.provenance_digest.clone(),
            proposed_goal,
            rationale,
            conflict_isolation_note,
            evidence_digest,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }

    pub fn has_required_governance(&self) -> bool {
        !self.proposed_goal.success_gate.required_evidence.is_empty()
            && self.proposed_goal.stop_condition.success_stops_goal
            && self
                .proposed_goal
                .stop_condition
                .budget_exhaustion_stops_goal
            && self.proposed_goal.stop_condition.rollback_stops_goal
            && self.proposed_goal.stop_condition.approval_hold_stops_queue
            && self
                .proposed_goal
                .rollback_condition
                .rollback_on_failed_required_evidence
            && self
                .proposed_goal
                .rollback_condition
                .rollback_on_trace_schema_failure
            && self
                .proposed_goal
                .rollback_condition
                .rollback_on_explicit_signal
            && self.proposed_goal.budget_cap.max_attempts > 0
            && self.proposed_goal.budget_cap.max_steps > 0
            && self.proposed_goal.budget_cap.max_tokens > 0
            && self.proposed_goal.budget_cap.max_runtime_ms > 0
            && self.proposed_goal.approval_gate.maintainer_required
            && self.proposed_goal.approval_gate.operator_required
            && self.proposed_goal.approval_gate.approval_evidence_required
            && !self.conflict_isolation_note.is_empty()
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.provenance_digest.starts_with("redaction-digest:")
            && self.evidence_digest.starts_with("redaction-digest:")
            && self.record_line().contains("redaction-digest:")
            && !contains_private_or_executable_marker(&self.record_line())
    }

    pub fn record_line(&self) -> String {
        [
            self.schema_version.to_owned(),
            self.stable_id.clone(),
            self.source.as_str().to_owned(),
            self.target_release.clone(),
            self.proposed_goal.stable_id.clone(),
            self.proposed_goal.priority.to_string(),
            self.proposed_goal.objective.clone(),
            evidence_kind_list(&self.proposed_goal.success_gate.required_evidence),
            self.proposed_goal
                .success_gate
                .min_passed_evidence
                .to_string(),
            budget_cap_field(self.proposed_goal.budget_cap),
            approval_gate_field(&self.proposed_goal.approval_gate),
            stop_condition_field(&self.proposed_goal.stop_condition),
            rollback_condition_field(&self.proposed_goal.rollback_condition),
            self.rationale.clone(),
            self.conflict_isolation_note.clone(),
            self.provenance_digest.clone(),
            self.evidence_digest.clone(),
            bool_to_field(self.read_only).to_owned(),
            bool_to_field(self.write_allowed).to_owned(),
            bool_to_field(self.applied).to_owned(),
        ]
        .iter()
        .map(|field| escape_field(field))
        .collect::<Vec<_>>()
        .join("\t")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfGoalProposalReport {
    pub schema_version: &'static str,
    pub trace_schema: &'static str,
    pub queue_schema_version: &'static str,
    pub active_goal_id: Option<String>,
    pub active_goal_objective_digest: Option<String>,
    pub candidate_count: usize,
    pub r97_candidate_count: usize,
    pub r98_candidate_count: usize,
    pub admission_gate_candidate_count: usize,
    pub policy: SelfGoalProposalPolicy,
    pub candidates: Vec<SelfGoalProposalCandidate>,
    pub candidate_record_lines: Vec<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SelfGoalProposalReport {
    pub fn from_queue(queue: &EvolutionGoalQueue, policy: SelfGoalProposalPolicy) -> Self {
        let queue_report = queue.evaluate(&[]);
        let active_goal_id = queue_report.active_goal_id;
        let active_goal = active_goal_id
            .as_ref()
            .and_then(|goal_id| queue.goals.iter().find(|goal| &goal.stable_id == goal_id));
        let active_goal_objective_digest = active_goal.map(|goal| {
            stable_redaction_digest([
                "self-goal-active-objective",
                goal.stable_id.as_str(),
                goal.objective.as_str(),
            ])
        });

        let active_objective = active_goal.map(|goal| goal.objective.as_str());
        let mut candidates = default_noiron_proposal_candidates(active_objective);
        candidates.truncate(policy.max_candidates);
        let candidate_record_lines = candidates
            .iter()
            .map(SelfGoalProposalCandidate::record_line)
            .collect::<Vec<_>>();
        let r97_candidate_count = candidates
            .iter()
            .filter(|candidate| candidate.target_release.contains("R97"))
            .count();
        let r98_candidate_count = candidates
            .iter()
            .filter(|candidate| candidate.target_release.contains("R98"))
            .count();
        let admission_gate_candidate_count = candidates
            .iter()
            .filter(|candidate| candidate.source == SelfGoalProposalSource::GovernanceGate)
            .count();

        Self {
            schema_version: SELF_GOAL_PROPOSAL_SCHEMA_VERSION,
            trace_schema: SELF_GOAL_PROPOSAL_TRACE_SCHEMA,
            queue_schema_version: queue.schema_version,
            active_goal_id,
            active_goal_objective_digest,
            candidate_count: candidates.len(),
            r97_candidate_count,
            r98_candidate_count,
            admission_gate_candidate_count,
            policy,
            candidates,
            candidate_record_lines,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn passed(&self) -> bool {
        self.is_preview_only()
            && self.policy.is_preview_safe()
            && self.candidate_count > 0
            && self.candidate_count == self.candidates.len()
            && self.candidate_record_lines.len() == self.candidates.len()
            && self.admission_gate_candidate_count > 0
            && self.candidates.iter().all(|candidate| {
                candidate.has_required_governance()
                    && candidate.evidence_is_redacted()
                    && candidate.is_preview_only()
            })
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self
                .candidates
                .iter()
                .all(SelfGoalProposalCandidate::is_preview_only)
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.active_goal_objective_digest
            .as_ref()
            .is_none_or(|digest| digest.starts_with("redaction-digest:"))
            && self.candidates.iter().all(|candidate| {
                candidate.evidence_is_redacted()
                    && !contains_private_or_executable_marker(&candidate.record_line())
            })
            && self.candidate_record_lines.iter().all(|line| {
                line.contains("redaction-digest:") && !contains_private_or_executable_marker(line)
            })
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_goal_proposal schema={} trace_schema={} passed={} candidates={} r97={} r98={} admission_gate={} evidence_redacted={} read_only={} write_allowed={} applied={}",
            self.schema_version,
            self.trace_schema,
            self.passed(),
            self.candidate_count,
            self.r97_candidate_count,
            self.r98_candidate_count,
            self.admission_gate_candidate_count,
            self.evidence_is_redacted(),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfGoalAdmissionPolicy {
    pub max_preview_admissions: usize,
    pub require_proposal_report_passed: bool,
    pub require_current_queue_clear: bool,
    pub require_candidate_governance: bool,
    pub require_digest_only_evidence: bool,
    pub require_operator_approval: bool,
    pub allow_queue_write: bool,
}

impl Default for SelfGoalAdmissionPolicy {
    fn default() -> Self {
        Self {
            max_preview_admissions: 1,
            require_proposal_report_passed: true,
            require_current_queue_clear: true,
            require_candidate_governance: true,
            require_digest_only_evidence: true,
            require_operator_approval: true,
            allow_queue_write: false,
        }
    }
}

impl SelfGoalAdmissionPolicy {
    pub fn is_preview_safe(self) -> bool {
        self.max_preview_admissions > 0
            && self.require_proposal_report_passed
            && self.require_current_queue_clear
            && self.require_candidate_governance
            && self.require_digest_only_evidence
            && self.require_operator_approval
            && !self.allow_queue_write
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelfGoalAdmissionDecision {
    PreviewAdmissible,
    HeldForPriorGoal,
    HeldForEvidence,
    HeldForApproval,
    HeldForAdmissionLimit,
    Rejected,
}

impl SelfGoalAdmissionDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreviewAdmissible => "preview_admissible",
            Self::HeldForPriorGoal => "held_for_prior_goal",
            Self::HeldForEvidence => "held_for_evidence",
            Self::HeldForApproval => "held_for_approval",
            Self::HeldForAdmissionLimit => "held_for_admission_limit",
            Self::Rejected => "rejected",
        }
    }

    pub fn is_hold(self) -> bool {
        matches!(
            self,
            Self::HeldForPriorGoal
                | Self::HeldForEvidence
                | Self::HeldForApproval
                | Self::HeldForAdmissionLimit
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfGoalAdmissionRecord {
    pub schema_version: &'static str,
    pub candidate_id: String,
    pub proposed_goal_id: String,
    pub source: SelfGoalProposalSource,
    pub target_release: String,
    pub decision: SelfGoalAdmissionDecision,
    pub reason_codes: Vec<String>,
    pub evidence_digests: Vec<String>,
    pub queue_insert_preview_digest: Option<String>,
    pub admitted_goal_record_line: Option<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SelfGoalAdmissionRecord {
    pub fn record_line(&self) -> String {
        let reasons = self.reason_codes.join("|");
        let evidence = self.evidence_digests.join("|");
        let record_digest = stable_redaction_digest([
            "self-goal-admission-record",
            self.candidate_id.as_str(),
            self.proposed_goal_id.as_str(),
            self.decision.as_str(),
            reasons.as_str(),
            evidence.as_str(),
        ]);
        [
            self.schema_version.to_owned(),
            self.candidate_id.clone(),
            self.proposed_goal_id.clone(),
            self.source.as_str().to_owned(),
            self.target_release.clone(),
            self.decision.as_str().to_owned(),
            reasons,
            evidence,
            self.queue_insert_preview_digest
                .clone()
                .unwrap_or_else(|| "none".to_owned()),
            self.admitted_goal_record_line
                .as_ref()
                .map(|line| stable_redaction_digest(["admitted-goal-record", line]))
                .unwrap_or_else(|| "none".to_owned()),
            record_digest,
            bool_to_field(self.read_only).to_owned(),
            bool_to_field(self.write_allowed).to_owned(),
            bool_to_field(self.applied).to_owned(),
        ]
        .iter()
        .map(|field| escape_field(field))
        .collect::<Vec<_>>()
        .join("\t")
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.evidence_digests
            .iter()
            .all(|digest| digest.starts_with("redaction-digest:"))
            && self
                .queue_insert_preview_digest
                .as_ref()
                .is_none_or(|digest| digest.starts_with("redaction-digest:"))
            && self.record_line().contains("redaction-digest:")
            && !contains_private_or_executable_marker(&self.record_line())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfGoalAdmissionReport {
    pub schema_version: &'static str,
    pub trace_schema: &'static str,
    pub proposal_schema_version: &'static str,
    pub active_goal_id: Option<String>,
    pub policy: SelfGoalAdmissionPolicy,
    pub record_count: usize,
    pub preview_admissible_count: usize,
    pub held_count: usize,
    pub rejected_count: usize,
    pub records: Vec<SelfGoalAdmissionRecord>,
    pub record_lines: Vec<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SelfGoalAdmissionReport {
    pub fn passed(&self) -> bool {
        self.is_preview_only()
            && self.policy.is_preview_safe()
            && self.record_count == self.records.len()
            && self.record_lines.len() == self.records.len()
            && self.preview_admissible_count <= self.policy.max_preview_admissions
            && self.records.iter().all(|record| {
                record.is_preview_only()
                    && record.evidence_is_redacted()
                    && !matches!(record.decision, SelfGoalAdmissionDecision::Rejected)
            })
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self
                .records
                .iter()
                .all(SelfGoalAdmissionRecord::is_preview_only)
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.records
            .iter()
            .all(SelfGoalAdmissionRecord::evidence_is_redacted)
            && self.record_lines.iter().all(|line| {
                line.contains("redaction-digest:") && !contains_private_or_executable_marker(line)
            })
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_goal_admission schema={} trace_schema={} passed={} records={} preview_admissible={} held={} rejected={} evidence_redacted={} read_only={} write_allowed={} applied={}",
            self.schema_version,
            self.trace_schema,
            self.passed(),
            self.record_count,
            self.preview_admissible_count,
            self.held_count,
            self.rejected_count,
            self.evidence_is_redacted(),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfGoalAdmissionGate {
    pub policy: SelfGoalAdmissionPolicy,
}

impl Default for SelfGoalAdmissionGate {
    fn default() -> Self {
        Self {
            policy: SelfGoalAdmissionPolicy::default(),
        }
    }
}

impl SelfGoalAdmissionGate {
    pub fn new(policy: SelfGoalAdmissionPolicy) -> Self {
        Self { policy }
    }

    pub fn evaluate(
        &self,
        proposal_report: &SelfGoalProposalReport,
        runs: &[EvolutionGoalRunEvidence],
    ) -> SelfGoalAdmissionReport {
        let mut preview_admissions = 0;
        let mut records = Vec::with_capacity(proposal_report.candidates.len());

        for candidate in &proposal_report.candidates {
            let record =
                self.evaluate_candidate(proposal_report, candidate, runs, &mut preview_admissions);
            records.push(record);
        }

        let record_lines = records
            .iter()
            .map(SelfGoalAdmissionRecord::record_line)
            .collect::<Vec<_>>();
        let preview_admissible_count = records
            .iter()
            .filter(|record| record.decision == SelfGoalAdmissionDecision::PreviewAdmissible)
            .count();
        let held_count = records
            .iter()
            .filter(|record| record.decision.is_hold())
            .count();
        let rejected_count = records
            .iter()
            .filter(|record| record.decision == SelfGoalAdmissionDecision::Rejected)
            .count();

        SelfGoalAdmissionReport {
            schema_version: SELF_GOAL_ADMISSION_SCHEMA_VERSION,
            trace_schema: SELF_GOAL_ADMISSION_TRACE_SCHEMA,
            proposal_schema_version: proposal_report.schema_version,
            active_goal_id: proposal_report.active_goal_id.clone(),
            policy: self.policy,
            record_count: records.len(),
            preview_admissible_count,
            held_count,
            rejected_count,
            records,
            record_lines,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    fn evaluate_candidate(
        &self,
        proposal_report: &SelfGoalProposalReport,
        candidate: &SelfGoalProposalCandidate,
        runs: &[EvolutionGoalRunEvidence],
        preview_admissions: &mut usize,
    ) -> SelfGoalAdmissionRecord {
        if !self.policy.is_preview_safe() {
            return admission_record(
                candidate,
                SelfGoalAdmissionDecision::Rejected,
                ["admission_policy_not_preview_safe"],
                Vec::new(),
                None,
            );
        }

        if self.policy.require_proposal_report_passed && !proposal_report.passed() {
            return admission_record(
                candidate,
                SelfGoalAdmissionDecision::Rejected,
                ["proposal_report_failed"],
                Vec::new(),
                None,
            );
        }

        if self.policy.require_candidate_governance && !candidate.has_required_governance() {
            return admission_record(
                candidate,
                SelfGoalAdmissionDecision::Rejected,
                ["candidate_governance_missing"],
                Vec::new(),
                None,
            );
        }

        if self.policy.require_digest_only_evidence && !candidate.evidence_is_redacted() {
            return admission_record(
                candidate,
                SelfGoalAdmissionDecision::Rejected,
                ["candidate_evidence_not_redacted"],
                Vec::new(),
                None,
            );
        }

        if self.policy.require_current_queue_clear && proposal_report.active_goal_id.is_some() {
            return admission_record(
                candidate,
                SelfGoalAdmissionDecision::HeldForPriorGoal,
                ["current_queue_has_active_goal", "conflict_isolation_hold"],
                Vec::new(),
                None,
            );
        }

        let matching_runs = runs
            .iter()
            .filter(|run| run.goal_id == candidate.proposed_goal.stable_id)
            .cloned()
            .collect::<Vec<_>>();
        let queue = EvolutionGoalQueue::new(vec![candidate.proposed_goal.clone()]);
        let report = queue.evaluate(&matching_runs);
        let decision = &report.decisions[0];

        match decision.status {
            EvolutionGoalStatus::Passed => {
                if *preview_admissions >= self.policy.max_preview_admissions {
                    return admission_record_from_vec(
                        candidate,
                        SelfGoalAdmissionDecision::HeldForAdmissionLimit,
                        vec!["admission_limit_reached".to_owned()],
                        decision.evidence_digests.clone(),
                        None,
                    );
                }
                *preview_admissions += 1;
                let digest = stable_redaction_digest([
                    SELF_GOAL_ADMISSION_SCHEMA_VERSION,
                    candidate.stable_id.as_str(),
                    candidate.proposed_goal.stable_id.as_str(),
                    candidate.proposed_goal.provenance_digest.as_str(),
                    "preview-queue-insert",
                ]);
                admission_record_from_vec(
                    candidate,
                    SelfGoalAdmissionDecision::PreviewAdmissible,
                    vec!["success_gate_passed_for_queue_admission_preview".to_owned()],
                    decision.evidence_digests.clone(),
                    Some(digest),
                )
            }
            EvolutionGoalStatus::BlockedForApproval => admission_record_from_vec(
                candidate,
                SelfGoalAdmissionDecision::HeldForApproval,
                decision.reason_codes.clone(),
                decision.evidence_digests.clone(),
                None,
            ),
            EvolutionGoalStatus::Active | EvolutionGoalStatus::Queued => admission_record_from_vec(
                candidate,
                SelfGoalAdmissionDecision::HeldForEvidence,
                decision.reason_codes.clone(),
                decision.evidence_digests.clone(),
                None,
            ),
            EvolutionGoalStatus::Failed
            | EvolutionGoalStatus::RolledBack
            | EvolutionGoalStatus::BudgetExhausted => admission_record_from_vec(
                candidate,
                SelfGoalAdmissionDecision::Rejected,
                decision.reason_codes.clone(),
                decision.evidence_digests.clone(),
                None,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfGoalQueuePreviewPolicy {
    pub max_append_records: usize,
    pub require_admission_report_passed: bool,
    pub require_preview_admissible: bool,
    pub require_queue_preview_only: bool,
    pub reject_duplicate_goal: bool,
    pub require_digest_only_evidence: bool,
    pub allow_queue_write: bool,
}

impl Default for SelfGoalQueuePreviewPolicy {
    fn default() -> Self {
        Self {
            max_append_records: 1,
            require_admission_report_passed: true,
            require_preview_admissible: true,
            require_queue_preview_only: true,
            reject_duplicate_goal: true,
            require_digest_only_evidence: true,
            allow_queue_write: false,
        }
    }
}

impl SelfGoalQueuePreviewPolicy {
    pub fn is_preview_safe(self) -> bool {
        self.max_append_records > 0
            && self.require_admission_report_passed
            && self.require_preview_admissible
            && self.require_queue_preview_only
            && self.reject_duplicate_goal
            && self.require_digest_only_evidence
            && !self.allow_queue_write
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelfGoalQueuePreviewDecision {
    AppendPreview,
    HeldForAdmissionGate,
    HeldForDuplicateGoal,
    HeldForAppendLimit,
    Rejected,
}

impl SelfGoalQueuePreviewDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AppendPreview => "append_preview",
            Self::HeldForAdmissionGate => "held_for_admission_gate",
            Self::HeldForDuplicateGoal => "held_for_duplicate_goal",
            Self::HeldForAppendLimit => "held_for_append_limit",
            Self::Rejected => "rejected",
        }
    }

    pub fn is_hold(self) -> bool {
        matches!(
            self,
            Self::HeldForAdmissionGate | Self::HeldForDuplicateGoal | Self::HeldForAppendLimit
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfGoalQueuePreviewRecord {
    pub schema_version: &'static str,
    pub candidate_id: String,
    pub proposed_goal_id: String,
    pub decision: SelfGoalQueuePreviewDecision,
    pub reason_codes: Vec<String>,
    pub existing_queue_digest: String,
    pub append_record_digest: Option<String>,
    pub resulting_queue_preview_digest: Option<String>,
    pub append_record_line: Option<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SelfGoalQueuePreviewRecord {
    pub fn record_line(&self) -> String {
        let reasons = self.reason_codes.join("|");
        let append_digest = self
            .append_record_digest
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let resulting_digest = self
            .resulting_queue_preview_digest
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let record_digest = stable_redaction_digest([
            "self-goal-queue-preview-record",
            self.candidate_id.as_str(),
            self.proposed_goal_id.as_str(),
            self.decision.as_str(),
            reasons.as_str(),
            self.existing_queue_digest.as_str(),
            append_digest.as_str(),
            resulting_digest.as_str(),
        ]);
        [
            self.schema_version.to_owned(),
            self.candidate_id.clone(),
            self.proposed_goal_id.clone(),
            self.decision.as_str().to_owned(),
            reasons,
            self.existing_queue_digest.clone(),
            append_digest,
            resulting_digest,
            record_digest,
            bool_to_field(self.read_only).to_owned(),
            bool_to_field(self.write_allowed).to_owned(),
            bool_to_field(self.applied).to_owned(),
        ]
        .iter()
        .map(|field| escape_field(field))
        .collect::<Vec<_>>()
        .join("\t")
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.existing_queue_digest.starts_with("redaction-digest:")
            && self
                .append_record_digest
                .as_ref()
                .is_none_or(|digest| digest.starts_with("redaction-digest:"))
            && self
                .resulting_queue_preview_digest
                .as_ref()
                .is_none_or(|digest| digest.starts_with("redaction-digest:"))
            && self
                .append_record_line
                .as_ref()
                .is_none_or(|line| !contains_private_or_executable_marker(line))
            && self.record_line().contains("redaction-digest:")
            && !contains_private_or_executable_marker(&self.record_line())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfGoalQueuePreviewReport {
    pub schema_version: &'static str,
    pub trace_schema: &'static str,
    pub admission_schema_version: &'static str,
    pub existing_queue_digest: String,
    pub policy: SelfGoalQueuePreviewPolicy,
    pub record_count: usize,
    pub append_preview_count: usize,
    pub held_count: usize,
    pub rejected_count: usize,
    pub records: Vec<SelfGoalQueuePreviewRecord>,
    pub record_lines: Vec<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SelfGoalQueuePreviewReport {
    pub fn passed(&self) -> bool {
        self.is_preview_only()
            && self.policy.is_preview_safe()
            && self.record_count == self.records.len()
            && self.record_lines.len() == self.records.len()
            && self.append_preview_count <= self.policy.max_append_records
            && self.rejected_count == 0
            && self
                .records
                .iter()
                .all(|record| record.is_preview_only() && record.evidence_is_redacted())
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self
                .records
                .iter()
                .all(SelfGoalQueuePreviewRecord::is_preview_only)
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.existing_queue_digest.starts_with("redaction-digest:")
            && self
                .records
                .iter()
                .all(SelfGoalQueuePreviewRecord::evidence_is_redacted)
            && self.record_lines.iter().all(|line| {
                line.contains("redaction-digest:") && !contains_private_or_executable_marker(line)
            })
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_goal_queue_preview schema={} trace_schema={} passed={} records={} append_preview={} held={} rejected={} evidence_redacted={} read_only={} write_allowed={} applied={}",
            self.schema_version,
            self.trace_schema,
            self.passed(),
            self.record_count,
            self.append_preview_count,
            self.held_count,
            self.rejected_count,
            self.evidence_is_redacted(),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfGoalQueueApplyPolicy {
    pub max_apply_records: usize,
    pub require_preview_report_passed: bool,
    pub require_writer_gate_ready: bool,
    pub require_evolution_goal_queue_domain: bool,
    pub require_matching_writer_candidate: bool,
    pub require_single_append_packet: bool,
    pub require_current_queue_digest_match: bool,
    pub reject_duplicate_goal: bool,
    pub require_digest_only_evidence: bool,
    pub allow_durable_queue_write: bool,
}

impl Default for SelfGoalQueueApplyPolicy {
    fn default() -> Self {
        Self {
            max_apply_records: 1,
            require_preview_report_passed: true,
            require_writer_gate_ready: true,
            require_evolution_goal_queue_domain: true,
            require_matching_writer_candidate: true,
            require_single_append_packet: true,
            require_current_queue_digest_match: true,
            reject_duplicate_goal: true,
            require_digest_only_evidence: true,
            allow_durable_queue_write: false,
        }
    }
}

impl SelfGoalQueueApplyPolicy {
    pub fn is_preview_safe(self) -> bool {
        self.max_apply_records > 0
            && self.require_preview_report_passed
            && self.require_writer_gate_ready
            && self.require_evolution_goal_queue_domain
            && self.require_matching_writer_candidate
            && self.require_single_append_packet
            && self.require_current_queue_digest_match
            && self.reject_duplicate_goal
            && self.require_digest_only_evidence
            && !self.allow_durable_queue_write
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelfGoalQueueApplyDecision {
    ReadyForExplicitApply,
    HeldForWriterGate,
    HeldForAppendPacket,
    HeldForDuplicateGoal,
    Rejected,
}

impl SelfGoalQueueApplyDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadyForExplicitApply => "ready_for_explicit_apply",
            Self::HeldForWriterGate => "held_for_writer_gate",
            Self::HeldForAppendPacket => "held_for_append_packet",
            Self::HeldForDuplicateGoal => "held_for_duplicate_goal",
            Self::Rejected => "rejected",
        }
    }

    pub fn is_hold(self) -> bool {
        matches!(
            self,
            Self::HeldForWriterGate | Self::HeldForAppendPacket | Self::HeldForDuplicateGoal
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfGoalQueueApplyRecord {
    pub schema_version: &'static str,
    pub candidate_id: String,
    pub proposed_goal_id: String,
    pub decision: SelfGoalQueueApplyDecision,
    pub reason_codes: Vec<String>,
    pub current_queue_digest: String,
    pub rollback_anchor_digest: String,
    pub append_record_digest: Option<String>,
    pub resulting_queue_preview_digest: Option<String>,
    pub expected_resulting_queue_digest: Option<String>,
    pub writer_gate_candidate_id: Option<String>,
    pub writer_gate_refs_digest: Option<String>,
    pub apply_plan_digest: String,
    pub explicit_apply_required: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SelfGoalQueueApplyRecord {
    pub fn record_line(&self) -> String {
        let reasons = self.reason_codes.join("|");
        let append_digest = self
            .append_record_digest
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let resulting_digest = self
            .resulting_queue_preview_digest
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let expected_digest = self
            .expected_resulting_queue_digest
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let writer_candidate = self
            .writer_gate_candidate_id
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let writer_refs = self
            .writer_gate_refs_digest
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let record_digest = stable_redaction_digest([
            "self-goal-queue-apply-record",
            self.candidate_id.as_str(),
            self.proposed_goal_id.as_str(),
            self.decision.as_str(),
            reasons.as_str(),
            self.current_queue_digest.as_str(),
            self.rollback_anchor_digest.as_str(),
            append_digest.as_str(),
            resulting_digest.as_str(),
            expected_digest.as_str(),
            writer_candidate.as_str(),
            writer_refs.as_str(),
            self.apply_plan_digest.as_str(),
        ]);
        [
            self.schema_version.to_owned(),
            self.candidate_id.clone(),
            self.proposed_goal_id.clone(),
            self.decision.as_str().to_owned(),
            reasons,
            self.current_queue_digest.clone(),
            self.rollback_anchor_digest.clone(),
            append_digest,
            resulting_digest,
            expected_digest,
            writer_candidate,
            writer_refs,
            self.apply_plan_digest.clone(),
            bool_to_field(self.explicit_apply_required).to_owned(),
            record_digest,
            bool_to_field(self.read_only).to_owned(),
            bool_to_field(self.write_allowed).to_owned(),
            bool_to_field(self.applied).to_owned(),
        ]
        .iter()
        .map(|field| escape_field(field))
        .collect::<Vec<_>>()
        .join("\t")
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.current_queue_digest.starts_with("redaction-digest:")
            && self.rollback_anchor_digest.starts_with("redaction-digest:")
            && self
                .append_record_digest
                .as_ref()
                .is_none_or(|digest| digest.starts_with("redaction-digest:"))
            && self
                .resulting_queue_preview_digest
                .as_ref()
                .is_none_or(|digest| digest.starts_with("redaction-digest:"))
            && self
                .expected_resulting_queue_digest
                .as_ref()
                .is_none_or(|digest| digest.starts_with("redaction-digest:"))
            && self
                .writer_gate_refs_digest
                .as_ref()
                .is_none_or(|digest| digest.starts_with("redaction-digest:"))
            && self.apply_plan_digest.starts_with("redaction-digest:")
            && self.record_line().contains("redaction-digest:")
            && !contains_private_or_executable_marker(&self.record_line())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfGoalQueueApplyReport {
    pub schema_version: &'static str,
    pub trace_schema: &'static str,
    pub queue_preview_schema_version: &'static str,
    pub writer_gate_schema_version: &'static str,
    pub current_queue_digest: String,
    pub writer_gate_decision: UnifiedWriterGateDecision,
    pub policy: SelfGoalQueueApplyPolicy,
    pub decision: SelfGoalQueueApplyDecision,
    pub record_count: usize,
    pub ready_count: usize,
    pub held_count: usize,
    pub rejected_count: usize,
    pub records: Vec<SelfGoalQueueApplyRecord>,
    pub record_lines: Vec<String>,
    pub explicit_apply_required: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SelfGoalQueueApplyReport {
    pub fn passed(&self) -> bool {
        self.is_preview_only()
            && self.policy.is_preview_safe()
            && self.record_count == self.records.len()
            && self.record_lines.len() == self.records.len()
            && self.ready_count <= self.policy.max_apply_records
            && self.rejected_count == 0
            && (self.ready_count == 0 || self.explicit_apply_required)
            && self
                .records
                .iter()
                .all(|record| record.is_preview_only() && record.evidence_is_redacted())
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self
                .records
                .iter()
                .all(SelfGoalQueueApplyRecord::is_preview_only)
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.current_queue_digest.starts_with("redaction-digest:")
            && self
                .records
                .iter()
                .all(SelfGoalQueueApplyRecord::evidence_is_redacted)
            && self.record_lines.iter().all(|line| {
                line.contains("redaction-digest:") && !contains_private_or_executable_marker(line)
            })
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_goal_queue_apply_plan schema={} trace_schema={} passed={} decision={} writer_gate_decision={} records={} ready={} held={} rejected={} explicit_apply_required={} evidence_redacted={} read_only={} write_allowed={} applied={}",
            self.schema_version,
            self.trace_schema,
            self.passed(),
            self.decision.as_str(),
            self.writer_gate_decision.as_str(),
            self.record_count,
            self.ready_count,
            self.held_count,
            self.rejected_count,
            self.explicit_apply_required,
            self.evidence_is_redacted(),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }

    pub fn json_line(&self) -> String {
        let reason_code_count = self
            .records
            .iter()
            .map(|record| record.reason_codes.len())
            .sum::<usize>();
        let mut digest_parts = vec![
            SELF_GOAL_QUEUE_APPLY_PLAN_TRACE_SCHEMA.to_owned(),
            self.schema_version.to_owned(),
            self.queue_preview_schema_version.to_owned(),
            self.writer_gate_schema_version.to_owned(),
            self.current_queue_digest.clone(),
            self.writer_gate_decision.as_str().to_owned(),
            self.decision.as_str().to_owned(),
            self.record_count.to_string(),
            self.ready_count.to_string(),
            self.held_count.to_string(),
            self.rejected_count.to_string(),
            reason_code_count.to_string(),
        ];
        digest_parts.extend(
            self.records
                .iter()
                .map(|record| record.apply_plan_digest.clone()),
        );
        let apply_plan_digest = stable_redaction_digest(digest_parts.iter().map(String::as_str));
        format!(
            "{{\"schema\":\"{}\",\"plan_schema\":\"{}\",\"queue_preview_schema\":\"{}\",\"writer_gate_schema\":\"{}\",\"decision\":\"{}\",\"writer_gate_decision\":\"{}\",\"records\":{},\"ready_records\":{},\"held_records\":{},\"rejected_records\":{},\"reason_code_count\":{},\"explicit_apply_required\":{},\"current_queue_digest\":\"{}\",\"apply_plan_digest\":\"{}\",\"read_only\":{},\"write_allowed\":{},\"applied\":{},\"summary\":\"{}\"}}",
            json_escape(SELF_GOAL_QUEUE_APPLY_PLAN_TRACE_SCHEMA),
            json_escape(self.schema_version),
            json_escape(self.queue_preview_schema_version),
            json_escape(self.writer_gate_schema_version),
            json_escape(self.decision.as_str()),
            json_escape(self.writer_gate_decision.as_str()),
            self.record_count,
            self.ready_count,
            self.held_count,
            self.rejected_count,
            reason_code_count,
            self.explicit_apply_required,
            json_escape(&self.current_queue_digest),
            json_escape(&apply_plan_digest),
            self.read_only,
            self.write_allowed,
            self.applied,
            json_escape(&self.summary_line())
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfGoalQueueApplyPlanner {
    pub policy: SelfGoalQueueApplyPolicy,
}

impl Default for SelfGoalQueueApplyPlanner {
    fn default() -> Self {
        Self {
            policy: SelfGoalQueueApplyPolicy::default(),
        }
    }
}

impl SelfGoalQueueApplyPlanner {
    pub fn new(policy: SelfGoalQueueApplyPolicy) -> Self {
        Self { policy }
    }

    pub fn evaluate(
        &self,
        current_queue: &EvolutionGoalQueue,
        queue_preview_report: &SelfGoalQueuePreviewReport,
        writer_gate_report: &UnifiedWriterGateReport,
    ) -> SelfGoalQueueApplyReport {
        let current_queue_digest = queue_digest(current_queue);
        let expected_writer_candidate_id = queue_writer_candidate_id(queue_preview_report);
        let writer_record =
            matching_queue_writer_record(writer_gate_report, &expected_writer_candidate_id);

        let append_records = queue_preview_report
            .records
            .iter()
            .filter(|record| record.decision == SelfGoalQueuePreviewDecision::AppendPreview)
            .collect::<Vec<_>>();
        let mut records = Vec::with_capacity(append_records.len().max(1));
        if queue_preview_report.records.is_empty() {
            records.push(self.evaluate_missing_record(
                current_queue,
                queue_preview_report,
                writer_gate_report,
                writer_record,
                &current_queue_digest,
                &expected_writer_candidate_id,
            ));
        } else if append_records.is_empty() {
            records.extend(queue_preview_report.records.iter().map(|preview_record| {
                self.evaluate_record(
                    current_queue,
                    queue_preview_report,
                    writer_gate_report,
                    writer_record,
                    preview_record,
                    &current_queue_digest,
                    &expected_writer_candidate_id,
                )
            }));
        } else {
            for preview_record in append_records {
                records.push(self.evaluate_record(
                    current_queue,
                    queue_preview_report,
                    writer_gate_report,
                    writer_record,
                    preview_record,
                    &current_queue_digest,
                    &expected_writer_candidate_id,
                ));
            }
        }

        let record_lines = records
            .iter()
            .map(SelfGoalQueueApplyRecord::record_line)
            .collect::<Vec<_>>();
        let ready_count = records
            .iter()
            .filter(|record| record.decision == SelfGoalQueueApplyDecision::ReadyForExplicitApply)
            .count();
        let held_count = records
            .iter()
            .filter(|record| record.decision.is_hold())
            .count();
        let rejected_count = records
            .iter()
            .filter(|record| record.decision == SelfGoalQueueApplyDecision::Rejected)
            .count();
        let decision = if rejected_count > 0 {
            SelfGoalQueueApplyDecision::Rejected
        } else if ready_count > 0 && held_count == 0 {
            SelfGoalQueueApplyDecision::ReadyForExplicitApply
        } else {
            records
                .iter()
                .find(|record| record.decision.is_hold())
                .map(|record| record.decision)
                .unwrap_or(SelfGoalQueueApplyDecision::HeldForAppendPacket)
        };
        let explicit_apply_required =
            ready_count > 0 || records.iter().any(|record| record.explicit_apply_required);

        SelfGoalQueueApplyReport {
            schema_version: SELF_GOAL_QUEUE_APPLY_PLAN_SCHEMA_VERSION,
            trace_schema: SELF_GOAL_QUEUE_APPLY_PLAN_TRACE_SCHEMA,
            queue_preview_schema_version: queue_preview_report.schema_version,
            writer_gate_schema_version: writer_gate_report.schema_version,
            current_queue_digest,
            writer_gate_decision: writer_gate_report.decision,
            policy: self.policy,
            decision,
            record_count: records.len(),
            ready_count,
            held_count,
            rejected_count,
            records,
            record_lines,
            explicit_apply_required,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    fn evaluate_missing_record(
        &self,
        current_queue: &EvolutionGoalQueue,
        queue_preview_report: &SelfGoalQueuePreviewReport,
        writer_gate_report: &UnifiedWriterGateReport,
        writer_record: Option<&UnifiedWriterGateRecord>,
        current_queue_digest: &str,
        expected_writer_candidate_id: &str,
    ) -> SelfGoalQueueApplyRecord {
        let candidate_id = stable_redaction_digest([
            "self-goal-queue-apply-missing-record",
            current_queue_digest,
            queue_preview_report.existing_queue_digest.as_str(),
        ]);
        let mut reasons = self.source_rejection_reasons(
            current_queue,
            queue_preview_report,
            writer_gate_report,
            writer_record,
            current_queue_digest,
            expected_writer_candidate_id,
        );
        if reasons.is_empty() {
            reasons.push("append_packet_missing".to_owned());
        }
        let decision = if has_rejection_reason(&reasons) {
            SelfGoalQueueApplyDecision::Rejected
        } else {
            SelfGoalQueueApplyDecision::HeldForAppendPacket
        };
        apply_record_from_vec(
            candidate_id,
            "none".to_owned(),
            decision,
            reasons,
            current_queue_digest,
            None,
            None,
            None,
            writer_record,
            false,
        )
    }

    fn evaluate_record(
        &self,
        current_queue: &EvolutionGoalQueue,
        queue_preview_report: &SelfGoalQueuePreviewReport,
        writer_gate_report: &UnifiedWriterGateReport,
        writer_record: Option<&UnifiedWriterGateRecord>,
        preview_record: &SelfGoalQueuePreviewRecord,
        current_queue_digest: &str,
        expected_writer_candidate_id: &str,
    ) -> SelfGoalQueueApplyRecord {
        let mut rejection_reasons = self.source_rejection_reasons(
            current_queue,
            queue_preview_report,
            writer_gate_report,
            writer_record,
            current_queue_digest,
            expected_writer_candidate_id,
        );

        if preview_record.decision != SelfGoalQueuePreviewDecision::AppendPreview {
            let mut reasons = vec![
                format!("preview_decision:{}", preview_record.decision.as_str()),
                "append_packet_not_ready".to_owned(),
            ];
            reasons.append(&mut rejection_reasons);
            let decision = if has_rejection_reason(&reasons) {
                SelfGoalQueueApplyDecision::Rejected
            } else {
                SelfGoalQueueApplyDecision::HeldForAppendPacket
            };
            return apply_record_from_vec(
                preview_record.candidate_id.clone(),
                preview_record.proposed_goal_id.clone(),
                decision,
                reasons,
                current_queue_digest,
                preview_record.append_record_digest.clone(),
                preview_record.resulting_queue_preview_digest.clone(),
                preview_record.resulting_queue_preview_digest.clone(),
                writer_record,
                false,
            );
        }

        if preview_record.append_record_digest.is_none()
            || preview_record.resulting_queue_preview_digest.is_none()
            || preview_record.append_record_line.is_none()
        {
            rejection_reasons.push("append_packet_incomplete".to_owned());
        }

        if self.policy.require_single_append_packet
            && queue_preview_report.append_preview_count != 1
        {
            rejection_reasons.push(format!(
                "append_packet_count_invalid:{}",
                queue_preview_report.append_preview_count
            ));
        }

        if has_rejection_reason(&rejection_reasons) {
            return apply_record_from_vec(
                preview_record.candidate_id.clone(),
                preview_record.proposed_goal_id.clone(),
                SelfGoalQueueApplyDecision::Rejected,
                rejection_reasons,
                current_queue_digest,
                preview_record.append_record_digest.clone(),
                preview_record.resulting_queue_preview_digest.clone(),
                preview_record.resulting_queue_preview_digest.clone(),
                writer_record,
                false,
            );
        }

        if self.policy.reject_duplicate_goal
            && current_queue
                .goals
                .iter()
                .any(|goal| goal.stable_id == preview_record.proposed_goal_id)
        {
            return apply_record_from_vec(
                preview_record.candidate_id.clone(),
                preview_record.proposed_goal_id.clone(),
                SelfGoalQueueApplyDecision::HeldForDuplicateGoal,
                ["duplicate_goal_already_in_queue"]
                    .into_iter()
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>(),
                current_queue_digest,
                preview_record.append_record_digest.clone(),
                preview_record.resulting_queue_preview_digest.clone(),
                preview_record.resulting_queue_preview_digest.clone(),
                writer_record,
                false,
            );
        }

        let writer_reasons = self.writer_gate_hold_reasons(writer_gate_report, writer_record);
        if !writer_reasons.is_empty() {
            return apply_record_from_vec(
                preview_record.candidate_id.clone(),
                preview_record.proposed_goal_id.clone(),
                SelfGoalQueueApplyDecision::HeldForWriterGate,
                writer_reasons,
                current_queue_digest,
                preview_record.append_record_digest.clone(),
                preview_record.resulting_queue_preview_digest.clone(),
                preview_record.resulting_queue_preview_digest.clone(),
                writer_record,
                false,
            );
        }

        apply_record_from_vec(
            preview_record.candidate_id.clone(),
            preview_record.proposed_goal_id.clone(),
            SelfGoalQueueApplyDecision::ReadyForExplicitApply,
            ["explicit_apply_plan_ready"]
                .into_iter()
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>(),
            current_queue_digest,
            preview_record.append_record_digest.clone(),
            preview_record.resulting_queue_preview_digest.clone(),
            preview_record.resulting_queue_preview_digest.clone(),
            writer_record,
            true,
        )
    }

    fn source_rejection_reasons(
        &self,
        current_queue: &EvolutionGoalQueue,
        queue_preview_report: &SelfGoalQueuePreviewReport,
        writer_gate_report: &UnifiedWriterGateReport,
        writer_record: Option<&UnifiedWriterGateRecord>,
        current_queue_digest: &str,
        expected_writer_candidate_id: &str,
    ) -> Vec<String> {
        let mut reasons = Vec::new();
        if !self.policy.is_preview_safe() {
            reasons.push("apply_policy_not_preview_safe".to_owned());
        }
        if self.policy.require_preview_report_passed && !queue_preview_report.passed() {
            reasons.push("queue_preview_report_failed".to_owned());
        }
        if !current_queue.read_only || current_queue.write_allowed || current_queue.applied {
            reasons.push("current_queue_not_preview_only".to_owned());
        }
        if !queue_preview_report.read_only
            || queue_preview_report.write_allowed
            || queue_preview_report.applied
            || queue_preview_report
                .records
                .iter()
                .any(|record| !record.read_only || record.write_allowed || record.applied)
        {
            reasons.push("queue_preview_source_not_preview_only".to_owned());
        }
        if self.policy.require_current_queue_digest_match
            && queue_preview_report.existing_queue_digest != current_queue_digest
        {
            reasons.push("current_queue_digest_mismatch".to_owned());
        }
        if self.policy.require_digest_only_evidence
            && (!queue_preview_report.evidence_is_redacted()
                || contains_private_or_executable_marker(&queue_preview_report.summary_line())
                || contains_private_or_executable_marker(&writer_gate_report.summary_line())
                || writer_gate_report
                    .records
                    .iter()
                    .any(|record| contains_private_or_executable_marker(&record.summary_line())))
        {
            reasons.push("apply_plan_evidence_not_redacted".to_owned());
        }
        if writer_gate_report.applied
            || writer_gate_report
                .records
                .iter()
                .any(|record| record.applied)
        {
            reasons.push("writer_gate_already_applied".to_owned());
        }
        let queue_writer_records = writer_gate_report
            .records
            .iter()
            .filter(|record| record.domain == UnifiedWriterGateDomain::EvolutionGoalQueue)
            .count();
        if self.policy.require_evolution_goal_queue_domain && queue_writer_records == 0 {
            reasons.push("writer_gate_queue_record_missing".to_owned());
        }
        if queue_writer_records > 1 {
            reasons.push("writer_gate_queue_record_count_invalid".to_owned());
        }
        if let Some(record) = writer_record {
            if self.policy.require_matching_writer_candidate
                && record.candidate_id != expected_writer_candidate_id
            {
                reasons.push("writer_gate_candidate_mismatch".to_owned());
            }
            if !record
                .requested_writes
                .contains(&UnifiedWriterGateWriteScope::EvolutionGoalQueue)
            {
                reasons.push("writer_gate_write_scope_mismatch".to_owned());
            }
            if record.decision == UnifiedWriterGateDecision::ReadyForExplicitApply
                && (record.review_packet_count == 0
                    || record.evidence_id_count == 0
                    || record.rollback_anchor_count == 0
                    || record.content_digest_count == 0
                    || record.source_report_schema_count == 0)
            {
                reasons.push("writer_gate_ready_refs_incomplete".to_owned());
            }
        }
        if writer_gate_report.decision == UnifiedWriterGateDecision::Reject
            || writer_record
                .is_some_and(|record| record.decision == UnifiedWriterGateDecision::Reject)
        {
            reasons.push("writer_gate_rejected".to_owned());
        }
        reasons
    }

    fn writer_gate_hold_reasons(
        &self,
        writer_gate_report: &UnifiedWriterGateReport,
        writer_record: Option<&UnifiedWriterGateRecord>,
    ) -> Vec<String> {
        let Some(record) = writer_record else {
            return vec!["writer_gate_record_missing".to_owned()];
        };
        if !self.policy.require_writer_gate_ready {
            return Vec::new();
        }
        if writer_gate_report.decision == UnifiedWriterGateDecision::ReadyForExplicitApply
            && record.decision == UnifiedWriterGateDecision::ReadyForExplicitApply
            && writer_gate_report.durable_write_allowed
            && writer_gate_report.explicit_apply_required
            && record.durable_write_allowed
            && record.explicit_apply_required
        {
            Vec::new()
        } else {
            let mut reasons = vec![
                format!(
                    "writer_gate_decision:{}",
                    writer_gate_report.decision.as_str()
                ),
                format!("writer_record_decision:{}", record.decision.as_str()),
            ];
            reasons.extend(record.reason_codes.iter().cloned());
            reasons
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfGoalQueuePreviewGate {
    pub policy: SelfGoalQueuePreviewPolicy,
}

impl Default for SelfGoalQueuePreviewGate {
    fn default() -> Self {
        Self {
            policy: SelfGoalQueuePreviewPolicy::default(),
        }
    }
}

impl SelfGoalQueuePreviewGate {
    pub fn new(policy: SelfGoalQueuePreviewPolicy) -> Self {
        Self { policy }
    }

    pub fn evaluate(
        &self,
        current_queue: &EvolutionGoalQueue,
        proposal_report: &SelfGoalProposalReport,
        admission_report: &SelfGoalAdmissionReport,
    ) -> SelfGoalQueuePreviewReport {
        let existing_queue_digest = queue_digest(current_queue);
        let mut append_previews = 0;
        let mut records = Vec::with_capacity(admission_report.records.len());

        for admission_record in &admission_report.records {
            let candidate = proposal_report
                .candidates
                .iter()
                .find(|candidate| candidate.stable_id == admission_record.candidate_id);
            let record = self.evaluate_record(
                current_queue,
                admission_report,
                admission_record,
                candidate,
                &existing_queue_digest,
                &mut append_previews,
            );
            records.push(record);
        }

        let record_lines = records
            .iter()
            .map(SelfGoalQueuePreviewRecord::record_line)
            .collect::<Vec<_>>();
        let append_preview_count = records
            .iter()
            .filter(|record| record.decision == SelfGoalQueuePreviewDecision::AppendPreview)
            .count();
        let held_count = records
            .iter()
            .filter(|record| record.decision.is_hold())
            .count();
        let rejected_count = records
            .iter()
            .filter(|record| record.decision == SelfGoalQueuePreviewDecision::Rejected)
            .count();

        SelfGoalQueuePreviewReport {
            schema_version: SELF_GOAL_QUEUE_PREVIEW_SCHEMA_VERSION,
            trace_schema: SELF_GOAL_QUEUE_PREVIEW_TRACE_SCHEMA,
            admission_schema_version: admission_report.schema_version,
            existing_queue_digest,
            policy: self.policy,
            record_count: records.len(),
            append_preview_count,
            held_count,
            rejected_count,
            records,
            record_lines,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    fn evaluate_record(
        &self,
        current_queue: &EvolutionGoalQueue,
        admission_report: &SelfGoalAdmissionReport,
        admission_record: &SelfGoalAdmissionRecord,
        candidate: Option<&SelfGoalProposalCandidate>,
        existing_queue_digest: &str,
        append_previews: &mut usize,
    ) -> SelfGoalQueuePreviewRecord {
        let Some(candidate) = candidate else {
            return queue_preview_record(
                admission_record,
                SelfGoalQueuePreviewDecision::Rejected,
                ["missing_matching_proposal_candidate"],
                existing_queue_digest,
                None,
                None,
                None,
            );
        };

        if !self.policy.is_preview_safe() {
            return queue_preview_record(
                admission_record,
                SelfGoalQueuePreviewDecision::Rejected,
                ["queue_preview_policy_not_preview_safe"],
                existing_queue_digest,
                None,
                None,
                None,
            );
        }

        if self.policy.require_admission_report_passed && !admission_report.passed() {
            return queue_preview_record(
                admission_record,
                SelfGoalQueuePreviewDecision::Rejected,
                ["admission_report_failed"],
                existing_queue_digest,
                None,
                None,
                None,
            );
        }

        if self.policy.require_queue_preview_only
            && (!current_queue.read_only || current_queue.write_allowed || current_queue.applied)
        {
            return queue_preview_record(
                admission_record,
                SelfGoalQueuePreviewDecision::Rejected,
                ["current_queue_not_preview_only"],
                existing_queue_digest,
                None,
                None,
                None,
            );
        }

        if self.policy.require_digest_only_evidence
            && (!admission_record.evidence_is_redacted() || !candidate.evidence_is_redacted())
        {
            return queue_preview_record(
                admission_record,
                SelfGoalQueuePreviewDecision::Rejected,
                ["queue_preview_evidence_not_redacted"],
                existing_queue_digest,
                None,
                None,
                None,
            );
        }

        if self.policy.require_preview_admissible
            && admission_record.decision != SelfGoalAdmissionDecision::PreviewAdmissible
        {
            return queue_preview_record_from_vec(
                admission_record,
                SelfGoalQueuePreviewDecision::HeldForAdmissionGate,
                vec![
                    format!("admission_decision:{}", admission_record.decision.as_str()),
                    "waiting_for_preview_admissible".to_owned(),
                ],
                existing_queue_digest,
                None,
                None,
                None,
            );
        }

        if self.policy.reject_duplicate_goal
            && current_queue
                .goals
                .iter()
                .any(|goal| goal.stable_id == candidate.proposed_goal.stable_id)
        {
            return queue_preview_record(
                admission_record,
                SelfGoalQueuePreviewDecision::HeldForDuplicateGoal,
                ["duplicate_goal_already_in_queue"],
                existing_queue_digest,
                None,
                None,
                None,
            );
        }

        if *append_previews >= self.policy.max_append_records {
            return queue_preview_record(
                admission_record,
                SelfGoalQueuePreviewDecision::HeldForAppendLimit,
                ["queue_preview_append_limit_reached"],
                existing_queue_digest,
                None,
                None,
                None,
            );
        }

        *append_previews += 1;
        let append_record_line = candidate.proposed_goal.to_record_line();
        let append_record_digest = stable_redaction_digest([
            "self-goal-queue-append-record",
            admission_record.candidate_id.as_str(),
            admission_record.proposed_goal_id.as_str(),
            append_record_line.as_str(),
        ]);
        let resulting_queue =
            queue_with_appended_goal(current_queue, candidate.proposed_goal.clone());
        let resulting_queue_digest = queue_digest(&resulting_queue);

        queue_preview_record(
            admission_record,
            SelfGoalQueuePreviewDecision::AppendPreview,
            ["queue_append_preview_ready"],
            existing_queue_digest,
            Some(append_record_digest),
            Some(resulting_queue_digest),
            Some(append_record_line),
        )
    }
}

pub fn default_self_goal_proposal_report(queue: &EvolutionGoalQueue) -> SelfGoalProposalReport {
    SelfGoalProposalReport::from_queue(queue, SelfGoalProposalPolicy::default())
}

pub fn default_noiron_self_goal_proposal_report() -> SelfGoalProposalReport {
    default_self_goal_proposal_report(&default_noiron_pursuit_goal_queue())
}

pub fn default_self_goal_admission_report(
    proposal_report: &SelfGoalProposalReport,
    runs: &[EvolutionGoalRunEvidence],
) -> SelfGoalAdmissionReport {
    SelfGoalAdmissionGate::default().evaluate(proposal_report, runs)
}

pub fn default_noiron_self_goal_admission_report() -> SelfGoalAdmissionReport {
    default_self_goal_admission_report(&default_noiron_self_goal_proposal_report(), &[])
}

pub fn default_self_goal_queue_preview_report(
    current_queue: &EvolutionGoalQueue,
    proposal_report: &SelfGoalProposalReport,
    admission_report: &SelfGoalAdmissionReport,
) -> SelfGoalQueuePreviewReport {
    SelfGoalQueuePreviewGate::default().evaluate(current_queue, proposal_report, admission_report)
}

pub fn default_noiron_self_goal_queue_preview_report() -> SelfGoalQueuePreviewReport {
    default_self_goal_queue_preview_report(
        &default_noiron_pursuit_goal_queue(),
        &default_noiron_self_goal_proposal_report(),
        &default_noiron_self_goal_admission_report(),
    )
}

pub fn default_self_goal_queue_apply_report(
    current_queue: &EvolutionGoalQueue,
    queue_preview_report: &SelfGoalQueuePreviewReport,
    writer_gate_report: &UnifiedWriterGateReport,
) -> SelfGoalQueueApplyReport {
    SelfGoalQueueApplyPlanner::default().evaluate(
        current_queue,
        queue_preview_report,
        writer_gate_report,
    )
}

pub fn default_noiron_self_goal_queue_apply_report() -> SelfGoalQueueApplyReport {
    let current_queue = default_noiron_pursuit_goal_queue();
    let queue_preview_report = default_self_goal_queue_preview_report(
        &current_queue,
        &default_noiron_self_goal_proposal_report(),
        &default_noiron_self_goal_admission_report(),
    );
    let writer_gate_report =
        UnifiedWriterGate::new().evaluate([UnifiedWriterGateCandidate::self_goal_queue_preview(
            &queue_preview_report,
        )]);
    default_self_goal_queue_apply_report(&current_queue, &queue_preview_report, &writer_gate_report)
}

fn default_noiron_proposal_candidates(
    active_objective: Option<&str>,
) -> Vec<SelfGoalProposalCandidate> {
    let active = active_objective.unwrap_or_default();
    if active.contains("R98") {
        return vec![
            r98_memory_consolidation_candidate(10),
            self_goal_admission_gate_candidate(11),
            r97_benchmark_gate_candidate(20),
            r97_endpoint_cli_candidate(30),
        ];
    }

    vec![
        r97_endpoint_cli_candidate(10),
        r97_benchmark_gate_candidate(11),
        r98_memory_consolidation_candidate(20),
        self_goal_admission_gate_candidate(21),
    ]
}

fn r97_endpoint_cli_candidate(priority: u32) -> SelfGoalProposalCandidate {
    SelfGoalProposalCandidate::new(
        SelfGoalProposalSource::ActiveQueueGap,
        "R97",
        priority,
        "R97 endpoint and CLI runner wiring for coding service eval artifacts",
        [
            EvolutionGoalEvidenceKind::CargoCheck,
            EvolutionGoalEvidenceKind::FocusedTests,
            EvolutionGoalEvidenceKind::TraceSchemaGate,
            EvolutionGoalEvidenceKind::OperatorApproval,
        ],
        EvolutionGoalBudgetCap::new(2, 8, 48_000, 600_000),
        [
            "queue:active:R97",
            "roadmap:R97:#75/#19/#29",
            "source:coding_service_eval_runner_report",
            "lane:service-cli-artifacts",
        ],
        "finish the current active queue gap before advancing successors",
        "single-writer hold; later goals wait until R97 service artifacts pass review",
    )
}

fn r97_benchmark_gate_candidate(priority: u32) -> SelfGoalProposalCandidate {
    SelfGoalProposalCandidate::new(
        SelfGoalProposalSource::EvidenceGap,
        "R97",
        priority,
        "R97 benchmark gate feed for coding service eval runner",
        [
            EvolutionGoalEvidenceKind::CargoCheck,
            EvolutionGoalEvidenceKind::FocusedTests,
            EvolutionGoalEvidenceKind::BenchmarkGate,
            EvolutionGoalEvidenceKind::TraceSchemaGate,
            EvolutionGoalEvidenceKind::OperatorApproval,
        ],
        EvolutionGoalBudgetCap::new(2, 8, 52_000, 600_000),
        [
            "queue:evidence-gap:R97",
            "roadmap:R97:#29",
            "source:coding_service_eval_runner_report",
            "lane:benchmark-gate-feed",
        ],
        "turn offline coding-service observations into benchmark gate evidence",
        "benchmark feed stays isolated from memory, genome, and experiment-ledger writes",
    )
}

fn r98_memory_consolidation_candidate(priority: u32) -> SelfGoalProposalCandidate {
    SelfGoalProposalCandidate::new(
        SelfGoalProposalSource::RoadmapSuccessor,
        "R98",
        priority,
        "R98 self-evolving memory consolidation admission-preview feed",
        [
            EvolutionGoalEvidenceKind::CargoCheck,
            EvolutionGoalEvidenceKind::FocusedTests,
            EvolutionGoalEvidenceKind::ExperimentLedger,
            EvolutionGoalEvidenceKind::TraceSchemaGate,
            EvolutionGoalEvidenceKind::OperatorApproval,
        ],
        EvolutionGoalBudgetCap::new(3, 10, 64_000, 780_000),
        [
            "queue:successor:R98",
            "roadmap:R98:#76/#36/#42",
            "source:self_evolving_memory_consolidation_report",
            "lane:episode-heuristic-tool-reliability",
        ],
        "prepare the next memory-evolution objective after R97 gate evidence lands",
        "successor remains queued until active R97 evidence and approval unblock it",
    )
}

fn self_goal_admission_gate_candidate(priority: u32) -> SelfGoalProposalCandidate {
    SelfGoalProposalCandidate::new(
        SelfGoalProposalSource::GovernanceGate,
        "R97/R98",
        priority,
        "Self-goal proposal admission gate before autonomous execution",
        [
            EvolutionGoalEvidenceKind::CargoCheck,
            EvolutionGoalEvidenceKind::FocusedTests,
            EvolutionGoalEvidenceKind::ExperimentLedger,
            EvolutionGoalEvidenceKind::TraceSchemaGate,
            EvolutionGoalEvidenceKind::OperatorApproval,
        ],
        EvolutionGoalBudgetCap::new(2, 6, 36_000, 420_000),
        [
            "queue:governance:self-goal-proposal",
            "roadmap:R97/R98",
            "source:evolution_goal_queue",
            "lane:autonomy-admission",
        ],
        "prove proposed goals can be admitted only after deterministic gates and approval",
        "proposal engine cannot create branches, write adaptive state, or apply queue changes",
    )
}

fn safe_text(value: String) -> String {
    if contains_private_or_executable_marker(&value) {
        stable_redaction_digest(["redacted-text", value.trim()])
    } else {
        value.trim().to_owned()
    }
}

fn admission_record<'a>(
    candidate: &SelfGoalProposalCandidate,
    decision: SelfGoalAdmissionDecision,
    reason_codes: impl IntoIterator<Item = &'a str>,
    evidence_digests: Vec<String>,
    queue_insert_preview_digest: Option<String>,
) -> SelfGoalAdmissionRecord {
    admission_record_from_vec(
        candidate,
        decision,
        reason_codes
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>(),
        evidence_digests,
        queue_insert_preview_digest,
    )
}

fn admission_record_from_vec(
    candidate: &SelfGoalProposalCandidate,
    decision: SelfGoalAdmissionDecision,
    reason_codes: Vec<String>,
    evidence_digests: Vec<String>,
    queue_insert_preview_digest: Option<String>,
) -> SelfGoalAdmissionRecord {
    let admitted_goal_record_line = if decision == SelfGoalAdmissionDecision::PreviewAdmissible {
        Some(candidate.proposed_goal.to_record_line())
    } else {
        None
    };

    SelfGoalAdmissionRecord {
        schema_version: SELF_GOAL_ADMISSION_SCHEMA_VERSION,
        candidate_id: candidate.stable_id.clone(),
        proposed_goal_id: candidate.proposed_goal.stable_id.clone(),
        source: candidate.source,
        target_release: candidate.target_release.clone(),
        decision,
        reason_codes,
        evidence_digests,
        queue_insert_preview_digest,
        admitted_goal_record_line,
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

fn queue_preview_record<'a>(
    admission_record: &SelfGoalAdmissionRecord,
    decision: SelfGoalQueuePreviewDecision,
    reason_codes: impl IntoIterator<Item = &'a str>,
    existing_queue_digest: &str,
    append_record_digest: Option<String>,
    resulting_queue_preview_digest: Option<String>,
    append_record_line: Option<String>,
) -> SelfGoalQueuePreviewRecord {
    queue_preview_record_from_vec(
        admission_record,
        decision,
        reason_codes
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>(),
        existing_queue_digest,
        append_record_digest,
        resulting_queue_preview_digest,
        append_record_line,
    )
}

fn queue_preview_record_from_vec(
    admission_record: &SelfGoalAdmissionRecord,
    decision: SelfGoalQueuePreviewDecision,
    reason_codes: Vec<String>,
    existing_queue_digest: &str,
    append_record_digest: Option<String>,
    resulting_queue_preview_digest: Option<String>,
    append_record_line: Option<String>,
) -> SelfGoalQueuePreviewRecord {
    SelfGoalQueuePreviewRecord {
        schema_version: SELF_GOAL_QUEUE_PREVIEW_SCHEMA_VERSION,
        candidate_id: admission_record.candidate_id.clone(),
        proposed_goal_id: admission_record.proposed_goal_id.clone(),
        decision,
        reason_codes,
        existing_queue_digest: existing_queue_digest.to_owned(),
        append_record_digest,
        resulting_queue_preview_digest,
        append_record_line,
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_record_from_vec(
    candidate_id: String,
    proposed_goal_id: String,
    decision: SelfGoalQueueApplyDecision,
    reason_codes: Vec<String>,
    current_queue_digest: &str,
    append_record_digest: Option<String>,
    resulting_queue_preview_digest: Option<String>,
    expected_resulting_queue_digest: Option<String>,
    writer_record: Option<&UnifiedWriterGateRecord>,
    explicit_apply_required: bool,
) -> SelfGoalQueueApplyRecord {
    let writer_gate_candidate_id = writer_record.map(|record| record.candidate_id.clone());
    let writer_gate_refs_digest = writer_record.map(|record| record.refs_digest.clone());
    let append_digest = append_record_digest.as_deref().unwrap_or("none");
    let resulting_digest = resulting_queue_preview_digest.as_deref().unwrap_or("none");
    let expected_digest = expected_resulting_queue_digest.as_deref().unwrap_or("none");
    let writer_candidate = writer_gate_candidate_id.as_deref().unwrap_or("none");
    let writer_refs = writer_gate_refs_digest.as_deref().unwrap_or("none");
    let reasons = reason_codes.join("|");
    let apply_plan_digest = stable_redaction_digest([
        SELF_GOAL_QUEUE_APPLY_PLAN_SCHEMA_VERSION,
        candidate_id.as_str(),
        proposed_goal_id.as_str(),
        decision.as_str(),
        reasons.as_str(),
        current_queue_digest,
        append_digest,
        resulting_digest,
        expected_digest,
        writer_candidate,
        writer_refs,
        bool_to_field(explicit_apply_required),
    ]);

    SelfGoalQueueApplyRecord {
        schema_version: SELF_GOAL_QUEUE_APPLY_PLAN_SCHEMA_VERSION,
        candidate_id,
        proposed_goal_id,
        decision,
        reason_codes,
        current_queue_digest: current_queue_digest.to_owned(),
        rollback_anchor_digest: current_queue_digest.to_owned(),
        append_record_digest,
        resulting_queue_preview_digest,
        expected_resulting_queue_digest,
        writer_gate_candidate_id,
        writer_gate_refs_digest,
        apply_plan_digest,
        explicit_apply_required,
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

fn has_rejection_reason(reasons: &[String]) -> bool {
    reasons.iter().any(|reason| {
        matches!(
            reason.as_str(),
            "apply_policy_not_preview_safe"
                | "queue_preview_report_failed"
                | "current_queue_not_preview_only"
                | "queue_preview_source_not_preview_only"
                | "current_queue_digest_mismatch"
                | "apply_plan_evidence_not_redacted"
                | "writer_gate_already_applied"
                | "writer_gate_queue_record_missing"
                | "writer_gate_queue_record_count_invalid"
                | "writer_gate_candidate_mismatch"
                | "writer_gate_write_scope_mismatch"
                | "writer_gate_ready_refs_incomplete"
                | "writer_gate_rejected"
                | "append_packet_incomplete"
        ) || reason.starts_with("append_packet_count_invalid:")
    })
}

fn matching_queue_writer_record<'a>(
    writer_gate_report: &'a UnifiedWriterGateReport,
    expected_writer_candidate_id: &str,
) -> Option<&'a UnifiedWriterGateRecord> {
    writer_gate_report
        .records
        .iter()
        .find(|record| {
            record.domain == UnifiedWriterGateDomain::EvolutionGoalQueue
                && record.candidate_id == expected_writer_candidate_id
        })
        .or_else(|| {
            let mut records = writer_gate_report
                .records
                .iter()
                .filter(|record| record.domain == UnifiedWriterGateDomain::EvolutionGoalQueue);
            let first = records.next()?;
            if records.next().is_none() {
                Some(first)
            } else {
                None
            }
        })
}

fn queue_writer_candidate_id(report: &SelfGoalQueuePreviewReport) -> String {
    let record_count = report.record_count.to_string();
    let append_preview_count = report.append_preview_count.to_string();
    stable_redaction_digest([
        "self-goal-queue-preview",
        report.existing_queue_digest.as_str(),
        record_count.as_str(),
        append_preview_count.as_str(),
    ])
}

fn queue_digest(queue: &EvolutionGoalQueue) -> String {
    let lines = queue
        .goals
        .iter()
        .map(EvolutionGoal::to_record_line)
        .collect::<Vec<_>>();
    let mut parts = Vec::with_capacity(lines.len() + 4);
    parts.push(queue.schema_version);
    parts.push(bool_to_field(queue.read_only));
    parts.push(bool_to_field(queue.write_allowed));
    parts.push(bool_to_field(queue.applied));
    parts.extend(lines.iter().map(String::as_str));
    stable_redaction_digest(parts)
}

fn queue_with_appended_goal(queue: &EvolutionGoalQueue, goal: EvolutionGoal) -> EvolutionGoalQueue {
    let mut goals = queue.goals.clone();
    goals.push(goal);
    EvolutionGoalQueue::new(goals)
}

fn evidence_kind_list(values: &[EvolutionGoalEvidenceKind]) -> String {
    values
        .iter()
        .map(|kind| kind.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn budget_cap_field(cap: EvolutionGoalBudgetCap) -> String {
    format!(
        "attempts={};steps={};tokens={};runtime_ms={}",
        cap.max_attempts, cap.max_steps, cap.max_tokens, cap.max_runtime_ms
    )
}

fn approval_gate_field(gate: &EvolutionGoalApprovalGate) -> String {
    format!(
        "maintainer={};operator={};evidence={}",
        gate.maintainer_required, gate.operator_required, gate.approval_evidence_required
    )
}

fn stop_condition_field(condition: &EvolutionGoalStopCondition) -> String {
    format!(
        "success={};budget={};rollback={};approval_hold={}",
        condition.success_stops_goal,
        condition.budget_exhaustion_stops_goal,
        condition.rollback_stops_goal,
        condition.approval_hold_stops_queue
    )
}

fn rollback_condition_field(condition: &EvolutionGoalRollbackCondition) -> String {
    format!(
        "required_evidence={};trace_schema={};explicit_signal={}",
        condition.rollback_on_failed_required_evidence,
        condition.rollback_on_trace_schema_failure,
        condition.rollback_on_explicit_signal
    )
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('|', "\\p")
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn bool_to_field(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evolution_goal::{EvolutionGoalBudgetUsage, EvolutionGoalEvidence};

    #[test]
    fn default_self_goal_proposal_report_is_preview_only_and_passes() {
        let report = default_noiron_self_goal_proposal_report();

        assert_eq!(report.schema_version, SELF_GOAL_PROPOSAL_SCHEMA_VERSION);
        assert_eq!(report.trace_schema, SELF_GOAL_PROPOSAL_TRACE_SCHEMA);
        assert_eq!(report.candidate_count, 4);
        assert!(report.r97_candidate_count >= 2);
        assert!(report.r98_candidate_count >= 1);
        assert_eq!(report.admission_gate_candidate_count, 1);
        assert!(report.passed(), "{}", report.summary_line());
        assert!(report.is_preview_only());
        assert!(report.evidence_is_redacted());
    }

    #[test]
    fn self_goal_proposals_are_deterministic() {
        let first = default_noiron_self_goal_proposal_report();
        let second = default_noiron_self_goal_proposal_report();

        assert_eq!(first.candidate_record_lines, second.candidate_record_lines);
        assert_eq!(
            first
                .candidates
                .iter()
                .map(|candidate| candidate.stable_id.clone())
                .collect::<Vec<_>>(),
            second
                .candidates
                .iter()
                .map(|candidate| candidate.stable_id.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn proposed_goals_include_gates_budget_rollback_and_approval() {
        let report = default_noiron_self_goal_proposal_report();

        for candidate in &report.candidates {
            assert!(candidate.has_required_governance(), "{candidate:?}");
            assert!(
                candidate
                    .proposed_goal
                    .success_gate
                    .required_evidence
                    .contains(&EvolutionGoalEvidenceKind::OperatorApproval)
            );
            assert!(candidate.proposed_goal.budget_cap.max_tokens <= 64_000);
            assert!(candidate.proposed_goal.read_only);
            assert!(!candidate.proposed_goal.write_allowed);
            assert!(!candidate.proposed_goal.applied);
        }
    }

    #[test]
    fn proposal_evidence_is_digest_only_and_not_applied() {
        let report = default_noiron_self_goal_proposal_report();

        for candidate in &report.candidates {
            assert!(candidate.evidence_digest.starts_with("redaction-digest:"));
            assert!(candidate.provenance_digest.starts_with("redaction-digest:"));
            assert!(candidate.evidence_is_redacted());
            assert!(candidate.is_preview_only());
            assert!(!candidate.record_line().contains("fixture prompt"));
            assert!(!contains_private_or_executable_marker(
                &candidate.record_line()
            ));
        }
    }

    #[test]
    fn proposals_align_to_current_r97_r98_roadmap() {
        let report = default_noiron_self_goal_proposal_report();
        let objectives = report
            .candidates
            .iter()
            .map(|candidate| candidate.proposed_goal.objective.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(objectives.contains("R97"));
        assert!(objectives.contains("R98"));
        assert!(objectives.contains("Self-goal proposal admission gate"));
        for unexpected in ["poetry", "image generation", "trading bot"] {
            assert!(!objectives.contains(unexpected));
        }
    }

    #[test]
    fn active_goal_objective_is_digest_only_when_queue_contains_private_text() {
        let private_goal = EvolutionGoal::new(
            10,
            "prompt: private prompt should be hashed",
            EvolutionGoalSuccessGate::new([EvolutionGoalEvidenceKind::CargoCheck]),
            ["issue:#privacy", "prompt: private prompt should be hashed"],
        );
        let queue = EvolutionGoalQueue::new(vec![private_goal]);
        let report = default_self_goal_proposal_report(&queue);

        assert!(
            report
                .active_goal_objective_digest
                .as_ref()
                .is_some_and(|digest| digest.starts_with("redaction-digest:"))
        );
        assert!(report.evidence_is_redacted());
        assert!(
            report
                .candidate_record_lines
                .iter()
                .all(|line| !line.contains("private prompt"))
        );
    }

    #[test]
    fn default_self_goal_admission_report_holds_while_current_queue_is_active() {
        let report = default_noiron_self_goal_admission_report();

        assert_eq!(report.schema_version, SELF_GOAL_ADMISSION_SCHEMA_VERSION);
        assert_eq!(report.trace_schema, SELF_GOAL_ADMISSION_TRACE_SCHEMA);
        assert_eq!(report.preview_admissible_count, 0);
        assert_eq!(report.held_count, report.record_count);
        assert_eq!(report.rejected_count, 0);
        assert!(report.passed(), "{}", report.summary_line());
        assert!(report.is_preview_only());
        assert!(report.evidence_is_redacted());
        assert!(
            report
                .records
                .iter()
                .all(|record| record.decision == SelfGoalAdmissionDecision::HeldForPriorGoal)
        );
    }

    #[test]
    fn admission_gate_previews_only_one_goal_after_required_evidence_passes() {
        let proposal = proposal_report_without_active_queue();
        let runs = proposal
            .candidates
            .iter()
            .take(2)
            .map(|candidate| passing_run_for_candidate(candidate).with_approval())
            .collect::<Vec<_>>();

        let report = default_self_goal_admission_report(&proposal, &runs);

        assert_eq!(report.preview_admissible_count, 1);
        assert_eq!(
            report.records[0].decision,
            SelfGoalAdmissionDecision::PreviewAdmissible
        );
        assert_eq!(
            report.records[1].decision,
            SelfGoalAdmissionDecision::HeldForAdmissionLimit
        );
        assert!(report.records[0].admitted_goal_record_line.is_some());
        assert!(report.records[0].queue_insert_preview_digest.is_some());
        assert!(report.passed(), "{}", report.summary_line());
        assert!(report.is_preview_only());
        assert!(!report.write_allowed);
        assert!(!report.applied);
    }

    #[test]
    fn admission_gate_holds_when_operator_approval_is_missing() {
        let proposal = proposal_report_without_active_queue();
        let run = passing_run_for_candidate(&proposal.candidates[0]);

        let report = default_self_goal_admission_report(&proposal, &[run]);

        assert_eq!(report.preview_admissible_count, 0);
        assert_eq!(
            report.records[0].decision,
            SelfGoalAdmissionDecision::HeldForApproval
        );
        assert!(
            report.records[0]
                .reason_codes
                .contains(&"approval_required_before_promotion".to_owned())
        );
        assert!(report.passed(), "{}", report.summary_line());
    }

    #[test]
    fn admission_gate_rejects_rollback_or_budget_exhausted_candidates() {
        let proposal = proposal_report_without_active_queue();
        let rollback_run = passing_run_for_candidate(&proposal.candidates[0])
            .with_approval()
            .with_rollback_signal();
        let budget_run = passing_run_for_candidate(&proposal.candidates[1])
            .with_approval()
            .with_budget_usage(EvolutionGoalBudgetUsage::new(9, 1, 1, 1));

        let report = default_self_goal_admission_report(&proposal, &[rollback_run, budget_run]);

        assert_eq!(report.rejected_count, 2);
        assert_eq!(
            report.records[0].decision,
            SelfGoalAdmissionDecision::Rejected
        );
        assert_eq!(
            report.records[1].decision,
            SelfGoalAdmissionDecision::Rejected
        );
        assert!(
            report.records[0]
                .reason_codes
                .contains(&"rollback_signal_triggered".to_owned())
        );
        assert!(
            report.records[1]
                .reason_codes
                .contains(&"budget_cap_exhausted".to_owned())
        );
        assert!(!report.passed());
        assert!(report.is_preview_only());
    }

    #[test]
    fn unsafe_admission_policy_is_rejected_without_writing_queue() {
        let proposal = proposal_report_without_active_queue();
        let run = passing_run_for_candidate(&proposal.candidates[0]).with_approval();
        let gate = SelfGoalAdmissionGate::new(SelfGoalAdmissionPolicy {
            allow_queue_write: true,
            ..SelfGoalAdmissionPolicy::default()
        });

        let report = gate.evaluate(&proposal, &[run]);

        assert_eq!(report.rejected_count, report.record_count);
        assert!(report.records.iter().all(|record| {
            record
                .reason_codes
                .contains(&"admission_policy_not_preview_safe".to_owned())
        }));
        assert!(!report.passed());
        assert!(report.is_preview_only());
        assert!(!report.write_allowed);
    }

    #[test]
    fn admission_record_lines_are_digest_only() {
        let proposal = proposal_report_without_active_queue();
        let runs = proposal
            .candidates
            .iter()
            .map(|candidate| passing_run_for_candidate(candidate).with_approval())
            .collect::<Vec<_>>();

        let report = default_self_goal_admission_report(&proposal, &runs);

        assert!(report.evidence_is_redacted());
        assert!(
            report
                .record_lines
                .iter()
                .all(|line| line.contains("redaction-digest:"))
        );
        assert!(
            report
                .record_lines
                .iter()
                .all(|line| !contains_private_or_executable_marker(line))
        );
        assert!(
            report
                .records
                .iter()
                .all(SelfGoalAdmissionRecord::is_preview_only)
        );
    }

    #[test]
    fn default_queue_preview_holds_while_admission_gate_is_held() {
        let report = default_noiron_self_goal_queue_preview_report();

        assert_eq!(
            report.schema_version,
            SELF_GOAL_QUEUE_PREVIEW_SCHEMA_VERSION
        );
        assert_eq!(report.trace_schema, SELF_GOAL_QUEUE_PREVIEW_TRACE_SCHEMA);
        assert_eq!(report.append_preview_count, 0);
        assert_eq!(report.held_count, report.record_count);
        assert_eq!(report.rejected_count, 0);
        assert!(report.passed(), "{}", report.summary_line());
        assert!(report.is_preview_only());
        assert!(report.evidence_is_redacted());
        assert!(report.records.iter().all(|record| {
            record.decision == SelfGoalQueuePreviewDecision::HeldForAdmissionGate
        }));
    }

    #[test]
    fn queue_preview_emits_append_packet_for_one_preview_admissible_goal() {
        let queue = EvolutionGoalQueue::new(Vec::new());
        let proposal = proposal_report_without_active_queue();
        let runs = [passing_run_for_candidate(&proposal.candidates[0]).with_approval()];
        let admission = default_self_goal_admission_report(&proposal, &runs);

        let report = default_self_goal_queue_preview_report(&queue, &proposal, &admission);

        assert_eq!(report.append_preview_count, 1);
        assert_eq!(report.held_count, report.record_count - 1);
        assert_eq!(report.rejected_count, 0);
        assert_eq!(
            report.records[0].decision,
            SelfGoalQueuePreviewDecision::AppendPreview
        );
        assert!(report.records[0].append_record_line.is_some());
        assert!(report.records[0].append_record_digest.is_some());
        assert!(report.records[0].resulting_queue_preview_digest.is_some());
        assert!(report.records[0].is_preview_only());
        assert!(report.records[0].evidence_is_redacted());
        assert!(report.passed(), "{}", report.summary_line());
        assert!(!report.write_allowed);
        assert!(!report.applied);
    }

    #[test]
    fn queue_preview_holds_duplicate_goal_without_appending() {
        let proposal = proposal_report_without_active_queue();
        let duplicate_goal = proposal.candidates[0].proposed_goal.clone();
        let queue = EvolutionGoalQueue::new(vec![duplicate_goal]);
        let runs = [passing_run_for_candidate(&proposal.candidates[0]).with_approval()];
        let admission = default_self_goal_admission_report(&proposal, &runs);

        let report = default_self_goal_queue_preview_report(&queue, &proposal, &admission);

        assert_eq!(report.append_preview_count, 0);
        assert_eq!(
            report.records[0].decision,
            SelfGoalQueuePreviewDecision::HeldForDuplicateGoal
        );
        assert!(
            report.records[0]
                .reason_codes
                .contains(&"duplicate_goal_already_in_queue".to_owned())
        );
        assert!(report.passed(), "{}", report.summary_line());
        assert!(report.is_preview_only());
    }

    #[test]
    fn queue_preview_holds_second_append_after_limit() {
        let queue = EvolutionGoalQueue::new(Vec::new());
        let proposal = proposal_report_without_active_queue();
        let runs = proposal
            .candidates
            .iter()
            .take(2)
            .map(|candidate| passing_run_for_candidate(candidate).with_approval())
            .collect::<Vec<_>>();
        let admission = SelfGoalAdmissionGate::new(SelfGoalAdmissionPolicy {
            max_preview_admissions: 2,
            ..SelfGoalAdmissionPolicy::default()
        })
        .evaluate(&proposal, &runs);

        let report = default_self_goal_queue_preview_report(&queue, &proposal, &admission);

        assert_eq!(admission.preview_admissible_count, 2);
        assert_eq!(report.append_preview_count, 1);
        assert_eq!(
            report.records[0].decision,
            SelfGoalQueuePreviewDecision::AppendPreview
        );
        assert_eq!(
            report.records[1].decision,
            SelfGoalQueuePreviewDecision::HeldForAppendLimit
        );
        assert!(
            report.records[1]
                .reason_codes
                .contains(&"queue_preview_append_limit_reached".to_owned())
        );
        assert!(report.passed(), "{}", report.summary_line());
    }

    #[test]
    fn queue_preview_rejects_failed_admission_report() {
        let queue = EvolutionGoalQueue::new(Vec::new());
        let proposal = proposal_report_without_active_queue();
        let rollback_run = passing_run_for_candidate(&proposal.candidates[0])
            .with_approval()
            .with_rollback_signal();
        let admission = default_self_goal_admission_report(&proposal, &[rollback_run]);

        let report = default_self_goal_queue_preview_report(&queue, &proposal, &admission);

        assert_eq!(admission.rejected_count, 1);
        assert!(admission.rejected_count > 0);
        assert_eq!(report.rejected_count, report.record_count);
        assert!(report.records.iter().all(|record| {
            record
                .reason_codes
                .contains(&"admission_report_failed".to_owned())
        }));
        assert!(!report.passed());
        assert!(report.is_preview_only());
    }

    #[test]
    fn unsafe_queue_preview_policy_is_rejected_without_write_flags() {
        let queue = EvolutionGoalQueue::new(Vec::new());
        let proposal = proposal_report_without_active_queue();
        let runs = [passing_run_for_candidate(&proposal.candidates[0]).with_approval()];
        let admission = default_self_goal_admission_report(&proposal, &runs);
        let gate = SelfGoalQueuePreviewGate::new(SelfGoalQueuePreviewPolicy {
            allow_queue_write: true,
            ..SelfGoalQueuePreviewPolicy::default()
        });

        let report = gate.evaluate(&queue, &proposal, &admission);

        assert_eq!(report.rejected_count, report.record_count);
        assert!(report.records.iter().all(|record| {
            record
                .reason_codes
                .contains(&"queue_preview_policy_not_preview_safe".to_owned())
        }));
        assert!(!report.passed());
        assert!(report.is_preview_only());
        assert!(!report.write_allowed);
    }

    #[test]
    fn queue_preview_record_lines_are_digest_only() {
        let queue = EvolutionGoalQueue::new(Vec::new());
        let proposal = proposal_report_without_active_queue();
        let runs = [passing_run_for_candidate(&proposal.candidates[0]).with_approval()];
        let admission = default_self_goal_admission_report(&proposal, &runs);

        let report = default_self_goal_queue_preview_report(&queue, &proposal, &admission);

        assert!(report.evidence_is_redacted());
        assert!(
            report
                .record_lines
                .iter()
                .all(|line| line.contains("redaction-digest:"))
        );
        assert!(
            report
                .record_lines
                .iter()
                .all(|line| !contains_private_or_executable_marker(line))
        );
        assert!(
            report
                .records
                .iter()
                .all(SelfGoalQueuePreviewRecord::is_preview_only)
        );
    }

    #[test]
    fn queue_apply_plan_holds_behind_default_writer_gate() {
        let (queue, preview) = queue_preview_with_append();
        let writer_gate = writer_gate_report_for_preview(&preview, false);

        let report = default_self_goal_queue_apply_report(&queue, &preview, &writer_gate);

        assert_eq!(
            report.decision,
            SelfGoalQueueApplyDecision::HeldForWriterGate
        );
        assert_eq!(report.ready_count, 0);
        assert_eq!(report.held_count, 1);
        assert_eq!(report.rejected_count, 0);
        assert!(report.passed(), "{}", report.summary_line());
        assert!(report.is_preview_only());
        assert!(!report.explicit_apply_required);
        assert!(!report.write_allowed);
        assert!(!report.applied);
        assert!(
            report.records[0]
                .reason_codes
                .contains(&"durable_writes_disabled".to_owned())
        );
    }

    #[test]
    fn queue_apply_plan_reaches_ready_without_applying_when_writer_gate_is_ready() {
        let (queue, preview) = queue_preview_with_append();
        let writer_gate = writer_gate_report_for_preview(&preview, true);

        let report = default_self_goal_queue_apply_report(&queue, &preview, &writer_gate);

        assert_eq!(
            report.decision,
            SelfGoalQueueApplyDecision::ReadyForExplicitApply
        );
        assert_eq!(report.ready_count, 1);
        assert_eq!(report.held_count, 0);
        assert_eq!(report.rejected_count, 0);
        assert!(report.explicit_apply_required);
        assert!(report.passed(), "{}", report.summary_line());
        assert!(report.is_preview_only());
        assert!(!report.write_allowed);
        assert!(!report.applied);
        assert_eq!(
            report.records[0].append_record_digest,
            preview.records[0].append_record_digest
        );
        assert_eq!(
            report.records[0].expected_resulting_queue_digest,
            preview.records[0].resulting_queue_preview_digest
        );
        assert_eq!(
            report.records[0].rollback_anchor_digest,
            report.current_queue_digest
        );
    }

    #[test]
    fn default_noiron_queue_apply_plan_holds_without_append_packet() {
        let report = default_noiron_self_goal_queue_apply_report();

        assert_eq!(
            report.decision,
            SelfGoalQueueApplyDecision::HeldForAppendPacket
        );
        assert_eq!(report.ready_count, 0);
        assert_eq!(report.rejected_count, 0);
        assert!(report.held_count > 0);
        assert!(report.passed(), "{}", report.summary_line());
        assert!(report.records.iter().all(|record| {
            record
                .reason_codes
                .contains(&"append_packet_not_ready".to_owned())
        }));
    }

    #[test]
    fn queue_apply_plan_rejects_stale_preview_after_queue_changes() {
        let (queue, preview) = queue_preview_with_append();
        let writer_gate = writer_gate_report_for_preview(&preview, true);
        let stale_queue = queue_with_appended_goal(&queue, previewed_goal(&preview));

        let report = default_self_goal_queue_apply_report(&stale_queue, &preview, &writer_gate);

        assert_eq!(report.decision, SelfGoalQueueApplyDecision::Rejected);
        assert_eq!(report.rejected_count, 1);
        assert!(!report.passed());
        assert!(
            report.records[0]
                .reason_codes
                .contains(&"current_queue_digest_mismatch".to_owned())
        );
    }

    #[test]
    fn queue_apply_plan_rejects_source_write_flags() {
        let (queue, mut preview) = queue_preview_with_append();
        let writer_gate = writer_gate_report_for_preview(&preview, true);
        preview.write_allowed = true;

        let report = default_self_goal_queue_apply_report(&queue, &preview, &writer_gate);

        assert_eq!(report.decision, SelfGoalQueueApplyDecision::Rejected);
        assert_eq!(report.rejected_count, 1);
        assert!(
            report.records[0]
                .reason_codes
                .contains(&"queue_preview_source_not_preview_only".to_owned())
        );
        assert!(report.is_preview_only());
    }

    #[test]
    fn queue_apply_plan_records_are_deterministic_and_digest_only() {
        let (queue, preview) = queue_preview_with_append();
        let writer_gate = writer_gate_report_for_preview(&preview, true);

        let first = default_self_goal_queue_apply_report(&queue, &preview, &writer_gate);
        let second = default_self_goal_queue_apply_report(&queue, &preview, &writer_gate);

        assert_eq!(first.record_lines, second.record_lines);
        assert!(first.evidence_is_redacted());
        assert!(
            first
                .record_lines
                .iter()
                .all(|line| line.contains("redaction-digest:"))
        );
        assert!(
            first
                .record_lines
                .iter()
                .all(|line| !contains_private_or_executable_marker(line))
        );
        assert!(
            first
                .records
                .iter()
                .all(SelfGoalQueueApplyRecord::is_preview_only)
        );
    }

    #[test]
    fn queue_apply_plan_json_line_is_deterministic_and_digest_only() {
        let (queue, preview) = queue_preview_with_append();
        let writer_gate = writer_gate_report_for_preview(&preview, false);

        let first = default_self_goal_queue_apply_report(&queue, &preview, &writer_gate);
        let second = default_self_goal_queue_apply_report(&queue, &preview, &writer_gate);
        let line = first.json_line();

        assert_eq!(line, second.json_line());
        assert!(line.contains("\"schema\":\"rust-norion-self-goal-queue-apply-plan-v1\""));
        assert!(line.contains("\"plan_schema\":\"self_goal_queue_apply_plan_v1\""));
        assert!(line.contains("\"records\":1"));
        assert!(line.contains("\"held_records\":1"));
        assert!(line.contains("\"ready_records\":0"));
        assert!(line.contains("\"current_queue_digest\":\"redaction-digest:"));
        assert!(line.contains("\"apply_plan_digest\":\"redaction-digest:"));
        assert!(!line.contains("\"records\":["));
        assert!(!line.contains("\"record_lines\":["));
        assert!(!contains_private_or_executable_marker(&line));
    }

    fn proposal_report_without_active_queue() -> SelfGoalProposalReport {
        default_self_goal_proposal_report(&EvolutionGoalQueue::new(Vec::new()))
    }

    fn queue_preview_with_append() -> (EvolutionGoalQueue, SelfGoalQueuePreviewReport) {
        let queue = EvolutionGoalQueue::new(Vec::new());
        let proposal = proposal_report_without_active_queue();
        let runs = [passing_run_for_candidate(&proposal.candidates[0]).with_approval()];
        let admission = default_self_goal_admission_report(&proposal, &runs);
        let preview = default_self_goal_queue_preview_report(&queue, &proposal, &admission);

        assert_eq!(preview.append_preview_count, 1);
        (queue, preview)
    }

    fn writer_gate_report_for_preview(
        preview: &SelfGoalQueuePreviewReport,
        durable_writes_enabled: bool,
    ) -> UnifiedWriterGateReport {
        let policy = crate::writer_gate::UnifiedWriterGatePolicy {
            durable_writes_enabled,
            ..crate::writer_gate::UnifiedWriterGatePolicy::default()
        };
        UnifiedWriterGate::new()
            .with_policy(policy)
            .evaluate([UnifiedWriterGateCandidate::self_goal_queue_preview(preview)])
    }

    fn previewed_goal(preview: &SelfGoalQueuePreviewReport) -> EvolutionGoal {
        let goal_id = preview.records[0].proposed_goal_id.as_str();
        let proposal = proposal_report_without_active_queue();
        proposal
            .candidates
            .into_iter()
            .find(|candidate| candidate.proposed_goal.stable_id == goal_id)
            .map(|candidate| candidate.proposed_goal)
            .expect("previewed goal must come from proposal fixture")
    }

    fn passing_run_for_candidate(
        candidate: &SelfGoalProposalCandidate,
    ) -> EvolutionGoalRunEvidence {
        let evidence = candidate
            .proposed_goal
            .success_gate
            .required_evidence
            .iter()
            .map(|kind| match kind {
                EvolutionGoalEvidenceKind::CargoCheck => EvolutionGoalEvidence::cargo_check(true),
                EvolutionGoalEvidenceKind::FocusedTests => {
                    EvolutionGoalEvidence::focused_tests(true, 3, 0)
                }
                EvolutionGoalEvidenceKind::BenchmarkGate => {
                    EvolutionGoalEvidence::benchmark_gate(true)
                }
                EvolutionGoalEvidenceKind::TraceSchemaGate => {
                    EvolutionGoalEvidence::trace_schema_gate(true)
                }
                EvolutionGoalEvidenceKind::ExperimentLedger => {
                    EvolutionGoalEvidence::experiment_ledger(true)
                }
                EvolutionGoalEvidenceKind::OperatorApproval => {
                    EvolutionGoalEvidence::operator_approval(true)
                }
            })
            .collect::<Vec<_>>();

        EvolutionGoalRunEvidence::new(candidate.proposed_goal.stable_id.clone())
            .with_evidence(evidence)
    }
}
