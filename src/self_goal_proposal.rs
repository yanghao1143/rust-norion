use crate::evolution_goal::{
    EvolutionGoal, EvolutionGoalApprovalGate, EvolutionGoalBudgetCap, EvolutionGoalEvidenceKind,
    EvolutionGoalQueue, EvolutionGoalRollbackCondition, EvolutionGoalStopCondition,
    EvolutionGoalSuccessGate, default_noiron_pursuit_goal_queue,
};
use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

pub const SELF_GOAL_PROPOSAL_SCHEMA_VERSION: &str = "self_goal_proposal_v1";
pub const SELF_GOAL_PROPOSAL_TRACE_SCHEMA: &str = "rust-norion-self-goal-proposal-preview-v1";

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

pub fn default_self_goal_proposal_report(queue: &EvolutionGoalQueue) -> SelfGoalProposalReport {
    SelfGoalProposalReport::from_queue(queue, SelfGoalProposalPolicy::default())
}

pub fn default_noiron_self_goal_proposal_report() -> SelfGoalProposalReport {
    default_self_goal_proposal_report(&default_noiron_pursuit_goal_queue())
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

fn bool_to_field(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
