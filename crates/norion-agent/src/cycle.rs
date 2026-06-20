use crate::aggregate::AggregationReport;
use std::collections::BTreeSet;

use crate::budget::{BudgetLedger, BudgetPolicy};
use crate::conflict::{ConflictReport, ConflictResolutionBook};
use crate::evolution::{
    ClosedLoopRewarder, ProcessRewardInput, ProcessRewardReport, RewardAction, ToolsmithPlan,
};
use crate::execute::{AgentExecutionFailure, AgentWaveExecution};
use crate::ports::{MemoryNote, ToolBuildReport, ToolBuildReportSummary};
use crate::reflection::ReflectionLoop;
use crate::run::{
    AgentRunLedger, AgentRunLedgerAdmission, AgentRunReport, RunBudgetAudit, SideEffectGate,
    SideEffectKind,
};
use crate::schedule::AgentExecutionWave;
use crate::task::{
    AgentResult, AgentRole, AgentTask, AgentTaskQueue, DispatchPlanner, TaskDispatchPlan,
};

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleEvidence {
    pub quality: f32,
    pub validation_passed: bool,
    pub runtime_response_ok: bool,
    pub reflection: Option<ReflectionLoop>,
    pub conflict_resolutions: ConflictResolutionBook,
    pub toolsmith_plan: ToolsmithPlan,
    pub tool_build_report: Option<ToolBuildReport>,
}

impl Default for AgentCycleEvidence {
    fn default() -> Self {
        Self {
            quality: 0.5,
            validation_passed: false,
            runtime_response_ok: false,
            reflection: None,
            conflict_resolutions: ConflictResolutionBook::new(),
            toolsmith_plan: ToolsmithPlan::new(),
            tool_build_report: None,
        }
    }
}

impl AgentCycleEvidence {
    pub fn reflection_complete(&self) -> bool {
        self.reflection
            .as_ref()
            .is_some_and(ReflectionLoop::is_complete)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCycleDispatch {
    pub wave: AgentExecutionWave,
    pub dispatch: TaskDispatchPlan,
    pub assigned_tasks: Vec<AgentTask>,
    pub blocked_task_ids: Vec<String>,
    pub remaining_queue: AgentTaskQueue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleReport {
    pub dispatch: TaskDispatchPlan,
    pub execution_failures: Vec<AgentExecutionFailure>,
    pub run_ledger_admission: AgentRunLedgerAdmission,
    pub run_report: AgentRunReport,
    pub reward_report: ProcessRewardReport,
    pub tool_build_report: Option<ToolBuildReportSummary>,
    pub follow_up_tasks: Vec<AgentTask>,
    pub memory_promotions: Vec<MemoryPromotion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryPromotion {
    pub note: MemoryNote,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleSummary {
    pub assigned_tasks: usize,
    pub rejected_tasks: usize,
    pub unique_messages: usize,
    pub duplicate_groups: usize,
    pub unresolved_conflicts: usize,
    pub blocked_side_effects: usize,
    pub budget_overspends: usize,
    pub execution_failures: usize,
    pub reward_total: f32,
    pub reward_action: RewardAction,
    pub evolution_signals: usize,
    pub follow_up_tasks: usize,
    pub memory_promotions: usize,
    pub tool_build_reports: usize,
    pub tool_build_missing_requests: usize,
    pub tool_build_unexpected_receipts: usize,
    pub tool_build_duplicate_receipts: usize,
    pub tool_build_held_receipts: usize,
    pub tool_build_rejected_receipts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCycleHandoff {
    pub memory_notes: Vec<MemoryNote>,
    pub follow_up_tasks: Vec<AgentTask>,
    pub blocked_reasons: Vec<String>,
}

impl AgentCycleHandoff {
    pub fn from_report(report: &AgentCycleReport) -> Self {
        let summary = AgentCycleSummary::from_report(report);
        let mut blocked_reasons = Vec::new();
        if summary.unresolved_conflicts > 0 {
            blocked_reasons.push(format!(
                "unresolved_conflicts={}",
                summary.unresolved_conflicts
            ));
        }
        if summary.blocked_side_effects > 0 {
            blocked_reasons.push(format!(
                "blocked_side_effects={}",
                summary.blocked_side_effects
            ));
        }
        if summary.budget_overspends > 0 {
            blocked_reasons.push(format!("budget_overspends={}", summary.budget_overspends));
        }
        if summary.execution_failures > 0 {
            blocked_reasons.push(format!("execution_failures={}", summary.execution_failures));
        }
        if report.run_ledger_admission.requires_repair_first {
            blocked_reasons.extend(
                report
                    .run_ledger_admission
                    .reasons
                    .iter()
                    .map(|reason| format!("run_ledger_admission:{reason}")),
            );
        }
        if summary.tool_build_missing_requests > 0 {
            blocked_reasons.push(format!(
                "tool_build_missing_requests={}",
                summary.tool_build_missing_requests
            ));
        }
        if summary.tool_build_unexpected_receipts > 0 {
            blocked_reasons.push(format!(
                "tool_build_unexpected_receipts={}",
                summary.tool_build_unexpected_receipts
            ));
        }
        if summary.tool_build_duplicate_receipts > 0 {
            blocked_reasons.push(format!(
                "tool_build_duplicate_receipts={}",
                summary.tool_build_duplicate_receipts
            ));
        }
        if summary.tool_build_held_receipts > 0 {
            blocked_reasons.push(format!(
                "tool_build_held_receipts={}",
                summary.tool_build_held_receipts
            ));
        }
        if summary.tool_build_rejected_receipts > 0 {
            blocked_reasons.push(format!(
                "tool_build_rejected_receipts={}",
                summary.tool_build_rejected_receipts
            ));
        }
        if summary.reward_action != RewardAction::Reinforce {
            blocked_reasons.push(format!("reward_action={}", summary.reward_action.as_str()));
        }

        Self {
            memory_notes: report
                .memory_promotions
                .iter()
                .map(|promotion| promotion.note.clone())
                .collect(),
            follow_up_tasks: report.follow_up_tasks.clone(),
            blocked_reasons,
        }
    }

    pub fn can_submit_memory(&self) -> bool {
        !self.memory_notes.is_empty() && self.blocked_reasons.is_empty()
    }
}

impl AgentCycleSummary {
    pub fn from_report(report: &AgentCycleReport) -> Self {
        Self {
            assigned_tasks: report.dispatch.assignments.len(),
            rejected_tasks: report.dispatch.rejections.len(),
            unique_messages: report.run_report.aggregation.unique_count,
            duplicate_groups: report.run_report.aggregation.duplicate_groups,
            unresolved_conflicts: report.run_report.conflicts.unresolved_count(),
            blocked_side_effects: report
                .run_report
                .side_effects
                .iter()
                .filter(|gate| !gate.allowed)
                .count(),
            budget_overspends: report.run_report.budget_audit.overspend_count(),
            execution_failures: report.execution_failures.len(),
            reward_total: report.reward_report.total,
            reward_action: report.reward_report.action,
            evolution_signals: report.reward_report.evolution_signals.len(),
            follow_up_tasks: report.follow_up_tasks.len(),
            memory_promotions: report.memory_promotions.len(),
            tool_build_reports: usize::from(report.tool_build_report.is_some()),
            tool_build_missing_requests: report
                .tool_build_report
                .as_ref()
                .map_or(0, |summary| summary.missing_requests),
            tool_build_unexpected_receipts: report
                .tool_build_report
                .as_ref()
                .map_or(0, |summary| summary.unexpected_receipts),
            tool_build_duplicate_receipts: report
                .tool_build_report
                .as_ref()
                .map_or(0, |summary| summary.duplicate_receipts),
            tool_build_held_receipts: report
                .tool_build_report
                .as_ref()
                .map_or(0, |summary| summary.held),
            tool_build_rejected_receipts: report
                .tool_build_report
                .as_ref()
                .map_or(0, |summary| summary.rejected),
        }
    }

    pub fn ready_for_memory_promotion(&self) -> bool {
        self.reward_action == RewardAction::Reinforce
            && self.unresolved_conflicts == 0
            && self.blocked_side_effects == 0
            && self.budget_overspends == 0
            && self.execution_failures == 0
            && self.tool_build_repair_pressure() == 0
    }

    pub fn tool_build_repair_pressure(&self) -> usize {
        self.tool_build_missing_requests
            + self.tool_build_unexpected_receipts
            + self.tool_build_duplicate_receipts
            + self.tool_build_held_receipts
            + self.tool_build_rejected_receipts
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentCycleSummaryHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AgentCycleSummaryHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentCycleSummaryHistory {
    summaries: Vec<AgentCycleSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleSummaryDashboard {
    pub total_records: usize,
    pub assigned_tasks: usize,
    pub rejected_tasks: usize,
    pub unique_messages: usize,
    pub duplicate_groups: usize,
    pub unresolved_conflicts: usize,
    pub blocked_side_effects: usize,
    pub budget_overspends: usize,
    pub execution_failures: usize,
    pub reinforce_records: usize,
    pub hold_records: usize,
    pub penalize_records: usize,
    pub evolution_signals: usize,
    pub follow_up_tasks: usize,
    pub memory_promotions: usize,
    pub memory_ready_records: usize,
    pub low_reward_records: usize,
    pub tool_build_reports: usize,
    pub tool_build_missing_requests: usize,
    pub tool_build_unexpected_receipts: usize,
    pub tool_build_duplicate_receipts: usize,
    pub tool_build_held_receipts: usize,
    pub tool_build_rejected_receipts: usize,
    pub tool_build_repair_pressure: usize,
    pub clean_record_rate: f32,
    pub memory_promotion_rate: f32,
    pub average_reward_total: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentCycleSummaryHealthPolicy {
    pub maximum_rejected_tasks: usize,
    pub maximum_unresolved_conflicts: usize,
    pub maximum_blocked_side_effects: usize,
    pub maximum_budget_overspends: usize,
    pub maximum_execution_failures: usize,
    pub maximum_penalize_records: usize,
    pub maximum_low_reward_records: usize,
    pub maximum_tool_build_missing_requests: usize,
    pub maximum_tool_build_unexpected_receipts: usize,
    pub maximum_tool_build_duplicate_receipts: usize,
    pub maximum_tool_build_held_receipts: usize,
    pub maximum_tool_build_rejected_receipts: usize,
    pub minimum_clean_record_rate: f32,
    pub minimum_average_reward_total: f32,
}

impl Default for AgentCycleSummaryHealthPolicy {
    fn default() -> Self {
        Self {
            maximum_rejected_tasks: 0,
            maximum_unresolved_conflicts: 0,
            maximum_blocked_side_effects: 0,
            maximum_budget_overspends: 0,
            maximum_execution_failures: 0,
            maximum_penalize_records: 0,
            maximum_low_reward_records: 0,
            maximum_tool_build_missing_requests: 0,
            maximum_tool_build_unexpected_receipts: 0,
            maximum_tool_build_duplicate_receipts: 0,
            maximum_tool_build_held_receipts: 0,
            maximum_tool_build_rejected_receipts: 0,
            minimum_clean_record_rate: 1.0,
            minimum_average_reward_total: 0.42,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleSummaryHealth {
    pub status: AgentCycleSummaryHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentCycleSummaryDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleSummaryHistoryRecord {
    pub history: AgentCycleSummaryHistory,
    pub appended_summary: AgentCycleSummary,
    pub dashboard: AgentCycleSummaryDashboard,
    pub health: AgentCycleSummaryHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentCycleSummaryHistoryRecorder;

impl AgentCycleSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentCycleSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentCycleSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentCycleSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentCycleSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentCycleSummaryDashboard {
        AgentCycleSummaryDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AgentCycleSummaryHealthPolicy) -> AgentCycleSummaryHealth {
        self.dashboard().health(policy)
    }
}

impl AgentCycleSummaryDashboard {
    pub fn from_summaries(summaries: &[AgentCycleSummary]) -> Self {
        let total_records = summaries.len();
        let assigned_tasks = summaries
            .iter()
            .map(|summary| summary.assigned_tasks)
            .sum::<usize>();
        let rejected_tasks = summaries
            .iter()
            .map(|summary| summary.rejected_tasks)
            .sum::<usize>();
        let unique_messages = summaries
            .iter()
            .map(|summary| summary.unique_messages)
            .sum::<usize>();
        let duplicate_groups = summaries
            .iter()
            .map(|summary| summary.duplicate_groups)
            .sum::<usize>();
        let unresolved_conflicts = summaries
            .iter()
            .map(|summary| summary.unresolved_conflicts)
            .sum::<usize>();
        let blocked_side_effects = summaries
            .iter()
            .map(|summary| summary.blocked_side_effects)
            .sum::<usize>();
        let budget_overspends = summaries
            .iter()
            .map(|summary| summary.budget_overspends)
            .sum::<usize>();
        let execution_failures = summaries
            .iter()
            .map(|summary| summary.execution_failures)
            .sum::<usize>();
        let reinforce_records = summaries
            .iter()
            .filter(|summary| summary.reward_action == RewardAction::Reinforce)
            .count();
        let hold_records = summaries
            .iter()
            .filter(|summary| summary.reward_action == RewardAction::Hold)
            .count();
        let penalize_records = summaries
            .iter()
            .filter(|summary| summary.reward_action == RewardAction::Penalize)
            .count();
        let evolution_signals = summaries
            .iter()
            .map(|summary| summary.evolution_signals)
            .sum::<usize>();
        let follow_up_tasks = summaries
            .iter()
            .map(|summary| summary.follow_up_tasks)
            .sum::<usize>();
        let memory_promotions = summaries
            .iter()
            .map(|summary| summary.memory_promotions)
            .sum::<usize>();
        let memory_ready_records = summaries
            .iter()
            .filter(|summary| summary.ready_for_memory_promotion())
            .count();
        let low_reward_records = summaries
            .iter()
            .filter(|summary| summary.reward_total < 0.42)
            .count();
        let tool_build_reports = summaries
            .iter()
            .map(|summary| summary.tool_build_reports)
            .sum::<usize>();
        let tool_build_missing_requests = summaries
            .iter()
            .map(|summary| summary.tool_build_missing_requests)
            .sum::<usize>();
        let tool_build_unexpected_receipts = summaries
            .iter()
            .map(|summary| summary.tool_build_unexpected_receipts)
            .sum::<usize>();
        let tool_build_duplicate_receipts = summaries
            .iter()
            .map(|summary| summary.tool_build_duplicate_receipts)
            .sum::<usize>();
        let tool_build_held_receipts = summaries
            .iter()
            .map(|summary| summary.tool_build_held_receipts)
            .sum::<usize>();
        let tool_build_rejected_receipts = summaries
            .iter()
            .map(|summary| summary.tool_build_rejected_receipts)
            .sum::<usize>();
        let tool_build_repair_pressure = summaries
            .iter()
            .map(AgentCycleSummary::tool_build_repair_pressure)
            .sum::<usize>();
        let clean_records = summaries
            .iter()
            .filter(|summary| summary.ready_for_memory_promotion())
            .count();
        let reward_total_sum = summaries
            .iter()
            .map(|summary| summary.reward_total)
            .sum::<f32>();
        let average_reward_total = if total_records == 0 {
            0.0
        } else {
            reward_total_sum / total_records as f32
        };
        let clean_record_rate = rate(clean_records, total_records);
        let memory_promotion_rate = rate(memory_promotions, total_records);
        let telemetry = agent_cycle_summary_dashboard_telemetry(
            total_records,
            assigned_tasks,
            rejected_tasks,
            unresolved_conflicts,
            blocked_side_effects,
            budget_overspends,
            execution_failures,
            reinforce_records,
            hold_records,
            penalize_records,
            memory_promotions,
            tool_build_reports,
            tool_build_missing_requests,
            tool_build_unexpected_receipts,
            tool_build_duplicate_receipts,
            tool_build_held_receipts,
            tool_build_rejected_receipts,
            tool_build_repair_pressure,
            clean_record_rate,
            memory_promotion_rate,
            average_reward_total,
        );

        Self {
            total_records,
            assigned_tasks,
            rejected_tasks,
            unique_messages,
            duplicate_groups,
            unresolved_conflicts,
            blocked_side_effects,
            budget_overspends,
            execution_failures,
            reinforce_records,
            hold_records,
            penalize_records,
            evolution_signals,
            follow_up_tasks,
            memory_promotions,
            memory_ready_records,
            low_reward_records,
            tool_build_reports,
            tool_build_missing_requests,
            tool_build_unexpected_receipts,
            tool_build_duplicate_receipts,
            tool_build_held_receipts,
            tool_build_rejected_receipts,
            tool_build_repair_pressure,
            clean_record_rate,
            memory_promotion_rate,
            average_reward_total,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: AgentCycleSummaryHealthPolicy) -> AgentCycleSummaryHealth {
        AgentCycleSummaryHealth::from_dashboard(self.clone(), policy)
    }
}

impl AgentCycleSummaryHealth {
    pub fn from_dashboard(
        dashboard: AgentCycleSummaryDashboard,
        policy: AgentCycleSummaryHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("agent_cycle_summary_history_empty".to_owned());
        } else if dashboard.clean_record_rate < policy.minimum_clean_record_rate {
            watch_reasons.push(format!(
                "agent_cycle_summary_clean_record_rate={:.3}<{}",
                dashboard.clean_record_rate, policy.minimum_clean_record_rate
            ));
        }

        if !dashboard.is_empty()
            && dashboard.average_reward_total < policy.minimum_average_reward_total
        {
            watch_reasons.push(format!(
                "agent_cycle_summary_average_reward_total={:.3}<{}",
                dashboard.average_reward_total, policy.minimum_average_reward_total
            ));
        }

        if dashboard.rejected_tasks > policy.maximum_rejected_tasks {
            repair_reasons.push(format!(
                "agent_cycle_summary_rejected_tasks={}>{}",
                dashboard.rejected_tasks, policy.maximum_rejected_tasks
            ));
        }

        if dashboard.unresolved_conflicts > policy.maximum_unresolved_conflicts {
            repair_reasons.push(format!(
                "agent_cycle_summary_unresolved_conflicts={}>{}",
                dashboard.unresolved_conflicts, policy.maximum_unresolved_conflicts
            ));
        }

        if dashboard.blocked_side_effects > policy.maximum_blocked_side_effects {
            repair_reasons.push(format!(
                "agent_cycle_summary_blocked_side_effects={}>{}",
                dashboard.blocked_side_effects, policy.maximum_blocked_side_effects
            ));
        }

        if dashboard.budget_overspends > policy.maximum_budget_overspends {
            repair_reasons.push(format!(
                "agent_cycle_summary_budget_overspends={}>{}",
                dashboard.budget_overspends, policy.maximum_budget_overspends
            ));
        }

        if dashboard.execution_failures > policy.maximum_execution_failures {
            repair_reasons.push(format!(
                "agent_cycle_summary_execution_failures={}>{}",
                dashboard.execution_failures, policy.maximum_execution_failures
            ));
        }

        if dashboard.penalize_records > policy.maximum_penalize_records {
            repair_reasons.push(format!(
                "agent_cycle_summary_penalize_records={}>{}",
                dashboard.penalize_records, policy.maximum_penalize_records
            ));
        }

        if dashboard.low_reward_records > policy.maximum_low_reward_records {
            repair_reasons.push(format!(
                "agent_cycle_summary_low_reward_records={}>{}",
                dashboard.low_reward_records, policy.maximum_low_reward_records
            ));
        }

        if dashboard.tool_build_missing_requests > policy.maximum_tool_build_missing_requests {
            repair_reasons.push(format!(
                "agent_cycle_summary_tool_build_missing_requests={}>{}",
                dashboard.tool_build_missing_requests, policy.maximum_tool_build_missing_requests
            ));
        }

        if dashboard.tool_build_unexpected_receipts > policy.maximum_tool_build_unexpected_receipts
        {
            repair_reasons.push(format!(
                "agent_cycle_summary_tool_build_unexpected_receipts={}>{}",
                dashboard.tool_build_unexpected_receipts,
                policy.maximum_tool_build_unexpected_receipts
            ));
        }

        if dashboard.tool_build_duplicate_receipts > policy.maximum_tool_build_duplicate_receipts {
            repair_reasons.push(format!(
                "agent_cycle_summary_tool_build_duplicate_receipts={}>{}",
                dashboard.tool_build_duplicate_receipts,
                policy.maximum_tool_build_duplicate_receipts
            ));
        }

        if dashboard.tool_build_held_receipts > policy.maximum_tool_build_held_receipts {
            repair_reasons.push(format!(
                "agent_cycle_summary_tool_build_held_receipts={}>{}",
                dashboard.tool_build_held_receipts, policy.maximum_tool_build_held_receipts
            ));
        }

        if dashboard.tool_build_rejected_receipts > policy.maximum_tool_build_rejected_receipts {
            repair_reasons.push(format!(
                "agent_cycle_summary_tool_build_rejected_receipts={}>{}",
                dashboard.tool_build_rejected_receipts, policy.maximum_tool_build_rejected_receipts
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentCycleSummaryHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentCycleSummaryHealthStatus::Watch, watch_reasons)
        } else {
            (AgentCycleSummaryHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentCycleSummaryHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentCycleSummaryHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentCycleSummaryHealthStatus::Repair
    }
}

impl AgentCycleSummaryHistoryRecord {
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

impl AgentCycleSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentCycleSummaryHistory,
        summary: AgentCycleSummary,
        policy: AgentCycleSummaryHealthPolicy,
    ) -> AgentCycleSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = agent_cycle_summary_history_record_telemetry(&dashboard, &health);

        AgentCycleSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_report_with_health(
        &self,
        history: AgentCycleSummaryHistory,
        report: &AgentCycleReport,
        policy: AgentCycleSummaryHealthPolicy,
    ) -> AgentCycleSummaryHistoryRecord {
        self.record_summary_with_health(history, AgentCycleSummary::from_report(report), policy)
    }
}

#[derive(Debug, Clone)]
pub struct AgentCycleOrchestrator {
    rewarder: ClosedLoopRewarder,
}

impl Default for AgentCycleOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentCycleOrchestrator {
    pub fn new() -> Self {
        Self {
            rewarder: ClosedLoopRewarder::new(),
        }
    }

    pub fn with_rewarder(rewarder: ClosedLoopRewarder) -> Self {
        Self { rewarder }
    }

    pub fn plan_next_wave(
        &self,
        mut queue: AgentTaskQueue,
        completed_task_ids: &BTreeSet<String>,
        ledger: BudgetLedger,
        policy: &BudgetPolicy,
        max_parallel_tasks: usize,
    ) -> AgentCycleDispatch {
        let ready = queue
            .drain_ready(completed_task_ids)
            .into_iter()
            .take(max_parallel_tasks.max(1))
            .collect::<Vec<_>>();
        let blocked_task_ids = queue
            .blocked_tasks(completed_task_ids)
            .into_iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let task_ids = ready.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
        let ready_by_id = ready
            .iter()
            .map(|task| (task.id.clone(), task.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut planner = DispatchPlanner::new(ledger);
        let dispatch = planner.plan_with_policy(ready, policy);
        let assigned_tasks = dispatch
            .assignments
            .iter()
            .filter_map(|assignment| ready_by_id.get(&assignment.task_id).cloned())
            .collect::<Vec<_>>();

        AgentCycleDispatch {
            wave: AgentExecutionWave {
                wave: completed_task_ids.len(),
                parallel_count: task_ids.len(),
                task_ids,
            },
            dispatch,
            assigned_tasks,
            blocked_task_ids,
            remaining_queue: queue,
        }
    }

    pub fn close_wave(
        &self,
        dispatch: TaskDispatchPlan,
        results: Vec<AgentResult>,
        evidence: AgentCycleEvidence,
    ) -> AgentCycleReport {
        self.close_wave_with_failures(dispatch, results, Vec::new(), evidence)
    }

    pub fn close_execution(
        &self,
        dispatch: TaskDispatchPlan,
        execution: AgentWaveExecution,
        evidence: AgentCycleEvidence,
    ) -> AgentCycleReport {
        self.close_wave_with_failures(dispatch, execution.results, execution.failures, evidence)
    }

    fn close_wave_with_failures(
        &self,
        dispatch: TaskDispatchPlan,
        results: Vec<AgentResult>,
        execution_failures: Vec<AgentExecutionFailure>,
        evidence: AgentCycleEvidence,
    ) -> AgentCycleReport {
        let run_ledger_admission = AgentRunLedger::admission(&dispatch.gate());
        let run_report = if let Ok(mut ledger) = AgentRunLedger::try_from_dispatch(dispatch.clone())
        {
            for result in results {
                ledger.record_result(result);
            }
            ledger.report_with_resolutions(
                evidence.reflection.as_ref(),
                &evidence.conflict_resolutions,
            )
        } else {
            closed_run_report(&run_ledger_admission)
        };
        let tool_build_report = evidence
            .tool_build_report
            .as_ref()
            .map(ToolBuildReportSummary::from_report);
        let reward_report = self.rewarder.score(ProcessRewardInput {
            quality: evidence.quality,
            validation_passed: evidence.validation_passed,
            runtime_response_ok: evidence.runtime_response_ok && execution_failures.is_empty(),
            execution_failures: execution_failures.len(),
            reflection_complete: evidence.reflection_complete(),
            recursive_chunks: dispatch.assignments.len().max(1),
            recursive_waves: 1,
            run_report: run_report.clone(),
            toolsmith_plan: evidence.toolsmith_plan,
            tool_build_report: evidence.tool_build_report,
        });
        let memory_promotions =
            memory_promotions(&run_report, &reward_report, evidence.reflection.as_ref());
        let follow_up_tasks = follow_up_tasks(&reward_report);

        AgentCycleReport {
            dispatch,
            execution_failures,
            run_ledger_admission,
            run_report,
            reward_report,
            tool_build_report,
            follow_up_tasks,
            memory_promotions,
        }
    }
}

fn closed_run_report(admission: &AgentRunLedgerAdmission) -> AgentRunReport {
    let reason = admission
        .reasons
        .first()
        .cloned()
        .unwrap_or_else(|| "run_ledger_dispatch_closed".to_owned());
    let blocked_reason = format!("run_ledger_closed:{reason}");

    AgentRunReport {
        aggregation: AggregationReport::default(),
        conflicts: ConflictReport::default(),
        budget_audit: RunBudgetAudit::default(),
        side_effects: vec![
            SideEffectGate::block(SideEffectKind::MemoryNote, blocked_reason.clone()),
            SideEffectGate::block(SideEffectKind::FileWrite, blocked_reason.clone()),
            SideEffectGate::block(SideEffectKind::AdaptiveStateWrite, blocked_reason.clone()),
            SideEffectGate::block(SideEffectKind::ExternalCall, blocked_reason),
        ],
    }
}

fn memory_promotions(
    run_report: &AgentRunReport,
    reward_report: &ProcessRewardReport,
    reflection: Option<&ReflectionLoop>,
) -> Vec<MemoryPromotion> {
    let memory_gate_allowed = run_report
        .side_effects
        .iter()
        .any(|gate| gate.kind == SideEffectKind::MemoryNote && gate.allowed);
    if reward_report.action != RewardAction::Reinforce
        || !memory_gate_allowed
        || run_report.conflicts.has_unresolved_conflicts()
        || run_report.budget_audit.has_overspends()
    {
        return Vec::new();
    }

    let Some(memory_note) = reflection.and_then(ReflectionLoop::memory_note) else {
        return Vec::new();
    };

    vec![MemoryPromotion {
        note: MemoryNote::new("agent_cycle", memory_note)
            .with_evidence(format!("reward_total={:.3}", reward_report.total))
            .with_evidence(format!(
                "unique_messages={}",
                run_report.aggregation.unique_count
            )),
        reason: "reinforced cycle with memory side-effect gate allowed".to_owned(),
    }]
}

fn follow_up_tasks(reward_report: &ProcessRewardReport) -> Vec<AgentTask> {
    match reward_report.action {
        RewardAction::Reinforce => reward_report
            .evolution_signals
            .iter()
            .enumerate()
            .map(|(index, signal)| {
                AgentTask::new(
                    format!("evolution-reinforce-{index}"),
                    AgentRole::MemoryCurator,
                    format!(
                        "promote {} via {} because {}",
                        signal.target, signal.action, signal.reason
                    ),
                    crate::budget::AgentBudget::new(16, 1, 1),
                )
                .with_lane("evolution")
                .with_priority(3)
            })
            .collect(),
        RewardAction::Hold => vec![
            AgentTask::new(
                "evolution-hold-evidence",
                AgentRole::Tester,
                "collect more validation evidence before promotion",
                crate::budget::AgentBudget::new(24, 2, 1),
            )
            .with_lane("validation")
            .with_priority(6),
        ],
        RewardAction::Penalize => vec![
            AgentTask::new(
                "evolution-repair-loop",
                AgentRole::Reviewer,
                "repair blocked agent loop before any memory or adaptive-state promotion",
                crate::budget::AgentBudget::new(32, 2, 1),
            )
            .with_lane("repair")
            .with_priority(8),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn agent_cycle_summary_dashboard_telemetry(
    total_records: usize,
    assigned_tasks: usize,
    rejected_tasks: usize,
    unresolved_conflicts: usize,
    blocked_side_effects: usize,
    budget_overspends: usize,
    execution_failures: usize,
    reinforce_records: usize,
    hold_records: usize,
    penalize_records: usize,
    memory_promotions: usize,
    tool_build_reports: usize,
    tool_build_missing_requests: usize,
    tool_build_unexpected_receipts: usize,
    tool_build_duplicate_receipts: usize,
    tool_build_held_receipts: usize,
    tool_build_rejected_receipts: usize,
    tool_build_repair_pressure: usize,
    clean_record_rate: f32,
    memory_promotion_rate: f32,
    average_reward_total: f32,
) -> Vec<String> {
    vec![
        "agent_cycle_summary_dashboard=true".to_owned(),
        format!("agent_cycle_summary_dashboard_records={total_records}"),
        format!("agent_cycle_summary_dashboard_assigned_tasks={assigned_tasks}"),
        format!("agent_cycle_summary_dashboard_rejected_tasks={rejected_tasks}"),
        format!("agent_cycle_summary_dashboard_unresolved_conflicts={unresolved_conflicts}"),
        format!("agent_cycle_summary_dashboard_blocked_side_effects={blocked_side_effects}"),
        format!("agent_cycle_summary_dashboard_budget_overspends={budget_overspends}"),
        format!("agent_cycle_summary_dashboard_execution_failures={execution_failures}"),
        format!("agent_cycle_summary_dashboard_reinforce_records={reinforce_records}"),
        format!("agent_cycle_summary_dashboard_hold_records={hold_records}"),
        format!("agent_cycle_summary_dashboard_penalize_records={penalize_records}"),
        format!("agent_cycle_summary_dashboard_memory_promotions={memory_promotions}"),
        format!("agent_cycle_summary_dashboard_tool_build_reports={tool_build_reports}"),
        format!(
            "agent_cycle_summary_dashboard_tool_build_missing_requests={tool_build_missing_requests}"
        ),
        format!(
            "agent_cycle_summary_dashboard_tool_build_unexpected_receipts={tool_build_unexpected_receipts}"
        ),
        format!(
            "agent_cycle_summary_dashboard_tool_build_duplicate_receipts={tool_build_duplicate_receipts}"
        ),
        format!(
            "agent_cycle_summary_dashboard_tool_build_held_receipts={tool_build_held_receipts}"
        ),
        format!(
            "agent_cycle_summary_dashboard_tool_build_rejected_receipts={tool_build_rejected_receipts}"
        ),
        format!(
            "agent_cycle_summary_dashboard_tool_build_repair_pressure={tool_build_repair_pressure}"
        ),
        format!("agent_cycle_summary_dashboard_clean_record_rate={clean_record_rate:.3}"),
        format!("agent_cycle_summary_dashboard_memory_promotion_rate={memory_promotion_rate:.3}"),
        format!("agent_cycle_summary_dashboard_average_reward_total={average_reward_total:.3}"),
    ]
}

fn agent_cycle_summary_history_record_telemetry(
    dashboard: &AgentCycleSummaryDashboard,
    health: &AgentCycleSummaryHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_cycle_summary_history_record=true".to_owned(),
        format!(
            "agent_cycle_summary_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_cycle_summary_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_cycle_summary_history_record_clean_record_rate={:.3}",
            dashboard.clean_record_rate
        ),
        format!(
            "agent_cycle_summary_history_record_average_reward_total={:.3}",
            dashboard.average_reward_total
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_cycle_summary_history_record_reason={reason}")),
    );
    telemetry
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
    use crate::budget::{AgentBudget, BudgetLedger, BudgetPolicy};
    use crate::conflict::ConflictResolution;
    use crate::message::{AgentMessage, AgentMessageKind};
    use crate::reflection::{ReflectionLoop, ReflectionStage};
    use crate::task::DispatchPlanner;

    fn clean_cycle_summary() -> AgentCycleSummary {
        AgentCycleSummary {
            assigned_tasks: 1,
            rejected_tasks: 0,
            unique_messages: 2,
            duplicate_groups: 0,
            unresolved_conflicts: 0,
            blocked_side_effects: 0,
            budget_overspends: 0,
            execution_failures: 0,
            reward_total: 0.86,
            reward_action: RewardAction::Reinforce,
            evolution_signals: 1,
            follow_up_tasks: 1,
            memory_promotions: 1,
            tool_build_reports: 0,
            tool_build_missing_requests: 0,
            tool_build_unexpected_receipts: 0,
            tool_build_duplicate_receipts: 0,
            tool_build_held_receipts: 0,
            tool_build_rejected_receipts: 0,
        }
    }

    #[test]
    fn cycle_summary_history_watches_empty() {
        let health =
            AgentCycleSummaryHistory::new().health(AgentCycleSummaryHealthPolicy::default());

        assert_eq!(health.status, AgentCycleSummaryHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_cycle_summary_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| line == "agent_cycle_summary_dashboard_records=0")
        );
    }

    #[test]
    fn cycle_summary_history_marks_clean_reinforced_cycle_stable() {
        let record = AgentCycleSummaryHistoryRecorder::new().record_summary_with_health(
            AgentCycleSummaryHistory::new(),
            clean_cycle_summary(),
            AgentCycleSummaryHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert_eq!(
            record.appended_summary.reward_action,
            RewardAction::Reinforce
        );
        assert_eq!(record.dashboard.assigned_tasks, 1);
        assert_eq!(record.dashboard.memory_promotions, 1);
        assert_eq!(record.dashboard.clean_record_rate, 1.0);
        assert_eq!(record.dashboard.memory_promotion_rate, 1.0);
        assert_eq!(record.health.status, AgentCycleSummaryHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.health.allows_service_advance());
        assert!(!record.health.requires_repair_first());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "agent_cycle_summary_history_record_status=stable")
        );
    }

    #[test]
    fn cycle_summary_history_repairs_dirty_cycle_pressure() {
        let dirty = AgentCycleSummary {
            assigned_tasks: 1,
            rejected_tasks: 1,
            unique_messages: 1,
            duplicate_groups: 1,
            unresolved_conflicts: 1,
            blocked_side_effects: 2,
            budget_overspends: 1,
            execution_failures: 1,
            reward_total: 0.30,
            reward_action: RewardAction::Penalize,
            evolution_signals: 1,
            follow_up_tasks: 1,
            memory_promotions: 0,
            tool_build_reports: 1,
            tool_build_missing_requests: 1,
            tool_build_unexpected_receipts: 1,
            tool_build_duplicate_receipts: 1,
            tool_build_held_receipts: 1,
            tool_build_rejected_receipts: 1,
        };
        let history = AgentCycleSummaryHistory::from_summaries(vec![clean_cycle_summary()]);

        let record = AgentCycleSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            dirty,
            AgentCycleSummaryHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.rejected_tasks, 1);
        assert_eq!(record.dashboard.unresolved_conflicts, 1);
        assert_eq!(record.dashboard.blocked_side_effects, 2);
        assert_eq!(record.dashboard.budget_overspends, 1);
        assert_eq!(record.dashboard.execution_failures, 1);
        assert_eq!(record.dashboard.penalize_records, 1);
        assert_eq!(record.dashboard.low_reward_records, 1);
        assert_eq!(record.dashboard.tool_build_reports, 1);
        assert_eq!(record.dashboard.tool_build_missing_requests, 1);
        assert_eq!(record.dashboard.tool_build_unexpected_receipts, 1);
        assert_eq!(record.dashboard.tool_build_duplicate_receipts, 1);
        assert_eq!(record.dashboard.tool_build_held_receipts, 1);
        assert_eq!(record.dashboard.tool_build_rejected_receipts, 1);
        assert_eq!(record.dashboard.tool_build_repair_pressure, 5);
        assert_eq!(record.dashboard.clean_record_rate, 0.5);
        assert_eq!(record.health.status, AgentCycleSummaryHealthStatus::Repair);
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "agent_cycle_summary_rejected_tasks=1>0".to_owned(),
                "agent_cycle_summary_unresolved_conflicts=1>0".to_owned(),
                "agent_cycle_summary_blocked_side_effects=2>0".to_owned(),
                "agent_cycle_summary_budget_overspends=1>0".to_owned(),
                "agent_cycle_summary_execution_failures=1>0".to_owned(),
                "agent_cycle_summary_penalize_records=1>0".to_owned(),
                "agent_cycle_summary_low_reward_records=1>0".to_owned(),
                "agent_cycle_summary_tool_build_missing_requests=1>0".to_owned(),
                "agent_cycle_summary_tool_build_unexpected_receipts=1>0".to_owned(),
                "agent_cycle_summary_tool_build_duplicate_receipts=1>0".to_owned(),
                "agent_cycle_summary_tool_build_held_receipts=1>0".to_owned(),
                "agent_cycle_summary_tool_build_rejected_receipts=1>0".to_owned(),
                "agent_cycle_summary_clean_record_rate=0.500<1".to_owned(),
            ]
        );
    }

    #[test]
    fn cycle_plans_next_ready_wave_with_budget_policy() {
        let queue = AgentTaskQueue::from_tasks(vec![
            AgentTask::new(
                "planner",
                AgentRole::Planner,
                "split work",
                AgentBudget::new(4, 1, 1),
            ),
            AgentTask::new(
                "memory",
                AgentRole::MemoryCurator,
                "capture lesson",
                AgentBudget::new(4, 1, 1),
            )
            .depends_on("reviewer"),
        ]);
        let ledger = BudgetLedger::new()
            .with_budget(AgentRole::Planner, AgentBudget::new(8, 2, 2))
            .with_budget(AgentRole::MemoryCurator, AgentBudget::new(8, 2, 2));

        let dispatch = AgentCycleOrchestrator::new().plan_next_wave(
            queue,
            &BTreeSet::new(),
            ledger,
            &BudgetPolicy::strict(),
            2,
        );

        assert_eq!(dispatch.wave.task_ids, vec!["planner"]);
        assert_eq!(dispatch.dispatch.assignments.len(), 1);
        assert_eq!(dispatch.assigned_tasks.len(), 1);
        assert_eq!(dispatch.assigned_tasks[0].id, "planner");
        assert_eq!(dispatch.blocked_task_ids, vec!["memory"]);
        assert_eq!(dispatch.remaining_queue.len(), 1);
    }

    #[test]
    fn cycle_close_wave_records_closed_run_ledger_admission_for_budget_rejection() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Reviewer, AgentBudget::new(4, 1, 1)),
        );
        let oversized = AgentTask::new(
            "oversized-review",
            AgentRole::Reviewer,
            "review request larger than the isolated budget",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = planner.plan_with_policy(vec![oversized], &BudgetPolicy::strict());

        let report = AgentCycleOrchestrator::new().close_wave(
            dispatch,
            Vec::new(),
            AgentCycleEvidence {
                quality: 0.95,
                validation_passed: true,
                runtime_response_ok: true,
                ..AgentCycleEvidence::default()
            },
        );
        let handoff = AgentCycleHandoff::from_report(&report);

        assert!(report.run_ledger_admission.requires_repair_first);
        assert!(!report.run_ledger_admission.can_build_ledger);
        assert!(!report.run_ledger_admission.can_submit_memory_note);
        assert_eq!(report.run_report.summary().blocked_side_effects, 4);
        assert!(report.memory_promotions.is_empty());
        assert_ne!(report.reward_report.action, RewardAction::Reinforce);
        assert!(!handoff.can_submit_memory());
        assert!(handoff.blocked_reasons.iter().any(|reason| {
            reason.starts_with("run_ledger_admission:dispatch_rejection task=oversized-review")
        }));
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "blocked_side_effects=4")
        );
    }

    #[test]
    fn cycle_closes_clean_wave_into_reinforcement_follow_up() {
        let task = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "draft patch",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![crate::task::TaskAssignment {
                task_id: task.id.clone(),
                role: task.role.clone(),
                lane: task.lane.clone(),
                budget_reserved: task.required_budget,
            }],
            ..TaskDispatchPlan::default()
        };
        let result = AgentResult::accepted(
            &task,
            "done",
            vec![AgentMessage::new(
                "decision",
                AgentRole::Coder,
                AgentMessageKind::Decision,
                "patch",
                "approve patch and proceed",
            )],
            AgentBudget::new(4, 1, 1),
        );
        let mut reflection = ReflectionLoop::new();
        reflection
            .submit(ReflectionStage::Draft, "draft accepted")
            .unwrap();
        reflection
            .submit(ReflectionStage::Critique, "no blocker")
            .unwrap();
        reflection
            .submit(ReflectionStage::Revision, "keep evidence")
            .unwrap();
        reflection
            .submit(ReflectionStage::MemoryNote, "remember clean loop")
            .unwrap();

        let report = AgentCycleOrchestrator::new().close_wave(
            dispatch,
            vec![result],
            AgentCycleEvidence {
                quality: 0.94,
                validation_passed: true,
                runtime_response_ok: true,
                reflection: Some(reflection),
                conflict_resolutions: ConflictResolutionBook::new(),
                toolsmith_plan: ToolsmithPlan::new(),
                tool_build_report: None,
            },
        );

        assert_eq!(report.reward_report.action, RewardAction::Reinforce);
        assert!(!report.follow_up_tasks.is_empty());
        assert_eq!(report.follow_up_tasks[0].role, AgentRole::MemoryCurator);
        assert_eq!(report.memory_promotions.len(), 1);
        assert_eq!(report.memory_promotions[0].note.topic, "agent_cycle");
        assert_eq!(
            report.memory_promotions[0].note.content,
            "remember clean loop"
        );

        let summary = AgentCycleSummary::from_report(&report);
        assert_eq!(summary.assigned_tasks, 1);
        assert_eq!(summary.reward_action, RewardAction::Reinforce);
        assert_eq!(summary.memory_promotions, 1);
        assert!(summary.ready_for_memory_promotion());

        let handoff = AgentCycleHandoff::from_report(&report);
        assert!(handoff.can_submit_memory());
        assert_eq!(handoff.memory_notes.len(), 1);
        assert!(handoff.blocked_reasons.is_empty());
    }

    #[test]
    fn cycle_evidence_tool_build_report_reaches_process_reward() {
        let task = AgentTask::new(
            "toolsmith",
            AgentRole::Planner,
            "close tool build receipts",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![crate::task::TaskAssignment {
                task_id: task.id.clone(),
                role: task.role.clone(),
                lane: task.lane.clone(),
                budget_reserved: task.required_budget,
            }],
            ..TaskDispatchPlan::default()
        };
        let result = AgentResult::accepted(
            &task,
            "tool build closed",
            vec![AgentMessage::new(
                "tool-build",
                AgentRole::Planner,
                AgentMessageKind::Decision,
                "toolsmith",
                "close tool build receipts before reinforcement",
            )],
            AgentBudget::new(4, 1, 1),
        );
        let build_requests = vec![crate::ports::ToolBuildRequest {
            proposal_id: "runtime-gate".to_owned(),
            intent: crate::ToolIntent::BenchmarkGate,
            rust_crate: "rust".to_owned(),
            entrypoint: "tools/runtime_gate.rs".to_owned(),
            gate_notes: Vec::new(),
        }];
        let tool_build_report = ToolBuildReport::from_requests_and_receipts(
            &build_requests,
            &[crate::ports::ToolBuildReceipt::held(
                "runtime-gate",
                "adapter timeout",
            )],
        );
        let toolsmith_plan = ToolsmithPlan::new().with_proposal(crate::ToolProposal::new(
            "runtime-gate",
            crate::ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            crate::ToolBuildStatus::Ready,
        ));

        let report = AgentCycleOrchestrator::new().close_wave(
            dispatch,
            vec![result],
            AgentCycleEvidence {
                quality: 0.94,
                validation_passed: true,
                runtime_response_ok: true,
                reflection: None,
                conflict_resolutions: ConflictResolutionBook::new(),
                toolsmith_plan,
                tool_build_report: Some(tool_build_report),
            },
        );

        assert_eq!(report.reward_report.components.toolsmith, 0.18);
        assert!(report.reward_report.notes.iter().any(|note| {
            note == "tool_build:repair_first missing=0 unexpected=0 duplicate=0 held=1 rejected=0"
        }));
        assert_eq!(
            report
                .tool_build_report
                .as_ref()
                .map(|summary| summary.held),
            Some(1)
        );

        let summary = AgentCycleSummary::from_report(&report);
        assert_eq!(summary.tool_build_reports, 1);
        assert_eq!(summary.tool_build_held_receipts, 1);
        assert_eq!(summary.tool_build_repair_pressure(), 1);
        assert!(!summary.ready_for_memory_promotion());

        let record = AgentCycleSummaryHistoryRecorder::new().record_report_with_health(
            AgentCycleSummaryHistory::new(),
            &report,
            AgentCycleSummaryHealthPolicy::default(),
        );
        assert_eq!(record.dashboard.tool_build_reports, 1);
        assert_eq!(record.dashboard.tool_build_held_receipts, 1);
        assert_eq!(record.dashboard.tool_build_repair_pressure, 1);
        assert_eq!(record.health.status, AgentCycleSummaryHealthStatus::Repair);
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "agent_cycle_summary_tool_build_held_receipts=1>0" })
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_cycle_summary_history_record_reason=agent_cycle_summary_tool_build_held_receipts=1>0"
        }));

        let handoff = AgentCycleHandoff::from_report(&report);
        assert!(!handoff.can_submit_memory());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "tool_build_held_receipts=1")
        );
    }

    #[test]
    fn cycle_turns_blocked_wave_into_repair_follow_up() {
        let task = AgentTask::new(
            "review",
            AgentRole::Reviewer,
            "review conflict",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![crate::task::TaskAssignment {
                task_id: task.id.clone(),
                role: task.role.clone(),
                lane: task.lane.clone(),
                budget_reserved: task.required_budget,
            }],
            ..TaskDispatchPlan::default()
        };
        let result = AgentResult::accepted(
            &task,
            "blocked",
            vec![
                AgentMessage::new(
                    "approve",
                    AgentRole::Coder,
                    AgentMessageKind::Decision,
                    "memory",
                    "approve memory note",
                ),
                AgentMessage::new(
                    "block",
                    AgentRole::Reviewer,
                    AgentMessageKind::Risk,
                    "memory",
                    "block memory note until validation passes",
                ),
            ],
            AgentBudget::new(4, 1, 1),
        );

        let report = AgentCycleOrchestrator::new().close_wave(
            dispatch,
            vec![result],
            AgentCycleEvidence {
                quality: 0.40,
                validation_passed: false,
                runtime_response_ok: true,
                reflection: None,
                conflict_resolutions: ConflictResolutionBook::new(),
                toolsmith_plan: ToolsmithPlan::new(),
                tool_build_report: None,
            },
        );

        assert_eq!(report.reward_report.action, RewardAction::Penalize);
        assert_eq!(report.follow_up_tasks.len(), 1);
        assert_eq!(report.follow_up_tasks[0].role, AgentRole::Reviewer);
        assert!(report.run_report.conflicts.has_unresolved_conflicts());
        assert!(report.memory_promotions.is_empty());

        let summary = AgentCycleSummary::from_report(&report);
        assert_eq!(summary.unresolved_conflicts, 1);
        assert_eq!(summary.reward_action, RewardAction::Penalize);
        assert!(!summary.ready_for_memory_promotion());

        let handoff = AgentCycleHandoff::from_report(&report);
        assert!(!handoff.can_submit_memory());
        assert!(handoff.memory_notes.is_empty());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "unresolved_conflicts=1")
        );
        assert_eq!(handoff.follow_up_tasks.len(), 1);
    }

    #[test]
    fn cycle_blocks_reinforcement_follow_up_when_conflict_is_unresolved() {
        let task = AgentTask::new(
            "memory-review",
            AgentRole::Reviewer,
            "review memory promotion",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![crate::task::TaskAssignment {
                task_id: task.id.clone(),
                role: task.role.clone(),
                lane: task.lane.clone(),
                budget_reserved: task.required_budget,
            }],
            ..TaskDispatchPlan::default()
        };
        let result = AgentResult::accepted(
            &task,
            "conflict remains",
            vec![
                AgentMessage::new(
                    "approve",
                    AgentRole::Coder,
                    AgentMessageKind::Decision,
                    "memory",
                    "approve memory note promotion",
                ),
                AgentMessage::new(
                    "block",
                    AgentRole::Reviewer,
                    AgentMessageKind::Risk,
                    "memory",
                    "block memory note promotion until validation passes",
                ),
            ],
            AgentBudget::new(4, 1, 1),
        );
        let mut reflection = ReflectionLoop::new();
        reflection
            .submit(ReflectionStage::Draft, "memory promotion looked useful")
            .unwrap();
        reflection
            .submit(ReflectionStage::Critique, "reviewer conflict remains")
            .unwrap();
        reflection
            .submit(ReflectionStage::Revision, "hold until resolved")
            .unwrap();
        reflection
            .submit(
                ReflectionStage::MemoryNote,
                "remember only after conflict repair",
            )
            .unwrap();

        let report = AgentCycleOrchestrator::new().close_wave(
            dispatch,
            vec![result],
            AgentCycleEvidence {
                quality: 0.99,
                validation_passed: true,
                runtime_response_ok: true,
                reflection: Some(reflection),
                conflict_resolutions: ConflictResolutionBook::new(),
                toolsmith_plan: ToolsmithPlan::new(),
                tool_build_report: None,
            },
        );

        assert_eq!(report.reward_report.action, RewardAction::Hold);
        assert!(report.run_report.conflicts.has_unresolved_conflicts());
        let run_summary = report.run_report.summary();
        assert_eq!(run_summary.unresolved_conflicts, 1);
        assert_eq!(run_summary.allowed_side_effects, 0);
        assert_eq!(run_summary.blocked_side_effects, 4);
        assert!(!run_summary.memory_note_allowed);
        assert!(!run_summary.adaptive_state_allowed);
        assert!(!run_summary.external_call_allowed);
        assert!(report.memory_promotions.is_empty());
        assert_eq!(report.follow_up_tasks.len(), 1);
        assert_eq!(report.follow_up_tasks[0].id, "evolution-hold-evidence");
        assert!(
            report
                .follow_up_tasks
                .iter()
                .all(|task| !task.id.starts_with("evolution-reinforce-"))
        );

        let summary = AgentCycleSummary::from_report(&report);
        assert_eq!(summary.unresolved_conflicts, 1);
        assert_eq!(summary.blocked_side_effects, 4);
        assert_eq!(summary.memory_promotions, 0);
        assert_eq!(summary.reward_action, RewardAction::Hold);
        assert!(!summary.ready_for_memory_promotion());

        let handoff = AgentCycleHandoff::from_report(&report);
        assert!(!handoff.can_submit_memory());
        assert!(handoff.memory_notes.is_empty());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "unresolved_conflicts=1" })
        );
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "blocked_side_effects=4" })
        );
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "reward_action=hold" })
        );
    }

    #[test]
    fn cycle_uses_conflict_resolutions_before_rewarding() {
        let task = AgentTask::new(
            "memory-review",
            AgentRole::Reviewer,
            "review memory decision",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![crate::task::TaskAssignment {
                task_id: task.id.clone(),
                role: task.role.clone(),
                lane: task.lane.clone(),
                budget_reserved: task.required_budget,
            }],
            ..TaskDispatchPlan::default()
        };
        let result = AgentResult::accepted(
            &task,
            "resolved",
            vec![
                AgentMessage::new(
                    "approve",
                    AgentRole::Coder,
                    AgentMessageKind::Decision,
                    "memory",
                    "approve memory note",
                ),
                AgentMessage::new(
                    "block",
                    AgentRole::Reviewer,
                    AgentMessageKind::Risk,
                    "memory",
                    "block memory note until validation passes",
                ),
            ],
            AgentBudget::new(4, 1, 1),
        );
        let mut reflection = ReflectionLoop::new();
        reflection
            .submit(ReflectionStage::Draft, "draft conclusion")
            .unwrap();
        reflection
            .submit(ReflectionStage::Critique, "validation required")
            .unwrap();
        reflection
            .submit(ReflectionStage::Revision, "validation passed")
            .unwrap();
        reflection
            .submit(ReflectionStage::MemoryNote, "remember validated note")
            .unwrap();
        let conflict_resolutions =
            ConflictResolutionBook::new().with_resolution(ConflictResolution::new(
                "memory",
                vec!["approve".to_owned(), "block".to_owned()],
                AgentRole::Planner,
                "validation passed and reviewer accepted the memory note",
            ));

        let report = AgentCycleOrchestrator::new().close_wave(
            dispatch,
            vec![result],
            AgentCycleEvidence {
                quality: 0.95,
                validation_passed: true,
                runtime_response_ok: true,
                reflection: Some(reflection),
                conflict_resolutions,
                toolsmith_plan: ToolsmithPlan::new(),
                tool_build_report: None,
            },
        );
        let summary = AgentCycleSummary::from_report(&report);

        assert_eq!(summary.unresolved_conflicts, 0);
        assert_eq!(summary.memory_promotions, 1);
        assert!(summary.ready_for_memory_promotion());
    }

    #[test]
    fn cycle_close_execution_preserves_failures_and_blocks_memory() {
        let task = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "draft patch",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![crate::task::TaskAssignment {
                task_id: task.id.clone(),
                role: task.role.clone(),
                lane: task.lane.clone(),
                budget_reserved: task.required_budget,
            }],
            ..TaskDispatchPlan::default()
        };
        let mut reflection = ReflectionLoop::new();
        reflection
            .submit(ReflectionStage::Draft, "draft accepted")
            .unwrap();
        reflection
            .submit(ReflectionStage::Critique, "engine failed")
            .unwrap();
        reflection
            .submit(ReflectionStage::Revision, "hold promotion")
            .unwrap();
        reflection
            .submit(ReflectionStage::MemoryNote, "remember failed execution")
            .unwrap();

        let report = AgentCycleOrchestrator::new().close_execution(
            dispatch,
            crate::execute::AgentWaveExecution {
                results: Vec::new(),
                failures: vec![crate::execute::AgentExecutionFailure {
                    task_id: "coder".to_owned(),
                    role: AgentRole::Coder,
                    reason: "engine failed coder".to_owned(),
                }],
            },
            AgentCycleEvidence {
                quality: 0.95,
                validation_passed: true,
                runtime_response_ok: true,
                reflection: Some(reflection),
                conflict_resolutions: ConflictResolutionBook::new(),
                toolsmith_plan: ToolsmithPlan::new(),
                tool_build_report: None,
            },
        );

        let summary = AgentCycleSummary::from_report(&report);
        let handoff = AgentCycleHandoff::from_report(&report);

        assert_eq!(summary.execution_failures, 1);
        assert!(!summary.ready_for_memory_promotion());
        assert!(report.memory_promotions.is_empty());
        assert!(
            report
                .reward_report
                .notes
                .iter()
                .any(|note| note == "execution:failures=1")
        );
        assert!(!handoff.can_submit_memory());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "execution_failures=1")
        );
    }
}
