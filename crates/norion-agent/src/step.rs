use std::collections::BTreeSet;

use crate::budget::{AgentBudget, BudgetLedger, BudgetPolicy};
use crate::control::{AgentBusinessLoopController, AgentBusinessLoopPlan};
use crate::cycle::{
    AgentCycleDispatch, AgentCycleEvidence, AgentCycleHandoff, AgentCycleOrchestrator,
    AgentCycleReport,
};
use crate::eval::{
    AgentCycleLedgerRecord, AgentReportEvidence, AgentReportGate, AgentReportGateDecision,
    AgentReportGatePolicy,
};
use crate::execute::{AgentWaveExecution, AgentWaveExecutor};
use crate::ledger::AgentCycleLedgerAdmissionStatus;
use crate::ledger::{AgentCycleLedger, AgentCycleLedgerEntry, AgentCycleLedgerPolicy};
use crate::loopback::{AgentLoopbackPlan, AgentLoopbackPlanner};
use crate::memory::{
    AgentMemoryReuseExecutionPreflightPlanner, AgentMemoryReuseExecutionPreflightPolicy,
    AgentMemoryReuseExecutionPreflightReport, AgentWaveMemoryRecallPlan,
    MemoryRecallDryRunEvidence, MemorySubmissionReport,
};
use crate::ports::EnginePort;
use crate::service::{
    AgentServiceCommandPlanner, AgentServiceCommandReceipt, AgentServiceExecutionReport,
    AgentServiceExecutionReportSummary,
};
use crate::task::{AgentRole, AgentTask, AgentTaskQueue};

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopStepInput {
    pub run_id: String,
    pub report: AgentCycleReport,
    pub handoff: AgentCycleHandoff,
    pub evidence: AgentReportEvidence,
    pub memory_submission: Option<MemorySubmissionReport>,
}

impl AgentClosedLoopStepInput {
    pub fn new(
        run_id: impl Into<String>,
        report: AgentCycleReport,
        handoff: AgentCycleHandoff,
        evidence: AgentReportEvidence,
        memory_submission: Option<MemorySubmissionReport>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            report,
            handoff,
            evidence,
            memory_submission,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopStep {
    pub record: AgentCycleLedgerRecord,
    pub report_decision: AgentReportGateDecision,
    pub loopback_plan: AgentLoopbackPlan,
    pub ledger_entry: AgentCycleLedgerEntry,
    pub updated_ledger: AgentCycleLedger,
    pub business_plan: AgentBusinessLoopPlan,
}

impl AgentClosedLoopStep {
    pub fn close_service_execution(
        &self,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopExecutionReport {
        let service_report = AgentServiceCommandPlanner::new().close_execution(
            &self.record.run_id,
            &self.business_plan,
            receipts,
        );

        AgentClosedLoopExecutionReport {
            step: self.clone(),
            service_report,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopExecutionReport {
    pub step: AgentClosedLoopStep,
    pub service_report: AgentServiceExecutionReport,
}

impl AgentClosedLoopExecutionReport {
    pub fn is_clean(&self) -> bool {
        self.step.report_decision.is_accepted()
            && self.step.loopback_plan.promote_adaptive_state
            && self.service_report.is_clean()
    }

    pub fn next_queue(&self) -> crate::task::AgentTaskQueue {
        self.service_report.next_queue()
    }

    pub fn service_summary(&self) -> AgentServiceExecutionReportSummary {
        self.service_report.summary()
    }

    pub fn summary(&self) -> AgentClosedLoopExecutionSummary {
        let service_summary = self.service_summary();
        let next_queue_task_ids = self.next_queue().task_ids();
        let mut blocked_reasons = self
            .step
            .report_decision
            .reasons
            .iter()
            .map(|reason| reason.as_line())
            .collect::<Vec<_>>();
        blocked_reasons.extend(self.step.loopback_plan.blocked_reasons.clone());
        blocked_reasons.extend(service_summary.blocked_reasons.clone());

        AgentClosedLoopExecutionSummary {
            run_id: self.step.record.run_id.clone(),
            clean: self.is_clean(),
            report_accepted: self.step.report_decision.is_accepted(),
            loopback_promoted: self.step.loopback_plan.promote_adaptive_state,
            service_clean: service_summary.clean,
            reward_total: self.step.record.summary.reward_total,
            admission_status: self.step.business_plan.status(),
            command_count: service_summary.command_count,
            missing_command_count: service_summary.missing_commands,
            failed_command_count: service_summary.failed_commands,
            skipped_command_count: service_summary.skipped_commands,
            next_queue_tasks: service_summary.next_queue_tasks,
            next_queue_task_ids,
            blocked_reasons,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopExecutionSummary {
    pub run_id: String,
    pub clean: bool,
    pub report_accepted: bool,
    pub loopback_promoted: bool,
    pub service_clean: bool,
    pub reward_total: f32,
    pub admission_status: AgentCycleLedgerAdmissionStatus,
    pub command_count: usize,
    pub missing_command_count: usize,
    pub failed_command_count: usize,
    pub skipped_command_count: usize,
    pub next_queue_tasks: usize,
    pub next_queue_task_ids: Vec<String>,
    pub blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentClosedLoopExecutionHistory {
    summaries: Vec<AgentClosedLoopExecutionSummary>,
}

impl AgentClosedLoopExecutionHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentClosedLoopExecutionSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentClosedLoopExecutionSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentClosedLoopExecutionSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentClosedLoopExecutionSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentClosedLoopExecutionDashboard {
        AgentClosedLoopExecutionDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopExecutionHealthPolicy,
    ) -> AgentClosedLoopExecutionHealth {
        self.dashboard().health(policy)
    }

    pub fn next_turn_plan(
        &self,
        next_queue: AgentTaskQueue,
        policy: AgentClosedLoopExecutionHealthPolicy,
    ) -> AgentClosedLoopNextTurnPlan {
        AgentClosedLoopNextTurnPlan::from_history(self.clone(), next_queue, policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopExecutionDashboard {
    pub total_runs: usize,
    pub clean_runs: usize,
    pub dirty_runs: usize,
    pub clean_rate: f32,
    pub report_blocked_runs: usize,
    pub loopback_blocked_runs: usize,
    pub service_dirty_runs: usize,
    pub promote_admissions: usize,
    pub hold_admissions: usize,
    pub repair_admissions: usize,
    pub command_count: usize,
    pub missing_command_count: usize,
    pub failed_command_count: usize,
    pub skipped_command_count: usize,
    pub service_failure_pressure: f32,
    pub total_next_queue_tasks: usize,
    pub average_reward_total: f32,
    pub latest_run_id: Option<String>,
    pub latest_blocked_reasons: Vec<String>,
}

impl AgentClosedLoopExecutionDashboard {
    pub fn from_summaries(summaries: &[AgentClosedLoopExecutionSummary]) -> Self {
        let total_runs = summaries.len();
        let clean_runs = summaries.iter().filter(|summary| summary.clean).count();
        let dirty_runs = total_runs.saturating_sub(clean_runs);
        let report_blocked_runs = summaries
            .iter()
            .filter(|summary| !summary.report_accepted)
            .count();
        let loopback_blocked_runs = summaries
            .iter()
            .filter(|summary| !summary.loopback_promoted)
            .count();
        let service_dirty_runs = summaries
            .iter()
            .filter(|summary| !summary.service_clean)
            .count();
        let promote_admissions = summaries
            .iter()
            .filter(|summary| summary.admission_status == AgentCycleLedgerAdmissionStatus::Promote)
            .count();
        let hold_admissions = summaries
            .iter()
            .filter(|summary| summary.admission_status == AgentCycleLedgerAdmissionStatus::Hold)
            .count();
        let repair_admissions = summaries
            .iter()
            .filter(|summary| summary.admission_status == AgentCycleLedgerAdmissionStatus::Repair)
            .count();
        let command_count = summaries
            .iter()
            .map(|summary| summary.command_count)
            .sum::<usize>();
        let missing_command_count = summaries
            .iter()
            .map(|summary| summary.missing_command_count)
            .sum::<usize>();
        let failed_command_count = summaries
            .iter()
            .map(|summary| summary.failed_command_count)
            .sum::<usize>();
        let skipped_command_count = summaries
            .iter()
            .map(|summary| summary.skipped_command_count)
            .sum::<usize>();
        let service_failure_count =
            missing_command_count + failed_command_count + skipped_command_count;
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let reward_total_sum = summaries
            .iter()
            .map(|summary| summary.reward_total)
            .sum::<f32>();
        let latest = summaries.last();

        Self {
            total_runs,
            clean_runs,
            dirty_runs,
            clean_rate: rate(clean_runs, total_runs),
            report_blocked_runs,
            loopback_blocked_runs,
            service_dirty_runs,
            promote_admissions,
            hold_admissions,
            repair_admissions,
            command_count,
            missing_command_count,
            failed_command_count,
            skipped_command_count,
            service_failure_pressure: rate(service_failure_count, command_count),
            total_next_queue_tasks,
            average_reward_total: if total_runs == 0 {
                0.0
            } else {
                reward_total_sum / total_runs as f32
            },
            latest_run_id: latest.map(|summary| summary.run_id.clone()),
            latest_blocked_reasons: latest
                .map(|summary| summary.blocked_reasons.clone())
                .unwrap_or_default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_runs == 0
    }

    pub fn is_service_clean(&self) -> bool {
        self.service_dirty_runs == 0
            && self.missing_command_count == 0
            && self.failed_command_count == 0
            && self.skipped_command_count == 0
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopExecutionHealthPolicy,
    ) -> AgentClosedLoopExecutionHealth {
        AgentClosedLoopExecutionHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentClosedLoopExecutionHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AgentClosedLoopExecutionHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentClosedLoopExecutionHealthPolicy {
    pub minimum_clean_rate: f32,
    pub maximum_service_failure_pressure: f32,
    pub maximum_report_blocked_runs: usize,
    pub maximum_loopback_blocked_runs: usize,
    pub maximum_repair_admissions: usize,
}

impl Default for AgentClosedLoopExecutionHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_clean_rate: 0.67,
            maximum_service_failure_pressure: 0.0,
            maximum_report_blocked_runs: 0,
            maximum_loopback_blocked_runs: 0,
            maximum_repair_admissions: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopExecutionHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentClosedLoopExecutionDashboard,
}

impl AgentClosedLoopExecutionHealth {
    pub fn from_dashboard(
        dashboard: AgentClosedLoopExecutionDashboard,
        policy: AgentClosedLoopExecutionHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("execution_history_empty".to_owned());
        }

        if dashboard.service_failure_pressure > policy.maximum_service_failure_pressure {
            repair_reasons.push(format!(
                "service_failure_pressure={:.3}>{}",
                dashboard.service_failure_pressure, policy.maximum_service_failure_pressure
            ));
        }

        if dashboard.repair_admissions > policy.maximum_repair_admissions {
            repair_reasons.push(format!(
                "repair_admissions={}>{}",
                dashboard.repair_admissions, policy.maximum_repair_admissions
            ));
        }

        if !dashboard.is_empty() && dashboard.clean_rate < policy.minimum_clean_rate {
            watch_reasons.push(format!(
                "clean_rate={:.3}<{}",
                dashboard.clean_rate, policy.minimum_clean_rate
            ));
        }

        if dashboard.report_blocked_runs > policy.maximum_report_blocked_runs {
            watch_reasons.push(format!(
                "report_blocked_runs={}>{}",
                dashboard.report_blocked_runs, policy.maximum_report_blocked_runs
            ));
        }

        if dashboard.loopback_blocked_runs > policy.maximum_loopback_blocked_runs {
            watch_reasons.push(format!(
                "loopback_blocked_runs={}>{}",
                dashboard.loopback_blocked_runs, policy.maximum_loopback_blocked_runs
            ));
        }

        if !dashboard.latest_blocked_reasons.is_empty() {
            watch_reasons.push(format!(
                "latest_blocked={}",
                dashboard.latest_blocked_reasons.join(";")
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentClosedLoopNextTurnMode {
    Continue,
    Observe,
    Repair,
    Idle,
}

impl AgentClosedLoopNextTurnMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::Observe => "observe",
            Self::Repair => "repair",
            Self::Idle => "idle",
        }
    }

    pub fn can_schedule(self) -> bool {
        !matches!(self, Self::Idle)
    }

    pub fn allows_adaptive_evolution(self) -> bool {
        matches!(self, Self::Continue)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopNextTurnPlan {
    pub mode: AgentClosedLoopNextTurnMode,
    pub history: AgentClosedLoopExecutionHistory,
    pub health: AgentClosedLoopExecutionHealth,
    pub next_queue: AgentTaskQueue,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopNextTurnPlan {
    pub fn from_history(
        history: AgentClosedLoopExecutionHistory,
        next_queue: AgentTaskQueue,
        policy: AgentClosedLoopExecutionHealthPolicy,
    ) -> Self {
        let health = history.health(policy);
        let mode = if next_queue.is_empty() {
            AgentClosedLoopNextTurnMode::Idle
        } else {
            match health.status {
                AgentClosedLoopExecutionHealthStatus::Stable => {
                    AgentClosedLoopNextTurnMode::Continue
                }
                AgentClosedLoopExecutionHealthStatus::Watch => AgentClosedLoopNextTurnMode::Observe,
                AgentClosedLoopExecutionHealthStatus::Repair => AgentClosedLoopNextTurnMode::Repair,
            }
        };
        let mut reasons = health.reasons.clone();
        if next_queue.is_empty() {
            reasons.push("next_queue_empty".to_owned());
        }
        let mut telemetry = vec![
            format!("next_turn_mode={}", mode.as_str()),
            format!("health_status={}", health.status.as_str()),
            format!("history_runs={}", history.len()),
            format!("next_queue_tasks={}", next_queue.len()),
        ];
        telemetry.extend(reasons.iter().map(|reason| format!("reason={reason}")));

        Self {
            mode,
            history,
            health,
            next_queue,
            reasons,
            telemetry,
        }
    }

    pub fn can_schedule(&self) -> bool {
        self.mode.can_schedule() && !self.next_queue.is_empty()
    }

    pub fn with_memory_reuse_preflight_health(
        mut self,
        memory_health: AgentClosedLoopMemoryReusePreflightExecutionHealth,
    ) -> Self {
        let previous_mode = self.mode;
        if memory_health.requires_repair_first() {
            self.mode = AgentClosedLoopNextTurnMode::Repair;
        } else if !memory_health.is_stable() && self.mode == AgentClosedLoopNextTurnMode::Continue {
            self.mode = AgentClosedLoopNextTurnMode::Observe;
        }

        for reason in &memory_health.reasons {
            self.reasons
                .push(format!("memory_reuse_preflight:{reason}"));
        }
        if let Some(line) = self
            .telemetry
            .iter_mut()
            .find(|line| line.starts_with("next_turn_mode="))
        {
            *line = format!("next_turn_mode={}", self.mode.as_str());
        } else {
            self.telemetry
                .push(format!("next_turn_mode={}", self.mode.as_str()));
        }
        self.telemetry.push(format!(
            "memory_reuse_preflight_health_status={}",
            memory_health.status.as_str()
        ));
        self.telemetry.push(format!(
            "memory_reuse_preflight_reports={}",
            memory_health.dashboard.total_reports
        ));
        self.telemetry.push(format!(
            "memory_reuse_preflight_clean_rate={:.3}",
            memory_health.dashboard.clean_rate
        ));
        self.telemetry.push(format!(
            "memory_reuse_preflight_skipped_engine_calls={}",
            memory_health.dashboard.skipped_engine_calls
        ));
        if self.mode != previous_mode {
            self.telemetry.push(format!(
                "memory_reuse_preflight_mode_override={}->{}",
                previous_mode.as_str(),
                self.mode.as_str()
            ));
        }
        self.telemetry.extend(
            memory_health
                .reasons
                .iter()
                .map(|reason| format!("reason=memory_reuse_preflight:{reason}")),
        );

        self
    }

    pub fn with_memory_reuse_preflight_repair_tasks(
        self,
        memory_health: AgentClosedLoopMemoryReusePreflightExecutionHealth,
    ) -> Self {
        let repair_task_plan =
            AgentClosedLoopMemoryReusePreflightRepairTaskPlan::from_health_and_queue(
                memory_health.clone(),
                self.next_queue.clone(),
            );
        let repair_task_summary = repair_task_plan.summary();
        let mut plan = self.with_memory_reuse_preflight_health(memory_health);
        if repair_task_summary.repair_tasks == 0 {
            return plan;
        }

        plan.next_queue = repair_task_plan.next_queue;
        let next_queue_tasks = plan.next_queue.len();
        if let Some(line) = plan
            .telemetry
            .iter_mut()
            .find(|line| line.starts_with("next_queue_tasks="))
        {
            *line = format!("next_queue_tasks={next_queue_tasks}");
        } else {
            plan.telemetry
                .push(format!("next_queue_tasks={next_queue_tasks}"));
        }
        plan.reasons.push(format!(
            "memory_reuse_preflight:repair_tasks={}",
            repair_task_summary.repair_tasks
        ));
        plan.telemetry.push(format!(
            "memory_reuse_preflight_repair_tasks={}",
            repair_task_summary.repair_tasks
        ));
        plan.telemetry.extend(
            repair_task_summary
                .repair_task_ids
                .iter()
                .map(|task_id| format!("memory_reuse_preflight_repair_task_id={task_id}")),
        );
        plan.telemetry.extend(repair_task_plan.telemetry);

        plan
    }

    pub fn with_memory_reuse_preflight_repair_task_plan_health(
        mut self,
        repair_health: AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealth,
    ) -> Self {
        let previous_mode = self.mode;
        if repair_health.requires_repair_first() {
            self.mode = AgentClosedLoopNextTurnMode::Repair;
        } else if !repair_health.is_stable() && self.mode == AgentClosedLoopNextTurnMode::Continue {
            self.mode = AgentClosedLoopNextTurnMode::Observe;
        }

        for reason in &repair_health.reasons {
            self.reasons
                .push(format!("memory_reuse_preflight_repair_task_plan:{reason}"));
        }
        if let Some(line) = self
            .telemetry
            .iter_mut()
            .find(|line| line.starts_with("next_turn_mode="))
        {
            *line = format!("next_turn_mode={}", self.mode.as_str());
        } else {
            self.telemetry
                .push(format!("next_turn_mode={}", self.mode.as_str()));
        }
        self.telemetry.push(format!(
            "memory_reuse_preflight_repair_task_plan_health_status={}",
            repair_health.status.as_str()
        ));
        self.telemetry.push(format!(
            "memory_reuse_preflight_repair_task_plan_records={}",
            repair_health.dashboard.total_records
        ));
        self.telemetry.push(format!(
            "memory_reuse_preflight_repair_task_plan_repair_first_records={}",
            repair_health.dashboard.repair_first_records
        ));
        self.telemetry.push(format!(
            "memory_reuse_preflight_repair_task_plan_repair_tasks={}",
            repair_health.dashboard.repair_tasks
        ));
        self.telemetry.push(format!(
            "memory_reuse_preflight_repair_task_plan_non_repair_rate={:.3}",
            repair_health.dashboard.non_repair_rate
        ));
        if self.mode != previous_mode {
            self.telemetry.push(format!(
                "memory_reuse_preflight_repair_task_plan_mode_override={}->{}",
                previous_mode.as_str(),
                self.mode.as_str()
            ));
        }
        self.telemetry.extend(
            repair_health
                .reasons
                .iter()
                .map(|reason| format!("reason=memory_reuse_preflight_repair_task_plan:{reason}")),
        );

        self
    }

    pub fn allows_adaptive_evolution(&self) -> bool {
        self.mode.allows_adaptive_evolution()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.mode == AgentClosedLoopNextTurnMode::Repair
    }

    pub fn allows_service_advance(&self) -> bool {
        !self.requires_repair_first()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopMemoryReusePreflightRepairTaskPlan {
    pub memory_health: AgentClosedLoopMemoryReusePreflightExecutionHealth,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopMemoryReusePreflightRepairTaskPlan {
    pub fn from_health_and_queue(
        memory_health: AgentClosedLoopMemoryReusePreflightExecutionHealth,
        next_queue: AgentTaskQueue,
    ) -> Self {
        let requires_repair_first = memory_health.requires_repair_first();
        let reasons = memory_health.reasons.clone();
        let repair_tasks = memory_reuse_preflight_repair_tasks(&memory_health);
        let next_queue = next_queue.with_repair_first(&repair_tasks);
        let repair_task_ids = repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = next_queue.task_ids();
        let telemetry = memory_reuse_preflight_repair_task_plan_telemetry(
            memory_health.status,
            requires_repair_first,
            repair_task_ids.len(),
            next_queue_task_ids.len(),
            reasons.len(),
            &repair_task_ids,
            &next_queue_task_ids,
        );

        Self {
            memory_health,
            requires_repair_first,
            repair_tasks,
            next_queue,
            reasons,
            telemetry,
        }
    }

    pub fn summary(&self) -> AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary {
        AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary::from_plan(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary {
    pub memory_health_status: AgentClosedLoopMemoryReusePreflightExecutionHealthStatus,
    pub requires_repair_first: bool,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub reasons: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary {
    pub fn from_plan(plan: &AgentClosedLoopMemoryReusePreflightRepairTaskPlan) -> Self {
        let repair_task_ids = plan
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = plan.next_queue.task_ids();
        let telemetry = memory_reuse_preflight_repair_task_plan_summary_telemetry(
            plan.memory_health.status,
            plan.requires_repair_first,
            repair_task_ids.len(),
            next_queue_task_ids.len(),
            plan.reasons.len(),
            &repair_task_ids,
            &next_queue_task_ids,
        );

        Self {
            memory_health_status: plan.memory_health.status,
            requires_repair_first: plan.requires_repair_first,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            reasons: plan.reasons.len(),
            repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory {
    summaries: Vec<AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopMemoryReusePreflightRepairTaskPlanDashboard {
    pub total_records: usize,
    pub repair_first_records: usize,
    pub non_repair_records: usize,
    pub repair_tasks: usize,
    pub reasons: usize,
    pub next_queue_tasks: usize,
    pub non_repair_rate: f32,
    pub latest_repair_task_ids: Vec<String>,
    pub latest_next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy {
    pub maximum_repair_first_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_reasons: usize,
    pub minimum_non_repair_rate: f32,
}

impl Default for AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy {
    fn default() -> Self {
        Self {
            maximum_repair_first_records: 0,
            maximum_repair_tasks: 0,
            maximum_reasons: 0,
            minimum_non_repair_rate: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealth {
    pub status: AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentClosedLoopMemoryReusePreflightRepairTaskPlanDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecord {
    pub history: AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory,
    pub appended_summary: AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary,
    pub dashboard: AgentClosedLoopMemoryReusePreflightRepairTaskPlanDashboard,
    pub health: AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecorder;

impl AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentClosedLoopMemoryReusePreflightRepairTaskPlanDashboard {
        AgentClosedLoopMemoryReusePreflightRepairTaskPlanDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy,
    ) -> AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealth {
        self.dashboard().health(policy)
    }
}

impl AgentClosedLoopMemoryReusePreflightRepairTaskPlanDashboard {
    pub fn from_summaries(
        summaries: &[AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary],
    ) -> Self {
        let total_records = summaries.len();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let non_repair_records = total_records.saturating_sub(repair_first_records);
        let repair_tasks = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let reasons = summaries
            .iter()
            .map(|summary| summary.reasons)
            .sum::<usize>();
        let next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let latest_repair_task_ids = summaries
            .last()
            .map(|summary| summary.repair_task_ids.clone())
            .unwrap_or_default();
        let latest_next_queue_task_ids = summaries
            .last()
            .map(|summary| summary.next_queue_task_ids.clone())
            .unwrap_or_default();
        let non_repair_rate = rate(non_repair_records, total_records);
        let telemetry = memory_reuse_preflight_repair_task_plan_dashboard_telemetry(
            total_records,
            repair_first_records,
            non_repair_records,
            repair_tasks,
            reasons,
            next_queue_tasks,
            non_repair_rate,
            &latest_repair_task_ids,
            &latest_next_queue_task_ids,
        );

        Self {
            total_records,
            repair_first_records,
            non_repair_records,
            repair_tasks,
            reasons,
            next_queue_tasks,
            non_repair_rate,
            latest_repair_task_ids,
            latest_next_queue_task_ids,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy,
    ) -> AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealth {
        AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

impl AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealth {
    pub fn from_dashboard(
        dashboard: AgentClosedLoopMemoryReusePreflightRepairTaskPlanDashboard,
        policy: AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("memory_reuse_preflight_repair_task_plan_history_empty".to_owned());
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "memory_reuse_preflight_repair_task_plan_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }
        if dashboard.repair_tasks > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "memory_reuse_preflight_repair_task_plan_repair_tasks={}>{}",
                dashboard.repair_tasks, policy.maximum_repair_tasks
            ));
        }

        if dashboard.reasons > policy.maximum_reasons {
            watch_reasons.push(format!(
                "memory_reuse_preflight_repair_task_plan_reasons={}>{}",
                dashboard.reasons, policy.maximum_reasons
            ));
        }
        if !dashboard.is_empty() && dashboard.non_repair_rate < policy.minimum_non_repair_rate {
            watch_reasons.push(format!(
                "memory_reuse_preflight_repair_task_plan_non_repair_rate={:.3}<{}",
                dashboard.non_repair_rate, policy.minimum_non_repair_rate
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (
                AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus::Repair,
                repair_reasons,
            )
        } else if !watch_reasons.is_empty() {
            (
                AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus::Watch,
                watch_reasons,
            )
        } else {
            (
                AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus::Stable,
                Vec::new(),
            )
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus::Repair
    }
}

impl AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecord {
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

impl AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory,
        summary: AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummary,
        policy: AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy,
    ) -> AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            memory_reuse_preflight_repair_task_plan_history_record_telemetry(&dashboard, &health);

        AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_plan_with_health(
        &self,
        history: AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory,
        plan: &AgentClosedLoopMemoryReusePreflightRepairTaskPlan,
        policy: AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy,
    ) -> AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecord {
        self.record_summary_with_health(history, plan.summary(), policy)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopNextTurnPlanner;

impl AgentClosedLoopNextTurnPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        history: &AgentClosedLoopExecutionHistory,
        execution: &AgentClosedLoopExecutionReport,
        policy: AgentClosedLoopExecutionHealthPolicy,
    ) -> AgentClosedLoopNextTurnPlan {
        let mut updated_history = history.clone();
        updated_history.push(execution.summary());
        AgentClosedLoopNextTurnPlan::from_history(updated_history, execution.next_queue(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopPreparedDispatch {
    pub turn_plan: AgentClosedLoopNextTurnPlan,
    pub dispatch: Option<AgentCycleDispatch>,
    pub skipped_reasons: Vec<String>,
}

impl AgentClosedLoopPreparedDispatch {
    pub fn can_dispatch(&self) -> bool {
        if !self.skipped_reasons.is_empty() {
            return false;
        }

        self.dispatch
            .as_ref()
            .is_some_and(|dispatch| !dispatch.assigned_tasks.is_empty())
    }

    pub fn assigned_task_ids(&self) -> Vec<String> {
        self.dispatch
            .as_ref()
            .map(|dispatch| {
                dispatch
                    .assigned_tasks
                    .iter()
                    .map(|task| task.id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopPreparedMemoryReusePreflight {
    pub prepared_dispatch: AgentClosedLoopPreparedDispatch,
    pub preflight: Option<AgentMemoryReuseExecutionPreflightReport>,
    pub skipped_reasons: Vec<String>,
}

impl AgentClosedLoopPreparedMemoryReusePreflight {
    pub fn has_preflight(&self) -> bool {
        self.preflight.is_some()
    }

    pub fn can_enter_execution(&self) -> bool {
        self.preflight
            .as_ref()
            .is_some_and(|preflight| preflight.can_enter_execution)
            && self.skipped_reasons.is_empty()
    }

    pub fn requires_repair_first(&self) -> bool {
        !self.skipped_reasons.is_empty()
            || self
                .preflight
                .as_ref()
                .is_some_and(|preflight| preflight.requires_repair_first)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClosedLoopPreparedMemoryReusePreflightPlanner {
    preflight_planner: AgentMemoryReuseExecutionPreflightPlanner,
}

impl Default for AgentClosedLoopPreparedMemoryReusePreflightPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentClosedLoopPreparedMemoryReusePreflightPlanner {
    pub fn new() -> Self {
        Self {
            preflight_planner: AgentMemoryReuseExecutionPreflightPlanner::new(),
        }
    }

    pub fn with_policy(policy: AgentMemoryReuseExecutionPreflightPolicy) -> Self {
        Self {
            preflight_planner: AgentMemoryReuseExecutionPreflightPlanner::new().with_policy(policy),
        }
    }

    pub fn plan(
        &self,
        prepared_dispatch: AgentClosedLoopPreparedDispatch,
        recall_plan: &AgentWaveMemoryRecallPlan,
        dry_run_evidence: &[MemoryRecallDryRunEvidence],
    ) -> AgentClosedLoopPreparedMemoryReusePreflight {
        if !prepared_dispatch.can_dispatch() {
            let skipped_reasons = if prepared_dispatch.skipped_reasons.is_empty() {
                vec!["prepared_dispatch_not_executable".to_owned()]
            } else {
                prepared_dispatch.skipped_reasons.clone()
            };
            return AgentClosedLoopPreparedMemoryReusePreflight {
                prepared_dispatch,
                preflight: None,
                skipped_reasons,
            };
        }

        let preflight = prepared_dispatch.dispatch.as_ref().map(|dispatch| {
            self.preflight_planner
                .plan_for_dispatch(dispatch, recall_plan, dry_run_evidence)
        });
        let skipped_reasons = preflight
            .as_ref()
            .filter(|preflight| !preflight.can_enter_execution)
            .map(|preflight| preflight.blocked_reasons.clone())
            .unwrap_or_default();

        AgentClosedLoopPreparedMemoryReusePreflight {
            prepared_dispatch,
            preflight,
            skipped_reasons,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentClosedLoopDispatchPreparer {
    orchestrator: AgentCycleOrchestrator,
}

impl Default for AgentClosedLoopDispatchPreparer {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentClosedLoopDispatchPreparer {
    pub fn new() -> Self {
        Self {
            orchestrator: AgentCycleOrchestrator::new(),
        }
    }

    pub fn with_orchestrator(orchestrator: AgentCycleOrchestrator) -> Self {
        Self { orchestrator }
    }

    pub fn prepare(
        &self,
        turn_plan: AgentClosedLoopNextTurnPlan,
        completed_task_ids: &BTreeSet<String>,
        ledger: BudgetLedger,
        policy: &BudgetPolicy,
        max_parallel_tasks: usize,
    ) -> AgentClosedLoopPreparedDispatch {
        if !turn_plan.can_schedule() {
            return AgentClosedLoopPreparedDispatch {
                skipped_reasons: if turn_plan.reasons.is_empty() {
                    vec!["next_turn_not_schedulable".to_owned()]
                } else {
                    turn_plan.reasons.clone()
                },
                turn_plan,
                dispatch: None,
            };
        }

        let dispatch = self.orchestrator.plan_next_wave(
            turn_plan.next_queue.clone(),
            completed_task_ids,
            ledger,
            policy,
            max_parallel_tasks,
        );
        let mut skipped_reasons = Vec::new();
        if dispatch.assigned_tasks.is_empty() {
            skipped_reasons.push("dispatch_empty".to_owned());
        }
        if turn_plan.requires_repair_first()
            && dispatch
                .assigned_tasks
                .iter()
                .all(|task| !is_repair_task(task))
        {
            skipped_reasons.push("repair_first_task_missing".to_owned());
        }

        AgentClosedLoopPreparedDispatch {
            turn_plan,
            dispatch: Some(dispatch),
            skipped_reasons,
        }
    }
}

fn is_repair_task(task: &AgentTask) -> bool {
    task.lane.contains("repair") || task.id.contains("repair")
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopPreparedExecution {
    pub prepared_dispatch: AgentClosedLoopPreparedDispatch,
    pub execution: Option<AgentWaveExecution>,
    pub skipped_reasons: Vec<String>,
}

impl AgentClosedLoopPreparedExecution {
    pub fn has_execution(&self) -> bool {
        self.execution.is_some()
    }

    pub fn is_complete(&self) -> bool {
        self.execution
            .as_ref()
            .is_some_and(AgentWaveExecution::is_complete)
    }

    pub fn result_count(&self) -> usize {
        self.execution
            .as_ref()
            .map(|execution| execution.results.len())
            .unwrap_or_default()
    }

    pub fn failure_count(&self) -> usize {
        self.execution
            .as_ref()
            .map(|execution| execution.failures.len())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopPreparedExecutor {
    executor: AgentWaveExecutor,
}

impl AgentClosedLoopPreparedExecutor {
    pub fn new() -> Self {
        Self {
            executor: AgentWaveExecutor::new(),
        }
    }

    pub fn with_executor(executor: AgentWaveExecutor) -> Self {
        Self { executor }
    }

    pub fn execute<E>(
        &self,
        prepared_dispatch: AgentClosedLoopPreparedDispatch,
        engine: &mut E,
    ) -> AgentClosedLoopPreparedExecution
    where
        E: EnginePort,
        E::Error: ToString,
    {
        if !prepared_dispatch.can_dispatch() {
            let skipped_reasons = if prepared_dispatch.skipped_reasons.is_empty() {
                vec!["prepared_dispatch_not_executable".to_owned()]
            } else {
                prepared_dispatch.skipped_reasons.clone()
            };
            return AgentClosedLoopPreparedExecution {
                prepared_dispatch,
                execution: None,
                skipped_reasons,
            };
        }

        let execution = prepared_dispatch
            .dispatch
            .as_ref()
            .map(|dispatch| self.executor.execute(dispatch, engine));

        AgentClosedLoopPreparedExecution {
            prepared_dispatch,
            execution,
            skipped_reasons: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopMemoryReusePreflightExecutor {
    executor: AgentClosedLoopPreparedExecutor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopMemoryReusePreflightExecutionReport {
    pub preflight: Option<AgentMemoryReuseExecutionPreflightReport>,
    pub execution: AgentClosedLoopPreparedExecution,
    pub planned_engine_calls: usize,
    pub executed_engine_calls: usize,
    pub skipped_engine_calls: usize,
    pub preflight_clean: bool,
    pub memory_reuse_ready: bool,
    pub can_enter_execution: bool,
    pub execution_complete: bool,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopMemoryReusePreflightExecutionReport {
    pub fn is_clean(&self) -> bool {
        self.preflight_clean
            && self.memory_reuse_ready
            && self.can_enter_execution
            && self.execution_complete
            && self.skipped_engine_calls == 0
            && self.blocked_reasons.is_empty()
    }

    pub fn saved_compute(&self) -> bool {
        self.skipped_engine_calls > 0
    }

    pub fn summary_line(&self) -> String {
        format!(
            "agent_memory_reuse_preflight_execution preflight_clean={} memory_reuse_ready={} can_enter_execution={} execution_complete={} planned_engine_calls={} executed_engine_calls={} skipped_engine_calls={} blocked_reasons={}",
            self.preflight_clean,
            self.memory_reuse_ready,
            self.can_enter_execution,
            self.execution_complete,
            self.planned_engine_calls,
            self.executed_engine_calls,
            self.skipped_engine_calls,
            self.blocked_reasons.len(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopMemoryReusePreflightExecutionSummary {
    pub clean: bool,
    pub preflight_present: bool,
    pub preflight_clean: bool,
    pub memory_reuse_ready: bool,
    pub can_enter_execution: bool,
    pub execution_complete: bool,
    pub saved_compute: bool,
    pub planned_engine_calls: usize,
    pub executed_engine_calls: usize,
    pub skipped_engine_calls: usize,
    pub blocked_reasons: Vec<String>,
}

impl AgentClosedLoopMemoryReusePreflightExecutionSummary {
    pub fn from_report(report: &AgentClosedLoopMemoryReusePreflightExecutionReport) -> Self {
        Self {
            clean: report.is_clean(),
            preflight_present: report.preflight.is_some(),
            preflight_clean: report.preflight_clean,
            memory_reuse_ready: report.memory_reuse_ready,
            can_enter_execution: report.can_enter_execution,
            execution_complete: report.execution_complete,
            saved_compute: report.saved_compute(),
            planned_engine_calls: report.planned_engine_calls,
            executed_engine_calls: report.executed_engine_calls,
            skipped_engine_calls: report.skipped_engine_calls,
            blocked_reasons: report.blocked_reasons.clone(),
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "agent_memory_reuse_preflight_execution_summary clean={} preflight_present={} memory_reuse_ready={} can_enter_execution={} execution_complete={} saved_compute={} planned_engine_calls={} executed_engine_calls={} skipped_engine_calls={} blocked_reasons={}",
            self.clean,
            self.preflight_present,
            self.memory_reuse_ready,
            self.can_enter_execution,
            self.execution_complete,
            self.saved_compute,
            self.planned_engine_calls,
            self.executed_engine_calls,
            self.skipped_engine_calls,
            self.blocked_reasons.len(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentClosedLoopMemoryReusePreflightExecutionHistory {
    summaries: Vec<AgentClosedLoopMemoryReusePreflightExecutionSummary>,
}

impl AgentClosedLoopMemoryReusePreflightExecutionHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentClosedLoopMemoryReusePreflightExecutionSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentClosedLoopMemoryReusePreflightExecutionSummary) {
        self.summaries.push(summary);
    }

    pub fn record_report(&mut self, report: &AgentClosedLoopMemoryReusePreflightExecutionReport) {
        self.push(AgentClosedLoopMemoryReusePreflightExecutionSummary::from_report(report));
    }

    pub fn summaries(&self) -> &[AgentClosedLoopMemoryReusePreflightExecutionSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentClosedLoopMemoryReusePreflightExecutionSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentClosedLoopMemoryReusePreflightExecutionDashboard {
        AgentClosedLoopMemoryReusePreflightExecutionDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy,
    ) -> AgentClosedLoopMemoryReusePreflightExecutionHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopMemoryReusePreflightExecutionDashboard {
    pub total_reports: usize,
    pub clean_reports: usize,
    pub blocked_reports: usize,
    pub missing_preflight_reports: usize,
    pub saved_compute_reports: usize,
    pub clean_rate: f32,
    pub planned_engine_calls: usize,
    pub executed_engine_calls: usize,
    pub skipped_engine_calls: usize,
    pub skipped_engine_call_rate: f32,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopMemoryReusePreflightExecutionDashboard {
    pub fn from_summaries(
        summaries: &[AgentClosedLoopMemoryReusePreflightExecutionSummary],
    ) -> Self {
        let total_reports = summaries.len();
        let clean_reports = summaries.iter().filter(|summary| summary.clean).count();
        let blocked_reports = summaries
            .iter()
            .filter(|summary| !summary.can_enter_execution || !summary.blocked_reasons.is_empty())
            .count();
        let missing_preflight_reports = summaries
            .iter()
            .filter(|summary| !summary.preflight_present)
            .count();
        let saved_compute_reports = summaries
            .iter()
            .filter(|summary| summary.saved_compute)
            .count();
        let planned_engine_calls = summaries
            .iter()
            .map(|summary| summary.planned_engine_calls)
            .sum::<usize>();
        let executed_engine_calls = summaries
            .iter()
            .map(|summary| summary.executed_engine_calls)
            .sum::<usize>();
        let skipped_engine_calls = summaries
            .iter()
            .map(|summary| summary.skipped_engine_calls)
            .sum::<usize>();
        let latest_blocked_reasons = summaries
            .last()
            .map(|summary| summary.blocked_reasons.clone())
            .unwrap_or_default();
        let telemetry = memory_reuse_preflight_execution_dashboard_telemetry(
            total_reports,
            clean_reports,
            blocked_reports,
            missing_preflight_reports,
            saved_compute_reports,
            planned_engine_calls,
            executed_engine_calls,
            skipped_engine_calls,
        );

        Self {
            total_reports,
            clean_reports,
            blocked_reports,
            missing_preflight_reports,
            saved_compute_reports,
            clean_rate: rate(clean_reports, total_reports),
            planned_engine_calls,
            executed_engine_calls,
            skipped_engine_calls,
            skipped_engine_call_rate: rate(skipped_engine_calls, planned_engine_calls),
            latest_blocked_reasons,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_reports == 0
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy,
    ) -> AgentClosedLoopMemoryReusePreflightExecutionHealth {
        AgentClosedLoopMemoryReusePreflightExecutionHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentClosedLoopMemoryReusePreflightExecutionHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AgentClosedLoopMemoryReusePreflightExecutionHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy {
    pub minimum_clean_rate: f32,
    pub maximum_blocked_reports: usize,
    pub maximum_missing_preflight_reports: usize,
    pub maximum_skipped_engine_calls: usize,
}

impl Default for AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_clean_rate: 1.0,
            maximum_blocked_reports: 0,
            maximum_missing_preflight_reports: 0,
            maximum_skipped_engine_calls: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopMemoryReusePreflightExecutionHealth {
    pub status: AgentClosedLoopMemoryReusePreflightExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentClosedLoopMemoryReusePreflightExecutionDashboard,
}

impl AgentClosedLoopMemoryReusePreflightExecutionHealth {
    pub fn from_dashboard(
        dashboard: AgentClosedLoopMemoryReusePreflightExecutionDashboard,
        policy: AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("memory_reuse_preflight_execution_history_empty".to_owned());
        }

        if dashboard.missing_preflight_reports > policy.maximum_missing_preflight_reports {
            repair_reasons.push(format!(
                "memory_reuse_missing_preflight_reports={}>{}",
                dashboard.missing_preflight_reports, policy.maximum_missing_preflight_reports
            ));
        }
        if dashboard.blocked_reports > policy.maximum_blocked_reports {
            repair_reasons.push(format!(
                "memory_reuse_blocked_reports={}>{}",
                dashboard.blocked_reports, policy.maximum_blocked_reports
            ));
        }

        if !dashboard.is_empty() && dashboard.clean_rate < policy.minimum_clean_rate {
            watch_reasons.push(format!(
                "memory_reuse_clean_rate={:.3}<{}",
                dashboard.clean_rate, policy.minimum_clean_rate
            ));
        }
        if dashboard.skipped_engine_calls > policy.maximum_skipped_engine_calls {
            watch_reasons.push(format!(
                "memory_reuse_skipped_engine_calls={}>{}",
                dashboard.skipped_engine_calls, policy.maximum_skipped_engine_calls
            ));
        }
        if !dashboard.latest_blocked_reasons.is_empty() {
            watch_reasons.push(format!(
                "memory_reuse_latest_blocked={}",
                dashboard.latest_blocked_reasons.join(";")
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (
                AgentClosedLoopMemoryReusePreflightExecutionHealthStatus::Repair,
                repair_reasons,
            )
        } else if !watch_reasons.is_empty() {
            (
                AgentClosedLoopMemoryReusePreflightExecutionHealthStatus::Watch,
                watch_reasons,
            )
        } else {
            (
                AgentClosedLoopMemoryReusePreflightExecutionHealthStatus::Stable,
                Vec::new(),
            )
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentClosedLoopMemoryReusePreflightExecutionHealthStatus::Stable
    }

    pub fn allows_execution_advance(&self) -> bool {
        self.status != AgentClosedLoopMemoryReusePreflightExecutionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentClosedLoopMemoryReusePreflightExecutionHealthStatus::Repair
    }
}

impl AgentClosedLoopMemoryReusePreflightExecutor {
    pub fn new() -> Self {
        Self {
            executor: AgentClosedLoopPreparedExecutor::new(),
        }
    }

    pub fn with_executor(executor: AgentClosedLoopPreparedExecutor) -> Self {
        Self { executor }
    }

    pub fn execute<E>(
        &self,
        prepared_preflight: AgentClosedLoopPreparedMemoryReusePreflight,
        engine: &mut E,
    ) -> AgentClosedLoopPreparedExecution
    where
        E: EnginePort,
        E::Error: ToString,
    {
        if !prepared_preflight.can_enter_execution() {
            let skipped_reasons = if prepared_preflight.skipped_reasons.is_empty() {
                prepared_preflight
                    .preflight
                    .as_ref()
                    .map(|preflight| preflight.blocked_reasons.clone())
                    .filter(|reasons| !reasons.is_empty())
                    .unwrap_or_else(|| vec!["memory_reuse_preflight_not_executable".to_owned()])
            } else {
                prepared_preflight.skipped_reasons.clone()
            };

            return AgentClosedLoopPreparedExecution {
                prepared_dispatch: prepared_preflight.prepared_dispatch,
                execution: None,
                skipped_reasons,
            };
        }

        self.executor
            .execute(prepared_preflight.prepared_dispatch, engine)
    }

    pub fn execute_with_report<E>(
        &self,
        prepared_preflight: AgentClosedLoopPreparedMemoryReusePreflight,
        engine: &mut E,
    ) -> AgentClosedLoopMemoryReusePreflightExecutionReport
    where
        E: EnginePort,
        E::Error: ToString,
    {
        let preflight = prepared_preflight.preflight.clone();
        let planned_engine_calls = prepared_preflight
            .prepared_dispatch
            .assigned_task_ids()
            .len();
        let preflight_clean = preflight
            .as_ref()
            .is_some_and(AgentMemoryReuseExecutionPreflightReport::is_clean);
        let memory_reuse_ready = preflight
            .as_ref()
            .is_some_and(|preflight| preflight.memory_reuse_ready);
        let can_enter_execution = prepared_preflight.can_enter_execution();

        let execution = self.execute(prepared_preflight, engine);
        let executed_engine_calls = execution.result_count() + execution.failure_count();
        let skipped_engine_calls = planned_engine_calls.saturating_sub(executed_engine_calls);
        let execution_complete = execution.is_complete();
        let mut blocked_reasons = execution.skipped_reasons.clone();
        if blocked_reasons.is_empty() {
            blocked_reasons.extend(
                preflight
                    .as_ref()
                    .map(|preflight| preflight.blocked_reasons.clone())
                    .unwrap_or_default(),
            );
        }
        let telemetry = memory_reuse_preflight_execution_telemetry(
            preflight.is_some(),
            preflight_clean,
            memory_reuse_ready,
            can_enter_execution,
            execution_complete,
            planned_engine_calls,
            executed_engine_calls,
            skipped_engine_calls,
            &blocked_reasons,
        );

        AgentClosedLoopMemoryReusePreflightExecutionReport {
            preflight,
            execution,
            planned_engine_calls,
            executed_engine_calls,
            skipped_engine_calls,
            preflight_clean,
            memory_reuse_ready,
            can_enter_execution,
            execution_complete,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopPreparedCycle {
    pub prepared_execution: AgentClosedLoopPreparedExecution,
    pub report: Option<AgentCycleReport>,
    pub skipped_reasons: Vec<String>,
}

impl AgentClosedLoopPreparedCycle {
    pub fn has_report(&self) -> bool {
        self.report.is_some()
    }

    pub fn execution_failure_count(&self) -> usize {
        self.report
            .as_ref()
            .map(|report| report.execution_failures.len())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct AgentClosedLoopPreparedCycleCloser {
    orchestrator: AgentCycleOrchestrator,
}

impl Default for AgentClosedLoopPreparedCycleCloser {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentClosedLoopPreparedCycleCloser {
    pub fn new() -> Self {
        Self {
            orchestrator: AgentCycleOrchestrator::new(),
        }
    }

    pub fn with_orchestrator(orchestrator: AgentCycleOrchestrator) -> Self {
        Self { orchestrator }
    }

    pub fn close(
        &self,
        prepared_execution: AgentClosedLoopPreparedExecution,
        evidence: AgentCycleEvidence,
    ) -> AgentClosedLoopPreparedCycle {
        let Some(dispatch) = prepared_execution
            .prepared_dispatch
            .dispatch
            .as_ref()
            .cloned()
        else {
            return AgentClosedLoopPreparedCycle {
                skipped_reasons: if prepared_execution.skipped_reasons.is_empty() {
                    vec!["prepared_dispatch_missing".to_owned()]
                } else {
                    prepared_execution.skipped_reasons.clone()
                },
                prepared_execution,
                report: None,
            };
        };

        let Some(execution) = prepared_execution.execution.clone() else {
            return AgentClosedLoopPreparedCycle {
                skipped_reasons: if prepared_execution.skipped_reasons.is_empty() {
                    vec!["wave_execution_missing".to_owned()]
                } else {
                    prepared_execution.skipped_reasons.clone()
                },
                prepared_execution,
                report: None,
            };
        };

        let report = self
            .orchestrator
            .close_execution(dispatch.dispatch, execution, evidence);

        AgentClosedLoopPreparedCycle {
            prepared_execution,
            report: Some(report),
            skipped_reasons: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentClosedLoopStepper {
    report_gate: AgentReportGate,
    loopback_planner: AgentLoopbackPlanner,
    business_controller: AgentBusinessLoopController,
    ledger_policy: AgentCycleLedgerPolicy,
}

impl Default for AgentClosedLoopStepper {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentClosedLoopStepper {
    pub fn new() -> Self {
        Self {
            report_gate: AgentReportGate::new(),
            loopback_planner: AgentLoopbackPlanner::new(),
            business_controller: AgentBusinessLoopController::new(),
            ledger_policy: AgentCycleLedgerPolicy::default(),
        }
    }

    pub fn with_policies(
        report_gate_policy: AgentReportGatePolicy,
        ledger_policy: AgentCycleLedgerPolicy,
    ) -> Self {
        Self {
            report_gate: AgentReportGate::with_policy(report_gate_policy),
            loopback_planner: AgentLoopbackPlanner::new(),
            business_controller: AgentBusinessLoopController::new(),
            ledger_policy,
        }
    }

    pub fn close(
        &self,
        ledger: &AgentCycleLedger,
        input: AgentClosedLoopStepInput,
    ) -> AgentClosedLoopStep {
        let record = AgentCycleLedgerRecord::from_report(
            input.run_id,
            &input.report,
            input.evidence,
            input.memory_submission,
        );
        let report_decision = self.report_gate.evaluate(&record);
        let loopback_plan = self.loopback_planner.plan(&input.handoff, &report_decision);
        let ledger_entry = AgentCycleLedgerEntry::new(
            record.clone(),
            report_decision.clone(),
            loopback_plan.clone(),
        );
        let mut updated_ledger = ledger.clone();
        updated_ledger.append(ledger_entry.clone());
        let business_plan = self
            .business_controller
            .plan(&updated_ledger, self.ledger_policy);

        AgentClosedLoopStep {
            record,
            report_decision,
            loopback_plan,
            ledger_entry,
            updated_ledger,
            business_plan,
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

fn memory_reuse_preflight_execution_telemetry(
    preflight_present: bool,
    preflight_clean: bool,
    memory_reuse_ready: bool,
    can_enter_execution: bool,
    execution_complete: bool,
    planned_engine_calls: usize,
    executed_engine_calls: usize,
    skipped_engine_calls: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_memory_reuse_preflight_execution=true".to_owned(),
        format!("agent_memory_reuse_preflight_present={preflight_present}"),
        format!("agent_memory_reuse_preflight_clean={preflight_clean}"),
        format!("agent_memory_reuse_ready={memory_reuse_ready}"),
        format!("agent_memory_reuse_can_enter_execution={can_enter_execution}"),
        format!("agent_memory_reuse_execution_complete={execution_complete}"),
        format!("agent_memory_reuse_planned_engine_calls={planned_engine_calls}"),
        format!("agent_memory_reuse_executed_engine_calls={executed_engine_calls}"),
        format!("agent_memory_reuse_skipped_engine_calls={skipped_engine_calls}"),
        format!(
            "agent_memory_reuse_preflight_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        blocked_reasons
            .iter()
            .map(|reason| format!("agent_memory_reuse_preflight_blocked_reason={reason}")),
    );
    telemetry
}

fn memory_reuse_preflight_execution_dashboard_telemetry(
    total_reports: usize,
    clean_reports: usize,
    blocked_reports: usize,
    missing_preflight_reports: usize,
    saved_compute_reports: usize,
    planned_engine_calls: usize,
    executed_engine_calls: usize,
    skipped_engine_calls: usize,
) -> Vec<String> {
    vec![
        "agent_memory_reuse_preflight_execution_dashboard=true".to_owned(),
        format!("agent_memory_reuse_preflight_execution_reports={total_reports}"),
        format!("agent_memory_reuse_preflight_execution_clean_reports={clean_reports}"),
        format!("agent_memory_reuse_preflight_execution_blocked_reports={blocked_reports}"),
        format!(
            "agent_memory_reuse_preflight_execution_missing_preflight_reports={missing_preflight_reports}"
        ),
        format!(
            "agent_memory_reuse_preflight_execution_saved_compute_reports={saved_compute_reports}"
        ),
        format!(
            "agent_memory_reuse_preflight_execution_planned_engine_calls={planned_engine_calls}"
        ),
        format!(
            "agent_memory_reuse_preflight_execution_executed_engine_calls={executed_engine_calls}"
        ),
        format!(
            "agent_memory_reuse_preflight_execution_skipped_engine_calls={skipped_engine_calls}"
        ),
    ]
}

fn memory_reuse_preflight_repair_task_plan_telemetry(
    memory_health_status: AgentClosedLoopMemoryReusePreflightExecutionHealthStatus,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    reasons: usize,
    repair_task_ids: &[String],
    next_queue_task_ids: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_memory_reuse_preflight_repair_task_plan=true".to_owned(),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_health_status={}",
            memory_health_status.as_str()
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_memory_reuse_preflight_repair_task_plan_repair_tasks={repair_tasks}"),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_next_queue_tasks={next_queue_tasks}"
        ),
        format!("agent_memory_reuse_preflight_repair_task_plan_reasons={reasons}"),
    ];
    telemetry.extend(repair_task_ids.iter().map(|task_id| {
        format!("agent_memory_reuse_preflight_repair_task_plan_repair_task_id={task_id}")
    }));
    telemetry.extend(next_queue_task_ids.iter().map(|task_id| {
        format!("agent_memory_reuse_preflight_repair_task_plan_next_queue_task_id={task_id}")
    }));
    telemetry
}

fn memory_reuse_preflight_repair_task_plan_summary_telemetry(
    memory_health_status: AgentClosedLoopMemoryReusePreflightExecutionHealthStatus,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    reasons: usize,
    repair_task_ids: &[String],
    next_queue_task_ids: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_memory_reuse_preflight_repair_task_plan_summary=true".to_owned(),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_summary_health_status={}",
            memory_health_status.as_str()
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!("agent_memory_reuse_preflight_repair_task_plan_summary_reasons={reasons}"),
    ];
    telemetry.extend(repair_task_ids.iter().map(|task_id| {
        format!("agent_memory_reuse_preflight_repair_task_plan_summary_repair_task_id={task_id}")
    }));
    telemetry.extend(next_queue_task_ids.iter().map(|task_id| {
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_summary_next_queue_task_id={task_id}"
        )
    }));
    telemetry
}

fn memory_reuse_preflight_repair_task_plan_dashboard_telemetry(
    total_records: usize,
    repair_first_records: usize,
    non_repair_records: usize,
    repair_tasks: usize,
    reasons: usize,
    next_queue_tasks: usize,
    non_repair_rate: f32,
    latest_repair_task_ids: &[String],
    latest_next_queue_task_ids: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_memory_reuse_preflight_repair_task_plan_dashboard=true".to_owned(),
        format!("agent_memory_reuse_preflight_repair_task_plan_dashboard_records={total_records}"),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_dashboard_repair_first_records={repair_first_records}"
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_dashboard_non_repair_records={non_repair_records}"
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_dashboard_repair_tasks={repair_tasks}"
        ),
        format!("agent_memory_reuse_preflight_repair_task_plan_dashboard_reasons={reasons}"),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_dashboard_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_dashboard_non_repair_rate={non_repair_rate:.3}"
        ),
    ];
    telemetry.extend(latest_repair_task_ids.iter().map(|task_id| {
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_dashboard_latest_repair_task_id={task_id}"
        )
    }));
    telemetry.extend(latest_next_queue_task_ids.iter().map(|task_id| {
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_dashboard_latest_next_queue_task_id={task_id}"
        )
    }));
    telemetry
}

fn memory_reuse_preflight_repair_task_plan_history_record_telemetry(
    dashboard: &AgentClosedLoopMemoryReusePreflightRepairTaskPlanDashboard,
    health: &AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealth,
) -> Vec<String> {
    vec![
        "agent_memory_reuse_preflight_repair_task_plan_history_record=true".to_owned(),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_history_record_repair_first_records={}",
            dashboard.repair_first_records
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_history_record_repair_tasks={}",
            dashboard.repair_tasks
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_history_record_requires_repair_first={}",
            health.requires_repair_first()
        ),
        format!(
            "agent_memory_reuse_preflight_repair_task_plan_history_record_allows_service_advance={}",
            health.allows_service_advance()
        ),
    ]
}

fn memory_reuse_preflight_repair_tasks(
    memory_health: &AgentClosedLoopMemoryReusePreflightExecutionHealth,
) -> Vec<AgentTask> {
    if !memory_health.requires_repair_first() {
        return Vec::new();
    }

    let reasons = if memory_health.reasons.is_empty() {
        vec!["memory_reuse_preflight_health_requires_repair".to_owned()]
    } else {
        memory_health.reasons.clone()
    };

    reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!("memory-reuse-preflight-repair-{index}"),
                AgentRole::MemoryCurator,
                format!(
                    "repair memory reuse preflight sidecar/evidence before execution: {reason}"
                ),
                AgentBudget::new(12, 1, 1),
            )
            .with_lane("memory-reuse-preflight-repair")
            .with_priority(9)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::AggregationReport;
    use crate::budget::AgentBudget;
    use crate::conflict::ConflictReport;
    use crate::cycle::{AgentCycleSummary, MemoryPromotion};
    use crate::evolution::{ProcessRewardComponents, ProcessRewardReport, RewardAction};
    use crate::ledger::{AgentCycleLedgerAdmissionStatus, AgentCycleLedgerEntry};
    use crate::message::{AgentMessage, AgentMessageKind};
    use crate::ports::{EnginePort, MemoryNote};
    use crate::run::{
        AgentRunLedgerAdmission, AgentRunReport, RunBudgetAudit, SideEffectGate, SideEffectKind,
    };
    use crate::service::{AgentServiceCommandPlanner, AgentServiceCommandReceipt};
    use crate::task::{AgentRole, AgentTask, TaskDispatchPlan};

    fn report(
        reward_total: f32,
        action: RewardAction,
        memory_promotions: usize,
    ) -> AgentCycleReport {
        let memory_promotions = (0..memory_promotions)
            .map(|index| MemoryPromotion {
                note: MemoryNote::new("agent_cycle", format!("note {index}")),
                reason: "clean reinforced loop".to_owned(),
            })
            .collect::<Vec<_>>();

        AgentCycleReport {
            dispatch: TaskDispatchPlan::default(),
            execution_failures: Vec::new(),
            run_ledger_admission: AgentRunLedgerAdmission {
                can_build_ledger: true,
                can_admit_side_effects: true,
                can_submit_memory_note: true,
                can_promote_adaptive_state: true,
                requires_repair_first: false,
                reasons: Vec::new(),
                telemetry: Vec::new(),
            },
            run_report: AgentRunReport {
                aggregation: AggregationReport::default(),
                conflicts: ConflictReport::default(),
                budget_audit: RunBudgetAudit::default(),
                side_effects: vec![SideEffectGate::allow(SideEffectKind::MemoryNote, "ok")],
            },
            reward_report: ProcessRewardReport {
                total: reward_total,
                components: ProcessRewardComponents::default(),
                action,
                notes: Vec::new(),
                evolution_signals: Vec::new(),
            },
            tool_build_report: None,
            follow_up_tasks: Vec::new(),
            memory_promotions,
        }
    }

    fn clean_evidence() -> AgentReportEvidence {
        AgentReportEvidence::new(true, true)
            .with_validation_ref("eval:validation:pass")
            .with_runtime_ref("service:runtime:ok")
    }

    fn clean_handoff() -> AgentCycleHandoff {
        AgentCycleHandoff {
            memory_notes: vec![MemoryNote::new("agent_cycle", "remember clean loop")],
            follow_up_tasks: Vec::new(),
            blocked_reasons: Vec::new(),
        }
    }

    fn clean_submission() -> MemorySubmissionReport {
        MemorySubmissionReport {
            submitted: vec![MemoryNote::new("agent_cycle", "remember clean loop")],
            failures: Vec::new(),
            blocked_reasons: Vec::new(),
        }
    }

    fn blocked_entry(run_id: &str) -> AgentCycleLedgerEntry {
        let repair = AgentTask::new(
            format!("repair-{run_id}"),
            AgentRole::Reviewer,
            "repair loop",
            AgentBudget::new(8, 1, 1),
        );
        AgentCycleLedgerEntry::new(
            AgentCycleLedgerRecord::new(
                run_id,
                AgentCycleSummary {
                    assigned_tasks: 1,
                    rejected_tasks: 0,
                    unique_messages: 1,
                    duplicate_groups: 0,
                    unresolved_conflicts: 0,
                    blocked_side_effects: 0,
                    budget_overspends: 0,
                    execution_failures: 1,
                    reward_total: 0.30,
                    reward_action: RewardAction::Penalize,
                    evolution_signals: 0,
                    follow_up_tasks: 1,
                    memory_promotions: 0,
                    tool_build_reports: 0,
                    tool_build_missing_requests: 0,
                    tool_build_unexpected_receipts: 0,
                    tool_build_duplicate_receipts: 0,
                    tool_build_held_receipts: 0,
                    tool_build_rejected_receipts: 0,
                },
                AgentReportEvidence::default(),
                None,
            ),
            AgentReportGateDecision {
                accepted: false,
                reasons: vec![crate::eval::AgentReportGateReason::new(
                    "execution_failures",
                    "1",
                )],
                follow_up_tasks: vec![repair.clone()],
            },
            AgentLoopbackPlan {
                promote_adaptive_state: false,
                enqueue_tasks: vec![repair],
                blocked_reasons: vec!["execution_failures=1".to_owned()],
            },
        )
    }

    #[test]
    fn closed_loop_step_accepts_clean_cycle_and_updates_business_plan() {
        let input = AgentClosedLoopStepInput::new(
            "run-1",
            report(0.92, RewardAction::Reinforce, 1),
            clean_handoff(),
            clean_evidence(),
            Some(clean_submission()),
        );

        let step = AgentClosedLoopStepper::new().close(&AgentCycleLedger::new(), input);

        assert!(step.report_decision.is_accepted());
        assert!(step.loopback_plan.promote_adaptive_state);
        assert_eq!(step.updated_ledger.len(), 1);
        assert_eq!(
            step.business_plan.status(),
            AgentCycleLedgerAdmissionStatus::Promote
        );
        assert!(step.business_plan.can_promote_adaptive_state());
        assert_eq!(
            step.business_plan.adaptive_state_candidate.unwrap().run_id,
            "run-1"
        );
    }

    #[test]
    fn closed_loop_step_turns_memory_failure_into_repair_queue() {
        let input = AgentClosedLoopStepInput::new(
            "run-3",
            report(0.91, RewardAction::Reinforce, 1),
            clean_handoff(),
            clean_evidence(),
            Some(MemorySubmissionReport {
                submitted: Vec::new(),
                failures: vec![crate::memory::MemorySubmissionFailure {
                    note: MemoryNote::new("agent_cycle", "remember clean loop"),
                    reason: "memory rejected note".to_owned(),
                }],
                blocked_reasons: Vec::new(),
            }),
        );
        let ledger =
            AgentCycleLedger::from_entries(vec![blocked_entry("run-1"), blocked_entry("run-2")]);

        let step = AgentClosedLoopStepper::new().close(&ledger, input);

        assert!(!step.report_decision.is_accepted());
        assert!(!step.loopback_plan.promote_adaptive_state);
        assert_eq!(
            step.business_plan.status(),
            AgentCycleLedgerAdmissionStatus::Repair
        );
        assert_eq!(
            step.business_plan.next_queue.task_ids(),
            vec!["report-gate-run-3-memory"]
        );
        assert!(
            step.business_plan
                .telemetry
                .iter()
                .any(|line| line == "reason=consecutive_blocked_cycles=3")
        );
    }

    #[test]
    fn closed_loop_execution_report_accepts_clean_service_receipts() {
        let input = AgentClosedLoopStepInput::new(
            "run-12",
            report(0.94, RewardAction::Reinforce, 1),
            clean_handoff(),
            clean_evidence(),
            Some(clean_submission()),
        );
        let step = AgentClosedLoopStepper::new().close(&AgentCycleLedger::new(), input);
        let command_plan = AgentServiceCommandPlanner::new().plan(&step.business_plan);
        let receipts = command_plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();

        let execution = step.close_service_execution(receipts);

        assert!(execution.is_clean());
        assert_eq!(execution.step.record.run_id, "run-12");
        assert!(execution.next_queue().is_empty());
        let command_kinds = execution.service_report.command_plan.command_kinds();
        assert_eq!(command_kinds[0], "promote_adaptive_state");
        assert!(
            command_kinds[1..]
                .iter()
                .all(|kind| *kind == "emit_telemetry")
        );

        let service_summary = execution.service_summary();
        let summary = execution.summary();

        assert!(summary.clean);
        assert!(summary.report_accepted);
        assert!(summary.loopback_promoted);
        assert!(summary.service_clean);
        assert_eq!(summary.service_clean, service_summary.clean);
        assert_eq!(summary.command_count, service_summary.command_count);
        assert_eq!(summary.run_id, "run-12");
        assert_eq!(
            summary.admission_status,
            AgentCycleLedgerAdmissionStatus::Promote
        );
        assert_eq!(
            summary.missing_command_count,
            service_summary.missing_commands
        );
        assert_eq!(summary.next_queue_tasks, service_summary.next_queue_tasks);
        assert!(summary.blocked_reasons.is_empty());
    }

    #[test]
    fn closed_loop_execution_report_turns_missing_service_receipts_into_queue() {
        let input = AgentClosedLoopStepInput::new(
            "run-13",
            report(0.94, RewardAction::Reinforce, 1),
            clean_handoff(),
            clean_evidence(),
            Some(clean_submission()),
        );
        let step = AgentClosedLoopStepper::new().close(&AgentCycleLedger::new(), input);

        let mut execution = step.close_service_execution(Vec::new());
        let service_summary = execution.service_summary();
        let summary = execution.summary();
        let ready_ids = execution
            .service_report
            .turnover
            .next_queue
            .drain_ready(&std::collections::BTreeSet::new())
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>();

        assert!(!execution.is_clean());
        assert_eq!(
            execution.service_report.turnover.blocked_reasons[0],
            "service_command_missing=promote_adaptive_state"
        );
        assert!(
            execution.service_report.turnover.blocked_reasons[1..]
                .iter()
                .all(|reason| reason == "service_command_missing=emit_telemetry")
        );
        assert_eq!(
            ready_ids[0],
            "service-feedback-run-13-0-promote_adaptive_state"
        );
        assert!(
            ready_ids[1..]
                .iter()
                .all(|task_id| task_id.ends_with("-emit_telemetry"))
        );

        assert!(!summary.clean);
        assert!(summary.report_accepted);
        assert!(summary.loopback_promoted);
        assert!(!summary.service_clean);
        assert_eq!(summary.run_id, "run-13");
        assert_eq!(
            summary.admission_status,
            AgentCycleLedgerAdmissionStatus::Promote
        );
        assert_eq!(
            summary.failed_command_count,
            service_summary.failed_commands
        );
        assert_eq!(
            summary.skipped_command_count,
            service_summary.skipped_commands
        );
        assert_eq!(
            summary.missing_command_count,
            service_summary.missing_commands
        );
        assert_eq!(summary.command_count, service_summary.command_count);
        assert_eq!(summary.next_queue_tasks, service_summary.next_queue_tasks);
        assert_eq!(
            summary.blocked_reasons[0],
            "service_command_missing=promote_adaptive_state"
        );
    }

    fn execution_summary(
        run_id: &str,
        clean: bool,
        report_accepted: bool,
        loopback_promoted: bool,
        service_clean: bool,
        reward_total: f32,
        admission_status: AgentCycleLedgerAdmissionStatus,
        command_counts: (usize, usize, usize, usize),
        next_queue_tasks: usize,
        blocked_reasons: Vec<&str>,
    ) -> AgentClosedLoopExecutionSummary {
        AgentClosedLoopExecutionSummary {
            run_id: run_id.to_owned(),
            clean,
            report_accepted,
            loopback_promoted,
            service_clean,
            reward_total,
            admission_status,
            command_count: command_counts.0,
            missing_command_count: command_counts.1,
            failed_command_count: command_counts.2,
            skipped_command_count: command_counts.3,
            next_queue_tasks,
            next_queue_task_ids: (0..next_queue_tasks)
                .map(|index| format!("{run_id}-task-{index}"))
                .collect(),
            blocked_reasons: blocked_reasons
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        }
    }

    fn memory_reuse_preflight_execution_summary(
        clean: bool,
        preflight_present: bool,
        memory_reuse_ready: bool,
        can_enter_execution: bool,
        execution_complete: bool,
        planned_engine_calls: usize,
        executed_engine_calls: usize,
        skipped_engine_calls: usize,
        blocked_reasons: Vec<&str>,
    ) -> AgentClosedLoopMemoryReusePreflightExecutionSummary {
        AgentClosedLoopMemoryReusePreflightExecutionSummary {
            clean,
            preflight_present,
            preflight_clean: clean,
            memory_reuse_ready,
            can_enter_execution,
            execution_complete,
            saved_compute: skipped_engine_calls > 0,
            planned_engine_calls,
            executed_engine_calls,
            skipped_engine_calls,
            blocked_reasons: blocked_reasons
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        }
    }

    #[test]
    fn closed_loop_execution_history_summarizes_dashboard_pressure() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![
            execution_summary(
                "run-1",
                true,
                true,
                true,
                true,
                0.92,
                AgentCycleLedgerAdmissionStatus::Promote,
                (3, 0, 0, 0),
                0,
                Vec::new(),
            ),
            execution_summary(
                "run-2",
                false,
                false,
                false,
                true,
                0.48,
                AgentCycleLedgerAdmissionStatus::Repair,
                (2, 0, 0, 0),
                2,
                vec!["unresolved_conflicts=1"],
            ),
            execution_summary(
                "run-3",
                false,
                true,
                true,
                false,
                0.81,
                AgentCycleLedgerAdmissionStatus::Hold,
                (4, 1, 1, 1),
                3,
                vec!["service_command_failed=emit_telemetry:writer offline"],
            ),
        ]);

        let dashboard = history.dashboard();

        assert_eq!(history.len(), 3);
        assert_eq!(history.latest().unwrap().run_id, "run-3");
        assert_eq!(dashboard.total_runs, 3);
        assert_eq!(dashboard.clean_runs, 1);
        assert_eq!(dashboard.dirty_runs, 2);
        assert!((dashboard.clean_rate - 0.333).abs() < 0.01);
        assert_eq!(dashboard.report_blocked_runs, 1);
        assert_eq!(dashboard.loopback_blocked_runs, 1);
        assert_eq!(dashboard.service_dirty_runs, 1);
        assert_eq!(dashboard.promote_admissions, 1);
        assert_eq!(dashboard.hold_admissions, 1);
        assert_eq!(dashboard.repair_admissions, 1);
        assert_eq!(dashboard.command_count, 9);
        assert_eq!(dashboard.missing_command_count, 1);
        assert_eq!(dashboard.failed_command_count, 1);
        assert_eq!(dashboard.skipped_command_count, 1);
        assert!((dashboard.service_failure_pressure - 0.333).abs() < 0.01);
        assert_eq!(dashboard.total_next_queue_tasks, 5);
        assert!((dashboard.average_reward_total - 0.736).abs() < 0.01);
        assert_eq!(dashboard.latest_run_id.as_deref(), Some("run-3"));
        assert_eq!(
            dashboard.latest_blocked_reasons,
            vec!["service_command_failed=emit_telemetry:writer offline"]
        );
        assert!(!dashboard.is_service_clean());
    }

    #[test]
    fn memory_reuse_preflight_execution_history_marks_clean_reports_stable() {
        let history = AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
            memory_reuse_preflight_execution_summary(
                true,
                true,
                true,
                true,
                true,
                1,
                1,
                0,
                Vec::new(),
            ),
            memory_reuse_preflight_execution_summary(
                true,
                true,
                true,
                true,
                true,
                2,
                2,
                0,
                Vec::new(),
            ),
        ]);

        let dashboard = history.dashboard();
        let health =
            dashboard.health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());

        assert_eq!(history.len(), 2);
        assert_eq!(dashboard.total_reports, 2);
        assert_eq!(dashboard.clean_reports, 2);
        assert_eq!(dashboard.blocked_reports, 0);
        assert_eq!(dashboard.missing_preflight_reports, 0);
        assert_eq!(dashboard.saved_compute_reports, 0);
        assert_eq!(dashboard.planned_engine_calls, 3);
        assert_eq!(dashboard.executed_engine_calls, 3);
        assert_eq!(dashboard.skipped_engine_calls, 0);
        assert_eq!(dashboard.clean_rate, 1.0);
        assert_eq!(dashboard.skipped_engine_call_rate, 0.0);
        assert!(health.is_stable());
        assert_eq!(
            health.status.as_str(),
            AgentClosedLoopMemoryReusePreflightExecutionHealthStatus::Stable.as_str()
        );
        assert!(health.reasons.is_empty());
        assert!(health.allows_execution_advance());
        assert!(!health.requires_repair_first());
        assert!(
            dashboard
                .telemetry
                .iter()
                .any(|line| line == "agent_memory_reuse_preflight_execution_clean_reports=2")
        );
    }

    #[test]
    fn memory_reuse_preflight_execution_history_repairs_missing_or_blocked_preflight() {
        let history = AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
            memory_reuse_preflight_execution_summary(
                true,
                true,
                true,
                true,
                true,
                1,
                1,
                0,
                Vec::new(),
            ),
            memory_reuse_preflight_execution_summary(
                false,
                false,
                false,
                false,
                false,
                1,
                0,
                1,
                vec!["memory_reuse_preflight_not_executable"],
            ),
        ]);

        let dashboard = history.dashboard();
        let health =
            history.health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());

        assert_eq!(dashboard.total_reports, 2);
        assert_eq!(dashboard.clean_reports, 1);
        assert_eq!(dashboard.blocked_reports, 1);
        assert_eq!(dashboard.missing_preflight_reports, 1);
        assert_eq!(dashboard.saved_compute_reports, 1);
        assert_eq!(dashboard.planned_engine_calls, 2);
        assert_eq!(dashboard.executed_engine_calls, 1);
        assert_eq!(dashboard.skipped_engine_calls, 1);
        assert!((dashboard.clean_rate - 0.5).abs() < 0.01);
        assert!((dashboard.skipped_engine_call_rate - 0.5).abs() < 0.01);
        assert_eq!(
            dashboard.latest_blocked_reasons,
            vec!["memory_reuse_preflight_not_executable"]
        );
        assert_eq!(
            health.status,
            AgentClosedLoopMemoryReusePreflightExecutionHealthStatus::Repair
        );
        assert!(!health.allows_execution_advance());
        assert!(health.requires_repair_first());
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "memory_reuse_missing_preflight_reports=1>0")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "memory_reuse_blocked_reports=1>0")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "memory_reuse_skipped_engine_calls=1>0")
        );
        assert!(
            history
                .latest()
                .unwrap()
                .summary_line()
                .contains("saved_compute=true")
        );
    }

    #[test]
    fn closed_loop_execution_history_handles_empty_dashboard() {
        let history = AgentClosedLoopExecutionHistory::new();

        let dashboard = history.dashboard();

        assert!(history.is_empty());
        assert_eq!(history.summaries(), &[]);
        assert!(dashboard.is_empty());
        assert!(dashboard.is_service_clean());
        assert_eq!(dashboard.clean_rate, 0.0);
        assert_eq!(dashboard.service_failure_pressure, 0.0);
        assert_eq!(dashboard.average_reward_total, 0.0);
        assert_eq!(dashboard.latest_run_id, None);
        assert!(dashboard.latest_blocked_reasons.is_empty());
    }

    #[test]
    fn closed_loop_execution_health_marks_clean_history_stable() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![
            execution_summary(
                "run-1",
                true,
                true,
                true,
                true,
                0.88,
                AgentCycleLedgerAdmissionStatus::Promote,
                (2, 0, 0, 0),
                0,
                Vec::new(),
            ),
            execution_summary(
                "run-2",
                true,
                true,
                true,
                true,
                0.91,
                AgentCycleLedgerAdmissionStatus::Promote,
                (2, 0, 0, 0),
                0,
                Vec::new(),
            ),
        ]);

        let health = history.health(AgentClosedLoopExecutionHealthPolicy::default());

        assert!(health.is_stable());
        assert_eq!(
            health.status.as_str(),
            AgentClosedLoopExecutionHealthStatus::Stable.as_str()
        );
        assert!(health.reasons.is_empty());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(health.dashboard.total_runs, 2);
    }

    #[test]
    fn closed_loop_execution_health_watches_low_clean_rate_without_service_drift() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![
            execution_summary(
                "run-1",
                true,
                true,
                true,
                true,
                0.87,
                AgentCycleLedgerAdmissionStatus::Promote,
                (2, 0, 0, 0),
                0,
                Vec::new(),
            ),
            execution_summary(
                "run-2",
                false,
                false,
                false,
                true,
                0.52,
                AgentCycleLedgerAdmissionStatus::Hold,
                (2, 0, 0, 0),
                1,
                vec!["runtime_evidence_missing=true"],
            ),
        ]);

        let health = history.health(AgentClosedLoopExecutionHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "clean_rate=0.500<0.67")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "report_blocked_runs=1>0")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "loopback_blocked_runs=1>0")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "latest_blocked=runtime_evidence_missing=true")
        );
    }

    #[test]
    fn closed_loop_execution_health_repairs_service_pressure_and_repair_admissions() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            false,
            true,
            true,
            false,
            0.72,
            AgentCycleLedgerAdmissionStatus::Repair,
            (4, 1, 1, 0),
            2,
            vec!["service_command_failed=enqueue_tasks:queue offline"],
        )]);

        let health = history.health(AgentClosedLoopExecutionHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Repair);
        assert!(!health.allows_service_advance());
        assert!(health.requires_repair_first());
        assert_eq!(health.reasons[0], "service_failure_pressure=0.500>0");
        assert_eq!(health.reasons[1], "repair_admissions=1>0");
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "clean_rate=0.000<0.67")
        );
        assert!(
            health.reasons.iter().any(|reason| reason
                == "latest_blocked=service_command_failed=enqueue_tasks:queue offline")
        );
    }

    fn next_queue() -> AgentTaskQueue {
        AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "next-turn",
            AgentRole::Planner,
            "schedule the next agent turn",
            AgentBudget::new(8, 1, 1),
        )])
    }

    #[derive(Debug, Clone)]
    struct CountingEngine {
        calls: usize,
        fail: bool,
    }

    impl EnginePort for CountingEngine {
        type Error = String;

        fn run_task(&mut self, task: &AgentTask) -> Result<crate::task::AgentResult, Self::Error> {
            self.calls += 1;
            if self.fail {
                return Err(format!("engine failed {}", task.id));
            }
            Ok(crate::task::AgentResult::accepted(
                task,
                format!("ran {}", task.id),
                vec![AgentMessage::new(
                    format!("message-{}", task.id),
                    task.role.clone(),
                    AgentMessageKind::Status,
                    "engine",
                    "runtime ok",
                )],
                AgentBudget::new(1, 1, 1),
            ))
        }
    }

    #[test]
    fn closed_loop_next_turn_plan_continues_stable_non_empty_queue() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);

        let plan = history.next_turn_plan(
            next_queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Continue);
        assert!(plan.can_schedule());
        assert!(plan.allows_adaptive_evolution());
        assert!(!plan.requires_repair_first());
        assert!(plan.allows_service_advance());
        assert!(plan.reasons.is_empty());
        assert_eq!(plan.next_queue.task_ids(), vec!["next-turn"]);
        assert_eq!(plan.telemetry[0], "next_turn_mode=continue");
        assert_eq!(plan.telemetry[1], "health_status=stable");
    }

    #[test]
    fn closed_loop_next_turn_plan_observes_watch_health_without_adaptive_evolution() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![
            execution_summary(
                "run-1",
                true,
                true,
                true,
                true,
                0.90,
                AgentCycleLedgerAdmissionStatus::Promote,
                (2, 0, 0, 0),
                0,
                Vec::new(),
            ),
            execution_summary(
                "run-2",
                false,
                false,
                false,
                true,
                0.50,
                AgentCycleLedgerAdmissionStatus::Hold,
                (2, 0, 0, 0),
                1,
                vec!["validation_evidence_missing=true"],
            ),
        ]);

        let plan = history.next_turn_plan(
            next_queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Observe);
        assert!(plan.can_schedule());
        assert!(!plan.allows_adaptive_evolution());
        assert!(!plan.requires_repair_first());
        assert!(plan.allows_service_advance());
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason == "clean_rate=0.500<0.67")
        );
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "reason=report_blocked_runs=1>0")
        );
    }

    #[test]
    fn closed_loop_next_turn_plan_repairs_service_drift_first() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            false,
            true,
            true,
            false,
            0.74,
            AgentCycleLedgerAdmissionStatus::Repair,
            (4, 1, 0, 0),
            1,
            vec!["service_command_missing=enqueue_tasks"],
        )]);

        let plan = history.next_turn_plan(
            next_queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Repair);
        assert!(plan.can_schedule());
        assert!(!plan.allows_adaptive_evolution());
        assert!(plan.requires_repair_first());
        assert!(!plan.allows_service_advance());
        assert_eq!(
            plan.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(plan.telemetry[0], "next_turn_mode=repair");
    }

    #[test]
    fn closed_loop_next_turn_plan_idles_when_queue_is_empty() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            0,
            Vec::new(),
        )]);

        let plan = history.next_turn_plan(
            AgentTaskQueue::new(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Idle);
        assert!(!plan.can_schedule());
        assert!(!plan.allows_adaptive_evolution());
        assert!(!plan.requires_repair_first());
        assert!(plan.allows_service_advance());
        assert_eq!(plan.reasons, vec!["next_queue_empty"]);
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "reason=next_queue_empty")
        );
    }

    #[test]
    fn closed_loop_next_turn_plan_keeps_continue_with_stable_memory_reuse_preflight_health() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    true,
                    true,
                    true,
                    true,
                    true,
                    1,
                    1,
                    0,
                    Vec::new(),
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());

        let plan = history
            .next_turn_plan(
                next_queue(),
                AgentClosedLoopExecutionHealthPolicy::default(),
            )
            .with_memory_reuse_preflight_health(memory_health);

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Continue);
        assert!(plan.can_schedule());
        assert!(plan.allows_adaptive_evolution());
        assert!(plan.reasons.is_empty());
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "next_turn_mode=continue")
        );
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "memory_reuse_preflight_health_status=stable")
        );
    }

    #[test]
    fn closed_loop_next_turn_plan_adds_no_memory_reuse_preflight_repair_task_when_stable() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    true,
                    true,
                    true,
                    true,
                    true,
                    1,
                    1,
                    0,
                    Vec::new(),
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());

        let plan = history
            .next_turn_plan(
                next_queue(),
                AgentClosedLoopExecutionHealthPolicy::default(),
            )
            .with_memory_reuse_preflight_repair_tasks(memory_health);

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Continue);
        assert_eq!(plan.next_queue.task_ids(), vec!["next-turn"]);
        assert!(plan.reasons.is_empty());
        assert!(
            !plan
                .telemetry
                .iter()
                .any(|line| line.starts_with("memory_reuse_preflight_repair_tasks="))
        );
    }

    #[test]
    fn closed_loop_memory_reuse_preflight_repair_task_plan_summarizes_stable_health() {
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    true,
                    true,
                    true,
                    true,
                    true,
                    1,
                    1,
                    0,
                    Vec::new(),
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());

        let repair_task_plan =
            AgentClosedLoopMemoryReusePreflightRepairTaskPlan::from_health_and_queue(
                memory_health,
                next_queue(),
            );
        let summary = repair_task_plan.summary();

        assert_eq!(
            summary.memory_health_status,
            AgentClosedLoopMemoryReusePreflightExecutionHealthStatus::Stable
        );
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_tasks, 1);
        assert!(summary.repair_task_ids.is_empty());
        assert_eq!(summary.next_queue_task_ids, vec!["next-turn"]);
        assert_eq!(repair_task_plan.repair_tasks, Vec::<AgentTask>::new());
        assert_eq!(repair_task_plan.next_queue.task_ids(), vec!["next-turn"]);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_memory_reuse_preflight_repair_task_plan_summary_repair_tasks=0"
        }));
    }

    #[test]
    fn closed_loop_next_turn_plan_observes_memory_reuse_preflight_watch_health() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    true,
                    true,
                    true,
                    true,
                    true,
                    1,
                    1,
                    0,
                    Vec::new(),
                ),
                memory_reuse_preflight_execution_summary(
                    false,
                    true,
                    true,
                    true,
                    false,
                    1,
                    1,
                    0,
                    Vec::new(),
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());

        let plan = history
            .next_turn_plan(
                next_queue(),
                AgentClosedLoopExecutionHealthPolicy::default(),
            )
            .with_memory_reuse_preflight_health(memory_health);

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Observe);
        assert!(plan.can_schedule());
        assert!(!plan.allows_adaptive_evolution());
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason == "memory_reuse_preflight:memory_reuse_clean_rate=0.500<1")
        );
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "next_turn_mode=observe")
        );
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "memory_reuse_preflight_mode_override=continue->observe")
        );
    }

    #[test]
    fn closed_loop_next_turn_plan_repairs_memory_reuse_preflight_health_first() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    false,
                    false,
                    false,
                    false,
                    false,
                    1,
                    0,
                    1,
                    vec!["memory_reuse_preflight_not_executable"],
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());

        let plan = history
            .next_turn_plan(
                next_queue(),
                AgentClosedLoopExecutionHealthPolicy::default(),
            )
            .with_memory_reuse_preflight_health(memory_health);

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Repair);
        assert!(plan.can_schedule());
        assert!(!plan.allows_adaptive_evolution());
        assert!(plan.requires_repair_first());
        assert!(!plan.allows_service_advance());
        assert!(
            plan.reasons.iter().any(|reason| reason
                == "memory_reuse_preflight:memory_reuse_missing_preflight_reports=1>0")
        );
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "next_turn_mode=repair")
        );
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "memory_reuse_preflight_mode_override=continue->repair")
        );
    }

    #[test]
    fn closed_loop_next_turn_plan_enqueues_memory_reuse_preflight_repair_tasks_first() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    false,
                    false,
                    false,
                    false,
                    false,
                    1,
                    0,
                    1,
                    vec!["memory_reuse_preflight_not_executable"],
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());
        let repair_task_count = memory_health.reasons.len();

        let plan = history
            .next_turn_plan(
                next_queue(),
                AgentClosedLoopExecutionHealthPolicy::default(),
            )
            .with_memory_reuse_preflight_repair_tasks(memory_health);

        let tasks = plan.next_queue.tasks();
        let repair_task_ids = (0..repair_task_count)
            .map(|index| format!("memory-reuse-preflight-repair-{index}"))
            .collect::<Vec<_>>();
        let next_turn = tasks
            .iter()
            .find(|task| task.id == "next-turn")
            .expect("next-turn task");
        let first_repair = tasks
            .iter()
            .find(|task| task.id == "memory-reuse-preflight-repair-0")
            .expect("memory preflight repair task");

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Repair);
        assert_eq!(plan.next_queue.len(), repair_task_count + 1);
        assert_eq!(
            plan.next_queue.task_ids()[..repair_task_count],
            repair_task_ids
        );
        assert_eq!(next_turn.dependencies, repair_task_ids);
        assert_eq!(first_repair.role, AgentRole::MemoryCurator);
        assert_eq!(first_repair.lane, "memory-reuse-preflight-repair");
        assert_eq!(first_repair.priority, 9);
        assert!(!first_repair.required_budget.is_zero());
        assert_eq!(first_repair.dependencies, Vec::<String>::new());
        assert_eq!(
            plan.next_queue
                .ready_tasks(&BTreeSet::new())
                .into_iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            repair_task_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );
        assert!(
            plan.reasons.iter().any(|reason| reason
                == &format!("memory_reuse_preflight:repair_tasks={repair_task_count}"))
        );
        assert!(plan.telemetry.iter().any(|line| {
            line == &format!("memory_reuse_preflight_repair_tasks={repair_task_count}")
        }));
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == &format!("next_queue_tasks={}", repair_task_count + 1))
        );
        assert!(plan.telemetry.iter().any(|line| {
            line == "memory_reuse_preflight_repair_task_id=memory-reuse-preflight-repair-0"
        }));
        assert!(plan.telemetry.iter().any(|line| {
            line == "agent_memory_reuse_preflight_repair_task_plan_repair_task_id=memory-reuse-preflight-repair-0"
        }));
    }

    #[test]
    fn closed_loop_memory_reuse_preflight_repair_task_plan_summarizes_repair_queue() {
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    false,
                    false,
                    false,
                    false,
                    false,
                    1,
                    0,
                    1,
                    vec!["memory_reuse_preflight_not_executable"],
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());
        let repair_task_count = memory_health.reasons.len();
        let repair_task_ids = (0..repair_task_count)
            .map(|index| format!("memory-reuse-preflight-repair-{index}"))
            .collect::<Vec<_>>();

        let repair_task_plan =
            AgentClosedLoopMemoryReusePreflightRepairTaskPlan::from_health_and_queue(
                memory_health,
                next_queue(),
            );
        let summary = repair_task_plan.summary();

        assert_eq!(
            summary.memory_health_status,
            AgentClosedLoopMemoryReusePreflightExecutionHealthStatus::Repair
        );
        assert!(summary.requires_repair_first);
        assert_eq!(summary.repair_tasks, repair_task_count);
        assert_eq!(summary.reasons, repair_task_count);
        assert_eq!(summary.repair_task_ids, repair_task_ids);
        assert_eq!(
            &summary.next_queue_task_ids[..repair_task_count],
            summary.repair_task_ids.as_slice()
        );
        assert_eq!(
            summary.next_queue_task_ids.last().map(String::as_str),
            Some("next-turn")
        );
        assert_eq!(
            repair_task_plan.repair_tasks[0].role,
            AgentRole::MemoryCurator
        );
        assert!(
            repair_task_plan
                .next_queue
                .tasks()
                .iter()
                .find(|task| task.id == "next-turn")
                .expect("next-turn task")
                .dependencies
                .iter()
                .all(|dependency| dependency.starts_with("memory-reuse-preflight-repair-"))
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == &format!(
                "agent_memory_reuse_preflight_repair_task_plan_summary_repair_tasks={repair_task_count}"
            )
        }));
    }

    #[test]
    fn memory_reuse_preflight_repair_task_plan_history_watches_empty_history() {
        let health = AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory::new()
            .health(AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy::default());

        assert_eq!(
            health.status,
            AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus::Watch
        );
        assert_eq!(
            health.reasons,
            vec!["memory_reuse_preflight_repair_task_plan_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert_eq!(health.dashboard.non_repair_rate, 0.0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(health.dashboard.telemetry.iter().any(|line| {
            line == "agent_memory_reuse_preflight_repair_task_plan_dashboard_records=0"
        }));
    }

    #[test]
    fn memory_reuse_preflight_repair_task_plan_history_marks_non_repair_history_stable() {
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    true,
                    true,
                    true,
                    true,
                    true,
                    1,
                    1,
                    0,
                    Vec::new(),
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());
        let summary = AgentClosedLoopMemoryReusePreflightRepairTaskPlan::from_health_and_queue(
            memory_health,
            next_queue(),
        )
        .summary();

        let history =
            AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory::from_summaries(vec![
                summary,
            ]);
        let dashboard = history.dashboard();
        let health = dashboard
            .health(AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy::default());

        assert_eq!(history.len(), 1);
        assert_eq!(
            history.latest().unwrap().next_queue_task_ids,
            vec!["next-turn"]
        );
        assert_eq!(history.summaries().len(), 1);
        assert_eq!(dashboard.total_records, 1);
        assert_eq!(dashboard.repair_first_records, 0);
        assert_eq!(dashboard.non_repair_records, 1);
        assert_eq!(dashboard.repair_tasks, 0);
        assert_eq!(dashboard.reasons, 0);
        assert_eq!(dashboard.next_queue_tasks, 1);
        assert_eq!(dashboard.non_repair_rate, 1.0);
        assert!(dashboard.latest_repair_task_ids.is_empty());
        assert_eq!(dashboard.latest_next_queue_task_ids, vec!["next-turn"]);
        assert!(health.is_stable());
        assert!(health.reasons.is_empty());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(dashboard.telemetry.iter().any(|line| line
            == "agent_memory_reuse_preflight_repair_task_plan_dashboard_non_repair_rate=1.000"));
    }

    #[test]
    fn memory_reuse_preflight_repair_task_plan_history_repairs_repeated_repair_pressure() {
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    false,
                    false,
                    false,
                    false,
                    false,
                    1,
                    0,
                    1,
                    vec!["memory_reuse_preflight_not_executable"],
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());
        let repair_task_count = memory_health.reasons.len();
        let plan = AgentClosedLoopMemoryReusePreflightRepairTaskPlan::from_health_and_queue(
            memory_health,
            next_queue(),
        );

        let record = AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecorder::new()
            .record_plan_with_health(
                AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory::new(),
                &plan,
                AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.repair_tasks, repair_task_count);
        assert_eq!(record.dashboard.reasons, repair_task_count);
        assert_eq!(
            record.dashboard.latest_repair_task_ids[0],
            "memory-reuse-preflight-repair-0"
        );
        assert_eq!(
            record.health.status,
            AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus::Repair
        );
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(record.health.reasons.iter().any(|reason| {
            reason == "memory_reuse_preflight_repair_task_plan_repair_first_records=1>0"
        }));
        assert!(record.health.reasons.iter().any(|reason| {
            reason
                == &format!(
                    "memory_reuse_preflight_repair_task_plan_repair_tasks={repair_task_count}>0"
                )
        }));
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_memory_reuse_preflight_repair_task_plan_history_record_status=repair"
        }));
    }

    #[test]
    fn memory_reuse_preflight_repair_task_plan_recorder_appends_summary_with_health() {
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    true,
                    true,
                    true,
                    true,
                    true,
                    1,
                    1,
                    0,
                    Vec::new(),
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());
        let plan = AgentClosedLoopMemoryReusePreflightRepairTaskPlan::from_health_and_queue(
            memory_health,
            next_queue(),
        );

        let record = AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecorder::new()
            .record_plan_with_health(
                AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory::new(),
                &plan,
                AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.history.latest(), Some(&record.appended_summary));
        assert_eq!(record.appended_summary.next_queue_tasks, 1);
        assert_eq!(record.dashboard.total_records, 1);
        assert_eq!(
            record.health.status.as_str(),
            AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthStatus::Stable.as_str()
        );
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_memory_reuse_preflight_repair_task_plan_history_record_records=1"
        }));
    }

    #[test]
    fn closed_loop_next_turn_plan_keeps_continue_with_stable_memory_reuse_preflight_repair_task_plan_health()
     {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    true,
                    true,
                    true,
                    true,
                    true,
                    1,
                    1,
                    0,
                    Vec::new(),
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());
        let repair_task_plan =
            AgentClosedLoopMemoryReusePreflightRepairTaskPlan::from_health_and_queue(
                memory_health,
                next_queue(),
            );
        let repair_health =
            AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecorder::new()
                .record_plan_with_health(
                    AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory::new(),
                    &repair_task_plan,
                    AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy::default(),
                )
                .health;

        let plan = history
            .next_turn_plan(
                next_queue(),
                AgentClosedLoopExecutionHealthPolicy::default(),
            )
            .with_memory_reuse_preflight_repair_task_plan_health(repair_health);

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Continue);
        assert!(plan.can_schedule());
        assert!(plan.allows_adaptive_evolution());
        assert!(plan.allows_service_advance());
        assert!(!plan.requires_repair_first());
        assert!(plan.reasons.is_empty());
        assert!(plan.telemetry.iter().any(|line| {
            line == "memory_reuse_preflight_repair_task_plan_health_status=stable"
        }));
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "memory_reuse_preflight_repair_task_plan_records=1")
        );
        assert!(plan.telemetry.iter().any(|line| {
            line == "memory_reuse_preflight_repair_task_plan_non_repair_rate=1.000"
        }));
        assert!(!plan.telemetry.iter().any(|line| {
            line.starts_with("memory_reuse_preflight_repair_task_plan_mode_override=")
        }));
    }

    #[test]
    fn closed_loop_next_turn_plan_observes_memory_reuse_preflight_repair_task_plan_watch_health() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let repair_health = AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory::new()
            .health(AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy::default());

        let plan = history
            .next_turn_plan(
                next_queue(),
                AgentClosedLoopExecutionHealthPolicy::default(),
            )
            .with_memory_reuse_preflight_repair_task_plan_health(repair_health);

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Observe);
        assert!(plan.can_schedule());
        assert!(!plan.allows_adaptive_evolution());
        assert!(plan.allows_service_advance());
        assert!(!plan.requires_repair_first());
        assert!(plan.reasons.iter().any(|reason| reason
            == "memory_reuse_preflight_repair_task_plan:memory_reuse_preflight_repair_task_plan_history_empty"));
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "next_turn_mode=observe")
        );
        assert!(
            plan.telemetry.iter().any(|line| {
                line == "memory_reuse_preflight_repair_task_plan_health_status=watch"
            })
        );
        assert!(plan.telemetry.iter().any(|line| {
            line == "memory_reuse_preflight_repair_task_plan_mode_override=continue->observe"
        }));
    }

    #[test]
    fn closed_loop_next_turn_plan_repairs_memory_reuse_preflight_repair_task_plan_health_first() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    false,
                    false,
                    false,
                    false,
                    false,
                    1,
                    0,
                    1,
                    vec!["memory_reuse_preflight_not_executable"],
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());
        let repair_task_plan =
            AgentClosedLoopMemoryReusePreflightRepairTaskPlan::from_health_and_queue(
                memory_health,
                next_queue(),
            );
        let repair_task_count = repair_task_plan.repair_tasks.len();
        let repair_health =
            AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecorder::new()
                .record_plan_with_health(
                    AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory::new(),
                    &repair_task_plan,
                    AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy::default(),
                )
                .health;

        let plan = history
            .next_turn_plan(
                next_queue(),
                AgentClosedLoopExecutionHealthPolicy::default(),
            )
            .with_memory_reuse_preflight_repair_task_plan_health(repair_health);

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Repair);
        assert!(plan.can_schedule());
        assert!(!plan.allows_adaptive_evolution());
        assert!(!plan.allows_service_advance());
        assert!(plan.requires_repair_first());
        assert!(plan.reasons.iter().any(|reason| reason
            == "memory_reuse_preflight_repair_task_plan:memory_reuse_preflight_repair_task_plan_repair_first_records=1>0"));
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "next_turn_mode=repair")
        );
        assert!(plan.telemetry.iter().any(|line| {
            line == "memory_reuse_preflight_repair_task_plan_health_status=repair"
        }));
        assert!(plan.telemetry.iter().any(|line| {
            line == &format!(
                "memory_reuse_preflight_repair_task_plan_repair_tasks={repair_task_count}"
            )
        }));
        assert!(plan.telemetry.iter().any(|line| {
            line == "memory_reuse_preflight_repair_task_plan_mode_override=continue->repair"
        }));
    }

    #[test]
    fn closed_loop_dispatch_preparer_dispatches_memory_reuse_preflight_repairs_before_next_turn() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    false,
                    false,
                    false,
                    false,
                    false,
                    1,
                    0,
                    1,
                    vec!["memory_reuse_preflight_not_executable"],
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());
        let repair_task_count = memory_health.reasons.len();
        let repair_task_ids = (0..repair_task_count)
            .map(|index| format!("memory-reuse-preflight-repair-{index}"))
            .collect::<Vec<_>>();
        let turn_plan = history
            .next_turn_plan(
                next_queue(),
                AgentClosedLoopExecutionHealthPolicy::default(),
            )
            .with_memory_reuse_preflight_repair_tasks(memory_health);
        let budget = BudgetLedger::new()
            .with_budget(AgentRole::MemoryCurator, AgentBudget::new(64, 8, 8))
            .with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1));

        let prepared = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            repair_task_count + 1,
        );

        assert!(prepared.can_dispatch());
        assert_eq!(prepared.assigned_task_ids(), repair_task_ids);
        assert!(prepared.skipped_reasons.is_empty());
        let dispatch = prepared.dispatch.expect("dispatch");
        assert_eq!(dispatch.wave.task_ids, repair_task_ids);
        assert_eq!(dispatch.remaining_queue.task_ids(), vec!["next-turn"]);
        assert_eq!(
            dispatch.remaining_queue.tasks()[0].dependencies,
            repair_task_ids
        );
    }

    #[test]
    fn closed_loop_dispatch_preparer_blocks_repair_plan_health_without_repair_task() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let memory_health =
            AgentClosedLoopMemoryReusePreflightExecutionHistory::from_summaries(vec![
                memory_reuse_preflight_execution_summary(
                    false,
                    false,
                    false,
                    false,
                    false,
                    1,
                    0,
                    1,
                    vec!["memory_reuse_preflight_not_executable"],
                ),
            ])
            .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());
        let repair_task_plan =
            AgentClosedLoopMemoryReusePreflightRepairTaskPlan::from_health_and_queue(
                memory_health,
                next_queue(),
            );
        let repair_health =
            AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistoryRecorder::new()
                .record_plan_with_health(
                    AgentClosedLoopMemoryReusePreflightRepairTaskPlanSummaryHistory::new(),
                    &repair_task_plan,
                    AgentClosedLoopMemoryReusePreflightRepairTaskPlanHealthPolicy::default(),
                )
                .health;
        let turn_plan = history
            .next_turn_plan(
                next_queue(),
                AgentClosedLoopExecutionHealthPolicy::default(),
            )
            .with_memory_reuse_preflight_repair_task_plan_health(repair_health);
        let budget = BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1));

        let prepared = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );

        assert_eq!(prepared.turn_plan.mode, AgentClosedLoopNextTurnMode::Repair);
        assert!(!prepared.can_dispatch());
        assert_eq!(prepared.assigned_task_ids(), vec!["next-turn"]);
        assert_eq!(prepared.skipped_reasons, vec!["repair_first_task_missing"]);
        let dispatch = prepared.dispatch.expect("dispatch audit");
        assert_eq!(dispatch.dispatch.assignments.len(), 1);
        assert_eq!(dispatch.dispatch.assignments[0].lane, "default");
        assert!(dispatch.dispatch.rejections.is_empty());
    }

    #[test]
    fn closed_loop_dispatch_preparer_plans_next_wave_from_continue_turn() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let turn_plan = history.next_turn_plan(
            next_queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );
        let budget = BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1));

        let prepared = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );

        assert!(prepared.can_dispatch());
        assert_eq!(prepared.assigned_task_ids(), vec!["next-turn"]);
        assert!(prepared.skipped_reasons.is_empty());
        let dispatch = prepared.dispatch.unwrap();
        assert_eq!(dispatch.wave.task_ids, vec!["next-turn"]);
        assert_eq!(dispatch.dispatch.assignments.len(), 1);
        assert!(dispatch.remaining_queue.is_empty());
    }

    #[test]
    fn closed_loop_dispatch_preparer_skips_idle_turn_without_budget_debit() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            0,
            Vec::new(),
        )]);
        let turn_plan = history.next_turn_plan(
            AgentTaskQueue::new(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );
        let budget =
            BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(100, 2, 2));

        let prepared = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );

        assert!(!prepared.can_dispatch());
        assert_eq!(prepared.dispatch, None);
        assert_eq!(prepared.skipped_reasons, vec!["next_queue_empty"]);
    }

    #[test]
    fn closed_loop_dispatch_preparer_preserves_budget_rejection_audit() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let turn_plan = history.next_turn_plan(
            next_queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );
        let budget = BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(1, 1, 1));

        let prepared = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );

        assert!(!prepared.can_dispatch());
        assert_eq!(prepared.assigned_task_ids(), Vec::<String>::new());
        assert_eq!(prepared.skipped_reasons, vec!["dispatch_empty"]);
        let dispatch = prepared.dispatch.unwrap();
        assert_eq!(dispatch.dispatch.assignments.len(), 0);
        assert_eq!(dispatch.dispatch.rejections.len(), 1);
        assert_eq!(dispatch.dispatch.rejections[0].task_id, "next-turn");
        assert!(
            dispatch.dispatch.rejections[0]
                .reason
                .contains("insufficient budget")
        );
    }

    #[test]
    fn closed_loop_prepared_memory_reuse_preflight_reports_sidecar_without_engine_call() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let turn_plan = history.next_turn_plan(
            next_queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );
        let budget = BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1));
        let prepared_dispatch = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );
        let task = prepared_dispatch.dispatch.as_ref().unwrap().assigned_tasks[0].clone();
        let evidence = MemoryRecallDryRunEvidence {
            source: "norion_memory_reuse_dry_run".to_owned(),
            read_only: true,
            candidate_count: 2,
            long_term_match_count: 2,
            context_decision_count: 2,
            accepted_context_count: 1,
            rejected_context_count: 1,
            used_tokens: 72,
            requested_kv_count: 3,
            kv_promote_count: 1,
            kv_missing_count: 1,
            kv_already_hot_count: 1,
            kv_duplicate_count: 0,
            kv_backend_available: true,
            memory_store_write_allowed: false,
            kv_prefetch_apply_allowed: false,
            reason_codes: vec!["read_only".to_owned(), "context_accepted".to_owned()],
            detail_codes: vec!["kv_prefetch:promote:6e657874".to_owned()],
        };
        let recall_context = crate::memory::MemoryRecallContextPlanner::new()
            .plan_from_dry_run_evidence(&task, &evidence);
        let recall_plan = AgentWaveMemoryRecallPlan {
            contexts: vec![recall_context],
            telemetry: Vec::new(),
        };
        let engine = CountingEngine {
            calls: 0,
            fail: false,
        };

        let prepared_preflight = AgentClosedLoopPreparedMemoryReusePreflightPlanner::new().plan(
            prepared_dispatch,
            &recall_plan,
            &[evidence],
        );

        assert_eq!(engine.calls, 0);
        assert!(prepared_preflight.has_preflight());
        assert!(prepared_preflight.can_enter_execution());
        assert!(!prepared_preflight.requires_repair_first());
        assert!(prepared_preflight.skipped_reasons.is_empty());
        let report = prepared_preflight.preflight.unwrap();
        assert!(report.read_only);
        assert!(report.memory_reuse_ready);
        assert!(report.can_enter_execution);
        assert!(!report.prompt_injection_allowed);
        assert!(!report.engine_port_touched);
        assert!(!report.memory_store_write_allowed);
        assert!(!report.kv_prefetch_apply_allowed);
        assert_eq!(report.task_ids, vec!["next-turn"]);
        assert_eq!(report.recall_task_ids, vec!["next-turn"]);
        assert_eq!(report.dry_run_evidence_count, 1);
        assert_eq!(report.kv_requested_count, 3);
        assert!(report.blocked_reasons.is_empty());
    }

    #[test]
    fn closed_loop_memory_reuse_preflight_executor_delegates_clean_preflight_to_engine() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let turn_plan = history.next_turn_plan(
            next_queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );
        let budget = BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1));
        let prepared_dispatch = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );
        let task = prepared_dispatch.dispatch.as_ref().unwrap().assigned_tasks[0].clone();
        let evidence = MemoryRecallDryRunEvidence {
            source: "norion_memory_reuse_dry_run".to_owned(),
            read_only: true,
            candidate_count: 1,
            long_term_match_count: 1,
            context_decision_count: 1,
            accepted_context_count: 1,
            rejected_context_count: 0,
            used_tokens: 48,
            requested_kv_count: 2,
            kv_promote_count: 1,
            kv_missing_count: 0,
            kv_already_hot_count: 1,
            kv_duplicate_count: 0,
            kv_backend_available: true,
            memory_store_write_allowed: false,
            kv_prefetch_apply_allowed: false,
            reason_codes: vec!["read_only".to_owned()],
            detail_codes: vec!["kv_prefetch:promote:6e657874".to_owned()],
        };
        let recall_context = crate::memory::MemoryRecallContextPlanner::new()
            .plan_from_dry_run_evidence(&task, &evidence);
        let recall_plan = AgentWaveMemoryRecallPlan {
            contexts: vec![recall_context],
            telemetry: Vec::new(),
        };
        let prepared_preflight = AgentClosedLoopPreparedMemoryReusePreflightPlanner::new().plan(
            prepared_dispatch,
            &recall_plan,
            &[evidence],
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };

        let report = AgentClosedLoopMemoryReusePreflightExecutor::new()
            .execute_with_report(prepared_preflight, &mut engine);
        let prepared_execution = &report.execution;

        assert_eq!(engine.calls, 1);
        assert!(prepared_execution.has_execution());
        assert!(prepared_execution.is_complete());
        assert_eq!(prepared_execution.result_count(), 1);
        assert_eq!(prepared_execution.failure_count(), 0);
        assert!(prepared_execution.skipped_reasons.is_empty());
        assert!(report.is_clean());
        assert!(!report.saved_compute());
        assert!(report.preflight.is_some());
        assert_eq!(report.planned_engine_calls, 1);
        assert_eq!(report.executed_engine_calls, 1);
        assert_eq!(report.skipped_engine_calls, 0);
        assert!(report.blocked_reasons.is_empty());
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| line == "agent_memory_reuse_skipped_engine_calls=0")
        );
        assert!(report.summary_line().contains("execution_complete=true"));
        let mut history = AgentClosedLoopMemoryReusePreflightExecutionHistory::new();
        history.record_report(&report);
        assert_eq!(history.latest().unwrap().executed_engine_calls, 1);
        assert!(
            history
                .health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default())
                .is_stable()
        );
    }

    #[test]
    fn closed_loop_memory_reuse_preflight_executor_skips_blocked_preflight_without_engine_call() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let turn_plan = history.next_turn_plan(
            next_queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );
        let budget = BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1));
        let prepared_dispatch = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );
        let recall_plan = AgentWaveMemoryRecallPlan {
            contexts: Vec::new(),
            telemetry: Vec::new(),
        };
        let prepared_preflight = AgentClosedLoopPreparedMemoryReusePreflightPlanner::new().plan(
            prepared_dispatch,
            &recall_plan,
            &[],
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };

        let report = AgentClosedLoopMemoryReusePreflightExecutor::new()
            .execute_with_report(prepared_preflight, &mut engine);
        let prepared_execution = &report.execution;

        assert_eq!(engine.calls, 0);
        assert!(!prepared_execution.has_execution());
        assert!(!prepared_execution.is_complete());
        assert_eq!(prepared_execution.result_count(), 0);
        assert_eq!(prepared_execution.failure_count(), 0);
        assert!(
            prepared_execution
                .skipped_reasons
                .contains(&"memory_reuse_dry_run_evidence_missing".to_owned())
        );
        assert!(
            prepared_execution
                .skipped_reasons
                .contains(&"memory_reuse_recall_missing_task=next-turn".to_owned())
        );
        assert!(!report.is_clean());
        assert!(report.saved_compute());
        assert!(report.preflight.is_some());
        assert_eq!(report.planned_engine_calls, 1);
        assert_eq!(report.executed_engine_calls, 0);
        assert_eq!(report.skipped_engine_calls, 1);
        assert!(
            report
                .blocked_reasons
                .contains(&"memory_reuse_dry_run_evidence_missing".to_owned())
        );
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| line == "agent_memory_reuse_skipped_engine_calls=1")
        );
        assert!(report.summary_line().contains("skipped_engine_calls=1"));
        let mut history = AgentClosedLoopMemoryReusePreflightExecutionHistory::new();
        history.record_report(&report);
        let health =
            history.health(AgentClosedLoopMemoryReusePreflightExecutionHealthPolicy::default());
        assert_eq!(
            health.status,
            AgentClosedLoopMemoryReusePreflightExecutionHealthStatus::Repair
        );
    }

    #[test]
    fn closed_loop_prepared_executor_runs_dispatch_through_engine_port() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let turn_plan = history.next_turn_plan(
            next_queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );
        let budget = BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1));
        let prepared_dispatch = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };

        let prepared_execution =
            AgentClosedLoopPreparedExecutor::new().execute(prepared_dispatch, &mut engine);

        assert_eq!(engine.calls, 1);
        assert!(prepared_execution.has_execution());
        assert!(prepared_execution.is_complete());
        assert_eq!(prepared_execution.result_count(), 1);
        assert_eq!(prepared_execution.failure_count(), 0);
        assert!(prepared_execution.skipped_reasons.is_empty());
    }

    #[test]
    fn closed_loop_prepared_executor_skips_non_dispatchable_turn_without_engine_call() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            0,
            Vec::new(),
        )]);
        let turn_plan = history.next_turn_plan(
            AgentTaskQueue::new(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );
        let budget = BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1));
        let prepared_dispatch = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };

        let prepared_execution =
            AgentClosedLoopPreparedExecutor::new().execute(prepared_dispatch, &mut engine);

        assert_eq!(engine.calls, 0);
        assert!(!prepared_execution.has_execution());
        assert!(!prepared_execution.is_complete());
        assert_eq!(prepared_execution.result_count(), 0);
        assert_eq!(prepared_execution.failure_count(), 0);
        assert_eq!(prepared_execution.skipped_reasons, vec!["next_queue_empty"]);
    }

    #[test]
    fn closed_loop_prepared_executor_preserves_engine_failures_as_data() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let turn_plan = history.next_turn_plan(
            next_queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );
        let budget = BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1));
        let prepared_dispatch = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: true,
        };

        let prepared_execution =
            AgentClosedLoopPreparedExecutor::new().execute(prepared_dispatch, &mut engine);

        assert_eq!(engine.calls, 1);
        assert!(prepared_execution.has_execution());
        assert!(!prepared_execution.is_complete());
        assert_eq!(prepared_execution.result_count(), 0);
        assert_eq!(prepared_execution.failure_count(), 1);
        assert_eq!(
            prepared_execution.execution.unwrap().failures[0].reason,
            "engine failed next-turn"
        );
    }

    #[test]
    fn closed_loop_prepared_cycle_closer_turns_execution_into_cycle_report() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            1,
            Vec::new(),
        )]);
        let turn_plan = history.next_turn_plan(
            next_queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );
        let budget = BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1));
        let prepared_dispatch = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let prepared_execution =
            AgentClosedLoopPreparedExecutor::new().execute(prepared_dispatch, &mut engine);

        let prepared_cycle = AgentClosedLoopPreparedCycleCloser::new().close(
            prepared_execution,
            AgentCycleEvidence {
                quality: 0.80,
                validation_passed: true,
                runtime_response_ok: true,
                reflection: None,
                conflict_resolutions: crate::conflict::ConflictResolutionBook::new(),
                toolsmith_plan: crate::evolution::ToolsmithPlan::new(),
                tool_build_report: None,
            },
        );

        assert!(prepared_cycle.has_report());
        assert_eq!(prepared_cycle.execution_failure_count(), 0);
        assert!(prepared_cycle.skipped_reasons.is_empty());
        let report = prepared_cycle.report.unwrap();
        assert_eq!(report.dispatch.assignments.len(), 1);
        assert_eq!(report.run_report.aggregation.unique_count, 1);
    }

    #[test]
    fn closed_loop_prepared_cycle_closer_skips_missing_execution_without_report() {
        let history = AgentClosedLoopExecutionHistory::from_summaries(vec![execution_summary(
            "run-1",
            true,
            true,
            true,
            true,
            0.90,
            AgentCycleLedgerAdmissionStatus::Promote,
            (2, 0, 0, 0),
            0,
            Vec::new(),
        )]);
        let turn_plan = history.next_turn_plan(
            AgentTaskQueue::new(),
            AgentClosedLoopExecutionHealthPolicy::default(),
        );
        let budget = BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1));
        let prepared_dispatch = AgentClosedLoopDispatchPreparer::new().prepare(
            turn_plan,
            &BTreeSet::new(),
            budget,
            &BudgetPolicy::strict(),
            2,
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let prepared_execution =
            AgentClosedLoopPreparedExecutor::new().execute(prepared_dispatch, &mut engine);

        let prepared_cycle = AgentClosedLoopPreparedCycleCloser::new()
            .close(prepared_execution, AgentCycleEvidence::default());

        assert_eq!(engine.calls, 0);
        assert!(!prepared_cycle.has_report());
        assert_eq!(prepared_cycle.execution_failure_count(), 0);
        assert_eq!(prepared_cycle.skipped_reasons, vec!["next_queue_empty"]);
    }
}
