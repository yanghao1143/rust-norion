use crate::control::{AdaptiveStateCandidate, AgentBusinessLoopPlan};
use crate::ledger::AgentCycleLedgerAdmissionStatus;
use crate::step::AgentClosedLoopExecutionHealthStatus;
use crate::{
    budget::AgentBudget,
    task::{AgentRole, AgentTask, AgentTaskQueue},
};

#[derive(Debug, Clone, PartialEq)]
pub enum AgentServiceCommand {
    PromoteAdaptiveState(AdaptiveStateCandidate),
    HoldBusinessLoop {
        reasons: Vec<String>,
    },
    OpenRepairMode {
        reasons: Vec<String>,
    },
    RunRustValidation {
        commands: Vec<AgentRustValidationCommand>,
        reasons: Vec<String>,
    },
    EnqueueTasks(AgentTaskQueue),
    EmitTelemetry(String),
}

impl AgentServiceCommand {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::PromoteAdaptiveState(_) => "promote_adaptive_state",
            Self::HoldBusinessLoop { .. } => "hold_business_loop",
            Self::OpenRepairMode { .. } => "open_repair_mode",
            Self::RunRustValidation { .. } => "run_rust_validation",
            Self::EnqueueTasks(_) => "enqueue_tasks",
            Self::EmitTelemetry(_) => "emit_telemetry",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRustValidationCommand {
    Format,
    Check,
    Test,
    Benchmark,
}

impl AgentRustValidationCommand {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Format => "cargo_fmt",
            Self::Check => "cargo_check",
            Self::Test => "cargo_test",
            Self::Benchmark => "cargo_benchmark",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceCommandPlan {
    pub commands: Vec<AgentServiceCommand>,
}

impl AgentServiceCommandPlan {
    pub fn command_kinds(&self) -> Vec<&'static str> {
        self.commands
            .iter()
            .map(AgentServiceCommand::kind)
            .collect()
    }

    pub fn requires_adaptive_state_write(&self) -> bool {
        self.commands
            .iter()
            .any(|command| matches!(command, AgentServiceCommand::PromoteAdaptiveState(_)))
    }

    pub fn repair_mode_requested(&self) -> bool {
        self.commands
            .iter()
            .any(|command| matches!(command, AgentServiceCommand::OpenRepairMode { .. }))
    }

    pub fn summary(&self) -> AgentServiceCommandPlanSummary {
        AgentServiceCommandPlanSummary::from_plan(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentServiceCommandPlanSummary {
    pub command_count: usize,
    pub command_kinds: Vec<String>,
    pub requires_adaptive_state_write: bool,
    pub repair_mode_requested: bool,
    pub hold_requested: bool,
    pub enqueue_commands: usize,
    pub enqueued_tasks: usize,
    pub reason_count: usize,
    pub memory_promotion_reason_count: usize,
    pub tool_build_reason_count: usize,
    pub rust_validation_commands: usize,
    pub telemetry_commands: usize,
    pub telemetry: Vec<String>,
}

impl AgentServiceCommandPlanSummary {
    pub fn from_plan(plan: &AgentServiceCommandPlan) -> Self {
        let command_kinds = plan
            .commands
            .iter()
            .map(|command| command.kind().to_owned())
            .collect::<Vec<_>>();
        let enqueue_commands = plan
            .commands
            .iter()
            .filter(|command| matches!(command, AgentServiceCommand::EnqueueTasks(_)))
            .count();
        let enqueued_tasks = plan
            .commands
            .iter()
            .map(|command| match command {
                AgentServiceCommand::EnqueueTasks(queue) => queue.task_ids().len(),
                _ => 0,
            })
            .sum();
        let telemetry_commands = plan
            .commands
            .iter()
            .filter(|command| matches!(command, AgentServiceCommand::EmitTelemetry(_)))
            .count();
        let reasons = command_reasons(&plan.commands);
        let reason_count = reasons.len();
        let memory_promotion_reason_count = reasons
            .iter()
            .filter(|reason| reason.starts_with("memory_promotion"))
            .count();
        let tool_build_reason_count = reasons
            .iter()
            .filter(|reason| reason.starts_with("tool_build"))
            .count();
        let rust_validation_commands = plan
            .commands
            .iter()
            .map(|command| match command {
                AgentServiceCommand::RunRustValidation { commands, .. } => commands.len(),
                _ => 0,
            })
            .sum();
        let requires_adaptive_state_write = plan.requires_adaptive_state_write();
        let repair_mode_requested = plan.repair_mode_requested();
        let hold_requested = plan
            .commands
            .iter()
            .any(|command| matches!(command, AgentServiceCommand::HoldBusinessLoop { .. }));
        let telemetry = service_command_plan_summary_telemetry(
            plan.commands.len(),
            requires_adaptive_state_write,
            repair_mode_requested,
            hold_requested,
            enqueue_commands,
            enqueued_tasks,
            reason_count,
            memory_promotion_reason_count,
            tool_build_reason_count,
            rust_validation_commands,
            telemetry_commands,
        );

        Self {
            command_count: plan.commands.len(),
            command_kinds,
            requires_adaptive_state_write,
            repair_mode_requested,
            hold_requested,
            enqueue_commands,
            enqueued_tasks,
            reason_count,
            memory_promotion_reason_count,
            tool_build_reason_count,
            rust_validation_commands,
            telemetry_commands,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentServiceCommandPlanSummaryHistory {
    summaries: Vec<AgentServiceCommandPlanSummary>,
}

impl AgentServiceCommandPlanSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentServiceCommandPlanSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentServiceCommandPlanSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentServiceCommandPlanSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentServiceCommandPlanSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentServiceCommandPlanDashboard {
        AgentServiceCommandPlanDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentServiceCommandPlanHealthPolicy,
    ) -> AgentServiceCommandPlanHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceCommandPlanDashboard {
    pub total_plans: usize,
    pub command_count: usize,
    pub adaptive_write_plans: usize,
    pub repair_mode_plans: usize,
    pub hold_plans: usize,
    pub repair_or_hold_plans: usize,
    pub enqueue_plans: usize,
    pub enqueue_command_count: usize,
    pub enqueued_task_count: usize,
    pub reason_count: usize,
    pub memory_promotion_reason_count: usize,
    pub memory_promotion_reason_plans: usize,
    pub tool_build_reason_count: usize,
    pub tool_build_reason_plans: usize,
    pub telemetry_command_count: usize,
    pub adaptive_write_rate: f32,
    pub repair_or_hold_rate: f32,
    pub enqueue_rate: f32,
    pub latest_command_kinds: Vec<String>,
    pub latest_requires_adaptive_state_write: Option<bool>,
    pub latest_repair_mode_requested: Option<bool>,
    pub latest_hold_requested: Option<bool>,
    pub telemetry: Vec<String>,
}

impl AgentServiceCommandPlanDashboard {
    pub fn from_summaries(summaries: &[AgentServiceCommandPlanSummary]) -> Self {
        let total_plans = summaries.len();
        let command_count = summaries
            .iter()
            .map(|summary| summary.command_count)
            .sum::<usize>();
        let adaptive_write_plans = summaries
            .iter()
            .filter(|summary| summary.requires_adaptive_state_write)
            .count();
        let repair_mode_plans = summaries
            .iter()
            .filter(|summary| summary.repair_mode_requested)
            .count();
        let hold_plans = summaries
            .iter()
            .filter(|summary| summary.hold_requested)
            .count();
        let repair_or_hold_plans = summaries
            .iter()
            .filter(|summary| summary.repair_mode_requested || summary.hold_requested)
            .count();
        let enqueue_plans = summaries
            .iter()
            .filter(|summary| summary.enqueue_commands > 0)
            .count();
        let enqueue_command_count = summaries
            .iter()
            .map(|summary| summary.enqueue_commands)
            .sum::<usize>();
        let enqueued_task_count = summaries
            .iter()
            .map(|summary| summary.enqueued_tasks)
            .sum::<usize>();
        let reason_count = summaries
            .iter()
            .map(|summary| summary.reason_count)
            .sum::<usize>();
        let memory_promotion_reason_count = summaries
            .iter()
            .map(|summary| summary.memory_promotion_reason_count)
            .sum::<usize>();
        let memory_promotion_reason_plans = summaries
            .iter()
            .filter(|summary| summary.memory_promotion_reason_count > 0)
            .count();
        let tool_build_reason_count = summaries
            .iter()
            .map(|summary| summary.tool_build_reason_count)
            .sum::<usize>();
        let tool_build_reason_plans = summaries
            .iter()
            .filter(|summary| summary.tool_build_reason_count > 0)
            .count();
        let telemetry_command_count = summaries
            .iter()
            .map(|summary| summary.telemetry_commands)
            .sum::<usize>();
        let adaptive_write_rate = service_execution_rate(adaptive_write_plans, total_plans);
        let repair_or_hold_rate = service_execution_rate(repair_or_hold_plans, total_plans);
        let enqueue_rate = service_execution_rate(enqueue_plans, total_plans);
        let latest = summaries.last();
        let latest_command_kinds = latest
            .map(|summary| summary.command_kinds.clone())
            .unwrap_or_default();
        let latest_requires_adaptive_state_write =
            latest.map(|summary| summary.requires_adaptive_state_write);
        let latest_repair_mode_requested = latest.map(|summary| summary.repair_mode_requested);
        let latest_hold_requested = latest.map(|summary| summary.hold_requested);
        let telemetry = service_command_plan_dashboard_telemetry(
            total_plans,
            command_count,
            adaptive_write_plans,
            repair_mode_plans,
            hold_plans,
            enqueue_plans,
            enqueue_command_count,
            enqueued_task_count,
            reason_count,
            memory_promotion_reason_count,
            memory_promotion_reason_plans,
            tool_build_reason_count,
            tool_build_reason_plans,
            telemetry_command_count,
            adaptive_write_rate,
            repair_or_hold_rate,
            enqueue_rate,
        );

        Self {
            total_plans,
            command_count,
            adaptive_write_plans,
            repair_mode_plans,
            hold_plans,
            repair_or_hold_plans,
            enqueue_plans,
            enqueue_command_count,
            enqueued_task_count,
            reason_count,
            memory_promotion_reason_count,
            memory_promotion_reason_plans,
            tool_build_reason_count,
            tool_build_reason_plans,
            telemetry_command_count,
            adaptive_write_rate,
            repair_or_hold_rate,
            enqueue_rate,
            latest_command_kinds,
            latest_requires_adaptive_state_write,
            latest_repair_mode_requested,
            latest_hold_requested,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_plans == 0
    }

    pub fn is_clean(&self) -> bool {
        self.total_plans > 0 && self.repair_or_hold_plans == 0
    }

    pub fn has_repair_or_hold_pressure(&self) -> bool {
        self.repair_or_hold_plans > 0
    }

    pub fn health(
        &self,
        policy: AgentServiceCommandPlanHealthPolicy,
    ) -> AgentServiceCommandPlanHealth {
        AgentServiceCommandPlanHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentServiceCommandPlanHealthPolicy {
    pub maximum_repair_or_hold_rate: f32,
    pub maximum_enqueued_tasks: usize,
    pub maximum_memory_promotion_reason_plans: usize,
    pub maximum_tool_build_reason_plans: usize,
    pub maximum_adaptive_write_rate: f32,
}

impl Default for AgentServiceCommandPlanHealthPolicy {
    fn default() -> Self {
        Self {
            maximum_repair_or_hold_rate: 0.0,
            maximum_enqueued_tasks: usize::MAX,
            maximum_memory_promotion_reason_plans: usize::MAX,
            maximum_tool_build_reason_plans: usize::MAX,
            maximum_adaptive_write_rate: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceCommandPlanHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentServiceCommandPlanDashboard,
}

impl AgentServiceCommandPlanHealth {
    pub fn from_dashboard(
        dashboard: AgentServiceCommandPlanDashboard,
        policy: AgentServiceCommandPlanHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("service_command_plan_history_empty".to_owned());
        }

        if dashboard.repair_or_hold_rate > policy.maximum_repair_or_hold_rate {
            repair_reasons.push(format!(
                "service_command_plan_repair_or_hold_rate={:.3}>{}",
                dashboard.repair_or_hold_rate, policy.maximum_repair_or_hold_rate
            ));
        }

        if dashboard.enqueued_task_count > policy.maximum_enqueued_tasks {
            watch_reasons.push(format!(
                "service_command_plan_enqueued_tasks={}>{}",
                dashboard.enqueued_task_count, policy.maximum_enqueued_tasks
            ));
        }

        if dashboard.memory_promotion_reason_plans > policy.maximum_memory_promotion_reason_plans {
            repair_reasons.push(format!(
                "service_command_plan_memory_promotion_reason_plans={}>{}",
                dashboard.memory_promotion_reason_plans,
                policy.maximum_memory_promotion_reason_plans
            ));
        }

        if dashboard.tool_build_reason_plans > policy.maximum_tool_build_reason_plans {
            repair_reasons.push(format!(
                "service_command_plan_tool_build_reason_plans={}>{}",
                dashboard.tool_build_reason_plans, policy.maximum_tool_build_reason_plans
            ));
        }

        if dashboard.adaptive_write_rate > policy.maximum_adaptive_write_rate {
            repair_reasons.push(format!(
                "service_command_plan_adaptive_write_rate={:.3}>{}",
                dashboard.adaptive_write_rate, policy.maximum_adaptive_write_rate
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentClosedLoopExecutionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentClosedLoopExecutionHealthStatus::Watch, watch_reasons)
        } else {
            (AgentClosedLoopExecutionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentClosedLoopExecutionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceCommandPlanSummaryHistoryRecord {
    pub history: AgentServiceCommandPlanSummaryHistory,
    pub appended_summary: AgentServiceCommandPlanSummary,
    pub dashboard: AgentServiceCommandPlanDashboard,
    pub health: AgentServiceCommandPlanHealth,
    pub telemetry: Vec<String>,
}

impl AgentServiceCommandPlanSummaryHistoryRecord {
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

#[derive(Debug, Clone, Default)]
pub struct AgentServiceCommandPlanSummaryHistoryRecorder;

impl AgentServiceCommandPlanSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary(
        &self,
        mut history: AgentServiceCommandPlanSummaryHistory,
        summary: AgentServiceCommandPlanSummary,
        policy: AgentServiceCommandPlanHealthPolicy,
    ) -> AgentServiceCommandPlanSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_command_plan_history_record_telemetry(&dashboard, &health);

        AgentServiceCommandPlanSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_plan(
        &self,
        history: AgentServiceCommandPlanSummaryHistory,
        plan: &AgentServiceCommandPlan,
        policy: AgentServiceCommandPlanHealthPolicy,
    ) -> AgentServiceCommandPlanSummaryHistoryRecord {
        self.record_summary(history, plan.summary(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentServiceCommandStatus {
    Applied,
    Failed,
    Skipped,
}

impl AgentServiceCommandStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentServiceCommandReceipt {
    pub command_kind: String,
    pub status: AgentServiceCommandStatus,
    pub detail: String,
}

impl AgentServiceCommandReceipt {
    pub fn applied(command: &AgentServiceCommand, detail: impl Into<String>) -> Self {
        Self::new(command.kind(), AgentServiceCommandStatus::Applied, detail)
    }

    pub fn failed(command: &AgentServiceCommand, detail: impl Into<String>) -> Self {
        Self::new(command.kind(), AgentServiceCommandStatus::Failed, detail)
    }

    pub fn skipped(command: &AgentServiceCommand, detail: impl Into<String>) -> Self {
        Self::new(command.kind(), AgentServiceCommandStatus::Skipped, detail)
    }

    pub fn new(
        command_kind: impl Into<String>,
        status: AgentServiceCommandStatus,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            command_kind: command_kind.into(),
            status,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentServiceCommandAudit {
    pub expected: Vec<String>,
    pub receipts: Vec<AgentServiceCommandReceipt>,
    pub missing: Vec<String>,
    pub failed: Vec<AgentServiceCommandReceipt>,
    pub skipped: Vec<AgentServiceCommandReceipt>,
}

impl AgentServiceCommandAudit {
    pub fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.failed.is_empty() && self.skipped.is_empty()
    }

    pub fn blocked_reasons(&self) -> Vec<String> {
        let mut reasons = self
            .missing
            .iter()
            .map(|kind| format!("service_command_missing={kind}"))
            .collect::<Vec<_>>();
        reasons.extend(self.failed.iter().map(|receipt| {
            format!(
                "service_command_failed={}:{}",
                receipt.command_kind, receipt.detail
            )
        }));
        reasons.extend(self.skipped.iter().map(|receipt| {
            format!(
                "service_command_skipped={}:{}",
                receipt.command_kind, receipt.detail
            )
        }));
        reasons
    }

    pub fn summary(&self) -> AgentServiceCommandAuditSummary {
        AgentServiceCommandAuditSummary::from_audit(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentServiceCommandAuditSummary {
    pub expected_commands: usize,
    pub receipts: usize,
    pub missing_commands: usize,
    pub failed_commands: usize,
    pub skipped_commands: usize,
    pub clean: bool,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceCommandAuditSummary {
    pub fn from_audit(audit: &AgentServiceCommandAudit) -> Self {
        let blocked_reasons = audit.blocked_reasons();
        let clean = audit.is_clean();
        let telemetry = service_command_audit_summary_telemetry(
            audit.expected.len(),
            audit.receipts.len(),
            audit.missing.len(),
            audit.failed.len(),
            audit.skipped.len(),
            clean,
            blocked_reasons.len(),
        );

        Self {
            expected_commands: audit.expected.len(),
            receipts: audit.receipts.len(),
            missing_commands: audit.missing.len(),
            failed_commands: audit.failed.len(),
            skipped_commands: audit.skipped.len(),
            clean,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentServiceCommandAuditSummaryHistory {
    summaries: Vec<AgentServiceCommandAuditSummary>,
}

impl AgentServiceCommandAuditSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentServiceCommandAuditSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentServiceCommandAuditSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentServiceCommandAuditSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentServiceCommandAuditSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentServiceCommandAuditDashboard {
        AgentServiceCommandAuditDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentServiceCommandAuditHealthPolicy,
    ) -> AgentServiceCommandAuditHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceCommandAuditDashboard {
    pub total_audits: usize,
    pub clean_audits: usize,
    pub dirty_audits: usize,
    pub clean_rate: f32,
    pub expected_command_count: usize,
    pub receipt_count: usize,
    pub missing_command_count: usize,
    pub failed_command_count: usize,
    pub skipped_command_count: usize,
    pub drift_event_count: usize,
    pub blocked_reason_count: usize,
    pub drift_rate: f32,
    pub latest_clean: Option<bool>,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceCommandAuditDashboard {
    pub fn from_summaries(summaries: &[AgentServiceCommandAuditSummary]) -> Self {
        let total_audits = summaries.len();
        let clean_audits = summaries.iter().filter(|summary| summary.clean).count();
        let dirty_audits = total_audits.saturating_sub(clean_audits);
        let expected_command_count = summaries
            .iter()
            .map(|summary| summary.expected_commands)
            .sum::<usize>();
        let receipt_count = summaries
            .iter()
            .map(|summary| summary.receipts)
            .sum::<usize>();
        let missing_command_count = summaries
            .iter()
            .map(|summary| summary.missing_commands)
            .sum::<usize>();
        let failed_command_count = summaries
            .iter()
            .map(|summary| summary.failed_commands)
            .sum::<usize>();
        let skipped_command_count = summaries
            .iter()
            .map(|summary| summary.skipped_commands)
            .sum::<usize>();
        let drift_event_count =
            missing_command_count + failed_command_count + skipped_command_count;
        let blocked_reason_count = summaries
            .iter()
            .map(|summary| summary.blocked_reasons.len())
            .sum::<usize>();
        let clean_rate = service_execution_rate(clean_audits, total_audits);
        let drift_rate = service_execution_rate(drift_event_count, expected_command_count);
        let latest = summaries.last();
        let latest_clean = latest.map(|summary| summary.clean);
        let latest_blocked_reasons = latest
            .map(|summary| summary.blocked_reasons.clone())
            .unwrap_or_default();
        let telemetry = service_command_audit_dashboard_telemetry(
            total_audits,
            clean_audits,
            dirty_audits,
            expected_command_count,
            receipt_count,
            drift_event_count,
            blocked_reason_count,
            clean_rate,
            drift_rate,
        );

        Self {
            total_audits,
            clean_audits,
            dirty_audits,
            clean_rate,
            expected_command_count,
            receipt_count,
            missing_command_count,
            failed_command_count,
            skipped_command_count,
            drift_event_count,
            blocked_reason_count,
            drift_rate,
            latest_clean,
            latest_blocked_reasons,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_audits == 0
    }

    pub fn is_clean(&self) -> bool {
        self.total_audits > 0 && self.dirty_audits == 0
    }

    pub fn has_drift(&self) -> bool {
        self.drift_event_count > 0
    }

    pub fn health(
        &self,
        policy: AgentServiceCommandAuditHealthPolicy,
    ) -> AgentServiceCommandAuditHealth {
        AgentServiceCommandAuditHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentServiceCommandAuditHealthPolicy {
    pub minimum_clean_rate: f32,
    pub maximum_drift_rate: f32,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentServiceCommandAuditHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_clean_rate: 0.67,
            maximum_drift_rate: 0.0,
            maximum_blocked_reasons: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceCommandAuditHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentServiceCommandAuditDashboard,
}

impl AgentServiceCommandAuditHealth {
    pub fn from_dashboard(
        dashboard: AgentServiceCommandAuditDashboard,
        policy: AgentServiceCommandAuditHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("service_command_audit_history_empty".to_owned());
        } else if dashboard.clean_rate < policy.minimum_clean_rate {
            watch_reasons.push(format!(
                "service_command_audit_clean_rate={:.3}<{}",
                dashboard.clean_rate, policy.minimum_clean_rate
            ));
        }

        if dashboard.drift_rate > policy.maximum_drift_rate {
            repair_reasons.push(format!(
                "service_command_audit_drift_rate={:.3}>{}",
                dashboard.drift_rate, policy.maximum_drift_rate
            ));
        }

        if dashboard.blocked_reason_count > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "service_command_audit_blocked_reasons={}>{}",
                dashboard.blocked_reason_count, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentClosedLoopExecutionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentClosedLoopExecutionHealthStatus::Watch, watch_reasons)
        } else {
            (AgentClosedLoopExecutionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentClosedLoopExecutionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceCommandAuditSummaryHistoryRecord {
    pub history: AgentServiceCommandAuditSummaryHistory,
    pub appended_summary: AgentServiceCommandAuditSummary,
    pub dashboard: AgentServiceCommandAuditDashboard,
    pub health: AgentServiceCommandAuditHealth,
    pub telemetry: Vec<String>,
}

impl AgentServiceCommandAuditSummaryHistoryRecord {
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

#[derive(Debug, Clone, Default)]
pub struct AgentServiceCommandAuditSummaryHistoryRecorder;

impl AgentServiceCommandAuditSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary(
        &self,
        mut history: AgentServiceCommandAuditSummaryHistory,
        summary: AgentServiceCommandAuditSummary,
        policy: AgentServiceCommandAuditHealthPolicy,
    ) -> AgentServiceCommandAuditSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_command_audit_history_record_telemetry(&dashboard, &health);

        AgentServiceCommandAuditSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_audit(
        &self,
        history: AgentServiceCommandAuditSummaryHistory,
        audit: &AgentServiceCommandAudit,
        policy: AgentServiceCommandAuditHealthPolicy,
    ) -> AgentServiceCommandAuditSummaryHistoryRecord {
        self.record_summary(history, audit.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceFeedback {
    pub audit: AgentServiceCommandAudit,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
}

impl AgentServiceFeedback {
    pub fn from_audit(run_id: impl AsRef<str>, audit: AgentServiceCommandAudit) -> Self {
        let run_id = run_id.as_ref();
        let repair_tasks = audit
            .blocked_reasons()
            .into_iter()
            .enumerate()
            .map(|(index, reason)| repair_task(run_id, index, reason))
            .collect::<Vec<_>>();
        let next_queue = AgentTaskQueue::from_tasks(repair_tasks.clone());

        Self {
            audit,
            repair_tasks,
            next_queue,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.audit.is_clean() && self.repair_tasks.is_empty()
    }

    pub fn summary(&self) -> AgentServiceFeedbackSummary {
        AgentServiceFeedbackSummary::from_feedback(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentServiceFeedbackSummary {
    pub audit_clean: bool,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: Vec<String>,
    pub clean: bool,
    pub telemetry: Vec<String>,
}

impl AgentServiceFeedbackSummary {
    pub fn from_feedback(feedback: &AgentServiceFeedback) -> Self {
        let blocked_reasons = feedback.audit.blocked_reasons();
        let audit_clean = feedback.audit.is_clean();
        let repair_tasks = feedback.repair_tasks.len();
        let next_queue_tasks = feedback.next_queue.task_ids().len();
        let clean = feedback.is_clean();
        let telemetry = service_feedback_summary_telemetry(
            audit_clean,
            repair_tasks,
            next_queue_tasks,
            blocked_reasons.len(),
            clean,
        );

        Self {
            audit_clean,
            repair_tasks,
            next_queue_tasks,
            blocked_reasons,
            clean,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentServiceFeedbackSummaryHistory {
    summaries: Vec<AgentServiceFeedbackSummary>,
}

impl AgentServiceFeedbackSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentServiceFeedbackSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentServiceFeedbackSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentServiceFeedbackSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentServiceFeedbackSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentServiceFeedbackDashboard {
        AgentServiceFeedbackDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AgentServiceFeedbackHealthPolicy) -> AgentServiceFeedbackHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceFeedbackDashboard {
    pub total_feedbacks: usize,
    pub clean_feedbacks: usize,
    pub dirty_feedbacks: usize,
    pub clean_rate: f32,
    pub audit_dirty_feedbacks: usize,
    pub repair_task_count: usize,
    pub total_next_queue_tasks: usize,
    pub blocked_reason_count: usize,
    pub repair_task_rate: f32,
    pub latest_clean: Option<bool>,
    pub latest_audit_clean: Option<bool>,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceFeedbackDashboard {
    pub fn from_summaries(summaries: &[AgentServiceFeedbackSummary]) -> Self {
        let total_feedbacks = summaries.len();
        let clean_feedbacks = summaries.iter().filter(|summary| summary.clean).count();
        let dirty_feedbacks = total_feedbacks.saturating_sub(clean_feedbacks);
        let audit_dirty_feedbacks = summaries
            .iter()
            .filter(|summary| !summary.audit_clean)
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reason_count = summaries
            .iter()
            .map(|summary| summary.blocked_reasons.len())
            .sum::<usize>();
        let clean_rate = service_execution_rate(clean_feedbacks, total_feedbacks);
        let repair_task_rate = service_execution_rate(repair_task_count, total_feedbacks);
        let latest = summaries.last();
        let latest_clean = latest.map(|summary| summary.clean);
        let latest_audit_clean = latest.map(|summary| summary.audit_clean);
        let latest_blocked_reasons = latest
            .map(|summary| summary.blocked_reasons.clone())
            .unwrap_or_default();
        let telemetry = service_feedback_dashboard_telemetry(
            total_feedbacks,
            clean_feedbacks,
            dirty_feedbacks,
            audit_dirty_feedbacks,
            repair_task_count,
            total_next_queue_tasks,
            blocked_reason_count,
            clean_rate,
            repair_task_rate,
        );

        Self {
            total_feedbacks,
            clean_feedbacks,
            dirty_feedbacks,
            clean_rate,
            audit_dirty_feedbacks,
            repair_task_count,
            total_next_queue_tasks,
            blocked_reason_count,
            repair_task_rate,
            latest_clean,
            latest_audit_clean,
            latest_blocked_reasons,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_feedbacks == 0
    }

    pub fn is_clean(&self) -> bool {
        self.total_feedbacks > 0 && self.dirty_feedbacks == 0
    }

    pub fn has_repair_pressure(&self) -> bool {
        self.repair_task_count > 0 || self.blocked_reason_count > 0
    }

    pub fn health(&self, policy: AgentServiceFeedbackHealthPolicy) -> AgentServiceFeedbackHealth {
        AgentServiceFeedbackHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentServiceFeedbackHealthPolicy {
    pub minimum_clean_rate: f32,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentServiceFeedbackHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_clean_rate: 0.67,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceFeedbackHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentServiceFeedbackDashboard,
}

impl AgentServiceFeedbackHealth {
    pub fn from_dashboard(
        dashboard: AgentServiceFeedbackDashboard,
        policy: AgentServiceFeedbackHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("service_feedback_history_empty".to_owned());
        } else if dashboard.clean_rate < policy.minimum_clean_rate {
            watch_reasons.push(format!(
                "service_feedback_clean_rate={:.3}<{}",
                dashboard.clean_rate, policy.minimum_clean_rate
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "service_feedback_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reason_count > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "service_feedback_blocked_reasons={}>{}",
                dashboard.blocked_reason_count, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentClosedLoopExecutionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentClosedLoopExecutionHealthStatus::Watch, watch_reasons)
        } else {
            (AgentClosedLoopExecutionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentClosedLoopExecutionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceFeedbackSummaryHistoryRecord {
    pub history: AgentServiceFeedbackSummaryHistory,
    pub appended_summary: AgentServiceFeedbackSummary,
    pub dashboard: AgentServiceFeedbackDashboard,
    pub health: AgentServiceFeedbackHealth,
    pub telemetry: Vec<String>,
}

impl AgentServiceFeedbackSummaryHistoryRecord {
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

#[derive(Debug, Clone, Default)]
pub struct AgentServiceFeedbackSummaryHistoryRecorder;

impl AgentServiceFeedbackSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary(
        &self,
        mut history: AgentServiceFeedbackSummaryHistory,
        summary: AgentServiceFeedbackSummary,
        policy: AgentServiceFeedbackHealthPolicy,
    ) -> AgentServiceFeedbackSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_feedback_history_record_telemetry(&dashboard, &health);

        AgentServiceFeedbackSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_feedback(
        &self,
        history: AgentServiceFeedbackSummaryHistory,
        feedback: &AgentServiceFeedback,
        policy: AgentServiceFeedbackHealthPolicy,
    ) -> AgentServiceFeedbackSummaryHistoryRecord {
        self.record_summary(history, feedback.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceTurnover {
    pub feedback: AgentServiceFeedback,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
}

impl AgentServiceTurnover {
    pub fn from_feedback(
        business_plan: &AgentBusinessLoopPlan,
        feedback: AgentServiceFeedback,
    ) -> Self {
        let next_queue = business_plan
            .next_queue
            .clone()
            .with_repair_first(&feedback.repair_tasks);
        let blocked_reasons = feedback.audit.blocked_reasons();

        Self {
            feedback,
            next_queue,
            blocked_reasons,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.feedback.is_clean() && self.blocked_reasons.is_empty()
    }

    pub fn summary(&self) -> AgentServiceTurnoverSummary {
        AgentServiceTurnoverSummary::from_turnover(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentServiceTurnoverSummary {
    pub feedback_clean: bool,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: Vec<String>,
    pub clean: bool,
    pub telemetry: Vec<String>,
}

impl AgentServiceTurnoverSummary {
    pub fn from_turnover(turnover: &AgentServiceTurnover) -> Self {
        let feedback_clean = turnover.feedback.is_clean();
        let repair_tasks = turnover.feedback.repair_tasks.len();
        let next_queue_tasks = turnover.next_queue.task_ids().len();
        let blocked_reasons = turnover.blocked_reasons.clone();
        let clean = turnover.is_clean();
        let telemetry = service_turnover_summary_telemetry(
            feedback_clean,
            repair_tasks,
            next_queue_tasks,
            blocked_reasons.len(),
            clean,
        );

        Self {
            feedback_clean,
            repair_tasks,
            next_queue_tasks,
            blocked_reasons,
            clean,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentServiceTurnoverSummaryHistory {
    summaries: Vec<AgentServiceTurnoverSummary>,
}

impl AgentServiceTurnoverSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentServiceTurnoverSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentServiceTurnoverSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentServiceTurnoverSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentServiceTurnoverSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentServiceTurnoverDashboard {
        AgentServiceTurnoverDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AgentServiceTurnoverHealthPolicy) -> AgentServiceTurnoverHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceTurnoverDashboard {
    pub total_turnovers: usize,
    pub clean_turnovers: usize,
    pub dirty_turnovers: usize,
    pub clean_rate: f32,
    pub feedback_dirty_turnovers: usize,
    pub repair_task_count: usize,
    pub total_next_queue_tasks: usize,
    pub blocked_reason_count: usize,
    pub repair_task_rate: f32,
    pub next_queue_task_rate: f32,
    pub latest_clean: Option<bool>,
    pub latest_feedback_clean: Option<bool>,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceTurnoverDashboard {
    pub fn from_summaries(summaries: &[AgentServiceTurnoverSummary]) -> Self {
        let total_turnovers = summaries.len();
        let clean_turnovers = summaries.iter().filter(|summary| summary.clean).count();
        let dirty_turnovers = total_turnovers.saturating_sub(clean_turnovers);
        let feedback_dirty_turnovers = summaries
            .iter()
            .filter(|summary| !summary.feedback_clean)
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reason_count = summaries
            .iter()
            .map(|summary| summary.blocked_reasons.len())
            .sum::<usize>();
        let clean_rate = service_execution_rate(clean_turnovers, total_turnovers);
        let repair_task_rate = service_execution_rate(repair_task_count, total_turnovers);
        let next_queue_task_rate = service_execution_rate(total_next_queue_tasks, total_turnovers);
        let latest = summaries.last();
        let latest_clean = latest.map(|summary| summary.clean);
        let latest_feedback_clean = latest.map(|summary| summary.feedback_clean);
        let latest_blocked_reasons = latest
            .map(|summary| summary.blocked_reasons.clone())
            .unwrap_or_default();
        let telemetry = service_turnover_dashboard_telemetry(
            total_turnovers,
            clean_turnovers,
            dirty_turnovers,
            feedback_dirty_turnovers,
            repair_task_count,
            total_next_queue_tasks,
            blocked_reason_count,
            clean_rate,
            repair_task_rate,
            next_queue_task_rate,
        );

        Self {
            total_turnovers,
            clean_turnovers,
            dirty_turnovers,
            clean_rate,
            feedback_dirty_turnovers,
            repair_task_count,
            total_next_queue_tasks,
            blocked_reason_count,
            repair_task_rate,
            next_queue_task_rate,
            latest_clean,
            latest_feedback_clean,
            latest_blocked_reasons,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_turnovers == 0
    }

    pub fn is_clean(&self) -> bool {
        self.total_turnovers > 0 && self.dirty_turnovers == 0
    }

    pub fn has_repair_pressure(&self) -> bool {
        self.repair_task_count > 0 || self.blocked_reason_count > 0
    }

    pub fn health(&self, policy: AgentServiceTurnoverHealthPolicy) -> AgentServiceTurnoverHealth {
        AgentServiceTurnoverHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentServiceTurnoverHealthPolicy {
    pub minimum_clean_rate: f32,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_next_queue_tasks: usize,
}

impl Default for AgentServiceTurnoverHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_clean_rate: 0.67,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: 0,
            maximum_next_queue_tasks: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceTurnoverHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentServiceTurnoverDashboard,
}

impl AgentServiceTurnoverHealth {
    pub fn from_dashboard(
        dashboard: AgentServiceTurnoverDashboard,
        policy: AgentServiceTurnoverHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("service_turnover_history_empty".to_owned());
        } else if dashboard.clean_rate < policy.minimum_clean_rate {
            watch_reasons.push(format!(
                "service_turnover_clean_rate={:.3}<{}",
                dashboard.clean_rate, policy.minimum_clean_rate
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "service_turnover_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reason_count > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "service_turnover_blocked_reasons={}>{}",
                dashboard.blocked_reason_count, policy.maximum_blocked_reasons
            ));
        }

        if dashboard.total_next_queue_tasks > policy.maximum_next_queue_tasks {
            watch_reasons.push(format!(
                "service_turnover_next_queue_tasks={}>{}",
                dashboard.total_next_queue_tasks, policy.maximum_next_queue_tasks
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentClosedLoopExecutionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentClosedLoopExecutionHealthStatus::Watch, watch_reasons)
        } else {
            (AgentClosedLoopExecutionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentClosedLoopExecutionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceTurnoverSummaryHistoryRecord {
    pub history: AgentServiceTurnoverSummaryHistory,
    pub appended_summary: AgentServiceTurnoverSummary,
    pub dashboard: AgentServiceTurnoverDashboard,
    pub health: AgentServiceTurnoverHealth,
    pub telemetry: Vec<String>,
}

impl AgentServiceTurnoverSummaryHistoryRecord {
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

#[derive(Debug, Clone, Default)]
pub struct AgentServiceTurnoverSummaryHistoryRecorder;

impl AgentServiceTurnoverSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary(
        &self,
        mut history: AgentServiceTurnoverSummaryHistory,
        summary: AgentServiceTurnoverSummary,
        policy: AgentServiceTurnoverHealthPolicy,
    ) -> AgentServiceTurnoverSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_turnover_history_record_telemetry(&dashboard, &health);

        AgentServiceTurnoverSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_turnover(
        &self,
        history: AgentServiceTurnoverSummaryHistory,
        turnover: &AgentServiceTurnover,
        policy: AgentServiceTurnoverHealthPolicy,
    ) -> AgentServiceTurnoverSummaryHistoryRecord {
        self.record_summary(history, turnover.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionReport {
    pub command_plan: AgentServiceCommandPlan,
    pub audit: AgentServiceCommandAudit,
    pub feedback: AgentServiceFeedback,
    pub turnover: AgentServiceTurnover,
}

impl AgentServiceExecutionReport {
    pub fn is_clean(&self) -> bool {
        self.audit.is_clean() && self.feedback.is_clean() && self.turnover.is_clean()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.turnover.next_queue.clone()
    }

    pub fn summary(&self) -> AgentServiceExecutionReportSummary {
        AgentServiceExecutionReportSummary::from_report(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentServiceExecutionReportSummary {
    pub command_count: usize,
    pub command_kinds: Vec<String>,
    pub memory_promotion_reason_count: usize,
    pub tool_build_reason_count: usize,
    pub expected_commands: usize,
    pub receipts: usize,
    pub missing_commands: usize,
    pub failed_commands: usize,
    pub skipped_commands: usize,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: Vec<String>,
    pub clean: bool,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionReportSummary {
    pub fn from_report(report: &AgentServiceExecutionReport) -> Self {
        let command_plan = report.command_plan.summary();
        let audit = report.audit.summary();
        let turnover = report.turnover.summary();
        let clean = report.is_clean();
        let telemetry = service_execution_report_summary_telemetry(
            command_plan.command_count,
            audit.expected_commands,
            audit.receipts,
            audit.missing_commands,
            audit.failed_commands,
            audit.skipped_commands,
            command_plan.memory_promotion_reason_count,
            command_plan.tool_build_reason_count,
            turnover.repair_tasks,
            turnover.next_queue_tasks,
            turnover.blocked_reasons.len(),
            clean,
        );

        Self {
            command_count: command_plan.command_count,
            command_kinds: command_plan.command_kinds,
            memory_promotion_reason_count: command_plan.memory_promotion_reason_count,
            tool_build_reason_count: command_plan.tool_build_reason_count,
            expected_commands: audit.expected_commands,
            receipts: audit.receipts,
            missing_commands: audit.missing_commands,
            failed_commands: audit.failed_commands,
            skipped_commands: audit.skipped_commands,
            repair_tasks: turnover.repair_tasks,
            next_queue_tasks: turnover.next_queue_tasks,
            blocked_reasons: turnover.blocked_reasons,
            clean,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentServiceExecutionHistory {
    summaries: Vec<AgentServiceExecutionReportSummary>,
}

impl AgentServiceExecutionHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentServiceExecutionReportSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentServiceExecutionReportSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentServiceExecutionReportSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentServiceExecutionReportSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentServiceExecutionDashboard {
        AgentServiceExecutionDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AgentServiceExecutionHealthPolicy) -> AgentServiceExecutionHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionDashboard {
    pub total_runs: usize,
    pub clean_runs: usize,
    pub dirty_runs: usize,
    pub clean_rate: f32,
    pub command_count: usize,
    pub receipt_count: usize,
    pub missing_command_count: usize,
    pub failed_command_count: usize,
    pub skipped_command_count: usize,
    pub memory_promotion_reason_count: usize,
    pub memory_promotion_reason_runs: usize,
    pub tool_build_reason_count: usize,
    pub tool_build_reason_runs: usize,
    pub repair_task_count: usize,
    pub total_next_queue_tasks: usize,
    pub service_drift_rate: f32,
    pub latest_clean: Option<bool>,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionDashboard {
    pub fn from_summaries(summaries: &[AgentServiceExecutionReportSummary]) -> Self {
        let total_runs = summaries.len();
        let clean_runs = summaries.iter().filter(|summary| summary.clean).count();
        let dirty_runs = total_runs.saturating_sub(clean_runs);
        let command_count = summaries
            .iter()
            .map(|summary| summary.command_count)
            .sum::<usize>();
        let receipt_count = summaries
            .iter()
            .map(|summary| summary.receipts)
            .sum::<usize>();
        let missing_command_count = summaries
            .iter()
            .map(|summary| summary.missing_commands)
            .sum::<usize>();
        let failed_command_count = summaries
            .iter()
            .map(|summary| summary.failed_commands)
            .sum::<usize>();
        let skipped_command_count = summaries
            .iter()
            .map(|summary| summary.skipped_commands)
            .sum::<usize>();
        let memory_promotion_reason_count = summaries
            .iter()
            .map(|summary| summary.memory_promotion_reason_count)
            .sum::<usize>();
        let memory_promotion_reason_runs = summaries
            .iter()
            .filter(|summary| summary.memory_promotion_reason_count > 0)
            .count();
        let tool_build_reason_count = summaries
            .iter()
            .map(|summary| summary.tool_build_reason_count)
            .sum::<usize>();
        let tool_build_reason_runs = summaries
            .iter()
            .filter(|summary| summary.tool_build_reason_count > 0)
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let service_drift_events =
            missing_command_count + failed_command_count + skipped_command_count;
        let clean_rate = service_execution_rate(clean_runs, total_runs);
        let service_drift_rate = service_execution_rate(service_drift_events, command_count);
        let latest = summaries.last();
        let latest_clean = latest.map(|summary| summary.clean);
        let latest_blocked_reasons = latest
            .map(|summary| summary.blocked_reasons.clone())
            .unwrap_or_default();
        let telemetry = service_execution_dashboard_telemetry(
            total_runs,
            clean_runs,
            dirty_runs,
            command_count,
            service_drift_events,
            memory_promotion_reason_count,
            memory_promotion_reason_runs,
            tool_build_reason_count,
            tool_build_reason_runs,
            repair_task_count,
            total_next_queue_tasks,
            clean_rate,
            service_drift_rate,
        );

        Self {
            total_runs,
            clean_runs,
            dirty_runs,
            clean_rate,
            command_count,
            receipt_count,
            missing_command_count,
            failed_command_count,
            skipped_command_count,
            memory_promotion_reason_count,
            memory_promotion_reason_runs,
            tool_build_reason_count,
            tool_build_reason_runs,
            repair_task_count,
            total_next_queue_tasks,
            service_drift_rate,
            latest_clean,
            latest_blocked_reasons,
            telemetry,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.total_runs > 0 && self.dirty_runs == 0
    }

    pub fn is_empty(&self) -> bool {
        self.total_runs == 0
    }

    pub fn has_service_drift(&self) -> bool {
        self.missing_command_count + self.failed_command_count + self.skipped_command_count > 0
    }

    pub fn health(&self, policy: AgentServiceExecutionHealthPolicy) -> AgentServiceExecutionHealth {
        AgentServiceExecutionHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentServiceExecutionHealthPolicy {
    pub minimum_clean_rate: f32,
    pub maximum_service_drift_rate: f32,
    pub maximum_memory_promotion_reason_runs: usize,
    pub maximum_tool_build_reason_runs: usize,
    pub maximum_repair_tasks: usize,
}

impl Default for AgentServiceExecutionHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_clean_rate: 0.67,
            maximum_service_drift_rate: 0.0,
            maximum_memory_promotion_reason_runs: usize::MAX,
            maximum_tool_build_reason_runs: usize::MAX,
            maximum_repair_tasks: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentServiceExecutionDashboard,
}

impl AgentServiceExecutionHealth {
    pub fn from_dashboard(
        dashboard: AgentServiceExecutionDashboard,
        policy: AgentServiceExecutionHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("service_execution_history_empty".to_owned());
        } else if dashboard.clean_rate < policy.minimum_clean_rate {
            watch_reasons.push(format!(
                "service_execution_clean_rate={:.3}<{}",
                dashboard.clean_rate, policy.minimum_clean_rate
            ));
        }

        if dashboard.service_drift_rate > policy.maximum_service_drift_rate {
            repair_reasons.push(format!(
                "service_execution_drift_rate={:.3}>{}",
                dashboard.service_drift_rate, policy.maximum_service_drift_rate
            ));
        }

        if dashboard.memory_promotion_reason_runs > policy.maximum_memory_promotion_reason_runs {
            repair_reasons.push(format!(
                "service_execution_memory_promotion_reason_runs={}>{}",
                dashboard.memory_promotion_reason_runs, policy.maximum_memory_promotion_reason_runs
            ));
        }

        if dashboard.tool_build_reason_runs > policy.maximum_tool_build_reason_runs {
            repair_reasons.push(format!(
                "service_execution_tool_build_reason_runs={}>{}",
                dashboard.tool_build_reason_runs, policy.maximum_tool_build_reason_runs
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "service_execution_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentClosedLoopExecutionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentClosedLoopExecutionHealthStatus::Watch, watch_reasons)
        } else {
            (AgentClosedLoopExecutionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentClosedLoopExecutionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHistoryRecord {
    pub history: AgentServiceExecutionHistory,
    pub appended_summary: AgentServiceExecutionReportSummary,
    pub dashboard: AgentServiceExecutionDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthRecord {
    pub history: AgentServiceExecutionHistory,
    pub appended_summary: AgentServiceExecutionReportSummary,
    pub dashboard: AgentServiceExecutionDashboard,
    pub health: AgentServiceExecutionHealth,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthRecord {
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

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateRecord {
    pub health_record: AgentServiceExecutionHealthRecord,
    pub gate_decision: AgentServiceExecutionHealthGateDecision,
    pub gate_summary: AgentServiceExecutionHealthGateSummary,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHistoryRecorder;

impl AgentServiceExecutionHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary(
        &self,
        mut history: AgentServiceExecutionHistory,
        summary: AgentServiceExecutionReportSummary,
    ) -> AgentServiceExecutionHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();

        AgentServiceExecutionHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
        }
    }

    pub fn record_report(
        &self,
        history: AgentServiceExecutionHistory,
        report: &AgentServiceExecutionReport,
    ) -> AgentServiceExecutionHistoryRecord {
        self.record_summary(history, report.summary())
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentServiceExecutionHistory,
        summary: AgentServiceExecutionReportSummary,
        policy: AgentServiceExecutionHealthPolicy,
    ) -> AgentServiceExecutionHealthRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_execution_health_record_telemetry(&dashboard, &health);

        AgentServiceExecutionHealthRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_report_with_health(
        &self,
        history: AgentServiceExecutionHistory,
        report: &AgentServiceExecutionReport,
        policy: AgentServiceExecutionHealthPolicy,
    ) -> AgentServiceExecutionHealthRecord {
        self.record_summary_with_health(history, report.summary(), policy)
    }

    pub fn record_summary_with_health_gate(
        &self,
        history: AgentServiceExecutionHistory,
        summary: AgentServiceExecutionReportSummary,
        policy: AgentServiceExecutionHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentServiceExecutionHealthGateRecord {
        let health_record = self.record_summary_with_health(history, summary, policy);
        let gate_decision = AgentServiceExecutionHealthGate::new().evaluate(
            run_id,
            &health_record.health,
            next_queue,
        );
        let gate_summary = gate_decision.summary();
        let telemetry =
            service_execution_health_gate_record_telemetry(&health_record, &gate_summary);

        AgentServiceExecutionHealthGateRecord {
            health_record,
            gate_decision,
            gate_summary,
            telemetry,
        }
    }

    pub fn record_report_with_health_gate(
        &self,
        history: AgentServiceExecutionHistory,
        report: &AgentServiceExecutionReport,
        policy: AgentServiceExecutionHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentServiceExecutionHealthGateRecord {
        self.record_summary_with_health_gate(history, report.summary(), policy, run_id, next_queue)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateDecision {
    pub health_status: AgentClosedLoopExecutionHealthStatus,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }

    pub fn summary(&self) -> AgentServiceExecutionHealthGateSummary {
        AgentServiceExecutionHealthGateSummary::from_decision(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentServiceExecutionHealthGateSummary {
    pub health_status: AgentClosedLoopExecutionHealthStatus,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateSummary {
    pub fn from_decision(decision: &AgentServiceExecutionHealthGateDecision) -> Self {
        let repair_task_ids = decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = decision.next_queue.task_ids();
        let telemetry = service_execution_health_gate_summary_telemetry(
            decision.health_status,
            decision.admitted,
            decision.requires_repair_first,
            repair_task_ids.len(),
            next_queue_task_ids.len(),
            decision.blocked_reasons.len(),
        );

        Self {
            health_status: decision.health_status,
            admitted: decision.admitted,
            requires_repair_first: decision.requires_repair_first,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            repair_task_ids,
            next_queue_task_ids,
            blocked_reasons: decision.blocked_reasons.clone(),
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentServiceExecutionHealthGateHistory {
    summaries: Vec<AgentServiceExecutionHealthGateSummary>,
}

impl AgentServiceExecutionHealthGateHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentServiceExecutionHealthGateSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentServiceExecutionHealthGateSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentServiceExecutionHealthGateSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentServiceExecutionHealthGateSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentServiceExecutionHealthGateDashboard {
        AgentServiceExecutionHealthGateDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentServiceExecutionHealthGateHealthPolicy,
    ) -> AgentServiceExecutionHealthGateHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateDashboard {
    pub total_records: usize,
    pub admitted_records: usize,
    pub repair_first_records: usize,
    pub repair_task_count: usize,
    pub total_next_queue_tasks: usize,
    pub admission_rate: f32,
    pub repair_first_rate: f32,
    pub latest_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateDashboard {
    pub fn from_summaries(summaries: &[AgentServiceExecutionHealthGateSummary]) -> Self {
        let total_records = summaries.len();
        let admitted_records = summaries
            .iter()
            .filter(|summary| summary.admitted && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let admission_rate = rate(admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let latest_health_status = summaries.last().map(|summary| summary.health_status);
        let latest_blocked_reasons = summaries
            .last()
            .map(|summary| summary.blocked_reasons.clone())
            .unwrap_or_default();
        let telemetry = service_execution_health_gate_dashboard_telemetry(
            total_records,
            admitted_records,
            repair_first_records,
            repair_task_count,
            total_next_queue_tasks,
            admission_rate,
            repair_first_rate,
            latest_health_status,
            latest_blocked_reasons.len(),
        );

        Self {
            total_records,
            admitted_records,
            repair_first_records,
            repair_task_count,
            total_next_queue_tasks,
            admission_rate,
            repair_first_rate,
            latest_health_status,
            latest_blocked_reasons,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn is_clean(&self) -> bool {
        !self.is_empty() && self.repair_first_records == 0
    }

    pub fn health(
        &self,
        policy: AgentServiceExecutionHealthGateHealthPolicy,
    ) -> AgentServiceExecutionHealthGateHealth {
        AgentServiceExecutionHealthGateHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentServiceExecutionHealthGateHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_tasks: usize,
}

impl Default for AgentServiceExecutionHealthGateHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_admission_rate: 0.67,
            maximum_repair_first_rate: 0.0,
            maximum_repair_tasks: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentServiceExecutionHealthGateDashboard,
}

impl AgentServiceExecutionHealthGateHealth {
    pub fn from_dashboard(
        dashboard: AgentServiceExecutionHealthGateDashboard,
        policy: AgentServiceExecutionHealthGateHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("service_execution_health_gate_history_empty".to_owned());
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "service_execution_health_gate_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "service_execution_health_gate_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "service_execution_health_gate_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentClosedLoopExecutionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentClosedLoopExecutionHealthStatus::Watch, watch_reasons)
        } else {
            (AgentClosedLoopExecutionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentClosedLoopExecutionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateHistoryRecord {
    pub history: AgentServiceExecutionHealthGateHistory,
    pub appended_summary: AgentServiceExecutionHealthGateSummary,
    pub dashboard: AgentServiceExecutionHealthGateDashboard,
    pub health: AgentServiceExecutionHealthGateHealth,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateHistoryRecord {
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

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateHistoryRecorder;

impl AgentServiceExecutionHealthGateHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentServiceExecutionHealthGateHistory,
        summary: AgentServiceExecutionHealthGateSummary,
        policy: AgentServiceExecutionHealthGateHealthPolicy,
    ) -> AgentServiceExecutionHealthGateHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_execution_health_gate_history_record_telemetry(&dashboard, &health);

        AgentServiceExecutionHealthGateHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_gate_record_with_health(
        &self,
        history: AgentServiceExecutionHealthGateHistory,
        record: &AgentServiceExecutionHealthGateRecord,
        policy: AgentServiceExecutionHealthGateHealthPolicy,
    ) -> AgentServiceExecutionHealthGateHistoryRecord {
        self.record_summary_with_health(history, record.gate_summary.clone(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffRecord {
    pub trend_record: AgentServiceExecutionHealthGateHistoryRecord,
    pub gate_decision: AgentServiceExecutionHealthGateDecision,
    pub gate_summary: AgentServiceExecutionHealthGateSummary,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentServiceExecutionHealthGateTrendHandoffSummary {
        AgentServiceExecutionHealthGateTrendHandoffSummary::from_record(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffSummary {
    pub trend_health_status: AgentClosedLoopExecutionHealthStatus,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub trend_records: usize,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffSummary {
    pub fn from_record(record: &AgentServiceExecutionHealthGateTrendHandoffRecord) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry = service_execution_health_gate_trend_handoff_summary_telemetry(
            record.trend_record.health.status,
            record.gate_decision.admitted,
            record.gate_decision.requires_repair_first,
            record.trend_record.dashboard.total_records,
            repair_task_ids.len(),
            next_queue_task_ids.len(),
            record.gate_decision.blocked_reasons.len(),
        );

        Self {
            trend_health_status: record.trend_record.health.status,
            admitted: record.gate_decision.admitted,
            requires_repair_first: record.gate_decision.requires_repair_first,
            trend_records: record.trend_record.dashboard.total_records,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            repair_task_ids,
            next_queue_task_ids,
            blocked_reasons: record.gate_decision.blocked_reasons.clone(),
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffHistory {
    summaries: Vec<AgentServiceExecutionHealthGateTrendHandoffSummary>,
}

impl AgentServiceExecutionHealthGateTrendHandoffHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentServiceExecutionHealthGateTrendHandoffSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentServiceExecutionHealthGateTrendHandoffSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentServiceExecutionHealthGateTrendHandoffSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentServiceExecutionHealthGateTrendHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentServiceExecutionHealthGateTrendHandoffDashboard {
        AgentServiceExecutionHealthGateTrendHandoffDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentServiceExecutionHealthGateTrendHandoffHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffDashboard {
    pub total_records: usize,
    pub admitted_records: usize,
    pub repair_first_records: usize,
    pub stable_records: usize,
    pub watch_records: usize,
    pub repair_records: usize,
    pub repair_task_count: usize,
    pub total_next_queue_tasks: usize,
    pub admission_rate: f32,
    pub repair_first_rate: f32,
    pub latest_trend_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffDashboard {
    pub fn from_summaries(
        summaries: &[AgentServiceExecutionHealthGateTrendHandoffSummary],
    ) -> Self {
        let total_records = summaries.len();
        let admitted_records = summaries
            .iter()
            .filter(|summary| summary.admitted && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let stable_records = summaries
            .iter()
            .filter(|summary| {
                summary.trend_health_status == AgentClosedLoopExecutionHealthStatus::Stable
            })
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| {
                summary.trend_health_status == AgentClosedLoopExecutionHealthStatus::Watch
            })
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| {
                summary.trend_health_status == AgentClosedLoopExecutionHealthStatus::Repair
            })
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let admission_rate = rate(admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let latest_trend_health_status =
            summaries.last().map(|summary| summary.trend_health_status);
        let latest_blocked_reasons = summaries
            .last()
            .map(|summary| summary.blocked_reasons.clone())
            .unwrap_or_default();
        let telemetry = service_execution_health_gate_trend_handoff_dashboard_telemetry(
            total_records,
            admitted_records,
            repair_first_records,
            stable_records,
            watch_records,
            repair_records,
            repair_task_count,
            total_next_queue_tasks,
            admission_rate,
            repair_first_rate,
            latest_trend_health_status,
            latest_blocked_reasons.len(),
        );

        Self {
            total_records,
            admitted_records,
            repair_first_records,
            stable_records,
            watch_records,
            repair_records,
            repair_task_count,
            total_next_queue_tasks,
            admission_rate,
            repair_first_rate,
            latest_trend_health_status,
            latest_blocked_reasons,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn is_clean(&self) -> bool {
        !self.is_empty() && self.repair_first_records == 0 && self.repair_records == 0
    }

    pub fn health(
        &self,
        policy: AgentServiceExecutionHealthGateTrendHandoffHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffHealth {
        AgentServiceExecutionHealthGateTrendHandoffHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentServiceExecutionHealthGateTrendHandoffHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_admission_rate: 0.67,
            maximum_repair_first_rate: 0.0,
            maximum_repair_records: 0,
            maximum_repair_tasks: 0,
            maximum_watch_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentServiceExecutionHealthGateTrendHandoffDashboard,
}

impl AgentServiceExecutionHealthGateTrendHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentServiceExecutionHealthGateTrendHandoffDashboard,
        policy: AgentServiceExecutionHealthGateTrendHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons
                .push("service_execution_health_gate_trend_handoff_history_empty".to_owned());
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentClosedLoopExecutionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentClosedLoopExecutionHealthStatus::Watch, watch_reasons)
        } else {
            (AgentClosedLoopExecutionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentClosedLoopExecutionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffHistoryRecord {
    pub history: AgentServiceExecutionHealthGateTrendHandoffHistory,
    pub appended_summary: AgentServiceExecutionHealthGateTrendHandoffSummary,
    pub dashboard: AgentServiceExecutionHealthGateTrendHandoffDashboard,
    pub health: AgentServiceExecutionHealthGateTrendHandoffHealth,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffHistoryRecord {
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

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffHistoryRecorder;

impl AgentServiceExecutionHealthGateTrendHandoffHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentServiceExecutionHealthGateTrendHandoffHistory,
        summary: AgentServiceExecutionHealthGateTrendHandoffSummary,
        policy: AgentServiceExecutionHealthGateTrendHandoffHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_execution_health_gate_trend_handoff_history_record_telemetry(
            &dashboard, &health,
        );

        AgentServiceExecutionHealthGateTrendHandoffHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_handoff_record_with_health(
        &self,
        history: AgentServiceExecutionHealthGateTrendHandoffHistory,
        record: &AgentServiceExecutionHealthGateTrendHandoffRecord,
        policy: AgentServiceExecutionHealthGateTrendHandoffHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffHistoryRecord {
        self.record_summary_with_health(history, record.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffGateDecision {
    pub requested_admitted: bool,
    pub handoff_health: AgentServiceExecutionHealthGateTrendHandoffHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffGate;

impl AgentServiceExecutionHealthGateTrendHandoffGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        handoff: &AgentServiceExecutionHealthGateTrendHandoffRecord,
        history_record: &AgentServiceExecutionHealthGateTrendHandoffHistoryRecord,
    ) -> AgentServiceExecutionHealthGateTrendHandoffGateDecision {
        let requested_admitted = handoff.is_admitted();
        let handoff_health = history_record.health.clone();
        let trend_requires_repair =
            handoff_health.status == AgentClosedLoopExecutionHealthStatus::Repair;
        let requires_repair_first =
            handoff.gate_decision.requires_repair_first || trend_requires_repair;
        let admitted = requested_admitted && !trend_requires_repair;
        let repair_tasks = if trend_requires_repair {
            handoff_health
                .reasons
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, reason)| {
                    service_execution_health_gate_trend_handoff_repair_task(
                        run_id.as_ref(),
                        index,
                        reason,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut next_queue = handoff.next_queue();
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let mut blocked_reasons = handoff.gate_decision.blocked_reasons.clone();
        if trend_requires_repair {
            blocked_reasons.extend(handoff_health.reasons.clone());
        }
        let telemetry = service_execution_health_gate_trend_handoff_gate_telemetry(
            handoff_health.status,
            requested_admitted,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            &blocked_reasons,
        );

        AgentServiceExecutionHealthGateTrendHandoffGateDecision {
            requested_admitted,
            handoff_health,
            admitted,
            requires_repair_first,
            repair_tasks,
            next_queue,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorRecord {
    pub handoff: AgentServiceExecutionHealthGateTrendHandoffRecord,
    pub history_record: AgentServiceExecutionHealthGateTrendHandoffHistoryRecord,
    pub gate_decision: AgentServiceExecutionHealthGateTrendHandoffGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentServiceExecutionHealthGateTrendHandoffMonitorSummary {
        AgentServiceExecutionHealthGateTrendHandoffMonitorSummary::from_monitor(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitor {
    history_recorder: AgentServiceExecutionHealthGateTrendHandoffHistoryRecorder,
    gate: AgentServiceExecutionHealthGateTrendHandoffGate,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        handoff: AgentServiceExecutionHealthGateTrendHandoffRecord,
        history: AgentServiceExecutionHealthGateTrendHandoffHistory,
        policy: AgentServiceExecutionHealthGateTrendHandoffHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorRecord {
        let history_record = self
            .history_recorder
            .record_handoff_record_with_health(history, &handoff, policy);
        let gate_decision = self.gate.evaluate(run_id, &handoff, &history_record);
        let telemetry = service_execution_health_gate_trend_handoff_monitor_telemetry(
            &handoff,
            &history_record,
            &gate_decision,
        );

        AgentServiceExecutionHealthGateTrendHandoffMonitorRecord {
            handoff,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorSummary {
    pub handoff_health_status: AgentClosedLoopExecutionHealthStatus,
    pub requested_admitted: bool,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub handoff_records: usize,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorSummary {
    pub fn from_monitor(record: &AgentServiceExecutionHealthGateTrendHandoffMonitorRecord) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry = service_execution_health_gate_trend_handoff_monitor_summary_telemetry(
            record.gate_decision.handoff_health.status,
            record.gate_decision.requested_admitted,
            record.gate_decision.admitted,
            record.gate_decision.requires_repair_first,
            record.history_record.dashboard.total_records,
            repair_task_ids.len(),
            next_queue_task_ids.len(),
            record.gate_decision.blocked_reasons.len(),
        );

        Self {
            handoff_health_status: record.gate_decision.handoff_health.status,
            requested_admitted: record.gate_decision.requested_admitted,
            admitted: record.gate_decision.admitted,
            requires_repair_first: record.gate_decision.requires_repair_first,
            handoff_records: record.history_record.dashboard.total_records,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: record.gate_decision.blocked_reasons.len(),
            repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory {
    summaries: Vec<AgentServiceExecutionHealthGateTrendHandoffMonitorSummary>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentServiceExecutionHealthGateTrendHandoffMonitorSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentServiceExecutionHealthGateTrendHandoffMonitorSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentServiceExecutionHealthGateTrendHandoffMonitorSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentServiceExecutionHealthGateTrendHandoffMonitorSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentServiceExecutionHealthGateTrendHandoffMonitorDashboard {
        AgentServiceExecutionHealthGateTrendHandoffMonitorDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorDashboard {
    pub total_records: usize,
    pub requested_admitted_records: usize,
    pub admitted_records: usize,
    pub repair_first_records: usize,
    pub stable_records: usize,
    pub watch_records: usize,
    pub repair_records: usize,
    pub repair_task_count: usize,
    pub total_next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub admission_rate: f32,
    pub repair_first_rate: f32,
    pub latest_handoff_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorDashboard {
    pub fn from_summaries(
        summaries: &[AgentServiceExecutionHealthGateTrendHandoffMonitorSummary],
    ) -> Self {
        let total_records = summaries.len();
        let requested_admitted_records = summaries
            .iter()
            .filter(|summary| summary.requested_admitted)
            .count();
        let admitted_records = summaries
            .iter()
            .filter(|summary| summary.admitted && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let stable_records = summaries
            .iter()
            .filter(|summary| {
                summary.handoff_health_status == AgentClosedLoopExecutionHealthStatus::Stable
            })
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| {
                summary.handoff_health_status == AgentClosedLoopExecutionHealthStatus::Watch
            })
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| {
                summary.handoff_health_status == AgentClosedLoopExecutionHealthStatus::Repair
            })
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reasons = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let admission_rate = rate(admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let latest_handoff_health_status = summaries
            .last()
            .map(|summary| summary.handoff_health_status);
        let telemetry = service_execution_health_gate_trend_handoff_monitor_dashboard_telemetry(
            total_records,
            requested_admitted_records,
            admitted_records,
            repair_first_records,
            stable_records,
            watch_records,
            repair_records,
            repair_task_count,
            total_next_queue_tasks,
            blocked_reasons,
            admission_rate,
            repair_first_rate,
            latest_handoff_health_status,
        );

        Self {
            total_records,
            requested_admitted_records,
            admitted_records,
            repair_first_records,
            stable_records,
            watch_records,
            repair_records,
            repair_task_count,
            total_next_queue_tasks,
            blocked_reasons,
            admission_rate,
            repair_first_rate,
            latest_handoff_health_status,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn is_clean(&self) -> bool {
        !self.is_empty() && self.repair_first_records == 0 && self.repair_records == 0
    }

    pub fn health(
        &self,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHealth {
        AgentServiceExecutionHealthGateTrendHandoffMonitorHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_admission_rate: 0.67,
            maximum_repair_first_rate: 0.0,
            maximum_repair_records: 0,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: 0,
            maximum_watch_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentServiceExecutionHealthGateTrendHandoffMonitorDashboard,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHealth {
    pub fn from_dashboard(
        dashboard: AgentServiceExecutionHealthGateTrendHandoffMonitorDashboard,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "service_execution_health_gate_trend_handoff_monitor_history_empty".to_owned(),
            );
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentClosedLoopExecutionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentClosedLoopExecutionHealthStatus::Watch, watch_reasons)
        } else {
            (AgentClosedLoopExecutionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentClosedLoopExecutionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecord {
    pub history: AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory,
    pub appended_summary: AgentServiceExecutionHealthGateTrendHandoffMonitorSummary,
    pub dashboard: AgentServiceExecutionHealthGateTrendHandoffMonitorDashboard,
    pub health: AgentServiceExecutionHealthGateTrendHandoffMonitorHealth,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecord {
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

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecorder;

impl AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory,
        summary: AgentServiceExecutionHealthGateTrendHandoffMonitorSummary,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            service_execution_health_gate_trend_handoff_monitor_history_record_telemetry(
                &dashboard, &health,
            );

        AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_monitor_with_health(
        &self,
        history: AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory,
        monitor: &AgentServiceExecutionHealthGateTrendHandoffMonitorRecord,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecord {
        self.record_summary_with_health(history, monitor.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorGateDecision {
    pub requested_admitted: bool,
    pub monitor_health: AgentServiceExecutionHealthGateTrendHandoffMonitorHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorGate;

impl AgentServiceExecutionHealthGateTrendHandoffMonitorGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        monitor: &AgentServiceExecutionHealthGateTrendHandoffMonitorRecord,
        history_record: &AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecord,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorGateDecision {
        let requested_admitted = monitor.is_admitted();
        let monitor_health = history_record.health.clone();
        let monitor_requires_repair =
            monitor_health.status == AgentClosedLoopExecutionHealthStatus::Repair;
        let requires_repair_first =
            monitor.gate_decision.requires_repair_first || monitor_requires_repair;
        let admitted = requested_admitted && !monitor_requires_repair;
        let repair_tasks = if monitor_requires_repair {
            monitor_health
                .reasons
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, reason)| {
                    service_execution_health_gate_trend_handoff_monitor_repair_task(
                        run_id.as_ref(),
                        index,
                        reason,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut next_queue = monitor.next_queue();
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let mut blocked_reasons = monitor.gate_decision.blocked_reasons.clone();
        if monitor_requires_repair {
            blocked_reasons.extend(monitor_health.reasons.clone());
        }
        let telemetry = service_execution_health_gate_trend_handoff_monitor_gate_telemetry(
            monitor_health.status,
            requested_admitted,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            &blocked_reasons,
        );

        AgentServiceExecutionHealthGateTrendHandoffMonitorGateDecision {
            requested_admitted,
            monitor_health,
            admitted,
            requires_repair_first,
            repair_tasks,
            next_queue,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffRecord {
    pub monitor: AgentServiceExecutionHealthGateTrendHandoffMonitorRecord,
    pub history_record: AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecord,
    pub gate_decision: AgentServiceExecutionHealthGateTrendHandoffMonitorGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary {
        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary::from_handoff(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoff {
    history_recorder: AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecorder,
    gate: AgentServiceExecutionHealthGateTrendHandoffMonitorGate,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        monitor: AgentServiceExecutionHealthGateTrendHandoffMonitorRecord,
        history: AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffRecord {
        let history_record = self
            .history_recorder
            .record_monitor_with_health(history, &monitor, policy);
        let gate_decision = self.gate.evaluate(run_id, &monitor, &history_record);
        let telemetry = service_execution_health_gate_trend_handoff_monitor_handoff_telemetry(
            &monitor,
            &history_record,
            &gate_decision,
        );

        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffRecord {
            monitor,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary {
    pub monitor_health_status: AgentClosedLoopExecutionHealthStatus,
    pub requested_admitted: bool,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub monitor_records: usize,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary {
    pub fn from_handoff(
        record: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffRecord,
    ) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry =
            service_execution_health_gate_trend_handoff_monitor_handoff_summary_telemetry(
                record.gate_decision.monitor_health.status,
                record.gate_decision.requested_admitted,
                record.gate_decision.admitted,
                record.gate_decision.requires_repair_first,
                record.history_record.dashboard.total_records,
                repair_task_ids.len(),
                next_queue_task_ids.len(),
                record.gate_decision.blocked_reasons.len(),
            );

        Self {
            monitor_health_status: record.gate_decision.monitor_health.status,
            requested_admitted: record.gate_decision.requested_admitted,
            admitted: record.gate_decision.admitted,
            requires_repair_first: record.gate_decision.requires_repair_first,
            monitor_records: record.history_record.dashboard.total_records,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: record.gate_decision.blocked_reasons.len(),
            repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory {
    summaries: Vec<AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(
        &mut self,
        summary: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary,
    ) {
        self.summaries.push(summary);
    }

    pub fn latest(
        &self,
    ) -> Option<&AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffDashboard {
        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffDashboard::from_summaries(
            &self.summaries,
        )
    }

    pub fn health(
        &self,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffDashboard {
    pub total_records: usize,
    pub requested_admitted_records: usize,
    pub admitted_records: usize,
    pub repair_first_records: usize,
    pub stable_records: usize,
    pub watch_records: usize,
    pub repair_records: usize,
    pub repair_task_count: usize,
    pub total_next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub admission_rate: f32,
    pub repair_first_rate: f32,
    pub latest_monitor_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffDashboard {
    pub fn from_summaries(
        summaries: &[AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary],
    ) -> Self {
        let total_records = summaries.len();
        let requested_admitted_records = summaries
            .iter()
            .filter(|summary| summary.requested_admitted)
            .count();
        let admitted_records = summaries
            .iter()
            .filter(|summary| summary.admitted && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let stable_records = summaries
            .iter()
            .filter(|summary| {
                summary.monitor_health_status == AgentClosedLoopExecutionHealthStatus::Stable
            })
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| {
                summary.monitor_health_status == AgentClosedLoopExecutionHealthStatus::Watch
            })
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| {
                summary.monitor_health_status == AgentClosedLoopExecutionHealthStatus::Repair
            })
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reasons = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let admission_rate = rate(admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let latest_monitor_health_status = summaries
            .last()
            .map(|summary| summary.monitor_health_status);
        let telemetry =
            service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_telemetry(
                total_records,
                requested_admitted_records,
                admitted_records,
                repair_first_records,
                stable_records,
                watch_records,
                repair_records,
                repair_task_count,
                total_next_queue_tasks,
                blocked_reasons,
                admission_rate,
                repair_first_rate,
                latest_monitor_health_status,
            );

        Self {
            total_records,
            requested_admitted_records,
            admitted_records,
            repair_first_records,
            stable_records,
            watch_records,
            repair_records,
            repair_task_count,
            total_next_queue_tasks,
            blocked_reasons,
            admission_rate,
            repair_first_rate,
            latest_monitor_health_status,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn is_clean(&self) -> bool {
        !self.is_empty() && self.repair_first_records == 0 && self.repair_records == 0
    }

    pub fn health(
        &self,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealth {
        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_admission_rate: 0.67,
            maximum_repair_first_rate: 0.0,
            maximum_repair_records: 0,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: 0,
            maximum_watch_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffDashboard,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffDashboard,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "service_execution_health_gate_trend_handoff_monitor_handoff_history_empty"
                    .to_owned(),
            );
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_handoff_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_handoff_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_handoff_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_handoff_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "service_execution_health_gate_trend_handoff_monitor_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentClosedLoopExecutionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentClosedLoopExecutionHealthStatus::Watch, watch_reasons)
        } else {
            (AgentClosedLoopExecutionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentClosedLoopExecutionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentClosedLoopExecutionHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
    pub history: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory,
    pub appended_summary: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary,
    pub dashboard: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffDashboard,
    pub health: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealth,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
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

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder;

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        summary: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            service_execution_health_gate_trend_handoff_monitor_handoff_history_record_telemetry(
                &dashboard, &health,
            );

        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_handoff_with_health(
        &self,
        history: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        handoff: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffRecord,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
        self.record_summary_with_health(history, handoff.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision {
    pub requested_admitted: bool,
    pub handoff_health: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGate;

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        handoff: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffRecord,
        history_record: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision {
        let requested_admitted = handoff.is_admitted();
        let handoff_health = history_record.health.clone();
        let handoff_requires_repair =
            handoff_health.status == AgentClosedLoopExecutionHealthStatus::Repair;
        let requires_repair_first =
            handoff.gate_decision.requires_repair_first || handoff_requires_repair;
        let admitted = requested_admitted && !handoff_requires_repair;
        let repair_tasks = if handoff_requires_repair {
            handoff_health
                .reasons
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, reason)| {
                    service_execution_health_gate_trend_handoff_monitor_handoff_repair_task(
                        run_id.as_ref(),
                        index,
                        reason,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut next_queue = handoff.next_queue();
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let mut blocked_reasons = handoff.gate_decision.blocked_reasons.clone();
        if handoff_requires_repair {
            blocked_reasons.extend(handoff_health.reasons.clone());
        }
        let telemetry = service_execution_health_gate_trend_handoff_monitor_handoff_gate_telemetry(
            handoff_health.status,
            requested_admitted,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            &blocked_reasons,
        );

        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision {
            requested_admitted,
            handoff_health,
            admitted,
            requires_repair_first,
            repair_tasks,
            next_queue,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoffRecord {
    pub handoff: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffRecord,
    pub history_record:
        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord,
    pub gate_decision: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoff {
    history_recorder:
        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder,
    gate: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGate,
}

impl AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        handoff: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffRecord,
        history: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        policy: AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoffRecord {
        let history_record = self
            .history_recorder
            .record_handoff_with_health(history, &handoff, policy);
        let gate_decision = self.gate.evaluate(run_id, &handoff, &history_record);
        let telemetry =
            service_execution_health_gate_trend_handoff_monitor_handoff_handoff_telemetry(
                &handoff,
                &history_record,
                &gate_decision,
            );

        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoffRecord {
            handoff,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateTrendHandoff;

impl AgentServiceExecutionHealthGateTrendHandoff {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_and_gate(
        &self,
        history: AgentServiceExecutionHealthGateHistory,
        summary: AgentServiceExecutionHealthGateSummary,
        policy: AgentServiceExecutionHealthGateHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentServiceExecutionHealthGateTrendHandoffRecord {
        let trend_record = AgentServiceExecutionHealthGateHistoryRecorder::new()
            .record_summary_with_health(history, summary, policy);
        self.gate_record(run_id, trend_record, next_queue)
    }

    pub fn record_gate_record_and_gate(
        &self,
        history: AgentServiceExecutionHealthGateHistory,
        record: &AgentServiceExecutionHealthGateRecord,
        policy: AgentServiceExecutionHealthGateHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentServiceExecutionHealthGateTrendHandoffRecord {
        let trend_record = AgentServiceExecutionHealthGateHistoryRecorder::new()
            .record_gate_record_with_health(history, record, policy);
        self.gate_record(run_id, trend_record, next_queue)
    }

    pub fn gate_record(
        &self,
        run_id: impl AsRef<str>,
        trend_record: AgentServiceExecutionHealthGateHistoryRecord,
        next_queue: &AgentTaskQueue,
    ) -> AgentServiceExecutionHealthGateTrendHandoffRecord {
        let gate_decision = AgentServiceExecutionHealthGateTrendGate::new().evaluate(
            run_id,
            &trend_record.health,
            next_queue,
        );
        let gate_summary = gate_decision.summary();
        let telemetry =
            service_execution_health_gate_trend_handoff_telemetry(&trend_record, &gate_summary);

        AgentServiceExecutionHealthGateTrendHandoffRecord {
            trend_record,
            gate_decision,
            gate_summary,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGateTrendGate;

impl AgentServiceExecutionHealthGateTrendGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        health: &AgentServiceExecutionHealthGateHealth,
        next_queue: &AgentTaskQueue,
    ) -> AgentServiceExecutionHealthGateDecision {
        let requires_repair_first = health.status == AgentClosedLoopExecutionHealthStatus::Repair;
        let admitted = !requires_repair_first;
        let repair_tasks = if requires_repair_first {
            health
                .reasons
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, reason)| {
                    service_execution_health_gate_trend_repair_task(run_id.as_ref(), index, reason)
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut merged_queue = next_queue.clone();
        for task in &repair_tasks {
            merged_queue.push(task.clone());
        }
        let blocked_reasons = if requires_repair_first {
            health.reasons.clone()
        } else {
            Vec::new()
        };
        let telemetry = service_execution_health_gate_trend_gate_telemetry(
            health.status,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            merged_queue.len(),
            &blocked_reasons,
        );

        AgentServiceExecutionHealthGateDecision {
            health_status: health.status,
            admitted,
            requires_repair_first,
            repair_tasks,
            next_queue: merged_queue,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentServiceExecutionHealthGate;

impl AgentServiceExecutionHealthGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        health: &AgentServiceExecutionHealth,
        next_queue: &AgentTaskQueue,
    ) -> AgentServiceExecutionHealthGateDecision {
        let requires_repair_first = health.status == AgentClosedLoopExecutionHealthStatus::Repair;
        let admitted = !requires_repair_first;
        let repair_tasks = if requires_repair_first {
            health
                .reasons
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, reason)| {
                    service_execution_health_repair_task(run_id.as_ref(), index, reason)
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut merged_queue = next_queue.clone();
        for task in &repair_tasks {
            merged_queue.push(task.clone());
        }
        let blocked_reasons = if requires_repair_first {
            health.reasons.clone()
        } else {
            Vec::new()
        };
        let telemetry = service_execution_health_gate_telemetry(
            health.status,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            merged_queue.len(),
            &blocked_reasons,
        );

        AgentServiceExecutionHealthGateDecision {
            health_status: health.status,
            admitted,
            requires_repair_first,
            repair_tasks,
            next_queue: merged_queue,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentServiceCommandPlanner;

impl AgentServiceCommandPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(&self, business_plan: &AgentBusinessLoopPlan) -> AgentServiceCommandPlan {
        let mut commands = Vec::new();

        match business_plan.status() {
            AgentCycleLedgerAdmissionStatus::Promote => {
                if let Some(candidate) = business_plan.adaptive_state_candidate.clone() {
                    commands.push(AgentServiceCommand::PromoteAdaptiveState(candidate));
                } else {
                    commands.push(AgentServiceCommand::HoldBusinessLoop {
                        reasons: vec!["adaptive_state_candidate_missing".to_owned()],
                    });
                }
            }
            AgentCycleLedgerAdmissionStatus::Hold => {
                commands.push(AgentServiceCommand::HoldBusinessLoop {
                    reasons: business_plan.admission.reasons.clone(),
                });
            }
            AgentCycleLedgerAdmissionStatus::Repair => {
                commands.push(AgentServiceCommand::OpenRepairMode {
                    reasons: business_plan.admission.reasons.clone(),
                });
                let validation_commands =
                    rust_validation_commands_for_reasons(&business_plan.admission.reasons);
                if !validation_commands.is_empty() {
                    commands.push(AgentServiceCommand::RunRustValidation {
                        commands: validation_commands,
                        reasons: business_plan.admission.reasons.clone(),
                    });
                }
            }
        }

        if !business_plan.next_queue.is_empty() {
            commands.push(AgentServiceCommand::EnqueueTasks(
                business_plan.next_queue.clone(),
            ));
        }

        commands.extend(
            business_plan
                .telemetry
                .iter()
                .cloned()
                .map(AgentServiceCommand::EmitTelemetry),
        );

        AgentServiceCommandPlan { commands }
    }

    pub fn audit(
        &self,
        plan: &AgentServiceCommandPlan,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentServiceCommandAudit {
        let mut remaining = receipts.clone();
        let mut missing = Vec::new();
        let expected = plan
            .commands
            .iter()
            .map(|command| command.kind().to_owned())
            .collect::<Vec<_>>();

        for command in &plan.commands {
            let Some(index) = remaining
                .iter()
                .position(|receipt| receipt.command_kind == command.kind())
            else {
                missing.push(command.kind().to_owned());
                continue;
            };
            remaining.remove(index);
        }

        let failed = receipts
            .iter()
            .filter(|receipt| receipt.status == AgentServiceCommandStatus::Failed)
            .cloned()
            .collect::<Vec<_>>();
        let skipped = receipts
            .iter()
            .filter(|receipt| receipt.status == AgentServiceCommandStatus::Skipped)
            .cloned()
            .collect::<Vec<_>>();

        AgentServiceCommandAudit {
            expected,
            receipts,
            missing,
            failed,
            skipped,
        }
    }

    pub fn close_execution(
        &self,
        run_id: impl AsRef<str>,
        business_plan: &AgentBusinessLoopPlan,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentServiceExecutionReport {
        let command_plan = self.plan(business_plan);
        let audit = self.audit(&command_plan, receipts);
        let feedback = AgentServiceFeedback::from_audit(run_id, audit.clone());
        let turnover = AgentServiceTurnover::from_feedback(business_plan, feedback.clone());

        AgentServiceExecutionReport {
            command_plan,
            audit,
            feedback,
            turnover,
        }
    }
}

fn repair_task(run_id: &str, index: usize, reason: String) -> AgentTask {
    let command_kind = service_command_kind(&reason);
    AgentTask::new(
        format!(
            "service-feedback-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(command_kind)
        ),
        repair_role(command_kind),
        format!("repair service command execution: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("service-feedback")
    .with_priority(8)
}

fn service_execution_health_repair_task(run_id: &str, index: usize, reason: String) -> AgentTask {
    AgentTask::new(
        format!(
            "service-execution-health-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair service execution health: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("service-execution-health")
    .with_priority(9)
}

fn service_execution_health_gate_trend_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "service-execution-health-gate-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair service execution health gate trend: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("service-execution-health-gate")
    .with_priority(9)
}

fn service_execution_health_gate_trend_handoff_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "service-execution-health-gate-handoff-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair service execution health gate trend handoff: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("service-execution-health-gate-handoff")
    .with_priority(9)
}

fn service_execution_health_gate_trend_handoff_monitor_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "service-execution-health-gate-handoff-monitor-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair service execution health gate trend handoff monitor: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("service-execution-health-gate-handoff-monitor")
    .with_priority(9)
}

fn service_execution_health_gate_trend_handoff_monitor_handoff_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "service-execution-health-gate-handoff-monitor-handoff-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair service execution health gate trend handoff monitor handoff: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("service-execution-health-gate-handoff-monitor-handoff")
    .with_priority(9)
}

fn service_command_kind(reason: &str) -> &str {
    let value = reason
        .strip_prefix("service_command_missing=")
        .or_else(|| reason.strip_prefix("service_command_failed="))
        .or_else(|| reason.strip_prefix("service_command_skipped="))
        .unwrap_or(reason);
    value.split(':').next().unwrap_or(value)
}

fn repair_role(command_kind: &str) -> AgentRole {
    match command_kind {
        "promote_adaptive_state" => AgentRole::MemoryCurator,
        "run_rust_validation" => AgentRole::Tester,
        "enqueue_tasks" => AgentRole::Planner,
        "emit_telemetry" => AgentRole::Aggregator,
        "open_repair_mode" | "hold_business_loop" => AgentRole::Reviewer,
        _ => AgentRole::Reviewer,
    }
}

fn rust_validation_commands_for_reasons(reasons: &[String]) -> Vec<AgentRustValidationCommand> {
    if !reasons
        .iter()
        .any(|reason| reason.starts_with("tool_build"))
    {
        return Vec::new();
    }
    vec![
        AgentRustValidationCommand::Format,
        AgentRustValidationCommand::Check,
        AgentRustValidationCommand::Test,
        AgentRustValidationCommand::Benchmark,
    ]
}

fn stable_id(raw: &str) -> String {
    let id = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    if id.is_empty() { "run".to_owned() } else { id }
}

fn service_command_plan_summary_telemetry(
    command_count: usize,
    requires_adaptive_state_write: bool,
    repair_mode_requested: bool,
    hold_requested: bool,
    enqueue_commands: usize,
    enqueued_tasks: usize,
    reason_count: usize,
    memory_promotion_reason_count: usize,
    tool_build_reason_count: usize,
    rust_validation_commands: usize,
    telemetry_commands: usize,
) -> Vec<String> {
    vec![
        "agent_service_command_plan_summary=true".to_owned(),
        format!("agent_service_command_plan_summary_commands={command_count}"),
        format!(
            "agent_service_command_plan_summary_adaptive_write={requires_adaptive_state_write}"
        ),
        format!("agent_service_command_plan_summary_repair_mode={repair_mode_requested}"),
        format!("agent_service_command_plan_summary_hold={hold_requested}"),
        format!("agent_service_command_plan_summary_enqueue_commands={enqueue_commands}"),
        format!("agent_service_command_plan_summary_enqueued_tasks={enqueued_tasks}"),
        format!("agent_service_command_plan_summary_reasons={reason_count}"),
        format!(
            "agent_service_command_plan_summary_memory_promotion_reasons={memory_promotion_reason_count}"
        ),
        format!("agent_service_command_plan_summary_tool_build_reasons={tool_build_reason_count}"),
        format!(
            "agent_service_command_plan_summary_rust_validation_commands={rust_validation_commands}"
        ),
        format!("agent_service_command_plan_summary_telemetry_commands={telemetry_commands}"),
    ]
}

fn service_command_plan_dashboard_telemetry(
    total_plans: usize,
    command_count: usize,
    adaptive_write_plans: usize,
    repair_mode_plans: usize,
    hold_plans: usize,
    enqueue_plans: usize,
    enqueue_command_count: usize,
    enqueued_task_count: usize,
    reason_count: usize,
    memory_promotion_reason_count: usize,
    memory_promotion_reason_plans: usize,
    tool_build_reason_count: usize,
    tool_build_reason_plans: usize,
    telemetry_command_count: usize,
    adaptive_write_rate: f32,
    repair_or_hold_rate: f32,
    enqueue_rate: f32,
) -> Vec<String> {
    vec![
        "agent_service_command_plan_dashboard=true".to_owned(),
        format!("agent_service_command_plan_dashboard_plans={total_plans}"),
        format!("agent_service_command_plan_dashboard_commands={command_count}"),
        format!("agent_service_command_plan_dashboard_adaptive_write_plans={adaptive_write_plans}"),
        format!("agent_service_command_plan_dashboard_repair_mode_plans={repair_mode_plans}"),
        format!("agent_service_command_plan_dashboard_hold_plans={hold_plans}"),
        format!("agent_service_command_plan_dashboard_enqueue_plans={enqueue_plans}"),
        format!("agent_service_command_plan_dashboard_enqueue_commands={enqueue_command_count}"),
        format!("agent_service_command_plan_dashboard_enqueued_tasks={enqueued_task_count}"),
        format!("agent_service_command_plan_dashboard_reasons={reason_count}"),
        format!(
            "agent_service_command_plan_dashboard_memory_promotion_reasons={memory_promotion_reason_count}"
        ),
        format!(
            "agent_service_command_plan_dashboard_memory_promotion_reason_plans={memory_promotion_reason_plans}"
        ),
        format!(
            "agent_service_command_plan_dashboard_tool_build_reasons={tool_build_reason_count}"
        ),
        format!(
            "agent_service_command_plan_dashboard_tool_build_reason_plans={tool_build_reason_plans}"
        ),
        format!(
            "agent_service_command_plan_dashboard_telemetry_commands={telemetry_command_count}"
        ),
        format!(
            "agent_service_command_plan_dashboard_adaptive_write_rate={adaptive_write_rate:.3}"
        ),
        format!(
            "agent_service_command_plan_dashboard_repair_or_hold_rate={repair_or_hold_rate:.3}"
        ),
        format!("agent_service_command_plan_dashboard_enqueue_rate={enqueue_rate:.3}"),
    ]
}

fn service_command_plan_history_record_telemetry(
    dashboard: &AgentServiceCommandPlanDashboard,
    health: &AgentServiceCommandPlanHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_command_plan_history_record=true".to_owned(),
        format!(
            "agent_service_command_plan_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_service_command_plan_history_record_plans={}",
            dashboard.total_plans
        ),
        format!(
            "agent_service_command_plan_history_record_commands={}",
            dashboard.command_count
        ),
        format!(
            "agent_service_command_plan_history_record_repair_or_hold_rate={:.3}",
            dashboard.repair_or_hold_rate
        ),
        format!(
            "agent_service_command_plan_history_record_enqueued_tasks={}",
            dashboard.enqueued_task_count
        ),
        format!(
            "agent_service_command_plan_history_record_adaptive_write_rate={:.3}",
            dashboard.adaptive_write_rate
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_service_command_plan_history_record_reason={reason}")),
    );
    telemetry
}

fn service_command_audit_summary_telemetry(
    expected_commands: usize,
    receipts: usize,
    missing_commands: usize,
    failed_commands: usize,
    skipped_commands: usize,
    clean: bool,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_service_command_audit_summary=true".to_owned(),
        format!("agent_service_command_audit_summary_expected={expected_commands}"),
        format!("agent_service_command_audit_summary_receipts={receipts}"),
        format!("agent_service_command_audit_summary_missing={missing_commands}"),
        format!("agent_service_command_audit_summary_failed={failed_commands}"),
        format!("agent_service_command_audit_summary_skipped={skipped_commands}"),
        format!("agent_service_command_audit_summary_clean={clean}"),
        format!("agent_service_command_audit_summary_blocked_reasons={blocked_reasons}"),
    ]
}

fn service_command_audit_dashboard_telemetry(
    total_audits: usize,
    clean_audits: usize,
    dirty_audits: usize,
    expected_command_count: usize,
    receipt_count: usize,
    drift_event_count: usize,
    blocked_reason_count: usize,
    clean_rate: f32,
    drift_rate: f32,
) -> Vec<String> {
    vec![
        "agent_service_command_audit_dashboard=true".to_owned(),
        format!("agent_service_command_audit_dashboard_audits={total_audits}"),
        format!("agent_service_command_audit_dashboard_clean_audits={clean_audits}"),
        format!("agent_service_command_audit_dashboard_dirty_audits={dirty_audits}"),
        format!("agent_service_command_audit_dashboard_expected_commands={expected_command_count}"),
        format!("agent_service_command_audit_dashboard_receipts={receipt_count}"),
        format!("agent_service_command_audit_dashboard_drift_events={drift_event_count}"),
        format!("agent_service_command_audit_dashboard_blocked_reasons={blocked_reason_count}"),
        format!("agent_service_command_audit_dashboard_clean_rate={clean_rate:.3}"),
        format!("agent_service_command_audit_dashboard_drift_rate={drift_rate:.3}"),
    ]
}

fn service_command_audit_history_record_telemetry(
    dashboard: &AgentServiceCommandAuditDashboard,
    health: &AgentServiceCommandAuditHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_command_audit_history_record=true".to_owned(),
        format!(
            "agent_service_command_audit_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_service_command_audit_history_record_audits={}",
            dashboard.total_audits
        ),
        format!(
            "agent_service_command_audit_history_record_clean_rate={:.3}",
            dashboard.clean_rate
        ),
        format!(
            "agent_service_command_audit_history_record_drift_rate={:.3}",
            dashboard.drift_rate
        ),
        format!(
            "agent_service_command_audit_history_record_blocked_reasons={}",
            dashboard.blocked_reason_count
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_service_command_audit_history_record_reason={reason}")),
    );
    telemetry
}

fn service_feedback_summary_telemetry(
    audit_clean: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    clean: bool,
) -> Vec<String> {
    vec![
        "agent_service_feedback_summary=true".to_owned(),
        format!("agent_service_feedback_summary_audit_clean={audit_clean}"),
        format!("agent_service_feedback_summary_repair_tasks={repair_tasks}"),
        format!("agent_service_feedback_summary_next_queue_tasks={next_queue_tasks}"),
        format!("agent_service_feedback_summary_blocked_reasons={blocked_reasons}"),
        format!("agent_service_feedback_summary_clean={clean}"),
    ]
}

fn service_feedback_dashboard_telemetry(
    total_feedbacks: usize,
    clean_feedbacks: usize,
    dirty_feedbacks: usize,
    audit_dirty_feedbacks: usize,
    repair_task_count: usize,
    total_next_queue_tasks: usize,
    blocked_reason_count: usize,
    clean_rate: f32,
    repair_task_rate: f32,
) -> Vec<String> {
    vec![
        "agent_service_feedback_dashboard=true".to_owned(),
        format!("agent_service_feedback_dashboard_feedbacks={total_feedbacks}"),
        format!("agent_service_feedback_dashboard_clean_feedbacks={clean_feedbacks}"),
        format!("agent_service_feedback_dashboard_dirty_feedbacks={dirty_feedbacks}"),
        format!("agent_service_feedback_dashboard_audit_dirty={audit_dirty_feedbacks}"),
        format!("agent_service_feedback_dashboard_repair_tasks={repair_task_count}"),
        format!("agent_service_feedback_dashboard_next_queue_tasks={total_next_queue_tasks}"),
        format!("agent_service_feedback_dashboard_blocked_reasons={blocked_reason_count}"),
        format!("agent_service_feedback_dashboard_clean_rate={clean_rate:.3}"),
        format!("agent_service_feedback_dashboard_repair_task_rate={repair_task_rate:.3}"),
    ]
}

fn service_feedback_history_record_telemetry(
    dashboard: &AgentServiceFeedbackDashboard,
    health: &AgentServiceFeedbackHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_feedback_history_record=true".to_owned(),
        format!(
            "agent_service_feedback_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_service_feedback_history_record_feedbacks={}",
            dashboard.total_feedbacks
        ),
        format!(
            "agent_service_feedback_history_record_clean_rate={:.3}",
            dashboard.clean_rate
        ),
        format!(
            "agent_service_feedback_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_service_feedback_history_record_blocked_reasons={}",
            dashboard.blocked_reason_count
        ),
        format!(
            "agent_service_feedback_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_service_feedback_history_record_reason={reason}")),
    );
    telemetry
}

fn service_turnover_summary_telemetry(
    feedback_clean: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    clean: bool,
) -> Vec<String> {
    vec![
        "agent_service_turnover_summary=true".to_owned(),
        format!("agent_service_turnover_summary_feedback_clean={feedback_clean}"),
        format!("agent_service_turnover_summary_repair_tasks={repair_tasks}"),
        format!("agent_service_turnover_summary_next_queue_tasks={next_queue_tasks}"),
        format!("agent_service_turnover_summary_blocked_reasons={blocked_reasons}"),
        format!("agent_service_turnover_summary_clean={clean}"),
    ]
}

fn service_turnover_dashboard_telemetry(
    total_turnovers: usize,
    clean_turnovers: usize,
    dirty_turnovers: usize,
    feedback_dirty_turnovers: usize,
    repair_task_count: usize,
    total_next_queue_tasks: usize,
    blocked_reason_count: usize,
    clean_rate: f32,
    repair_task_rate: f32,
    next_queue_task_rate: f32,
) -> Vec<String> {
    vec![
        "agent_service_turnover_dashboard=true".to_owned(),
        format!("agent_service_turnover_dashboard_turnovers={total_turnovers}"),
        format!("agent_service_turnover_dashboard_clean_turnovers={clean_turnovers}"),
        format!("agent_service_turnover_dashboard_dirty_turnovers={dirty_turnovers}"),
        format!("agent_service_turnover_dashboard_feedback_dirty={feedback_dirty_turnovers}"),
        format!("agent_service_turnover_dashboard_repair_tasks={repair_task_count}"),
        format!("agent_service_turnover_dashboard_next_queue_tasks={total_next_queue_tasks}"),
        format!("agent_service_turnover_dashboard_blocked_reasons={blocked_reason_count}"),
        format!("agent_service_turnover_dashboard_clean_rate={clean_rate:.3}"),
        format!("agent_service_turnover_dashboard_repair_task_rate={repair_task_rate:.3}"),
        format!("agent_service_turnover_dashboard_next_queue_task_rate={next_queue_task_rate:.3}"),
    ]
}

fn service_turnover_history_record_telemetry(
    dashboard: &AgentServiceTurnoverDashboard,
    health: &AgentServiceTurnoverHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_turnover_history_record=true".to_owned(),
        format!(
            "agent_service_turnover_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_service_turnover_history_record_turnovers={}",
            dashboard.total_turnovers
        ),
        format!(
            "agent_service_turnover_history_record_clean_rate={:.3}",
            dashboard.clean_rate
        ),
        format!(
            "agent_service_turnover_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_service_turnover_history_record_blocked_reasons={}",
            dashboard.blocked_reason_count
        ),
        format!(
            "agent_service_turnover_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_service_turnover_history_record_reason={reason}")),
    );
    telemetry
}

fn service_execution_report_summary_telemetry(
    command_count: usize,
    expected_commands: usize,
    receipts: usize,
    missing_commands: usize,
    failed_commands: usize,
    skipped_commands: usize,
    memory_promotion_reason_count: usize,
    tool_build_reason_count: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    clean: bool,
) -> Vec<String> {
    vec![
        "agent_service_execution_report_summary=true".to_owned(),
        format!("agent_service_execution_report_summary_commands={command_count}"),
        format!("agent_service_execution_report_summary_expected={expected_commands}"),
        format!("agent_service_execution_report_summary_receipts={receipts}"),
        format!("agent_service_execution_report_summary_missing={missing_commands}"),
        format!("agent_service_execution_report_summary_failed={failed_commands}"),
        format!("agent_service_execution_report_summary_skipped={skipped_commands}"),
        format!(
            "agent_service_execution_report_summary_memory_promotion_reasons={memory_promotion_reason_count}"
        ),
        format!(
            "agent_service_execution_report_summary_tool_build_reasons={tool_build_reason_count}"
        ),
        format!("agent_service_execution_report_summary_repair_tasks={repair_tasks}"),
        format!("agent_service_execution_report_summary_next_queue_tasks={next_queue_tasks}"),
        format!("agent_service_execution_report_summary_blocked_reasons={blocked_reasons}"),
        format!("agent_service_execution_report_summary_clean={clean}"),
    ]
}

fn service_execution_rate(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn service_execution_dashboard_telemetry(
    total_runs: usize,
    clean_runs: usize,
    dirty_runs: usize,
    command_count: usize,
    service_drift_events: usize,
    memory_promotion_reason_count: usize,
    memory_promotion_reason_runs: usize,
    tool_build_reason_count: usize,
    tool_build_reason_runs: usize,
    repair_task_count: usize,
    total_next_queue_tasks: usize,
    clean_rate: f32,
    service_drift_rate: f32,
) -> Vec<String> {
    vec![
        "agent_service_execution_dashboard=true".to_owned(),
        format!("agent_service_execution_dashboard_runs={total_runs}"),
        format!("agent_service_execution_dashboard_clean_runs={clean_runs}"),
        format!("agent_service_execution_dashboard_dirty_runs={dirty_runs}"),
        format!("agent_service_execution_dashboard_commands={command_count}"),
        format!("agent_service_execution_dashboard_drift_events={service_drift_events}"),
        format!(
            "agent_service_execution_dashboard_memory_promotion_reasons={memory_promotion_reason_count}"
        ),
        format!(
            "agent_service_execution_dashboard_memory_promotion_reason_runs={memory_promotion_reason_runs}"
        ),
        format!("agent_service_execution_dashboard_tool_build_reasons={tool_build_reason_count}"),
        format!(
            "agent_service_execution_dashboard_tool_build_reason_runs={tool_build_reason_runs}"
        ),
        format!("agent_service_execution_dashboard_repair_tasks={repair_task_count}"),
        format!("agent_service_execution_dashboard_next_queue_tasks={total_next_queue_tasks}"),
        format!("agent_service_execution_dashboard_clean_rate={clean_rate:.3}"),
        format!("agent_service_execution_dashboard_drift_rate={service_drift_rate:.3}"),
    ]
}

fn service_execution_health_record_telemetry(
    dashboard: &AgentServiceExecutionDashboard,
    health: &AgentServiceExecutionHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_record=true".to_owned(),
        format!(
            "agent_service_execution_health_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_service_execution_health_record_runs={}",
            dashboard.total_runs
        ),
        format!(
            "agent_service_execution_health_record_clean_rate={:.3}",
            dashboard.clean_rate
        ),
        format!(
            "agent_service_execution_health_record_drift_rate={:.3}",
            dashboard.service_drift_rate
        ),
        format!(
            "agent_service_execution_health_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_service_execution_health_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_service_execution_health_record_reason={reason}")),
    );
    telemetry
}

fn service_execution_health_gate_telemetry(
    health_status: AgentClosedLoopExecutionHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_status={}",
            health_status.as_str()
        ),
        format!("agent_service_execution_health_gate_admitted={admitted}"),
        format!(
            "agent_service_execution_health_gate_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_service_execution_health_gate_repair_tasks={repair_tasks}"),
        format!("agent_service_execution_health_gate_next_queue_tasks={next_queue_tasks}"),
        format!(
            "agent_service_execution_health_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        blocked_reasons
            .iter()
            .map(|reason| format!("agent_service_execution_health_gate_reason={reason}")),
    );
    telemetry
}

fn service_execution_health_gate_trend_gate_telemetry(
    health_status: AgentClosedLoopExecutionHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_trend_gate=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_gate_status={}",
            health_status.as_str()
        ),
        format!("agent_service_execution_health_gate_trend_gate_admitted={admitted}"),
        format!(
            "agent_service_execution_health_gate_trend_gate_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_service_execution_health_gate_trend_gate_repair_tasks={repair_tasks}"),
        format!(
            "agent_service_execution_health_gate_trend_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        blocked_reasons.iter().map(|reason| {
            format!("agent_service_execution_health_gate_trend_gate_reason={reason}")
        }),
    );
    telemetry
}

fn service_execution_health_gate_summary_telemetry(
    health_status: AgentClosedLoopExecutionHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_service_execution_health_gate_summary=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_summary_status={}",
            health_status.as_str()
        ),
        format!("agent_service_execution_health_gate_summary_admitted={admitted}"),
        format!(
            "agent_service_execution_health_gate_summary_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_service_execution_health_gate_summary_repair_tasks={repair_tasks}"),
        format!("agent_service_execution_health_gate_summary_next_queue_tasks={next_queue_tasks}"),
        format!("agent_service_execution_health_gate_summary_blocked_reasons={blocked_reasons}"),
    ]
}

fn service_execution_health_gate_dashboard_telemetry(
    total_records: usize,
    admitted_records: usize,
    repair_first_records: usize,
    repair_task_count: usize,
    total_next_queue_tasks: usize,
    admission_rate: f32,
    repair_first_rate: f32,
    latest_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
    latest_blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_service_execution_health_gate_dashboard=true".to_owned(),
        format!("agent_service_execution_health_gate_dashboard_records={total_records}"),
        format!("agent_service_execution_health_gate_dashboard_admitted={admitted_records}"),
        format!(
            "agent_service_execution_health_gate_dashboard_repair_first={repair_first_records}"
        ),
        format!("agent_service_execution_health_gate_dashboard_repair_tasks={repair_task_count}"),
        format!(
            "agent_service_execution_health_gate_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!("agent_service_execution_health_gate_dashboard_admission_rate={admission_rate:.3}"),
        format!(
            "agent_service_execution_health_gate_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_service_execution_health_gate_dashboard_latest_status={}",
            latest_health_status
                .map(AgentClosedLoopExecutionHealthStatus::as_str)
                .unwrap_or("none")
        ),
        format!(
            "agent_service_execution_health_gate_dashboard_latest_blocked_reasons={latest_blocked_reasons}"
        ),
    ]
}

fn service_execution_health_gate_history_record_telemetry(
    dashboard: &AgentServiceExecutionHealthGateDashboard,
    health: &AgentServiceExecutionHealthGateHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_history_record=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_service_execution_health_gate_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_service_execution_health_gate_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_service_execution_health_gate_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_service_execution_health_gate_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!("agent_service_execution_health_gate_history_record_reason={reason}")
    }));
    telemetry
}

fn service_execution_health_gate_trend_handoff_telemetry(
    trend_record: &AgentServiceExecutionHealthGateHistoryRecord,
    gate_summary: &AgentServiceExecutionHealthGateSummary,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_trend_handoff=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_health_status={}",
            trend_record.health.status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_records={}",
            trend_record.dashboard.total_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_admitted={}",
            gate_summary.admitted
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_requires_repair_first={}",
            gate_summary.requires_repair_first
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_repair_tasks={}",
            gate_summary.repair_tasks
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_next_queue_tasks={}",
            gate_summary.next_queue_tasks
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_blocked_reasons={}",
            gate_summary.blocked_reasons.len()
        ),
    ];
    telemetry.extend(trend_record.telemetry.iter().cloned());
    telemetry.extend(gate_summary.telemetry.iter().cloned());
    telemetry
}

fn service_execution_health_gate_trend_handoff_summary_telemetry(
    trend_health_status: AgentClosedLoopExecutionHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    trend_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_service_execution_health_gate_trend_handoff_summary=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_summary_status={}",
            trend_health_status.as_str()
        ),
        format!("agent_service_execution_health_gate_trend_handoff_summary_admitted={admitted}"),
        format!(
            "agent_service_execution_health_gate_trend_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_summary_records={trend_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

fn service_execution_health_gate_trend_handoff_dashboard_telemetry(
    total_records: usize,
    admitted_records: usize,
    repair_first_records: usize,
    stable_records: usize,
    watch_records: usize,
    repair_records: usize,
    repair_task_count: usize,
    total_next_queue_tasks: usize,
    admission_rate: f32,
    repair_first_rate: f32,
    latest_trend_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
    latest_blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_service_execution_health_gate_trend_handoff_dashboard=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_records={total_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_latest_status={}",
            latest_trend_health_status
                .map(AgentClosedLoopExecutionHealthStatus::as_str)
                .unwrap_or("none")
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_dashboard_latest_blocked_reasons={latest_blocked_reasons}"
        ),
    ]
}

fn service_execution_health_gate_trend_handoff_history_record_telemetry(
    dashboard: &AgentServiceExecutionHealthGateTrendHandoffDashboard,
    health: &AgentServiceExecutionHealthGateTrendHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_trend_handoff_history_record=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_history_record_stable_records={}",
            dashboard.stable_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_history_record_watch_records={}",
            dashboard.watch_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_history_record_repair_records={}",
            dashboard.repair_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!("agent_service_execution_health_gate_trend_handoff_history_record_reason={reason}")
    }));
    telemetry
}

fn service_execution_health_gate_trend_handoff_gate_telemetry(
    handoff_health_status: AgentClosedLoopExecutionHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_trend_handoff_gate=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_gate_status={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_gate_requested_admitted={requested_admitted}"
        ),
        format!("agent_service_execution_health_gate_trend_handoff_gate_admitted={admitted}"),
        format!(
            "agent_service_execution_health_gate_trend_handoff_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!("agent_service_execution_health_gate_trend_handoff_gate_reason={reason}")
    }));
    telemetry
}

fn service_execution_health_gate_trend_handoff_monitor_telemetry(
    handoff: &AgentServiceExecutionHealthGateTrendHandoffRecord,
    history_record: &AgentServiceExecutionHealthGateTrendHandoffHistoryRecord,
    gate_decision: &AgentServiceExecutionHealthGateTrendHandoffGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_trend_handoff_monitor=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_status={}",
            handoff.trend_record.health.status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn service_execution_health_gate_trend_handoff_monitor_summary_telemetry(
    handoff_health_status: AgentClosedLoopExecutionHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    handoff_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_service_execution_health_gate_trend_handoff_monitor_summary=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_summary_status={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_summary_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_summary_admitted={admitted}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_summary_records={handoff_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

fn service_execution_health_gate_trend_handoff_monitor_dashboard_telemetry(
    total_records: usize,
    requested_admitted_records: usize,
    admitted_records: usize,
    repair_first_records: usize,
    stable_records: usize,
    watch_records: usize,
    repair_records: usize,
    repair_task_count: usize,
    total_next_queue_tasks: usize,
    blocked_reasons: usize,
    admission_rate: f32,
    repair_first_rate: f32,
    latest_handoff_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
) -> Vec<String> {
    vec![
        "agent_service_execution_health_gate_trend_handoff_monitor_dashboard=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_records={total_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_requested_admitted={requested_admitted_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_latest_status={}",
            latest_handoff_health_status
                .map(AgentClosedLoopExecutionHealthStatus::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn service_execution_health_gate_trend_handoff_monitor_history_record_telemetry(
    dashboard: &AgentServiceExecutionHealthGateTrendHandoffMonitorDashboard,
    health: &AgentServiceExecutionHealthGateTrendHandoffMonitorHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_trend_handoff_monitor_history_record=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_history_record_repair_records={}",
            dashboard.repair_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_history_record_watch_records={}",
            dashboard.watch_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!("agent_service_execution_health_gate_trend_handoff_monitor_history_record_reason={reason}")
    }));
    telemetry
}

fn service_execution_health_gate_trend_handoff_monitor_gate_telemetry(
    monitor_health_status: AgentClosedLoopExecutionHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_trend_handoff_monitor_gate=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_gate_status={}",
            monitor_health_status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_gate_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_gate_admitted={admitted}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!("agent_service_execution_health_gate_trend_handoff_monitor_gate_reason={reason}")
    }));
    telemetry
}

fn service_execution_health_gate_trend_handoff_monitor_handoff_telemetry(
    monitor: &AgentServiceExecutionHealthGateTrendHandoffMonitorRecord,
    history_record: &AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecord,
    gate_decision: &AgentServiceExecutionHealthGateTrendHandoffMonitorGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_trend_handoff_monitor_handoff=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(monitor.telemetry.iter().cloned());
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn service_execution_health_gate_trend_handoff_monitor_handoff_summary_telemetry(
    monitor_health_status: AgentClosedLoopExecutionHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    monitor_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_service_execution_health_gate_trend_handoff_monitor_handoff_summary=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_summary_status={}",
            monitor_health_status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_summary_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_summary_admitted={admitted}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_summary_records={monitor_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

fn service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_telemetry(
    total_records: usize,
    requested_admitted_records: usize,
    admitted_records: usize,
    repair_first_records: usize,
    stable_records: usize,
    watch_records: usize,
    repair_records: usize,
    repair_task_count: usize,
    total_next_queue_tasks: usize,
    blocked_reasons: usize,
    admission_rate: f32,
    repair_first_rate: f32,
    latest_monitor_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
) -> Vec<String> {
    vec![
        "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard=true"
            .to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_records={total_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_requested_admitted={requested_admitted_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_dashboard_latest_status={}",
            latest_monitor_health_status
                .map(AgentClosedLoopExecutionHealthStatus::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn service_execution_health_gate_trend_handoff_monitor_handoff_history_record_telemetry(
    dashboard: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffDashboard,
    health: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record=true"
            .to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record_repair_records={}",
            dashboard.repair_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record_watch_records={}",
            dashboard.watch_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!("agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record_reason={reason}")
    }));
    telemetry
}

fn service_execution_health_gate_trend_handoff_monitor_handoff_gate_telemetry(
    handoff_health_status: AgentClosedLoopExecutionHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_trend_handoff_monitor_handoff_gate=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_gate_status={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_gate_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_gate_admitted={admitted}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_gate_reason={reason}"
        )
    }));
    telemetry
}

fn service_execution_health_gate_trend_handoff_monitor_handoff_handoff_telemetry(
    handoff: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffRecord,
    history_record: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord,
    gate_decision: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_trend_handoff_monitor_handoff_handoff=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_handoff_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_handoff_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_handoff_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_handoff_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_service_execution_health_gate_trend_handoff_monitor_handoff_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(handoff.telemetry.iter().cloned());
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn service_execution_health_gate_record_telemetry(
    health_record: &AgentServiceExecutionHealthRecord,
    gate_summary: &AgentServiceExecutionHealthGateSummary,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_service_execution_health_gate_record=true".to_owned(),
        format!(
            "agent_service_execution_health_gate_record_health_status={}",
            health_record.health.status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_record_gate_status={}",
            gate_summary.health_status.as_str()
        ),
        format!(
            "agent_service_execution_health_gate_record_runs={}",
            health_record.dashboard.total_runs
        ),
        format!(
            "agent_service_execution_health_gate_record_admitted={}",
            gate_summary.admitted
        ),
        format!(
            "agent_service_execution_health_gate_record_requires_repair_first={}",
            gate_summary.requires_repair_first
        ),
        format!(
            "agent_service_execution_health_gate_record_repair_tasks={}",
            gate_summary.repair_tasks
        ),
        format!(
            "agent_service_execution_health_gate_record_next_queue_tasks={}",
            gate_summary.next_queue_tasks
        ),
        format!(
            "agent_service_execution_health_gate_record_blocked_reasons={}",
            gate_summary.blocked_reasons.len()
        ),
    ];
    telemetry.extend(health_record.telemetry.iter().cloned());
    telemetry.extend(gate_summary.telemetry.iter().cloned());
    telemetry
}

fn rate(part: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        part as f32 / total as f32
    }
}

fn command_reasons(commands: &[AgentServiceCommand]) -> Vec<String> {
    commands
        .iter()
        .flat_map(|command| match command {
            AgentServiceCommand::HoldBusinessLoop { reasons }
            | AgentServiceCommand::OpenRepairMode { reasons } => reasons.clone(),
            _ => Vec::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::AgentBusinessLoopPlan;
    use crate::ledger::{
        AgentCycleLedgerAdmissionDecision, AgentCycleLedgerAdmissionStatus, AgentCycleLedgerSummary,
    };
    use crate::step::AgentClosedLoopExecutionHealthStatus;
    use crate::task::{AgentRole, AgentTask};

    fn summary() -> AgentCycleLedgerSummary {
        AgentCycleLedgerSummary {
            total_cycles: 1,
            accepted_cycles: 1,
            blocked_cycles: 0,
            adaptive_promotions: 1,
            enqueued_tasks: 0,
            memory_promotion_records: 0,
            memory_promotion_no_candidate_cycles: 0,
            memory_promotion_promotable_cycles: 0,
            memory_promotion_watch_cycles: 0,
            memory_promotion_blocked_cycles: 0,
            memory_promotion_repair_cycles: 0,
            tool_build_blocked_cycles: 0,
            consecutive_blocked_cycles: 0,
            acceptance_rate: 1.0,
            average_reward_total: 0.90,
            latest_run_id: Some("run-1".to_owned()),
            latest_blocked_reasons: Vec::new(),
        }
    }

    fn plan(
        status: AgentCycleLedgerAdmissionStatus,
        reasons: Vec<String>,
        next_queue: AgentTaskQueue,
        candidate: Option<AdaptiveStateCandidate>,
    ) -> AgentBusinessLoopPlan {
        AgentBusinessLoopPlan {
            admission: AgentCycleLedgerAdmissionDecision {
                status,
                reasons,
                summary: summary(),
            },
            next_queue,
            adaptive_state_candidate: candidate,
            telemetry: vec!["status=line".to_owned()],
        }
    }

    fn service_report_summary(
        clean: bool,
        command_counts: (usize, usize, usize, usize),
        repair_tasks: usize,
        next_queue_tasks: usize,
        blocked_reasons: Vec<&str>,
    ) -> AgentServiceExecutionReportSummary {
        AgentServiceExecutionReportSummary {
            command_count: command_counts.0,
            command_kinds: (0..command_counts.0)
                .map(|index| format!("command-{index}"))
                .collect(),
            memory_promotion_reason_count: 0,
            tool_build_reason_count: 0,
            expected_commands: command_counts.0,
            receipts: command_counts.0.saturating_sub(command_counts.1),
            missing_commands: command_counts.1,
            failed_commands: command_counts.2,
            skipped_commands: command_counts.3,
            repair_tasks,
            next_queue_tasks,
            blocked_reasons: blocked_reasons
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            clean,
            telemetry: vec![format!("fixture_clean={clean}")],
        }
    }

    fn trend_handoff_summary(
        status: AgentClosedLoopExecutionHealthStatus,
        admitted: bool,
        requires_repair_first: bool,
        repair_tasks: usize,
        next_queue_tasks: usize,
        blocked_reasons: Vec<&str>,
    ) -> AgentServiceExecutionHealthGateTrendHandoffSummary {
        AgentServiceExecutionHealthGateTrendHandoffSummary {
            trend_health_status: status,
            admitted,
            requires_repair_first,
            trend_records: 1,
            repair_tasks,
            next_queue_tasks,
            repair_task_ids: (0..repair_tasks)
                .map(|index| format!("repair-task-{index}"))
                .collect(),
            next_queue_task_ids: (0..next_queue_tasks)
                .map(|index| format!("queue-task-{index}"))
                .collect(),
            blocked_reasons: blocked_reasons
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            telemetry: vec![format!("fixture_trend_handoff_status={}", status.as_str())],
        }
    }

    fn trend_handoff_monitor_summary(
        status: AgentClosedLoopExecutionHealthStatus,
        requested_admitted: bool,
        admitted: bool,
        requires_repair_first: bool,
        repair_tasks: usize,
        next_queue_tasks: usize,
        blocked_reasons: usize,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorSummary {
        AgentServiceExecutionHealthGateTrendHandoffMonitorSummary {
            handoff_health_status: status,
            requested_admitted,
            admitted,
            requires_repair_first,
            handoff_records: 1,
            repair_tasks,
            next_queue_tasks,
            blocked_reasons,
            repair_task_ids: (0..repair_tasks)
                .map(|index| format!("monitor-repair-task-{index}"))
                .collect(),
            next_queue_task_ids: (0..next_queue_tasks)
                .map(|index| format!("monitor-queue-task-{index}"))
                .collect(),
            telemetry: vec![format!(
                "fixture_trend_handoff_monitor_status={}",
                status.as_str()
            )],
        }
    }

    fn trend_handoff_monitor_handoff_summary(
        status: AgentClosedLoopExecutionHealthStatus,
        requested_admitted: bool,
        admitted: bool,
        requires_repair_first: bool,
        repair_tasks: usize,
        next_queue_tasks: usize,
        blocked_reasons: usize,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary {
        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary {
            monitor_health_status: status,
            requested_admitted,
            admitted,
            requires_repair_first,
            monitor_records: 1,
            repair_tasks,
            next_queue_tasks,
            blocked_reasons,
            repair_task_ids: (0..repair_tasks)
                .map(|index| format!("monitor-handoff-repair-task-{index}"))
                .collect(),
            next_queue_task_ids: (0..next_queue_tasks)
                .map(|index| format!("monitor-handoff-queue-task-{index}"))
                .collect(),
            telemetry: vec![format!(
                "fixture_trend_handoff_monitor_handoff_status={}",
                status.as_str()
            )],
        }
    }

    fn stable_trend_handoff_monitor_record()
    -> AgentServiceExecutionHealthGateTrendHandoffMonitorRecord {
        let service_recorder = AgentServiceExecutionHistoryRecorder::new();
        let handoff_builder = AgentServiceExecutionHealthGateTrendHandoff::new();
        let monitor = AgentServiceExecutionHealthGateTrendHandoffMonitor::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());
        let gate_record = service_recorder.record_summary_with_health_gate(
            AgentServiceExecutionHistory::new(),
            clean,
            AgentServiceExecutionHealthPolicy::default(),
            "run/stable-service",
            &queue,
        );
        let handoff = handoff_builder.record_gate_record_and_gate(
            AgentServiceExecutionHealthGateHistory::new(),
            &gate_record,
            AgentServiceExecutionHealthGateHealthPolicy::default(),
            "run/stable-handoff",
            &queue,
        );

        monitor.record_and_gate(
            handoff,
            AgentServiceExecutionHealthGateTrendHandoffHistory::new(),
            AgentServiceExecutionHealthGateTrendHandoffHealthPolicy::default(),
            "run/stable-monitor",
        )
    }

    fn stable_trend_handoff_monitor_handoff_record()
    -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffRecord {
        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/stable-monitor-handoff",
        )
    }

    #[test]
    fn service_commands_promote_candidate_and_emit_telemetry() {
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::new(),
            Some(AdaptiveStateCandidate {
                run_id: "run-1".to_owned(),
                reward_total: 0.91,
                acceptance_rate: 1.0,
                average_reward_total: 0.91,
                evidence_refs: vec!["eval:pass".to_owned()],
            }),
        );

        let commands = AgentServiceCommandPlanner::new().plan(&business_plan);

        assert!(commands.requires_adaptive_state_write());
        assert_eq!(
            commands.command_kinds(),
            vec!["promote_adaptive_state", "emit_telemetry"]
        );

        let summary = commands.summary();
        assert_eq!(summary.command_count, 2);
        assert_eq!(
            summary.command_kinds,
            vec!["promote_adaptive_state", "emit_telemetry"]
        );
        assert!(summary.requires_adaptive_state_write);
        assert!(!summary.repair_mode_requested);
        assert!(!summary.hold_requested);
        assert_eq!(summary.telemetry_commands, 1);
    }

    #[test]
    fn service_commands_repair_and_enqueue_next_tasks() {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "repair",
            AgentRole::Reviewer,
            "repair blocked loop",
            crate::budget::AgentBudget::new(8, 1, 1),
        )]);
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Repair,
            vec!["consecutive_blocked_cycles=3".to_owned()],
            queue,
            None,
        );

        let commands = AgentServiceCommandPlanner::new().plan(&business_plan);

        assert!(commands.repair_mode_requested());
        assert!(!commands.requires_adaptive_state_write());
        assert_eq!(
            commands.command_kinds(),
            vec!["open_repair_mode", "enqueue_tasks", "emit_telemetry"]
        );
        let AgentServiceCommand::EnqueueTasks(queue) = &commands.commands[1] else {
            panic!("expected enqueue command");
        };
        assert_eq!(queue.task_ids(), vec!["repair"]);

        let summary = commands.summary();
        assert_eq!(summary.command_count, 3);
        assert!(!summary.requires_adaptive_state_write);
        assert!(summary.repair_mode_requested);
        assert!(!summary.hold_requested);
        assert_eq!(summary.enqueue_commands, 1);
        assert_eq!(summary.enqueued_tasks, 1);
        assert_eq!(summary.reason_count, 1);
        assert_eq!(summary.memory_promotion_reason_count, 0);
        assert_eq!(summary.tool_build_reason_count, 0);
        assert_eq!(summary.telemetry_commands, 1);
    }

    #[test]
    fn service_commands_surface_memory_promotion_repair_reasons() {
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Repair,
            vec![
                "memory_promotion_blocked_cycles=1>0".to_owned(),
                "memory_promotion_repair_cycles=1>0".to_owned(),
            ],
            AgentTaskQueue::new(),
            None,
        );

        let commands = AgentServiceCommandPlanner::new().plan(&business_plan);

        assert_eq!(
            commands.command_kinds(),
            vec!["open_repair_mode", "emit_telemetry"]
        );
        let summary = commands.summary();
        assert!(summary.repair_mode_requested);
        assert_eq!(summary.reason_count, 2);
        assert_eq!(summary.memory_promotion_reason_count, 2);
        assert_eq!(summary.tool_build_reason_count, 0);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_service_command_plan_summary_memory_promotion_reasons=2"
        }));

        let health = AgentServiceCommandPlanSummaryHistory::from_summaries(vec![summary]).health(
            AgentServiceCommandPlanHealthPolicy {
                maximum_repair_or_hold_rate: 1.0,
                maximum_memory_promotion_reason_plans: 0,
                ..AgentServiceCommandPlanHealthPolicy::default()
            },
        );

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Repair);
        assert_eq!(health.dashboard.memory_promotion_reason_plans, 1);
        assert_eq!(health.dashboard.memory_promotion_reason_count, 2);
        assert_eq!(
            health.reasons,
            vec!["service_command_plan_memory_promotion_reason_plans=1>0"]
        );
    }

    #[test]
    fn service_commands_surface_tool_build_repair_reasons() {
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Repair,
            vec!["tool_build_blocked_cycles=1>0".to_owned()],
            AgentTaskQueue::new(),
            None,
        );

        let commands = AgentServiceCommandPlanner::new().plan(&business_plan);

        assert_eq!(
            commands.command_kinds(),
            vec!["open_repair_mode", "run_rust_validation", "emit_telemetry"]
        );
        let AgentServiceCommand::RunRustValidation {
            commands: validation_commands,
            reasons,
        } = &commands.commands[1]
        else {
            panic!("expected rust validation command");
        };
        assert_eq!(
            validation_commands
                .iter()
                .map(|command| command.as_str())
                .collect::<Vec<_>>(),
            vec!["cargo_fmt", "cargo_check", "cargo_test", "cargo_benchmark"]
        );
        assert_eq!(reasons, &vec!["tool_build_blocked_cycles=1>0".to_owned()]);
        let summary = commands.summary();
        assert!(summary.repair_mode_requested);
        assert_eq!(summary.reason_count, 1);
        assert_eq!(summary.memory_promotion_reason_count, 0);
        assert_eq!(summary.tool_build_reason_count, 1);
        assert_eq!(summary.rust_validation_commands, 4);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_service_command_plan_summary_tool_build_reasons=1" })
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_service_command_plan_summary_rust_validation_commands=4"
        }));

        let health = AgentServiceCommandPlanSummaryHistory::from_summaries(vec![summary]).health(
            AgentServiceCommandPlanHealthPolicy {
                maximum_repair_or_hold_rate: 1.0,
                maximum_tool_build_reason_plans: 0,
                ..AgentServiceCommandPlanHealthPolicy::default()
            },
        );

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Repair);
        assert_eq!(health.dashboard.tool_build_reason_plans, 1);
        assert_eq!(health.dashboard.tool_build_reason_count, 1);
        assert!(health.dashboard.telemetry.iter().any(|line| {
            line == "agent_service_command_plan_dashboard_tool_build_reason_plans=1"
        }));
        assert_eq!(
            health.reasons,
            vec!["service_command_plan_tool_build_reason_plans=1>0"]
        );
    }

    #[test]
    fn service_commands_hold_when_promote_candidate_is_missing() {
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::new(),
            None,
        );

        let commands = AgentServiceCommandPlanner::new().plan(&business_plan);

        assert_eq!(
            commands.command_kinds(),
            vec!["hold_business_loop", "emit_telemetry"]
        );
        let AgentServiceCommand::HoldBusinessLoop { reasons } = &commands.commands[0] else {
            panic!("expected hold command");
        };
        assert_eq!(
            reasons,
            &vec!["adaptive_state_candidate_missing".to_owned()]
        );

        let summary = commands.summary();
        assert_eq!(summary.command_count, 2);
        assert!(!summary.requires_adaptive_state_write);
        assert!(!summary.repair_mode_requested);
        assert!(summary.hold_requested);
        assert_eq!(summary.enqueue_commands, 0);
        assert_eq!(summary.telemetry_commands, 1);
    }

    #[test]
    fn service_command_plan_history_empty_is_watch() {
        let health = AgentServiceCommandPlanSummaryHistory::new()
            .health(AgentServiceCommandPlanHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.reasons, vec!["service_command_plan_history_empty"]);
        assert_eq!(health.dashboard.total_plans, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn service_command_plan_history_is_stable_for_promote_command_plan() {
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::new(),
            Some(AdaptiveStateCandidate {
                run_id: "run-1".to_owned(),
                reward_total: 0.91,
                acceptance_rate: 1.0,
                average_reward_total: 0.91,
                evidence_refs: vec!["eval:pass".to_owned()],
            }),
        );
        let command_plan = AgentServiceCommandPlanner::new().plan(&business_plan);
        let record = AgentServiceCommandPlanSummaryHistoryRecorder::new().record_plan(
            AgentServiceCommandPlanSummaryHistory::new(),
            &command_plan,
            AgentServiceCommandPlanHealthPolicy::default(),
        );

        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.health.is_stable());
        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.total_plans, 1);
        assert_eq!(record.dashboard.command_count, 2);
        assert_eq!(record.dashboard.adaptive_write_plans, 1);
        assert_eq!(record.dashboard.repair_or_hold_plans, 0);
        assert_eq!(
            record.dashboard.latest_command_kinds,
            vec!["promote_adaptive_state", "emit_telemetry"]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_service_command_plan_history_record_status=stable" })
        );
    }

    #[test]
    fn service_command_plan_history_watches_enqueue_pressure_policy() {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "repair",
            AgentRole::Reviewer,
            "repair blocked loop",
            crate::budget::AgentBudget::new(8, 1, 1),
        )]);
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Repair,
            vec!["consecutive_blocked_cycles=3".to_owned()],
            queue,
            None,
        );
        let command_plan = AgentServiceCommandPlanner::new().plan(&business_plan);
        let policy = AgentServiceCommandPlanHealthPolicy {
            maximum_repair_or_hold_rate: 1.0,
            maximum_enqueued_tasks: 0,
            ..AgentServiceCommandPlanHealthPolicy::default()
        };

        let health =
            AgentServiceCommandPlanSummaryHistory::from_summaries(vec![command_plan.summary()])
                .health(policy);

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.dashboard.enqueued_task_count, 1);
        assert_eq!(
            health.reasons,
            vec!["service_command_plan_enqueued_tasks=1>0"]
        );
    }

    #[test]
    fn service_command_plan_history_repairs_repair_or_hold_pressure() {
        let repair_queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "repair",
            AgentRole::Reviewer,
            "repair blocked loop",
            crate::budget::AgentBudget::new(8, 1, 1),
        )]);
        let repair_plan = AgentServiceCommandPlanner::new().plan(&plan(
            AgentCycleLedgerAdmissionStatus::Repair,
            vec!["consecutive_blocked_cycles=3".to_owned()],
            repair_queue,
            None,
        ));
        let hold_plan = AgentServiceCommandPlanner::new().plan(&plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::new(),
            None,
        ));
        let recorder = AgentServiceCommandPlanSummaryHistoryRecorder::new();
        let first = recorder.record_plan(
            AgentServiceCommandPlanSummaryHistory::new(),
            &repair_plan,
            AgentServiceCommandPlanHealthPolicy::default(),
        );
        let second = recorder.record_plan(
            first.history,
            &hold_plan,
            AgentServiceCommandPlanHealthPolicy::default(),
        );

        assert_eq!(
            second.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(second.dashboard.total_plans, 2);
        assert_eq!(second.dashboard.repair_mode_plans, 1);
        assert_eq!(second.dashboard.hold_plans, 1);
        assert_eq!(second.dashboard.repair_or_hold_rate, 1.0);
        assert_eq!(
            second
                .history
                .summaries()
                .iter()
                .map(|summary| summary.command_kinds.clone())
                .collect::<Vec<_>>(),
            vec![
                vec![
                    "open_repair_mode".to_owned(),
                    "enqueue_tasks".to_owned(),
                    "emit_telemetry".to_owned(),
                ],
                vec!["hold_business_loop".to_owned(), "emit_telemetry".to_owned()],
            ]
        );
        assert!(
            second
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "service_command_plan_repair_or_hold_rate=1.000>0" })
        );
    }

    #[test]
    fn service_command_audit_is_clean_when_every_command_applies() {
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::new(),
            Some(AdaptiveStateCandidate {
                run_id: "run-1".to_owned(),
                reward_total: 0.91,
                acceptance_rate: 1.0,
                average_reward_total: 0.91,
                evidence_refs: vec!["eval:pass".to_owned()],
            }),
        );
        let planner = AgentServiceCommandPlanner::new();
        let plan = planner.plan(&business_plan);
        let receipts = plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();

        let audit = planner.audit(&plan, receipts);

        assert!(audit.is_clean());
        assert_eq!(
            audit.expected,
            vec!["promote_adaptive_state", "emit_telemetry"]
        );
        assert!(audit.blocked_reasons().is_empty());

        let summary = audit.summary();
        assert_eq!(summary.expected_commands, 2);
        assert_eq!(summary.receipts, 2);
        assert_eq!(summary.missing_commands, 0);
        assert_eq!(summary.failed_commands, 0);
        assert_eq!(summary.skipped_commands, 0);
        assert!(summary.clean);
        assert!(summary.blocked_reasons.is_empty());
    }

    #[test]
    fn service_command_audit_reports_missing_failed_and_skipped_commands() {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "repair",
            AgentRole::Reviewer,
            "repair blocked loop",
            crate::budget::AgentBudget::new(8, 1, 1),
        )]);
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Repair,
            vec!["consecutive_blocked_cycles=3".to_owned()],
            queue,
            None,
        );
        let planner = AgentServiceCommandPlanner::new();
        let plan = planner.plan(&business_plan);
        let receipts = vec![
            AgentServiceCommandReceipt::failed(&plan.commands[0], "repair writer offline"),
            AgentServiceCommandReceipt::skipped(&plan.commands[1], "queue paused"),
        ];

        let audit = planner.audit(&plan, receipts);

        assert!(!audit.is_clean());
        assert_eq!(audit.missing, vec!["emit_telemetry"]);
        assert_eq!(audit.failed.len(), 1);
        assert_eq!(audit.skipped.len(), 1);
        assert_eq!(
            audit.blocked_reasons(),
            vec![
                "service_command_missing=emit_telemetry",
                "service_command_failed=open_repair_mode:repair writer offline",
                "service_command_skipped=enqueue_tasks:queue paused",
            ]
        );

        let summary = audit.summary();
        assert_eq!(summary.expected_commands, 3);
        assert_eq!(summary.receipts, 2);
        assert_eq!(summary.missing_commands, 1);
        assert_eq!(summary.failed_commands, 1);
        assert_eq!(summary.skipped_commands, 1);
        assert!(!summary.clean);
        assert_eq!(
            summary.blocked_reasons,
            vec![
                "service_command_missing=emit_telemetry",
                "service_command_failed=open_repair_mode:repair writer offline",
                "service_command_skipped=enqueue_tasks:queue paused",
            ]
        );
    }

    #[test]
    fn service_command_audit_history_empty_is_watch() {
        let health = AgentServiceCommandAuditSummaryHistory::new()
            .health(AgentServiceCommandAuditHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.reasons, vec!["service_command_audit_history_empty"]);
        assert_eq!(health.dashboard.total_audits, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn service_command_audit_history_marks_clean_receipts_stable() {
        let summary = AgentServiceCommandAuditSummary {
            expected_commands: 2,
            receipts: 2,
            missing_commands: 0,
            failed_commands: 0,
            skipped_commands: 0,
            clean: true,
            blocked_reasons: Vec::new(),
            telemetry: vec!["fixture_clean=true".to_owned()],
        };

        let record = AgentServiceCommandAuditSummaryHistoryRecorder::new().record_summary(
            AgentServiceCommandAuditSummaryHistory::new(),
            summary,
            AgentServiceCommandAuditHealthPolicy::default(),
        );

        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.health.is_stable());
        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.total_audits, 1);
        assert_eq!(record.dashboard.clean_audits, 1);
        assert_eq!(record.dashboard.drift_event_count, 0);
        assert_eq!(record.dashboard.blocked_reason_count, 0);
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_service_command_audit_history_record_status=stable" })
        );
    }

    #[test]
    fn service_command_audit_history_watches_low_clean_rate_when_drift_is_allowed() {
        let clean = AgentServiceCommandAuditSummary {
            expected_commands: 2,
            receipts: 2,
            missing_commands: 0,
            failed_commands: 0,
            skipped_commands: 0,
            clean: true,
            blocked_reasons: Vec::new(),
            telemetry: Vec::new(),
        };
        let dirty = AgentServiceCommandAuditSummary {
            expected_commands: 2,
            receipts: 1,
            missing_commands: 1,
            failed_commands: 0,
            skipped_commands: 0,
            clean: false,
            blocked_reasons: vec!["service_command_missing=emit_telemetry".to_owned()],
            telemetry: Vec::new(),
        };
        let policy = AgentServiceCommandAuditHealthPolicy {
            maximum_drift_rate: 1.0,
            maximum_blocked_reasons: usize::MAX,
            ..AgentServiceCommandAuditHealthPolicy::default()
        };

        let health = AgentServiceCommandAuditSummaryHistory::from_summaries(vec![clean, dirty])
            .health(policy);

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.dashboard.clean_rate, 0.5);
        assert_eq!(
            health.reasons,
            vec!["service_command_audit_clean_rate=0.500<0.67"]
        );
    }

    #[test]
    fn service_command_audit_history_repairs_drift_and_preserves_order() {
        let planner = AgentServiceCommandPlanner::new();
        let promote_plan = planner.plan(&plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::new(),
            Some(AdaptiveStateCandidate {
                run_id: "run-1".to_owned(),
                reward_total: 0.91,
                acceptance_rate: 1.0,
                average_reward_total: 0.91,
                evidence_refs: vec!["eval:pass".to_owned()],
            }),
        ));
        let clean_receipts = promote_plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let clean_audit = planner.audit(&promote_plan, clean_receipts);
        let repair_plan = planner.plan(&plan(
            AgentCycleLedgerAdmissionStatus::Repair,
            vec!["consecutive_blocked_cycles=3".to_owned()],
            AgentTaskQueue::from_tasks(vec![AgentTask::new(
                "repair",
                AgentRole::Reviewer,
                "repair blocked loop",
                crate::budget::AgentBudget::new(8, 1, 1),
            )]),
            None,
        ));
        let dirty_audit = planner.audit(
            &repair_plan,
            vec![
                AgentServiceCommandReceipt::failed(
                    &repair_plan.commands[0],
                    "repair writer offline",
                ),
                AgentServiceCommandReceipt::skipped(&repair_plan.commands[1], "queue paused"),
            ],
        );
        let recorder = AgentServiceCommandAuditSummaryHistoryRecorder::new();
        let first = recorder.record_audit(
            AgentServiceCommandAuditSummaryHistory::new(),
            &clean_audit,
            AgentServiceCommandAuditHealthPolicy::default(),
        );
        let second = recorder.record_audit(
            first.history,
            &dirty_audit,
            AgentServiceCommandAuditHealthPolicy::default(),
        );

        assert_eq!(
            second.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(second.dashboard.total_audits, 2);
        assert_eq!(second.dashboard.clean_audits, 1);
        assert_eq!(second.dashboard.dirty_audits, 1);
        assert_eq!(second.dashboard.drift_event_count, 3);
        assert_eq!(second.dashboard.blocked_reason_count, 3);
        assert_eq!(
            second
                .history
                .summaries()
                .iter()
                .map(|summary| summary.clean)
                .collect::<Vec<_>>(),
            vec![true, false]
        );
        assert_eq!(
            second.dashboard.latest_blocked_reasons,
            vec![
                "service_command_missing=emit_telemetry",
                "service_command_failed=open_repair_mode:repair writer offline",
                "service_command_skipped=enqueue_tasks:queue paused",
            ]
        );
        assert!(
            second
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "service_command_audit_drift_rate=0.600>0" })
        );
    }

    #[test]
    fn service_feedback_history_empty_is_watch() {
        let health = AgentServiceFeedbackSummaryHistory::new()
            .health(AgentServiceFeedbackHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.reasons, vec!["service_feedback_history_empty"]);
        assert_eq!(health.dashboard.total_feedbacks, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn service_feedback_history_marks_clean_feedback_stable() {
        let planner = AgentServiceCommandPlanner::new();
        let command_plan = planner.plan(&plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::new(),
            Some(AdaptiveStateCandidate {
                run_id: "run-1".to_owned(),
                reward_total: 0.91,
                acceptance_rate: 1.0,
                average_reward_total: 0.91,
                evidence_refs: vec!["eval:pass".to_owned()],
            }),
        ));
        let receipts = command_plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let audit = planner.audit(&command_plan, receipts);
        let feedback = AgentServiceFeedback::from_audit("run/clean-feedback", audit);

        let record = AgentServiceFeedbackSummaryHistoryRecorder::new().record_feedback(
            AgentServiceFeedbackSummaryHistory::new(),
            &feedback,
            AgentServiceFeedbackHealthPolicy::default(),
        );

        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.health.is_stable());
        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.total_feedbacks, 1);
        assert_eq!(record.dashboard.clean_feedbacks, 1);
        assert_eq!(record.dashboard.repair_task_count, 0);
        assert_eq!(record.dashboard.blocked_reason_count, 0);
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_service_feedback_history_record_status=stable" })
        );
    }

    #[test]
    fn service_feedback_history_watches_low_clean_rate_when_repair_pressure_is_allowed() {
        let clean = AgentServiceFeedbackSummary {
            audit_clean: true,
            repair_tasks: 0,
            next_queue_tasks: 0,
            blocked_reasons: Vec::new(),
            clean: true,
            telemetry: Vec::new(),
        };
        let dirty = AgentServiceFeedbackSummary {
            audit_clean: false,
            repair_tasks: 1,
            next_queue_tasks: 1,
            blocked_reasons: vec!["service_command_missing=emit_telemetry".to_owned()],
            clean: false,
            telemetry: Vec::new(),
        };
        let policy = AgentServiceFeedbackHealthPolicy {
            maximum_repair_tasks: usize::MAX,
            maximum_blocked_reasons: usize::MAX,
            ..AgentServiceFeedbackHealthPolicy::default()
        };

        let health =
            AgentServiceFeedbackSummaryHistory::from_summaries(vec![clean, dirty]).health(policy);

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.dashboard.clean_rate, 0.5);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["service_feedback_clean_rate=0.500<0.67"]
        );
    }

    #[test]
    fn service_feedback_history_repairs_feedback_pressure_and_preserves_order() {
        let planner = AgentServiceCommandPlanner::new();
        let clean_plan = planner.plan(&plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::new(),
            Some(AdaptiveStateCandidate {
                run_id: "run-1".to_owned(),
                reward_total: 0.91,
                acceptance_rate: 1.0,
                average_reward_total: 0.91,
                evidence_refs: vec!["eval:pass".to_owned()],
            }),
        ));
        let clean_receipts = clean_plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let clean_feedback = AgentServiceFeedback::from_audit(
            "run/clean-feedback",
            planner.audit(&clean_plan, clean_receipts),
        );
        let repair_plan = planner.plan(&plan(
            AgentCycleLedgerAdmissionStatus::Repair,
            vec!["consecutive_blocked_cycles=3".to_owned()],
            AgentTaskQueue::from_tasks(vec![AgentTask::new(
                "repair",
                AgentRole::Reviewer,
                "repair blocked loop",
                crate::budget::AgentBudget::new(8, 1, 1),
            )]),
            None,
        ));
        let dirty_feedback = AgentServiceFeedback::from_audit(
            "run/dirty-feedback",
            planner.audit(
                &repair_plan,
                vec![
                    AgentServiceCommandReceipt::failed(
                        &repair_plan.commands[0],
                        "repair writer offline",
                    ),
                    AgentServiceCommandReceipt::skipped(&repair_plan.commands[1], "queue paused"),
                ],
            ),
        );
        let recorder = AgentServiceFeedbackSummaryHistoryRecorder::new();
        let first = recorder.record_feedback(
            AgentServiceFeedbackSummaryHistory::new(),
            &clean_feedback,
            AgentServiceFeedbackHealthPolicy::default(),
        );
        let second = recorder.record_feedback(
            first.history,
            &dirty_feedback,
            AgentServiceFeedbackHealthPolicy::default(),
        );

        assert_eq!(
            second.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(second.dashboard.total_feedbacks, 2);
        assert_eq!(second.dashboard.clean_feedbacks, 1);
        assert_eq!(second.dashboard.dirty_feedbacks, 1);
        assert_eq!(second.dashboard.audit_dirty_feedbacks, 1);
        assert_eq!(second.dashboard.repair_task_count, 3);
        assert_eq!(second.dashboard.total_next_queue_tasks, 3);
        assert_eq!(second.dashboard.blocked_reason_count, 3);
        assert_eq!(
            second
                .history
                .summaries()
                .iter()
                .map(|summary| summary.clean)
                .collect::<Vec<_>>(),
            vec![true, false]
        );
        assert_eq!(
            second.dashboard.latest_blocked_reasons,
            vec![
                "service_command_missing=emit_telemetry",
                "service_command_failed=open_repair_mode:repair writer offline",
                "service_command_skipped=enqueue_tasks:queue paused",
            ]
        );
        assert!(
            second
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "service_feedback_repair_tasks=3>0" })
        );
    }

    #[test]
    fn service_turnover_history_empty_is_watch() {
        let health = AgentServiceTurnoverSummaryHistory::new()
            .health(AgentServiceTurnoverHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.reasons, vec!["service_turnover_history_empty"]);
        assert_eq!(health.dashboard.total_turnovers, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn service_turnover_history_marks_clean_turnover_stable() {
        let planner = AgentServiceCommandPlanner::new();
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::from_tasks(vec![AgentTask::new(
                "business-next",
                AgentRole::Planner,
                "continue business queue",
                crate::budget::AgentBudget::new(5, 1, 1),
            )]),
            Some(AdaptiveStateCandidate {
                run_id: "run-1".to_owned(),
                reward_total: 0.91,
                acceptance_rate: 1.0,
                average_reward_total: 0.91,
                evidence_refs: vec!["eval:pass".to_owned()],
            }),
        );
        let command_plan = planner.plan(&business_plan);
        let receipts = command_plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let feedback = AgentServiceFeedback::from_audit(
            "run/clean-turnover",
            planner.audit(&command_plan, receipts),
        );
        let turnover = AgentServiceTurnover::from_feedback(&business_plan, feedback);

        let record = AgentServiceTurnoverSummaryHistoryRecorder::new().record_turnover(
            AgentServiceTurnoverSummaryHistory::new(),
            &turnover,
            AgentServiceTurnoverHealthPolicy::default(),
        );

        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.health.is_stable());
        assert_eq!(record.dashboard.total_turnovers, 1);
        assert_eq!(record.dashboard.clean_turnovers, 1);
        assert_eq!(record.dashboard.repair_task_count, 0);
        assert_eq!(record.dashboard.total_next_queue_tasks, 1);
        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_service_turnover_history_record_status=stable" })
        );
    }

    #[test]
    fn service_turnover_history_watches_next_queue_pressure_policy() {
        let summary = AgentServiceTurnoverSummary {
            feedback_clean: true,
            repair_tasks: 0,
            next_queue_tasks: 2,
            blocked_reasons: Vec::new(),
            clean: true,
            telemetry: Vec::new(),
        };
        let policy = AgentServiceTurnoverHealthPolicy {
            maximum_next_queue_tasks: 1,
            ..AgentServiceTurnoverHealthPolicy::default()
        };

        let health =
            AgentServiceTurnoverSummaryHistory::from_summaries(vec![summary]).health(policy);

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.dashboard.total_next_queue_tasks, 2);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["service_turnover_next_queue_tasks=2>1"]
        );
    }

    #[test]
    fn service_turnover_history_repairs_turnover_pressure_and_preserves_order() {
        let planner = AgentServiceCommandPlanner::new();
        let clean_business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::new(),
            Some(AdaptiveStateCandidate {
                run_id: "run-1".to_owned(),
                reward_total: 0.91,
                acceptance_rate: 1.0,
                average_reward_total: 0.91,
                evidence_refs: vec!["eval:pass".to_owned()],
            }),
        );
        let clean_plan = planner.plan(&clean_business_plan);
        let clean_receipts = clean_plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let clean_feedback = AgentServiceFeedback::from_audit(
            "run/clean-turnover",
            planner.audit(&clean_plan, clean_receipts),
        );
        let clean_turnover =
            AgentServiceTurnover::from_feedback(&clean_business_plan, clean_feedback);
        let repair_business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Repair,
            vec!["consecutive_blocked_cycles=3".to_owned()],
            AgentTaskQueue::from_tasks(vec![AgentTask::new(
                "business-repair",
                AgentRole::Reviewer,
                "continue repair queue",
                crate::budget::AgentBudget::new(8, 1, 1),
            )]),
            None,
        );
        let repair_plan = planner.plan(&repair_business_plan);
        let dirty_feedback = AgentServiceFeedback::from_audit(
            "run/dirty-turnover",
            planner.audit(
                &repair_plan,
                vec![
                    AgentServiceCommandReceipt::failed(
                        &repair_plan.commands[0],
                        "repair writer offline",
                    ),
                    AgentServiceCommandReceipt::skipped(&repair_plan.commands[1], "queue paused"),
                ],
            ),
        );
        let dirty_turnover =
            AgentServiceTurnover::from_feedback(&repair_business_plan, dirty_feedback);
        let recorder = AgentServiceTurnoverSummaryHistoryRecorder::new();
        let first = recorder.record_turnover(
            AgentServiceTurnoverSummaryHistory::new(),
            &clean_turnover,
            AgentServiceTurnoverHealthPolicy::default(),
        );
        let second = recorder.record_turnover(
            first.history,
            &dirty_turnover,
            AgentServiceTurnoverHealthPolicy::default(),
        );

        assert_eq!(
            second.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(second.dashboard.total_turnovers, 2);
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(second.dashboard.clean_turnovers, 1);
        assert_eq!(second.dashboard.dirty_turnovers, 1);
        assert_eq!(second.dashboard.feedback_dirty_turnovers, 1);
        assert_eq!(second.dashboard.repair_task_count, 3);
        assert_eq!(second.dashboard.total_next_queue_tasks, 4);
        assert_eq!(second.dashboard.blocked_reason_count, 3);
        assert_eq!(
            second
                .history
                .summaries()
                .iter()
                .map(|summary| summary.clean)
                .collect::<Vec<_>>(),
            vec![true, false]
        );
        assert_eq!(
            second.dashboard.latest_blocked_reasons,
            vec![
                "service_command_missing=emit_telemetry",
                "service_command_failed=open_repair_mode:repair writer offline",
                "service_command_skipped=enqueue_tasks:queue paused",
            ]
        );
        assert!(
            second
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "service_turnover_repair_tasks=3>0" })
        );
    }

    #[test]
    fn service_execution_history_handles_empty_dashboard() {
        let history = AgentServiceExecutionHistory::new();
        let dashboard = history.dashboard();

        assert!(history.is_empty());
        assert_eq!(dashboard.total_runs, 0);
        assert_eq!(dashboard.clean_rate, 0.0);
        assert_eq!(dashboard.service_drift_rate, 0.0);
        assert_eq!(dashboard.latest_clean, None);
        assert!(!dashboard.is_clean());
        assert!(!dashboard.has_service_drift());
        assert!(
            dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_service_execution_dashboard_runs=0" })
        );
    }

    #[test]
    fn service_execution_history_summarizes_receipt_close_pressure() {
        let history = AgentServiceExecutionHistory::from_summaries(vec![
            service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new()),
            service_report_summary(
                false,
                (4, 1, 1, 1),
                3,
                5,
                vec!["service_command_failed=emit_telemetry:writer offline"],
            ),
        ]);

        let dashboard = history.dashboard();

        assert_eq!(history.len(), 2);
        assert_eq!(dashboard.total_runs, 2);
        assert_eq!(dashboard.clean_runs, 1);
        assert_eq!(dashboard.dirty_runs, 1);
        assert_eq!(dashboard.clean_rate, 0.5);
        assert_eq!(dashboard.command_count, 6);
        assert_eq!(dashboard.receipt_count, 5);
        assert_eq!(dashboard.missing_command_count, 1);
        assert_eq!(dashboard.failed_command_count, 1);
        assert_eq!(dashboard.skipped_command_count, 1);
        assert_eq!(dashboard.repair_task_count, 3);
        assert_eq!(dashboard.total_next_queue_tasks, 5);
        assert_eq!(dashboard.service_drift_rate, 0.5);
        assert_eq!(dashboard.latest_clean, Some(false));
        assert_eq!(
            dashboard.latest_blocked_reasons,
            vec!["service_command_failed=emit_telemetry:writer offline"]
        );
        assert!(dashboard.has_service_drift());
        assert!(!dashboard.is_clean());
    }

    #[test]
    fn service_execution_health_watches_empty_history() {
        let health = AgentServiceExecutionHistory::new()
            .health(AgentServiceExecutionHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.reasons, vec!["service_execution_history_empty"]);
        assert_eq!(health.dashboard.total_runs, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn service_execution_health_marks_clean_receipts_stable() {
        let history = AgentServiceExecutionHistory::from_summaries(vec![
            service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new()),
            service_report_summary(true, (3, 0, 0, 0), 0, 1, Vec::new()),
        ]);

        let health = history.health(AgentServiceExecutionHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert!(health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(health.dashboard.clean_rate, 1.0);
    }

    #[test]
    fn service_execution_health_repairs_receipt_drift() {
        let history = AgentServiceExecutionHistory::from_summaries(vec![
            service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new()),
            service_report_summary(
                false,
                (4, 1, 1, 1),
                3,
                5,
                vec!["service_command_failed=emit_telemetry:writer offline"],
            ),
        ]);

        let health = history.health(AgentServiceExecutionHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Repair);
        assert_eq!(
            health.reasons,
            vec![
                "service_execution_drift_rate=0.500>0",
                "service_execution_repair_tasks=3>0",
                "service_execution_clean_rate=0.500<0.67",
            ]
        );
        assert!(!health.is_stable());
        assert!(!health.allows_service_advance());
        assert!(health.requires_repair_first());
    }

    #[test]
    fn service_execution_history_recorder_appends_summary_and_dashboard() {
        let recorder = AgentServiceExecutionHistoryRecorder::new();
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());
        let dirty = service_report_summary(
            false,
            (3, 1, 0, 0),
            1,
            2,
            vec!["service_command_missing=enqueue_tasks"],
        );

        let first = recorder.record_summary(AgentServiceExecutionHistory::new(), clean);
        let second = recorder.record_summary(first.history, dirty.clone());

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.history.latest(), Some(&dirty));
        assert_eq!(second.appended_summary, dirty);
        assert_eq!(second.dashboard.total_runs, 2);
        assert_eq!(second.dashboard.dirty_runs, 1);
        assert_eq!(
            second.dashboard.latest_blocked_reasons,
            vec!["service_command_missing=enqueue_tasks"]
        );
    }

    #[test]
    fn service_execution_history_recorder_appends_summary_and_health() {
        let recorder = AgentServiceExecutionHistoryRecorder::new();
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());
        let dirty = service_report_summary(
            false,
            (3, 1, 0, 0),
            1,
            2,
            vec!["service_command_missing=enqueue_tasks"],
        );
        let first = recorder.record_summary_with_health(
            AgentServiceExecutionHistory::new(),
            clean,
            AgentServiceExecutionHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            dirty.clone(),
            AgentServiceExecutionHealthPolicy::default(),
        );

        assert_eq!(
            first.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(second.history.len(), 2);
        assert_eq!(second.history.latest(), Some(&dirty));
        assert_eq!(second.appended_summary, dirty);
        assert_eq!(
            second.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(second.dashboard.total_runs, 2);
        assert!(
            second
                .telemetry
                .iter()
                .any(|line| { line == "agent_service_execution_health_record_status=repair" })
        );
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_record_reason=service_execution_drift_rate=0.200>0"
        }));
    }

    #[test]
    fn service_execution_history_recorder_gates_clean_health_record() {
        let recorder = AgentServiceExecutionHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());

        let record = recorder.record_summary_with_health_gate(
            AgentServiceExecutionHistory::new(),
            clean.clone(),
            AgentServiceExecutionHealthPolicy::default(),
            "run/10",
            &queue,
        );

        assert!(record.is_admitted());
        assert_eq!(record.health_record.appended_summary, clean);
        assert_eq!(
            record.health_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            record.gate_summary.health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.gate_summary.repair_task_ids.is_empty());
        assert_eq!(
            record.gate_summary.next_queue_task_ids,
            vec!["business-task"]
        );
        assert_eq!(record.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(queue.task_ids(), vec!["business-task"]);
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_service_execution_health_gate_record_admitted=true" })
        );
    }

    #[test]
    fn service_execution_history_recorder_gates_dirty_health_record_to_repair_first() {
        let recorder = AgentServiceExecutionHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());
        let dirty = service_report_summary(
            false,
            (4, 1, 1, 1),
            2,
            4,
            vec!["service_command_failed=emit_telemetry:writer offline"],
        );
        let first = recorder.record_summary_with_health(
            AgentServiceExecutionHistory::new(),
            clean,
            AgentServiceExecutionHealthPolicy::default(),
        );

        let record = recorder.record_summary_with_health_gate(
            first.history,
            dirty.clone(),
            AgentServiceExecutionHealthPolicy::default(),
            "run/11",
            &queue,
        );

        assert!(!record.is_admitted());
        assert_eq!(record.health_record.appended_summary, dirty);
        assert_eq!(
            record.health_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            record.gate_decision.health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(record.gate_summary.repair_tasks, 3);
        assert_eq!(record.gate_summary.next_queue_tasks, 4);
        assert_eq!(
            record.gate_summary.repair_task_ids,
            vec![
                "service-execution-health-repair-run-11-0-service_execution_drift_rate-0-500-0",
                "service-execution-health-repair-run-11-1-service_execution_repair_tasks-2-0",
                "service-execution-health-repair-run-11-2-service_execution_clean_rate-0-500-0-67",
            ]
        );
        assert_eq!(
            record.gate_summary.next_queue_task_ids,
            record.next_queue().task_ids()
        );
        assert_eq!(queue.task_ids(), vec!["business-task"]);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_record_requires_repair_first=true"
        }));
    }

    #[test]
    fn service_execution_health_gate_history_handles_empty_dashboard() {
        let history = AgentServiceExecutionHealthGateHistory::new();
        let dashboard = history.dashboard();

        assert!(history.is_empty());
        assert_eq!(dashboard.total_records, 0);
        assert_eq!(dashboard.admitted_records, 0);
        assert_eq!(dashboard.repair_first_records, 0);
        assert_eq!(dashboard.admission_rate, 0.0);
        assert_eq!(dashboard.repair_first_rate, 0.0);
        assert_eq!(dashboard.latest_health_status, None);
        assert!(dashboard.latest_blocked_reasons.is_empty());
        assert!(dashboard.is_empty());
        assert!(!dashboard.is_clean());
        assert!(dashboard.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_dashboard_latest_status=none"
        }));
    }

    #[test]
    fn service_execution_health_gate_history_summarizes_admission_pressure() {
        let recorder = AgentServiceExecutionHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());
        let dirty = service_report_summary(
            false,
            (4, 1, 1, 1),
            2,
            4,
            vec!["service_command_failed=emit_telemetry:writer offline"],
        );
        let first = recorder.record_summary_with_health_gate(
            AgentServiceExecutionHistory::new(),
            clean,
            AgentServiceExecutionHealthPolicy::default(),
            "run/12",
            &queue,
        );
        let second = recorder.record_summary_with_health_gate(
            first.health_record.history.clone(),
            dirty,
            AgentServiceExecutionHealthPolicy::default(),
            "run/13",
            &queue,
        );

        let history = AgentServiceExecutionHealthGateHistory::from_summaries(vec![
            first.gate_summary,
            second.gate_summary.clone(),
        ]);
        let dashboard = history.dashboard();

        assert_eq!(history.len(), 2);
        assert_eq!(history.latest(), Some(&second.gate_summary));
        assert_eq!(history.summaries().len(), 2);
        assert_eq!(dashboard.total_records, 2);
        assert_eq!(dashboard.admitted_records, 1);
        assert_eq!(dashboard.repair_first_records, 1);
        assert_eq!(dashboard.repair_task_count, 3);
        assert_eq!(dashboard.total_next_queue_tasks, 5);
        assert_eq!(dashboard.admission_rate, 0.5);
        assert_eq!(dashboard.repair_first_rate, 0.5);
        assert_eq!(
            dashboard.latest_health_status,
            Some(AgentClosedLoopExecutionHealthStatus::Repair)
        );
        assert_eq!(
            dashboard.latest_blocked_reasons,
            second.gate_summary.blocked_reasons
        );
        assert!(!dashboard.is_clean());
        assert!(dashboard.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_dashboard_repair_first=1"
        }));
    }

    #[test]
    fn service_execution_health_gate_health_watches_empty_history() {
        let health = AgentServiceExecutionHealthGateHistory::new()
            .health(AgentServiceExecutionHealthGateHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["service_execution_health_gate_history_empty"]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
    }

    #[test]
    fn service_execution_health_gate_health_marks_clean_admission_stable() {
        let recorder = AgentServiceExecutionHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());
        let gate_record = recorder.record_summary_with_health_gate(
            AgentServiceExecutionHistory::new(),
            clean,
            AgentServiceExecutionHealthPolicy::default(),
            "run/14",
            &queue,
        );
        let history =
            AgentServiceExecutionHealthGateHistory::from_summaries(vec![gate_record.gate_summary]);

        let health = history.health(AgentServiceExecutionHealthGateHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert_eq!(health.dashboard.admission_rate, 1.0);
        assert_eq!(health.dashboard.repair_first_rate, 0.0);
        assert!(health.is_stable());
    }

    #[test]
    fn service_execution_health_gate_history_recorder_repairs_repair_first_pressure() {
        let service_recorder = AgentServiceExecutionHistoryRecorder::new();
        let gate_recorder = AgentServiceExecutionHealthGateHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());
        let dirty = service_report_summary(
            false,
            (4, 1, 1, 1),
            2,
            4,
            vec!["service_command_failed=emit_telemetry:writer offline"],
        );
        let clean_record = service_recorder.record_summary_with_health_gate(
            AgentServiceExecutionHistory::new(),
            clean,
            AgentServiceExecutionHealthPolicy::default(),
            "run/15",
            &queue,
        );
        let clean_gate = gate_recorder.record_gate_record_with_health(
            AgentServiceExecutionHealthGateHistory::new(),
            &clean_record,
            AgentServiceExecutionHealthGateHealthPolicy::default(),
        );
        let dirty_record = service_recorder.record_summary_with_health_gate(
            clean_record.health_record.history,
            dirty.clone(),
            AgentServiceExecutionHealthPolicy::default(),
            "run/16",
            &queue,
        );

        let dirty_gate = gate_recorder.record_gate_record_with_health(
            clean_gate.history,
            &dirty_record,
            AgentServiceExecutionHealthGateHealthPolicy::default(),
        );

        assert_eq!(dirty_gate.history.len(), 2);
        assert_eq!(dirty_gate.appended_summary, dirty_record.gate_summary);
        assert_eq!(
            dirty_gate.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(dirty_gate.records(), 2);
        assert!(!dirty_gate.allows_service_advance());
        assert!(dirty_gate.requires_repair_first());
        assert_eq!(
            dirty_gate.health.reasons,
            vec![
                "service_execution_health_gate_repair_first_rate=0.500>0",
                "service_execution_health_gate_repair_tasks=3>0",
                "service_execution_health_gate_admission_rate=0.500<0.67",
            ]
        );
        assert_eq!(dirty_gate.dashboard.repair_first_records, 1);
        assert_eq!(dirty_gate.dashboard.repair_task_count, 3);
        assert!(dirty_gate.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_history_record_status=repair"
        }));
        assert_eq!(
            dirty_gate.appended_summary.health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
    }

    #[test]
    fn service_execution_health_gate_trend_gate_admits_stable_and_watch() {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let stable = AgentServiceExecutionHealthGateHistory::from_summaries(vec![
            AgentServiceExecutionHealthGateDecision {
                health_status: AgentClosedLoopExecutionHealthStatus::Stable,
                admitted: true,
                requires_repair_first: false,
                repair_tasks: Vec::new(),
                next_queue: queue.clone(),
                blocked_reasons: Vec::new(),
                telemetry: Vec::new(),
            }
            .summary(),
        ])
        .health(AgentServiceExecutionHealthGateHealthPolicy::default());
        let watch = AgentServiceExecutionHealthGateHistory::new()
            .health(AgentServiceExecutionHealthGateHealthPolicy::default());
        let gate = AgentServiceExecutionHealthGateTrendGate::new();

        let stable_decision = gate.evaluate("run/17", &stable, &queue);
        let watch_decision = gate.evaluate("run/17", &watch, &queue);

        assert!(stable_decision.is_admitted());
        assert!(watch_decision.is_admitted());
        assert_eq!(
            stable_decision.health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            watch_decision.health_status,
            AgentClosedLoopExecutionHealthStatus::Watch
        );
        assert!(stable_decision.repair_tasks.is_empty());
        assert!(watch_decision.repair_tasks.is_empty());
        assert_eq!(stable_decision.next_queue.task_ids(), vec!["business-task"]);
        assert_eq!(watch_decision.next_queue.task_ids(), vec!["business-task"]);
        assert_eq!(queue.task_ids(), vec!["business-task"]);
    }

    #[test]
    fn service_execution_health_gate_trend_gate_blocks_repair_trend() {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let health = AgentServiceExecutionHealthGateHealth {
            status: AgentClosedLoopExecutionHealthStatus::Repair,
            reasons: vec![
                "service_execution_health_gate_repair_first_rate=0.500>0".to_owned(),
                "service_execution_health_gate_repair_tasks=3>0".to_owned(),
                "service_execution_health_gate_admission_rate=0.500<0.67".to_owned(),
            ],
            dashboard: AgentServiceExecutionHealthGateHistory::new().dashboard(),
        };

        let decision =
            AgentServiceExecutionHealthGateTrendGate::new().evaluate("run/18", &health, &queue);

        assert!(!decision.is_admitted());
        assert!(decision.requires_repair_first);
        assert_eq!(
            decision.health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(decision.blocked_reasons, health.reasons);
        assert_eq!(decision.repair_tasks.len(), 3);
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "service-execution-health-gate" && task.priority == 9)
        );
        assert_eq!(
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "service-execution-health-gate-repair-run-18-0-service_execution_health_gate_repair_first_rate-0-500-0",
                "service-execution-health-gate-repair-run-18-1-service_execution_health_gate_repair_tasks-3-0",
                "service-execution-health-gate-repair-run-18-2-service_execution_health_gate_admission_rate-0-500-0-67",
            ]
        );
        assert_eq!(
            decision.next_queue.task_ids(),
            vec![
                "business-task",
                "service-execution-health-gate-repair-run-18-0-service_execution_health_gate_repair_first_rate-0-500-0",
                "service-execution-health-gate-repair-run-18-1-service_execution_health_gate_repair_tasks-3-0",
                "service-execution-health-gate-repair-run-18-2-service_execution_health_gate_admission_rate-0-500-0-67",
            ]
        );
        assert_eq!(queue.task_ids(), vec!["business-task"]);
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_records_and_admits_stable_boundary() {
        let service_recorder = AgentServiceExecutionHistoryRecorder::new();
        let handoff = AgentServiceExecutionHealthGateTrendHandoff::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());
        let gate_record = service_recorder.record_summary_with_health_gate(
            AgentServiceExecutionHistory::new(),
            clean,
            AgentServiceExecutionHealthPolicy::default(),
            "run/19",
            &queue,
        );

        let record = handoff.record_gate_record_and_gate(
            AgentServiceExecutionHealthGateHistory::new(),
            &gate_record,
            AgentServiceExecutionHealthGateHealthPolicy::default(),
            "run/20",
            &queue,
        );

        assert!(record.is_admitted());
        assert_eq!(record.trend_record.history.len(), 1);
        assert_eq!(
            record.trend_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.gate_decision.repair_tasks.is_empty());
        assert_eq!(
            record.gate_summary.next_queue_task_ids,
            vec!["business-task"]
        );
        assert_eq!(record.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(queue.task_ids(), vec!["business-task"]);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_admitted=true"
        }));

        let summary = record.summary();
        assert_eq!(
            summary.trend_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.trend_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(summary.blocked_reasons.is_empty());
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_summary_status=stable"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_records_and_repairs_dirty_boundary() {
        let service_recorder = AgentServiceExecutionHistoryRecorder::new();
        let handoff = AgentServiceExecutionHealthGateTrendHandoff::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());
        let dirty = service_report_summary(
            false,
            (4, 1, 1, 1),
            2,
            4,
            vec!["service_command_failed=emit_telemetry:writer offline"],
        );
        let clean_gate_record = service_recorder.record_summary_with_health_gate(
            AgentServiceExecutionHistory::new(),
            clean,
            AgentServiceExecutionHealthPolicy::default(),
            "run/21",
            &queue,
        );
        let clean_handoff = handoff.record_gate_record_and_gate(
            AgentServiceExecutionHealthGateHistory::new(),
            &clean_gate_record,
            AgentServiceExecutionHealthGateHealthPolicy::default(),
            "run/22",
            &queue,
        );
        let clean_handoff_summary = clean_handoff.summary();
        let dirty_gate_record = service_recorder.record_summary_with_health_gate(
            clean_gate_record.health_record.history,
            dirty,
            AgentServiceExecutionHealthPolicy::default(),
            "run/23",
            &queue,
        );

        let dirty_handoff = handoff.record_gate_record_and_gate(
            clean_handoff.trend_record.history,
            &dirty_gate_record,
            AgentServiceExecutionHealthGateHealthPolicy::default(),
            "run/24",
            &queue,
        );

        assert!(!dirty_handoff.is_admitted());
        assert_eq!(dirty_handoff.trend_record.history.len(), 2);
        assert_eq!(
            dirty_handoff.trend_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(dirty_handoff.gate_decision.requires_repair_first);
        assert_eq!(dirty_handoff.gate_decision.repair_tasks.len(), 3);
        assert!(
            dirty_handoff
                .gate_decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "service-execution-health-gate")
        );
        assert_eq!(
            dirty_handoff.gate_summary.next_queue_task_ids,
            dirty_handoff.next_queue().task_ids()
        );
        assert_eq!(queue.task_ids(), vec!["business-task"]);
        assert!(dirty_handoff.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_requires_repair_first=true"
        }));

        let history = AgentServiceExecutionHealthGateTrendHandoffHistory::from_summaries(vec![
            clean_handoff_summary,
            dirty_handoff.summary(),
        ]);
        let dashboard = history.dashboard();

        assert_eq!(history.len(), 2);
        assert_eq!(history.summaries().len(), 2);
        assert_eq!(
            history.latest().map(|summary| summary.trend_health_status),
            Some(AgentClosedLoopExecutionHealthStatus::Repair)
        );
        assert_eq!(dashboard.total_records, 2);
        assert_eq!(dashboard.admitted_records, 1);
        assert_eq!(dashboard.repair_first_records, 1);
        assert_eq!(dashboard.stable_records, 1);
        assert_eq!(dashboard.watch_records, 0);
        assert_eq!(dashboard.repair_records, 1);
        assert_eq!(dashboard.repair_task_count, 3);
        assert_eq!(dashboard.total_next_queue_tasks, 5);
        assert_eq!(dashboard.admission_rate, 0.5);
        assert_eq!(dashboard.repair_first_rate, 0.5);
        assert_eq!(
            dashboard.latest_trend_health_status,
            Some(AgentClosedLoopExecutionHealthStatus::Repair)
        );
        assert_eq!(
            dashboard.latest_blocked_reasons,
            dirty_handoff.summary().blocked_reasons
        );
        assert!(!dashboard.is_clean());
        assert!(dashboard.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_dashboard_repair=1"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_health_watches_empty_history() {
        let health = AgentServiceExecutionHealthGateTrendHandoffHistory::new()
            .health(AgentServiceExecutionHealthGateTrendHandoffHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["service_execution_health_gate_trend_handoff_history_empty"]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_health_marks_stable_admissions_stable() {
        let history = AgentServiceExecutionHealthGateTrendHandoffHistory::from_summaries(vec![
            trend_handoff_summary(
                AgentClosedLoopExecutionHealthStatus::Stable,
                true,
                false,
                0,
                1,
                Vec::new(),
            ),
            trend_handoff_summary(
                AgentClosedLoopExecutionHealthStatus::Stable,
                true,
                false,
                0,
                2,
                Vec::new(),
            ),
        ]);

        let health =
            history.health(AgentServiceExecutionHealthGateTrendHandoffHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert!(health.is_stable());
        assert_eq!(health.dashboard.admitted_records, 2);
        assert_eq!(health.dashboard.admission_rate, 1.0);
        assert_eq!(health.dashboard.repair_task_count, 0);
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_history_recorder_repairs_dirty_handoff() {
        let recorder = AgentServiceExecutionHealthGateTrendHandoffHistoryRecorder::new();
        let stable = trend_handoff_summary(
            AgentClosedLoopExecutionHealthStatus::Stable,
            true,
            false,
            0,
            1,
            Vec::new(),
        );
        let repair = trend_handoff_summary(
            AgentClosedLoopExecutionHealthStatus::Repair,
            false,
            true,
            2,
            3,
            vec!["service_execution_health_gate_repair_first_rate=0.500>0"],
        );

        let first = recorder.record_summary_with_health(
            AgentServiceExecutionHealthGateTrendHandoffHistory::new(),
            stable,
            AgentServiceExecutionHealthGateTrendHandoffHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            repair.clone(),
            AgentServiceExecutionHealthGateTrendHandoffHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.appended_summary, repair);
        assert_eq!(
            second.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "service_execution_health_gate_trend_handoff_repair_first_rate=0.500>0",
                "service_execution_health_gate_trend_handoff_repair_records=1>0",
                "service_execution_health_gate_trend_handoff_repair_tasks=2>0",
                "service_execution_health_gate_trend_handoff_admission_rate=0.500<0.67",
            ]
        );
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 2);
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_history_record_status=repair"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_admits_stable_history() {
        let service_recorder = AgentServiceExecutionHistoryRecorder::new();
        let handoff_builder = AgentServiceExecutionHealthGateTrendHandoff::new();
        let monitor = AgentServiceExecutionHealthGateTrendHandoffMonitor::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());
        let gate_record = service_recorder.record_summary_with_health_gate(
            AgentServiceExecutionHistory::new(),
            clean,
            AgentServiceExecutionHealthPolicy::default(),
            "run/25",
            &queue,
        );
        let handoff = handoff_builder.record_gate_record_and_gate(
            AgentServiceExecutionHealthGateHistory::new(),
            &gate_record,
            AgentServiceExecutionHealthGateHealthPolicy::default(),
            "run/26",
            &queue,
        );

        let monitor_record = monitor.record_and_gate(
            handoff,
            AgentServiceExecutionHealthGateTrendHandoffHistory::new(),
            AgentServiceExecutionHealthGateTrendHandoffHealthPolicy::default(),
            "run/27",
        );

        assert!(monitor_record.is_admitted());
        assert_eq!(monitor_record.history_record.history.len(), 1);
        assert_eq!(
            monitor_record.history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(monitor_record.gate_decision.repair_tasks.is_empty());
        assert_eq!(
            monitor_record.next_queue().task_ids(),
            vec!["business-task"]
        );
        assert!(monitor_record.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_admitted=true"
        }));

        let summary = monitor_record.summary();
        assert_eq!(
            summary.handoff_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.handoff_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_summary_status=stable"
        }));

        let history =
            AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::from_summaries(vec![
                summary,
            ]);
        let dashboard = history.dashboard();

        assert_eq!(history.len(), 1);
        assert_eq!(dashboard.total_records, 1);
        assert_eq!(dashboard.requested_admitted_records, 1);
        assert_eq!(dashboard.admitted_records, 1);
        assert_eq!(dashboard.stable_records, 1);
        assert_eq!(dashboard.repair_task_count, 0);
        assert_eq!(dashboard.admission_rate, 1.0);
        assert!(dashboard.is_clean());
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_blocks_repair_history() {
        let service_recorder = AgentServiceExecutionHistoryRecorder::new();
        let handoff_builder = AgentServiceExecutionHealthGateTrendHandoff::new();
        let monitor = AgentServiceExecutionHealthGateTrendHandoffMonitor::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let clean = service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new());
        let dirty = service_report_summary(
            false,
            (4, 1, 1, 1),
            2,
            4,
            vec!["service_command_failed=emit_telemetry:writer offline"],
        );
        let clean_gate_record = service_recorder.record_summary_with_health_gate(
            AgentServiceExecutionHistory::new(),
            clean,
            AgentServiceExecutionHealthPolicy::default(),
            "run/28",
            &queue,
        );
        let clean_handoff = handoff_builder.record_gate_record_and_gate(
            AgentServiceExecutionHealthGateHistory::new(),
            &clean_gate_record,
            AgentServiceExecutionHealthGateHealthPolicy::default(),
            "run/29",
            &queue,
        );
        let clean_handoff_summary = clean_handoff.summary();
        let dirty_gate_record = service_recorder.record_summary_with_health_gate(
            clean_gate_record.health_record.history,
            dirty,
            AgentServiceExecutionHealthPolicy::default(),
            "run/30",
            &queue,
        );
        let dirty_handoff = handoff_builder.record_gate_record_and_gate(
            clean_handoff.trend_record.history,
            &dirty_gate_record,
            AgentServiceExecutionHealthGateHealthPolicy::default(),
            "run/31",
            &queue,
        );
        let history = AgentServiceExecutionHealthGateTrendHandoffHistory::from_summaries(vec![
            clean_handoff_summary,
        ]);

        let monitor_record = monitor.record_and_gate(
            dirty_handoff,
            history,
            AgentServiceExecutionHealthGateTrendHandoffHealthPolicy::default(),
            "run/32",
        );

        assert!(!monitor_record.is_admitted());
        assert!(!monitor_record.gate_decision.requested_admitted);
        assert!(monitor_record.gate_decision.requires_repair_first);
        assert_eq!(
            monitor_record.history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(monitor_record.gate_decision.repair_tasks.len(), 4);
        assert!(
            monitor_record
                .gate_decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "service-execution-health-gate-handoff")
        );
        assert_eq!(
            monitor_record
                .gate_decision
                .repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "service-execution-health-gate-handoff-repair-run-32-0-service_execution_health_gate_trend_handoff_repair_first_rate-0-500-0",
                "service-execution-health-gate-handoff-repair-run-32-1-service_execution_health_gate_trend_handoff_repair_records-1-0",
                "service-execution-health-gate-handoff-repair-run-32-2-service_execution_health_gate_trend_handoff_repair_tasks-3-0",
                "service-execution-health-gate-handoff-repair-run-32-3-service_execution_health_gate_trend_handoff_admission_rate-0-500-0-67",
            ]
        );
        assert_eq!(
            monitor_record.history_record.health.reasons,
            vec![
                "service_execution_health_gate_trend_handoff_repair_first_rate=0.500>0",
                "service_execution_health_gate_trend_handoff_repair_records=1>0",
                "service_execution_health_gate_trend_handoff_repair_tasks=3>0",
                "service_execution_health_gate_trend_handoff_admission_rate=0.500<0.67",
            ]
        );
        assert_eq!(
            monitor_record
                .next_queue()
                .task_ids()
                .into_iter()
                .filter(|id| id.starts_with("service-execution-health-gate-handoff-repair"))
                .count(),
            4
        );
        assert!(monitor_record.gate_decision.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_gate_requires_repair_first=true"
        }));

        let summary = monitor_record.summary();
        assert_eq!(
            summary.handoff_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.handoff_records, 2);
        assert_eq!(summary.repair_tasks, 4);
        assert_eq!(summary.next_queue_tasks, 8);
        assert_eq!(summary.blocked_reasons, 7);

        let history =
            AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::from_summaries(vec![
                summary,
            ]);
        let dashboard = history.dashboard();

        assert_eq!(
            history
                .latest()
                .map(|summary| summary.handoff_health_status),
            Some(AgentClosedLoopExecutionHealthStatus::Repair)
        );
        assert_eq!(dashboard.total_records, 1);
        assert_eq!(dashboard.requested_admitted_records, 0);
        assert_eq!(dashboard.admitted_records, 0);
        assert_eq!(dashboard.repair_first_records, 1);
        assert_eq!(dashboard.repair_records, 1);
        assert_eq!(dashboard.repair_task_count, 4);
        assert_eq!(dashboard.total_next_queue_tasks, 8);
        assert_eq!(dashboard.blocked_reasons, 7);
        assert_eq!(dashboard.admission_rate, 0.0);
        assert_eq!(dashboard.repair_first_rate, 1.0);
        assert!(!dashboard.is_clean());
        assert!(dashboard.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_dashboard_repair=1"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_health_watches_empty_history() {
        let health = AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::new()
            .health(AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["service_execution_health_gate_trend_handoff_monitor_history_empty"]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_health_marks_clean_trend_stable() {
        let history =
            AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::from_summaries(vec![
                trend_handoff_monitor_summary(
                    AgentClosedLoopExecutionHealthStatus::Stable,
                    true,
                    true,
                    false,
                    0,
                    1,
                    0,
                ),
                trend_handoff_monitor_summary(
                    AgentClosedLoopExecutionHealthStatus::Stable,
                    true,
                    true,
                    false,
                    0,
                    2,
                    0,
                ),
            ]);

        let health = history
            .health(AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert!(health.is_stable());
        assert_eq!(health.dashboard.admission_rate, 1.0);
        assert_eq!(health.dashboard.repair_first_rate, 0.0);
        assert_eq!(health.dashboard.repair_task_count, 0);
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_history_recorder_repairs_dirty_trend() {
        let recorder =
            AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecorder::new();
        let stable = trend_handoff_monitor_summary(
            AgentClosedLoopExecutionHealthStatus::Stable,
            true,
            true,
            false,
            0,
            1,
            0,
        );
        let repair = trend_handoff_monitor_summary(
            AgentClosedLoopExecutionHealthStatus::Repair,
            false,
            false,
            true,
            4,
            8,
            7,
        );

        let first = recorder.record_summary_with_health(
            AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::new(),
            stable,
            AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            repair.clone(),
            AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.appended_summary, repair);
        assert_eq!(
            second.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "service_execution_health_gate_trend_handoff_monitor_repair_first_rate=0.500>0",
                "service_execution_health_gate_trend_handoff_monitor_repair_records=1>0",
                "service_execution_health_gate_trend_handoff_monitor_repair_tasks=4>0",
                "service_execution_health_gate_trend_handoff_monitor_blocked_reasons=7>0",
                "service_execution_health_gate_trend_handoff_monitor_admission_rate=0.500<0.67",
            ]
        );
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 4);
        assert_eq!(second.dashboard.blocked_reasons, 7);
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_history_record_status=repair"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_gate_preserves_stable_queue() {
        let monitor_record = stable_trend_handoff_monitor_record();
        let history_record =
            AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecorder::new()
                .record_monitor_with_health(
                    AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::new(),
                    &monitor_record,
                    AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default(),
                );

        let decision = AgentServiceExecutionHealthGateTrendHandoffMonitorGate::new().evaluate(
            "run/33",
            &monitor_record,
            &history_record,
        );

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.monitor_health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_gate_status=stable"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_gate_preserves_watch_queue() {
        let monitor_record = stable_trend_handoff_monitor_record();
        let history_record =
            AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecorder::new()
                .record_monitor_with_health(
                    AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::new(),
                    &monitor_record,
                    AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy {
                        minimum_admission_rate: 1.1,
                        ..AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default()
                    },
                );

        let decision = AgentServiceExecutionHealthGateTrendHandoffMonitorGate::new().evaluate(
            "run/34",
            &monitor_record,
            &history_record,
        );

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.monitor_health.status,
            AgentClosedLoopExecutionHealthStatus::Watch
        );
        assert_eq!(
            decision.monitor_health.reasons,
            vec!["service_execution_health_gate_trend_handoff_monitor_admission_rate=1.000<1.1"]
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_gate_status=watch"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_gate_blocks_repair_history() {
        let monitor_record = stable_trend_handoff_monitor_record();
        let recorder =
            AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistoryRecorder::new();
        let stable = trend_handoff_monitor_summary(
            AgentClosedLoopExecutionHealthStatus::Stable,
            true,
            true,
            false,
            0,
            1,
            0,
        );
        let repair = trend_handoff_monitor_summary(
            AgentClosedLoopExecutionHealthStatus::Repair,
            false,
            false,
            true,
            4,
            8,
            7,
        );
        let first = recorder.record_summary_with_health(
            AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::new(),
            stable,
            AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default(),
        );
        let history_record = recorder.record_summary_with_health(
            first.history,
            repair,
            AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default(),
        );

        let decision = AgentServiceExecutionHealthGateTrendHandoffMonitorGate::new().evaluate(
            "run/35",
            &monitor_record,
            &history_record,
        );

        assert!(decision.requested_admitted);
        assert!(!decision.is_admitted());
        assert_eq!(
            decision.monitor_health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert_eq!(decision.repair_tasks.len(), 5);
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "service-execution-health-gate-handoff-monitor")
        );
        assert_eq!(
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "service-execution-health-gate-handoff-monitor-repair-run-35-0-service_execution_health_gate_trend_handoff_monitor_repair_first_rate-0-500-0",
                "service-execution-health-gate-handoff-monitor-repair-run-35-1-service_execution_health_gate_trend_handoff_monitor_repair_records-1-0",
                "service-execution-health-gate-handoff-monitor-repair-run-35-2-service_execution_health_gate_trend_handoff_monitor_repair_tasks-4-0",
                "service-execution-health-gate-handoff-monitor-repair-run-35-3-service_execution_health_gate_trend_handoff_monitor_blocked_reasons-7-0",
                "service-execution-health-gate-handoff-monitor-repair-run-35-4-service_execution_health_gate_trend_handoff_monitor_admission_rate-0-500-0-67",
            ]
        );
        assert_eq!(
            decision.next_queue.task_ids().first().map(String::as_str),
            Some("business-task")
        );
        assert_eq!(decision.next_queue.len(), 6);
        assert_eq!(decision.blocked_reasons, history_record.health.reasons);
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_handoff_records_stable_gate() {
        let monitor_record = stable_trend_handoff_monitor_record();

        let handoff = AgentServiceExecutionHealthGateTrendHandoffMonitorHandoff::new()
            .record_and_gate(
                monitor_record,
                AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::new(),
                AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default(),
                "run/36",
            );

        assert!(handoff.is_admitted());
        assert_eq!(handoff.history_record.history.len(), 1);
        assert_eq!(
            handoff.history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(handoff.gate_decision.repair_tasks.is_empty());
        assert_eq!(handoff.next_queue().task_ids(), vec!["business-task"]);
        assert!(handoff.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_handoff=true"
        }));
        assert!(handoff.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_handoff_admitted=true"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_handoff_repairs_dirty_history() {
        let monitor_record = stable_trend_handoff_monitor_record();
        let dirty_history =
            AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::from_summaries(vec![
                trend_handoff_monitor_summary(
                    AgentClosedLoopExecutionHealthStatus::Repair,
                    false,
                    false,
                    true,
                    4,
                    8,
                    7,
                ),
            ]);

        let handoff = AgentServiceExecutionHealthGateTrendHandoffMonitorHandoff::new()
            .record_and_gate(
                monitor_record,
                dirty_history,
                AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default(),
                "run/37",
            );

        assert!(!handoff.is_admitted());
        assert_eq!(handoff.history_record.history.len(), 2);
        assert_eq!(
            handoff.history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(handoff.gate_decision.requires_repair_first);
        assert_eq!(handoff.gate_decision.repair_tasks.len(), 5);
        assert_eq!(handoff.next_queue().len(), 6);
        assert_eq!(
            handoff
                .gate_decision
                .repair_tasks
                .iter()
                .map(|task| task.lane.as_str())
                .collect::<Vec<_>>(),
            vec![
                "service-execution-health-gate-handoff-monitor",
                "service-execution-health-gate-handoff-monitor",
                "service-execution-health-gate-handoff-monitor",
                "service-execution-health-gate-handoff-monitor",
                "service-execution-health-gate-handoff-monitor",
            ]
        );
        assert!(handoff.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_handoff_requires_repair_first=true"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_handoff_summary_compacts_stable_gate() {
        let monitor_record = stable_trend_handoff_monitor_record();
        let handoff = AgentServiceExecutionHealthGateTrendHandoffMonitorHandoff::new()
            .record_and_gate(
                monitor_record,
                AgentServiceExecutionHealthGateTrendHandoffMonitorSummaryHistory::new(),
                AgentServiceExecutionHealthGateTrendHandoffMonitorHealthPolicy::default(),
                "run/38",
            );

        let summary = handoff.summary();

        assert_eq!(
            summary.monitor_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.monitor_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert_eq!(summary.blocked_reasons, 0);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_handoff_summary_status=stable"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_handoff_health_watches_empty_history() {
        let health = AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory::new()
            .health(
                AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
            );

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec![
                "service_execution_health_gate_trend_handoff_monitor_handoff_history_empty"
                    .to_owned()
            ]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_handoff_history_recorder_repairs_dirty_trend()
     {
        let recorder =
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder::new();
        let stable = trend_handoff_monitor_handoff_summary(
            AgentClosedLoopExecutionHealthStatus::Stable,
            true,
            true,
            false,
            0,
            1,
            0,
        );
        let repair = trend_handoff_monitor_handoff_summary(
            AgentClosedLoopExecutionHealthStatus::Repair,
            false,
            false,
            true,
            5,
            6,
            5,
        );

        let first = recorder.record_summary_with_health(
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
            stable,
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            repair.clone(),
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.appended_summary, repair);
        assert_eq!(
            second.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "service_execution_health_gate_trend_handoff_monitor_handoff_repair_first_rate=0.500>0",
                "service_execution_health_gate_trend_handoff_monitor_handoff_repair_records=1>0",
                "service_execution_health_gate_trend_handoff_monitor_handoff_repair_tasks=5>0",
                "service_execution_health_gate_trend_handoff_monitor_handoff_blocked_reasons=5>0",
                "service_execution_health_gate_trend_handoff_monitor_handoff_admission_rate=0.500<0.67",
            ]
        );
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 5);
        assert_eq!(second.dashboard.blocked_reasons, 5);
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_handoff_history_record_status=repair"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_handoff_gate_preserves_stable_queue() {
        let handoff = stable_trend_handoff_monitor_handoff_record();
        let history_record =
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder::new()
                .record_handoff_with_health(
                    AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                    &handoff,
                    AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(
                    ),
                );

        let decision = AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGate::new()
            .evaluate("run/39", &handoff, &history_record);

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.handoff_health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_handoff_gate_status=stable"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_handoff_gate_preserves_watch_queue() {
        let handoff = stable_trend_handoff_monitor_handoff_record();
        let history_record =
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder::new()
                .record_handoff_with_health(
                    AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                    &handoff,
                    AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy {
                        minimum_admission_rate: 1.1,
                        ..AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy::default()
                    },
                );

        let decision = AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGate::new()
            .evaluate("run/40", &handoff, &history_record);

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.handoff_health.status,
            AgentClosedLoopExecutionHealthStatus::Watch
        );
        assert_eq!(
            decision.handoff_health.reasons,
            vec![
                "service_execution_health_gate_trend_handoff_monitor_handoff_admission_rate=1.000<1.1"
                    .to_owned()
            ]
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_handoff_gate_blocks_repair_history() {
        let handoff = stable_trend_handoff_monitor_handoff_record();
        let recorder =
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder::new();
        let stable = trend_handoff_monitor_handoff_summary(
            AgentClosedLoopExecutionHealthStatus::Stable,
            true,
            true,
            false,
            0,
            1,
            0,
        );
        let repair = trend_handoff_monitor_handoff_summary(
            AgentClosedLoopExecutionHealthStatus::Repair,
            false,
            false,
            true,
            5,
            6,
            5,
        );
        let first = recorder.record_summary_with_health(
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
            stable,
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
        );
        let history_record = recorder.record_summary_with_health(
            first.history,
            repair,
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
        );

        let decision = AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGate::new()
            .evaluate("run/41", &handoff, &history_record);

        assert!(decision.requested_admitted);
        assert!(!decision.is_admitted());
        assert_eq!(
            decision.handoff_health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert_eq!(decision.repair_tasks.len(), 5);
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "service-execution-health-gate-handoff-monitor-handoff")
        );
        assert_eq!(
            decision.next_queue.task_ids().first().map(String::as_str),
            Some("business-task")
        );
        assert_eq!(decision.next_queue.len(), 6);
        assert_eq!(decision.blocked_reasons, history_record.health.reasons);
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_handoff_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_handoff_handoff_records_stable_gate() {
        let handoff = stable_trend_handoff_monitor_handoff_record();

        let record = AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/42",
            );

        assert!(record.is_admitted());
        assert_eq!(record.history_record.history.len(), 1);
        assert_eq!(
            record.history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.gate_decision.repair_tasks.is_empty());
        assert_eq!(record.next_queue().task_ids(), vec!["business-task"]);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_handoff_handoff=true"
        }));
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_handoff_handoff_admitted=true"
        }));
    }

    #[test]
    fn service_execution_health_gate_trend_handoff_monitor_handoff_handoff_repairs_dirty_history() {
        let handoff = stable_trend_handoff_monitor_handoff_record();
        let dirty_history =
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory::from_summaries(
                vec![trend_handoff_monitor_handoff_summary(
                    AgentClosedLoopExecutionHealthStatus::Repair,
                    false,
                    false,
                    true,
                    5,
                    6,
                    5,
                )],
            );

        let record = AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                dirty_history,
                AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/43",
            );

        assert!(!record.is_admitted());
        assert_eq!(record.history_record.history.len(), 2);
        assert_eq!(
            record.history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(record.gate_decision.requires_repair_first);
        assert_eq!(record.gate_decision.repair_tasks.len(), 5);
        assert_eq!(record.next_queue().len(), 6);
        assert!(
            record
                .gate_decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "service-execution-health-gate-handoff-monitor-handoff")
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_trend_handoff_monitor_handoff_handoff_requires_repair_first=true"
        }));
    }

    #[test]
    fn service_execution_health_gate_admits_stable_and_watch_without_repair_tasks() {
        let gate = AgentServiceExecutionHealthGate::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);
        let stable_health =
            AgentServiceExecutionHistory::from_summaries(vec![service_report_summary(
                true,
                (2, 0, 0, 0),
                0,
                0,
                Vec::new(),
            )])
            .health(AgentServiceExecutionHealthPolicy::default());
        let watch_health = AgentServiceExecutionHistory::new()
            .health(AgentServiceExecutionHealthPolicy::default());

        let stable = gate.evaluate("run/8", &stable_health, &queue);
        let watch = gate.evaluate("run/8", &watch_health, &queue);

        assert!(stable.is_admitted());
        assert_eq!(
            stable.health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(stable.repair_tasks.is_empty());
        assert_eq!(stable.next_queue.task_ids(), vec!["business-task"]);
        assert!(stable.blocked_reasons.is_empty());
        assert!(watch.is_admitted());
        assert_eq!(
            watch.health_status,
            AgentClosedLoopExecutionHealthStatus::Watch
        );
        assert!(watch.repair_tasks.is_empty());
        assert_eq!(watch.next_queue.task_ids(), vec!["business-task"]);
        assert_eq!(queue.task_ids(), vec!["business-task"]);
    }

    #[test]
    fn service_execution_health_gate_blocks_repair_and_merges_repair_queue() {
        let history = AgentServiceExecutionHistory::from_summaries(vec![
            service_report_summary(true, (2, 0, 0, 0), 0, 0, Vec::new()),
            service_report_summary(
                false,
                (4, 1, 1, 1),
                3,
                5,
                vec!["service_command_failed=emit_telemetry:writer offline"],
            ),
        ]);
        let health = history.health(AgentServiceExecutionHealthPolicy::default());
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue service command handoff",
            crate::budget::AgentBudget::new(4, 1, 1),
        )]);

        let decision = AgentServiceExecutionHealthGate::new().evaluate("run/9", &health, &queue);

        assert!(!decision.admitted);
        assert!(!decision.is_admitted());
        assert!(decision.requires_repair_first);
        assert_eq!(
            decision.health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(decision.blocked_reasons, health.reasons);
        assert_eq!(decision.repair_tasks.len(), 3);
        assert_eq!(
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "service-execution-health-repair-run-9-0-service_execution_drift_rate-0-500-0",
                "service-execution-health-repair-run-9-1-service_execution_repair_tasks-3-0",
                "service-execution-health-repair-run-9-2-service_execution_clean_rate-0-500-0-67",
            ]
        );
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "service-execution-health" && task.priority == 9)
        );
        assert_eq!(
            decision.next_queue.task_ids(),
            vec![
                "business-task",
                "service-execution-health-repair-run-9-0-service_execution_drift_rate-0-500-0",
                "service-execution-health-repair-run-9-1-service_execution_repair_tasks-3-0",
                "service-execution-health-repair-run-9-2-service_execution_clean_rate-0-500-0-67",
            ]
        );
        assert_eq!(queue.task_ids(), vec!["business-task"]);
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_service_execution_health_gate_requires_repair_first=true"
        }));

        let summary = decision.summary();
        assert_eq!(
            summary.health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.repair_tasks, 3);
        assert_eq!(summary.next_queue_tasks, 4);
        assert_eq!(
            summary.repair_task_ids,
            vec![
                "service-execution-health-repair-run-9-0-service_execution_drift_rate-0-500-0",
                "service-execution-health-repair-run-9-1-service_execution_repair_tasks-3-0",
                "service-execution-health-repair-run-9-2-service_execution_clean_rate-0-500-0-67",
            ]
        );
        assert_eq!(summary.next_queue_task_ids, decision.next_queue.task_ids());
        assert_eq!(summary.blocked_reasons, decision.blocked_reasons);
        assert!(
            summary.telemetry.iter().any(|line| {
                line == "agent_service_execution_health_gate_summary_status=repair"
            })
        );
    }

    #[test]
    fn service_feedback_turns_audit_blockers_into_repair_queue() {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "repair",
            AgentRole::Reviewer,
            "repair blocked loop",
            crate::budget::AgentBudget::new(8, 1, 1),
        )]);
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Repair,
            vec!["consecutive_blocked_cycles=3".to_owned()],
            queue,
            None,
        );
        let planner = AgentServiceCommandPlanner::new();
        let plan = planner.plan(&business_plan);
        let audit = planner.audit(
            &plan,
            vec![
                AgentServiceCommandReceipt::failed(&plan.commands[0], "repair writer offline"),
                AgentServiceCommandReceipt::skipped(&plan.commands[1], "queue paused"),
            ],
        );

        let feedback = AgentServiceFeedback::from_audit("run/7", audit);
        let task_ids = feedback
            .repair_tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>();
        let roles = feedback
            .repair_tasks
            .iter()
            .map(|task| task.role.clone())
            .collect::<Vec<_>>();

        assert!(!feedback.is_clean());
        assert_eq!(
            task_ids,
            vec![
                "service-feedback-run-7-0-emit_telemetry",
                "service-feedback-run-7-1-open_repair_mode",
                "service-feedback-run-7-2-enqueue_tasks",
            ]
        );
        assert_eq!(
            roles,
            vec![
                AgentRole::Aggregator,
                AgentRole::Reviewer,
                AgentRole::Planner
            ]
        );
        assert_eq!(feedback.next_queue.task_ids(), task_ids);

        let summary = feedback.summary();
        assert!(!summary.audit_clean);
        assert_eq!(summary.repair_tasks, 3);
        assert_eq!(summary.next_queue_tasks, 3);
        assert_eq!(
            summary.blocked_reasons,
            vec![
                "service_command_missing=emit_telemetry",
                "service_command_failed=open_repair_mode:repair writer offline",
                "service_command_skipped=enqueue_tasks:queue paused",
            ]
        );
        assert!(!summary.clean);
    }

    #[test]
    fn service_feedback_is_clean_for_clean_audit() {
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::new(),
            Some(AdaptiveStateCandidate {
                run_id: "run-1".to_owned(),
                reward_total: 0.91,
                acceptance_rate: 1.0,
                average_reward_total: 0.91,
                evidence_refs: vec!["eval:pass".to_owned()],
            }),
        );
        let planner = AgentServiceCommandPlanner::new();
        let plan = planner.plan(&business_plan);
        let receipts = plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let audit = planner.audit(&plan, receipts);

        let feedback = AgentServiceFeedback::from_audit("run-1", audit);

        assert!(feedback.is_clean());
        assert!(feedback.repair_tasks.is_empty());
        assert!(feedback.next_queue.is_empty());

        let summary = feedback.summary();
        assert!(summary.audit_clean);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_tasks, 0);
        assert!(summary.blocked_reasons.is_empty());
        assert!(summary.clean);
    }

    #[test]
    fn service_turnover_keeps_business_queue_when_feedback_is_clean() {
        let planned = AgentTask::new(
            "planned-follow-up",
            AgentRole::Tester,
            "continue validation",
            crate::budget::AgentBudget::new(8, 1, 1),
        );
        let mut business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Hold,
            vec!["more_evidence_required".to_owned()],
            AgentTaskQueue::from_tasks(vec![planned]),
            None,
        );
        business_plan.telemetry.clear();
        let planner = AgentServiceCommandPlanner::new();
        let command_plan = planner.plan(&business_plan);
        let receipts = command_plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let feedback =
            AgentServiceFeedback::from_audit("run-8", planner.audit(&command_plan, receipts));

        let turnover = AgentServiceTurnover::from_feedback(&business_plan, feedback);

        assert!(turnover.is_clean());
        assert_eq!(turnover.next_queue.task_ids(), vec!["planned-follow-up"]);
        assert!(turnover.blocked_reasons.is_empty());

        let summary = turnover.summary();
        assert!(summary.feedback_clean);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_tasks, 1);
        assert!(summary.blocked_reasons.is_empty());
        assert!(summary.clean);
    }

    #[test]
    fn service_turnover_merges_feedback_repairs_with_business_queue() {
        let planned = AgentTask::new(
            "planned-follow-up",
            AgentRole::Tester,
            "continue validation",
            crate::budget::AgentBudget::new(8, 1, 1),
        )
        .with_priority(3);
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Hold,
            vec!["more_evidence_required".to_owned()],
            AgentTaskQueue::from_tasks(vec![planned]),
            None,
        );
        let planner = AgentServiceCommandPlanner::new();
        let command_plan = planner.plan(&business_plan);
        let audit = planner.audit(
            &command_plan,
            vec![AgentServiceCommandReceipt::failed(
                &command_plan.commands[0],
                "hold writer offline",
            )],
        );
        let feedback = AgentServiceFeedback::from_audit("run-9", audit);

        let mut turnover = AgentServiceTurnover::from_feedback(&business_plan, feedback);
        let summary = turnover.summary();
        let repair_wave = turnover
            .next_queue
            .drain_ready(&std::collections::BTreeSet::new());
        let repair_wave_ids = repair_wave
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>();
        let completed_repairs = repair_wave
            .iter()
            .map(|task| task.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let business_wave = turnover.next_queue.ready_tasks(&completed_repairs);
        let business_wave_ids = business_wave
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>();

        assert!(!turnover.is_clean());
        assert_eq!(
            turnover.blocked_reasons,
            vec![
                "service_command_missing=enqueue_tasks",
                "service_command_missing=emit_telemetry",
                "service_command_failed=hold_business_loop:hold writer offline",
            ]
        );
        assert_eq!(
            repair_wave_ids,
            vec![
                "service-feedback-run-9-0-enqueue_tasks",
                "service-feedback-run-9-1-emit_telemetry",
                "service-feedback-run-9-2-hold_business_loop",
            ]
        );
        assert_eq!(business_wave_ids, vec!["planned-follow-up"]);

        assert!(!summary.feedback_clean);
        assert_eq!(summary.repair_tasks, 3);
        assert_eq!(summary.next_queue_tasks, 4);
        assert_eq!(
            summary.blocked_reasons,
            vec![
                "service_command_missing=enqueue_tasks",
                "service_command_missing=emit_telemetry",
                "service_command_failed=hold_business_loop:hold writer offline",
            ]
        );
        assert!(!summary.clean);
    }

    #[test]
    fn service_execution_report_closes_clean_command_receipts() {
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Promote,
            Vec::new(),
            AgentTaskQueue::new(),
            Some(AdaptiveStateCandidate {
                run_id: "run-10".to_owned(),
                reward_total: 0.94,
                acceptance_rate: 1.0,
                average_reward_total: 0.94,
                evidence_refs: vec!["eval:pass".to_owned()],
            }),
        );
        let planner = AgentServiceCommandPlanner::new();
        let command_plan = planner.plan(&business_plan);
        let receipts = command_plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();

        let report = planner.close_execution("run-10", &business_plan, receipts);

        assert!(report.is_clean());
        assert_eq!(
            report.command_plan.command_kinds(),
            vec!["promote_adaptive_state", "emit_telemetry"]
        );
        assert!(report.next_queue().is_empty());
        assert!(report.turnover.blocked_reasons.is_empty());

        let summary = report.summary();
        assert!(summary.clean);
        assert_eq!(summary.command_count, 2);
        assert_eq!(summary.memory_promotion_reason_count, 0);
        assert_eq!(summary.tool_build_reason_count, 0);
        assert_eq!(summary.expected_commands, 2);
        assert_eq!(summary.receipts, 2);
        assert_eq!(summary.missing_commands, 0);
        assert_eq!(summary.failed_commands, 0);
        assert_eq!(summary.skipped_commands, 0);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_tasks, 0);
        assert!(summary.blocked_reasons.is_empty());
        assert!(
            summary.telemetry.iter().any(|line| {
                line == "agent_service_execution_report_summary_tool_build_reasons=0"
            })
        );
    }

    #[test]
    fn service_execution_surfaces_tool_build_repair_reason_pressure() {
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Repair,
            vec!["tool_build_blocked_cycles=1>0".to_owned()],
            AgentTaskQueue::new(),
            None,
        );
        let planner = AgentServiceCommandPlanner::new();
        let command_plan = planner.plan(&business_plan);
        let receipts = command_plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();

        let report = planner.close_execution("run-tool-build", &business_plan, receipts);
        let summary = report.summary();
        let dashboard =
            AgentServiceExecutionHistory::from_summaries(vec![summary.clone()]).dashboard();
        let health = dashboard.health(AgentServiceExecutionHealthPolicy {
            maximum_tool_build_reason_runs: 0,
            ..AgentServiceExecutionHealthPolicy::default()
        });

        assert!(summary.clean);
        assert_eq!(summary.memory_promotion_reason_count, 0);
        assert_eq!(summary.tool_build_reason_count, 1);
        assert_eq!(dashboard.tool_build_reason_count, 1);
        assert_eq!(dashboard.tool_build_reason_runs, 1);
        assert_eq!(dashboard.memory_promotion_reason_count, 0);
        assert_eq!(dashboard.memory_promotion_reason_runs, 0);
        assert!(
            dashboard.telemetry.iter().any(|line| {
                line == "agent_service_execution_dashboard_tool_build_reason_runs=1"
            })
        );
        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Repair);
        assert_eq!(
            health.reasons,
            vec!["service_execution_tool_build_reason_runs=1>0"]
        );
    }

    #[test]
    fn service_execution_report_closes_dirty_receipts_into_turnover_queue() {
        let planned = AgentTask::new(
            "business-follow-up",
            AgentRole::Tester,
            "collect more evidence",
            crate::budget::AgentBudget::new(8, 1, 1),
        )
        .with_priority(2);
        let business_plan = plan(
            AgentCycleLedgerAdmissionStatus::Hold,
            vec!["more_evidence_required".to_owned()],
            AgentTaskQueue::from_tasks(vec![planned]),
            None,
        );
        let planner = AgentServiceCommandPlanner::new();
        let command_plan = planner.plan(&business_plan);
        let receipts = vec![AgentServiceCommandReceipt::failed(
            &command_plan.commands[0],
            "hold writer offline",
        )];

        let mut report = planner.close_execution("run-11", &business_plan, receipts);
        let summary = report.summary();
        let repair_wave = report
            .turnover
            .next_queue
            .drain_ready(&std::collections::BTreeSet::new())
            .into_iter()
            .collect::<Vec<_>>();
        let repair_wave_ids = repair_wave
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>();
        let completed_repairs = repair_wave
            .iter()
            .map(|task| task.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let business_wave_ids = report
            .turnover
            .next_queue
            .ready_tasks(&completed_repairs)
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>();

        assert!(!report.is_clean());
        assert_eq!(
            report.turnover.blocked_reasons,
            vec![
                "service_command_missing=enqueue_tasks",
                "service_command_missing=emit_telemetry",
                "service_command_failed=hold_business_loop:hold writer offline",
            ]
        );
        assert_eq!(
            repair_wave_ids,
            vec![
                "service-feedback-run-11-0-enqueue_tasks",
                "service-feedback-run-11-1-emit_telemetry",
                "service-feedback-run-11-2-hold_business_loop",
            ]
        );
        assert_eq!(business_wave_ids, vec!["business-follow-up"]);

        assert!(!summary.clean);
        assert_eq!(summary.command_count, 3);
        assert_eq!(
            summary.command_kinds,
            vec!["hold_business_loop", "enqueue_tasks", "emit_telemetry"]
        );
        assert_eq!(summary.expected_commands, 3);
        assert_eq!(summary.receipts, 1);
        assert_eq!(summary.missing_commands, 2);
        assert_eq!(summary.failed_commands, 1);
        assert_eq!(summary.skipped_commands, 0);
        assert_eq!(summary.repair_tasks, 3);
        assert_eq!(summary.next_queue_tasks, 4);
        assert_eq!(
            summary.blocked_reasons,
            vec![
                "service_command_missing=enqueue_tasks",
                "service_command_missing=emit_telemetry",
                "service_command_failed=hold_business_loop:hold writer offline",
            ]
        );
    }
}
