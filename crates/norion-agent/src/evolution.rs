use crate::budget::AgentBudget;
use crate::conflict::ConflictReport;
use crate::ports::ToolBuildReport;
use crate::reflection::{ReflectionLoopHealthStatus, ReflectionLoopHistoryGateRecord};
use crate::run::{AgentRunReport, SideEffectKind};
use crate::task::{AgentRole, AgentTask, AgentTaskQueue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolIntent {
    Discovery,
    TraceAnalysis,
    StateInspection,
    BenchmarkGate,
    RuntimeAdapter,
    MemoryMaintenance,
    Generic,
}

impl ToolIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::TraceAnalysis => "trace_analysis",
            Self::StateInspection => "state_inspection",
            Self::BenchmarkGate => "benchmark_gate",
            Self::RuntimeAdapter => "runtime_adapter",
            Self::MemoryMaintenance => "memory_maintenance",
            Self::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolBuildStatus {
    Ready,
    Held,
    Rejected,
}

impl ToolBuildStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Held => "held",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolsmithBlueprintMovementDecision {
    AllowPreviewMove,
    HoldForScopeReview,
    QuarantineBlueprint,
    RejectContextJump,
}

impl ToolsmithBlueprintMovementDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllowPreviewMove => "allow_preview_move",
            Self::HoldForScopeReview => "hold_for_scope_review",
            Self::QuarantineBlueprint => "quarantine_blueprint",
            Self::RejectContextJump => "reject_context_jump",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolsmithBlueprintMovementReview {
    pub source_proposal_id: String,
    pub source_digest: String,
    pub source_scope: String,
    pub target_scope: String,
    pub allowed_scope_tags: Vec<String>,
    pub forbidden_scope_tags: Vec<String>,
    pub collision_risk: bool,
    pub decision: ToolsmithBlueprintMovementDecision,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl ToolsmithBlueprintMovementReview {
    pub fn new(
        source_proposal_id: impl Into<String>,
        source_digest: impl Into<String>,
        source_scope: impl Into<String>,
        target_scope: impl Into<String>,
    ) -> Self {
        Self {
            source_proposal_id: source_proposal_id.into(),
            source_digest: source_digest.into(),
            source_scope: source_scope.into(),
            target_scope: target_scope.into(),
            allowed_scope_tags: Vec::new(),
            forbidden_scope_tags: Vec::new(),
            collision_risk: false,
            decision: ToolsmithBlueprintMovementDecision::HoldForScopeReview,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn with_allowed_scope_tags(mut self, tags: impl IntoIterator<Item = String>) -> Self {
        self.allowed_scope_tags = tags.into_iter().collect();
        self
    }

    pub fn with_forbidden_scope_tags(mut self, tags: impl IntoIterator<Item = String>) -> Self {
        self.forbidden_scope_tags = tags.into_iter().collect();
        self
    }

    pub fn with_collision_risk(mut self, collision_risk: bool) -> Self {
        self.collision_risk = collision_risk;
        self
    }

    pub fn with_decision(mut self, decision: ToolsmithBlueprintMovementDecision) -> Self {
        self.decision = decision;
        self
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolProposal {
    pub id: String,
    pub intent: ToolIntent,
    pub rust_crate: String,
    pub entrypoint: String,
    pub status: ToolBuildStatus,
    pub gate_notes: Vec<String>,
    pub source_scope: Option<String>,
    pub target_scope: Option<String>,
    pub movement_review: Option<ToolsmithBlueprintMovementReview>,
}

impl ToolProposal {
    pub fn new(
        id: impl Into<String>,
        intent: ToolIntent,
        rust_crate: impl Into<String>,
        entrypoint: impl Into<String>,
        status: ToolBuildStatus,
    ) -> Self {
        Self {
            id: id.into(),
            intent,
            rust_crate: rust_crate.into(),
            entrypoint: entrypoint.into(),
            status,
            gate_notes: Vec::new(),
            source_scope: None,
            target_scope: None,
            movement_review: None,
        }
    }

    pub fn with_gate_note(mut self, note: impl Into<String>) -> Self {
        self.gate_notes.push(note.into());
        self
    }

    pub fn with_source_scope(mut self, source_scope: impl Into<String>) -> Self {
        self.source_scope = Some(source_scope.into());
        self
    }

    pub fn with_target_scope(mut self, target_scope: impl Into<String>) -> Self {
        self.target_scope = Some(target_scope.into());
        self
    }

    pub fn with_movement_review(mut self, review: ToolsmithBlueprintMovementReview) -> Self {
        self.movement_review = Some(review);
        self
    }

    pub fn rust_only(&self) -> bool {
        self.rust_crate == "rust" && self.entrypoint.ends_with(".rs")
    }

    pub fn blueprint_digest(&self) -> String {
        stable_toolsmith_digest([
            self.id.as_str(),
            self.intent.as_str(),
            self.rust_crate.as_str(),
            self.entrypoint.as_str(),
            self.status.as_str(),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolsmithPlan {
    pub rust_only: bool,
    pub proposals: Vec<ToolProposal>,
    pub rejected_requests: Vec<String>,
    pub notes: Vec<String>,
}

impl Default for ToolsmithPlan {
    fn default() -> Self {
        Self {
            rust_only: true,
            proposals: Vec::new(),
            rejected_requests: Vec::new(),
            notes: Vec::new(),
        }
    }
}

impl ToolsmithPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_proposal(mut self, proposal: ToolProposal) -> Self {
        self.proposals.push(proposal);
        self
    }

    pub fn with_rejected_request(mut self, request: impl Into<String>) -> Self {
        self.rejected_requests.push(request.into());
        self
    }

    pub fn ready_count(&self) -> usize {
        self.proposals
            .iter()
            .filter(|proposal| proposal.status == ToolBuildStatus::Ready)
            .count()
    }

    pub fn held_count(&self) -> usize {
        self.proposals
            .iter()
            .filter(|proposal| proposal.status == ToolBuildStatus::Held)
            .count()
    }

    pub fn rejected_count(&self) -> usize {
        self.rejected_requests.len()
            + self
                .proposals
                .iter()
                .filter(|proposal| proposal.status == ToolBuildStatus::Rejected)
                .count()
    }

    pub fn passed_rust_gate(&self) -> bool {
        self.rust_only
            && self.rejected_requests.is_empty()
            && self.proposals.iter().all(ToolProposal::rust_only)
    }

    pub fn summary(&self) -> ToolsmithPlanSummary {
        ToolsmithPlanSummary::from_plan(self)
    }

    pub fn reward_notes(&self) -> Vec<String> {
        if self.proposals.is_empty() && self.rejected_requests.is_empty() {
            return Vec::new();
        }

        let mut notes = vec![format!(
            "toolsmith:proposals={}:ready={}:held={}:rejected={}:rust_only={}",
            self.proposals.len(),
            self.ready_count(),
            self.held_count(),
            self.rejected_count(),
            self.rust_only
        )];
        notes.extend(self.proposals.iter().take(3).map(|proposal| {
            format!(
                "toolsmith:{}:{}:{}",
                proposal.id,
                proposal.intent.as_str(),
                proposal.status.as_str()
            )
        }));
        notes
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolsmithPlanSummary {
    pub proposals: usize,
    pub ready: usize,
    pub held: usize,
    pub rejected: usize,
    pub rejected_requests: usize,
    pub non_rust_proposals: usize,
    pub rust_only: bool,
    pub rust_gate_passed: bool,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolsmithPlanHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl ToolsmithPlanHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolsmithPlanSummaryHistory {
    summaries: Vec<ToolsmithPlanSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolsmithPlanDashboard {
    pub total_records: usize,
    pub proposals: usize,
    pub ready: usize,
    pub held: usize,
    pub rejected: usize,
    pub rejected_requests: usize,
    pub non_rust_proposals: usize,
    pub rust_gate_failed_records: usize,
    pub empty_records: usize,
    pub ready_rate: f32,
    pub rust_gate_pass_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolsmithPlanHealthPolicy {
    pub maximum_rejected: usize,
    pub maximum_rejected_requests: usize,
    pub maximum_non_rust_proposals: usize,
    pub maximum_rust_gate_failed_records: usize,
    pub maximum_empty_records: usize,
    pub minimum_ready_rate: f32,
}

impl Default for ToolsmithPlanHealthPolicy {
    fn default() -> Self {
        Self {
            maximum_rejected: 0,
            maximum_rejected_requests: 0,
            maximum_non_rust_proposals: 0,
            maximum_rust_gate_failed_records: 0,
            maximum_empty_records: usize::MAX,
            minimum_ready_rate: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolsmithPlanHealth {
    pub status: ToolsmithPlanHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: ToolsmithPlanDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolsmithPlanSummaryHistoryRecord {
    pub history: ToolsmithPlanSummaryHistory,
    pub appended_summary: ToolsmithPlanSummary,
    pub dashboard: ToolsmithPlanDashboard,
    pub health: ToolsmithPlanHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolsmithPlanSummaryHistoryRecorder;

impl ToolsmithPlanSummary {
    pub fn from_plan(plan: &ToolsmithPlan) -> Self {
        let proposals = plan.proposals.len();
        let ready = plan.ready_count();
        let held = plan.held_count();
        let rejected_requests = plan.rejected_requests.len();
        let rejected = plan.rejected_count();
        let non_rust_proposals = plan
            .proposals
            .iter()
            .filter(|proposal| !proposal.rust_only())
            .count();
        let rust_gate_passed = plan.passed_rust_gate();
        let telemetry = toolsmith_plan_summary_telemetry(
            proposals,
            ready,
            held,
            rejected,
            rejected_requests,
            non_rust_proposals,
            plan.rust_only,
            rust_gate_passed,
        );

        Self {
            proposals,
            ready,
            held,
            rejected,
            rejected_requests,
            non_rust_proposals,
            rust_only: plan.rust_only,
            rust_gate_passed,
            telemetry,
        }
    }
}

impl ToolsmithPlanSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<ToolsmithPlanSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: ToolsmithPlanSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&ToolsmithPlanSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[ToolsmithPlanSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> ToolsmithPlanDashboard {
        ToolsmithPlanDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: ToolsmithPlanHealthPolicy) -> ToolsmithPlanHealth {
        self.dashboard().health(policy)
    }
}

impl ToolsmithPlanDashboard {
    pub fn from_summaries(summaries: &[ToolsmithPlanSummary]) -> Self {
        let total_records = summaries.len();
        let proposals = summaries
            .iter()
            .map(|summary| summary.proposals)
            .sum::<usize>();
        let ready = summaries.iter().map(|summary| summary.ready).sum::<usize>();
        let held = summaries.iter().map(|summary| summary.held).sum::<usize>();
        let rejected = summaries
            .iter()
            .map(|summary| summary.rejected)
            .sum::<usize>();
        let rejected_requests = summaries
            .iter()
            .map(|summary| summary.rejected_requests)
            .sum::<usize>();
        let non_rust_proposals = summaries
            .iter()
            .map(|summary| summary.non_rust_proposals)
            .sum::<usize>();
        let rust_gate_failed_records = summaries
            .iter()
            .filter(|summary| !summary.rust_gate_passed)
            .count();
        let empty_records = summaries
            .iter()
            .filter(|summary| summary.proposals == 0 && summary.rejected_requests == 0)
            .count();
        let ready_rate = rate(ready, proposals);
        let rust_gate_pass_rate = rate(
            total_records.saturating_sub(rust_gate_failed_records),
            total_records,
        );
        let telemetry = toolsmith_plan_dashboard_telemetry(
            total_records,
            proposals,
            ready,
            held,
            rejected,
            rejected_requests,
            non_rust_proposals,
            rust_gate_failed_records,
            empty_records,
            ready_rate,
            rust_gate_pass_rate,
        );

        Self {
            total_records,
            proposals,
            ready,
            held,
            rejected,
            rejected_requests,
            non_rust_proposals,
            rust_gate_failed_records,
            empty_records,
            ready_rate,
            rust_gate_pass_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: ToolsmithPlanHealthPolicy) -> ToolsmithPlanHealth {
        ToolsmithPlanHealth::from_dashboard(self.clone(), policy)
    }
}

impl ToolsmithPlanHealth {
    pub fn from_dashboard(
        dashboard: ToolsmithPlanDashboard,
        policy: ToolsmithPlanHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("toolsmith_plan_history_empty".to_owned());
        } else if dashboard.ready_rate < policy.minimum_ready_rate {
            watch_reasons.push(format!(
                "toolsmith_plan_ready_rate={:.3}<{}",
                dashboard.ready_rate, policy.minimum_ready_rate
            ));
        }

        if dashboard.rejected > policy.maximum_rejected {
            repair_reasons.push(format!(
                "toolsmith_plan_rejected={}>{}",
                dashboard.rejected, policy.maximum_rejected
            ));
        }

        if dashboard.rejected_requests > policy.maximum_rejected_requests {
            repair_reasons.push(format!(
                "toolsmith_plan_rejected_requests={}>{}",
                dashboard.rejected_requests, policy.maximum_rejected_requests
            ));
        }

        if dashboard.non_rust_proposals > policy.maximum_non_rust_proposals {
            repair_reasons.push(format!(
                "toolsmith_plan_non_rust_proposals={}>{}",
                dashboard.non_rust_proposals, policy.maximum_non_rust_proposals
            ));
        }

        if dashboard.rust_gate_failed_records > policy.maximum_rust_gate_failed_records {
            repair_reasons.push(format!(
                "toolsmith_plan_rust_gate_failed_records={}>{}",
                dashboard.rust_gate_failed_records, policy.maximum_rust_gate_failed_records
            ));
        }

        if dashboard.empty_records > policy.maximum_empty_records {
            watch_reasons.push(format!(
                "toolsmith_plan_empty_records={}>{}",
                dashboard.empty_records, policy.maximum_empty_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (ToolsmithPlanHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (ToolsmithPlanHealthStatus::Watch, watch_reasons)
        } else {
            (ToolsmithPlanHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == ToolsmithPlanHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != ToolsmithPlanHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == ToolsmithPlanHealthStatus::Repair
    }
}

impl ToolsmithPlanSummaryHistoryRecord {
    pub fn records(&self) -> usize {
        self.history.len()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.health.requires_repair_first()
    }
}

impl ToolsmithPlanSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: ToolsmithPlanSummaryHistory,
        summary: ToolsmithPlanSummary,
        policy: ToolsmithPlanHealthPolicy,
    ) -> ToolsmithPlanSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = toolsmith_plan_history_record_telemetry(&dashboard, &health);

        ToolsmithPlanSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_plan_with_health(
        &self,
        history: ToolsmithPlanSummaryHistory,
        plan: &ToolsmithPlan,
        policy: ToolsmithPlanHealthPolicy,
    ) -> ToolsmithPlanSummaryHistoryRecord {
        self.record_summary_with_health(history, plan.summary(), policy)
    }

    pub fn record_plan_with_health_gate(
        &self,
        history: ToolsmithPlanSummaryHistory,
        plan: &ToolsmithPlan,
        policy: ToolsmithPlanHealthPolicy,
    ) -> ToolsmithPlanHistoryGateRecord {
        let health_record = self.record_plan_with_health(history, plan, policy);
        let gate_decision = ToolsmithPlanHistoryGate::new().gate(plan, &health_record);
        let telemetry =
            toolsmith_plan_history_gate_record_telemetry(&health_record, &gate_decision);

        ToolsmithPlanHistoryGateRecord {
            health_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolsmithPlanHistoryGateDecision {
    pub plan_summary: ToolsmithPlanSummary,
    pub toolsmith_health: ToolsmithPlanHealth,
    pub can_promote_ready_proposals: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl ToolsmithPlanHistoryGateDecision {
    pub fn is_promotable(&self) -> bool {
        self.can_promote_ready_proposals && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolsmithPlanHistoryGateRecord {
    pub health_record: ToolsmithPlanSummaryHistoryRecord,
    pub gate_decision: ToolsmithPlanHistoryGateDecision,
    pub telemetry: Vec<String>,
}

impl ToolsmithPlanHistoryGateRecord {
    pub fn records(&self) -> usize {
        self.health_record.records()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn can_promote_ready_proposals(&self) -> bool {
        self.gate_decision.can_promote_ready_proposals
    }
}

#[derive(Debug, Clone, Default)]
pub struct ToolsmithPlanHistoryGate;

impl ToolsmithPlanHistoryGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        plan: &ToolsmithPlan,
        history_record: &ToolsmithPlanSummaryHistoryRecord,
    ) -> ToolsmithPlanHistoryGateDecision {
        let plan_summary = plan.summary();
        let toolsmith_health = history_record.health.clone();
        let mut reasons = toolsmith_plan_gate_reasons(plan, &plan_summary);
        extend_ordered_unique(
            &mut reasons,
            toolsmith_health
                .reasons
                .iter()
                .map(|reason| format!("toolsmith_plan_history:{reason}"))
                .collect::<Vec<_>>(),
        );
        let current_requires_repair = !plan_summary.rust_gate_passed
            || plan_summary.rejected > 0
            || plan_summary.non_rust_proposals > 0
            || reasons
                .iter()
                .any(|reason| reason.starts_with("toolsmith_blueprint_movement:"));
        let requires_repair_first =
            current_requires_repair || toolsmith_health.requires_repair_first();
        let can_promote_ready_proposals = plan_summary.ready > 0
            && plan_summary.rust_gate_passed
            && toolsmith_health.allows_service_advance()
            && !requires_repair_first;
        let repair_tasks =
            toolsmith_plan_history_gate_repair_tasks(requires_repair_first, &reasons);
        let telemetry = toolsmith_plan_history_gate_telemetry(
            can_promote_ready_proposals,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            &plan_summary,
            toolsmith_health.status,
        );

        ToolsmithPlanHistoryGateDecision {
            plan_summary,
            toolsmith_health,
            can_promote_ready_proposals,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardAction {
    Reinforce,
    Hold,
    Penalize,
}

impl RewardAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reinforce => "reinforce",
            Self::Hold => "hold",
            Self::Penalize => "penalize",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessRewardComponents {
    pub coordination: f32,
    pub reflection: f32,
    pub validation: f32,
    pub toolsmith: f32,
    pub recursion: f32,
    pub admission: f32,
}

impl Default for ProcessRewardComponents {
    fn default() -> Self {
        Self {
            coordination: 0.5,
            reflection: 0.5,
            validation: 0.5,
            toolsmith: 0.5,
            recursion: 0.5,
            admission: 0.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessRewardInput {
    pub quality: f32,
    pub validation_passed: bool,
    pub runtime_response_ok: bool,
    pub execution_failures: usize,
    pub reflection_complete: bool,
    pub recursive_chunks: usize,
    pub recursive_waves: usize,
    pub run_report: AgentRunReport,
    pub toolsmith_plan: ToolsmithPlan,
    pub tool_build_report: Option<ToolBuildReport>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionSignal {
    pub target: String,
    pub action: String,
    pub reason: String,
    pub score: f32,
}

impl EvolutionSignal {
    pub fn new(
        target: impl Into<String>,
        action: impl Into<String>,
        reason: impl Into<String>,
        score: f32,
    ) -> Self {
        Self {
            target: target.into(),
            action: action.into(),
            reason: reason.into(),
            score: score.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessRewardReport {
    pub total: f32,
    pub components: ProcessRewardComponents,
    pub action: RewardAction,
    pub notes: Vec<String>,
    pub evolution_signals: Vec<EvolutionSignal>,
}

impl ProcessRewardReport {
    pub fn summary(&self) -> ProcessRewardReportSummary {
        ProcessRewardReportSummary::from_report(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessRewardReportSummary {
    pub total: f32,
    pub action: RewardAction,
    pub signal_count: usize,
    pub note_count: usize,
    pub low_component_count: usize,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessRewardReportHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl ProcessRewardReportHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProcessRewardReportSummaryHistory {
    summaries: Vec<ProcessRewardReportSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessRewardReportDashboard {
    pub total_records: usize,
    pub reinforce_records: usize,
    pub hold_records: usize,
    pub penalize_records: usize,
    pub signal_count: usize,
    pub note_count: usize,
    pub low_component_count: usize,
    pub low_score_records: usize,
    pub missing_signal_records: usize,
    pub average_total: f32,
    pub reinforce_rate: f32,
    pub penalize_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessRewardReportHealthPolicy {
    pub minimum_average_total: f32,
    pub maximum_penalize_records: usize,
    pub maximum_low_score_records: usize,
    pub maximum_missing_signal_records: usize,
    pub maximum_low_component_count: usize,
}

impl Default for ProcessRewardReportHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_average_total: 0.42,
            maximum_penalize_records: 0,
            maximum_low_score_records: 0,
            maximum_missing_signal_records: 0,
            maximum_low_component_count: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessRewardReportHealth {
    pub status: ProcessRewardReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: ProcessRewardReportDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessRewardReportSummaryHistoryRecord {
    pub history: ProcessRewardReportSummaryHistory,
    pub appended_summary: ProcessRewardReportSummary,
    pub dashboard: ProcessRewardReportDashboard,
    pub health: ProcessRewardReportHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessRewardReportSummaryHistoryRecorder;

impl ProcessRewardReportSummary {
    pub fn from_report(report: &ProcessRewardReport) -> Self {
        let low_component_count = [
            report.components.coordination,
            report.components.reflection,
            report.components.validation,
            report.components.toolsmith,
            report.components.recursion,
            report.components.admission,
        ]
        .into_iter()
        .filter(|component| *component < 0.42)
        .count();
        let signal_count = report.evolution_signals.len();
        let note_count = report.notes.len();
        let telemetry = process_reward_report_summary_telemetry(
            report.total,
            report.action,
            signal_count,
            note_count,
            low_component_count,
        );

        Self {
            total: report.total,
            action: report.action,
            signal_count,
            note_count,
            low_component_count,
            telemetry,
        }
    }
}

impl ProcessRewardReportSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<ProcessRewardReportSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: ProcessRewardReportSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&ProcessRewardReportSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[ProcessRewardReportSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> ProcessRewardReportDashboard {
        ProcessRewardReportDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: ProcessRewardReportHealthPolicy) -> ProcessRewardReportHealth {
        self.dashboard().health(policy)
    }
}

impl ProcessRewardReportDashboard {
    pub fn from_summaries(summaries: &[ProcessRewardReportSummary]) -> Self {
        let total_records = summaries.len();
        let reinforce_records = summaries
            .iter()
            .filter(|summary| summary.action == RewardAction::Reinforce)
            .count();
        let hold_records = summaries
            .iter()
            .filter(|summary| summary.action == RewardAction::Hold)
            .count();
        let penalize_records = summaries
            .iter()
            .filter(|summary| summary.action == RewardAction::Penalize)
            .count();
        let signal_count = summaries
            .iter()
            .map(|summary| summary.signal_count)
            .sum::<usize>();
        let note_count = summaries
            .iter()
            .map(|summary| summary.note_count)
            .sum::<usize>();
        let low_component_count = summaries
            .iter()
            .map(|summary| summary.low_component_count)
            .sum::<usize>();
        let low_score_records = summaries
            .iter()
            .filter(|summary| summary.total < 0.42)
            .count();
        let missing_signal_records = summaries
            .iter()
            .filter(|summary| summary.signal_count == 0)
            .count();
        let total_score = summaries.iter().map(|summary| summary.total).sum::<f32>();
        let average_total = if total_records == 0 {
            0.0
        } else {
            total_score / total_records as f32
        };
        let reinforce_rate = rate(reinforce_records, total_records);
        let penalize_rate = rate(penalize_records, total_records);
        let telemetry = process_reward_report_dashboard_telemetry(
            total_records,
            reinforce_records,
            hold_records,
            penalize_records,
            signal_count,
            note_count,
            low_component_count,
            low_score_records,
            missing_signal_records,
            average_total,
            reinforce_rate,
            penalize_rate,
        );

        Self {
            total_records,
            reinforce_records,
            hold_records,
            penalize_records,
            signal_count,
            note_count,
            low_component_count,
            low_score_records,
            missing_signal_records,
            average_total,
            reinforce_rate,
            penalize_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: ProcessRewardReportHealthPolicy) -> ProcessRewardReportHealth {
        ProcessRewardReportHealth::from_dashboard(self.clone(), policy)
    }
}

impl ProcessRewardReportHealth {
    pub fn from_dashboard(
        dashboard: ProcessRewardReportDashboard,
        policy: ProcessRewardReportHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("process_reward_report_history_empty".to_owned());
        } else if dashboard.average_total < policy.minimum_average_total {
            watch_reasons.push(format!(
                "process_reward_report_average_total={:.3}<{}",
                dashboard.average_total, policy.minimum_average_total
            ));
        }

        if dashboard.penalize_records > policy.maximum_penalize_records {
            repair_reasons.push(format!(
                "process_reward_report_penalize_records={}>{}",
                dashboard.penalize_records, policy.maximum_penalize_records
            ));
        }

        if dashboard.low_score_records > policy.maximum_low_score_records {
            repair_reasons.push(format!(
                "process_reward_report_low_score_records={}>{}",
                dashboard.low_score_records, policy.maximum_low_score_records
            ));
        }

        if dashboard.missing_signal_records > policy.maximum_missing_signal_records {
            repair_reasons.push(format!(
                "process_reward_report_missing_signal_records={}>{}",
                dashboard.missing_signal_records, policy.maximum_missing_signal_records
            ));
        }

        if dashboard.low_component_count > policy.maximum_low_component_count {
            watch_reasons.push(format!(
                "process_reward_report_low_component_count={}>{}",
                dashboard.low_component_count, policy.maximum_low_component_count
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (ProcessRewardReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (ProcessRewardReportHealthStatus::Watch, watch_reasons)
        } else {
            (ProcessRewardReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == ProcessRewardReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != ProcessRewardReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == ProcessRewardReportHealthStatus::Repair
    }
}

impl ProcessRewardReportSummaryHistoryRecord {
    pub fn records(&self) -> usize {
        self.history.len()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.health.requires_repair_first()
    }
}

impl ProcessRewardReportSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: ProcessRewardReportSummaryHistory,
        summary: ProcessRewardReportSummary,
        policy: ProcessRewardReportHealthPolicy,
    ) -> ProcessRewardReportSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = process_reward_report_history_record_telemetry(&dashboard, &health);

        ProcessRewardReportSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_report_with_health(
        &self,
        history: ProcessRewardReportSummaryHistory,
        report: &ProcessRewardReport,
        policy: ProcessRewardReportHealthPolicy,
    ) -> ProcessRewardReportSummaryHistoryRecord {
        self.record_summary_with_health(history, report.summary(), policy)
    }

    pub fn record_report_with_health_gate(
        &self,
        history: ProcessRewardReportSummaryHistory,
        report: &ProcessRewardReport,
        policy: ProcessRewardReportHealthPolicy,
    ) -> ProcessRewardReportHistoryGateRecord {
        let health_record = self.record_report_with_health(history, report, policy);
        let gate_decision = ProcessRewardReportHistoryGate::new().gate(report, &health_record);
        let telemetry =
            process_reward_report_history_gate_record_telemetry(&health_record, &gate_decision);

        ProcessRewardReportHistoryGateRecord {
            health_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessRewardReportHistoryGateDecision {
    pub report_summary: ProcessRewardReportSummary,
    pub reward_health: ProcessRewardReportHealth,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl ProcessRewardReportHistoryGateDecision {
    pub fn is_promotable(&self) -> bool {
        self.can_promote_evolution_signals && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessRewardReportHistoryGateRecord {
    pub health_record: ProcessRewardReportSummaryHistoryRecord,
    pub gate_decision: ProcessRewardReportHistoryGateDecision,
    pub telemetry: Vec<String>,
}

impl ProcessRewardReportHistoryGateRecord {
    pub fn records(&self) -> usize {
        self.health_record.records()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn can_promote_evolution_signals(&self) -> bool {
        self.gate_decision.can_promote_evolution_signals
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReflectionRewardAdmissionRecord {
    pub reflection_record: ReflectionLoopHistoryGateRecord,
    pub reward_record: ProcessRewardReportHistoryGateRecord,
    pub can_continue_reflection: bool,
    pub can_promote_memory_note: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl ReflectionRewardAdmissionRecord {
    pub fn is_admitted(&self) -> bool {
        self.can_promote_memory_note
            && self.can_promote_evolution_signals
            && !self.requires_repair_first
    }

    pub fn repair_task_ids(&self) -> Vec<String> {
        self.repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect()
    }

    pub fn summary(&self) -> ReflectionRewardAdmissionSummary {
        ReflectionRewardAdmissionSummary::from_record(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectionRewardAdmissionSummary {
    pub reflection_health_status: ReflectionLoopHealthStatus,
    pub reward_health_status: ProcessRewardReportHealthStatus,
    pub can_continue_reflection: bool,
    pub can_promote_memory_note: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: usize,
    pub blocked_reasons: usize,
    pub repair_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl ReflectionRewardAdmissionSummary {
    pub fn from_record(record: &ReflectionRewardAdmissionRecord) -> Self {
        let repair_task_ids = record.repair_task_ids();
        let telemetry = reflection_reward_admission_summary_telemetry(
            record
                .reflection_record
                .gate_decision
                .reflection_health
                .status,
            record.reward_record.gate_decision.reward_health.status,
            record.can_continue_reflection,
            record.can_promote_memory_note,
            record.can_promote_evolution_signals,
            record.can_reinforce_process,
            record.requires_repair_first,
            repair_task_ids.len(),
            record.blocked_reasons.len(),
        );

        Self {
            reflection_health_status: record
                .reflection_record
                .gate_decision
                .reflection_health
                .status,
            reward_health_status: record.reward_record.gate_decision.reward_health.status,
            can_continue_reflection: record.can_continue_reflection,
            can_promote_memory_note: record.can_promote_memory_note,
            can_promote_evolution_signals: record.can_promote_evolution_signals,
            can_reinforce_process: record.can_reinforce_process,
            requires_repair_first: record.requires_repair_first,
            repair_tasks: repair_task_ids.len(),
            blocked_reasons: record.blocked_reasons.len(),
            repair_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReflectionRewardAdmissionGate;

impl ReflectionRewardAdmissionGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        reflection_record: ReflectionLoopHistoryGateRecord,
        reward_record: ProcessRewardReportHistoryGateRecord,
    ) -> ReflectionRewardAdmissionRecord {
        let reflection_requires_repair = reflection_record.requires_repair_first();
        let reward_requires_repair = reward_record.requires_repair_first();
        let requires_repair_first = reflection_requires_repair || reward_requires_repair;
        let can_continue_reflection = reflection_record.gate_decision.can_continue_reflection
            && reward_record.allows_service_advance()
            && !requires_repair_first;
        let reflection_memory_promotable = reflection_record.gate_decision.is_memory_promotable();
        let can_promote_memory_note = reflection_memory_promotable
            && reward_record.allows_service_advance()
            && !requires_repair_first;
        let can_promote_evolution_signals = can_promote_memory_note
            && reward_record.gate_decision.can_promote_evolution_signals
            && !requires_repair_first;
        let can_reinforce_process = can_promote_evolution_signals
            && reward_record.gate_decision.can_reinforce_process
            && !requires_repair_first;
        let mut blocked_reasons = reflection_record
            .gate_decision
            .reasons
            .iter()
            .map(|reason| format!("reflection:{reason}"))
            .collect::<Vec<_>>();
        extend_ordered_unique(
            &mut blocked_reasons,
            reward_record
                .gate_decision
                .reasons
                .iter()
                .map(|reason| format!("process_reward:{reason}"))
                .collect::<Vec<_>>(),
        );
        let mut repair_tasks = reflection_record.gate_decision.repair_tasks.clone();
        repair_tasks.extend(reward_record.gate_decision.repair_tasks.clone());
        let telemetry = reflection_reward_admission_telemetry(
            reflection_record.gate_decision.reflection_health.status,
            reward_record.gate_decision.reward_health.status,
            can_continue_reflection,
            can_promote_memory_note,
            can_promote_evolution_signals,
            can_reinforce_process,
            requires_repair_first,
            repair_tasks.len(),
            blocked_reasons.len(),
        );

        ReflectionRewardAdmissionRecord {
            reflection_record,
            reward_record,
            can_continue_reflection,
            can_promote_memory_note,
            can_promote_evolution_signals,
            can_reinforce_process,
            requires_repair_first,
            repair_tasks,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProcessRewardReportHistoryGate;

impl ProcessRewardReportHistoryGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        report: &ProcessRewardReport,
        history_record: &ProcessRewardReportSummaryHistoryRecord,
    ) -> ProcessRewardReportHistoryGateDecision {
        let report_summary = report.summary();
        let reward_health = history_record.health.clone();
        let mut reasons = process_reward_report_gate_reasons(&report_summary);
        extend_ordered_unique(
            &mut reasons,
            reward_health
                .reasons
                .iter()
                .map(|reason| format!("process_reward_report_history:{reason}"))
                .collect::<Vec<_>>(),
        );
        let current_requires_repair = report_summary.action == RewardAction::Penalize
            || report_summary.total < 0.42
            || report_summary.signal_count == 0;
        let requires_repair_first =
            current_requires_repair || reward_health.requires_repair_first();
        let can_promote_evolution_signals = report_summary.signal_count > 0
            && reward_health.allows_service_advance()
            && !requires_repair_first;
        let can_reinforce_process = can_promote_evolution_signals
            && report_summary.action == RewardAction::Reinforce
            && report.evolution_signals.iter().any(|signal| {
                signal.action == "promote_closed_loop_pattern" || signal.action == "reinforce"
            });
        let repair_tasks =
            process_reward_report_history_gate_repair_tasks(requires_repair_first, &reasons);
        let telemetry = process_reward_report_history_gate_telemetry(
            can_promote_evolution_signals,
            can_reinforce_process,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            &report_summary,
            reward_health.status,
        );

        ProcessRewardReportHistoryGateDecision {
            report_summary,
            reward_health,
            can_promote_evolution_signals,
            can_reinforce_process,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionDecision {
    pub toolsmith_health_status: ToolsmithPlanHealthStatus,
    pub reward_health_status: ProcessRewardReportHealthStatus,
    pub can_promote_ready_proposals: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl EvolutionAdmissionDecision {
    pub fn is_admitted(&self) -> bool {
        (self.can_promote_ready_proposals
            || self.can_promote_evolution_signals
            || self.can_reinforce_process
            || self.can_promote_adaptive_state)
            && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionRecord {
    pub toolsmith_record: ToolsmithPlanHistoryGateRecord,
    pub reward_record: ProcessRewardReportHistoryGateRecord,
    pub decision: EvolutionAdmissionDecision,
    pub telemetry: Vec<String>,
}

impl EvolutionAdmissionRecord {
    pub fn records(&self) -> usize {
        self.toolsmith_record
            .records()
            .max(self.reward_record.records())
    }

    pub fn allows_service_advance(&self) -> bool {
        self.toolsmith_record.allows_service_advance()
            && self.reward_record.allows_service_advance()
            && !self.decision.requires_repair_first
    }

    pub fn requires_repair_first(&self) -> bool {
        self.decision.requires_repair_first
    }

    pub fn can_promote_ready_proposals(&self) -> bool {
        self.decision.can_promote_ready_proposals
    }

    pub fn can_promote_evolution_signals(&self) -> bool {
        self.decision.can_promote_evolution_signals
    }

    pub fn can_promote_adaptive_state(&self) -> bool {
        self.decision.can_promote_adaptive_state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvolutionAdmissionHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl EvolutionAdmissionHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionSummary {
    pub records: usize,
    pub admitted: bool,
    pub can_promote_ready_proposals: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub repair_task_count: usize,
    pub blocked_reason_count: usize,
    pub toolsmith_health_status: ToolsmithPlanHealthStatus,
    pub reward_health_status: ProcessRewardReportHealthStatus,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EvolutionAdmissionSummaryHistory {
    summaries: Vec<EvolutionAdmissionSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionDashboard {
    pub total_records: usize,
    pub admitted_records: usize,
    pub repair_first_records: usize,
    pub ready_promotion_records: usize,
    pub signal_promotion_records: usize,
    pub reinforcement_records: usize,
    pub adaptive_state_records: usize,
    pub repair_task_count: usize,
    pub blocked_reason_count: usize,
    pub toolsmith_repair_records: usize,
    pub reward_repair_records: usize,
    pub admission_rate: f32,
    pub adaptive_state_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvolutionAdmissionHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_repair_task_count: usize,
    pub maximum_blocked_reason_count: usize,
    pub maximum_toolsmith_repair_records: usize,
    pub maximum_reward_repair_records: usize,
}

impl Default for EvolutionAdmissionHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_admission_rate: 0.0,
            maximum_repair_first_records: 0,
            maximum_repair_task_count: 0,
            maximum_blocked_reason_count: 0,
            maximum_toolsmith_repair_records: 0,
            maximum_reward_repair_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHealth {
    pub status: EvolutionAdmissionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: EvolutionAdmissionDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionSummaryHistoryRecord {
    pub history: EvolutionAdmissionSummaryHistory,
    pub appended_summary: EvolutionAdmissionSummary,
    pub dashboard: EvolutionAdmissionDashboard,
    pub health: EvolutionAdmissionHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EvolutionAdmissionSummaryHistoryRecorder;

impl EvolutionAdmissionSummary {
    pub fn from_record(record: &EvolutionAdmissionRecord) -> Self {
        let decision = &record.decision;
        let telemetry = evolution_admission_summary_telemetry(
            record.records(),
            decision.is_admitted(),
            decision.can_promote_ready_proposals,
            decision.can_promote_evolution_signals,
            decision.can_reinforce_process,
            decision.can_promote_adaptive_state,
            decision.requires_repair_first,
            decision.repair_tasks.len(),
            decision.blocked_reasons.len(),
            decision.toolsmith_health_status,
            decision.reward_health_status,
        );

        Self {
            records: record.records(),
            admitted: decision.is_admitted(),
            can_promote_ready_proposals: decision.can_promote_ready_proposals,
            can_promote_evolution_signals: decision.can_promote_evolution_signals,
            can_reinforce_process: decision.can_reinforce_process,
            can_promote_adaptive_state: decision.can_promote_adaptive_state,
            requires_repair_first: decision.requires_repair_first,
            repair_task_count: decision.repair_tasks.len(),
            blocked_reason_count: decision.blocked_reasons.len(),
            toolsmith_health_status: decision.toolsmith_health_status,
            reward_health_status: decision.reward_health_status,
            telemetry,
        }
    }
}

impl EvolutionAdmissionSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<EvolutionAdmissionSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: EvolutionAdmissionSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&EvolutionAdmissionSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[EvolutionAdmissionSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> EvolutionAdmissionDashboard {
        EvolutionAdmissionDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: EvolutionAdmissionHealthPolicy) -> EvolutionAdmissionHealth {
        self.dashboard().health(policy)
    }
}

impl EvolutionAdmissionDashboard {
    pub fn from_summaries(summaries: &[EvolutionAdmissionSummary]) -> Self {
        let total_records = summaries.len();
        let admitted_records = summaries.iter().filter(|summary| summary.admitted).count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let ready_promotion_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_ready_proposals)
            .count();
        let signal_promotion_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_evolution_signals)
            .count();
        let reinforcement_records = summaries
            .iter()
            .filter(|summary| summary.can_reinforce_process)
            .count();
        let adaptive_state_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_task_count)
            .sum::<usize>();
        let blocked_reason_count = summaries
            .iter()
            .map(|summary| summary.blocked_reason_count)
            .sum::<usize>();
        let toolsmith_repair_records = summaries
            .iter()
            .filter(|summary| summary.toolsmith_health_status == ToolsmithPlanHealthStatus::Repair)
            .count();
        let reward_repair_records = summaries
            .iter()
            .filter(|summary| {
                summary.reward_health_status == ProcessRewardReportHealthStatus::Repair
            })
            .count();
        let admission_rate = rate(admitted_records, total_records);
        let adaptive_state_rate = rate(adaptive_state_records, total_records);
        let telemetry = evolution_admission_dashboard_telemetry(
            total_records,
            admitted_records,
            repair_first_records,
            ready_promotion_records,
            signal_promotion_records,
            reinforcement_records,
            adaptive_state_records,
            repair_task_count,
            blocked_reason_count,
            toolsmith_repair_records,
            reward_repair_records,
            admission_rate,
            adaptive_state_rate,
        );

        Self {
            total_records,
            admitted_records,
            repair_first_records,
            ready_promotion_records,
            signal_promotion_records,
            reinforcement_records,
            adaptive_state_records,
            repair_task_count,
            blocked_reason_count,
            toolsmith_repair_records,
            reward_repair_records,
            admission_rate,
            adaptive_state_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: EvolutionAdmissionHealthPolicy) -> EvolutionAdmissionHealth {
        EvolutionAdmissionHealth::from_dashboard(self.clone(), policy)
    }
}

impl EvolutionAdmissionHealth {
    pub fn from_dashboard(
        dashboard: EvolutionAdmissionDashboard,
        policy: EvolutionAdmissionHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("evolution_admission_history_empty".to_owned());
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "evolution_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "evolution_admission_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_task_count {
            repair_reasons.push(format!(
                "evolution_admission_repair_task_count={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_task_count
            ));
        }

        if dashboard.blocked_reason_count > policy.maximum_blocked_reason_count {
            repair_reasons.push(format!(
                "evolution_admission_blocked_reason_count={}>{}",
                dashboard.blocked_reason_count, policy.maximum_blocked_reason_count
            ));
        }

        if dashboard.toolsmith_repair_records > policy.maximum_toolsmith_repair_records {
            repair_reasons.push(format!(
                "evolution_admission_toolsmith_repair_records={}>{}",
                dashboard.toolsmith_repair_records, policy.maximum_toolsmith_repair_records
            ));
        }

        if dashboard.reward_repair_records > policy.maximum_reward_repair_records {
            repair_reasons.push(format!(
                "evolution_admission_reward_repair_records={}>{}",
                dashboard.reward_repair_records, policy.maximum_reward_repair_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (EvolutionAdmissionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (EvolutionAdmissionHealthStatus::Watch, watch_reasons)
        } else {
            (EvolutionAdmissionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == EvolutionAdmissionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != EvolutionAdmissionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == EvolutionAdmissionHealthStatus::Repair
    }
}

impl EvolutionAdmissionSummaryHistoryRecord {
    pub fn records(&self) -> usize {
        self.history.len()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.health.requires_repair_first()
    }
}

impl EvolutionAdmissionSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: EvolutionAdmissionSummaryHistory,
        summary: EvolutionAdmissionSummary,
        policy: EvolutionAdmissionHealthPolicy,
    ) -> EvolutionAdmissionSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = evolution_admission_history_record_telemetry(&dashboard, &health);

        EvolutionAdmissionSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_admission_with_health(
        &self,
        history: EvolutionAdmissionSummaryHistory,
        admission: &EvolutionAdmissionRecord,
        policy: EvolutionAdmissionHealthPolicy,
    ) -> EvolutionAdmissionSummaryHistoryRecord {
        self.record_summary_with_health(
            history,
            EvolutionAdmissionSummary::from_record(admission),
            policy,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHistoryGateDecision {
    pub admission_summary: EvolutionAdmissionSummary,
    pub admission_health: EvolutionAdmissionHealth,
    pub can_promote_ready_proposals: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl EvolutionAdmissionHistoryGateDecision {
    pub fn is_admitted(&self) -> bool {
        (self.can_promote_ready_proposals
            || self.can_promote_evolution_signals
            || self.can_reinforce_process
            || self.can_promote_adaptive_state)
            && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHistoryGateRecord {
    pub health_record: EvolutionAdmissionSummaryHistoryRecord,
    pub gate_decision: EvolutionAdmissionHistoryGateDecision,
    pub telemetry: Vec<String>,
}

impl EvolutionAdmissionHistoryGateRecord {
    pub fn records(&self) -> usize {
        self.health_record.records()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health_record.allows_service_advance() && !self.gate_decision.requires_repair_first
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn can_promote_ready_proposals(&self) -> bool {
        self.gate_decision.can_promote_ready_proposals
    }

    pub fn can_promote_evolution_signals(&self) -> bool {
        self.gate_decision.can_promote_evolution_signals
    }

    pub fn can_promote_adaptive_state(&self) -> bool {
        self.gate_decision.can_promote_adaptive_state
    }
}

#[derive(Debug, Clone, Default)]
pub struct EvolutionAdmissionHistoryGate;

impl EvolutionAdmissionHistoryGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        admission: &EvolutionAdmissionRecord,
        history_record: &EvolutionAdmissionSummaryHistoryRecord,
    ) -> EvolutionAdmissionHistoryGateDecision {
        let admission_summary = EvolutionAdmissionSummary::from_record(admission);
        let admission_health = history_record.health.clone();
        let mut blocked_reasons = admission.decision.blocked_reasons.clone();
        extend_ordered_unique(
            &mut blocked_reasons,
            admission_health
                .reasons
                .iter()
                .map(|reason| format!("evolution_admission_history:{reason}"))
                .collect::<Vec<_>>(),
        );
        let history_requires_repair = admission_health.requires_repair_first();
        let requires_repair_first = admission.requires_repair_first() || history_requires_repair;
        let mut repair_tasks = admission.decision.repair_tasks.clone();
        repair_tasks.extend(evolution_admission_history_gate_repair_tasks(
            history_requires_repair,
            &admission_health.reasons,
        ));
        let can_promote_ready_proposals = admission.can_promote_ready_proposals()
            && admission_health.allows_service_advance()
            && !requires_repair_first;
        let can_promote_evolution_signals = admission.can_promote_evolution_signals()
            && admission_health.allows_service_advance()
            && !requires_repair_first;
        let can_reinforce_process = admission.decision.can_reinforce_process
            && admission_health.allows_service_advance()
            && !requires_repair_first;
        let can_promote_adaptive_state = admission.can_promote_adaptive_state()
            && admission_health.is_stable()
            && !requires_repair_first;
        let telemetry = evolution_admission_history_gate_telemetry(
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks.len(),
            blocked_reasons.len(),
            admission_summary.admitted,
            admission_health.status,
        );

        EvolutionAdmissionHistoryGateDecision {
            admission_summary,
            admission_health,
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks,
            blocked_reasons,
            telemetry,
        }
    }
}

impl EvolutionAdmissionSummaryHistoryRecorder {
    pub fn record_admission_with_health_gate(
        &self,
        history: EvolutionAdmissionSummaryHistory,
        admission: &EvolutionAdmissionRecord,
        policy: EvolutionAdmissionHealthPolicy,
    ) -> EvolutionAdmissionHistoryGateRecord {
        let health_record = self.record_admission_with_health(history, admission, policy);
        let gate_decision = EvolutionAdmissionHistoryGate::new().gate(admission, &health_record);
        let telemetry =
            evolution_admission_history_gate_record_telemetry(&health_record, &gate_decision);

        EvolutionAdmissionHistoryGateRecord {
            health_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoff {
    pub gate_record: EvolutionAdmissionHistoryGateRecord,
    pub effective_admitted: bool,
    pub can_promote_ready_proposals: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionAdmissionHandoffSummary {
    pub admission_health_status: EvolutionAdmissionHealthStatus,
    pub requested_admitted: bool,
    pub effective_admitted: bool,
    pub requires_repair_first: bool,
    pub can_promote_ready_proposals: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub can_promote_adaptive_state: bool,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub records: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvolutionAdmissionHandoffSummaryHistory {
    summaries: Vec<EvolutionAdmissionHandoffSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffDashboard {
    pub total_records: usize,
    pub requested_admitted_records: usize,
    pub effective_admitted_records: usize,
    pub repair_first_records: usize,
    pub ready_promotion_records: usize,
    pub signal_promotion_records: usize,
    pub reinforcement_records: usize,
    pub adaptive_state_records: usize,
    pub repair_task_count: usize,
    pub next_queue_task_count: usize,
    pub blocked_reason_count: usize,
    pub admission_repair_records: usize,
    pub effective_admitted_rate: f32,
    pub adaptive_state_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvolutionAdmissionHandoffHealthPolicy {
    pub minimum_effective_admitted_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_admission_repair_records: usize,
    pub maximum_next_queue_tasks: usize,
}

impl Default for EvolutionAdmissionHandoffHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_effective_admitted_rate: 0.0,
            maximum_repair_first_records: 0,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: usize::MAX,
            maximum_admission_repair_records: 0,
            maximum_next_queue_tasks: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffHealth {
    pub status: EvolutionAdmissionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: EvolutionAdmissionHandoffDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffSummaryHistoryRecord {
    pub history: EvolutionAdmissionHandoffSummaryHistory,
    pub appended_summary: EvolutionAdmissionHandoffSummary,
    pub dashboard: EvolutionAdmissionHandoffDashboard,
    pub health: EvolutionAdmissionHandoffHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EvolutionAdmissionHandoffSummaryHistoryRecorder;

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendGateDecision {
    pub requested_admitted: bool,
    pub effective_admitted: bool,
    pub handoff_health: EvolutionAdmissionHandoffHealth,
    pub requires_repair_first: bool,
    pub can_promote_ready_proposals: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub can_promote_adaptive_state: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EvolutionAdmissionHandoffTrendGate;

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendMonitorRecord {
    pub history_record: EvolutionAdmissionHandoffSummaryHistoryRecord,
    pub gate_decision: EvolutionAdmissionHandoffTrendGateDecision,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EvolutionAdmissionHandoffTrendMonitor;

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendContinuation {
    pub next_queue: AgentTaskQueue,
    pub handoff_history: EvolutionAdmissionHandoffSummaryHistory,
    pub handoff_policy: EvolutionAdmissionHandoffHealthPolicy,
    pub handoff_health_status: EvolutionAdmissionHealthStatus,
    pub effective_admitted: bool,
    pub can_promote_ready_proposals: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionAdmissionHandoffTrendContinuationSummary {
    pub handoff_health_status: EvolutionAdmissionHealthStatus,
    pub effective_admitted: bool,
    pub can_promote_ready_proposals: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub next_queue_tasks: usize,
    pub handoff_history_records: usize,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvolutionAdmissionHandoffTrendContinuationSummaryHistory {
    summaries: Vec<EvolutionAdmissionHandoffTrendContinuationSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendContinuationDashboard {
    pub total_records: usize,
    pub effective_admitted_records: usize,
    pub repair_first_records: usize,
    pub ready_promotion_records: usize,
    pub signal_promotion_records: usize,
    pub reinforcement_records: usize,
    pub adaptive_state_records: usize,
    pub next_queue_task_count: usize,
    pub handoff_history_record_count: usize,
    pub handoff_repair_records: usize,
    pub effective_admitted_rate: f32,
    pub adaptive_state_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendContinuationHealthPolicy {
    pub minimum_effective_admitted_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_next_queue_tasks: usize,
    pub maximum_handoff_history_records: usize,
    pub maximum_handoff_repair_records: usize,
}

impl Default for EvolutionAdmissionHandoffTrendContinuationHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_effective_admitted_rate: 0.0,
            maximum_repair_first_records: 0,
            maximum_next_queue_tasks: usize::MAX,
            maximum_handoff_history_records: usize::MAX,
            maximum_handoff_repair_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendContinuationHealth {
    pub status: EvolutionAdmissionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: EvolutionAdmissionHandoffTrendContinuationDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryRecord {
    pub history: EvolutionAdmissionHandoffTrendContinuationSummaryHistory,
    pub appended_summary: EvolutionAdmissionHandoffTrendContinuationSummary,
    pub dashboard: EvolutionAdmissionHandoffTrendContinuationDashboard,
    pub health: EvolutionAdmissionHandoffTrendContinuationHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryRecorder;

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryGateDecision {
    pub continuation_summary: EvolutionAdmissionHandoffTrendContinuationSummary,
    pub continuation_health: EvolutionAdmissionHandoffTrendContinuationHealth,
    pub effective_admitted: bool,
    pub can_promote_ready_proposals: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary {
    pub continuation_health_status: EvolutionAdmissionHealthStatus,
    pub effective_admitted: bool,
    pub can_promote_ready_proposals: bool,
    pub can_promote_evolution_signals: bool,
    pub can_reinforce_process: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub records: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory {
    summaries: Vec<EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryGateDashboard {
    pub total_records: usize,
    pub effective_admitted_records: usize,
    pub repair_first_records: usize,
    pub ready_promotion_records: usize,
    pub signal_promotion_records: usize,
    pub reinforcement_records: usize,
    pub adaptive_state_records: usize,
    pub repair_task_count: usize,
    pub next_queue_task_count: usize,
    pub blocked_reason_count: usize,
    pub continuation_repair_records: usize,
    pub effective_admitted_rate: f32,
    pub adaptive_state_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy {
    pub minimum_effective_admitted_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_continuation_repair_records: usize,
    pub maximum_next_queue_tasks: usize,
}

impl Default for EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_effective_admitted_rate: 0.0,
            maximum_repair_first_records: 0,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: usize::MAX,
            maximum_continuation_repair_records: 0,
            maximum_next_queue_tasks: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryGateHealth {
    pub status: EvolutionAdmissionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: EvolutionAdmissionHandoffTrendContinuationHistoryGateDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord {
    pub history: EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory,
    pub appended_summary: EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary,
    pub dashboard: EvolutionAdmissionHandoffTrendContinuationHistoryGateDashboard,
    pub health: EvolutionAdmissionHandoffTrendContinuationHistoryGateHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder;

#[derive(Debug, Clone, PartialEq)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryGateRecord {
    pub health_record: EvolutionAdmissionHandoffTrendContinuationHistoryRecord,
    pub gate_decision: EvolutionAdmissionHandoffTrendContinuationHistoryGateDecision,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EvolutionAdmissionHandoffTrendContinuationHistoryGate;

#[derive(Debug, Clone, Default)]
pub struct EvolutionAdmissionHandoffTrendContinuationPlanner;

impl EvolutionAdmissionHandoff {
    pub fn from_gate_record(
        gate_record: EvolutionAdmissionHistoryGateRecord,
        next_queue: AgentTaskQueue,
    ) -> Self {
        let gate_decision = &gate_record.gate_decision;
        let effective_admitted =
            gate_decision.is_admitted() && gate_record.allows_service_advance();
        let can_promote_ready_proposals = gate_decision.can_promote_ready_proposals;
        let can_promote_evolution_signals = gate_decision.can_promote_evolution_signals;
        let can_reinforce_process = gate_decision.can_reinforce_process;
        let can_promote_adaptive_state = gate_decision.can_promote_adaptive_state;
        let requires_repair_first = gate_decision.requires_repair_first;
        let repair_tasks = gate_decision.repair_tasks.clone();
        let blocked_reasons = gate_decision.blocked_reasons.clone();
        let mut next_queue = next_queue;
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let telemetry = evolution_admission_handoff_telemetry(
            effective_admitted,
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            blocked_reasons.len(),
            gate_record.records(),
        );

        Self {
            gate_record,
            effective_admitted,
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks,
            next_queue,
            blocked_reasons,
            telemetry,
        }
    }

    pub fn is_admitted(&self) -> bool {
        self.effective_admitted
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.next_queue.clone()
    }

    pub fn summary(&self) -> EvolutionAdmissionHandoffSummary {
        EvolutionAdmissionHandoffSummary::from_handoff(self)
    }
}

impl EvolutionAdmissionHandoffSummary {
    pub fn from_handoff(handoff: &EvolutionAdmissionHandoff) -> Self {
        let repair_task_ids = handoff
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = handoff.next_queue.task_ids();
        let telemetry = evolution_admission_handoff_summary_telemetry(
            handoff.effective_admitted,
            handoff.requires_repair_first,
            handoff.repair_tasks.len(),
            next_queue_task_ids.len(),
            handoff.blocked_reasons.len(),
            handoff.gate_record.records(),
        );

        Self {
            admission_health_status: handoff.gate_record.gate_decision.admission_health.status,
            requested_admitted: handoff.gate_record.gate_decision.is_admitted(),
            effective_admitted: handoff.effective_admitted,
            requires_repair_first: handoff.requires_repair_first,
            can_promote_ready_proposals: handoff.can_promote_ready_proposals,
            can_promote_evolution_signals: handoff.can_promote_evolution_signals,
            can_reinforce_process: handoff.can_reinforce_process,
            can_promote_adaptive_state: handoff.can_promote_adaptive_state,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: handoff.blocked_reasons.len(),
            records: handoff.gate_record.records(),
            repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

impl EvolutionAdmissionHandoffSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<EvolutionAdmissionHandoffSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: EvolutionAdmissionHandoffSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&EvolutionAdmissionHandoffSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[EvolutionAdmissionHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> EvolutionAdmissionHandoffDashboard {
        EvolutionAdmissionHandoffDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: EvolutionAdmissionHandoffHealthPolicy,
    ) -> EvolutionAdmissionHandoffHealth {
        self.dashboard().health(policy)
    }
}

impl EvolutionAdmissionHandoffDashboard {
    pub fn from_summaries(summaries: &[EvolutionAdmissionHandoffSummary]) -> Self {
        let total_records = summaries.len();
        let requested_admitted_records = summaries
            .iter()
            .filter(|summary| summary.requested_admitted)
            .count();
        let effective_admitted_records = summaries
            .iter()
            .filter(|summary| summary.effective_admitted)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let ready_promotion_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_ready_proposals)
            .count();
        let signal_promotion_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_evolution_signals)
            .count();
        let reinforcement_records = summaries
            .iter()
            .filter(|summary| summary.can_reinforce_process)
            .count();
        let adaptive_state_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let next_queue_task_count = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reason_count = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let admission_repair_records = summaries
            .iter()
            .filter(|summary| {
                summary.admission_health_status == EvolutionAdmissionHealthStatus::Repair
            })
            .count();
        let effective_admitted_rate = rate(effective_admitted_records, total_records);
        let adaptive_state_rate = rate(adaptive_state_records, total_records);
        let telemetry = evolution_admission_handoff_dashboard_telemetry(
            total_records,
            requested_admitted_records,
            effective_admitted_records,
            repair_first_records,
            ready_promotion_records,
            signal_promotion_records,
            reinforcement_records,
            adaptive_state_records,
            repair_task_count,
            next_queue_task_count,
            blocked_reason_count,
            admission_repair_records,
            effective_admitted_rate,
            adaptive_state_rate,
        );

        Self {
            total_records,
            requested_admitted_records,
            effective_admitted_records,
            repair_first_records,
            ready_promotion_records,
            signal_promotion_records,
            reinforcement_records,
            adaptive_state_records,
            repair_task_count,
            next_queue_task_count,
            blocked_reason_count,
            admission_repair_records,
            effective_admitted_rate,
            adaptive_state_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: EvolutionAdmissionHandoffHealthPolicy,
    ) -> EvolutionAdmissionHandoffHealth {
        EvolutionAdmissionHandoffHealth::from_dashboard(self.clone(), policy)
    }
}

impl EvolutionAdmissionHandoffHealth {
    pub fn from_dashboard(
        dashboard: EvolutionAdmissionHandoffDashboard,
        policy: EvolutionAdmissionHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("evolution_admission_handoff_history_empty".to_owned());
        } else if dashboard.effective_admitted_rate < policy.minimum_effective_admitted_rate {
            watch_reasons.push(format!(
                "evolution_admission_handoff_effective_admitted_rate={:.3}<{}",
                dashboard.effective_admitted_rate, policy.minimum_effective_admitted_rate
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "evolution_admission_handoff_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "evolution_admission_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reason_count > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "evolution_admission_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reason_count, policy.maximum_blocked_reasons
            ));
        }

        if dashboard.admission_repair_records > policy.maximum_admission_repair_records {
            repair_reasons.push(format!(
                "evolution_admission_handoff_admission_repair_records={}>{}",
                dashboard.admission_repair_records, policy.maximum_admission_repair_records
            ));
        }

        if dashboard.next_queue_task_count > policy.maximum_next_queue_tasks {
            watch_reasons.push(format!(
                "evolution_admission_handoff_next_queue_tasks={}>{}",
                dashboard.next_queue_task_count, policy.maximum_next_queue_tasks
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (EvolutionAdmissionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (EvolutionAdmissionHealthStatus::Watch, watch_reasons)
        } else {
            (EvolutionAdmissionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == EvolutionAdmissionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != EvolutionAdmissionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == EvolutionAdmissionHealthStatus::Repair
    }
}

impl EvolutionAdmissionHandoffSummaryHistoryRecord {
    pub fn records(&self) -> usize {
        self.history.len()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.health.requires_repair_first()
    }
}

impl EvolutionAdmissionHandoffSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: EvolutionAdmissionHandoffSummaryHistory,
        summary: EvolutionAdmissionHandoffSummary,
        policy: EvolutionAdmissionHandoffHealthPolicy,
    ) -> EvolutionAdmissionHandoffSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = evolution_admission_handoff_history_record_telemetry(&dashboard, &health);

        EvolutionAdmissionHandoffSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_handoff_with_health(
        &self,
        history: EvolutionAdmissionHandoffSummaryHistory,
        handoff: &EvolutionAdmissionHandoff,
        policy: EvolutionAdmissionHandoffHealthPolicy,
    ) -> EvolutionAdmissionHandoffSummaryHistoryRecord {
        self.record_summary_with_health(history, handoff.summary(), policy)
    }
}

impl EvolutionAdmissionHandoffTrendGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.effective_admitted && !self.requires_repair_first
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.next_queue.clone()
    }
}

impl EvolutionAdmissionHandoffTrendGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        handoff: &EvolutionAdmissionHandoff,
        history_record: &EvolutionAdmissionHandoffSummaryHistoryRecord,
    ) -> EvolutionAdmissionHandoffTrendGateDecision {
        let requested_admitted = handoff.is_admitted();
        let handoff_health = history_record.health.clone();
        let requires_repair_first =
            handoff.requires_repair_first || handoff_health.requires_repair_first();
        let mut blocked_reasons = handoff.blocked_reasons.clone();
        extend_ordered_unique(
            &mut blocked_reasons,
            handoff_health
                .reasons
                .iter()
                .map(|reason| format!("handoff_history:{reason}"))
                .collect(),
        );
        let repair_tasks =
            evolution_admission_handoff_trend_gate_repair_tasks(&handoff_health, &blocked_reasons);
        let mut next_queue = handoff.next_queue.clone();
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let effective_admitted = requested_admitted && handoff_health.allows_service_advance();
        let stable_trend = handoff_health.is_stable();
        let can_promote_ready_proposals =
            effective_admitted && stable_trend && handoff.can_promote_ready_proposals;
        let can_promote_evolution_signals =
            effective_admitted && stable_trend && handoff.can_promote_evolution_signals;
        let can_reinforce_process =
            effective_admitted && stable_trend && handoff.can_reinforce_process;
        let can_promote_adaptive_state =
            effective_admitted && stable_trend && handoff.can_promote_adaptive_state;
        let telemetry = evolution_admission_handoff_trend_gate_telemetry(
            requested_admitted,
            effective_admitted,
            handoff_health.status,
            requires_repair_first,
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            repair_tasks.len(),
            next_queue.len(),
            blocked_reasons.len(),
            history_record.records(),
        );

        EvolutionAdmissionHandoffTrendGateDecision {
            requested_admitted,
            effective_admitted,
            handoff_health,
            requires_repair_first,
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            repair_tasks,
            next_queue,
            blocked_reasons,
            telemetry,
        }
    }
}

impl EvolutionAdmissionHandoffTrendMonitorRecord {
    pub fn records(&self) -> usize {
        self.history_record.records()
    }

    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.history_record.allows_service_advance() && self.gate_decision.is_admitted()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue()
    }

    pub fn continuation(
        &self,
        handoff_policy: EvolutionAdmissionHandoffHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuation {
        EvolutionAdmissionHandoffTrendContinuation::from_monitor_record(self, handoff_policy)
    }
}

impl EvolutionAdmissionHandoffTrendMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn monitor(
        &self,
        history: EvolutionAdmissionHandoffSummaryHistory,
        handoff: &EvolutionAdmissionHandoff,
        policy: EvolutionAdmissionHandoffHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendMonitorRecord {
        let history_record = EvolutionAdmissionHandoffSummaryHistoryRecorder::new()
            .record_handoff_with_health(history, handoff, policy);
        let gate_decision =
            EvolutionAdmissionHandoffTrendGate::new().gate(handoff, &history_record);
        let telemetry = evolution_admission_handoff_trend_monitor_telemetry(
            history_record.health.status,
            gate_decision.is_admitted(),
            gate_decision.requires_repair_first,
            gate_decision.repair_tasks.len(),
            gate_decision.next_queue.len(),
            history_record.records(),
        );

        EvolutionAdmissionHandoffTrendMonitorRecord {
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

impl EvolutionAdmissionHandoffTrendContinuation {
    pub fn from_monitor_record(
        record: &EvolutionAdmissionHandoffTrendMonitorRecord,
        handoff_policy: EvolutionAdmissionHandoffHealthPolicy,
    ) -> Self {
        let effective_admitted = record.is_admitted();
        let can_promote_ready_proposals =
            effective_admitted && record.gate_decision.can_promote_ready_proposals;
        let can_promote_evolution_signals =
            effective_admitted && record.gate_decision.can_promote_evolution_signals;
        let can_reinforce_process =
            effective_admitted && record.gate_decision.can_reinforce_process;
        let can_promote_adaptive_state =
            effective_admitted && record.gate_decision.can_promote_adaptive_state;
        let requires_repair_first = record.requires_repair_first();
        let next_queue = record.next_queue();
        let handoff_history = record.history_record.history.clone();
        let handoff_health_status = record.history_record.health.status;
        let telemetry = evolution_admission_handoff_trend_continuation_telemetry(
            handoff_health_status,
            effective_admitted,
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            requires_repair_first,
            next_queue.len(),
            handoff_history.len(),
        );

        Self {
            next_queue,
            handoff_history,
            handoff_policy,
            handoff_health_status,
            effective_admitted,
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            requires_repair_first,
            telemetry,
        }
    }

    pub fn is_admitted(&self) -> bool {
        self.effective_admitted && !self.requires_repair_first
    }

    pub fn summary(&self) -> EvolutionAdmissionHandoffTrendContinuationSummary {
        EvolutionAdmissionHandoffTrendContinuationSummary::from_continuation(self)
    }
}

impl EvolutionAdmissionHandoffTrendContinuationSummary {
    pub fn from_continuation(continuation: &EvolutionAdmissionHandoffTrendContinuation) -> Self {
        let next_queue_task_ids = continuation.next_queue.task_ids();
        let telemetry = evolution_admission_handoff_trend_continuation_summary_telemetry(
            continuation.handoff_health_status,
            continuation.effective_admitted,
            continuation.can_promote_ready_proposals,
            continuation.can_promote_evolution_signals,
            continuation.can_reinforce_process,
            continuation.can_promote_adaptive_state,
            continuation.requires_repair_first,
            next_queue_task_ids.len(),
            continuation.handoff_history.len(),
        );

        Self {
            handoff_health_status: continuation.handoff_health_status,
            effective_admitted: continuation.effective_admitted,
            can_promote_ready_proposals: continuation.can_promote_ready_proposals,
            can_promote_evolution_signals: continuation.can_promote_evolution_signals,
            can_reinforce_process: continuation.can_reinforce_process,
            can_promote_adaptive_state: continuation.can_promote_adaptive_state,
            requires_repair_first: continuation.requires_repair_first,
            next_queue_tasks: next_queue_task_ids.len(),
            handoff_history_records: continuation.handoff_history.len(),
            next_queue_task_ids,
            telemetry,
        }
    }
}

impl EvolutionAdmissionHandoffTrendContinuationSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<EvolutionAdmissionHandoffTrendContinuationSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: EvolutionAdmissionHandoffTrendContinuationSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&EvolutionAdmissionHandoffTrendContinuationSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[EvolutionAdmissionHandoffTrendContinuationSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> EvolutionAdmissionHandoffTrendContinuationDashboard {
        EvolutionAdmissionHandoffTrendContinuationDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: EvolutionAdmissionHandoffTrendContinuationHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuationHealth {
        self.dashboard().health(policy)
    }
}

impl EvolutionAdmissionHandoffTrendContinuationDashboard {
    pub fn from_summaries(summaries: &[EvolutionAdmissionHandoffTrendContinuationSummary]) -> Self {
        let total_records = summaries.len();
        let effective_admitted_records = summaries
            .iter()
            .filter(|summary| summary.effective_admitted)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let ready_promotion_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_ready_proposals)
            .count();
        let signal_promotion_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_evolution_signals)
            .count();
        let reinforcement_records = summaries
            .iter()
            .filter(|summary| summary.can_reinforce_process)
            .count();
        let adaptive_state_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let next_queue_task_count = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let handoff_history_record_count = summaries
            .iter()
            .map(|summary| summary.handoff_history_records)
            .sum::<usize>();
        let handoff_repair_records = summaries
            .iter()
            .filter(|summary| {
                summary.handoff_health_status == EvolutionAdmissionHealthStatus::Repair
            })
            .count();
        let effective_admitted_rate = rate(effective_admitted_records, total_records);
        let adaptive_state_rate = rate(adaptive_state_records, total_records);
        let telemetry = evolution_admission_handoff_trend_continuation_dashboard_telemetry(
            total_records,
            effective_admitted_records,
            repair_first_records,
            ready_promotion_records,
            signal_promotion_records,
            reinforcement_records,
            adaptive_state_records,
            next_queue_task_count,
            handoff_history_record_count,
            handoff_repair_records,
            effective_admitted_rate,
            adaptive_state_rate,
        );

        Self {
            total_records,
            effective_admitted_records,
            repair_first_records,
            ready_promotion_records,
            signal_promotion_records,
            reinforcement_records,
            adaptive_state_records,
            next_queue_task_count,
            handoff_history_record_count,
            handoff_repair_records,
            effective_admitted_rate,
            adaptive_state_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: EvolutionAdmissionHandoffTrendContinuationHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuationHealth {
        EvolutionAdmissionHandoffTrendContinuationHealth::from_dashboard(self.clone(), policy)
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHealth {
    pub fn from_dashboard(
        dashboard: EvolutionAdmissionHandoffTrendContinuationDashboard,
        policy: EvolutionAdmissionHandoffTrendContinuationHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons
                .push("evolution_admission_handoff_trend_continuation_history_empty".to_owned());
        } else if dashboard.effective_admitted_rate < policy.minimum_effective_admitted_rate {
            watch_reasons.push(format!(
                "evolution_admission_handoff_trend_continuation_effective_admitted_rate={:.3}<{}",
                dashboard.effective_admitted_rate, policy.minimum_effective_admitted_rate
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "evolution_admission_handoff_trend_continuation_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.handoff_repair_records > policy.maximum_handoff_repair_records {
            repair_reasons.push(format!(
                "evolution_admission_handoff_trend_continuation_handoff_repair_records={}>{}",
                dashboard.handoff_repair_records, policy.maximum_handoff_repair_records
            ));
        }

        if dashboard.next_queue_task_count > policy.maximum_next_queue_tasks {
            watch_reasons.push(format!(
                "evolution_admission_handoff_trend_continuation_next_queue_tasks={}>{}",
                dashboard.next_queue_task_count, policy.maximum_next_queue_tasks
            ));
        }

        if dashboard.handoff_history_record_count > policy.maximum_handoff_history_records {
            watch_reasons.push(format!(
                "evolution_admission_handoff_trend_continuation_handoff_history_records={}>{}",
                dashboard.handoff_history_record_count, policy.maximum_handoff_history_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (EvolutionAdmissionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (EvolutionAdmissionHealthStatus::Watch, watch_reasons)
        } else {
            (EvolutionAdmissionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == EvolutionAdmissionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != EvolutionAdmissionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == EvolutionAdmissionHealthStatus::Repair
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHistoryRecord {
    pub fn records(&self) -> usize {
        self.history.len()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.health.requires_repair_first()
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHistoryGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.effective_admitted && !self.requires_repair_first
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.next_queue.clone()
    }

    pub fn summary(&self) -> EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary {
        EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary::from_decision(self)
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary {
    pub fn from_decision(
        decision: &EvolutionAdmissionHandoffTrendContinuationHistoryGateDecision,
    ) -> Self {
        let repair_task_ids = decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = decision.next_queue.task_ids();
        let telemetry =
            evolution_admission_handoff_trend_continuation_history_gate_summary_telemetry(
                decision.continuation_health.status,
                decision.effective_admitted,
                decision.can_promote_ready_proposals,
                decision.can_promote_evolution_signals,
                decision.can_reinforce_process,
                decision.can_promote_adaptive_state,
                decision.requires_repair_first,
                repair_task_ids.len(),
                next_queue_task_ids.len(),
                decision.blocked_reasons.len(),
                decision.continuation_summary.handoff_history_records,
            );

        Self {
            continuation_health_status: decision.continuation_health.status,
            effective_admitted: decision.effective_admitted,
            can_promote_ready_proposals: decision.can_promote_ready_proposals,
            can_promote_evolution_signals: decision.can_promote_evolution_signals,
            can_reinforce_process: decision.can_reinforce_process,
            can_promote_adaptive_state: decision.can_promote_adaptive_state,
            requires_repair_first: decision.requires_repair_first,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: decision.blocked_reasons.len(),
            records: decision.continuation_summary.handoff_history_records,
            repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> EvolutionAdmissionHandoffTrendContinuationHistoryGateDashboard {
        EvolutionAdmissionHandoffTrendContinuationHistoryGateDashboard::from_summaries(
            &self.summaries,
        )
    }

    pub fn health(
        &self,
        policy: EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuationHistoryGateHealth {
        self.dashboard().health(policy)
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHistoryGateDashboard {
    pub fn from_summaries(
        summaries: &[EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary],
    ) -> Self {
        let total_records = summaries.len();
        let effective_admitted_records = summaries
            .iter()
            .filter(|summary| summary.effective_admitted)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let ready_promotion_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_ready_proposals)
            .count();
        let signal_promotion_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_evolution_signals)
            .count();
        let reinforcement_records = summaries
            .iter()
            .filter(|summary| summary.can_reinforce_process)
            .count();
        let adaptive_state_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let next_queue_task_count = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reason_count = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let continuation_repair_records = summaries
            .iter()
            .filter(|summary| {
                summary.continuation_health_status == EvolutionAdmissionHealthStatus::Repair
            })
            .count();
        let effective_admitted_rate = rate(effective_admitted_records, total_records);
        let adaptive_state_rate = rate(adaptive_state_records, total_records);
        let telemetry =
            evolution_admission_handoff_trend_continuation_history_gate_dashboard_telemetry(
                total_records,
                effective_admitted_records,
                repair_first_records,
                ready_promotion_records,
                signal_promotion_records,
                reinforcement_records,
                adaptive_state_records,
                repair_task_count,
                next_queue_task_count,
                blocked_reason_count,
                continuation_repair_records,
                effective_admitted_rate,
                adaptive_state_rate,
            );

        Self {
            total_records,
            effective_admitted_records,
            repair_first_records,
            ready_promotion_records,
            signal_promotion_records,
            reinforcement_records,
            adaptive_state_records,
            repair_task_count,
            next_queue_task_count,
            blocked_reason_count,
            continuation_repair_records,
            effective_admitted_rate,
            adaptive_state_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuationHistoryGateHealth {
        EvolutionAdmissionHandoffTrendContinuationHistoryGateHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHistoryGateHealth {
    pub fn from_dashboard(
        dashboard: EvolutionAdmissionHandoffTrendContinuationHistoryGateDashboard,
        policy: EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "evolution_admission_handoff_trend_continuation_history_gate_history_empty"
                    .to_owned(),
            );
        } else if dashboard.effective_admitted_rate < policy.minimum_effective_admitted_rate {
            watch_reasons.push(format!(
                "evolution_admission_handoff_trend_continuation_history_gate_effective_admitted_rate={:.3}<{}",
                dashboard.effective_admitted_rate, policy.minimum_effective_admitted_rate
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "evolution_admission_handoff_trend_continuation_history_gate_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "evolution_admission_handoff_trend_continuation_history_gate_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reason_count > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "evolution_admission_handoff_trend_continuation_history_gate_blocked_reasons={}>{}",
                dashboard.blocked_reason_count, policy.maximum_blocked_reasons
            ));
        }

        if dashboard.continuation_repair_records > policy.maximum_continuation_repair_records {
            repair_reasons.push(format!(
                "evolution_admission_handoff_trend_continuation_history_gate_continuation_repair_records={}>{}",
                dashboard.continuation_repair_records,
                policy.maximum_continuation_repair_records
            ));
        }

        if dashboard.next_queue_task_count > policy.maximum_next_queue_tasks {
            watch_reasons.push(format!(
                "evolution_admission_handoff_trend_continuation_history_gate_next_queue_tasks={}>{}",
                dashboard.next_queue_task_count, policy.maximum_next_queue_tasks
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (EvolutionAdmissionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (EvolutionAdmissionHealthStatus::Watch, watch_reasons)
        } else {
            (EvolutionAdmissionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == EvolutionAdmissionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != EvolutionAdmissionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == EvolutionAdmissionHealthStatus::Repair
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord {
    pub fn records(&self) -> usize {
        self.history.len()
    }

    pub fn is_effectively_admitted(&self) -> bool {
        self.appended_summary.effective_admitted && self.health.allows_service_advance()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.health.requires_repair_first()
    }

    pub fn can_promote_ready_proposals(&self) -> bool {
        self.health.is_stable()
            && self.is_effectively_admitted()
            && self.appended_summary.can_promote_ready_proposals
    }

    pub fn can_promote_evolution_signals(&self) -> bool {
        self.health.is_stable()
            && self.is_effectively_admitted()
            && self.appended_summary.can_promote_evolution_signals
    }

    pub fn can_reinforce_process(&self) -> bool {
        self.health.is_stable()
            && self.is_effectively_admitted()
            && self.appended_summary.can_reinforce_process
    }

    pub fn can_promote_adaptive_state(&self) -> bool {
        self.health.is_stable()
            && self.is_effectively_admitted()
            && self.appended_summary.can_promote_adaptive_state
    }

    pub fn repair_task_ids(&self) -> &[String] {
        &self.appended_summary.repair_task_ids
    }

    pub fn next_queue_task_ids(&self) -> &[String] {
        &self.appended_summary.next_queue_task_ids
    }

    pub fn blocked_reason_count(&self) -> usize {
        self.appended_summary.blocked_reasons
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory,
        summary: EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary,
        policy: EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            evolution_admission_handoff_trend_continuation_history_gate_history_record_telemetry(
                &dashboard, &health,
            );

        EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_gate_with_health(
        &self,
        history: EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory,
        gate_record: &EvolutionAdmissionHandoffTrendContinuationHistoryGateRecord,
        policy: EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord {
        self.record_summary_with_health(history, gate_record.summary(), policy)
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHistoryGateRecord {
    pub fn records(&self) -> usize {
        self.health_record.records()
    }

    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health_record.allows_service_advance() && self.gate_decision.is_admitted()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue()
    }

    pub fn summary(&self) -> EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary {
        self.gate_decision.summary()
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: EvolutionAdmissionHandoffTrendContinuationSummaryHistory,
        summary: EvolutionAdmissionHandoffTrendContinuationSummary,
        policy: EvolutionAdmissionHandoffTrendContinuationHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuationHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = evolution_admission_handoff_trend_continuation_history_record_telemetry(
            &dashboard, &health,
        );

        EvolutionAdmissionHandoffTrendContinuationHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_continuation_with_health(
        &self,
        history: EvolutionAdmissionHandoffTrendContinuationSummaryHistory,
        continuation: &EvolutionAdmissionHandoffTrendContinuation,
        policy: EvolutionAdmissionHandoffTrendContinuationHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuationHistoryRecord {
        self.record_summary_with_health(history, continuation.summary(), policy)
    }

    pub fn record_continuation_with_health_gate(
        &self,
        history: EvolutionAdmissionHandoffTrendContinuationSummaryHistory,
        continuation: &EvolutionAdmissionHandoffTrendContinuation,
        policy: EvolutionAdmissionHandoffTrendContinuationHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuationHistoryGateRecord {
        let health_record = self.record_continuation_with_health(history, continuation, policy);
        let gate_decision = EvolutionAdmissionHandoffTrendContinuationHistoryGate::new()
            .gate(continuation, &health_record);
        let telemetry =
            evolution_admission_handoff_trend_continuation_history_gate_record_telemetry(
                &health_record,
                &gate_decision,
            );

        EvolutionAdmissionHandoffTrendContinuationHistoryGateRecord {
            health_record,
            gate_decision,
            telemetry,
        }
    }
}

impl EvolutionAdmissionHandoffTrendContinuationHistoryGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        continuation: &EvolutionAdmissionHandoffTrendContinuation,
        history_record: &EvolutionAdmissionHandoffTrendContinuationHistoryRecord,
    ) -> EvolutionAdmissionHandoffTrendContinuationHistoryGateDecision {
        let continuation_summary = continuation.summary();
        let continuation_health = history_record.health.clone();
        let requires_repair_first =
            continuation.requires_repair_first || continuation_health.requires_repair_first();
        let mut blocked_reasons = Vec::new();
        extend_ordered_unique(
            &mut blocked_reasons,
            continuation_health
                .reasons
                .iter()
                .map(|reason| format!("continuation_history:{reason}"))
                .collect(),
        );
        let repair_tasks = evolution_admission_handoff_trend_continuation_history_gate_repair_tasks(
            &continuation_health,
            &blocked_reasons,
        );
        let mut next_queue = continuation.next_queue.clone();
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let effective_admitted =
            continuation.is_admitted() && continuation_health.allows_service_advance();
        let stable_trend = continuation_health.is_stable();
        let can_promote_ready_proposals =
            effective_admitted && stable_trend && continuation.can_promote_ready_proposals;
        let can_promote_evolution_signals =
            effective_admitted && stable_trend && continuation.can_promote_evolution_signals;
        let can_reinforce_process =
            effective_admitted && stable_trend && continuation.can_reinforce_process;
        let can_promote_adaptive_state =
            effective_admitted && stable_trend && continuation.can_promote_adaptive_state;
        let telemetry = evolution_admission_handoff_trend_continuation_history_gate_telemetry(
            continuation_health.status,
            effective_admitted,
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            blocked_reasons.len(),
            history_record.records(),
        );

        EvolutionAdmissionHandoffTrendContinuationHistoryGateDecision {
            continuation_summary,
            continuation_health,
            effective_admitted,
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks,
            next_queue,
            blocked_reasons,
            telemetry,
        }
    }
}

impl EvolutionAdmissionHandoffTrendContinuationPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        record: &EvolutionAdmissionHandoffTrendMonitorRecord,
        handoff_policy: EvolutionAdmissionHandoffHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuation {
        record.continuation(handoff_policy)
    }
}

#[derive(Debug, Clone, Default)]
pub struct EvolutionAdmissionGate;

impl EvolutionAdmissionGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        toolsmith_record: ToolsmithPlanHistoryGateRecord,
        reward_record: ProcessRewardReportHistoryGateRecord,
    ) -> EvolutionAdmissionRecord {
        let mut blocked_reasons = Vec::new();
        extend_ordered_unique(
            &mut blocked_reasons,
            toolsmith_record
                .gate_decision
                .reasons
                .iter()
                .map(|reason| format!("toolsmith:{reason}"))
                .collect::<Vec<_>>(),
        );
        extend_ordered_unique(
            &mut blocked_reasons,
            reward_record
                .gate_decision
                .reasons
                .iter()
                .map(|reason| format!("process_reward:{reason}"))
                .collect::<Vec<_>>(),
        );

        let requires_repair_first =
            toolsmith_record.requires_repair_first() || reward_record.requires_repair_first();
        let mut repair_tasks = toolsmith_record.gate_decision.repair_tasks.clone();
        for task in &reward_record.gate_decision.repair_tasks {
            repair_tasks.push(task.clone());
        }

        let reward_reinforced = reward_record.gate_decision.report_summary.action
            == RewardAction::Reinforce
            && reward_record.gate_decision.can_promote_evolution_signals;
        let can_promote_ready_proposals = toolsmith_record.can_promote_ready_proposals()
            && reward_reinforced
            && reward_record.allows_service_advance()
            && !requires_repair_first;
        let can_promote_evolution_signals = reward_record.can_promote_evolution_signals()
            && toolsmith_record.allows_service_advance()
            && !requires_repair_first;
        let can_reinforce_process = can_promote_evolution_signals
            && reward_record.gate_decision.can_reinforce_process
            && toolsmith_record.allows_service_advance();
        let can_promote_adaptive_state = can_promote_ready_proposals
            && can_promote_evolution_signals
            && can_reinforce_process
            && toolsmith_record.gate_decision.toolsmith_health.is_stable()
            && reward_record.gate_decision.reward_health.is_stable();
        let telemetry = evolution_admission_decision_telemetry(
            toolsmith_record.gate_decision.toolsmith_health.status,
            reward_record.gate_decision.reward_health.status,
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks.len(),
            blocked_reasons.len(),
        );

        let decision = EvolutionAdmissionDecision {
            toolsmith_health_status: toolsmith_record.gate_decision.toolsmith_health.status,
            reward_health_status: reward_record.gate_decision.reward_health.status,
            can_promote_ready_proposals,
            can_promote_evolution_signals,
            can_reinforce_process,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks,
            blocked_reasons,
            telemetry,
        };
        let telemetry =
            evolution_admission_record_telemetry(&toolsmith_record, &reward_record, &decision);

        EvolutionAdmissionRecord {
            toolsmith_record,
            reward_record,
            decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessRewardPolicy {
    pub reinforce_at: f32,
    pub penalize_at: f32,
}

impl Default for ProcessRewardPolicy {
    fn default() -> Self {
        Self {
            reinforce_at: 0.72,
            penalize_at: 0.42,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClosedLoopRewarder {
    policy: ProcessRewardPolicy,
}

impl ClosedLoopRewarder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(policy: ProcessRewardPolicy) -> Self {
        Self { policy }
    }

    pub fn score(&self, input: ProcessRewardInput) -> ProcessRewardReport {
        let quality = input.quality.clamp(0.0, 1.0);
        let components = ProcessRewardComponents {
            coordination: coordination_score(&input.run_report.conflicts),
            reflection: reflection_score(input.reflection_complete, quality),
            validation: validation_score(
                input.validation_passed,
                input.runtime_response_ok,
                input.execution_failures,
            ),
            toolsmith: toolsmith_score(&input.toolsmith_plan, input.tool_build_report.as_ref()),
            recursion: recursion_score(input.recursive_chunks, input.recursive_waves),
            admission: admission_score(&input.run_report, quality),
        };
        let total = weighted_total(components);
        let action = self.action_for_input(total, &input);
        let notes = reward_notes(&input, components, total, action);
        let evolution_signals = evolution_signals(&input, components, total, action);

        ProcessRewardReport {
            total,
            components,
            action,
            notes,
            evolution_signals,
        }
    }

    fn action_for_total(&self, total: f32) -> RewardAction {
        if total >= self.policy.reinforce_at {
            RewardAction::Reinforce
        } else if total <= self.policy.penalize_at {
            RewardAction::Penalize
        } else {
            RewardAction::Hold
        }
    }

    fn action_for_input(&self, total: f32, input: &ProcessRewardInput) -> RewardAction {
        let action = self.action_for_total(total);
        if action == RewardAction::Reinforce && !reinforcement_admission_clear(input) {
            RewardAction::Hold
        } else {
            action
        }
    }
}

fn coordination_score(conflicts: &ConflictReport) -> f32 {
    if conflicts.has_unresolved_conflicts() {
        return (0.28 - conflicts.unresolved_count() as f32 * 0.06).clamp(0.0, 1.0);
    }
    0.86
}

fn reflection_score(reflection_complete: bool, quality: f32) -> f32 {
    if reflection_complete {
        (0.48 + quality * 0.46).clamp(0.0, 1.0)
    } else {
        (0.18 + quality * 0.20).clamp(0.0, 1.0)
    }
}

fn validation_score(
    validation_passed: bool,
    runtime_response_ok: bool,
    execution_failures: usize,
) -> f32 {
    if execution_failures > 0 {
        return 0.18;
    }
    match (validation_passed, runtime_response_ok) {
        (true, true) => 0.88,
        (true, false) => 0.44,
        (false, true) => 0.36,
        (false, false) => 0.18,
    }
}

fn toolsmith_score(plan: &ToolsmithPlan, build_report: Option<&ToolBuildReport>) -> f32 {
    if build_report.is_some_and(ToolBuildReport::requires_repair_first) {
        return 0.18;
    }
    if !plan.passed_rust_gate() {
        return 0.22;
    }
    if plan.ready_count() > 0 {
        return (0.72 + plan.ready_count() as f32 * 0.04).clamp(0.0, 0.92);
    }
    if plan.held_count() > 0 {
        return 0.54;
    }
    0.64
}

fn recursion_score(chunks: usize, waves: usize) -> f32 {
    if chunks <= 1 {
        return 0.78;
    }
    let chunk_pressure = (chunks.saturating_sub(1) as f32 / 128.0).min(0.18);
    let wave_pressure = (waves.saturating_sub(1) as f32 / 32.0).min(0.20);
    (0.82 - chunk_pressure - wave_pressure).clamp(0.0, 1.0)
}

fn admission_score(report: &AgentRunReport, quality: f32) -> f32 {
    if report.budget_audit.has_overspends() {
        return 0.20;
    }
    if report.conflicts.has_unresolved_conflicts() {
        return 0.22;
    }
    let blocked = report
        .side_effects
        .iter()
        .filter(|gate| !gate.allowed)
        .count();
    let memory_allowed = report
        .side_effects
        .iter()
        .any(|gate| gate.kind == SideEffectKind::MemoryNote && gate.allowed);

    if quality < 0.55 && memory_allowed {
        return 0.28;
    }
    (0.78 - blocked as f32 * 0.10).clamp(0.0, 1.0)
}

fn weighted_total(components: ProcessRewardComponents) -> f32 {
    (components.coordination * 0.18
        + components.reflection * 0.22
        + components.validation * 0.24
        + components.toolsmith * 0.12
        + components.recursion * 0.10
        + components.admission * 0.14)
        .clamp(0.0, 1.0)
}

fn reward_notes(
    input: &ProcessRewardInput,
    components: ProcessRewardComponents,
    total: f32,
    action: RewardAction,
) -> Vec<String> {
    let mut notes = Vec::new();
    if input.run_report.conflicts.has_unresolved_conflicts() {
        notes.push(format!(
            "coordination:unresolved_conflicts={}",
            input.run_report.conflicts.unresolved_count()
        ));
    }
    if !input.reflection_complete {
        notes.push("reflection:incomplete".to_owned());
    }
    if !input.validation_passed {
        notes.push("validation:failed_or_missing".to_owned());
    }
    if !input.runtime_response_ok {
        notes.push("runtime:response_missing_or_invalid".to_owned());
    }
    if input.execution_failures > 0 {
        notes.push(format!("execution:failures={}", input.execution_failures));
    }
    if input.recursive_chunks > 1 {
        notes.push(format!(
            "recursive:chunks={}:waves={}",
            input.recursive_chunks, input.recursive_waves
        ));
    }
    if components.admission <= 0.35 || blocked_side_effect_count(&input.run_report) > 0 {
        notes.push("admission:side_effects_blocked_or_low_quality".to_owned());
    }
    if input.run_report.budget_audit.has_overspends() {
        notes.push(format!(
            "budget:overspends={}",
            input.run_report.budget_audit.overspend_count()
        ));
    }
    if let Some(report) = input.tool_build_report.as_ref() {
        if report.requires_repair_first() {
            notes.push(format!(
                "tool_build:repair_first missing={} unexpected={} duplicate={} held={} rejected={}",
                report.missing_request_ids.len(),
                report.unexpected_receipt_ids.len(),
                report.duplicate_receipt_ids.len(),
                report.held,
                report.rejected
            ));
        }
    }
    notes.extend(input.toolsmith_plan.reward_notes());
    notes.push(format!("total:{total:.3}:{}", action.as_str()));
    notes
}

fn reinforcement_admission_clear(input: &ProcessRewardInput) -> bool {
    input.reflection_complete
        && input.validation_passed
        && input.runtime_response_ok
        && input.execution_failures == 0
        && input.run_report.conflicts.unresolved_count() == 0
        && !input.run_report.budget_audit.has_overspends()
        && blocked_side_effect_count(&input.run_report) == 0
        && !input
            .tool_build_report
            .as_ref()
            .is_some_and(ToolBuildReport::requires_repair_first)
}

fn blocked_side_effect_count(report: &AgentRunReport) -> usize {
    report
        .side_effects
        .iter()
        .filter(|gate| !gate.allowed)
        .count()
}

fn evolution_signals(
    input: &ProcessRewardInput,
    components: ProcessRewardComponents,
    total: f32,
    action: RewardAction,
) -> Vec<EvolutionSignal> {
    let mut signals = Vec::new();
    match action {
        RewardAction::Reinforce => {
            signals.push(EvolutionSignal::new(
                "agent_coordination",
                "promote_closed_loop_pattern",
                "high reward with validation, reflection, and side-effect gates satisfied",
                total,
            ));
            if input.toolsmith_plan.ready_count() > 0 && components.toolsmith >= 0.72 {
                signals.push(EvolutionSignal::new(
                    "toolsmith_routing",
                    "reuse_ready_rust_tool_proposals",
                    "tool proposals passed the Rust-only gate",
                    components.toolsmith,
                ));
            }
        }
        RewardAction::Hold => {
            signals.push(EvolutionSignal::new(
                "agent_coordination",
                "hold_for_more_evidence",
                "reward is inconclusive for promotion or penalty",
                total,
            ));
        }
        RewardAction::Penalize => {
            signals.push(EvolutionSignal::new(
                "agent_coordination",
                "penalize_or_repair_loop",
                "closed-loop gates found unresolved risk",
                1.0 - total,
            ));
        }
    }
    signals
}

#[allow(clippy::too_many_arguments)]
fn toolsmith_plan_summary_telemetry(
    proposals: usize,
    ready: usize,
    held: usize,
    rejected: usize,
    rejected_requests: usize,
    non_rust_proposals: usize,
    rust_only: bool,
    rust_gate_passed: bool,
) -> Vec<String> {
    vec![
        "agent_toolsmith_plan_summary=true".to_owned(),
        format!("agent_toolsmith_plan_summary_proposals={proposals}"),
        format!("agent_toolsmith_plan_summary_ready={ready}"),
        format!("agent_toolsmith_plan_summary_held={held}"),
        format!("agent_toolsmith_plan_summary_rejected={rejected}"),
        format!("agent_toolsmith_plan_summary_rejected_requests={rejected_requests}"),
        format!("agent_toolsmith_plan_summary_non_rust_proposals={non_rust_proposals}"),
        format!("agent_toolsmith_plan_summary_rust_only={rust_only}"),
        format!("agent_toolsmith_plan_summary_rust_gate_passed={rust_gate_passed}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn toolsmith_plan_dashboard_telemetry(
    total_records: usize,
    proposals: usize,
    ready: usize,
    held: usize,
    rejected: usize,
    rejected_requests: usize,
    non_rust_proposals: usize,
    rust_gate_failed_records: usize,
    empty_records: usize,
    ready_rate: f32,
    rust_gate_pass_rate: f32,
) -> Vec<String> {
    vec![
        "agent_toolsmith_plan_dashboard=true".to_owned(),
        format!("agent_toolsmith_plan_dashboard_records={total_records}"),
        format!("agent_toolsmith_plan_dashboard_proposals={proposals}"),
        format!("agent_toolsmith_plan_dashboard_ready={ready}"),
        format!("agent_toolsmith_plan_dashboard_held={held}"),
        format!("agent_toolsmith_plan_dashboard_rejected={rejected}"),
        format!("agent_toolsmith_plan_dashboard_rejected_requests={rejected_requests}"),
        format!("agent_toolsmith_plan_dashboard_non_rust_proposals={non_rust_proposals}"),
        format!(
            "agent_toolsmith_plan_dashboard_rust_gate_failed_records={rust_gate_failed_records}"
        ),
        format!("agent_toolsmith_plan_dashboard_empty_records={empty_records}"),
        format!("agent_toolsmith_plan_dashboard_ready_rate={ready_rate:.3}"),
        format!("agent_toolsmith_plan_dashboard_rust_gate_pass_rate={rust_gate_pass_rate:.3}"),
    ]
}

fn toolsmith_plan_history_record_telemetry(
    dashboard: &ToolsmithPlanDashboard,
    health: &ToolsmithPlanHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_toolsmith_plan_history_record=true".to_owned(),
        format!(
            "agent_toolsmith_plan_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_toolsmith_plan_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_toolsmith_plan_history_record_ready_rate={:.3}",
            dashboard.ready_rate
        ),
        format!(
            "agent_toolsmith_plan_history_record_rust_gate_pass_rate={:.3}",
            dashboard.rust_gate_pass_rate
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_toolsmith_plan_history_record_reason={reason}")),
    );
    telemetry
}

fn toolsmith_plan_gate_reasons(
    plan: &ToolsmithPlan,
    summary: &ToolsmithPlanSummary,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !summary.rust_only {
        reasons.push("toolsmith_plan_rust_only=false".to_owned());
    }
    if summary.rejected_requests > 0 {
        reasons.push(format!(
            "toolsmith_plan_rejected_requests={}",
            summary.rejected_requests
        ));
    }
    if summary.non_rust_proposals > 0 {
        reasons.push(format!(
            "toolsmith_plan_non_rust_proposals={}",
            summary.non_rust_proposals
        ));
    }
    if !summary.rust_gate_passed {
        reasons.push("toolsmith_plan_rust_gate_failed".to_owned());
    }
    reasons.extend(plan.proposals.iter().filter_map(|proposal| {
        toolsmith_blueprint_movement_blocker(proposal)
            .map(|reason| format!("toolsmith_blueprint_movement:{}:{reason}", proposal.id))
    }));
    reasons.extend(
        plan.proposals
            .iter()
            .filter(|proposal| proposal.status == ToolBuildStatus::Rejected)
            .map(|proposal| format!("toolsmith_plan_rejected_proposal={}", proposal.id)),
    );
    reasons
}

fn toolsmith_blueprint_movement_blocker(proposal: &ToolProposal) -> Option<&'static str> {
    let target_scope = toolsmith_blueprint_target_scope(proposal);
    let source_scope = proposal.source_scope.as_deref().unwrap_or(&target_scope);
    let moved = source_scope != target_scope;
    let Some(review) = &proposal.movement_review else {
        return moved.then_some("review_missing");
    };

    if !review.is_preview_only() {
        return Some("write_violation");
    }
    if review.source_proposal_id != proposal.id
        || review.source_digest != proposal.blueprint_digest()
        || review.source_scope != source_scope
        || review.target_scope != target_scope
    {
        return Some("evidence_stale");
    }
    if review
        .forbidden_scope_tags
        .iter()
        .any(|tag| tag == "*" || tag == &target_scope)
    {
        return Some("forbidden_target_scope");
    }
    if review.collision_risk {
        return Some("neighbor_collision_risk");
    }

    match review.decision {
        ToolsmithBlueprintMovementDecision::AllowPreviewMove => {
            if moved
                && !review
                    .allowed_scope_tags
                    .iter()
                    .any(|tag| tag == &target_scope)
            {
                Some("target_scope_not_allowed")
            } else {
                None
            }
        }
        ToolsmithBlueprintMovementDecision::HoldForScopeReview => Some("hold_for_scope_review"),
        ToolsmithBlueprintMovementDecision::QuarantineBlueprint => Some("quarantine_requested"),
        ToolsmithBlueprintMovementDecision::RejectContextJump => Some("context_jump_rejected"),
    }
}

fn toolsmith_blueprint_target_scope(proposal: &ToolProposal) -> String {
    proposal.target_scope.clone().unwrap_or_else(|| {
        stable_toolsmith_digest([proposal.rust_crate.as_str(), proposal.entrypoint.as_str()])
    })
}

fn stable_toolsmith_digest<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("toolsmith-digest:{hash:016x}")
}

fn toolsmith_plan_history_gate_repair_tasks(
    requires_repair_first: bool,
    reasons: &[String],
) -> Vec<AgentTask> {
    if !requires_repair_first {
        return Vec::new();
    }

    reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!("toolsmith-plan-repair-{index}"),
                AgentRole::Custom("toolsmith".to_owned()),
                format!("repair toolsmith plan: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("toolsmith-plan-repair")
            .with_priority(1)
        })
        .collect()
}

fn toolsmith_plan_history_gate_telemetry(
    can_promote_ready_proposals: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    summary: &ToolsmithPlanSummary,
    health_status: ToolsmithPlanHealthStatus,
) -> Vec<String> {
    vec![
        "agent_toolsmith_plan_history_gate=true".to_owned(),
        format!(
            "agent_toolsmith_plan_history_gate_health={}",
            health_status.as_str()
        ),
        format!("agent_toolsmith_plan_history_gate_promote_ready={can_promote_ready_proposals}"),
        format!("agent_toolsmith_plan_history_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_toolsmith_plan_history_gate_repair_tasks={repair_tasks}"),
        format!("agent_toolsmith_plan_history_gate_reasons={reasons}"),
        format!(
            "agent_toolsmith_plan_history_gate_rust_gate_passed={}",
            summary.rust_gate_passed
        ),
        format!("agent_toolsmith_plan_history_gate_ready={}", summary.ready),
        format!(
            "agent_toolsmith_plan_history_gate_non_rust_proposals={}",
            summary.non_rust_proposals
        ),
    ]
}

fn toolsmith_plan_history_gate_record_telemetry(
    health_record: &ToolsmithPlanSummaryHistoryRecord,
    gate_decision: &ToolsmithPlanHistoryGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_toolsmith_plan_history_gate_record=true".to_owned(),
        format!(
            "agent_toolsmith_plan_history_gate_record_health={}",
            health_record.health.status.as_str()
        ),
        format!(
            "agent_toolsmith_plan_history_gate_record_records={}",
            health_record.records()
        ),
        format!(
            "agent_toolsmith_plan_history_gate_record_promote_ready={}",
            gate_decision.can_promote_ready_proposals
        ),
        format!(
            "agent_toolsmith_plan_history_gate_record_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_toolsmith_plan_history_gate_record_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
    ];
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn process_reward_report_summary_telemetry(
    total: f32,
    action: RewardAction,
    signal_count: usize,
    note_count: usize,
    low_component_count: usize,
) -> Vec<String> {
    vec![
        "agent_process_reward_report_summary=true".to_owned(),
        format!("agent_process_reward_report_summary_total={total:.3}"),
        format!(
            "agent_process_reward_report_summary_action={}",
            action.as_str()
        ),
        format!("agent_process_reward_report_summary_signal_count={signal_count}"),
        format!("agent_process_reward_report_summary_note_count={note_count}"),
        format!("agent_process_reward_report_summary_low_component_count={low_component_count}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn process_reward_report_dashboard_telemetry(
    total_records: usize,
    reinforce_records: usize,
    hold_records: usize,
    penalize_records: usize,
    signal_count: usize,
    note_count: usize,
    low_component_count: usize,
    low_score_records: usize,
    missing_signal_records: usize,
    average_total: f32,
    reinforce_rate: f32,
    penalize_rate: f32,
) -> Vec<String> {
    vec![
        "agent_process_reward_report_dashboard=true".to_owned(),
        format!("agent_process_reward_report_dashboard_records={total_records}"),
        format!("agent_process_reward_report_dashboard_reinforce_records={reinforce_records}"),
        format!("agent_process_reward_report_dashboard_hold_records={hold_records}"),
        format!("agent_process_reward_report_dashboard_penalize_records={penalize_records}"),
        format!("agent_process_reward_report_dashboard_signal_count={signal_count}"),
        format!("agent_process_reward_report_dashboard_note_count={note_count}"),
        format!("agent_process_reward_report_dashboard_low_component_count={low_component_count}"),
        format!("agent_process_reward_report_dashboard_low_score_records={low_score_records}"),
        format!(
            "agent_process_reward_report_dashboard_missing_signal_records={missing_signal_records}"
        ),
        format!("agent_process_reward_report_dashboard_average_total={average_total:.3}"),
        format!("agent_process_reward_report_dashboard_reinforce_rate={reinforce_rate:.3}"),
        format!("agent_process_reward_report_dashboard_penalize_rate={penalize_rate:.3}"),
    ]
}

fn process_reward_report_history_record_telemetry(
    dashboard: &ProcessRewardReportDashboard,
    health: &ProcessRewardReportHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_process_reward_report_history_record=true".to_owned(),
        format!(
            "agent_process_reward_report_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_process_reward_report_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_process_reward_report_history_record_average_total={:.3}",
            dashboard.average_total
        ),
        format!(
            "agent_process_reward_report_history_record_penalize_records={}",
            dashboard.penalize_records
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_process_reward_report_history_record_reason={reason}")),
    );
    telemetry
}

fn process_reward_report_gate_reasons(summary: &ProcessRewardReportSummary) -> Vec<String> {
    let mut reasons = Vec::new();
    if summary.action == RewardAction::Penalize {
        reasons.push("process_reward_report_action=penalize".to_owned());
    }
    if summary.total < 0.42 {
        reasons.push(format!(
            "process_reward_report_total={:.3}<0.42",
            summary.total
        ));
    }
    if summary.signal_count == 0 {
        reasons.push("process_reward_report_missing_evolution_signals".to_owned());
    }
    reasons
}

fn process_reward_report_history_gate_repair_tasks(
    requires_repair_first: bool,
    reasons: &[String],
) -> Vec<AgentTask> {
    if !requires_repair_first {
        return Vec::new();
    }

    reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!("process-reward-report-repair-{index}"),
                AgentRole::Custom("process-reward".to_owned()),
                format!("repair process reward report: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("process-reward-report-repair")
            .with_priority(1)
        })
        .collect()
}

fn process_reward_report_history_gate_telemetry(
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    summary: &ProcessRewardReportSummary,
    health_status: ProcessRewardReportHealthStatus,
) -> Vec<String> {
    vec![
        "agent_process_reward_report_history_gate=true".to_owned(),
        format!(
            "agent_process_reward_report_history_gate_health={}",
            health_status.as_str()
        ),
        format!(
            "agent_process_reward_report_history_gate_promote_signals={can_promote_evolution_signals}"
        ),
        format!(
            "agent_process_reward_report_history_gate_reinforce_process={can_reinforce_process}"
        ),
        format!(
            "agent_process_reward_report_history_gate_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_process_reward_report_history_gate_repair_tasks={repair_tasks}"),
        format!("agent_process_reward_report_history_gate_reasons={reasons}"),
        format!(
            "agent_process_reward_report_history_gate_action={}",
            summary.action.as_str()
        ),
        format!(
            "agent_process_reward_report_history_gate_total={:.3}",
            summary.total
        ),
        format!(
            "agent_process_reward_report_history_gate_signal_count={}",
            summary.signal_count
        ),
    ]
}

fn process_reward_report_history_gate_record_telemetry(
    health_record: &ProcessRewardReportSummaryHistoryRecord,
    gate_decision: &ProcessRewardReportHistoryGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_process_reward_report_history_gate_record=true".to_owned(),
        format!(
            "agent_process_reward_report_history_gate_record_health={}",
            health_record.health.status.as_str()
        ),
        format!(
            "agent_process_reward_report_history_gate_record_records={}",
            health_record.records()
        ),
        format!(
            "agent_process_reward_report_history_gate_record_promote_signals={}",
            gate_decision.can_promote_evolution_signals
        ),
        format!(
            "agent_process_reward_report_history_gate_record_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_process_reward_report_history_gate_record_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
    ];
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn reflection_reward_admission_telemetry(
    reflection_health_status: ReflectionLoopHealthStatus,
    reward_health_status: ProcessRewardReportHealthStatus,
    can_continue_reflection: bool,
    can_promote_memory_note: bool,
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_reflection_reward_admission=true".to_owned(),
        format!(
            "agent_reflection_reward_admission_reflection_health={}",
            reflection_health_status.as_str()
        ),
        format!(
            "agent_reflection_reward_admission_reward_health={}",
            reward_health_status.as_str()
        ),
        format!("agent_reflection_reward_admission_continue={can_continue_reflection}"),
        format!("agent_reflection_reward_admission_memory_note={can_promote_memory_note}"),
        format!(
            "agent_reflection_reward_admission_evolution_signals={can_promote_evolution_signals}"
        ),
        format!("agent_reflection_reward_admission_reinforce_process={can_reinforce_process}"),
        format!("agent_reflection_reward_admission_requires_repair_first={requires_repair_first}"),
        format!("agent_reflection_reward_admission_repair_tasks={repair_tasks}"),
        format!("agent_reflection_reward_admission_blocked_reasons={blocked_reasons}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn reflection_reward_admission_summary_telemetry(
    reflection_health_status: ReflectionLoopHealthStatus,
    reward_health_status: ProcessRewardReportHealthStatus,
    can_continue_reflection: bool,
    can_promote_memory_note: bool,
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_reflection_reward_admission_summary=true".to_owned(),
        format!(
            "agent_reflection_reward_admission_summary_reflection_health={}",
            reflection_health_status.as_str()
        ),
        format!(
            "agent_reflection_reward_admission_summary_reward_health={}",
            reward_health_status.as_str()
        ),
        format!("agent_reflection_reward_admission_summary_continue={can_continue_reflection}"),
        format!("agent_reflection_reward_admission_summary_memory_note={can_promote_memory_note}"),
        format!(
            "agent_reflection_reward_admission_summary_evolution_signals={can_promote_evolution_signals}"
        ),
        format!(
            "agent_reflection_reward_admission_summary_reinforce_process={can_reinforce_process}"
        ),
        format!(
            "agent_reflection_reward_admission_summary_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_reflection_reward_admission_summary_repair_tasks={repair_tasks}"),
        format!("agent_reflection_reward_admission_summary_blocked_reasons={blocked_reasons}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_decision_telemetry(
    toolsmith_health_status: ToolsmithPlanHealthStatus,
    reward_health_status: ProcessRewardReportHealthStatus,
    can_promote_ready_proposals: bool,
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_evolution_admission=true".to_owned(),
        format!(
            "agent_evolution_admission_toolsmith_health={}",
            toolsmith_health_status.as_str()
        ),
        format!(
            "agent_evolution_admission_reward_health={}",
            reward_health_status.as_str()
        ),
        format!("agent_evolution_admission_promote_ready={can_promote_ready_proposals}"),
        format!("agent_evolution_admission_promote_signals={can_promote_evolution_signals}"),
        format!("agent_evolution_admission_reinforce={can_reinforce_process}"),
        format!("agent_evolution_admission_adaptive_state={can_promote_adaptive_state}"),
        format!("agent_evolution_admission_requires_repair_first={requires_repair_first}"),
        format!("agent_evolution_admission_repair_tasks={repair_tasks}"),
        format!("agent_evolution_admission_blocked_reasons={blocked_reasons}"),
    ]
}

fn evolution_admission_record_telemetry(
    toolsmith_record: &ToolsmithPlanHistoryGateRecord,
    reward_record: &ProcessRewardReportHistoryGateRecord,
    decision: &EvolutionAdmissionDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_evolution_admission_record=true".to_owned(),
        format!(
            "agent_evolution_admission_record_toolsmith_records={}",
            toolsmith_record.records()
        ),
        format!(
            "agent_evolution_admission_record_reward_records={}",
            reward_record.records()
        ),
        format!(
            "agent_evolution_admission_record_promote_ready={}",
            decision.can_promote_ready_proposals
        ),
        format!(
            "agent_evolution_admission_record_promote_signals={}",
            decision.can_promote_evolution_signals
        ),
        format!(
            "agent_evolution_admission_record_adaptive_state={}",
            decision.can_promote_adaptive_state
        ),
        format!(
            "agent_evolution_admission_record_requires_repair_first={}",
            decision.requires_repair_first
        ),
        format!(
            "agent_evolution_admission_record_repair_tasks={}",
            decision.repair_tasks.len()
        ),
    ];
    telemetry.extend(decision.telemetry.iter().cloned());
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_summary_telemetry(
    records: usize,
    admitted: bool,
    can_promote_ready_proposals: bool,
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    blocked_reasons: usize,
    toolsmith_health_status: ToolsmithPlanHealthStatus,
    reward_health_status: ProcessRewardReportHealthStatus,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_summary=true".to_owned(),
        format!("agent_evolution_admission_summary_records={records}"),
        format!("agent_evolution_admission_summary_admitted={admitted}"),
        format!("agent_evolution_admission_summary_promote_ready={can_promote_ready_proposals}"),
        format!(
            "agent_evolution_admission_summary_promote_signals={can_promote_evolution_signals}"
        ),
        format!("agent_evolution_admission_summary_reinforce={can_reinforce_process}"),
        format!("agent_evolution_admission_summary_adaptive_state={can_promote_adaptive_state}"),
        format!("agent_evolution_admission_summary_requires_repair_first={requires_repair_first}"),
        format!("agent_evolution_admission_summary_repair_tasks={repair_tasks}"),
        format!("agent_evolution_admission_summary_blocked_reasons={blocked_reasons}"),
        format!(
            "agent_evolution_admission_summary_toolsmith_health={}",
            toolsmith_health_status.as_str()
        ),
        format!(
            "agent_evolution_admission_summary_reward_health={}",
            reward_health_status.as_str()
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_dashboard_telemetry(
    total_records: usize,
    admitted_records: usize,
    repair_first_records: usize,
    ready_promotion_records: usize,
    signal_promotion_records: usize,
    reinforcement_records: usize,
    adaptive_state_records: usize,
    repair_task_count: usize,
    blocked_reason_count: usize,
    toolsmith_repair_records: usize,
    reward_repair_records: usize,
    admission_rate: f32,
    adaptive_state_rate: f32,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_dashboard=true".to_owned(),
        format!("agent_evolution_admission_dashboard_records={total_records}"),
        format!("agent_evolution_admission_dashboard_admitted_records={admitted_records}"),
        format!("agent_evolution_admission_dashboard_repair_first_records={repair_first_records}"),
        format!(
            "agent_evolution_admission_dashboard_ready_promotion_records={ready_promotion_records}"
        ),
        format!(
            "agent_evolution_admission_dashboard_signal_promotion_records={signal_promotion_records}"
        ),
        format!(
            "agent_evolution_admission_dashboard_reinforcement_records={reinforcement_records}"
        ),
        format!(
            "agent_evolution_admission_dashboard_adaptive_state_records={adaptive_state_records}"
        ),
        format!("agent_evolution_admission_dashboard_repair_task_count={repair_task_count}"),
        format!("agent_evolution_admission_dashboard_blocked_reason_count={blocked_reason_count}"),
        format!(
            "agent_evolution_admission_dashboard_toolsmith_repair_records={toolsmith_repair_records}"
        ),
        format!(
            "agent_evolution_admission_dashboard_reward_repair_records={reward_repair_records}"
        ),
        format!("agent_evolution_admission_dashboard_admission_rate={admission_rate:.3}"),
        format!("agent_evolution_admission_dashboard_adaptive_state_rate={adaptive_state_rate:.3}"),
    ]
}

fn evolution_admission_history_record_telemetry(
    dashboard: &EvolutionAdmissionDashboard,
    health: &EvolutionAdmissionHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_evolution_admission_history_record=true".to_owned(),
        format!(
            "agent_evolution_admission_history_health={}",
            health.status.as_str()
        ),
        format!(
            "agent_evolution_admission_history_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_evolution_admission_history_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_evolution_admission_history_adaptive_state_rate={:.3}",
            dashboard.adaptive_state_rate
        ),
        format!(
            "agent_evolution_admission_history_repair_first_records={}",
            dashboard.repair_first_records
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_evolution_admission_history_reason={reason}")),
    );
    telemetry
}

fn evolution_admission_history_gate_repair_tasks(
    history_requires_repair: bool,
    reasons: &[String],
) -> Vec<AgentTask> {
    if !history_requires_repair {
        return Vec::new();
    }

    reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!("evolution-admission-repair-{index}"),
                AgentRole::Custom("evolution-admission".to_owned()),
                format!("repair evolution admission trend: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("evolution-admission")
            .with_priority(8)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_history_gate_telemetry(
    can_promote_ready_proposals: bool,
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    blocked_reasons: usize,
    admission_summary_admitted: bool,
    admission_health_status: EvolutionAdmissionHealthStatus,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_history_gate=true".to_owned(),
        format!(
            "agent_evolution_admission_history_gate_health={}",
            admission_health_status.as_str()
        ),
        format!(
            "agent_evolution_admission_history_gate_summary_admitted={admission_summary_admitted}"
        ),
        format!(
            "agent_evolution_admission_history_gate_promote_ready={can_promote_ready_proposals}"
        ),
        format!(
            "agent_evolution_admission_history_gate_promote_signals={can_promote_evolution_signals}"
        ),
        format!("agent_evolution_admission_history_gate_reinforce={can_reinforce_process}"),
        format!(
            "agent_evolution_admission_history_gate_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_evolution_admission_history_gate_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_evolution_admission_history_gate_repair_tasks={repair_tasks}"),
        format!("agent_evolution_admission_history_gate_blocked_reasons={blocked_reasons}"),
    ]
}

fn evolution_admission_history_gate_record_telemetry(
    health_record: &EvolutionAdmissionSummaryHistoryRecord,
    gate_decision: &EvolutionAdmissionHistoryGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_evolution_admission_history_gate_record=true".to_owned(),
        format!(
            "agent_evolution_admission_history_gate_record_records={}",
            health_record.records()
        ),
        format!(
            "agent_evolution_admission_history_gate_record_health={}",
            gate_decision.admission_health.status.as_str()
        ),
        format!(
            "agent_evolution_admission_history_gate_record_admitted={}",
            gate_decision.is_admitted()
        ),
        format!(
            "agent_evolution_admission_history_gate_record_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_evolution_admission_history_gate_record_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
    ];
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_handoff_telemetry(
    effective_admitted: bool,
    can_promote_ready_proposals: bool,
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    records: usize,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_handoff=true".to_owned(),
        format!("agent_evolution_admission_handoff_effective_admitted={effective_admitted}"),
        format!("agent_evolution_admission_handoff_promote_ready={can_promote_ready_proposals}"),
        format!(
            "agent_evolution_admission_handoff_promote_signals={can_promote_evolution_signals}"
        ),
        format!("agent_evolution_admission_handoff_reinforce={can_reinforce_process}"),
        format!("agent_evolution_admission_handoff_adaptive_state={can_promote_adaptive_state}"),
        format!("agent_evolution_admission_handoff_requires_repair_first={requires_repair_first}"),
        format!("agent_evolution_admission_handoff_repair_tasks={repair_tasks}"),
        format!("agent_evolution_admission_handoff_next_queue_tasks={next_queue_tasks}"),
        format!("agent_evolution_admission_handoff_blocked_reasons={blocked_reasons}"),
        format!("agent_evolution_admission_handoff_records={records}"),
    ]
}

fn evolution_admission_handoff_summary_telemetry(
    effective_admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    records: usize,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_handoff_summary=true".to_owned(),
        format!(
            "agent_evolution_admission_handoff_summary_effective_admitted={effective_admitted}"
        ),
        format!(
            "agent_evolution_admission_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_evolution_admission_handoff_summary_repair_tasks={repair_tasks}"),
        format!("agent_evolution_admission_handoff_summary_next_queue_tasks={next_queue_tasks}"),
        format!("agent_evolution_admission_handoff_summary_blocked_reasons={blocked_reasons}"),
        format!("agent_evolution_admission_handoff_summary_records={records}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_handoff_dashboard_telemetry(
    total_records: usize,
    requested_admitted_records: usize,
    effective_admitted_records: usize,
    repair_first_records: usize,
    ready_promotion_records: usize,
    signal_promotion_records: usize,
    reinforcement_records: usize,
    adaptive_state_records: usize,
    repair_task_count: usize,
    next_queue_task_count: usize,
    blocked_reason_count: usize,
    admission_repair_records: usize,
    effective_admitted_rate: f32,
    adaptive_state_rate: f32,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_handoff_dashboard=true".to_owned(),
        format!("agent_evolution_admission_handoff_dashboard_records={total_records}"),
        format!(
            "agent_evolution_admission_handoff_dashboard_requested_admitted_records={requested_admitted_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_dashboard_effective_admitted_records={effective_admitted_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_dashboard_repair_first_records={repair_first_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_dashboard_ready_promotion_records={ready_promotion_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_dashboard_signal_promotion_records={signal_promotion_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_dashboard_reinforcement_records={reinforcement_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_dashboard_adaptive_state_records={adaptive_state_records}"
        ),
        format!("agent_evolution_admission_handoff_dashboard_repair_tasks={repair_task_count}"),
        format!(
            "agent_evolution_admission_handoff_dashboard_next_queue_tasks={next_queue_task_count}"
        ),
        format!(
            "agent_evolution_admission_handoff_dashboard_blocked_reasons={blocked_reason_count}"
        ),
        format!(
            "agent_evolution_admission_handoff_dashboard_admission_repair_records={admission_repair_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_dashboard_effective_admitted_rate={effective_admitted_rate:.3}"
        ),
        format!(
            "agent_evolution_admission_handoff_dashboard_adaptive_state_rate={adaptive_state_rate:.3}"
        ),
    ]
}

fn evolution_admission_handoff_history_record_telemetry(
    dashboard: &EvolutionAdmissionHandoffDashboard,
    health: &EvolutionAdmissionHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_evolution_admission_handoff_history_record=true".to_owned(),
        format!(
            "agent_evolution_admission_handoff_history_health={}",
            health.status.as_str()
        ),
        format!(
            "agent_evolution_admission_handoff_history_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_evolution_admission_handoff_history_effective_admitted_rate={:.3}",
            dashboard.effective_admitted_rate
        ),
        format!(
            "agent_evolution_admission_handoff_history_adaptive_state_rate={:.3}",
            dashboard.adaptive_state_rate
        ),
        format!(
            "agent_evolution_admission_handoff_history_repair_first_records={}",
            dashboard.repair_first_records
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_evolution_admission_handoff_history_reason={reason}")),
    );
    telemetry
}

fn evolution_admission_handoff_trend_gate_repair_tasks(
    handoff_health: &EvolutionAdmissionHandoffHealth,
    blocked_reasons: &[String],
) -> Vec<AgentTask> {
    if !handoff_health.requires_repair_first() {
        return Vec::new();
    }

    blocked_reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!("evolution-admission-handoff-trend-repair-{index}"),
                AgentRole::Custom("evolution-admission-handoff".to_owned()),
                format!("repair evolution admission handoff trend: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("evolution-admission-handoff-trend-repair")
            .with_priority(8)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_handoff_trend_gate_telemetry(
    requested_admitted: bool,
    effective_admitted: bool,
    handoff_health_status: EvolutionAdmissionHealthStatus,
    requires_repair_first: bool,
    can_promote_ready_proposals: bool,
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    can_promote_adaptive_state: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    records: usize,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_handoff_trend_gate=true".to_owned(),
        format!(
            "agent_evolution_admission_handoff_trend_gate_health={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_evolution_admission_handoff_trend_gate_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_gate_effective_admitted={effective_admitted}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_gate_promote_ready={can_promote_ready_proposals}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_gate_promote_signals={can_promote_evolution_signals}"
        ),
        format!("agent_evolution_admission_handoff_trend_gate_reinforce={can_reinforce_process}"),
        format!(
            "agent_evolution_admission_handoff_trend_gate_adaptive_state={can_promote_adaptive_state}"
        ),
        format!("agent_evolution_admission_handoff_trend_gate_repair_tasks={repair_tasks}"),
        format!("agent_evolution_admission_handoff_trend_gate_next_queue_tasks={next_queue_tasks}"),
        format!("agent_evolution_admission_handoff_trend_gate_blocked_reasons={blocked_reasons}"),
        format!("agent_evolution_admission_handoff_trend_gate_records={records}"),
    ]
}

fn evolution_admission_handoff_trend_monitor_telemetry(
    handoff_health_status: EvolutionAdmissionHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    records: usize,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_handoff_trend_monitor=true".to_owned(),
        format!(
            "agent_evolution_admission_handoff_trend_monitor_health={}",
            handoff_health_status.as_str()
        ),
        format!("agent_evolution_admission_handoff_trend_monitor_admitted={admitted}"),
        format!(
            "agent_evolution_admission_handoff_trend_monitor_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_evolution_admission_handoff_trend_monitor_repair_tasks={repair_tasks}"),
        format!(
            "agent_evolution_admission_handoff_trend_monitor_next_queue_tasks={next_queue_tasks}"
        ),
        format!("agent_evolution_admission_handoff_trend_monitor_records={records}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_handoff_trend_continuation_telemetry(
    handoff_health_status: EvolutionAdmissionHealthStatus,
    effective_admitted: bool,
    can_promote_ready_proposals: bool,
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    next_queue_tasks: usize,
    handoff_history_records: usize,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_handoff_trend_continuation=true".to_owned(),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_health={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_effective_admitted={effective_admitted}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_promote_ready={can_promote_ready_proposals}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_promote_signals={can_promote_evolution_signals}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_reinforce={can_reinforce_process}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_records={handoff_history_records}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_handoff_trend_continuation_summary_telemetry(
    handoff_health_status: EvolutionAdmissionHealthStatus,
    effective_admitted: bool,
    can_promote_ready_proposals: bool,
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    next_queue_tasks: usize,
    handoff_history_records: usize,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_handoff_trend_continuation_summary=true".to_owned(),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_summary_health={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_summary_effective_admitted={effective_admitted}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_summary_promote_ready={can_promote_ready_proposals}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_summary_promote_signals={can_promote_evolution_signals}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_summary_reinforce={can_reinforce_process}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_summary_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_summary_history_records={handoff_history_records}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_handoff_trend_continuation_dashboard_telemetry(
    total_records: usize,
    effective_admitted_records: usize,
    repair_first_records: usize,
    ready_promotion_records: usize,
    signal_promotion_records: usize,
    reinforcement_records: usize,
    adaptive_state_records: usize,
    next_queue_task_count: usize,
    handoff_history_record_count: usize,
    handoff_repair_records: usize,
    effective_admitted_rate: f32,
    adaptive_state_rate: f32,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_handoff_trend_continuation_dashboard=true".to_owned(),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_records={total_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_effective_admitted_records={effective_admitted_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_repair_first_records={repair_first_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_ready_promotion_records={ready_promotion_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_signal_promotion_records={signal_promotion_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_reinforcement_records={reinforcement_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_adaptive_state_records={adaptive_state_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_next_queue_tasks={next_queue_task_count}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_handoff_history_records={handoff_history_record_count}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_handoff_repair_records={handoff_repair_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_effective_admitted_rate={effective_admitted_rate:.3}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_dashboard_adaptive_state_rate={adaptive_state_rate:.3}"
        ),
    ]
}

fn evolution_admission_handoff_trend_continuation_history_record_telemetry(
    dashboard: &EvolutionAdmissionHandoffTrendContinuationDashboard,
    health: &EvolutionAdmissionHandoffTrendContinuationHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_evolution_admission_handoff_trend_continuation_history_record=true".to_owned(),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_health={}",
            health.status.as_str()
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_effective_admitted_rate={:.3}",
            dashboard.effective_admitted_rate
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_adaptive_state_rate={:.3}",
            dashboard.adaptive_state_rate
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_repair_first_records={}",
            dashboard.repair_first_records
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!("agent_evolution_admission_handoff_trend_continuation_history_reason={reason}")
    }));
    telemetry
}

fn evolution_admission_handoff_trend_continuation_history_gate_repair_tasks(
    continuation_health: &EvolutionAdmissionHandoffTrendContinuationHealth,
    blocked_reasons: &[String],
) -> Vec<AgentTask> {
    if !continuation_health.requires_repair_first() {
        return Vec::new();
    }

    blocked_reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!("evolution-admission-handoff-trend-continuation-repair-{index}"),
                AgentRole::Custom("evolution-admission-handoff-continuation".to_owned()),
                format!("repair evolution admission handoff continuation: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("evolution-admission-handoff-trend-continuation-repair")
            .with_priority(8)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_handoff_trend_continuation_history_gate_summary_telemetry(
    continuation_health_status: EvolutionAdmissionHealthStatus,
    effective_admitted: bool,
    can_promote_ready_proposals: bool,
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    records: usize,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_handoff_trend_continuation_history_gate_summary=true".to_owned(),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_health={}",
            continuation_health_status.as_str()
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_effective_admitted={effective_admitted}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_promote_ready={can_promote_ready_proposals}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_promote_signals={can_promote_evolution_signals}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_reinforce={can_reinforce_process}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_records={records}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_handoff_trend_continuation_history_gate_dashboard_telemetry(
    total_records: usize,
    effective_admitted_records: usize,
    repair_first_records: usize,
    ready_promotion_records: usize,
    signal_promotion_records: usize,
    reinforcement_records: usize,
    adaptive_state_records: usize,
    repair_task_count: usize,
    next_queue_task_count: usize,
    blocked_reason_count: usize,
    continuation_repair_records: usize,
    effective_admitted_rate: f32,
    adaptive_state_rate: f32,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard=true"
            .to_owned(),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_records={total_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_effective_admitted_records={effective_admitted_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_repair_first_records={repair_first_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_ready_promotion_records={ready_promotion_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_signal_promotion_records={signal_promotion_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_reinforcement_records={reinforcement_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_adaptive_state_records={adaptive_state_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_next_queue_tasks={next_queue_task_count}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_blocked_reasons={blocked_reason_count}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_continuation_repair_records={continuation_repair_records}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_effective_admitted_rate={effective_admitted_rate:.3}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_adaptive_state_rate={adaptive_state_rate:.3}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn evolution_admission_handoff_trend_continuation_history_gate_telemetry(
    continuation_health_status: EvolutionAdmissionHealthStatus,
    effective_admitted: bool,
    can_promote_ready_proposals: bool,
    can_promote_evolution_signals: bool,
    can_reinforce_process: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    records: usize,
) -> Vec<String> {
    vec![
        "agent_evolution_admission_handoff_trend_continuation_history_gate=true".to_owned(),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_health={}",
            continuation_health_status.as_str()
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_effective_admitted={effective_admitted}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_promote_ready={can_promote_ready_proposals}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_promote_signals={can_promote_evolution_signals}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_reinforce={can_reinforce_process}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_records={records}"
        ),
    ]
}

fn evolution_admission_handoff_trend_continuation_history_gate_record_telemetry(
    health_record: &EvolutionAdmissionHandoffTrendContinuationHistoryRecord,
    gate_decision: &EvolutionAdmissionHandoffTrendContinuationHistoryGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_evolution_admission_handoff_trend_continuation_history_gate_record=true".to_owned(),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_record_records={}",
            health_record.records()
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_record_health={}",
            gate_decision.continuation_health.status.as_str()
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_record_admitted={}",
            gate_decision.is_admitted()
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_record_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_record_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
    ];
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn evolution_admission_handoff_trend_continuation_history_gate_history_record_telemetry(
    dashboard: &EvolutionAdmissionHandoffTrendContinuationHistoryGateDashboard,
    health: &EvolutionAdmissionHandoffTrendContinuationHistoryGateHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_evolution_admission_handoff_trend_continuation_history_gate_history_record=true"
            .to_owned(),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_history_health={}",
            health.status.as_str()
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_history_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_history_effective_admitted_rate={:.3}",
            dashboard.effective_admitted_rate
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_history_adaptive_state_rate={:.3}",
            dashboard.adaptive_state_rate
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_history_repair_first_records={}",
            dashboard.repair_first_records
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_history_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_history_next_queue_tasks={}",
            dashboard.next_queue_task_count
        ),
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_history_blocked_reasons={}",
            dashboard.blocked_reason_count
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "agent_evolution_admission_handoff_trend_continuation_history_gate_history_reason={reason}"
        )
    }));
    telemetry
}

fn extend_ordered_unique(target: &mut Vec<String>, items: Vec<String>) {
    for item in items {
        if !target.contains(&item) {
            target.push(item);
        }
    }
}

fn rate(count: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        count as f32 / total as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::AggregationReport;
    use crate::conflict::{AgentConflict, ConflictReport};
    use crate::reflection::{
        ReflectionLoop, ReflectionLoopHealthPolicy, ReflectionLoopHistoryGateRecord,
        ReflectionLoopSummary, ReflectionLoopSummaryHistory, ReflectionLoopSummaryHistoryRecorder,
        ReflectionStage,
    };
    use crate::run::SideEffectGate;
    use crate::task::AgentRole;

    fn closed_reflection_loop() -> ReflectionLoop {
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft answer")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "needs evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "add evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember reflection evidence")
            .unwrap();
        loop_state
    }

    fn draft_only_reflection_loop() -> ReflectionLoop {
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft answer")
            .unwrap();
        loop_state
    }

    fn stalled_reflection_summary() -> ReflectionLoopSummary {
        ReflectionLoopSummary {
            entries: 1,
            next_stage: ReflectionStage::Critique,
            is_complete: false,
            memory_note_ready: false,
            remaining_stages: vec![
                ReflectionStage::Critique,
                ReflectionStage::Revision,
                ReflectionStage::MemoryNote,
            ],
            telemetry: Vec::new(),
        }
    }

    fn reinforce_reward_report() -> ProcessRewardReport {
        ProcessRewardReport {
            total: 0.82,
            components: ProcessRewardComponents {
                coordination: 0.8,
                reflection: 0.8,
                validation: 0.8,
                toolsmith: 0.8,
                recursion: 0.8,
                admission: 0.8,
            },
            action: RewardAction::Reinforce,
            notes: vec!["total:0.820:reinforce".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "agent_coordination",
                "promote_closed_loop_pattern",
                "clean reward",
                0.82,
            )],
        }
    }

    fn dirty_reward_summary() -> ProcessRewardReportSummary {
        ProcessRewardReportSummary {
            total: 0.20,
            action: RewardAction::Penalize,
            signal_count: 0,
            note_count: 2,
            low_component_count: 3,
            telemetry: Vec::new(),
        }
    }

    fn reflection_gate_record(
        history: ReflectionLoopSummaryHistory,
        loop_state: &ReflectionLoop,
    ) -> ReflectionLoopHistoryGateRecord {
        ReflectionLoopSummaryHistoryRecorder::new().record_loop_with_health_gate(
            history,
            loop_state,
            ReflectionLoopHealthPolicy::default(),
        )
    }

    fn reward_gate_record(
        history: ProcessRewardReportSummaryHistory,
        report: &ProcessRewardReport,
    ) -> ProcessRewardReportHistoryGateRecord {
        ProcessRewardReportSummaryHistoryRecorder::new().record_report_with_health_gate(
            history,
            report,
            ProcessRewardReportHealthPolicy::default(),
        )
    }

    fn clean_evolution_admission_record() -> EvolutionAdmissionRecord {
        let plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));
        let toolsmith_record = ToolsmithPlanSummaryHistoryRecorder::new()
            .record_plan_with_health_gate(
                ToolsmithPlanSummaryHistory::new(),
                &plan,
                ToolsmithPlanHealthPolicy::default(),
            );
        let reward_report = ProcessRewardReport {
            total: 0.82,
            components: ProcessRewardComponents {
                coordination: 0.8,
                reflection: 0.8,
                validation: 0.8,
                toolsmith: 0.8,
                recursion: 0.8,
                admission: 0.8,
            },
            action: RewardAction::Reinforce,
            notes: vec!["total:0.820:reinforce".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "agent_coordination",
                "promote_closed_loop_pattern",
                "clean reward",
                0.82,
            )],
        };
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &reward_report,
                ProcessRewardReportHealthPolicy::default(),
            );

        EvolutionAdmissionGate::new().gate(toolsmith_record, reward_record)
    }

    fn repair_evolution_admission_record() -> EvolutionAdmissionRecord {
        let dirty_plan = ToolsmithPlan::new()
            .with_proposal(ToolProposal::new(
                "trace-script",
                ToolIntent::TraceAnalysis,
                "python",
                "tools/trace.py",
                ToolBuildStatus::Rejected,
            ))
            .with_rejected_request("shell tool outside rust crate");
        let toolsmith_record = ToolsmithPlanSummaryHistoryRecorder::new()
            .record_plan_with_health_gate(
                ToolsmithPlanSummaryHistory::new(),
                &dirty_plan,
                ToolsmithPlanHealthPolicy::default(),
            );
        let reward_report = ProcessRewardReport {
            total: 0.20,
            components: ProcessRewardComponents {
                coordination: 0.2,
                reflection: 0.2,
                validation: 0.2,
                toolsmith: 0.2,
                recursion: 0.2,
                admission: 0.2,
            },
            action: RewardAction::Penalize,
            notes: vec!["total:0.200:penalize".to_owned()],
            evolution_signals: Vec::new(),
        };
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &reward_report,
                ProcessRewardReportHealthPolicy::default(),
            );

        EvolutionAdmissionGate::new().gate(toolsmith_record, reward_record)
    }

    fn stable_evolution_admission_handoff() -> EvolutionAdmissionHandoff {
        let admission = clean_evolution_admission_record();
        let gate_record = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health_gate(
                EvolutionAdmissionSummaryHistory::new(),
                &admission,
                EvolutionAdmissionHealthPolicy::default(),
            );

        EvolutionAdmissionHandoff::from_gate_record(
            gate_record,
            AgentTaskQueue::from_tasks(vec![AgentTask::new(
                "business-task",
                AgentRole::Planner,
                "continue self-evolution loop",
                AgentBudget::new(8, 1, 1),
            )]),
        )
    }

    fn repair_evolution_admission_handoff() -> EvolutionAdmissionHandoff {
        let dirty_admission = repair_evolution_admission_record();
        let clean_admission = clean_evolution_admission_record();
        let dirty_history = EvolutionAdmissionSummaryHistory::from_summaries(vec![
            EvolutionAdmissionSummary::from_record(&dirty_admission),
        ]);
        let gate_record = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health_gate(
                dirty_history,
                &clean_admission,
                EvolutionAdmissionHealthPolicy::default(),
            );

        EvolutionAdmissionHandoff::from_gate_record(
            gate_record,
            AgentTaskQueue::from_tasks(vec![AgentTask::new(
                "business-task",
                AgentRole::Planner,
                "continue self-evolution loop",
                AgentBudget::new(8, 1, 1),
            )]),
        )
    }

    fn stable_continuation_history_gate_summary()
    -> EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary {
        let handoff = stable_evolution_admission_handoff();
        let handoff_policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            handoff_policy,
        );
        let continuation = monitor_record.continuation(handoff_policy);
        EvolutionAdmissionHandoffTrendContinuationHistoryRecorder::new()
            .record_continuation_with_health_gate(
                EvolutionAdmissionHandoffTrendContinuationSummaryHistory::new(),
                &continuation,
                EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default(),
            )
            .summary()
    }

    fn repair_continuation_history_gate_summary()
    -> EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary {
        let dirty_handoff = repair_evolution_admission_handoff();
        let clean_handoff = stable_evolution_admission_handoff();
        let handoff_policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let dirty_monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &dirty_handoff,
            handoff_policy,
        );
        let dirty_continuation = dirty_monitor_record.continuation(handoff_policy);
        let clean_monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &clean_handoff,
            handoff_policy,
        );
        let clean_continuation = clean_monitor_record.continuation(handoff_policy);
        let dirty_history =
            EvolutionAdmissionHandoffTrendContinuationSummaryHistory::from_summaries(vec![
                dirty_continuation.summary(),
            ]);
        let history_record = EvolutionAdmissionHandoffTrendContinuationHistoryRecorder::new()
            .record_continuation_with_health(
                dirty_history,
                &clean_continuation,
                EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default(),
            );

        EvolutionAdmissionHandoffTrendContinuationHistoryGate::new()
            .gate(&clean_continuation, &history_record)
            .summary()
    }

    #[test]
    fn toolsmith_plan_history_watches_empty() {
        let health =
            ToolsmithPlanSummaryHistory::new().health(ToolsmithPlanHealthPolicy::default());

        assert_eq!(health.status, ToolsmithPlanHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["toolsmith_plan_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| line == "agent_toolsmith_plan_dashboard_records=0")
        );
    }

    #[test]
    fn toolsmith_plan_history_marks_rust_ready_plan_stable() {
        let plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));

        let record = ToolsmithPlanSummaryHistoryRecorder::new().record_plan_with_health(
            ToolsmithPlanSummaryHistory::new(),
            &plan,
            ToolsmithPlanHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert_eq!(record.appended_summary.ready, 1);
        assert_eq!(record.dashboard.ready_rate, 1.0);
        assert_eq!(record.dashboard.rust_gate_pass_rate, 1.0);
        assert_eq!(record.health.status, ToolsmithPlanHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.health.allows_service_advance());
        assert!(!record.health.requires_repair_first());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "agent_toolsmith_plan_history_record_status=stable")
        );
    }

    #[test]
    fn toolsmith_plan_history_repairs_non_rust_and_rejected_requests() {
        let clean = ToolsmithPlanSummary {
            proposals: 1,
            ready: 1,
            held: 0,
            rejected: 0,
            rejected_requests: 0,
            non_rust_proposals: 0,
            rust_only: true,
            rust_gate_passed: true,
            telemetry: Vec::new(),
        };
        let dirty = ToolsmithPlan::new()
            .with_proposal(ToolProposal::new(
                "trace-script",
                ToolIntent::TraceAnalysis,
                "python",
                "tools/trace.py",
                ToolBuildStatus::Ready,
            ))
            .with_rejected_request("shell tool outside rust crate")
            .summary();
        let history = ToolsmithPlanSummaryHistory::from_summaries(vec![clean]);

        let record = ToolsmithPlanSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            dirty,
            ToolsmithPlanHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.rejected, 1);
        assert_eq!(record.dashboard.rejected_requests, 1);
        assert_eq!(record.dashboard.non_rust_proposals, 1);
        assert_eq!(record.dashboard.rust_gate_failed_records, 1);
        assert_eq!(record.health.status, ToolsmithPlanHealthStatus::Repair);
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "toolsmith_plan_rejected=1>0".to_owned(),
                "toolsmith_plan_rejected_requests=1>0".to_owned(),
                "toolsmith_plan_non_rust_proposals=1>0".to_owned(),
                "toolsmith_plan_rust_gate_failed_records=1>0".to_owned(),
            ]
        );
    }

    #[test]
    fn toolsmith_plan_history_gate_promotes_stable_rust_ready_plan() {
        let plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));
        let history_record = ToolsmithPlanSummaryHistoryRecorder::new().record_plan_with_health(
            ToolsmithPlanSummaryHistory::new(),
            &plan,
            ToolsmithPlanHealthPolicy::default(),
        );

        let gate = ToolsmithPlanHistoryGate::new().gate(&plan, &history_record);

        assert!(gate.can_promote_ready_proposals);
        assert!(gate.is_promotable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert!(gate.reasons.is_empty());
        assert_eq!(
            gate.toolsmith_health.status,
            ToolsmithPlanHealthStatus::Stable
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_toolsmith_plan_history_gate_promote_ready=true" })
        );
    }

    #[test]
    fn toolsmith_plan_history_gate_blocks_moved_blueprint_without_review() {
        let plan = ToolsmithPlan::new().with_proposal(
            ToolProposal::new(
                "runtime-gate",
                ToolIntent::BenchmarkGate,
                "rust",
                "tools/runtime_gate.rs",
                ToolBuildStatus::Ready,
            )
            .with_source_scope("workspace-a")
            .with_target_scope("workspace-b"),
        );
        let history_record = ToolsmithPlanSummaryHistoryRecorder::new().record_plan_with_health(
            ToolsmithPlanSummaryHistory::new(),
            &plan,
            ToolsmithPlanHealthPolicy::default(),
        );

        let gate = ToolsmithPlanHistoryGate::new().gate(&plan, &history_record);

        assert!(!gate.can_promote_ready_proposals);
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec!["toolsmith_blueprint_movement:runtime-gate:review_missing".to_owned()]
        );
        assert_eq!(gate.repair_tasks.len(), 1);
    }

    #[test]
    fn toolsmith_plan_history_gate_promotes_moved_blueprint_with_preview_review() {
        let target_scope = "workspace-b".to_owned();
        let proposal = ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        )
        .with_source_scope("workspace-a")
        .with_target_scope(target_scope.clone());
        let review = ToolsmithBlueprintMovementReview::new(
            proposal.id.clone(),
            proposal.blueprint_digest(),
            "workspace-a",
            target_scope.clone(),
        )
        .with_allowed_scope_tags(vec![target_scope])
        .with_decision(ToolsmithBlueprintMovementDecision::AllowPreviewMove);
        let plan = ToolsmithPlan::new().with_proposal(proposal.with_movement_review(review));
        let history_record = ToolsmithPlanSummaryHistoryRecorder::new().record_plan_with_health(
            ToolsmithPlanSummaryHistory::new(),
            &plan,
            ToolsmithPlanHealthPolicy::default(),
        );

        let gate = ToolsmithPlanHistoryGate::new().gate(&plan, &history_record);

        assert!(gate.can_promote_ready_proposals);
        assert!(gate.is_promotable());
        assert!(!gate.requires_repair_first);
        assert!(gate.reasons.is_empty());
        assert!(gate.repair_tasks.is_empty());
    }

    #[test]
    fn toolsmith_plan_history_gate_repairs_dirty_history_before_promotion() {
        let dirty = ToolsmithPlan::new()
            .with_proposal(ToolProposal::new(
                "trace-script",
                ToolIntent::TraceAnalysis,
                "python",
                "tools/trace.py",
                ToolBuildStatus::Ready,
            ))
            .with_rejected_request("shell tool outside rust crate")
            .summary();
        let clean_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));
        let history_record = ToolsmithPlanSummaryHistoryRecorder::new().record_plan_with_health(
            ToolsmithPlanSummaryHistory::from_summaries(vec![dirty]),
            &clean_plan,
            ToolsmithPlanHealthPolicy::default(),
        );

        let gate = ToolsmithPlanHistoryGate::new().gate(&clean_plan, &history_record);

        assert!(!gate.can_promote_ready_proposals);
        assert!(!gate.is_promotable());
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.toolsmith_health.status,
            ToolsmithPlanHealthStatus::Repair
        );
        assert_eq!(gate.repair_tasks.len(), 4);
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "toolsmith-plan-repair-0",
                "toolsmith-plan-repair-1",
                "toolsmith-plan-repair-2",
                "toolsmith-plan-repair-3",
            ]
        );
        assert_eq!(
            gate.reasons,
            vec![
                "toolsmith_plan_history:toolsmith_plan_rejected=1>0",
                "toolsmith_plan_history:toolsmith_plan_rejected_requests=1>0",
                "toolsmith_plan_history:toolsmith_plan_non_rust_proposals=1>0",
                "toolsmith_plan_history:toolsmith_plan_rust_gate_failed_records=1>0",
            ]
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_toolsmith_plan_history_gate_repair_tasks=4" })
        );
    }

    #[test]
    fn toolsmith_plan_history_recorder_records_and_gates_ready_plan() {
        let plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));

        let record = ToolsmithPlanSummaryHistoryRecorder::new().record_plan_with_health_gate(
            ToolsmithPlanSummaryHistory::new(),
            &plan,
            ToolsmithPlanHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.can_promote_ready_proposals());
        assert!(record.gate_decision.is_promotable());
        assert_eq!(
            record.health_record.health.status,
            ToolsmithPlanHealthStatus::Stable
        );
        assert!(
            record.telemetry.iter().any(|line| {
                line == "agent_toolsmith_plan_history_gate_record_promote_ready=true"
            })
        );
    }

    #[test]
    fn toolsmith_plan_history_recorder_records_and_gates_repair_first() {
        let dirty = ToolsmithPlan::new()
            .with_proposal(ToolProposal::new(
                "trace-script",
                ToolIntent::TraceAnalysis,
                "python",
                "tools/trace.py",
                ToolBuildStatus::Ready,
            ))
            .with_rejected_request("shell tool outside rust crate")
            .summary();
        let clean_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));

        let record = ToolsmithPlanSummaryHistoryRecorder::new().record_plan_with_health_gate(
            ToolsmithPlanSummaryHistory::from_summaries(vec![dirty]),
            &clean_plan,
            ToolsmithPlanHealthPolicy::default(),
        );

        assert_eq!(record.records(), 2);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(!record.can_promote_ready_proposals());
        assert_eq!(
            record.health_record.health.status,
            ToolsmithPlanHealthStatus::Repair
        );
        assert_eq!(record.gate_decision.repair_tasks.len(), 4);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_toolsmith_plan_history_gate_record_requires_repair_first=true"
        }));
    }

    #[test]
    fn process_reward_report_history_watches_empty() {
        let health = ProcessRewardReportSummaryHistory::new()
            .health(ProcessRewardReportHealthPolicy::default());

        assert_eq!(health.status, ProcessRewardReportHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["process_reward_report_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| line == "agent_process_reward_report_dashboard_records=0")
        );
    }

    #[test]
    fn process_reward_report_history_marks_reinforce_stable() {
        let report = ProcessRewardReport {
            total: 0.82,
            components: ProcessRewardComponents {
                coordination: 0.8,
                reflection: 0.8,
                validation: 0.8,
                toolsmith: 0.8,
                recursion: 0.8,
                admission: 0.8,
            },
            action: RewardAction::Reinforce,
            notes: vec!["total:0.820:reinforce".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "agent_coordination",
                "promote_closed_loop_pattern",
                "clean reward",
                0.82,
            )],
        };

        let record = ProcessRewardReportSummaryHistoryRecorder::new().record_report_with_health(
            ProcessRewardReportSummaryHistory::new(),
            &report,
            ProcessRewardReportHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert_eq!(record.appended_summary.action, RewardAction::Reinforce);
        assert_eq!(record.dashboard.reinforce_records, 1);
        assert_eq!(record.dashboard.average_total, 0.82);
        assert_eq!(
            record.health.status,
            ProcessRewardReportHealthStatus::Stable
        );
        assert!(record.health.is_stable());
        assert!(record.health.allows_service_advance());
        assert!(!record.health.requires_repair_first());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "agent_process_reward_report_history_record_status=stable")
        );
    }

    #[test]
    fn process_reward_report_history_repairs_penalties_and_missing_signals() {
        let clean = ProcessRewardReportSummary {
            total: 0.76,
            action: RewardAction::Reinforce,
            signal_count: 1,
            note_count: 1,
            low_component_count: 0,
            telemetry: Vec::new(),
        };
        let dirty = ProcessRewardReportSummary {
            total: 0.20,
            action: RewardAction::Penalize,
            signal_count: 0,
            note_count: 2,
            low_component_count: 3,
            telemetry: Vec::new(),
        };
        let history = ProcessRewardReportSummaryHistory::from_summaries(vec![clean]);

        let record = ProcessRewardReportSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            dirty,
            ProcessRewardReportHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.penalize_records, 1);
        assert_eq!(record.dashboard.low_score_records, 1);
        assert_eq!(record.dashboard.missing_signal_records, 1);
        assert_eq!(record.dashboard.average_total, 0.48);
        assert_eq!(
            record.health.status,
            ProcessRewardReportHealthStatus::Repair
        );
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "process_reward_report_penalize_records=1>0".to_owned(),
                "process_reward_report_low_score_records=1>0".to_owned(),
                "process_reward_report_missing_signal_records=1>0".to_owned(),
            ]
        );
    }

    #[test]
    fn process_reward_report_history_gate_promotes_stable_reinforce_report() {
        let report = ProcessRewardReport {
            total: 0.82,
            components: ProcessRewardComponents {
                coordination: 0.8,
                reflection: 0.8,
                validation: 0.8,
                toolsmith: 0.8,
                recursion: 0.8,
                admission: 0.8,
            },
            action: RewardAction::Reinforce,
            notes: vec!["total:0.820:reinforce".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "agent_coordination",
                "promote_closed_loop_pattern",
                "clean reward",
                0.82,
            )],
        };
        let history_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health(
                ProcessRewardReportSummaryHistory::new(),
                &report,
                ProcessRewardReportHealthPolicy::default(),
            );

        let gate = ProcessRewardReportHistoryGate::new().gate(&report, &history_record);

        assert!(gate.can_promote_evolution_signals);
        assert!(gate.can_reinforce_process);
        assert!(gate.is_promotable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert!(gate.reasons.is_empty());
        assert_eq!(
            gate.reward_health.status,
            ProcessRewardReportHealthStatus::Stable
        );
        assert!(gate.telemetry.iter().any(|line| {
            line == "agent_process_reward_report_history_gate_promote_signals=true"
        }));
    }

    #[test]
    fn reflection_reward_admission_promotes_only_after_closed_reflection_and_stable_reward() {
        let loop_state = closed_reflection_loop();
        let report = reinforce_reward_report();
        let reflection_record =
            reflection_gate_record(ReflectionLoopSummaryHistory::new(), &loop_state);
        let reward_record = reward_gate_record(ProcessRewardReportSummaryHistory::new(), &report);

        let admission = ReflectionRewardAdmissionGate::new().gate(reflection_record, reward_record);
        let summary = admission.summary();

        assert!(admission.is_admitted());
        assert!(!admission.can_continue_reflection);
        assert!(admission.can_promote_memory_note);
        assert!(admission.can_promote_evolution_signals);
        assert!(admission.can_reinforce_process);
        assert!(!admission.requires_repair_first);
        assert!(admission.repair_tasks.is_empty());
        assert!(admission.blocked_reasons.is_empty());
        assert_eq!(
            summary.reflection_health_status,
            ReflectionLoopHealthStatus::Stable
        );
        assert_eq!(
            summary.reward_health_status,
            ProcessRewardReportHealthStatus::Stable
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_reflection_reward_admission_summary_reinforce_process=true"
        }));
    }

    #[test]
    fn reflection_reward_admission_blocks_reward_when_memory_note_is_not_promotable() {
        let loop_state = draft_only_reflection_loop();
        let report = reinforce_reward_report();
        let reflection_record =
            reflection_gate_record(ReflectionLoopSummaryHistory::new(), &loop_state);
        let reward_record = reward_gate_record(ProcessRewardReportSummaryHistory::new(), &report);

        let admission = ReflectionRewardAdmissionGate::new().gate(reflection_record, reward_record);

        assert!(!admission.is_admitted());
        assert!(admission.can_continue_reflection);
        assert!(!admission.can_promote_memory_note);
        assert!(!admission.can_promote_evolution_signals);
        assert!(!admission.can_reinforce_process);
        assert!(!admission.requires_repair_first);
        assert!(admission.repair_tasks.is_empty());
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason == "reflection:reflection_incomplete_next_stage=critique")
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .all(|reason| reason.starts_with("reflection:"))
        );
        assert_eq!(
            admission
                .reflection_record
                .gate_decision
                .reflection_health
                .status,
            ReflectionLoopHealthStatus::Watch
        );
        assert_eq!(
            admission.reward_record.gate_decision.reward_health.status,
            ProcessRewardReportHealthStatus::Stable
        );
    }

    #[test]
    fn reflection_reward_admission_repairs_reflection_before_reward_promotion() {
        let stalled = stalled_reflection_summary();
        let loop_state = closed_reflection_loop();
        let report = reinforce_reward_report();
        let reflection_record = reflection_gate_record(
            ReflectionLoopSummaryHistory::from_summaries(vec![stalled.clone(), stalled]),
            &loop_state,
        );
        let reward_record = reward_gate_record(
            ProcessRewardReportSummaryHistory::from_summaries(vec![dirty_reward_summary()]),
            &report,
        );

        let admission = ReflectionRewardAdmissionGate::new().gate(reflection_record, reward_record);

        assert!(!admission.is_admitted());
        assert!(!admission.can_continue_reflection);
        assert!(!admission.can_promote_memory_note);
        assert!(!admission.can_promote_evolution_signals);
        assert!(!admission.can_reinforce_process);
        assert!(admission.requires_repair_first);
        assert_eq!(
            admission.repair_task_ids(),
            vec![
                "reflection-loop-repair-0".to_owned(),
                "reflection-loop-repair-1".to_owned(),
                "reflection-loop-repair-2".to_owned(),
                "reflection-loop-repair-3".to_owned(),
                "process-reward-report-repair-0".to_owned(),
                "process-reward-report-repair-1".to_owned(),
                "process-reward-report-repair-2".to_owned(),
            ]
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("reflection:reflection_loop_history:"))
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("process_reward:process_reward_report_history:"))
        );
        assert!(admission.telemetry.iter().any(|line| {
            line == "agent_reflection_reward_admission_requires_repair_first=true"
        }));
    }

    #[test]
    fn process_reward_report_history_gate_repairs_dirty_history_before_promotion() {
        let dirty = ProcessRewardReportSummary {
            total: 0.20,
            action: RewardAction::Penalize,
            signal_count: 0,
            note_count: 2,
            low_component_count: 3,
            telemetry: Vec::new(),
        };
        let clean_report = ProcessRewardReport {
            total: 0.82,
            components: ProcessRewardComponents {
                coordination: 0.8,
                reflection: 0.8,
                validation: 0.8,
                toolsmith: 0.8,
                recursion: 0.8,
                admission: 0.8,
            },
            action: RewardAction::Reinforce,
            notes: vec!["total:0.820:reinforce".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "agent_coordination",
                "promote_closed_loop_pattern",
                "clean reward",
                0.82,
            )],
        };
        let history_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health(
                ProcessRewardReportSummaryHistory::from_summaries(vec![dirty]),
                &clean_report,
                ProcessRewardReportHealthPolicy::default(),
            );

        let gate = ProcessRewardReportHistoryGate::new().gate(&clean_report, &history_record);

        assert!(!gate.can_promote_evolution_signals);
        assert!(!gate.can_reinforce_process);
        assert!(!gate.is_promotable());
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reward_health.status,
            ProcessRewardReportHealthStatus::Repair
        );
        assert_eq!(gate.repair_tasks.len(), 3);
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "process-reward-report-repair-0",
                "process-reward-report-repair-1",
                "process-reward-report-repair-2",
            ]
        );
        assert_eq!(
            gate.reasons,
            vec![
                "process_reward_report_history:process_reward_report_penalize_records=1>0",
                "process_reward_report_history:process_reward_report_low_score_records=1>0",
                "process_reward_report_history:process_reward_report_missing_signal_records=1>0",
            ]
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_process_reward_report_history_gate_repair_tasks=3" })
        );
    }

    #[test]
    fn process_reward_report_history_gate_blocks_current_penalty_report() {
        let report = ProcessRewardReport {
            total: 0.20,
            components: ProcessRewardComponents {
                coordination: 0.2,
                reflection: 0.2,
                validation: 0.2,
                toolsmith: 0.2,
                recursion: 0.2,
                admission: 0.2,
            },
            action: RewardAction::Penalize,
            notes: vec!["total:0.200:penalize".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "agent_coordination",
                "repair_closed_loop_pattern",
                "low reward",
                0.20,
            )],
        };
        let history_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health(
                ProcessRewardReportSummaryHistory::new(),
                &report,
                ProcessRewardReportHealthPolicy::default(),
            );

        let gate = ProcessRewardReportHistoryGate::new().gate(&report, &history_record);

        assert!(!gate.can_promote_evolution_signals);
        assert!(!gate.can_reinforce_process);
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons.first().map(String::as_str),
            Some("process_reward_report_action=penalize")
        );
        assert!(gate.reasons.iter().any(|reason| {
            reason == "process_reward_report_history:process_reward_report_low_score_records=1>0"
        }));
        assert_eq!(gate.repair_tasks.len(), gate.reasons.len());
        assert!(gate.telemetry.iter().any(|line| {
            line == "agent_process_reward_report_history_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn process_reward_report_history_gate_blocks_high_score_without_evolution_signal() {
        let report = ProcessRewardReport {
            total: 0.82,
            components: ProcessRewardComponents {
                coordination: 0.8,
                reflection: 0.8,
                validation: 0.8,
                toolsmith: 0.8,
                recursion: 0.8,
                admission: 0.8,
            },
            action: RewardAction::Hold,
            notes: vec!["total:0.820:hold".to_owned()],
            evolution_signals: Vec::new(),
        };
        let history_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health(
                ProcessRewardReportSummaryHistory::new(),
                &report,
                ProcessRewardReportHealthPolicy::default(),
            );

        let gate = ProcessRewardReportHistoryGate::new().gate(&report, &history_record);

        assert!(!gate.can_promote_evolution_signals);
        assert!(!gate.can_reinforce_process);
        assert!(!gate.is_promotable());
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reward_health.status,
            ProcessRewardReportHealthStatus::Repair
        );
        assert_eq!(
            gate.reasons,
            vec![
                "process_reward_report_missing_evolution_signals",
                "process_reward_report_history:process_reward_report_missing_signal_records=1>0",
            ]
        );
        assert_eq!(gate.repair_tasks.len(), 2);
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_process_reward_report_history_gate_signal_count=0" })
        );
    }

    #[test]
    fn process_reward_report_history_recorder_records_and_gates_reinforce_report() {
        let report = ProcessRewardReport {
            total: 0.82,
            components: ProcessRewardComponents {
                coordination: 0.8,
                reflection: 0.8,
                validation: 0.8,
                toolsmith: 0.8,
                recursion: 0.8,
                admission: 0.8,
            },
            action: RewardAction::Reinforce,
            notes: vec!["total:0.820:reinforce".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "agent_coordination",
                "promote_closed_loop_pattern",
                "clean reward",
                0.82,
            )],
        };

        let record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &report,
                ProcessRewardReportHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.can_promote_evolution_signals());
        assert!(record.gate_decision.can_reinforce_process);
        assert!(record.gate_decision.is_promotable());
        assert_eq!(
            record.health_record.health.status,
            ProcessRewardReportHealthStatus::Stable
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_process_reward_report_history_gate_record_promote_signals=true"
        }));
    }

    #[test]
    fn process_reward_report_history_recorder_records_and_gates_repair_first() {
        let dirty = ProcessRewardReportSummary {
            total: 0.20,
            action: RewardAction::Penalize,
            signal_count: 0,
            note_count: 2,
            low_component_count: 3,
            telemetry: Vec::new(),
        };
        let report = ProcessRewardReport {
            total: 0.82,
            components: ProcessRewardComponents {
                coordination: 0.8,
                reflection: 0.8,
                validation: 0.8,
                toolsmith: 0.8,
                recursion: 0.8,
                admission: 0.8,
            },
            action: RewardAction::Reinforce,
            notes: vec!["total:0.820:reinforce".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "agent_coordination",
                "promote_closed_loop_pattern",
                "clean reward",
                0.82,
            )],
        };

        let record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::from_summaries(vec![dirty]),
                &report,
                ProcessRewardReportHealthPolicy::default(),
            );

        assert_eq!(record.records(), 2);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(!record.can_promote_evolution_signals());
        assert_eq!(
            record.health_record.health.status,
            ProcessRewardReportHealthStatus::Repair
        );
        assert_eq!(record.gate_decision.repair_tasks.len(), 3);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_process_reward_report_history_gate_record_requires_repair_first=true"
        }));
    }

    #[test]
    fn evolution_admission_gate_admits_clean_toolsmith_and_reward_records() {
        let plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));
        let toolsmith_record = ToolsmithPlanSummaryHistoryRecorder::new()
            .record_plan_with_health_gate(
                ToolsmithPlanSummaryHistory::new(),
                &plan,
                ToolsmithPlanHealthPolicy::default(),
            );
        let reward_report = ProcessRewardReport {
            total: 0.82,
            components: ProcessRewardComponents {
                coordination: 0.8,
                reflection: 0.8,
                validation: 0.8,
                toolsmith: 0.8,
                recursion: 0.8,
                admission: 0.8,
            },
            action: RewardAction::Reinforce,
            notes: vec!["total:0.820:reinforce".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "agent_coordination",
                "promote_closed_loop_pattern",
                "clean reward",
                0.82,
            )],
        };
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &reward_report,
                ProcessRewardReportHealthPolicy::default(),
            );

        let admission = EvolutionAdmissionGate::new().gate(toolsmith_record, reward_record);

        assert_eq!(admission.records(), 1);
        assert!(admission.allows_service_advance());
        assert!(!admission.requires_repair_first());
        assert!(admission.can_promote_ready_proposals());
        assert!(admission.can_promote_evolution_signals());
        assert!(admission.can_promote_adaptive_state());
        assert!(admission.decision.can_reinforce_process);
        assert!(admission.decision.is_admitted());
        assert!(admission.decision.repair_tasks.is_empty());
        assert!(admission.decision.blocked_reasons.is_empty());
        assert_eq!(
            admission.decision.toolsmith_health_status,
            ToolsmithPlanHealthStatus::Stable
        );
        assert_eq!(
            admission.decision.reward_health_status,
            ProcessRewardReportHealthStatus::Stable
        );
        assert!(
            admission
                .telemetry
                .iter()
                .any(|line| { line == "agent_evolution_admission_record_adaptive_state=true" })
        );
    }

    #[test]
    fn evolution_admission_gate_blocks_and_merges_repair_pressure() {
        let dirty_toolsmith = ToolsmithPlan::new()
            .with_proposal(ToolProposal::new(
                "trace-script",
                ToolIntent::TraceAnalysis,
                "python",
                "tools/trace.py",
                ToolBuildStatus::Ready,
            ))
            .with_rejected_request("shell tool outside rust crate")
            .summary();
        let clean_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));
        let toolsmith_record = ToolsmithPlanSummaryHistoryRecorder::new()
            .record_plan_with_health_gate(
                ToolsmithPlanSummaryHistory::from_summaries(vec![dirty_toolsmith]),
                &clean_plan,
                ToolsmithPlanHealthPolicy::default(),
            );
        let dirty_reward = ProcessRewardReportSummary {
            total: 0.20,
            action: RewardAction::Penalize,
            signal_count: 0,
            note_count: 2,
            low_component_count: 3,
            telemetry: Vec::new(),
        };
        let clean_report = ProcessRewardReport {
            total: 0.82,
            components: ProcessRewardComponents {
                coordination: 0.8,
                reflection: 0.8,
                validation: 0.8,
                toolsmith: 0.8,
                recursion: 0.8,
                admission: 0.8,
            },
            action: RewardAction::Reinforce,
            notes: vec!["total:0.820:reinforce".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "agent_coordination",
                "promote_closed_loop_pattern",
                "clean reward",
                0.82,
            )],
        };
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::from_summaries(vec![dirty_reward]),
                &clean_report,
                ProcessRewardReportHealthPolicy::default(),
            );

        let admission = EvolutionAdmissionGate::new().gate(toolsmith_record, reward_record);

        assert_eq!(admission.records(), 2);
        assert!(!admission.allows_service_advance());
        assert!(admission.requires_repair_first());
        assert!(!admission.can_promote_ready_proposals());
        assert!(!admission.can_promote_evolution_signals());
        assert!(!admission.can_promote_adaptive_state());
        assert!(!admission.decision.can_reinforce_process);
        assert!(!admission.decision.is_admitted());
        assert_eq!(admission.decision.repair_tasks.len(), 7);
        assert_eq!(admission.decision.blocked_reasons.len(), 7);
        assert_eq!(
            admission
                .decision
                .repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "toolsmith-plan-repair-0",
                "toolsmith-plan-repair-1",
                "toolsmith-plan-repair-2",
                "toolsmith-plan-repair-3",
                "process-reward-report-repair-0",
                "process-reward-report-repair-1",
                "process-reward-report-repair-2",
            ]
        );
        assert!(admission.decision.blocked_reasons.iter().any(|reason| {
            reason == "toolsmith:toolsmith_plan_history:toolsmith_plan_rejected=1>0"
        }));
        assert!(admission.decision.blocked_reasons.iter().any(|reason| {
            reason
                == "process_reward:process_reward_report_history:process_reward_report_missing_signal_records=1>0"
        }));
        assert!(
            admission.telemetry.iter().any(|line| {
                line == "agent_evolution_admission_record_requires_repair_first=true"
            })
        );
    }

    #[test]
    fn evolution_admission_history_records_stable_admission_boundary() {
        let admission = clean_evolution_admission_record();

        let record = EvolutionAdmissionSummaryHistoryRecorder::new().record_admission_with_health(
            EvolutionAdmissionSummaryHistory::new(),
            &admission,
            EvolutionAdmissionHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Stable);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.appended_summary.admitted);
        assert!(record.appended_summary.can_promote_adaptive_state);
        assert_eq!(record.dashboard.admitted_records, 1);
        assert_eq!(record.dashboard.adaptive_state_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_evolution_admission_history_health=stable" })
        );
    }

    #[test]
    fn evolution_admission_history_repairs_repair_first_pressure() {
        let admission = repair_evolution_admission_record();

        let record = EvolutionAdmissionSummaryHistoryRecorder::new().record_admission_with_health(
            EvolutionAdmissionSummaryHistory::new(),
            &admission,
            EvolutionAdmissionHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(!record.appended_summary.admitted);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert!(record.dashboard.repair_task_count > 0);
        assert!(record.dashboard.blocked_reason_count > 0);
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "evolution_admission_repair_first_records=1>0" })
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_evolution_admission_history_health=repair" })
        );
    }

    #[test]
    fn evolution_admission_history_gate_admits_stable_admission() {
        let admission = clean_evolution_admission_record();

        let record = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health_gate(
                EvolutionAdmissionSummaryHistory::new(),
                &admission,
                EvolutionAdmissionHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(
            record.gate_decision.admission_health.status,
            EvolutionAdmissionHealthStatus::Stable
        );
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.can_promote_ready_proposals());
        assert!(record.can_promote_evolution_signals());
        assert!(record.can_promote_adaptive_state());
        assert!(record.gate_decision.can_reinforce_process);
        assert!(record.gate_decision.is_admitted());
        assert!(record.gate_decision.repair_tasks.is_empty());
        assert!(record.gate_decision.blocked_reasons.is_empty());
        assert!(
            record.telemetry.iter().any(|line| {
                line == "agent_evolution_admission_history_gate_record_admitted=true"
            })
        );
    }

    #[test]
    fn evolution_admission_history_gate_repairs_dirty_history_before_promotion() {
        let dirty_admission = repair_evolution_admission_record();
        let clean_admission = clean_evolution_admission_record();
        let dirty_history = EvolutionAdmissionSummaryHistory::from_summaries(vec![
            EvolutionAdmissionSummary::from_record(&dirty_admission),
        ]);

        let record = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health_gate(
                dirty_history,
                &clean_admission,
                EvolutionAdmissionHealthPolicy::default(),
            );

        assert_eq!(record.records(), 2);
        assert_eq!(
            record.gate_decision.admission_health.status,
            EvolutionAdmissionHealthStatus::Repair
        );
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(!record.can_promote_ready_proposals());
        assert!(!record.can_promote_evolution_signals());
        assert!(!record.can_promote_adaptive_state());
        assert!(!record.gate_decision.can_reinforce_process);
        assert!(!record.gate_decision.is_admitted());
        assert!(record.gate_decision.blocked_reasons.iter().any(|reason| {
            reason == "evolution_admission_history:evolution_admission_repair_first_records=1>0"
        }));
        assert!(!record.gate_decision.repair_tasks.is_empty());
        assert!(record.gate_decision.repair_tasks.iter().all(|task| {
            task.id.starts_with("evolution-admission-repair-")
                && task.lane == "evolution-admission"
                && task.priority == 8
        }));
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_history_gate_record_requires_repair_first=true"
        }));
    }

    #[test]
    fn evolution_admission_handoff_preserves_stable_business_queue() {
        let admission = clean_evolution_admission_record();
        let gate_record = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health_gate(
                EvolutionAdmissionSummaryHistory::new(),
                &admission,
                EvolutionAdmissionHealthPolicy::default(),
            );
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue self-evolution loop",
            AgentBudget::new(8, 1, 1),
        )]);

        let handoff = EvolutionAdmissionHandoff::from_gate_record(gate_record, queue);
        let summary = handoff.summary();

        assert!(handoff.is_admitted());
        assert!(!handoff.requires_repair_first);
        assert!(handoff.repair_tasks.is_empty());
        assert_eq!(handoff.next_queue().task_ids(), vec!["business-task"]);
        assert!(handoff.can_promote_ready_proposals);
        assert!(handoff.can_promote_evolution_signals);
        assert!(handoff.can_reinforce_process);
        assert!(handoff.can_promote_adaptive_state);
        assert_eq!(summary.next_queue_tasks, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(
            handoff.telemetry.iter().any(|line| {
                line == "agent_evolution_admission_handoff_effective_admitted=true"
            })
        );
    }

    #[test]
    fn evolution_admission_handoff_merges_repair_tasks_into_next_queue() {
        let dirty_admission = repair_evolution_admission_record();
        let clean_admission = clean_evolution_admission_record();
        let dirty_history = EvolutionAdmissionSummaryHistory::from_summaries(vec![
            EvolutionAdmissionSummary::from_record(&dirty_admission),
        ]);
        let gate_record = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health_gate(
                dirty_history,
                &clean_admission,
                EvolutionAdmissionHealthPolicy::default(),
            );
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue self-evolution loop",
            AgentBudget::new(8, 1, 1),
        )]);

        let handoff = EvolutionAdmissionHandoff::from_gate_record(gate_record, queue);
        let summary = handoff.summary();

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_promote_ready_proposals);
        assert!(!handoff.can_promote_evolution_signals);
        assert!(!handoff.can_reinforce_process);
        assert!(!handoff.can_promote_adaptive_state);
        assert!(!handoff.repair_tasks.is_empty());
        assert_eq!(summary.repair_tasks, handoff.repair_tasks.len());
        assert_eq!(summary.next_queue_tasks, handoff.repair_tasks.len() + 1);
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(
            summary
                .repair_task_ids
                .iter()
                .all(|task_id| task_id.starts_with("evolution-admission-repair-"))
        );
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id.starts_with("evolution-admission-repair-"))
        );
        assert!(handoff.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_requires_repair_first=true"
        }));
    }

    #[test]
    fn evolution_admission_handoff_history_records_stable_handoff() {
        let admission = clean_evolution_admission_record();
        let gate_record = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health_gate(
                EvolutionAdmissionSummaryHistory::new(),
                &admission,
                EvolutionAdmissionHealthPolicy::default(),
            );
        let handoff = EvolutionAdmissionHandoff::from_gate_record(
            gate_record,
            AgentTaskQueue::from_tasks(vec![AgentTask::new(
                "business-task",
                AgentRole::Planner,
                "continue self-evolution loop",
                AgentBudget::new(8, 1, 1),
            )]),
        );

        let record = EvolutionAdmissionHandoffSummaryHistoryRecorder::new()
            .record_handoff_with_health(
                EvolutionAdmissionHandoffSummaryHistory::new(),
                &handoff,
                EvolutionAdmissionHandoffHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Stable);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.effective_admitted_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert_eq!(record.dashboard.next_queue_task_count, 1);
        assert!(
            record
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_evolution_admission_handoff_dashboard=true" })
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_evolution_admission_handoff_history_record=true" })
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_evolution_admission_handoff_history_health=stable" })
        );
    }

    #[test]
    fn evolution_admission_handoff_history_repairs_repair_first_pressure() {
        let dirty_admission = repair_evolution_admission_record();
        let clean_admission = clean_evolution_admission_record();
        let dirty_history = EvolutionAdmissionSummaryHistory::from_summaries(vec![
            EvolutionAdmissionSummary::from_record(&dirty_admission),
        ]);
        let gate_record = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health_gate(
                dirty_history,
                &clean_admission,
                EvolutionAdmissionHealthPolicy::default(),
            );
        let handoff = EvolutionAdmissionHandoff::from_gate_record(
            gate_record,
            AgentTaskQueue::from_tasks(vec![AgentTask::new(
                "business-task",
                AgentRole::Planner,
                "continue self-evolution loop",
                AgentBudget::new(8, 1, 1),
            )]),
        );

        let record = EvolutionAdmissionHandoffSummaryHistoryRecorder::new()
            .record_handoff_with_health(
                EvolutionAdmissionHandoffSummaryHistory::new(),
                &handoff,
                EvolutionAdmissionHandoffHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(record.dashboard.effective_admitted_records, 0);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert!(record.dashboard.repair_task_count > 0);
        assert!(record.dashboard.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_dashboard_repair_first_records=1"
        }));
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "evolution_admission_handoff_repair_first_records=1>0" })
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_evolution_admission_handoff_history_health=repair" })
        );
    }

    #[test]
    fn evolution_admission_handoff_history_watches_queue_pressure_policy() {
        let admission = clean_evolution_admission_record();
        let gate_record = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health_gate(
                EvolutionAdmissionSummaryHistory::new(),
                &admission,
                EvolutionAdmissionHealthPolicy::default(),
            );
        let handoff = EvolutionAdmissionHandoff::from_gate_record(
            gate_record,
            AgentTaskQueue::from_tasks(vec![AgentTask::new(
                "business-task",
                AgentRole::Planner,
                "continue self-evolution loop",
                AgentBudget::new(8, 1, 1),
            )]),
        );
        let policy = EvolutionAdmissionHandoffHealthPolicy {
            maximum_next_queue_tasks: 0,
            ..EvolutionAdmissionHandoffHealthPolicy::default()
        };

        let record = EvolutionAdmissionHandoffSummaryHistoryRecorder::new()
            .record_handoff_with_health(
                EvolutionAdmissionHandoffSummaryHistory::new(),
                &handoff,
                policy,
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Watch);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.next_queue_task_count, 1);
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "evolution_admission_handoff_next_queue_tasks=1>0" })
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_evolution_admission_handoff_history_health=watch" })
        );
    }

    #[test]
    fn evolution_admission_handoff_trend_gate_admits_stable_queue() {
        let handoff = stable_evolution_admission_handoff();
        let history_record = EvolutionAdmissionHandoffSummaryHistoryRecorder::new()
            .record_handoff_with_health(
                EvolutionAdmissionHandoffSummaryHistory::new(),
                &handoff,
                EvolutionAdmissionHandoffHealthPolicy::default(),
            );

        let decision = EvolutionAdmissionHandoffTrendGate::new().gate(&handoff, &history_record);

        assert!(decision.is_admitted());
        assert_eq!(
            decision.handoff_health.status,
            EvolutionAdmissionHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue().task_ids(), vec!["business-task"]);
        assert!(decision.can_promote_ready_proposals);
        assert!(decision.can_promote_evolution_signals);
        assert!(decision.can_reinforce_process);
        assert!(decision.can_promote_adaptive_state);
        assert!(
            decision.telemetry.iter().any(|line| {
                line == "agent_evolution_admission_handoff_trend_gate_health=stable"
            })
        );
    }

    #[test]
    fn evolution_admission_handoff_trend_gate_observes_watch_without_promotion() {
        let handoff = stable_evolution_admission_handoff();
        let history_policy = EvolutionAdmissionHandoffHealthPolicy {
            maximum_next_queue_tasks: 0,
            ..EvolutionAdmissionHandoffHealthPolicy::default()
        };
        let history_record = EvolutionAdmissionHandoffSummaryHistoryRecorder::new()
            .record_handoff_with_health(
                EvolutionAdmissionHandoffSummaryHistory::new(),
                &handoff,
                history_policy,
            );

        let decision = EvolutionAdmissionHandoffTrendGate::new().gate(&handoff, &history_record);

        assert!(decision.is_admitted());
        assert_eq!(
            decision.handoff_health.status,
            EvolutionAdmissionHealthStatus::Watch
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue().task_ids(), vec!["business-task"]);
        assert!(!decision.can_promote_ready_proposals);
        assert!(!decision.can_promote_evolution_signals);
        assert!(!decision.can_reinforce_process);
        assert!(!decision.can_promote_adaptive_state);
        assert!(decision.blocked_reasons.iter().any(|reason| {
            reason == "handoff_history:evolution_admission_handoff_next_queue_tasks=1>0"
        }));
        assert!(
            decision.telemetry.iter().any(|line| {
                line == "agent_evolution_admission_handoff_trend_gate_health=watch"
            })
        );
    }

    #[test]
    fn evolution_admission_handoff_trend_gate_repairs_dirty_trend_before_scheduler() {
        let handoff = repair_evolution_admission_handoff();
        let history_record = EvolutionAdmissionHandoffSummaryHistoryRecorder::new()
            .record_handoff_with_health(
                EvolutionAdmissionHandoffSummaryHistory::new(),
                &handoff,
                EvolutionAdmissionHandoffHealthPolicy::default(),
            );

        let decision = EvolutionAdmissionHandoffTrendGate::new().gate(&handoff, &history_record);
        let next_queue_task_ids = decision.next_queue().task_ids();

        assert!(!decision.is_admitted());
        assert_eq!(
            decision.handoff_health.status,
            EvolutionAdmissionHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert!(!decision.effective_admitted);
        assert!(!decision.can_promote_ready_proposals);
        assert!(!decision.can_promote_evolution_signals);
        assert!(!decision.can_reinforce_process);
        assert!(!decision.can_promote_adaptive_state);
        assert!(!decision.repair_tasks.is_empty());
        assert!(decision.repair_tasks.iter().all(|task| {
            task.id
                .starts_with("evolution-admission-handoff-trend-repair-")
        }));
        assert_eq!(
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.clone())
                .collect::<Vec<_>>(),
            (0..decision.repair_tasks.len())
                .map(|index| format!("evolution-admission-handoff-trend-repair-{index}"))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            next_queue_task_ids.first().map(String::as_str),
            Some("business-task")
        );
        assert!(
            next_queue_task_ids
                .iter()
                .any(|task_id| task_id.starts_with("evolution-admission-repair-"))
        );
        let queued_trend_repair_ids = next_queue_task_ids
            .iter()
            .filter(|task_id| task_id.starts_with("evolution-admission-handoff-trend-repair-"))
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            queued_trend_repair_ids,
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.clone())
                .collect::<Vec<_>>()
        );
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_monitor_records_and_admits_stable_handoff() {
        let handoff = stable_evolution_admission_handoff();

        let record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            EvolutionAdmissionHandoffHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert!(record.is_admitted());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(
            record.history_record.health.status,
            EvolutionAdmissionHealthStatus::Stable
        );
        assert_eq!(record.next_queue().task_ids(), vec!["business-task"]);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_monitor_health=stable"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_monitor_repairs_dirty_handoff() {
        let handoff = repair_evolution_admission_handoff();

        let record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            EvolutionAdmissionHandoffHealthPolicy::default(),
        );
        let next_queue_task_ids = record.next_queue().task_ids();

        assert_eq!(record.records(), 1);
        assert!(!record.is_admitted());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.history_record.health.status,
            EvolutionAdmissionHealthStatus::Repair
        );
        assert!(
            next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(
            next_queue_task_ids.iter().any(|task_id| {
                task_id.starts_with("evolution-admission-handoff-trend-repair-")
            })
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_monitor_requires_repair_first=true"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_packages_stable_scheduler_state() {
        let handoff = stable_evolution_admission_handoff();
        let policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            policy,
        );

        let continuation =
            EvolutionAdmissionHandoffTrendContinuationPlanner::new().plan(&record, policy);

        assert!(continuation.is_admitted());
        assert_eq!(
            continuation.handoff_health_status,
            EvolutionAdmissionHealthStatus::Stable
        );
        assert_eq!(continuation.next_queue.task_ids(), vec!["business-task"]);
        assert_eq!(continuation.handoff_history.len(), 1);
        assert_eq!(continuation.handoff_policy, policy);
        assert!(continuation.can_promote_ready_proposals);
        assert!(continuation.can_promote_evolution_signals);
        assert!(continuation.can_reinforce_process);
        assert!(continuation.can_promote_adaptive_state);
        assert!(!continuation.requires_repair_first);
        assert!(continuation.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_health=stable"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_observes_watch_without_promotion() {
        let handoff = stable_evolution_admission_handoff();
        let policy = EvolutionAdmissionHandoffHealthPolicy {
            maximum_next_queue_tasks: 0,
            ..EvolutionAdmissionHandoffHealthPolicy::default()
        };
        let record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            policy,
        );

        let continuation = record.continuation(policy);

        assert!(continuation.is_admitted());
        assert_eq!(
            continuation.handoff_health_status,
            EvolutionAdmissionHealthStatus::Watch
        );
        assert_eq!(continuation.next_queue.task_ids(), vec!["business-task"]);
        assert_eq!(continuation.handoff_history.len(), 1);
        assert!(!continuation.can_promote_ready_proposals);
        assert!(!continuation.can_promote_evolution_signals);
        assert!(!continuation.can_reinforce_process);
        assert!(!continuation.can_promote_adaptive_state);
        assert!(!continuation.requires_repair_first);
        assert!(continuation.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_promote_ready=false"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_preserves_repair_queue() {
        let handoff = repair_evolution_admission_handoff();
        let policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            policy,
        );

        let continuation = record.continuation(policy);
        let next_queue_task_ids = continuation.next_queue.task_ids();

        assert!(!continuation.is_admitted());
        assert_eq!(
            continuation.handoff_health_status,
            EvolutionAdmissionHealthStatus::Repair
        );
        assert_eq!(continuation.handoff_history.len(), 1);
        assert!(continuation.requires_repair_first);
        assert!(!continuation.can_promote_ready_proposals);
        assert!(!continuation.can_promote_evolution_signals);
        assert!(!continuation.can_reinforce_process);
        assert!(!continuation.can_promote_adaptive_state);
        assert!(
            next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(
            next_queue_task_ids.iter().any(|task_id| {
                task_id.starts_with("evolution-admission-handoff-trend-repair-")
            })
        );
        assert!(continuation.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_requires_repair_first=true"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_summary_compacts_stable_state() {
        let handoff = stable_evolution_admission_handoff();
        let policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            policy,
        );
        let continuation = record.continuation(policy);

        let summary = continuation.summary();

        assert_eq!(
            summary.handoff_health_status,
            EvolutionAdmissionHealthStatus::Stable
        );
        assert!(summary.effective_admitted);
        assert!(summary.can_promote_ready_proposals);
        assert!(summary.can_promote_evolution_signals);
        assert!(summary.can_reinforce_process);
        assert!(summary.can_promote_adaptive_state);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.next_queue_tasks, 1);
        assert_eq!(summary.handoff_history_records, 1);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_summary_health=stable"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_summary_compacts_repair_queue() {
        let handoff = repair_evolution_admission_handoff();
        let policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            policy,
        );
        let continuation = record.continuation(policy);

        let summary = continuation.summary();

        assert_eq!(
            summary.handoff_health_status,
            EvolutionAdmissionHealthStatus::Repair
        );
        assert!(!summary.effective_admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.handoff_history_records, 1);
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(
            summary.next_queue_task_ids.iter().any(|task_id| {
                task_id.starts_with("evolution-admission-handoff-trend-repair-")
            })
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_summary_requires_repair_first=true"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_watches_empty() {
        let health = EvolutionAdmissionHandoffTrendContinuationSummaryHistory::new()
            .health(EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default());

        assert_eq!(health.status, EvolutionAdmissionHealthStatus::Watch);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.reasons.iter().any(|reason| {
            reason == "evolution_admission_handoff_trend_continuation_history_empty"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_records_stable_state() {
        let handoff = stable_evolution_admission_handoff();
        let handoff_policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            handoff_policy,
        );
        let continuation = monitor_record.continuation(handoff_policy);

        let record = EvolutionAdmissionHandoffTrendContinuationHistoryRecorder::new()
            .record_continuation_with_health(
                EvolutionAdmissionHandoffTrendContinuationSummaryHistory::new(),
                &continuation,
                EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Stable);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.effective_admitted_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert_eq!(record.dashboard.next_queue_task_count, 1);
        assert_eq!(record.dashboard.handoff_history_record_count, 1);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_history_health=stable"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_watches_queue_pressure() {
        let handoff = stable_evolution_admission_handoff();
        let handoff_policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            handoff_policy,
        );
        let continuation = monitor_record.continuation(handoff_policy);
        let policy = EvolutionAdmissionHandoffTrendContinuationHealthPolicy {
            maximum_next_queue_tasks: 0,
            ..EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default()
        };

        let record = EvolutionAdmissionHandoffTrendContinuationHistoryRecorder::new()
            .record_continuation_with_health(
                EvolutionAdmissionHandoffTrendContinuationSummaryHistory::new(),
                &continuation,
                policy,
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Watch);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.health.reasons.iter().any(|reason| {
            reason == "evolution_admission_handoff_trend_continuation_next_queue_tasks=1>0"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_repairs_dirty_queue() {
        let handoff = repair_evolution_admission_handoff();
        let handoff_policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            handoff_policy,
        );
        let continuation = monitor_record.continuation(handoff_policy);

        let record = EvolutionAdmissionHandoffTrendContinuationHistoryRecorder::new()
            .record_continuation_with_health(
                EvolutionAdmissionHandoffTrendContinuationSummaryHistory::new(),
                &continuation,
                EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(record.dashboard.effective_admitted_records, 0);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.handoff_repair_records, 1);
        assert!(record.health.reasons.iter().any(|reason| {
            reason == "evolution_admission_handoff_trend_continuation_repair_first_records=1>0"
        }));
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_history_health=repair"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_preserves_stable_queue() {
        let handoff = stable_evolution_admission_handoff();
        let handoff_policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            handoff_policy,
        );
        let continuation = monitor_record.continuation(handoff_policy);
        let gate_record = EvolutionAdmissionHandoffTrendContinuationHistoryRecorder::new()
            .record_continuation_with_health_gate(
                EvolutionAdmissionHandoffTrendContinuationSummaryHistory::new(),
                &continuation,
                EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default(),
            );

        assert_eq!(gate_record.records(), 1);
        assert!(gate_record.is_admitted());
        assert!(gate_record.allows_service_advance());
        assert!(!gate_record.requires_repair_first());
        assert_eq!(
            gate_record.gate_decision.continuation_health.status,
            EvolutionAdmissionHealthStatus::Stable
        );
        assert_eq!(gate_record.next_queue().task_ids(), vec!["business-task"]);
        assert!(gate_record.gate_decision.repair_tasks.is_empty());
        assert!(gate_record.gate_decision.can_promote_ready_proposals);
        assert!(gate_record.gate_decision.can_promote_evolution_signals);
        assert!(gate_record.gate_decision.can_reinforce_process);
        assert!(gate_record.gate_decision.can_promote_adaptive_state);
        let summary = gate_record.gate_decision.summary();
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_effective_admitted=true"
        }));
        assert!(gate_record.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_history_gate_record_admitted=true"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_observes_watch_queue() {
        let handoff = stable_evolution_admission_handoff();
        let handoff_policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            handoff_policy,
        );
        let continuation = monitor_record.continuation(handoff_policy);
        let policy = EvolutionAdmissionHandoffTrendContinuationHealthPolicy {
            maximum_next_queue_tasks: 0,
            ..EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default()
        };
        let history_record = EvolutionAdmissionHandoffTrendContinuationHistoryRecorder::new()
            .record_continuation_with_health(
                EvolutionAdmissionHandoffTrendContinuationSummaryHistory::new(),
                &continuation,
                policy,
            );

        let decision = EvolutionAdmissionHandoffTrendContinuationHistoryGate::new()
            .gate(&continuation, &history_record);

        assert!(decision.is_admitted());
        assert_eq!(
            decision.continuation_health.status,
            EvolutionAdmissionHealthStatus::Watch
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue().task_ids(), vec!["business-task"]);
        assert!(!decision.can_promote_ready_proposals);
        assert!(!decision.can_promote_evolution_signals);
        assert!(!decision.can_reinforce_process);
        assert!(!decision.can_promote_adaptive_state);
        assert!(decision.blocked_reasons.iter().any(|reason| {
            reason
                == "continuation_history:evolution_admission_handoff_trend_continuation_next_queue_tasks=1>0"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_repairs_dirty_history() {
        let dirty_handoff = repair_evolution_admission_handoff();
        let clean_handoff = stable_evolution_admission_handoff();
        let handoff_policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let dirty_monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &dirty_handoff,
            handoff_policy,
        );
        let dirty_continuation = dirty_monitor_record.continuation(handoff_policy);
        let clean_monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &clean_handoff,
            handoff_policy,
        );
        let clean_continuation = clean_monitor_record.continuation(handoff_policy);
        let dirty_history =
            EvolutionAdmissionHandoffTrendContinuationSummaryHistory::from_summaries(vec![
                dirty_continuation.summary(),
            ]);
        let history_record = EvolutionAdmissionHandoffTrendContinuationHistoryRecorder::new()
            .record_continuation_with_health(
                dirty_history,
                &clean_continuation,
                EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default(),
            );

        let decision = EvolutionAdmissionHandoffTrendContinuationHistoryGate::new()
            .gate(&clean_continuation, &history_record);
        let next_queue_task_ids = decision.next_queue().task_ids();

        assert!(!decision.is_admitted());
        assert_eq!(
            decision.continuation_health.status,
            EvolutionAdmissionHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert!(!decision.effective_admitted);
        assert!(!decision.can_promote_ready_proposals);
        assert!(!decision.can_promote_evolution_signals);
        assert!(!decision.can_reinforce_process);
        assert!(!decision.can_promote_adaptive_state);
        assert!(!decision.repair_tasks.is_empty());
        assert!(decision.repair_tasks.iter().all(|task| {
            task.id
                .starts_with("evolution-admission-handoff-trend-continuation-repair-")
        }));
        assert_eq!(
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.clone())
                .collect::<Vec<_>>(),
            (0..decision.repair_tasks.len())
                .map(|index| {
                    format!("evolution-admission-handoff-trend-continuation-repair-{index}")
                })
                .collect::<Vec<_>>()
        );
        assert!(
            next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(next_queue_task_ids.iter().any(|task_id| {
            task_id.starts_with("evolution-admission-handoff-trend-continuation-repair-")
        }));
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_history_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_summary_compacts_stable_decision()
     {
        let handoff = stable_evolution_admission_handoff();
        let handoff_policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            handoff_policy,
        );
        let continuation = monitor_record.continuation(handoff_policy);
        let gate_record = EvolutionAdmissionHandoffTrendContinuationHistoryRecorder::new()
            .record_continuation_with_health_gate(
                EvolutionAdmissionHandoffTrendContinuationSummaryHistory::new(),
                &continuation,
                EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default(),
            );

        let summary = gate_record.summary();

        assert_eq!(
            summary.continuation_health_status,
            EvolutionAdmissionHealthStatus::Stable
        );
        assert!(summary.effective_admitted);
        assert!(summary.can_promote_ready_proposals);
        assert!(summary.can_promote_evolution_signals);
        assert!(summary.can_reinforce_process);
        assert!(summary.can_promote_adaptive_state);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_tasks, 1);
        assert_eq!(summary.blocked_reasons, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(summary.repair_task_ids.is_empty());
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_health=stable"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_summary_compacts_watch_decision()
    {
        let handoff = stable_evolution_admission_handoff();
        let handoff_policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &handoff,
            handoff_policy,
        );
        let continuation = monitor_record.continuation(handoff_policy);
        let policy = EvolutionAdmissionHandoffTrendContinuationHealthPolicy {
            maximum_next_queue_tasks: 0,
            ..EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default()
        };
        let history_record = EvolutionAdmissionHandoffTrendContinuationHistoryRecorder::new()
            .record_continuation_with_health(
                EvolutionAdmissionHandoffTrendContinuationSummaryHistory::new(),
                &continuation,
                policy,
            );
        let decision = EvolutionAdmissionHandoffTrendContinuationHistoryGate::new()
            .gate(&continuation, &history_record);

        let summary = decision.summary();

        assert_eq!(
            summary.continuation_health_status,
            EvolutionAdmissionHealthStatus::Watch
        );
        assert!(summary.effective_admitted);
        assert!(!summary.can_promote_ready_proposals);
        assert!(!summary.can_promote_evolution_signals);
        assert!(!summary.can_reinforce_process);
        assert!(!summary.can_promote_adaptive_state);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert_eq!(summary.blocked_reasons, 1);
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_summary_compacts_repair_decision()
     {
        let dirty_handoff = repair_evolution_admission_handoff();
        let clean_handoff = stable_evolution_admission_handoff();
        let handoff_policy = EvolutionAdmissionHandoffHealthPolicy::default();
        let dirty_monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &dirty_handoff,
            handoff_policy,
        );
        let dirty_continuation = dirty_monitor_record.continuation(handoff_policy);
        let clean_monitor_record = EvolutionAdmissionHandoffTrendMonitor::new().monitor(
            EvolutionAdmissionHandoffSummaryHistory::new(),
            &clean_handoff,
            handoff_policy,
        );
        let clean_continuation = clean_monitor_record.continuation(handoff_policy);
        let dirty_history =
            EvolutionAdmissionHandoffTrendContinuationSummaryHistory::from_summaries(vec![
                dirty_continuation.summary(),
            ]);
        let history_record = EvolutionAdmissionHandoffTrendContinuationHistoryRecorder::new()
            .record_continuation_with_health(
                dirty_history,
                &clean_continuation,
                EvolutionAdmissionHandoffTrendContinuationHealthPolicy::default(),
            );
        let decision = EvolutionAdmissionHandoffTrendContinuationHistoryGate::new()
            .gate(&clean_continuation, &history_record);

        let summary = decision.summary();

        assert_eq!(
            summary.continuation_health_status,
            EvolutionAdmissionHealthStatus::Repair
        );
        assert!(!summary.effective_admitted);
        assert!(summary.requires_repair_first);
        assert!(!summary.can_promote_ready_proposals);
        assert!(!summary.can_promote_evolution_signals);
        assert!(!summary.can_reinforce_process);
        assert!(!summary.can_promote_adaptive_state);
        assert!(!summary.repair_task_ids.is_empty());
        assert!(summary.repair_task_ids.iter().all(|task_id| {
            task_id.starts_with("evolution-admission-handoff-trend-continuation-repair-")
        }));
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_history_gate_summary_requires_repair_first=true"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_summary_history_watches_empty() {
        let health = EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory::new()
            .health(EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default());

        assert_eq!(health.status, EvolutionAdmissionHealthStatus::Watch);
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(health.reasons.iter().any(|reason| {
            reason == "evolution_admission_handoff_trend_continuation_history_gate_history_empty"
        }));
        assert!(health.dashboard.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_history_gate_dashboard_records=0"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_summary_history_records_stable_state()
     {
        let summary = stable_continuation_history_gate_summary();

        let record = EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder::new()
            .record_summary_with_health(
                EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory::new(),
                summary,
                EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Stable);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.effective_admitted_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert_eq!(record.dashboard.next_queue_task_count, 1);
        assert_eq!(
            record.history.summaries()[0].next_queue_task_ids,
            vec!["business-task"]
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_history_gate_history_health=stable"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_summary_history_watches_queue_pressure()
     {
        let summary = stable_continuation_history_gate_summary();
        let policy = EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy {
            maximum_next_queue_tasks: 0,
            ..EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default()
        };

        let record = EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder::new()
            .record_summary_with_health(
                EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory::new(),
                summary,
                policy,
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Watch);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.next_queue_task_count, 1);
        assert!(record.health.reasons.iter().any(|reason| {
            reason
                == "evolution_admission_handoff_trend_continuation_history_gate_next_queue_tasks=1>0"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_summary_history_repairs_dirty_gate_outputs()
     {
        let summary = repair_continuation_history_gate_summary();

        let record = EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder::new()
            .record_summary_with_health(
                EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory::new(),
                summary,
                EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(record.dashboard.effective_admitted_records, 0);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert!(record.dashboard.repair_task_count > 0);
        assert!(record.dashboard.blocked_reason_count > 0);
        assert!(record.health.reasons.iter().any(|reason| {
            reason
                == "evolution_admission_handoff_trend_continuation_history_gate_repair_first_records=1>0"
        }));
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_evolution_admission_handoff_trend_continuation_history_gate_history_health=repair"
        }));
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_history_record_exposes_stable_promotions()
     {
        let summary = stable_continuation_history_gate_summary();

        let record = EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder::new()
            .record_summary_with_health(
                EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory::new(),
                summary,
                EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
            );

        assert!(record.is_effectively_admitted());
        assert!(record.can_promote_ready_proposals());
        assert!(record.can_promote_evolution_signals());
        assert!(record.can_reinforce_process());
        assert!(record.can_promote_adaptive_state());
        assert_eq!(record.next_queue_task_ids(), &["business-task".to_owned()]);
        assert!(record.repair_task_ids().is_empty());
        assert_eq!(record.blocked_reason_count(), 0);
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_history_record_closes_promotions_on_watch()
     {
        let summary = stable_continuation_history_gate_summary();
        let policy = EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy {
            maximum_next_queue_tasks: 0,
            ..EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default()
        };

        let record = EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder::new()
            .record_summary_with_health(
                EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory::new(),
                summary,
                policy,
            );

        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Watch);
        assert!(record.is_effectively_admitted());
        assert!(record.allows_service_advance());
        assert!(!record.can_promote_ready_proposals());
        assert!(!record.can_promote_evolution_signals());
        assert!(!record.can_reinforce_process());
        assert!(!record.can_promote_adaptive_state());
        assert_eq!(record.next_queue_task_ids(), &["business-task".to_owned()]);
    }

    #[test]
    fn evolution_admission_handoff_trend_continuation_history_gate_history_record_closes_admission_on_repair()
     {
        let summary = repair_continuation_history_gate_summary();

        let record = EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder::new()
            .record_summary_with_health(
                EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory::new(),
                summary,
                EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
            );

        assert_eq!(record.health.status, EvolutionAdmissionHealthStatus::Repair);
        assert!(!record.is_effectively_admitted());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(!record.can_promote_ready_proposals());
        assert!(!record.can_promote_evolution_signals());
        assert!(!record.can_reinforce_process());
        assert!(!record.can_promote_adaptive_state());
        assert!(!record.repair_task_ids().is_empty());
        assert!(record.blocked_reason_count() > 0);
    }

    #[test]
    fn rust_only_toolsmith_plan_blocks_non_rust_proposals() {
        let plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "trace-script",
            ToolIntent::TraceAnalysis,
            "python",
            "tools/trace.py",
            ToolBuildStatus::Ready,
        ));

        assert!(!plan.passed_rust_gate());
        assert_eq!(plan.ready_count(), 1);
        assert!(plan.reward_notes()[0].contains("proposals=1"));
    }

    #[test]
    fn clean_closed_loop_reinforces_and_emits_evolution_signal() {
        let report = AgentRunReport {
            aggregation: AggregationReport::default(),
            conflicts: ConflictReport::default(),
            budget_audit: crate::run::RunBudgetAudit::default(),
            side_effects: vec![
                SideEffectGate::allow(SideEffectKind::MemoryNote, "ok"),
                SideEffectGate::allow(SideEffectKind::FileWrite, "ok"),
            ],
        };
        let toolsmith_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));
        let input = ProcessRewardInput {
            quality: 0.92,
            validation_passed: true,
            runtime_response_ok: true,
            execution_failures: 0,
            reflection_complete: true,
            recursive_chunks: 1,
            recursive_waves: 1,
            run_report: report,
            toolsmith_plan,
            tool_build_report: None,
        };

        let scored = ClosedLoopRewarder::new().score(input);

        assert_eq!(scored.action, RewardAction::Reinforce);
        assert!(scored.total >= 0.72);
        assert!(
            scored
                .evolution_signals
                .iter()
                .any(|signal| signal.action == "promote_closed_loop_pattern")
        );
    }

    #[test]
    fn dirty_tool_build_report_lowers_process_reward_toolsmith_component() {
        use crate::ports::{ToolBuildReceipt, ToolBuildRequest};

        let report = AgentRunReport {
            aggregation: AggregationReport::default(),
            conflicts: ConflictReport::default(),
            budget_audit: crate::run::RunBudgetAudit::default(),
            side_effects: vec![SideEffectGate::allow(SideEffectKind::MemoryNote, "ok")],
        };
        let toolsmith_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));
        let build_requests = vec![ToolBuildRequest {
            proposal_id: "runtime-gate".to_owned(),
            intent: ToolIntent::BenchmarkGate,
            rust_crate: "rust".to_owned(),
            entrypoint: "tools/runtime_gate.rs".to_owned(),
            gate_notes: Vec::new(),
        }];
        let build_report = ToolBuildReport::from_requests_and_receipts(
            &build_requests,
            &[ToolBuildReceipt::held("runtime-gate", "adapter timeout")],
        );
        let input = ProcessRewardInput {
            quality: 0.92,
            validation_passed: true,
            runtime_response_ok: true,
            execution_failures: 0,
            reflection_complete: true,
            recursive_chunks: 1,
            recursive_waves: 1,
            run_report: report,
            toolsmith_plan,
            tool_build_report: Some(build_report),
        };

        let scored = ClosedLoopRewarder::new().score(input);
        let summary = scored.summary();

        assert_eq!(scored.components.toolsmith, 0.18);
        assert!(summary.low_component_count >= 1);
        assert!(scored.notes.iter().any(|note| {
            note == "tool_build:repair_first missing=0 unexpected=0 duplicate=0 held=1 rejected=0"
        }));
    }

    #[test]
    fn high_quality_incomplete_reflection_holds_process_reward_before_reinforcement() {
        let report = AgentRunReport {
            aggregation: AggregationReport::default(),
            conflicts: ConflictReport::default(),
            budget_audit: crate::run::RunBudgetAudit::default(),
            side_effects: vec![
                SideEffectGate::block(
                    SideEffectKind::MemoryNote,
                    "memory note requires a complete reflection loop",
                ),
                SideEffectGate::allow(SideEffectKind::FileWrite, "ok"),
                SideEffectGate::allow(SideEffectKind::AdaptiveStateWrite, "ok"),
                SideEffectGate::allow(SideEffectKind::ExternalCall, "ok"),
            ],
        };
        let toolsmith_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));
        let input = ProcessRewardInput {
            quality: 0.92,
            validation_passed: true,
            runtime_response_ok: true,
            execution_failures: 0,
            reflection_complete: false,
            recursive_chunks: 1,
            recursive_waves: 1,
            run_report: report,
            toolsmith_plan,
            tool_build_report: None,
        };

        let scored = ClosedLoopRewarder::new().score(input);

        assert_eq!(scored.action, RewardAction::Hold);
        assert!(scored.total < ProcessRewardPolicy::default().reinforce_at);
        assert!((scored.components.reflection - 0.364).abs() < 0.001);
        assert!(
            scored
                .notes
                .iter()
                .any(|note| note == "reflection:incomplete")
        );
        assert!(
            scored
                .evolution_signals
                .iter()
                .any(|signal| signal.action == "hold_for_more_evidence")
        );
        assert!(
            !scored
                .evolution_signals
                .iter()
                .any(|signal| signal.action == "promote_closed_loop_pattern")
        );
    }

    #[test]
    fn blocked_side_effect_holds_tool_build_even_with_ready_proposal_and_high_reward() {
        use crate::ports::ToolBuildRequest;

        let report = AgentRunReport {
            aggregation: AggregationReport::default(),
            conflicts: ConflictReport::default(),
            budget_audit: crate::run::RunBudgetAudit::default(),
            side_effects: vec![
                SideEffectGate::block(
                    SideEffectKind::MemoryNote,
                    "memory note waits for side-effect admission",
                ),
                SideEffectGate::allow(SideEffectKind::FileWrite, "ok"),
                SideEffectGate::allow(SideEffectKind::AdaptiveStateWrite, "ok"),
                SideEffectGate::allow(SideEffectKind::ExternalCall, "ok"),
            ],
        };
        let toolsmith_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));
        let scored = ClosedLoopRewarder::new().score(ProcessRewardInput {
            quality: 0.92,
            validation_passed: true,
            runtime_response_ok: true,
            execution_failures: 0,
            reflection_complete: true,
            recursive_chunks: 1,
            recursive_waves: 1,
            run_report: report,
            toolsmith_plan: toolsmith_plan.clone(),
            tool_build_report: None,
        });
        let toolsmith_record = ToolsmithPlanSummaryHistoryRecorder::new()
            .record_plan_with_health_gate(
                ToolsmithPlanSummaryHistory::new(),
                &toolsmith_plan,
                ToolsmithPlanHealthPolicy::default(),
            );
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &scored,
                ProcessRewardReportHealthPolicy::default(),
            );

        let admission = EvolutionAdmissionGate::new().gate(toolsmith_record, reward_record);
        let final_gate = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health_gate(
                EvolutionAdmissionSummaryHistory::new(),
                &admission,
                EvolutionAdmissionHealthPolicy::default(),
            );

        assert!(scored.total >= ProcessRewardPolicy::default().reinforce_at);
        assert_eq!(scored.action, RewardAction::Hold);
        assert!(
            scored
                .notes
                .iter()
                .any(|note| { note == "admission:side_effects_blocked_or_low_quality" })
        );
        assert!(admission.allows_service_advance());
        assert!(!admission.requires_repair_first());
        assert!(!admission.can_promote_ready_proposals());
        assert!(admission.can_promote_evolution_signals());
        assert!(!admission.decision.can_reinforce_process);
        assert!(!final_gate.can_promote_ready_proposals());
        assert!(ToolBuildRequest::ready_requests(&toolsmith_plan).len() == 1);
        assert!(
            ToolBuildRequest::admitted_by_evolution(&toolsmith_plan, &final_gate.gate_decision)
                .is_empty()
        );
    }

    #[test]
    fn conflicts_and_failed_validation_penalize_closed_loop() {
        let report = AgentRunReport {
            aggregation: AggregationReport::default(),
            conflicts: ConflictReport {
                conflicts: vec![AgentConflict {
                    topic: "memory".to_owned(),
                    message_ids: vec!["coder".to_owned(), "reviewer".to_owned()],
                    roles: vec![AgentRole::Coder, AgentRole::Reviewer],
                    summary: "conflicting memory write".to_owned(),
                    resolved: false,
                    resolution_hint: "resolve before memory write".to_owned(),
                }],
                messages: Vec::new(),
            },
            budget_audit: crate::run::RunBudgetAudit::default(),
            side_effects: vec![SideEffectGate::block(
                SideEffectKind::MemoryNote,
                "unresolved conflict",
            )],
        };
        let input = ProcessRewardInput {
            quality: 0.48,
            validation_passed: false,
            runtime_response_ok: true,
            execution_failures: 0,
            reflection_complete: false,
            recursive_chunks: 16,
            recursive_waves: 4,
            run_report: report,
            toolsmith_plan: ToolsmithPlan::new(),
            tool_build_report: None,
        };

        let scored = ClosedLoopRewarder::new().score(input);

        assert_eq!(scored.action, RewardAction::Penalize);
        assert!(scored.total <= 0.42);
        assert!(
            scored
                .notes
                .iter()
                .any(|note| note == "validation:failed_or_missing")
        );
    }

    #[test]
    fn budget_overspend_penalizes_closed_loop_admission() {
        let report = AgentRunReport {
            aggregation: AggregationReport::default(),
            conflicts: ConflictReport::default(),
            budget_audit: crate::run::RunBudgetAudit {
                overspends: vec![crate::run::RunBudgetOverspend {
                    task_id: "coder".to_owned(),
                    role: AgentRole::Coder,
                    reserved: crate::budget::AgentBudget::new(8, 1, 1),
                    spent: crate::budget::AgentBudget::new(9, 1, 1),
                }],
            },
            side_effects: vec![SideEffectGate::allow(SideEffectKind::MemoryNote, "ok")],
        };
        let input = ProcessRewardInput {
            quality: 0.90,
            validation_passed: true,
            runtime_response_ok: true,
            execution_failures: 0,
            reflection_complete: true,
            recursive_chunks: 1,
            recursive_waves: 1,
            run_report: report,
            toolsmith_plan: ToolsmithPlan::new(),
            tool_build_report: None,
        };

        let scored = ClosedLoopRewarder::new().score(input);

        assert!(scored.components.admission <= 0.20);
        assert!(
            scored
                .notes
                .iter()
                .any(|note| note == "budget:overspends=1")
        );
    }
}
