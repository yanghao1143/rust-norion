use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

pub const EVOLUTION_GOAL_SCHEMA_VERSION: &str = "evolution_goal_v1";
pub const EVOLUTION_GOAL_QUEUE_RECORD_SCHEMA_VERSION: &str = "evolution_goal_queue_records_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvolutionGoalEvidenceKind {
    CargoCheck,
    FocusedTests,
    BenchmarkGate,
    TraceSchemaGate,
    ExperimentLedger,
    OperatorApproval,
}

impl EvolutionGoalEvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CargoCheck => "cargo_check",
            Self::FocusedTests => "focused_tests",
            Self::BenchmarkGate => "benchmark_gate",
            Self::TraceSchemaGate => "trace_schema_gate",
            Self::ExperimentLedger => "experiment_ledger",
            Self::OperatorApproval => "operator_approval",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "cargo_check" => Some(Self::CargoCheck),
            "focused_tests" => Some(Self::FocusedTests),
            "benchmark_gate" => Some(Self::BenchmarkGate),
            "trace_schema_gate" => Some(Self::TraceSchemaGate),
            "experiment_ledger" => Some(Self::ExperimentLedger),
            "operator_approval" => Some(Self::OperatorApproval),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoalSuccessGate {
    pub required_evidence: Vec<EvolutionGoalEvidenceKind>,
    pub require_all_required: bool,
    pub min_passed_evidence: usize,
}

impl EvolutionGoalSuccessGate {
    pub fn new(required_evidence: impl IntoIterator<Item = EvolutionGoalEvidenceKind>) -> Self {
        let mut required_evidence = required_evidence.into_iter().collect::<Vec<_>>();
        required_evidence.sort();
        required_evidence.dedup();
        Self {
            min_passed_evidence: required_evidence.len(),
            required_evidence,
            require_all_required: true,
        }
    }

    pub fn with_min_passed_evidence(mut self, min_passed_evidence: usize) -> Self {
        self.min_passed_evidence = min_passed_evidence;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoalStopCondition {
    pub success_stops_goal: bool,
    pub budget_exhaustion_stops_goal: bool,
    pub rollback_stops_goal: bool,
    pub approval_hold_stops_queue: bool,
}

impl Default for EvolutionGoalStopCondition {
    fn default() -> Self {
        Self {
            success_stops_goal: true,
            budget_exhaustion_stops_goal: true,
            rollback_stops_goal: true,
            approval_hold_stops_queue: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoalRollbackCondition {
    pub rollback_on_failed_required_evidence: bool,
    pub rollback_on_trace_schema_failure: bool,
    pub rollback_on_explicit_signal: bool,
}

impl Default for EvolutionGoalRollbackCondition {
    fn default() -> Self {
        Self {
            rollback_on_failed_required_evidence: true,
            rollback_on_trace_schema_failure: true,
            rollback_on_explicit_signal: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvolutionGoalBudgetCap {
    pub max_attempts: u32,
    pub max_steps: u32,
    pub max_tokens: u64,
    pub max_runtime_ms: u64,
}

impl EvolutionGoalBudgetCap {
    pub fn new(max_attempts: u32, max_steps: u32, max_tokens: u64, max_runtime_ms: u64) -> Self {
        Self {
            max_attempts,
            max_steps,
            max_tokens,
            max_runtime_ms,
        }
    }
}

impl Default for EvolutionGoalBudgetCap {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            max_steps: 12,
            max_tokens: 80_000,
            max_runtime_ms: 900_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoalApprovalGate {
    pub maintainer_required: bool,
    pub operator_required: bool,
    pub approval_evidence_required: bool,
}

impl Default for EvolutionGoalApprovalGate {
    fn default() -> Self {
        Self {
            maintainer_required: true,
            operator_required: true,
            approval_evidence_required: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoal {
    pub schema_version: &'static str,
    pub stable_id: String,
    pub priority: u32,
    pub objective: String,
    pub success_gate: EvolutionGoalSuccessGate,
    pub stop_condition: EvolutionGoalStopCondition,
    pub rollback_condition: EvolutionGoalRollbackCondition,
    pub budget_cap: EvolutionGoalBudgetCap,
    pub approval_gate: EvolutionGoalApprovalGate,
    pub provenance_digest: String,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl EvolutionGoal {
    pub fn new(
        priority: u32,
        objective: impl Into<String>,
        success_gate: EvolutionGoalSuccessGate,
        provenance_parts: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        Self::with_policy(
            priority,
            objective,
            success_gate,
            EvolutionGoalStopCondition::default(),
            EvolutionGoalRollbackCondition::default(),
            EvolutionGoalBudgetCap::default(),
            EvolutionGoalApprovalGate::default(),
            provenance_parts,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_policy(
        priority: u32,
        objective: impl Into<String>,
        success_gate: EvolutionGoalSuccessGate,
        stop_condition: EvolutionGoalStopCondition,
        rollback_condition: EvolutionGoalRollbackCondition,
        budget_cap: EvolutionGoalBudgetCap,
        approval_gate: EvolutionGoalApprovalGate,
        provenance_parts: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        let objective = safe_text(objective.into());
        let provenance = provenance_parts
            .into_iter()
            .map(|part| safe_text(part.as_ref().to_owned()))
            .collect::<Vec<_>>();
        let provenance_refs = provenance.iter().map(String::as_str).collect::<Vec<_>>();
        let provenance_digest = stable_redaction_digest(provenance_refs);
        let stable_id = stable_redaction_digest([
            EVOLUTION_GOAL_SCHEMA_VERSION,
            &priority.to_string(),
            objective.as_str(),
            provenance_digest.as_str(),
            &success_gate_digest(&success_gate),
        ]);

        Self {
            schema_version: EVOLUTION_GOAL_SCHEMA_VERSION,
            stable_id,
            priority,
            objective,
            success_gate,
            stop_condition,
            rollback_condition,
            budget_cap,
            approval_gate,
            provenance_digest,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn to_record_line(&self) -> String {
        let fields = [
            self.schema_version.to_owned(),
            self.stable_id.clone(),
            self.priority.to_string(),
            self.objective.clone(),
            evidence_kind_list(&self.success_gate.required_evidence),
            bool_to_field(self.success_gate.require_all_required).to_owned(),
            self.success_gate.min_passed_evidence.to_string(),
            bool_to_field(self.stop_condition.success_stops_goal).to_owned(),
            bool_to_field(self.stop_condition.budget_exhaustion_stops_goal).to_owned(),
            bool_to_field(self.stop_condition.rollback_stops_goal).to_owned(),
            bool_to_field(self.stop_condition.approval_hold_stops_queue).to_owned(),
            bool_to_field(self.rollback_condition.rollback_on_failed_required_evidence).to_owned(),
            bool_to_field(self.rollback_condition.rollback_on_trace_schema_failure).to_owned(),
            bool_to_field(self.rollback_condition.rollback_on_explicit_signal).to_owned(),
            self.budget_cap.max_attempts.to_string(),
            self.budget_cap.max_steps.to_string(),
            self.budget_cap.max_tokens.to_string(),
            self.budget_cap.max_runtime_ms.to_string(),
            bool_to_field(self.approval_gate.maintainer_required).to_owned(),
            bool_to_field(self.approval_gate.operator_required).to_owned(),
            bool_to_field(self.approval_gate.approval_evidence_required).to_owned(),
            self.provenance_digest.clone(),
            bool_to_field(self.read_only).to_owned(),
            bool_to_field(self.write_allowed).to_owned(),
            bool_to_field(self.applied).to_owned(),
        ];

        fields
            .iter()
            .map(|field| escape_field(field))
            .collect::<Vec<_>>()
            .join("\t")
    }

    pub fn from_record_line(line: &str) -> Result<Self, EvolutionGoalRecordDecodeError> {
        let fields = split_record_fields(line)?;
        if fields.len() != 25 {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_record_field_count",
                line,
            ));
        }
        if fields[0] != EVOLUTION_GOAL_SCHEMA_VERSION {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_record_schema_mismatch",
                line,
            ));
        }
        if contains_private_or_executable_marker(&fields[3]) {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_record_private_marker",
                line,
            ));
        }

        let goal = Self {
            schema_version: EVOLUTION_GOAL_SCHEMA_VERSION,
            stable_id: require_redaction_digest(&fields[1], "stable_id", line)?,
            priority: parse_u32_field(&fields[2], "priority", line)?,
            objective: fields[3].trim().to_owned(),
            success_gate: EvolutionGoalSuccessGate {
                required_evidence: parse_evidence_kinds(&fields[4], line)?,
                require_all_required: parse_bool_field(&fields[5], "require_all_required", line)?,
                min_passed_evidence: parse_usize_field(&fields[6], "min_passed_evidence", line)?,
            },
            stop_condition: EvolutionGoalStopCondition {
                success_stops_goal: parse_bool_field(&fields[7], "success_stops_goal", line)?,
                budget_exhaustion_stops_goal: parse_bool_field(
                    &fields[8],
                    "budget_exhaustion_stops_goal",
                    line,
                )?,
                rollback_stops_goal: parse_bool_field(&fields[9], "rollback_stops_goal", line)?,
                approval_hold_stops_queue: parse_bool_field(
                    &fields[10],
                    "approval_hold_stops_queue",
                    line,
                )?,
            },
            rollback_condition: EvolutionGoalRollbackCondition {
                rollback_on_failed_required_evidence: parse_bool_field(
                    &fields[11],
                    "rollback_on_failed_required_evidence",
                    line,
                )?,
                rollback_on_trace_schema_failure: parse_bool_field(
                    &fields[12],
                    "rollback_on_trace_schema_failure",
                    line,
                )?,
                rollback_on_explicit_signal: parse_bool_field(
                    &fields[13],
                    "rollback_on_explicit_signal",
                    line,
                )?,
            },
            budget_cap: EvolutionGoalBudgetCap {
                max_attempts: parse_u32_field(&fields[14], "max_attempts", line)?,
                max_steps: parse_u32_field(&fields[15], "max_steps", line)?,
                max_tokens: parse_u64_field(&fields[16], "max_tokens", line)?,
                max_runtime_ms: parse_u64_field(&fields[17], "max_runtime_ms", line)?,
            },
            approval_gate: EvolutionGoalApprovalGate {
                maintainer_required: parse_bool_field(&fields[18], "maintainer_required", line)?,
                operator_required: parse_bool_field(&fields[19], "operator_required", line)?,
                approval_evidence_required: parse_bool_field(
                    &fields[20],
                    "approval_evidence_required",
                    line,
                )?,
            },
            provenance_digest: require_redaction_digest(&fields[21], "provenance_digest", line)?,
            read_only: parse_bool_field(&fields[22], "read_only", line)?,
            write_allowed: parse_bool_field(&fields[23], "write_allowed", line)?,
            applied: parse_bool_field(&fields[24], "applied", line)?,
        };

        if !goal.read_only || goal.write_allowed || goal.applied {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_record_write_flags",
                line,
            ));
        }
        if goal.objective.is_empty() {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_record_objective_empty",
                line,
            ));
        }
        Ok(goal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoalRecordDecodeError {
    pub redacted_error: String,
    pub error_digest: String,
}

impl EvolutionGoalRecordDecodeError {
    fn new(reason: &str, payload: &str) -> Self {
        Self {
            redacted_error: reason.to_owned(),
            error_digest: stable_redaction_digest([
                "evolution-goal-record-decode",
                reason,
                payload,
            ]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvolutionGoalStatus {
    Queued,
    Active,
    Passed,
    Failed,
    RolledBack,
    BudgetExhausted,
    BlockedForApproval,
}

impl EvolutionGoalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Active => "active",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::RolledBack => "rolled_back",
            Self::BudgetExhausted => "budget_exhausted",
            Self::BlockedForApproval => "blocked_for_approval",
        }
    }

    fn stops_queue(self) -> bool {
        matches!(
            self,
            Self::Active
                | Self::Failed
                | Self::RolledBack
                | Self::BudgetExhausted
                | Self::BlockedForApproval
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionGoalEvidence {
    pub kind: EvolutionGoalEvidenceKind,
    pub label: String,
    pub passed: bool,
    pub item_count: u64,
    pub failure_count: u64,
    pub score: f32,
    pub evidence_digest: String,
}

impl EvolutionGoalEvidence {
    pub fn new(
        kind: EvolutionGoalEvidenceKind,
        label: impl Into<String>,
        passed: bool,
        item_count: u64,
        failure_count: u64,
    ) -> Self {
        let label = safe_text(label.into());
        let evidence_digest = stable_redaction_digest([
            kind.as_str(),
            label.as_str(),
            bool_to_field(passed),
            &item_count.to_string(),
            &failure_count.to_string(),
        ]);
        Self {
            kind,
            label,
            passed,
            item_count,
            failure_count,
            score: if passed { 1.0 } else { 0.0 },
            evidence_digest,
        }
    }

    pub fn cargo_check(passed: bool) -> Self {
        Self::new(
            EvolutionGoalEvidenceKind::CargoCheck,
            "cargo-check",
            passed,
            1,
            u64::from(!passed),
        )
    }

    pub fn focused_tests(passed: bool, item_count: u64, failure_count: u64) -> Self {
        Self::new(
            EvolutionGoalEvidenceKind::FocusedTests,
            "focused-tests",
            passed,
            item_count,
            failure_count,
        )
    }

    pub fn benchmark_gate(passed: bool) -> Self {
        Self::new(
            EvolutionGoalEvidenceKind::BenchmarkGate,
            "benchmark-gate",
            passed,
            1,
            u64::from(!passed),
        )
    }

    pub fn trace_schema_gate(passed: bool) -> Self {
        Self::new(
            EvolutionGoalEvidenceKind::TraceSchemaGate,
            "trace-schema-gate",
            passed,
            1,
            u64::from(!passed),
        )
    }

    pub fn experiment_ledger(passed: bool) -> Self {
        Self::new(
            EvolutionGoalEvidenceKind::ExperimentLedger,
            "experiment-ledger",
            passed,
            1,
            u64::from(!passed),
        )
    }

    pub fn operator_approval(passed: bool) -> Self {
        Self::new(
            EvolutionGoalEvidenceKind::OperatorApproval,
            "operator-approval",
            passed,
            1,
            u64::from(!passed),
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EvolutionGoalBudgetUsage {
    pub attempts: u32,
    pub steps: u32,
    pub tokens: u64,
    pub runtime_ms: u64,
}

impl EvolutionGoalBudgetUsage {
    pub fn new(attempts: u32, steps: u32, tokens: u64, runtime_ms: u64) -> Self {
        Self {
            attempts,
            steps,
            tokens,
            runtime_ms,
        }
    }

    fn exceeds(self, cap: EvolutionGoalBudgetCap) -> bool {
        self.attempts > cap.max_attempts
            || self.steps > cap.max_steps
            || self.tokens > cap.max_tokens
            || self.runtime_ms > cap.max_runtime_ms
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionGoalRunEvidence {
    pub goal_id: String,
    pub evidence: Vec<EvolutionGoalEvidence>,
    pub budget_usage: EvolutionGoalBudgetUsage,
    pub rollback_signal: bool,
    pub approval_granted: bool,
}

impl EvolutionGoalRunEvidence {
    pub fn new(goal_id: impl Into<String>) -> Self {
        Self {
            goal_id: goal_id.into(),
            evidence: Vec::new(),
            budget_usage: EvolutionGoalBudgetUsage::default(),
            rollback_signal: false,
            approval_granted: false,
        }
    }

    pub fn with_evidence(
        mut self,
        evidence: impl IntoIterator<Item = EvolutionGoalEvidence>,
    ) -> Self {
        self.evidence.extend(evidence);
        self
    }

    pub fn with_budget_usage(mut self, budget_usage: EvolutionGoalBudgetUsage) -> Self {
        self.budget_usage = budget_usage;
        self
    }

    pub fn with_rollback_signal(mut self) -> Self {
        self.rollback_signal = true;
        self
    }

    pub fn with_approval(mut self) -> Self {
        self.approval_granted = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoalDecision {
    pub goal_id: String,
    pub priority: u32,
    pub status: EvolutionGoalStatus,
    pub reason_codes: Vec<String>,
    pub evidence_digests: Vec<String>,
    pub active: bool,
    pub conflict_isolated: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl EvolutionGoalDecision {
    pub fn summary_line(&self) -> String {
        format!(
            "evolution_goal_decision_v1 goal={} priority={} status={} active={} isolated={} reasons={} evidence={} write_allowed={} applied={}",
            self.goal_id,
            self.priority,
            self.status.as_str(),
            self.active,
            self.conflict_isolated,
            self.reason_codes.join("|"),
            self.evidence_digests.join("|"),
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoalQueueReport {
    pub schema_version: &'static str,
    pub decisions: Vec<EvolutionGoalDecision>,
    pub active_goal_id: Option<String>,
    pub passed_count: usize,
    pub failed_count: usize,
    pub rolled_back_count: usize,
    pub budget_exhausted_count: usize,
    pub approval_hold_count: usize,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl EvolutionGoalQueueReport {
    pub fn is_preview_only(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self
                .decisions
                .iter()
                .all(|decision| decision.read_only && !decision.write_allowed && !decision.applied)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoalQueue {
    pub schema_version: &'static str,
    pub goals: Vec<EvolutionGoal>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl EvolutionGoalQueue {
    pub fn new(mut goals: Vec<EvolutionGoal>) -> Self {
        goals.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.stable_id.cmp(&right.stable_id))
        });
        Self {
            schema_version: EVOLUTION_GOAL_SCHEMA_VERSION,
            goals,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn evaluate(&self, runs: &[EvolutionGoalRunEvidence]) -> EvolutionGoalQueueReport {
        let mut decisions = Vec::with_capacity(self.goals.len());
        let mut active_goal_id = None;
        let mut prior_blocking_status: Option<(String, EvolutionGoalStatus)> = None;

        for goal in &self.goals {
            if let Some((prior_goal_id, status)) = &prior_blocking_status {
                decisions.push(queued_after_prior_block(goal, prior_goal_id, *status));
                continue;
            }

            let run = runs.iter().find(|run| run.goal_id == goal.stable_id);
            let decision = evaluate_goal(goal, run);
            if decision.active {
                active_goal_id = Some(goal.stable_id.clone());
            }
            if decision.status.stops_queue() {
                prior_blocking_status = Some((goal.stable_id.clone(), decision.status));
            }
            decisions.push(decision);
        }

        let passed_count = decisions
            .iter()
            .filter(|decision| decision.status == EvolutionGoalStatus::Passed)
            .count();
        let failed_count = decisions
            .iter()
            .filter(|decision| decision.status == EvolutionGoalStatus::Failed)
            .count();
        let rolled_back_count = decisions
            .iter()
            .filter(|decision| decision.status == EvolutionGoalStatus::RolledBack)
            .count();
        let budget_exhausted_count = decisions
            .iter()
            .filter(|decision| decision.status == EvolutionGoalStatus::BudgetExhausted)
            .count();
        let approval_hold_count = decisions
            .iter()
            .filter(|decision| decision.status == EvolutionGoalStatus::BlockedForApproval)
            .count();

        EvolutionGoalQueueReport {
            schema_version: EVOLUTION_GOAL_SCHEMA_VERSION,
            decisions,
            active_goal_id,
            passed_count,
            failed_count,
            rolled_back_count,
            budget_exhausted_count,
            approval_hold_count,
            read_only: self.read_only,
            write_allowed: self.write_allowed,
            applied: self.applied,
        }
    }

    pub fn redaction_digest(&self) -> String {
        let lines = self
            .goals
            .iter()
            .map(EvolutionGoal::to_record_line)
            .collect::<Vec<_>>();
        let mut parts = Vec::with_capacity(lines.len() + 4);
        parts.push(self.schema_version);
        parts.push(bool_to_field(self.read_only));
        parts.push(bool_to_field(self.write_allowed));
        parts.push(bool_to_field(self.applied));
        parts.extend(lines.iter().map(String::as_str));
        stable_redaction_digest(parts)
    }

    pub fn to_record_text(&self) -> String {
        let mut lines = Vec::with_capacity(self.goals.len() + 1);
        lines.push(
            [
                EVOLUTION_GOAL_QUEUE_RECORD_SCHEMA_VERSION.to_owned(),
                self.schema_version.to_owned(),
                bool_to_field(self.read_only).to_owned(),
                bool_to_field(self.write_allowed).to_owned(),
                bool_to_field(self.applied).to_owned(),
                self.goals.len().to_string(),
                self.redaction_digest(),
            ]
            .iter()
            .map(|field| escape_field(field))
            .collect::<Vec<_>>()
            .join("\t"),
        );
        lines.extend(self.goals.iter().map(EvolutionGoal::to_record_line));
        lines.join("\n")
    }

    pub fn from_record_text(text: &str) -> Result<Self, EvolutionGoalRecordDecodeError> {
        let mut lines = text.lines();
        let Some(header) = lines.next() else {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_queue_empty",
                text,
            ));
        };
        let fields = split_record_fields(header)?;
        if fields.len() != 7 {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_queue_header_field_count",
                text,
            ));
        }
        if fields[0] != EVOLUTION_GOAL_QUEUE_RECORD_SCHEMA_VERSION {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_queue_schema_mismatch",
                text,
            ));
        }
        if fields[1] != EVOLUTION_GOAL_SCHEMA_VERSION {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_queue_goal_schema_mismatch",
                text,
            ));
        }
        let read_only = parse_bool_field(&fields[2], "queue_read_only", text)?;
        let write_allowed = parse_bool_field(&fields[3], "queue_write_allowed", text)?;
        let applied = parse_bool_field(&fields[4], "queue_applied", text)?;
        if !read_only || write_allowed || applied {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_queue_write_flags",
                text,
            ));
        }
        let expected_count = parse_usize_field(&fields[5], "queue_goal_count", text)?;
        let expected_digest = require_redaction_digest(&fields[6], "queue_digest", text)?;
        let goals = lines
            .filter(|line| !line.trim().is_empty())
            .map(EvolutionGoal::from_record_line)
            .collect::<Result<Vec<_>, _>>()?;
        if goals.len() != expected_count {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_queue_goal_count_mismatch",
                text,
            ));
        }
        let queue = EvolutionGoalQueue::new(goals);
        if queue.redaction_digest() != expected_digest {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_queue_digest_mismatch",
                text,
            ));
        }
        Ok(queue)
    }
}

pub fn default_noiron_pursuit_goals() -> Vec<EvolutionGoal> {
    vec![EvolutionGoal::new(
        10,
        "R97 English/Chinese/Rust coding service and eval harness",
        EvolutionGoalSuccessGate::new([
            EvolutionGoalEvidenceKind::CargoCheck,
            EvolutionGoalEvidenceKind::FocusedTests,
            EvolutionGoalEvidenceKind::TraceSchemaGate,
            EvolutionGoalEvidenceKind::OperatorApproval,
        ]),
        [
            "roadmap:R97",
            "issues:#75,#19,#29",
            "pursuit:multilingual-coding-service-eval",
        ],
    )]
}

pub fn default_noiron_pursuit_goal_queue() -> EvolutionGoalQueue {
    EvolutionGoalQueue::new(default_noiron_pursuit_goals())
}

fn evaluate_goal(
    goal: &EvolutionGoal,
    run: Option<&EvolutionGoalRunEvidence>,
) -> EvolutionGoalDecision {
    let Some(run) = run else {
        return decision(
            goal,
            EvolutionGoalStatus::Active,
            ["awaiting_goal_evidence"],
            Vec::new(),
            true,
            false,
        );
    };

    let evidence_digests = run
        .evidence
        .iter()
        .map(|evidence| evidence.evidence_digest.clone())
        .collect::<Vec<_>>();

    if goal.rollback_condition.rollback_on_explicit_signal && run.rollback_signal {
        return decision(
            goal,
            EvolutionGoalStatus::RolledBack,
            ["rollback_signal_triggered"],
            evidence_digests,
            false,
            true,
        );
    }

    if goal.stop_condition.budget_exhaustion_stops_goal && run.budget_usage.exceeds(goal.budget_cap)
    {
        return decision(
            goal,
            EvolutionGoalStatus::BudgetExhausted,
            ["budget_cap_exhausted"],
            evidence_digests,
            false,
            true,
        );
    }

    let failed_required = failed_required_evidence(goal, &run.evidence);
    if !failed_required.is_empty() {
        let status = if goal.rollback_condition.rollback_on_failed_required_evidence
            || failed_required.iter().any(|kind| {
                *kind == EvolutionGoalEvidenceKind::TraceSchemaGate
                    && goal.rollback_condition.rollback_on_trace_schema_failure
            }) {
            EvolutionGoalStatus::RolledBack
        } else {
            EvolutionGoalStatus::Failed
        };
        let reasons = failed_required
            .iter()
            .map(|kind| format!("required_evidence_failed:{}", kind.as_str()))
            .collect::<Vec<_>>();
        return decision_from_vec(goal, status, reasons, evidence_digests, false, true);
    }

    let missing_required = missing_required_evidence(goal, &run.evidence);
    if !missing_required.is_empty() {
        let reasons = missing_required
            .iter()
            .map(|kind| format!("required_evidence_missing:{}", kind.as_str()))
            .collect::<Vec<_>>();
        return decision_from_vec(
            goal,
            EvolutionGoalStatus::Active,
            reasons,
            evidence_digests,
            true,
            false,
        );
    }

    let passed_count = run
        .evidence
        .iter()
        .filter(|evidence| evidence.passed)
        .count();
    if passed_count < goal.success_gate.min_passed_evidence {
        return decision(
            goal,
            EvolutionGoalStatus::Active,
            ["success_gate_not_satisfied"],
            evidence_digests,
            true,
            false,
        );
    }

    if approval_required(goal) && !run.approval_granted {
        return decision(
            goal,
            EvolutionGoalStatus::BlockedForApproval,
            ["approval_required_before_promotion"],
            evidence_digests,
            false,
            true,
        );
    }

    decision(
        goal,
        EvolutionGoalStatus::Passed,
        ["success_gate_passed"],
        evidence_digests,
        false,
        false,
    )
}

fn decision<'a>(
    goal: &EvolutionGoal,
    status: EvolutionGoalStatus,
    reason_codes: impl IntoIterator<Item = &'a str>,
    evidence_digests: Vec<String>,
    active: bool,
    conflict_isolated: bool,
) -> EvolutionGoalDecision {
    decision_from_vec(
        goal,
        status,
        reason_codes
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>(),
        evidence_digests,
        active,
        conflict_isolated,
    )
}

fn decision_from_vec(
    goal: &EvolutionGoal,
    status: EvolutionGoalStatus,
    reason_codes: Vec<String>,
    evidence_digests: Vec<String>,
    active: bool,
    conflict_isolated: bool,
) -> EvolutionGoalDecision {
    EvolutionGoalDecision {
        goal_id: goal.stable_id.clone(),
        priority: goal.priority,
        status,
        reason_codes,
        evidence_digests,
        active,
        conflict_isolated,
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

fn queued_after_prior_block(
    goal: &EvolutionGoal,
    prior_goal_id: &str,
    prior_status: EvolutionGoalStatus,
) -> EvolutionGoalDecision {
    decision_from_vec(
        goal,
        EvolutionGoalStatus::Queued,
        vec![
            format!("waiting_for_prior_goal:{prior_goal_id}"),
            format!("prior_goal_status:{}", prior_status.as_str()),
            "conflict_isolation_hold".to_owned(),
        ],
        Vec::new(),
        false,
        true,
    )
}

fn failed_required_evidence(
    goal: &EvolutionGoal,
    evidence: &[EvolutionGoalEvidence],
) -> Vec<EvolutionGoalEvidenceKind> {
    let mut failed = Vec::new();
    for kind in &goal.success_gate.required_evidence {
        if evidence
            .iter()
            .any(|item| item.kind == *kind && (!item.passed || item.failure_count > 0))
        {
            failed.push(*kind);
        }
    }
    failed
}

fn missing_required_evidence(
    goal: &EvolutionGoal,
    evidence: &[EvolutionGoalEvidence],
) -> Vec<EvolutionGoalEvidenceKind> {
    if !goal.success_gate.require_all_required {
        return Vec::new();
    }
    let mut missing = Vec::new();
    for kind in &goal.success_gate.required_evidence {
        if !evidence.iter().any(|item| item.kind == *kind) {
            missing.push(*kind);
        }
    }
    missing
}

fn approval_required(goal: &EvolutionGoal) -> bool {
    goal.approval_gate.maintainer_required
        || goal.approval_gate.operator_required
        || goal.approval_gate.approval_evidence_required
}

fn safe_text(value: String) -> String {
    if contains_private_or_executable_marker(&value) {
        stable_redaction_digest(["redacted-text", value.trim()])
    } else {
        value.trim().to_owned()
    }
}

fn success_gate_digest(gate: &EvolutionGoalSuccessGate) -> String {
    stable_redaction_digest([
        &evidence_kind_list(&gate.required_evidence),
        bool_to_field(gate.require_all_required),
        &gate.min_passed_evidence.to_string(),
    ])
}

fn evidence_kind_list(values: &[EvolutionGoalEvidenceKind]) -> String {
    values
        .iter()
        .map(|kind| kind.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('|', "\\p")
}

fn split_record_fields(line: &str) -> Result<Vec<String>, EvolutionGoalRecordDecodeError> {
    line.split('\t')
        .map(|field| unescape_field(field, line))
        .collect()
}

fn unescape_field(value: &str, payload: &str) -> Result<String, EvolutionGoalRecordDecodeError> {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(escaped) = chars.next() else {
            return Err(EvolutionGoalRecordDecodeError::new(
                "evolution_goal_record_bad_escape",
                payload,
            ));
        };
        match escaped {
            '\\' => out.push('\\'),
            't' => out.push('\t'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            'p' => out.push('|'),
            _ => {
                return Err(EvolutionGoalRecordDecodeError::new(
                    "evolution_goal_record_bad_escape",
                    payload,
                ));
            }
        }
    }
    Ok(out)
}

fn parse_evidence_kinds(
    value: &str,
    payload: &str,
) -> Result<Vec<EvolutionGoalEvidenceKind>, EvolutionGoalRecordDecodeError> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut kinds = value
        .split('|')
        .map(|kind| {
            EvolutionGoalEvidenceKind::from_str(kind).ok_or_else(|| {
                EvolutionGoalRecordDecodeError::new(
                    "evolution_goal_record_unknown_evidence_kind",
                    payload,
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    kinds.sort();
    kinds.dedup();
    Ok(kinds)
}

fn parse_bool_field(
    value: &str,
    field: &str,
    payload: &str,
) -> Result<bool, EvolutionGoalRecordDecodeError> {
    match value {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err(EvolutionGoalRecordDecodeError::new(
            &format!("evolution_goal_record_bad_bool:{field}"),
            payload,
        )),
    }
}

fn parse_u32_field(
    value: &str,
    field: &str,
    payload: &str,
) -> Result<u32, EvolutionGoalRecordDecodeError> {
    value.parse::<u32>().map_err(|_| {
        EvolutionGoalRecordDecodeError::new(
            &format!("evolution_goal_record_bad_u32:{field}"),
            payload,
        )
    })
}

fn parse_u64_field(
    value: &str,
    field: &str,
    payload: &str,
) -> Result<u64, EvolutionGoalRecordDecodeError> {
    value.parse::<u64>().map_err(|_| {
        EvolutionGoalRecordDecodeError::new(
            &format!("evolution_goal_record_bad_u64:{field}"),
            payload,
        )
    })
}

fn parse_usize_field(
    value: &str,
    field: &str,
    payload: &str,
) -> Result<usize, EvolutionGoalRecordDecodeError> {
    value.parse::<usize>().map_err(|_| {
        EvolutionGoalRecordDecodeError::new(
            &format!("evolution_goal_record_bad_usize:{field}"),
            payload,
        )
    })
}

fn require_redaction_digest(
    value: &str,
    field: &str,
    payload: &str,
) -> Result<String, EvolutionGoalRecordDecodeError> {
    if value.starts_with("redaction-digest:") && !contains_private_or_executable_marker(value) {
        Ok(value.to_owned())
    } else {
        Err(EvolutionGoalRecordDecodeError::new(
            &format!("evolution_goal_record_bad_digest:{field}"),
            payload,
        ))
    }
}

fn bool_to_field(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evolution_goal_record_is_deterministic_and_preview_only() {
        let goal = sample_goal(10, "ship transaction queue");
        let first = goal.to_record_line();
        let second = goal.to_record_line();

        assert_eq!(first, second);
        assert!(first.contains(EVOLUTION_GOAL_SCHEMA_VERSION));
        assert!(first.contains("redaction-digest:"));
        assert!(goal.read_only);
        assert!(!goal.write_allowed);
        assert!(!goal.applied);
    }

    #[test]
    fn evolution_goal_record_round_trips_without_write_flags() {
        let goal = sample_goal(10, "ship transaction queue");
        let parsed = EvolutionGoal::from_record_line(&goal.to_record_line()).unwrap();

        assert_eq!(parsed, goal);
        assert!(parsed.read_only);
        assert!(!parsed.write_allowed);
        assert!(!parsed.applied);
    }

    #[test]
    fn evolution_goal_queue_record_text_round_trips_and_checks_digest() {
        let first = sample_goal(10, "first");
        let second = sample_goal(20, "second");
        let queue = EvolutionGoalQueue::new(vec![second, first]);
        let text = queue.to_record_text();
        let parsed = EvolutionGoalQueue::from_record_text(&text).unwrap();

        assert_eq!(parsed, queue);
        assert_eq!(parsed.redaction_digest(), queue.redaction_digest());
        assert!(text.contains(EVOLUTION_GOAL_QUEUE_RECORD_SCHEMA_VERSION));
    }

    #[test]
    fn evolution_goal_record_decode_rejects_write_flags_and_digest_tampering() {
        let goal = sample_goal(10, "ship transaction queue");
        let mut fields = goal
            .to_record_line()
            .split('\t')
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        fields[23] = "1".to_owned();
        let write_allowed = fields.join("\t");
        let bad_digest = goal
            .to_record_line()
            .replacen("redaction-digest:", "fnv64:", 1);
        let queue_bad_digest = EvolutionGoalQueue::new(vec![goal])
            .to_record_text()
            .replacen("redaction-digest:", "fnv64:", 1);

        assert!(EvolutionGoal::from_record_line(&write_allowed).is_err());
        assert!(EvolutionGoal::from_record_line(&bad_digest).is_err());
        assert!(EvolutionGoalQueue::from_record_text(&queue_bad_digest).is_err());
    }

    #[test]
    fn evolution_goal_queue_orders_goals_and_activates_one_at_a_time() {
        let first = sample_goal(10, "first");
        let second = sample_goal(20, "second");
        let third = sample_goal(30, "third");
        let queue = EvolutionGoalQueue::new(vec![third.clone(), first.clone(), second.clone()]);
        let first_run = passing_run(&first);

        let report = queue.evaluate(&[first_run]);

        assert_eq!(report.passed_count, 1);
        assert_eq!(report.active_goal_id, Some(second.stable_id.clone()));
        assert_eq!(report.decisions[0].goal_id, first.stable_id);
        assert_eq!(report.decisions[0].status, EvolutionGoalStatus::Passed);
        assert_eq!(report.decisions[1].goal_id, second.stable_id);
        assert_eq!(report.decisions[1].status, EvolutionGoalStatus::Active);
        assert_eq!(report.decisions[2].goal_id, third.stable_id);
        assert_eq!(report.decisions[2].status, EvolutionGoalStatus::Queued);
        assert!(report.decisions[2].conflict_isolated);
        assert!(report.is_preview_only());
    }

    #[test]
    fn evolution_goal_queue_stops_on_success_gate_for_single_goal() {
        let goal = sample_goal(10, "success stop");
        let queue = EvolutionGoalQueue::new(vec![goal.clone()]);

        let report = queue.evaluate(&[passing_run(&goal)]);

        assert_eq!(report.active_goal_id, None);
        assert_eq!(report.passed_count, 1);
        assert_eq!(report.decisions[0].status, EvolutionGoalStatus::Passed);
        assert!(
            report.decisions[0]
                .reason_codes
                .contains(&"success_gate_passed".to_owned())
        );
    }

    #[test]
    fn evolution_goal_queue_stops_on_budget_exhaustion() {
        let goal = sample_goal(10, "budget stop");
        let queue = EvolutionGoalQueue::new(vec![goal.clone()]);
        let run = EvolutionGoalRunEvidence::new(goal.stable_id.clone())
            .with_budget_usage(EvolutionGoalBudgetUsage::new(4, 1, 1, 1))
            .with_evidence(required_success_evidence())
            .with_approval();

        let report = queue.evaluate(&[run]);

        assert_eq!(report.budget_exhausted_count, 1);
        assert_eq!(
            report.decisions[0].status,
            EvolutionGoalStatus::BudgetExhausted
        );
        assert!(report.decisions[0].conflict_isolated);
    }

    #[test]
    fn evolution_goal_queue_stops_on_rollback_signal() {
        let goal = sample_goal(10, "rollback stop");
        let queue = EvolutionGoalQueue::new(vec![goal.clone()]);
        let run = passing_run(&goal).with_rollback_signal();

        let report = queue.evaluate(&[run]);

        assert_eq!(report.rolled_back_count, 1);
        assert_eq!(report.decisions[0].status, EvolutionGoalStatus::RolledBack);
        assert!(
            report.decisions[0]
                .reason_codes
                .contains(&"rollback_signal_triggered".to_owned())
        );
    }

    #[test]
    fn evolution_goal_queue_blocks_for_approval_after_success_evidence() {
        let goal = sample_goal(10, "approval hold");
        let queue = EvolutionGoalQueue::new(vec![goal.clone()]);
        let run = EvolutionGoalRunEvidence::new(goal.stable_id.clone())
            .with_evidence(required_success_evidence());

        let report = queue.evaluate(&[run]);

        assert_eq!(report.approval_hold_count, 1);
        assert_eq!(
            report.decisions[0].status,
            EvolutionGoalStatus::BlockedForApproval
        );
        assert!(
            report.decisions[0]
                .reason_codes
                .contains(&"approval_required_before_promotion".to_owned())
        );
        assert!(report.is_preview_only());
    }

    #[test]
    fn evolution_goal_queue_isolates_later_goals_after_failure_or_rollback() {
        let first = sample_goal(10, "trace failure");
        let second = sample_goal(20, "must wait");
        let queue = EvolutionGoalQueue::new(vec![first.clone(), second.clone()]);
        let failed_run = EvolutionGoalRunEvidence::new(first.stable_id.clone()).with_evidence([
            EvolutionGoalEvidence::cargo_check(true),
            EvolutionGoalEvidence::focused_tests(true, 3, 0),
            EvolutionGoalEvidence::benchmark_gate(true),
            EvolutionGoalEvidence::trace_schema_gate(false),
        ]);

        let report = queue.evaluate(&[failed_run, passing_run(&second)]);

        assert_eq!(report.rolled_back_count, 1);
        assert_eq!(report.decisions[0].status, EvolutionGoalStatus::RolledBack);
        assert_eq!(report.decisions[1].status, EvolutionGoalStatus::Queued);
        assert!(report.decisions[1].conflict_isolated);
        assert!(
            report.decisions[1]
                .reason_codes
                .iter()
                .any(|reason| reason.starts_with("waiting_for_prior_goal:"))
        );
    }

    #[test]
    fn default_noiron_pursuit_goal_queue_advances_to_coding_service_eval() {
        let queue = default_noiron_pursuit_goal_queue();

        assert_eq!(queue.goals.len(), 1);
        assert!(queue.goals[0].objective.contains("R97"));

        let report = queue.evaluate(&[]);

        assert_eq!(
            report.active_goal_id,
            Some(queue.goals[0].stable_id.clone())
        );
        assert_eq!(report.decisions[0].status, EvolutionGoalStatus::Active);
        assert!(report.is_preview_only());
    }

    fn sample_goal(priority: u32, objective: &str) -> EvolutionGoal {
        EvolutionGoal::new(
            priority,
            objective,
            EvolutionGoalSuccessGate::new([
                EvolutionGoalEvidenceKind::CargoCheck,
                EvolutionGoalEvidenceKind::FocusedTests,
                EvolutionGoalEvidenceKind::BenchmarkGate,
                EvolutionGoalEvidenceKind::TraceSchemaGate,
            ]),
            ["issue:#79", objective],
        )
    }

    fn required_success_evidence() -> Vec<EvolutionGoalEvidence> {
        vec![
            EvolutionGoalEvidence::cargo_check(true),
            EvolutionGoalEvidence::focused_tests(true, 3, 0),
            EvolutionGoalEvidence::benchmark_gate(true),
            EvolutionGoalEvidence::trace_schema_gate(true),
        ]
    }

    fn passing_run(goal: &EvolutionGoal) -> EvolutionGoalRunEvidence {
        EvolutionGoalRunEvidence::new(goal.stable_id.clone())
            .with_evidence(required_success_evidence())
            .with_approval()
    }
}
