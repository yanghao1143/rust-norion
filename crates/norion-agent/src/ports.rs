use crate::budget::AgentBudget;
use crate::evolution::{
    EvolutionAdmissionHistoryGateDecision, ToolBuildStatus, ToolIntent, ToolProposal,
    ToolsmithPlan, ToolsmithPlanHistoryGateDecision,
};
use crate::message::{AgentMessage, AgentMessageKind};
use crate::task::{AgentResult, AgentRole, AgentTask, AgentTaskQueue};
use std::collections::BTreeSet;

pub trait EnginePort {
    type Error;

    fn run_task(&mut self, task: &AgentTask) -> Result<AgentResult, Self::Error>;
}

pub trait RoutedEnginePort: EnginePort {
    fn run_routed_task(
        &mut self,
        request: &AgentModelRouteRequest,
    ) -> Result<AgentResult, AgentModelRouteRunError<Self::Error>> {
        request.validate().map_err(AgentModelRouteRunError::Route)?;
        let mut result = self
            .run_task(&request.task)
            .map_err(AgentModelRouteRunError::Engine)?;
        result.messages.push(request.route_gate_message());
        Ok(result)
    }
}

impl<T: EnginePort + ?Sized> RoutedEnginePort for T {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentModelRouteProof {
    pub model_registry_id: String,
    pub model_profile_id: String,
    pub inference_backend_id: String,
    pub model_pool_id: String,
}

impl AgentModelRouteProof {
    pub fn new(
        model_registry_id: impl AsRef<str>,
        model_profile_id: impl AsRef<str>,
        inference_backend_id: impl AsRef<str>,
        model_pool_id: impl AsRef<str>,
    ) -> Self {
        Self {
            model_registry_id: trimmed(model_registry_id.as_ref()),
            model_profile_id: trimmed(model_profile_id.as_ref()),
            inference_backend_id: trimmed(inference_backend_id.as_ref()),
            model_pool_id: trimmed(model_pool_id.as_ref()),
        }
    }

    pub fn validate(&self) -> Result<(), AgentModelRouteError> {
        require_field("model_registry_id", &self.model_registry_id)?;
        require_field("model_profile_id", &self.model_profile_id)?;
        require_field("inference_backend_id", &self.inference_backend_id)?;
        require_field("model_pool_id", &self.model_pool_id)
    }

    pub fn telemetry(&self) -> Vec<String> {
        vec![
            format!(
                "agent_model_route_model_registry_id={}",
                self.model_registry_id
            ),
            format!(
                "agent_model_route_model_profile_id={}",
                self.model_profile_id
            ),
            format!(
                "agent_model_route_inference_backend_id={}",
                self.inference_backend_id
            ),
            format!("agent_model_route_model_pool_id={}", self.model_pool_id),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentModelRouteRequest {
    pub task: AgentTask,
    pub prompt: String,
    pub route: AgentModelRouteProof,
}

impl AgentModelRouteRequest {
    pub fn try_new(
        task: AgentTask,
        prompt: impl AsRef<str>,
        route: AgentModelRouteProof,
    ) -> Result<Self, AgentModelRouteError> {
        let request = Self {
            task,
            prompt: trimmed(prompt.as_ref()),
            route,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), AgentModelRouteError> {
        require_field("prompt", &self.prompt)?;
        self.route.validate()
    }

    pub fn route_gate_message(&self) -> AgentMessage {
        let mut message = AgentMessage::new(
            format!("{}-layer-b-model-route", self.task.id),
            self.task.role.clone(),
            AgentMessageKind::Gate,
            "layer_b_model_route",
            format!(
                "model_registry_id={} model_profile_id={} inference_backend_id={} model_pool_id={} prompt_chars={}",
                self.route.model_registry_id,
                self.route.model_profile_id,
                self.route.inference_backend_id,
                self.route.model_pool_id,
                self.prompt.chars().count()
            ),
        );
        for line in self.route.telemetry() {
            message = message.with_evidence(line);
        }
        message.with_evidence(format!(
            "agent_model_route_prompt_chars={}",
            self.prompt.chars().count()
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentModelRouteError {
    MissingField(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentModelRouteRunError<E> {
    Route(AgentModelRouteError),
    Engine(E),
}

pub trait MemoryPort {
    type Error;

    fn recall(
        &self,
        request: &MemoryRecallRequest,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>, Self::Error>;

    fn propose_note(&mut self, note: MemoryNote) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryScope {
    pub tenant_id: String,
    pub workspace_id: String,
    pub session_id: String,
}

impl AgentMemoryScope {
    pub fn new(
        tenant_id: impl AsRef<str>,
        workspace_id: impl AsRef<str>,
        session_id: impl AsRef<str>,
    ) -> Self {
        Self {
            tenant_id: scope_id(tenant_id.as_ref(), "local"),
            workspace_id: scope_id(workspace_id.as_ref(), "default"),
            session_id: scope_id(session_id.as_ref(), "interactive"),
        }
    }

    pub fn local_single_user() -> Self {
        Self::new("local", "default", "interactive")
    }
}

impl Default for AgentMemoryScope {
    fn default() -> Self {
        Self::local_single_user()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecallRequest {
    pub query: String,
    pub scope: AgentMemoryScope,
}

impl MemoryRecallRequest {
    pub fn new(query: impl Into<String>, scope: AgentMemoryScope) -> Self {
        Self {
            query: query.into(),
            scope,
        }
    }
}

fn scope_id(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn trimmed(value: &str) -> String {
    value.trim().to_owned()
}

fn require_field(field: &'static str, value: &str) -> Result<(), AgentModelRouteError> {
    if value.trim().is_empty() {
        Err(AgentModelRouteError::MissingField(field))
    } else {
        Ok(())
    }
}

pub trait ToolBuildPort {
    type Error;

    fn build_tool(&mut self, request: &ToolBuildRequest) -> Result<ToolBuildReceipt, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecord {
    pub id: String,
    pub summary: String,
    pub source: String,
}

impl MemoryRecord {
    pub fn new(
        id: impl Into<String>,
        summary: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            summary: summary.into(),
            source: source.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryNote {
    pub topic: String,
    pub content: String,
    pub evidence: Vec<String>,
}

impl MemoryNote {
    pub fn new(topic: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            content: content.into(),
            evidence: Vec::new(),
        }
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBuildRequest {
    pub proposal_id: String,
    pub intent: ToolIntent,
    pub rust_crate: String,
    pub entrypoint: String,
    pub gate_notes: Vec<String>,
}

impl ToolBuildRequest {
    pub fn from_ready_proposal(proposal: &ToolProposal) -> Option<Self> {
        if proposal.status != ToolBuildStatus::Ready || !proposal.rust_only() {
            return None;
        }

        Some(Self {
            proposal_id: proposal.id.clone(),
            intent: proposal.intent,
            rust_crate: proposal.rust_crate.clone(),
            entrypoint: proposal.entrypoint.clone(),
            gate_notes: proposal.gate_notes.clone(),
        })
    }

    pub fn ready_requests(plan: &ToolsmithPlan) -> Vec<Self> {
        plan.proposals
            .iter()
            .filter_map(Self::from_ready_proposal)
            .collect()
    }

    pub fn admitted_requests(
        plan: &ToolsmithPlan,
        gate_decision: &ToolsmithPlanHistoryGateDecision,
    ) -> Vec<Self> {
        if !gate_decision.is_promotable() {
            return Vec::new();
        }

        Self::ready_requests(plan)
    }

    pub fn admitted_by_evolution(
        plan: &ToolsmithPlan,
        gate_decision: &EvolutionAdmissionHistoryGateDecision,
    ) -> Vec<Self> {
        if gate_decision.requires_repair_first || !gate_decision.can_promote_ready_proposals {
            return Vec::new();
        }

        Self::ready_requests(plan)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBuildReceipt {
    pub proposal_id: String,
    pub status: ToolBuildStatus,
    pub artifact: Option<String>,
    pub diagnostics: Vec<String>,
}

impl ToolBuildReceipt {
    pub fn built(proposal_id: impl Into<String>, artifact: impl Into<String>) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            status: ToolBuildStatus::Ready,
            artifact: Some(artifact.into()),
            diagnostics: Vec::new(),
        }
    }

    pub fn held(proposal_id: impl Into<String>, diagnostic: impl Into<String>) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            status: ToolBuildStatus::Held,
            artifact: None,
            diagnostics: vec![diagnostic.into()],
        }
    }

    pub fn rejected(proposal_id: impl Into<String>, diagnostic: impl Into<String>) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            status: ToolBuildStatus::Rejected,
            artifact: None,
            diagnostics: vec![diagnostic.into()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBuildReport {
    pub requested: usize,
    pub received: usize,
    pub built: usize,
    pub held: usize,
    pub rejected: usize,
    pub missing_request_ids: Vec<String>,
    pub unexpected_receipt_ids: Vec<String>,
    pub duplicate_receipt_ids: Vec<String>,
    pub diagnostics: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBuildReportSummary {
    pub requested: usize,
    pub received: usize,
    pub built: usize,
    pub held: usize,
    pub rejected: usize,
    pub missing_requests: usize,
    pub unexpected_receipts: usize,
    pub duplicate_receipts: usize,
    pub diagnostics: usize,
    pub is_clean: bool,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolBuildReliabilitySummary {
    pub attempts: usize,
    pub successes: usize,
    pub issue_count: usize,
    pub success_rate: f32,
    pub issue_rate: f32,
    pub reliable: bool,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ToolBuildReportSummaryHistory {
    summaries: Vec<ToolBuildReportSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolBuildReportDashboard {
    pub total_records: usize,
    pub clean_records: usize,
    pub repair_records: usize,
    pub requested: usize,
    pub received: usize,
    pub built: usize,
    pub held: usize,
    pub rejected: usize,
    pub missing_requests: usize,
    pub unexpected_receipts: usize,
    pub duplicate_receipts: usize,
    pub diagnostics: usize,
    pub clean_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolBuildReportHealthPolicy {
    pub maximum_missing_requests: usize,
    pub maximum_unexpected_receipts: usize,
    pub maximum_duplicate_receipts: usize,
    pub maximum_held_receipts: usize,
    pub maximum_rejected_receipts: usize,
}

impl Default for ToolBuildReportHealthPolicy {
    fn default() -> Self {
        Self {
            maximum_missing_requests: 0,
            maximum_unexpected_receipts: 0,
            maximum_duplicate_receipts: 0,
            maximum_held_receipts: 0,
            maximum_rejected_receipts: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolBuildReportHealthStatus {
    Stable,
    Watch,
    Repair,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolBuildReportHealth {
    pub status: ToolBuildReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: ToolBuildReportDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolBuildReportSummaryHistoryRecord {
    pub history: ToolBuildReportSummaryHistory,
    pub appended_summary: ToolBuildReportSummary,
    pub dashboard: ToolBuildReportDashboard,
    pub health: ToolBuildReportHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolBuildReportSummaryHistoryRecorder;

#[derive(Debug, Clone, PartialEq)]
pub struct ToolBuildReportHistoryGateDecision {
    pub report_summary: ToolBuildReportSummary,
    pub report_health: ToolBuildReportHealth,
    pub can_open_tool_build_boundary: bool,
    pub can_promote_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub can_finalize_eval: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl ToolBuildReportHistoryGateDecision {
    pub fn is_tool_build_boundary_open(&self) -> bool {
        self.can_open_tool_build_boundary && !self.requires_repair_first
    }

    pub fn is_promotion_safe(&self) -> bool {
        self.can_promote_memory_note
            && self.can_promote_adaptive_state
            && self.can_finalize_eval
            && !self.requires_repair_first
    }

    pub fn repair_first_queue(&self, next_queue: AgentTaskQueue) -> AgentTaskQueue {
        next_queue.with_repair_first(&self.repair_tasks)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolBuildReportHistoryGateRecord {
    pub health_record: ToolBuildReportSummaryHistoryRecord,
    pub gate_decision: ToolBuildReportHistoryGateDecision,
    pub telemetry: Vec<String>,
}

impl ToolBuildReportHistoryGateRecord {
    pub fn records(&self) -> usize {
        self.health_record.records()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn can_open_tool_build_boundary(&self) -> bool {
        self.gate_decision.can_open_tool_build_boundary
    }

    pub fn can_promote_memory_note(&self) -> bool {
        self.gate_decision.can_promote_memory_note
    }

    pub fn can_promote_adaptive_state(&self) -> bool {
        self.gate_decision.can_promote_adaptive_state
    }

    pub fn can_finalize_eval(&self) -> bool {
        self.gate_decision.can_finalize_eval
    }
}

#[derive(Debug, Clone, Default)]
pub struct ToolBuildReportHistoryGate;

impl ToolBuildReport {
    pub fn from_requests_and_receipts(
        requests: &[ToolBuildRequest],
        receipts: &[ToolBuildReceipt],
    ) -> Self {
        let requested_ids = requests
            .iter()
            .map(|request| request.proposal_id.clone())
            .collect::<Vec<_>>();
        let requested_set = requested_ids.iter().cloned().collect::<BTreeSet<_>>();
        let mut seen_receipts = BTreeSet::new();
        let mut built = 0;
        let mut held = 0;
        let mut rejected = 0;
        let mut unexpected_receipt_ids = Vec::new();
        let mut duplicate_receipt_ids = Vec::new();
        let mut diagnostics = Vec::new();

        for receipt in receipts {
            if !requested_set.contains(&receipt.proposal_id) {
                unexpected_receipt_ids.push(receipt.proposal_id.clone());
            }
            if !seen_receipts.insert(receipt.proposal_id.clone()) {
                duplicate_receipt_ids.push(receipt.proposal_id.clone());
            }

            match receipt.status {
                ToolBuildStatus::Ready => built += 1,
                ToolBuildStatus::Held => held += 1,
                ToolBuildStatus::Rejected => rejected += 1,
            }

            diagnostics.extend(
                receipt
                    .diagnostics
                    .iter()
                    .map(|diagnostic| format!("{}:{diagnostic}", receipt.proposal_id)),
            );
        }

        let receipt_ids = receipts
            .iter()
            .map(|receipt| receipt.proposal_id.clone())
            .collect::<BTreeSet<_>>();
        let missing_request_ids = requested_ids
            .into_iter()
            .filter(|id| !receipt_ids.contains(id))
            .collect::<Vec<_>>();
        let received = receipts.len();
        let telemetry = tool_build_report_telemetry(
            requests.len(),
            received,
            built,
            held,
            rejected,
            missing_request_ids.len(),
            unexpected_receipt_ids.len(),
            duplicate_receipt_ids.len(),
        );

        Self {
            requested: requests.len(),
            received,
            built,
            held,
            rejected,
            missing_request_ids,
            unexpected_receipt_ids,
            duplicate_receipt_ids,
            diagnostics,
            telemetry,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.missing_request_ids.is_empty()
            && self.unexpected_receipt_ids.is_empty()
            && self.duplicate_receipt_ids.is_empty()
            && self.held == 0
            && self.rejected == 0
    }

    pub fn requires_repair_first(&self) -> bool {
        !self.is_clean()
    }

    pub fn summary(&self) -> ToolBuildReportSummary {
        ToolBuildReportSummary::from_report(self)
    }
}

impl ToolBuildReportSummary {
    pub fn from_report(report: &ToolBuildReport) -> Self {
        let missing_requests = report.missing_request_ids.len();
        let unexpected_receipts = report.unexpected_receipt_ids.len();
        let duplicate_receipts = report.duplicate_receipt_ids.len();
        let diagnostics = report.diagnostics.len();
        let is_clean = report.is_clean();
        let telemetry = tool_build_report_summary_telemetry(
            report.requested,
            report.received,
            report.built,
            report.held,
            report.rejected,
            missing_requests,
            unexpected_receipts,
            duplicate_receipts,
            diagnostics,
            is_clean,
        );

        Self {
            requested: report.requested,
            received: report.received,
            built: report.built,
            held: report.held,
            rejected: report.rejected,
            missing_requests,
            unexpected_receipts,
            duplicate_receipts,
            diagnostics,
            is_clean,
            telemetry,
        }
    }

    pub fn requires_repair_first(&self) -> bool {
        !self.is_clean
    }

    pub fn reliability(&self) -> ToolBuildReliabilitySummary {
        ToolBuildReliabilitySummary::from_counts(
            "agent_tool_build_report_summary",
            self.requested,
            self.received,
            self.built,
            self.held,
            self.rejected,
            self.missing_requests,
            self.unexpected_receipts,
            self.duplicate_receipts,
        )
    }
}

impl ToolBuildReliabilitySummary {
    #[allow(clippy::too_many_arguments)]
    fn from_counts(
        telemetry_scope: &str,
        attempts: usize,
        received: usize,
        successes: usize,
        held: usize,
        rejected: usize,
        missing_requests: usize,
        unexpected_receipts: usize,
        duplicate_receipts: usize,
    ) -> Self {
        let issue_count = held
            .saturating_add(rejected)
            .saturating_add(missing_requests)
            .saturating_add(unexpected_receipts)
            .saturating_add(duplicate_receipts);
        let success_rate = rate(successes, attempts);
        let issue_rate = rate(issue_count, attempts.max(received));
        let reliable = attempts > 0 && success_rate >= 1.0 && issue_count == 0;
        let telemetry = vec![
            format!("{telemetry_scope}_reliability=true"),
            format!("{telemetry_scope}_attempts={attempts}"),
            format!("{telemetry_scope}_successes={successes}"),
            format!("{telemetry_scope}_issue_count={issue_count}"),
            format!("{telemetry_scope}_success_rate={success_rate:.3}"),
            format!("{telemetry_scope}_issue_rate={issue_rate:.3}"),
            format!("{telemetry_scope}_reliable={reliable}"),
        ];

        Self {
            attempts,
            successes,
            issue_count,
            success_rate,
            issue_rate,
            reliable,
            telemetry,
        }
    }
}

impl ToolBuildReportSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<ToolBuildReportSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: ToolBuildReportSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[ToolBuildReportSummary] {
        &self.summaries
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> ToolBuildReportDashboard {
        ToolBuildReportDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: ToolBuildReportHealthPolicy) -> ToolBuildReportHealth {
        self.dashboard().health(policy)
    }
}

impl ToolBuildReportDashboard {
    pub fn from_summaries(summaries: &[ToolBuildReportSummary]) -> Self {
        let total_records = summaries.len();
        let clean_records = summaries.iter().filter(|summary| summary.is_clean).count();
        let repair_records = total_records.saturating_sub(clean_records);
        let requested = summaries
            .iter()
            .map(|summary| summary.requested)
            .sum::<usize>();
        let received = summaries
            .iter()
            .map(|summary| summary.received)
            .sum::<usize>();
        let built = summaries.iter().map(|summary| summary.built).sum::<usize>();
        let held = summaries.iter().map(|summary| summary.held).sum::<usize>();
        let rejected = summaries
            .iter()
            .map(|summary| summary.rejected)
            .sum::<usize>();
        let missing_requests = summaries
            .iter()
            .map(|summary| summary.missing_requests)
            .sum::<usize>();
        let unexpected_receipts = summaries
            .iter()
            .map(|summary| summary.unexpected_receipts)
            .sum::<usize>();
        let duplicate_receipts = summaries
            .iter()
            .map(|summary| summary.duplicate_receipts)
            .sum::<usize>();
        let diagnostics = summaries
            .iter()
            .map(|summary| summary.diagnostics)
            .sum::<usize>();
        let clean_rate = rate(clean_records, total_records);
        let telemetry = tool_build_report_dashboard_telemetry(
            total_records,
            clean_records,
            repair_records,
            requested,
            received,
            built,
            held,
            rejected,
            missing_requests,
            unexpected_receipts,
            duplicate_receipts,
            diagnostics,
            clean_rate,
        );

        Self {
            total_records,
            clean_records,
            repair_records,
            requested,
            received,
            built,
            held,
            rejected,
            missing_requests,
            unexpected_receipts,
            duplicate_receipts,
            diagnostics,
            clean_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: ToolBuildReportHealthPolicy) -> ToolBuildReportHealth {
        ToolBuildReportHealth::from_dashboard(self.clone(), policy)
    }

    pub fn reliability(&self) -> ToolBuildReliabilitySummary {
        ToolBuildReliabilitySummary::from_counts(
            "agent_tool_build_report_dashboard",
            self.requested,
            self.received,
            self.built,
            self.held,
            self.rejected,
            self.missing_requests,
            self.unexpected_receipts,
            self.duplicate_receipts,
        )
    }
}

impl ToolBuildReportHealth {
    pub fn from_dashboard(
        dashboard: ToolBuildReportDashboard,
        policy: ToolBuildReportHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("tool_build_report_history_empty".to_owned());
        }

        if dashboard.missing_requests > policy.maximum_missing_requests {
            repair_reasons.push(format!(
                "tool_build_report_missing_requests={}>{}",
                dashboard.missing_requests, policy.maximum_missing_requests
            ));
        }

        if dashboard.unexpected_receipts > policy.maximum_unexpected_receipts {
            repair_reasons.push(format!(
                "tool_build_report_unexpected_receipts={}>{}",
                dashboard.unexpected_receipts, policy.maximum_unexpected_receipts
            ));
        }

        if dashboard.duplicate_receipts > policy.maximum_duplicate_receipts {
            repair_reasons.push(format!(
                "tool_build_report_duplicate_receipts={}>{}",
                dashboard.duplicate_receipts, policy.maximum_duplicate_receipts
            ));
        }

        if dashboard.held > policy.maximum_held_receipts {
            repair_reasons.push(format!(
                "tool_build_report_held_receipts={}>{}",
                dashboard.held, policy.maximum_held_receipts
            ));
        }

        if dashboard.rejected > policy.maximum_rejected_receipts {
            repair_reasons.push(format!(
                "tool_build_report_rejected_receipts={}>{}",
                dashboard.rejected, policy.maximum_rejected_receipts
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            (ToolBuildReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (ToolBuildReportHealthStatus::Watch, watch_reasons)
        } else {
            (ToolBuildReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == ToolBuildReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != ToolBuildReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == ToolBuildReportHealthStatus::Repair
    }
}

impl ToolBuildReportSummaryHistoryRecord {
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

impl ToolBuildReportSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: ToolBuildReportSummaryHistory,
        summary: ToolBuildReportSummary,
        policy: ToolBuildReportHealthPolicy,
    ) -> ToolBuildReportSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = tool_build_report_history_record_telemetry(&dashboard, &health);

        ToolBuildReportSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_report_with_health(
        &self,
        history: ToolBuildReportSummaryHistory,
        report: &ToolBuildReport,
        policy: ToolBuildReportHealthPolicy,
    ) -> ToolBuildReportSummaryHistoryRecord {
        self.record_summary_with_health(history, report.summary(), policy)
    }

    pub fn record_report_with_health_gate(
        &self,
        history: ToolBuildReportSummaryHistory,
        report: &ToolBuildReport,
        policy: ToolBuildReportHealthPolicy,
    ) -> ToolBuildReportHistoryGateRecord {
        let health_record = self.record_report_with_health(history, report, policy);
        let gate_decision = ToolBuildReportHistoryGate::new().gate(report, &health_record);
        let telemetry =
            tool_build_report_history_gate_record_telemetry(&health_record, &gate_decision);

        ToolBuildReportHistoryGateRecord {
            health_record,
            gate_decision,
            telemetry,
        }
    }
}

impl ToolBuildReportHistoryGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        report: &ToolBuildReport,
        history_record: &ToolBuildReportSummaryHistoryRecord,
    ) -> ToolBuildReportHistoryGateDecision {
        let report_summary = report.summary();
        let report_health = history_record.health.clone();
        let mut reasons = tool_build_report_gate_reasons(&report_summary);
        push_ordered_unique(
            &mut reasons,
            report_health
                .reasons
                .iter()
                .map(|reason| format!("tool_build_report_history:{reason}")),
        );
        let current_requires_repair = report_summary.requires_repair_first();
        let requires_repair_first =
            current_requires_repair || report_health.requires_repair_first();
        let can_open_tool_build_boundary = report_summary.is_clean
            && report_health.allows_service_advance()
            && !requires_repair_first;
        let can_promote_memory_note = can_open_tool_build_boundary && report_health.is_stable();
        let can_promote_adaptive_state = can_open_tool_build_boundary && report_health.is_stable();
        let can_finalize_eval = can_open_tool_build_boundary && report_health.is_stable();
        let repair_tasks =
            tool_build_report_history_gate_repair_tasks(requires_repair_first, &reasons);
        let telemetry = tool_build_report_history_gate_telemetry(
            can_open_tool_build_boundary,
            can_promote_memory_note,
            can_promote_adaptive_state,
            can_finalize_eval,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            &report_summary,
            report_health.status,
        );

        ToolBuildReportHistoryGateDecision {
            report_summary,
            report_health,
            can_open_tool_build_boundary,
            can_promote_memory_note,
            can_promote_adaptive_state,
            can_finalize_eval,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

fn tool_build_report_telemetry(
    requested: usize,
    received: usize,
    built: usize,
    held: usize,
    rejected: usize,
    missing: usize,
    unexpected: usize,
    duplicate: usize,
) -> Vec<String> {
    vec![
        "agent_tool_build_report=true".to_owned(),
        format!("agent_tool_build_report_requested={requested}"),
        format!("agent_tool_build_report_received={received}"),
        format!("agent_tool_build_report_built={built}"),
        format!("agent_tool_build_report_held={held}"),
        format!("agent_tool_build_report_rejected={rejected}"),
        format!("agent_tool_build_report_missing={missing}"),
        format!("agent_tool_build_report_unexpected={unexpected}"),
        format!("agent_tool_build_report_duplicate={duplicate}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn tool_build_report_summary_telemetry(
    requested: usize,
    received: usize,
    built: usize,
    held: usize,
    rejected: usize,
    missing_requests: usize,
    unexpected_receipts: usize,
    duplicate_receipts: usize,
    diagnostics: usize,
    is_clean: bool,
) -> Vec<String> {
    let reliability = ToolBuildReliabilitySummary::from_counts(
        "agent_tool_build_report_summary",
        requested,
        received,
        built,
        held,
        rejected,
        missing_requests,
        unexpected_receipts,
        duplicate_receipts,
    );
    let mut telemetry = vec![
        "agent_tool_build_report_summary=true".to_owned(),
        format!("agent_tool_build_report_summary_requested={requested}"),
        format!("agent_tool_build_report_summary_received={received}"),
        format!("agent_tool_build_report_summary_built={built}"),
        format!("agent_tool_build_report_summary_held={held}"),
        format!("agent_tool_build_report_summary_rejected={rejected}"),
        format!("agent_tool_build_report_summary_missing_requests={missing_requests}"),
        format!("agent_tool_build_report_summary_unexpected_receipts={unexpected_receipts}"),
        format!("agent_tool_build_report_summary_duplicate_receipts={duplicate_receipts}"),
        format!("agent_tool_build_report_summary_diagnostics={diagnostics}"),
        format!("agent_tool_build_report_summary_clean={is_clean}"),
    ];
    telemetry.extend(reliability.telemetry);
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn tool_build_report_dashboard_telemetry(
    total_records: usize,
    clean_records: usize,
    repair_records: usize,
    requested: usize,
    received: usize,
    built: usize,
    held: usize,
    rejected: usize,
    missing_requests: usize,
    unexpected_receipts: usize,
    duplicate_receipts: usize,
    diagnostics: usize,
    clean_rate: f32,
) -> Vec<String> {
    let reliability = ToolBuildReliabilitySummary::from_counts(
        "agent_tool_build_report_dashboard",
        requested,
        received,
        built,
        held,
        rejected,
        missing_requests,
        unexpected_receipts,
        duplicate_receipts,
    );
    let mut telemetry = vec![
        "agent_tool_build_report_dashboard=true".to_owned(),
        format!("agent_tool_build_report_dashboard_records={total_records}"),
        format!("agent_tool_build_report_dashboard_clean_records={clean_records}"),
        format!("agent_tool_build_report_dashboard_repair_records={repair_records}"),
        format!("agent_tool_build_report_dashboard_requested={requested}"),
        format!("agent_tool_build_report_dashboard_received={received}"),
        format!("agent_tool_build_report_dashboard_built={built}"),
        format!("agent_tool_build_report_dashboard_held={held}"),
        format!("agent_tool_build_report_dashboard_rejected={rejected}"),
        format!("agent_tool_build_report_dashboard_missing_requests={missing_requests}"),
        format!("agent_tool_build_report_dashboard_unexpected_receipts={unexpected_receipts}"),
        format!("agent_tool_build_report_dashboard_duplicate_receipts={duplicate_receipts}"),
        format!("agent_tool_build_report_dashboard_diagnostics={diagnostics}"),
        format!("agent_tool_build_report_dashboard_clean_rate={clean_rate:.3}"),
    ];
    telemetry.extend(reliability.telemetry);
    telemetry
}

fn tool_build_report_history_record_telemetry(
    dashboard: &ToolBuildReportDashboard,
    health: &ToolBuildReportHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_tool_build_report_history_record=true".to_owned(),
        format!(
            "agent_tool_build_report_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_tool_build_report_history_record_status={:?}",
            health.status
        ),
    ];

    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_tool_build_report_history_record_reason={reason}")),
    );
    telemetry
}

fn tool_build_report_gate_reasons(summary: &ToolBuildReportSummary) -> Vec<String> {
    let mut reasons = Vec::new();

    if summary.missing_requests > 0 {
        reasons.push(format!(
            "tool_build_report_missing_requests={}",
            summary.missing_requests
        ));
    }
    if summary.unexpected_receipts > 0 {
        reasons.push(format!(
            "tool_build_report_unexpected_receipts={}",
            summary.unexpected_receipts
        ));
    }
    if summary.duplicate_receipts > 0 {
        reasons.push(format!(
            "tool_build_report_duplicate_receipts={}",
            summary.duplicate_receipts
        ));
    }
    if summary.held > 0 {
        reasons.push(format!("tool_build_report_held_receipts={}", summary.held));
    }
    if summary.rejected > 0 {
        reasons.push(format!(
            "tool_build_report_rejected_receipts={}",
            summary.rejected
        ));
    }

    reasons
}

fn tool_build_report_history_gate_repair_tasks(
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
                format!("tool-build-report-repair-{index}"),
                AgentRole::Planner,
                format!("repair tool build report: {reason}"),
                AgentBudget::new(14, 1, 1),
            )
            .with_lane("tool-build-report-repair")
            .with_priority(8)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn tool_build_report_history_gate_telemetry(
    can_open_tool_build_boundary: bool,
    can_promote_memory_note: bool,
    can_promote_adaptive_state: bool,
    can_finalize_eval: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    summary: &ToolBuildReportSummary,
    health_status: ToolBuildReportHealthStatus,
) -> Vec<String> {
    vec![
        "agent_tool_build_report_history_gate=true".to_owned(),
        format!(
            "agent_tool_build_report_history_gate_open_tool_build_boundary={can_open_tool_build_boundary}"
        ),
        format!(
            "agent_tool_build_report_history_gate_promote_memory_note={can_promote_memory_note}"
        ),
        format!(
            "agent_tool_build_report_history_gate_promote_adaptive_state={can_promote_adaptive_state}"
        ),
        format!("agent_tool_build_report_history_gate_finalize_eval={can_finalize_eval}"),
        format!(
            "agent_tool_build_report_history_gate_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_tool_build_report_history_gate_repair_tasks={repair_tasks}"),
        format!("agent_tool_build_report_history_gate_reasons={reasons}"),
        format!(
            "agent_tool_build_report_history_gate_clean={}",
            summary.is_clean
        ),
        format!(
            "agent_tool_build_report_history_gate_missing_requests={}",
            summary.missing_requests
        ),
        format!(
            "agent_tool_build_report_history_gate_unexpected_receipts={}",
            summary.unexpected_receipts
        ),
        format!(
            "agent_tool_build_report_history_gate_duplicate_receipts={}",
            summary.duplicate_receipts
        ),
        format!("agent_tool_build_report_history_gate_held={}", summary.held),
        format!(
            "agent_tool_build_report_history_gate_rejected={}",
            summary.rejected
        ),
        format!("agent_tool_build_report_history_gate_health={health_status:?}"),
    ]
}

fn tool_build_report_history_gate_record_telemetry(
    health_record: &ToolBuildReportSummaryHistoryRecord,
    gate_decision: &ToolBuildReportHistoryGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_tool_build_report_history_gate_record=true".to_owned(),
        format!(
            "agent_tool_build_report_history_gate_record_records={}",
            health_record.records()
        ),
        format!(
            "agent_tool_build_report_history_gate_record_open_tool_build_boundary={}",
            gate_decision.can_open_tool_build_boundary
        ),
        format!(
            "agent_tool_build_report_history_gate_record_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_tool_build_report_history_gate_record_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
    ];

    telemetry.extend(health_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn push_ordered_unique(target: &mut Vec<String>, values: impl IntoIterator<Item = String>) {
    let mut seen = target.iter().cloned().collect::<BTreeSet<_>>();
    for value in values {
        if seen.insert(value.clone()) {
            target.push(value);
        }
    }
}

fn rate(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::RecursiveAgentScheduler;

    struct FakeToolBuilder {
        built_ids: Vec<String>,
    }

    #[derive(Debug, Default)]
    struct FakeEngine {
        calls: usize,
    }

    impl EnginePort for FakeEngine {
        type Error = String;

        fn run_task(&mut self, task: &AgentTask) -> Result<AgentResult, Self::Error> {
            self.calls += 1;
            Ok(AgentResult::accepted(
                task,
                format!("ran {}", task.id),
                Vec::new(),
                AgentBudget::new(1, 1, 1),
            ))
        }
    }

    impl ToolBuildPort for FakeToolBuilder {
        type Error = ();

        fn build_tool(
            &mut self,
            request: &ToolBuildRequest,
        ) -> Result<ToolBuildReceipt, Self::Error> {
            self.built_ids.push(request.proposal_id.clone());
            Ok(ToolBuildReceipt::built(
                request.proposal_id.clone(),
                format!("artifact:{}", request.entrypoint),
            ))
        }
    }

    fn assert_rate_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected rate {actual} to be close to {expected}"
        );
    }

    #[test]
    fn memory_scope_preserves_caller_identity_fields() {
        let scope = AgentMemoryScope::new(" Tenant-A/Prod ", "Workspace:Runtime", "Session.42");

        assert_eq!(scope.tenant_id, "Tenant-A/Prod");
        assert_eq!(scope.workspace_id, "Workspace:Runtime");
        assert_eq!(scope.session_id, "Session.42");
    }

    #[test]
    fn memory_scope_defaults_blank_identity_fields() {
        let scope = AgentMemoryScope::new(" ", "", "\n");

        assert_eq!(scope, AgentMemoryScope::local_single_user());
    }

    #[test]
    fn model_route_request_trims_and_requires_layer_b_fields() {
        let task = AgentTask::new(
            "coding-model",
            AgentRole::Coder,
            "answer through model pool",
            AgentBudget::new(8, 1, 1),
        );
        let request = AgentModelRouteRequest::try_new(
            task,
            "  implement focused patch  ",
            AgentModelRouteProof::new(
                "  model-registry-v1 ",
                " qwen-local-fast ",
                " deterministic-inference-backend ",
                " default-model-pool ",
            ),
        )
        .unwrap();

        assert_eq!(request.prompt, "implement focused patch");
        assert_eq!(request.route.model_registry_id, "model-registry-v1");
        assert_eq!(request.route.model_profile_id, "qwen-local-fast");
        assert_eq!(
            request.route.inference_backend_id,
            "deterministic-inference-backend"
        );
        assert_eq!(request.route.model_pool_id, "default-model-pool");
    }

    #[test]
    fn routed_engine_refuses_missing_layer_b_route_before_engine_call() {
        let task = AgentTask::new(
            "coding-model",
            AgentRole::Coder,
            "answer through model pool",
            AgentBudget::new(8, 1, 1),
        );
        let request = AgentModelRouteRequest {
            task,
            prompt: "write the patch".to_owned(),
            route: AgentModelRouteProof::new(
                "model-registry-v1",
                "qwen-local-fast",
                "",
                "default-model-pool",
            ),
        };
        let mut engine = FakeEngine::default();

        let error = engine.run_routed_task(&request).unwrap_err();

        assert_eq!(
            error,
            AgentModelRouteRunError::Route(AgentModelRouteError::MissingField(
                "inference_backend_id"
            ))
        );
        assert_eq!(engine.calls, 0);
    }

    #[test]
    fn routed_engine_runs_after_model_registry_and_backend_route_proof() {
        let task = AgentTask::new(
            "coding-model",
            AgentRole::Coder,
            "answer through model pool",
            AgentBudget::new(8, 1, 1),
        );
        let request = AgentModelRouteRequest::try_new(
            task,
            "write the patch",
            AgentModelRouteProof::new(
                "model-registry-v1",
                "qwen-local-fast",
                "deterministic-inference-backend",
                "default-model-pool",
            ),
        )
        .unwrap();
        let mut engine = FakeEngine::default();

        let result = engine.run_routed_task(&request).unwrap();

        assert_eq!(result.task_id, "coding-model");
        assert_eq!(result.summary, "ran coding-model");
        assert_eq!(engine.calls, 1);
        assert_eq!(result.messages.len(), 1);
        let route_gate = &result.messages[0];
        assert_eq!(route_gate.kind, AgentMessageKind::Gate);
        assert_eq!(route_gate.topic, "layer_b_model_route");
        assert!(
            route_gate
                .content
                .contains("model_registry_id=model-registry-v1")
        );
        assert!(
            route_gate
                .content
                .contains("model_profile_id=qwen-local-fast")
        );
        assert!(
            route_gate
                .content
                .contains("inference_backend_id=deterministic-inference-backend")
        );
        assert!(
            route_gate
                .content
                .contains("model_pool_id=default-model-pool")
        );
        assert!(route_gate.content.contains("prompt_chars=15"));
        assert!(!route_gate.content.contains("write the patch"));
        assert!(
            route_gate
                .evidence
                .iter()
                .any(|line| line == "agent_model_route_prompt_chars=15")
        );
    }

    #[test]
    fn tool_build_requests_only_materialize_ready_rust_proposals() {
        let plan = ToolsmithPlan::new()
            .with_proposal(ToolProposal::new(
                "ready-rust",
                ToolIntent::TraceAnalysis,
                "rust",
                "tools/trace.rs",
                ToolBuildStatus::Ready,
            ))
            .with_proposal(ToolProposal::new(
                "held-rust",
                ToolIntent::Discovery,
                "rust",
                "tools/discovery.rs",
                ToolBuildStatus::Held,
            ))
            .with_proposal(ToolProposal::new(
                "ready-python",
                ToolIntent::BenchmarkGate,
                "python",
                "tools/bench.py",
                ToolBuildStatus::Ready,
            ))
            .with_proposal(ToolProposal::new(
                "rejected-rust",
                ToolIntent::RuntimeAdapter,
                "rust",
                "tools/runtime.rs",
                ToolBuildStatus::Rejected,
            ));

        let requests = ToolBuildRequest::ready_requests(&plan);

        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].proposal_id, "ready-rust");
        assert_eq!(requests[0].intent, ToolIntent::TraceAnalysis);
        assert_eq!(requests[0].entrypoint, "tools/trace.rs");
    }

    #[test]
    fn tool_build_port_receipts_preserve_request_identity() {
        let proposal = ToolProposal::new(
            "runtime-adapter",
            ToolIntent::RuntimeAdapter,
            "rust",
            "tools/runtime_adapter.rs",
            ToolBuildStatus::Ready,
        );
        let request = ToolBuildRequest::from_ready_proposal(&proposal).unwrap();
        let mut builder = FakeToolBuilder {
            built_ids: Vec::new(),
        };

        let receipt = builder.build_tool(&request).unwrap();

        assert_eq!(builder.built_ids, vec!["runtime-adapter"]);
        assert_eq!(receipt.proposal_id, "runtime-adapter");
        assert_eq!(receipt.status, ToolBuildStatus::Ready);
        assert_eq!(
            receipt.artifact,
            Some("artifact:tools/runtime_adapter.rs".to_owned())
        );
    }

    #[test]
    fn tool_build_requests_respect_toolsmith_history_gate() {
        use crate::evolution::{
            ToolsmithPlanHealthPolicy, ToolsmithPlanHistoryGate, ToolsmithPlanSummaryHistory,
            ToolsmithPlanSummaryHistoryRecorder,
        };

        let dirty_history = ToolsmithPlan::new()
            .with_proposal(ToolProposal::new(
                "non-rust",
                ToolIntent::Discovery,
                "python",
                "tools/discovery.py",
                ToolBuildStatus::Ready,
            ))
            .with_rejected_request("shell plugin request")
            .summary();
        let clean_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "ready-rust",
            ToolIntent::TraceAnalysis,
            "rust",
            "tools/trace.rs",
            ToolBuildStatus::Ready,
        ));
        let history_record = ToolsmithPlanSummaryHistoryRecorder::new().record_plan_with_health(
            ToolsmithPlanSummaryHistory::from_summaries(vec![dirty_history]),
            &clean_plan,
            ToolsmithPlanHealthPolicy::default(),
        );
        let gate = ToolsmithPlanHistoryGate::new().gate(&clean_plan, &history_record);

        assert_eq!(ToolBuildRequest::ready_requests(&clean_plan).len(), 1);
        assert!(gate.requires_repair_first);
        assert!(ToolBuildRequest::admitted_requests(&clean_plan, &gate).is_empty());
    }

    #[test]
    fn tool_build_requests_wait_for_final_evolution_admission() {
        use crate::evolution::{
            EvolutionAdmissionGate, EvolutionAdmissionHealthPolicy, EvolutionAdmissionHistoryGate,
            EvolutionAdmissionSummaryHistory, EvolutionAdmissionSummaryHistoryRecorder,
            ProcessRewardComponents, ProcessRewardReport, ProcessRewardReportHealthPolicy,
            ProcessRewardReportSummaryHistory, ProcessRewardReportSummaryHistoryRecorder,
            RewardAction, ToolsmithPlanHealthPolicy, ToolsmithPlanSummaryHistory,
            ToolsmithPlanSummaryHistoryRecorder,
        };

        let clean_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "ready-rust",
            ToolIntent::RuntimeAdapter,
            "rust",
            "tools/runtime_adapter.rs",
            ToolBuildStatus::Ready,
        ));
        let toolsmith_record = ToolsmithPlanSummaryHistoryRecorder::new()
            .record_plan_with_health_gate(
                ToolsmithPlanSummaryHistory::new(),
                &clean_plan,
                ToolsmithPlanHealthPolicy::default(),
            );
        let penalized_reward = ProcessRewardReport {
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
                &penalized_reward,
                ProcessRewardReportHealthPolicy::default(),
            );
        let admission = EvolutionAdmissionGate::new().gate(toolsmith_record, reward_record);
        let history_record = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health(
                EvolutionAdmissionSummaryHistory::new(),
                &admission,
                EvolutionAdmissionHealthPolicy::default(),
            );
        let final_gate = EvolutionAdmissionHistoryGate::new().gate(&admission, &history_record);

        assert_eq!(ToolBuildRequest::ready_requests(&clean_plan).len(), 1);
        assert!(!admission.can_promote_ready_proposals());
        assert!(final_gate.requires_repair_first);
        assert!(ToolBuildRequest::admitted_by_evolution(&clean_plan, &final_gate).is_empty());
    }

    #[test]
    fn tool_build_requests_open_after_clean_final_evolution_admission() {
        use crate::evolution::{
            EvolutionAdmissionGate, EvolutionAdmissionHealthPolicy, EvolutionAdmissionHistoryGate,
            EvolutionAdmissionSummaryHistory, EvolutionAdmissionSummaryHistoryRecorder,
            EvolutionSignal, ProcessRewardComponents, ProcessRewardReport,
            ProcessRewardReportHealthPolicy, ProcessRewardReportSummaryHistory,
            ProcessRewardReportSummaryHistoryRecorder, RewardAction, ToolsmithPlanHealthPolicy,
            ToolsmithPlanSummaryHistory, ToolsmithPlanSummaryHistoryRecorder,
        };

        let plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "ready-rust",
            ToolIntent::RuntimeAdapter,
            "rust",
            "tools/runtime_adapter.rs",
            ToolBuildStatus::Ready,
        ));
        let toolsmith_record = ToolsmithPlanSummaryHistoryRecorder::new()
            .record_plan_with_health_gate(
                ToolsmithPlanSummaryHistory::new(),
                &plan,
                ToolsmithPlanHealthPolicy::default(),
            );
        let reward_report = ProcessRewardReport {
            total: 0.86,
            components: ProcessRewardComponents {
                coordination: 0.86,
                reflection: 0.86,
                validation: 0.86,
                toolsmith: 0.86,
                recursion: 0.86,
                admission: 0.86,
            },
            action: RewardAction::Reinforce,
            notes: vec!["total:0.860:reinforce".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "toolsmith",
                "promote_runtime_adapter",
                "clean toolsmith admission",
                0.86,
            )],
        };
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &reward_report,
                ProcessRewardReportHealthPolicy::default(),
            );
        let admission = EvolutionAdmissionGate::new().gate(toolsmith_record, reward_record);
        let history_record = EvolutionAdmissionSummaryHistoryRecorder::new()
            .record_admission_with_health(
                EvolutionAdmissionSummaryHistory::new(),
                &admission,
                EvolutionAdmissionHealthPolicy::default(),
            );
        let final_gate = EvolutionAdmissionHistoryGate::new().gate(&admission, &history_record);

        let requests = ToolBuildRequest::admitted_by_evolution(&plan, &final_gate);

        assert!(!final_gate.requires_repair_first);
        assert!(final_gate.can_promote_ready_proposals);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].proposal_id, "ready-rust");
        assert_eq!(requests[0].intent, ToolIntent::RuntimeAdapter);
        assert_eq!(requests[0].entrypoint, "tools/runtime_adapter.rs");
    }

    #[test]
    fn tool_build_report_closes_clean_receipts() {
        let requests = vec![ToolBuildRequest {
            proposal_id: "ready-rust".to_owned(),
            intent: ToolIntent::RuntimeAdapter,
            rust_crate: "rust".to_owned(),
            entrypoint: "tools/runtime_adapter.rs".to_owned(),
            gate_notes: Vec::new(),
        }];
        let receipts = vec![ToolBuildReceipt::built(
            "ready-rust",
            "artifacts/runtime_adapter",
        )];

        let report = ToolBuildReport::from_requests_and_receipts(&requests, &receipts);

        assert_eq!(report.requested, 1);
        assert_eq!(report.received, 1);
        assert_eq!(report.built, 1);
        assert!(report.is_clean());
        assert!(!report.requires_repair_first());
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| line == "agent_tool_build_report_built=1")
        );
    }

    #[test]
    fn tool_build_report_repairs_missing_unexpected_and_held_receipts() {
        let requests = vec![
            ToolBuildRequest {
                proposal_id: "expected-built".to_owned(),
                intent: ToolIntent::RuntimeAdapter,
                rust_crate: "rust".to_owned(),
                entrypoint: "tools/runtime_adapter.rs".to_owned(),
                gate_notes: Vec::new(),
            },
            ToolBuildRequest {
                proposal_id: "missing-build".to_owned(),
                intent: ToolIntent::TraceAnalysis,
                rust_crate: "rust".to_owned(),
                entrypoint: "tools/trace.rs".to_owned(),
                gate_notes: Vec::new(),
            },
        ];
        let receipts = vec![
            ToolBuildReceipt::built("expected-built", "artifacts/runtime_adapter"),
            ToolBuildReceipt::held("unexpected-build", "no admitted request"),
            ToolBuildReceipt::built("expected-built", "artifacts/runtime_adapter_retry"),
        ];

        let report = ToolBuildReport::from_requests_and_receipts(&requests, &receipts);

        assert!(report.requires_repair_first());
        assert_eq!(report.built, 2);
        assert_eq!(report.held, 1);
        assert_eq!(report.missing_request_ids, vec!["missing-build"]);
        assert_eq!(report.unexpected_receipt_ids, vec!["unexpected-build"]);
        assert_eq!(report.duplicate_receipt_ids, vec!["expected-built"]);
        assert_eq!(
            report.diagnostics,
            vec!["unexpected-build:no admitted request"]
        );
    }

    #[test]
    fn clean_tool_build_report_summary_marks_reliable() {
        let requests = vec![ToolBuildRequest {
            proposal_id: "ready-rust".to_owned(),
            intent: ToolIntent::RuntimeAdapter,
            rust_crate: "rust".to_owned(),
            entrypoint: "tools/runtime_adapter.rs".to_owned(),
            gate_notes: Vec::new(),
        }];
        let receipts = vec![ToolBuildReceipt::built(
            "ready-rust",
            "artifacts/runtime_adapter",
        )];
        let report = ToolBuildReport::from_requests_and_receipts(&requests, &receipts);
        let summary = report.summary();

        let reliability = summary.reliability();

        assert_eq!(reliability.attempts, 1);
        assert_eq!(reliability.successes, 1);
        assert_eq!(reliability.issue_count, 0);
        assert_rate_close(reliability.success_rate, 1.0);
        assert_rate_close(reliability.issue_rate, 0.0);
        assert!(reliability.reliable);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "agent_tool_build_report_summary_success_rate=1.000")
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "agent_tool_build_report_summary_issue_rate=0.000")
        );
    }

    #[test]
    fn dirty_tool_build_report_summary_marks_unreliable() {
        let report = ToolBuildReport {
            requested: 2,
            received: 1,
            built: 1,
            held: 0,
            rejected: 0,
            missing_request_ids: vec!["missing-build".to_owned()],
            unexpected_receipt_ids: Vec::new(),
            duplicate_receipt_ids: Vec::new(),
            diagnostics: Vec::new(),
            telemetry: Vec::new(),
        };
        let summary = report.summary();

        let reliability = summary.reliability();

        assert_eq!(reliability.attempts, 2);
        assert_eq!(reliability.successes, 1);
        assert_eq!(reliability.issue_count, 1);
        assert_rate_close(reliability.success_rate, 0.5);
        assert_rate_close(reliability.issue_rate, 0.5);
        assert!(!reliability.reliable);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "agent_tool_build_report_summary_reliable=false")
        );
    }

    #[test]
    fn tool_build_report_dashboard_summarizes_reliability() {
        let clean_summary = ToolBuildReport {
            requested: 1,
            received: 1,
            built: 1,
            held: 0,
            rejected: 0,
            missing_request_ids: Vec::new(),
            unexpected_receipt_ids: Vec::new(),
            duplicate_receipt_ids: Vec::new(),
            diagnostics: Vec::new(),
            telemetry: Vec::new(),
        }
        .summary();
        let dirty_summary = ToolBuildReport {
            requested: 2,
            received: 1,
            built: 1,
            held: 0,
            rejected: 0,
            missing_request_ids: vec!["missing-build".to_owned()],
            unexpected_receipt_ids: Vec::new(),
            duplicate_receipt_ids: Vec::new(),
            diagnostics: Vec::new(),
            telemetry: Vec::new(),
        }
        .summary();
        let dashboard =
            ToolBuildReportSummaryHistory::from_summaries(vec![clean_summary, dirty_summary])
                .dashboard();

        let reliability = dashboard.reliability();

        assert_eq!(reliability.attempts, 3);
        assert_eq!(reliability.successes, 2);
        assert_eq!(reliability.issue_count, 1);
        assert_rate_close(reliability.success_rate, 2.0 / 3.0);
        assert_rate_close(reliability.issue_rate, 1.0 / 3.0);
        assert!(!reliability.reliable);
        assert!(
            dashboard
                .telemetry
                .iter()
                .any(|line| line == "agent_tool_build_report_dashboard_success_rate=0.667")
        );
        assert!(
            dashboard
                .telemetry
                .iter()
                .any(|line| line == "agent_tool_build_report_dashboard_issue_rate=0.333")
        );
    }

    #[test]
    fn empty_tool_build_report_dashboard_is_not_reliable() {
        let dashboard = ToolBuildReportSummaryHistory::new().dashboard();

        let reliability = dashboard.reliability();

        assert_eq!(reliability.attempts, 0);
        assert_eq!(reliability.successes, 0);
        assert_eq!(reliability.issue_count, 0);
        assert_rate_close(reliability.success_rate, 0.0);
        assert_rate_close(reliability.issue_rate, 0.0);
        assert!(!reliability.reliable);
        assert!(
            dashboard
                .telemetry
                .iter()
                .any(|line| line == "agent_tool_build_report_dashboard_reliable=false")
        );
    }

    #[test]
    fn clean_tool_build_report_history_marks_stable() {
        let requests = vec![ToolBuildRequest {
            proposal_id: "ready-rust".to_owned(),
            intent: ToolIntent::RuntimeAdapter,
            rust_crate: "rust".to_owned(),
            entrypoint: "tools/runtime_adapter.rs".to_owned(),
            gate_notes: Vec::new(),
        }];
        let report = ToolBuildReport::from_requests_and_receipts(
            &requests,
            &[ToolBuildReceipt::built(
                "ready-rust",
                "artifacts/runtime_adapter",
            )],
        );

        let record = ToolBuildReportSummaryHistoryRecorder::new().record_report_with_health(
            ToolBuildReportSummaryHistory::new(),
            &report,
            ToolBuildReportHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, ToolBuildReportHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.clean_records, 1);
        assert_eq!(record.dashboard.repair_records, 0);
        assert_eq!(record.dashboard.clean_rate, 1.0);
        assert_eq!(record.history.summaries()[0].built, 1);
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_tool_build_report_history_record_status=Stable" })
        );
    }

    #[test]
    fn empty_tool_build_report_history_watches() {
        let health =
            ToolBuildReportSummaryHistory::new().health(ToolBuildReportHealthPolicy::default());

        assert_eq!(health.status, ToolBuildReportHealthStatus::Watch);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(health.dashboard.total_records, 0);
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "tool_build_report_history_empty")
        );
    }

    #[test]
    fn dirty_tool_build_report_history_repairs_all_receipt_pressure() {
        let requests = vec![
            ToolBuildRequest {
                proposal_id: "expected-built".to_owned(),
                intent: ToolIntent::RuntimeAdapter,
                rust_crate: "rust".to_owned(),
                entrypoint: "tools/runtime_adapter.rs".to_owned(),
                gate_notes: Vec::new(),
            },
            ToolBuildRequest {
                proposal_id: "missing-build".to_owned(),
                intent: ToolIntent::TraceAnalysis,
                rust_crate: "rust".to_owned(),
                entrypoint: "tools/trace.rs".to_owned(),
                gate_notes: Vec::new(),
            },
        ];
        let receipts = vec![
            ToolBuildReceipt::built("expected-built", "artifacts/runtime_adapter"),
            ToolBuildReceipt::held("unexpected-build", "no admitted request"),
            ToolBuildReceipt::rejected("expected-built", "duplicate rejected"),
        ];
        let report = ToolBuildReport::from_requests_and_receipts(&requests, &receipts);

        let record = ToolBuildReportSummaryHistoryRecorder::new().record_report_with_health(
            ToolBuildReportSummaryHistory::new(),
            &report,
            ToolBuildReportHealthPolicy::default(),
        );

        assert_eq!(record.health.status, ToolBuildReportHealthStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(record.dashboard.repair_records, 1);
        assert_eq!(record.dashboard.missing_requests, 1);
        assert_eq!(record.dashboard.unexpected_receipts, 1);
        assert_eq!(record.dashboard.duplicate_receipts, 1);
        assert_eq!(record.dashboard.held, 1);
        assert_eq!(record.dashboard.rejected, 1);
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "tool_build_report_missing_requests=1>0" })
        );
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "tool_build_report_unexpected_receipts=1>0" })
        );
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "tool_build_report_duplicate_receipts=1>0" })
        );
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| reason == "tool_build_report_held_receipts=1>0")
        );
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "tool_build_report_rejected_receipts=1>0" })
        );
    }

    #[test]
    fn tool_build_report_recorder_appends_report_and_computes_health() {
        let clean_summary = ToolBuildReportSummary {
            requested: 1,
            received: 1,
            built: 1,
            held: 0,
            rejected: 0,
            missing_requests: 0,
            unexpected_receipts: 0,
            duplicate_receipts: 0,
            diagnostics: 0,
            is_clean: true,
            telemetry: Vec::new(),
        };
        let dirty_report = ToolBuildReport {
            requested: 1,
            received: 0,
            built: 0,
            held: 0,
            rejected: 0,
            missing_request_ids: vec!["missing-build".to_owned()],
            unexpected_receipt_ids: Vec::new(),
            duplicate_receipt_ids: Vec::new(),
            diagnostics: Vec::new(),
            telemetry: Vec::new(),
        };
        let history = ToolBuildReportSummaryHistory::from_summaries(vec![clean_summary]);

        let record = ToolBuildReportSummaryHistoryRecorder::new().record_report_with_health(
            history,
            &dirty_report,
            ToolBuildReportHealthPolicy::default(),
        );

        assert_eq!(record.records(), 2);
        assert_eq!(record.appended_summary.missing_requests, 1);
        assert_eq!(record.dashboard.clean_records, 1);
        assert_eq!(record.dashboard.repair_records, 1);
        assert_eq!(record.health.status, ToolBuildReportHealthStatus::Repair);
        assert_eq!(
            record.history.summaries()[1].telemetry[0],
            "agent_tool_build_report_summary=true"
        );
    }

    #[test]
    fn tool_build_report_history_gate_opens_clean_boundaries() {
        let requests = vec![ToolBuildRequest {
            proposal_id: "ready-rust".to_owned(),
            intent: ToolIntent::RuntimeAdapter,
            rust_crate: "rust".to_owned(),
            entrypoint: "tools/runtime_adapter.rs".to_owned(),
            gate_notes: Vec::new(),
        }];
        let report = ToolBuildReport::from_requests_and_receipts(
            &requests,
            &[ToolBuildReceipt::built(
                "ready-rust",
                "artifacts/runtime_adapter",
            )],
        );

        let record = ToolBuildReportSummaryHistoryRecorder::new().record_report_with_health_gate(
            ToolBuildReportSummaryHistory::new(),
            &report,
            ToolBuildReportHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.can_open_tool_build_boundary());
        assert!(record.can_promote_memory_note());
        assert!(record.can_promote_adaptive_state());
        assert!(record.can_finalize_eval());
        assert!(record.gate_decision.is_tool_build_boundary_open());
        assert!(record.gate_decision.is_promotion_safe());
        assert!(record.gate_decision.repair_tasks.is_empty());
        assert!(record.gate_decision.reasons.is_empty());
    }

    #[test]
    fn tool_build_report_history_gate_repairs_dirty_receipts() {
        let report = ToolBuildReport {
            requested: 1,
            received: 0,
            built: 0,
            held: 0,
            rejected: 0,
            missing_request_ids: vec!["missing-build".to_owned()],
            unexpected_receipt_ids: Vec::new(),
            duplicate_receipt_ids: Vec::new(),
            diagnostics: Vec::new(),
            telemetry: Vec::new(),
        };

        let record = ToolBuildReportSummaryHistoryRecorder::new().record_report_with_health_gate(
            ToolBuildReportSummaryHistory::new(),
            &report,
            ToolBuildReportHealthPolicy::default(),
        );

        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(!record.can_open_tool_build_boundary());
        assert!(!record.can_promote_memory_note());
        assert!(!record.can_promote_adaptive_state());
        assert!(!record.can_finalize_eval());
        assert_eq!(
            record
                .gate_decision
                .repair_tasks
                .iter()
                .map(|task| (task.id.as_str(), task.role.clone(), task.lane.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (
                    "tool-build-report-repair-0",
                    AgentRole::Planner,
                    "tool-build-report-repair"
                ),
                (
                    "tool-build-report-repair-1",
                    AgentRole::Planner,
                    "tool-build-report-repair"
                )
            ]
        );
        assert!(
            record
                .gate_decision
                .reasons
                .iter()
                .any(|reason| { reason == "tool_build_report_missing_requests=1" })
        );
        assert!(record.gate_decision.reasons.iter().any(|reason| {
            reason == "tool_build_report_history:tool_build_report_missing_requests=1>0"
        }));
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_tool_build_report_history_gate_record_requires_repair_first=true"
        }));
    }

    #[test]
    fn tool_build_report_repair_queue_blocks_memory_adaptive_and_eval_tasks() {
        let requests = vec![
            ToolBuildRequest {
                proposal_id: "expected-built".to_owned(),
                intent: ToolIntent::RuntimeAdapter,
                rust_crate: "rust".to_owned(),
                entrypoint: "tools/runtime_adapter.rs".to_owned(),
                gate_notes: Vec::new(),
            },
            ToolBuildRequest {
                proposal_id: "missing-build".to_owned(),
                intent: ToolIntent::TraceAnalysis,
                rust_crate: "rust".to_owned(),
                entrypoint: "tools/trace.rs".to_owned(),
                gate_notes: Vec::new(),
            },
        ];
        let receipts = vec![
            ToolBuildReceipt::built("expected-built", "artifacts/runtime_adapter"),
            ToolBuildReceipt::held("unexpected-build", "no admitted request"),
            ToolBuildReceipt::rejected("expected-built", "duplicate rejected"),
        ];
        let report = ToolBuildReport::from_requests_and_receipts(&requests, &receipts);
        let record = ToolBuildReportSummaryHistoryRecorder::new().record_report_with_health_gate(
            ToolBuildReportSummaryHistory::new(),
            &report,
            ToolBuildReportHealthPolicy::default(),
        );
        let next_queue = AgentTaskQueue::from_tasks(vec![
            AgentTask::new(
                "memory-note",
                AgentRole::MemoryCurator,
                "promote memory note after clean tool build receipts",
                AgentBudget::new(4, 1, 1),
            )
            .with_priority(10),
            AgentTask::new(
                "adaptive-state",
                AgentRole::Planner,
                "write adaptive state after clean tool build receipts",
                AgentBudget::new(4, 1, 1),
            )
            .with_priority(10),
            AgentTask::new(
                "finalize-eval",
                AgentRole::Reviewer,
                "finalize eval after clean tool build receipts",
                AgentBudget::new(4, 1, 1),
            )
            .with_priority(10),
        ]);

        let gated_queue = record.gate_decision.repair_first_queue(next_queue);
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let schedule = RecursiveAgentScheduler::new(16).plan(gated_queue.tasks());

        assert!(record.requires_repair_first());
        assert!(!record.can_promote_memory_note());
        assert!(!record.can_promote_adaptive_state());
        assert!(!record.can_finalize_eval());
        assert_eq!(record.gate_decision.repair_tasks.len(), 10);
        for task_id in ["adaptive-state", "finalize-eval", "memory-note"] {
            let task = gated_queue
                .tasks()
                .into_iter()
                .find(|task| task.id == task_id)
                .expect("business task should remain behind tool-build repair");
            assert_eq!(task.dependencies, repair_task_ids);
        }
        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.waves[0].task_ids, repair_task_ids);
        assert_eq!(
            schedule.waves[1].task_ids,
            vec!["adaptive-state", "finalize-eval", "memory-note"]
        );
    }

    #[test]
    fn tool_build_report_history_gate_blocks_clean_report_on_repair_history() {
        let dirty_history =
            ToolBuildReportSummaryHistory::from_summaries(vec![ToolBuildReportSummary {
                requested: 1,
                received: 0,
                built: 0,
                held: 0,
                rejected: 0,
                missing_requests: 1,
                unexpected_receipts: 0,
                duplicate_receipts: 0,
                diagnostics: 0,
                is_clean: false,
                telemetry: Vec::new(),
            }]);
        let clean_report = ToolBuildReport {
            requested: 1,
            received: 1,
            built: 1,
            held: 0,
            rejected: 0,
            missing_request_ids: Vec::new(),
            unexpected_receipt_ids: Vec::new(),
            duplicate_receipt_ids: Vec::new(),
            diagnostics: Vec::new(),
            telemetry: Vec::new(),
        };

        let record = ToolBuildReportSummaryHistoryRecorder::new().record_report_with_health_gate(
            dirty_history,
            &clean_report,
            ToolBuildReportHealthPolicy::default(),
        );

        assert_eq!(record.records(), 2);
        assert!(record.requires_repair_first());
        assert!(!record.can_open_tool_build_boundary());
        assert!(!record.can_promote_memory_note());
        assert_eq!(record.gate_decision.report_summary.built, 1);
        assert!(record.gate_decision.reasons.iter().any(|reason| {
            reason == "tool_build_report_history:tool_build_report_missing_requests=1>0"
        }));
    }
}
