use std::collections::{BTreeMap, BTreeSet};

use crate::aggregate::{AggregationReport, MessageAggregator};
use crate::budget::AgentBudget;
use crate::conflict::{ConflictReport, ConflictResolutionBook, ConflictResolver};
use crate::message::AgentMessage;
use crate::reflection::ReflectionLoop;
use crate::task::{
    AgentResult, AgentRole, AgentTask, AgentTaskQueue, TaskDispatchGateDecision, TaskDispatchPlan,
    TaskDispatchPlanSummary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SideEffectKind {
    MemoryNote,
    FileWrite,
    AdaptiveStateWrite,
    ExternalCall,
}

impl SideEffectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MemoryNote => "memory_note",
            Self::FileWrite => "file_write",
            Self::AdaptiveStateWrite => "adaptive_state_write",
            Self::ExternalCall => "external_call",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideEffectGate {
    pub kind: SideEffectKind,
    pub allowed: bool,
    pub reason: String,
}

impl SideEffectGate {
    pub fn allow(kind: SideEffectKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            allowed: true,
            reason: reason.into(),
        }
    }

    pub fn block(kind: SideEffectKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            allowed: false,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReport {
    pub aggregation: AggregationReport,
    pub conflicts: ConflictReport,
    pub budget_audit: RunBudgetAudit,
    pub side_effects: Vec<SideEffectGate>,
}

impl AgentRunReport {
    pub fn summary(&self) -> AgentRunReportSummary {
        AgentRunReportSummary::from_report(self)
    }

    pub fn gate(&self) -> AgentRunGateDecision {
        AgentRunGateDecision::from_report(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportSummary {
    pub input_messages: usize,
    pub unique_messages: usize,
    pub duplicate_groups: usize,
    pub unresolved_conflicts: usize,
    pub budget_overspends: usize,
    pub side_effects: usize,
    pub allowed_side_effects: usize,
    pub blocked_side_effects: usize,
    pub memory_note_allowed: bool,
    pub adaptive_state_allowed: bool,
    pub external_call_allowed: bool,
    pub all_side_effects_allowed: bool,
    pub telemetry: Vec<String>,
}

impl AgentRunReportSummary {
    pub fn from_report(report: &AgentRunReport) -> Self {
        let input_messages = report.aggregation.input_count;
        let unique_messages = report.aggregation.unique_count;
        let duplicate_groups = report.aggregation.duplicate_groups;
        let unresolved_conflicts = report.conflicts.unresolved_count();
        let budget_overspends = report.budget_audit.overspend_count();
        let side_effects = report.side_effects.len();
        let allowed_side_effects = report
            .side_effects
            .iter()
            .filter(|gate| gate.allowed)
            .count();
        let blocked_side_effects = side_effects.saturating_sub(allowed_side_effects);
        let memory_note_allowed =
            side_effect_allowed(&report.side_effects, SideEffectKind::MemoryNote);
        let adaptive_state_allowed =
            side_effect_allowed(&report.side_effects, SideEffectKind::AdaptiveStateWrite);
        let external_call_allowed =
            side_effect_allowed(&report.side_effects, SideEffectKind::ExternalCall);
        let all_side_effects_allowed = side_effects > 0 && blocked_side_effects == 0;
        let telemetry = agent_run_report_summary_telemetry(
            input_messages,
            unique_messages,
            duplicate_groups,
            unresolved_conflicts,
            budget_overspends,
            side_effects,
            allowed_side_effects,
            blocked_side_effects,
            memory_note_allowed,
            adaptive_state_allowed,
            external_call_allowed,
            all_side_effects_allowed,
        );

        Self {
            input_messages,
            unique_messages,
            duplicate_groups,
            unresolved_conflicts,
            budget_overspends,
            side_effects,
            allowed_side_effects,
            blocked_side_effects,
            memory_note_allowed,
            adaptive_state_allowed,
            external_call_allowed,
            all_side_effects_allowed,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentRunReportSummaryHistory {
    summaries: Vec<AgentRunReportSummary>,
}

impl AgentRunReportSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentRunReportSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentRunReportSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentRunReportSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentRunReportSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentRunReportDashboard {
        AgentRunReportDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AgentRunReportHealthPolicy) -> AgentRunReportHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportDashboard {
    pub total_runs: usize,
    pub clean_runs: usize,
    pub conflict_runs: usize,
    pub budget_overspend_runs: usize,
    pub side_effect_blocked_runs: usize,
    pub memory_note_admitted_runs: usize,
    pub adaptive_state_admitted_runs: usize,
    pub external_call_admitted_runs: usize,
    pub total_unresolved_conflicts: usize,
    pub total_budget_overspends: usize,
    pub total_blocked_side_effects: usize,
    pub clean_rate: f32,
    pub memory_note_admission_rate: f32,
    pub adaptive_state_admission_rate: f32,
    pub external_call_admission_rate: f32,
    pub latest_all_side_effects_allowed: Option<bool>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportDashboard {
    pub fn from_summaries(summaries: &[AgentRunReportSummary]) -> Self {
        let total_runs = summaries.len();
        let clean_runs = summaries
            .iter()
            .filter(|summary| {
                summary.unresolved_conflicts == 0
                    && summary.budget_overspends == 0
                    && summary.blocked_side_effects == 0
                    && summary.all_side_effects_allowed
            })
            .count();
        let conflict_runs = summaries
            .iter()
            .filter(|summary| summary.unresolved_conflicts > 0)
            .count();
        let budget_overspend_runs = summaries
            .iter()
            .filter(|summary| summary.budget_overspends > 0)
            .count();
        let side_effect_blocked_runs = summaries
            .iter()
            .filter(|summary| summary.blocked_side_effects > 0)
            .count();
        let memory_note_admitted_runs = summaries
            .iter()
            .filter(|summary| summary.memory_note_allowed)
            .count();
        let adaptive_state_admitted_runs = summaries
            .iter()
            .filter(|summary| summary.adaptive_state_allowed)
            .count();
        let external_call_admitted_runs = summaries
            .iter()
            .filter(|summary| summary.external_call_allowed)
            .count();
        let total_unresolved_conflicts = summaries
            .iter()
            .map(|summary| summary.unresolved_conflicts)
            .sum::<usize>();
        let total_budget_overspends = summaries
            .iter()
            .map(|summary| summary.budget_overspends)
            .sum::<usize>();
        let total_blocked_side_effects = summaries
            .iter()
            .map(|summary| summary.blocked_side_effects)
            .sum::<usize>();
        let clean_rate = rate(clean_runs, total_runs);
        let memory_note_admission_rate = rate(memory_note_admitted_runs, total_runs);
        let adaptive_state_admission_rate = rate(adaptive_state_admitted_runs, total_runs);
        let external_call_admission_rate = rate(external_call_admitted_runs, total_runs);
        let latest_all_side_effects_allowed = summaries
            .last()
            .map(|summary| summary.all_side_effects_allowed);
        let telemetry = agent_run_report_dashboard_telemetry(
            total_runs,
            clean_runs,
            conflict_runs,
            budget_overspend_runs,
            side_effect_blocked_runs,
            memory_note_admitted_runs,
            adaptive_state_admitted_runs,
            external_call_admitted_runs,
            total_unresolved_conflicts,
            total_budget_overspends,
            total_blocked_side_effects,
            clean_rate,
            memory_note_admission_rate,
            adaptive_state_admission_rate,
            external_call_admission_rate,
            latest_all_side_effects_allowed,
        );

        Self {
            total_runs,
            clean_runs,
            conflict_runs,
            budget_overspend_runs,
            side_effect_blocked_runs,
            memory_note_admitted_runs,
            adaptive_state_admitted_runs,
            external_call_admitted_runs,
            total_unresolved_conflicts,
            total_budget_overspends,
            total_blocked_side_effects,
            clean_rate,
            memory_note_admission_rate,
            adaptive_state_admission_rate,
            external_call_admission_rate,
            latest_all_side_effects_allowed,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_runs == 0
    }

    pub fn health(&self, policy: AgentRunReportHealthPolicy) -> AgentRunReportHealth {
        AgentRunReportHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentRunReportHealthPolicy {
    pub minimum_clean_rate: f32,
    pub minimum_memory_note_admission_rate: f32,
    pub minimum_adaptive_state_admission_rate: f32,
    pub maximum_conflict_runs: usize,
    pub maximum_budget_overspend_runs: usize,
    pub maximum_side_effect_blocked_runs: usize,
}

impl Default for AgentRunReportHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_clean_rate: 0.67,
            minimum_memory_note_admission_rate: 0.67,
            minimum_adaptive_state_admission_rate: 0.67,
            maximum_conflict_runs: 0,
            maximum_budget_overspend_runs: 0,
            maximum_side_effect_blocked_runs: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealth {
    pub status: AgentRunReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentRunReportDashboard,
}

impl AgentRunReportHealth {
    pub fn from_dashboard(
        dashboard: AgentRunReportDashboard,
        policy: AgentRunReportHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("agent_run_report_history_empty".to_owned());
        } else if dashboard.clean_rate < policy.minimum_clean_rate {
            watch_reasons.push(format!(
                "agent_run_report_clean_rate={:.3}<{}",
                dashboard.clean_rate, policy.minimum_clean_rate
            ));
        }

        if !dashboard.is_empty()
            && dashboard.memory_note_admission_rate < policy.minimum_memory_note_admission_rate
        {
            watch_reasons.push(format!(
                "agent_run_report_memory_note_admission_rate={:.3}<{}",
                dashboard.memory_note_admission_rate, policy.minimum_memory_note_admission_rate
            ));
        }

        if !dashboard.is_empty()
            && dashboard.adaptive_state_admission_rate
                < policy.minimum_adaptive_state_admission_rate
        {
            watch_reasons.push(format!(
                "agent_run_report_adaptive_state_admission_rate={:.3}<{}",
                dashboard.adaptive_state_admission_rate,
                policy.minimum_adaptive_state_admission_rate
            ));
        }

        if dashboard.conflict_runs > policy.maximum_conflict_runs {
            repair_reasons.push(format!(
                "agent_run_report_conflict_runs={}>{}",
                dashboard.conflict_runs, policy.maximum_conflict_runs
            ));
        }

        if dashboard.budget_overspend_runs > policy.maximum_budget_overspend_runs {
            repair_reasons.push(format!(
                "agent_run_report_budget_overspend_runs={}>{}",
                dashboard.budget_overspend_runs, policy.maximum_budget_overspend_runs
            ));
        }

        if dashboard.side_effect_blocked_runs > policy.maximum_side_effect_blocked_runs {
            repair_reasons.push(format!(
                "agent_run_report_side_effect_blocked_runs={}>{}",
                dashboard.side_effect_blocked_runs, policy.maximum_side_effect_blocked_runs
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentRunReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentRunReportHealthStatus::Watch, watch_reasons)
        } else {
            (AgentRunReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentRunReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Repair
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRunReportHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AgentRunReportHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportSummaryHistoryRecord {
    pub history: AgentRunReportSummaryHistory,
    pub appended_summary: AgentRunReportSummary,
    pub dashboard: AgentRunReportDashboard,
    pub health: AgentRunReportHealth,
    pub telemetry: Vec<String>,
}

impl AgentRunReportSummaryHistoryRecord {
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
pub struct AgentRunReportSummaryHistoryRecorder;

impl AgentRunReportSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentRunReportSummaryHistory,
        summary: AgentRunReportSummary,
        policy: AgentRunReportHealthPolicy,
    ) -> AgentRunReportSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = agent_run_report_history_record_telemetry(&dashboard, &health);

        AgentRunReportSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_report_with_health(
        &self,
        history: AgentRunReportSummaryHistory,
        report: &AgentRunReport,
        policy: AgentRunReportHealthPolicy,
    ) -> AgentRunReportSummaryHistoryRecord {
        self.record_summary_with_health(history, report.summary(), policy)
    }

    pub fn record_summary_with_health_gate(
        &self,
        history: AgentRunReportSummaryHistory,
        summary: AgentRunReportSummary,
        policy: AgentRunReportHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentRunReportHealthGateRecord {
        let health_record = self.record_summary_with_health(history, summary, policy);
        let gate_decision =
            AgentRunReportHealthGate::new().evaluate(run_id, &health_record.health, next_queue);
        let gate_summary = gate_decision.summary();
        let telemetry =
            agent_run_report_health_gate_record_telemetry(&health_record, &gate_summary);

        AgentRunReportHealthGateRecord {
            health_record,
            gate_decision,
            gate_summary,
            telemetry,
        }
    }

    pub fn record_report_with_health_gate(
        &self,
        history: AgentRunReportSummaryHistory,
        report: &AgentRunReport,
        policy: AgentRunReportHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentRunReportHealthGateRecord {
        self.record_summary_with_health_gate(history, report.summary(), policy, run_id, next_queue)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateRecord {
    pub health_record: AgentRunReportSummaryHistoryRecord,
    pub gate_decision: AgentRunReportHealthGateDecision,
    pub gate_summary: AgentRunReportHealthGateSummary,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunProgressReportGateRecord {
    pub progress_record: AgentRunLedgerProgressSummaryHistoryRecord,
    pub report_gate_record: AgentRunReportHealthGateRecord,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub progress_repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunProgressReportGateRecord {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.next_queue.clone()
    }

    pub fn summary(&self) -> AgentRunProgressReportGateSummary {
        AgentRunProgressReportGateSummary::from_record(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunProgressReportGateSummary {
    pub progress_health_status: AgentRunReportHealthStatus,
    pub report_health_status: AgentRunReportHealthStatus,
    pub requested_admitted: bool,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub progress_repair_tasks: usize,
    pub report_repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub progress_repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunProgressReportGateSummary {
    pub fn from_record(record: &AgentRunProgressReportGateRecord) -> Self {
        let progress_repair_task_ids = record
            .progress_repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.next_queue.task_ids();
        let report_repair_tasks = record.report_gate_record.gate_decision.repair_tasks.len();
        let telemetry = agent_run_progress_report_gate_summary_telemetry(
            record.progress_record.health.status,
            record.report_gate_record.gate_decision.health_status,
            record.report_gate_record.is_admitted(),
            record.admitted,
            record.requires_repair_first,
            progress_repair_task_ids.len(),
            report_repair_tasks,
            next_queue_task_ids.len(),
            record.blocked_reasons.len(),
        );

        Self {
            progress_health_status: record.progress_record.health.status,
            report_health_status: record.report_gate_record.gate_decision.health_status,
            requested_admitted: record.report_gate_record.is_admitted(),
            admitted: record.admitted,
            requires_repair_first: record.requires_repair_first,
            progress_repair_tasks: progress_repair_task_ids.len(),
            report_repair_tasks,
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: record.blocked_reasons.len(),
            progress_repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentRunProgressReportGateSummaryHistory {
    summaries: Vec<AgentRunProgressReportGateSummary>,
}

impl AgentRunProgressReportGateSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentRunProgressReportGateSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentRunProgressReportGateSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentRunProgressReportGateSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentRunProgressReportGateSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentRunProgressReportGateDashboard {
        AgentRunProgressReportGateDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentRunProgressReportGateHealthPolicy,
    ) -> AgentRunProgressReportGateHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunProgressReportGateDashboard {
    pub total_records: usize,
    pub requested_admitted_records: usize,
    pub admitted_records: usize,
    pub repair_first_records: usize,
    pub progress_stable_records: usize,
    pub progress_watch_records: usize,
    pub progress_repair_records: usize,
    pub report_stable_records: usize,
    pub report_watch_records: usize,
    pub report_repair_records: usize,
    pub progress_repair_tasks: usize,
    pub report_repair_tasks: usize,
    pub total_next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub admission_rate: f32,
    pub repair_first_rate: f32,
    pub latest_progress_health_status: Option<AgentRunReportHealthStatus>,
    pub latest_report_health_status: Option<AgentRunReportHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentRunProgressReportGateDashboard {
    pub fn from_summaries(summaries: &[AgentRunProgressReportGateSummary]) -> Self {
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
        let progress_stable_records = summaries
            .iter()
            .filter(|summary| summary.progress_health_status == AgentRunReportHealthStatus::Stable)
            .count();
        let progress_watch_records = summaries
            .iter()
            .filter(|summary| summary.progress_health_status == AgentRunReportHealthStatus::Watch)
            .count();
        let progress_repair_records = summaries
            .iter()
            .filter(|summary| summary.progress_health_status == AgentRunReportHealthStatus::Repair)
            .count();
        let report_stable_records = summaries
            .iter()
            .filter(|summary| summary.report_health_status == AgentRunReportHealthStatus::Stable)
            .count();
        let report_watch_records = summaries
            .iter()
            .filter(|summary| summary.report_health_status == AgentRunReportHealthStatus::Watch)
            .count();
        let report_repair_records = summaries
            .iter()
            .filter(|summary| summary.report_health_status == AgentRunReportHealthStatus::Repair)
            .count();
        let progress_repair_tasks = summaries
            .iter()
            .map(|summary| summary.progress_repair_tasks)
            .sum::<usize>();
        let report_repair_tasks = summaries
            .iter()
            .map(|summary| summary.report_repair_tasks)
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
        let latest_progress_health_status = summaries
            .last()
            .map(|summary| summary.progress_health_status);
        let latest_report_health_status =
            summaries.last().map(|summary| summary.report_health_status);
        let telemetry = agent_run_progress_report_gate_dashboard_telemetry(
            total_records,
            requested_admitted_records,
            admitted_records,
            repair_first_records,
            progress_stable_records,
            progress_watch_records,
            progress_repair_records,
            report_stable_records,
            report_watch_records,
            report_repair_records,
            progress_repair_tasks,
            report_repair_tasks,
            total_next_queue_tasks,
            blocked_reasons,
            admission_rate,
            repair_first_rate,
            latest_progress_health_status,
            latest_report_health_status,
        );

        Self {
            total_records,
            requested_admitted_records,
            admitted_records,
            repair_first_records,
            progress_stable_records,
            progress_watch_records,
            progress_repair_records,
            report_stable_records,
            report_watch_records,
            report_repair_records,
            progress_repair_tasks,
            report_repair_tasks,
            total_next_queue_tasks,
            blocked_reasons,
            admission_rate,
            repair_first_rate,
            latest_progress_health_status,
            latest_report_health_status,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: AgentRunProgressReportGateHealthPolicy,
    ) -> AgentRunProgressReportGateHealth {
        AgentRunProgressReportGateHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentRunProgressReportGateHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_progress_repair_records: usize,
    pub maximum_report_repair_records: usize,
    pub maximum_progress_repair_tasks: usize,
    pub maximum_report_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentRunProgressReportGateHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_admission_rate: 0.67,
            maximum_repair_first_records: 0,
            maximum_progress_repair_records: 0,
            maximum_report_repair_records: 0,
            maximum_progress_repair_tasks: 0,
            maximum_report_repair_tasks: 0,
            maximum_blocked_reasons: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunProgressReportGateHealth {
    pub status: AgentRunReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentRunProgressReportGateDashboard,
}

impl AgentRunProgressReportGateHealth {
    pub fn from_dashboard(
        dashboard: AgentRunProgressReportGateDashboard,
        policy: AgentRunProgressReportGateHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("agent_run_progress_report_gate_history_empty".to_owned());
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_run_progress_report_gate_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "agent_run_progress_report_gate_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }
        if dashboard.progress_repair_records > policy.maximum_progress_repair_records {
            repair_reasons.push(format!(
                "agent_run_progress_report_gate_progress_repair_records={}>{}",
                dashboard.progress_repair_records, policy.maximum_progress_repair_records
            ));
        }
        if dashboard.report_repair_records > policy.maximum_report_repair_records {
            repair_reasons.push(format!(
                "agent_run_progress_report_gate_report_repair_records={}>{}",
                dashboard.report_repair_records, policy.maximum_report_repair_records
            ));
        }
        if dashboard.progress_repair_tasks > policy.maximum_progress_repair_tasks {
            repair_reasons.push(format!(
                "agent_run_progress_report_gate_progress_repair_tasks={}>{}",
                dashboard.progress_repair_tasks, policy.maximum_progress_repair_tasks
            ));
        }
        if dashboard.report_repair_tasks > policy.maximum_report_repair_tasks {
            repair_reasons.push(format!(
                "agent_run_progress_report_gate_report_repair_tasks={}>{}",
                dashboard.report_repair_tasks, policy.maximum_report_repair_tasks
            ));
        }
        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_run_progress_report_gate_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentRunReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentRunReportHealthStatus::Watch, watch_reasons)
        } else {
            (AgentRunReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentRunReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunProgressReportGateSummaryHistoryRecord {
    pub history: AgentRunProgressReportGateSummaryHistory,
    pub appended_summary: AgentRunProgressReportGateSummary,
    pub dashboard: AgentRunProgressReportGateDashboard,
    pub health: AgentRunProgressReportGateHealth,
    pub telemetry: Vec<String>,
}

impl AgentRunProgressReportGateSummaryHistoryRecord {
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
pub struct AgentRunProgressReportGateSummaryHistoryRecorder;

impl AgentRunProgressReportGateSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentRunProgressReportGateSummaryHistory,
        summary: AgentRunProgressReportGateSummary,
        policy: AgentRunProgressReportGateHealthPolicy,
    ) -> AgentRunProgressReportGateSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            agent_run_progress_report_gate_history_record_telemetry(&dashboard, &health);

        AgentRunProgressReportGateSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_gate_with_health(
        &self,
        history: AgentRunProgressReportGateSummaryHistory,
        record: &AgentRunProgressReportGateRecord,
        policy: AgentRunProgressReportGateHealthPolicy,
    ) -> AgentRunProgressReportGateSummaryHistoryRecord {
        self.record_summary_with_health(history, record.summary(), policy)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunProgressReportGate;

impl AgentRunProgressReportGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        run_id: impl AsRef<str>,
        progress_record: AgentRunLedgerProgressSummaryHistoryRecord,
        report_gate_record: AgentRunReportHealthGateRecord,
    ) -> AgentRunProgressReportGateRecord {
        let progress_requires_repair = progress_record.requires_repair_first()
            || progress_record.appended_summary.requires_repair_first
            || !progress_record.appended_summary.can_close_run;
        let requires_repair_first =
            progress_requires_repair || report_gate_record.gate_decision.requires_repair_first;
        let admitted = report_gate_record.is_admitted()
            && progress_record.allows_service_advance()
            && progress_record.appended_summary.can_close_run
            && !requires_repair_first;
        let progress_blocked_reasons =
            agent_run_progress_report_gate_progress_reasons(&progress_record);
        let progress_repair_tasks = agent_run_progress_report_gate_repair_tasks(
            run_id.as_ref(),
            progress_requires_repair,
            &progress_blocked_reasons,
        );
        let mut blocked_reasons = progress_blocked_reasons;
        blocked_reasons.extend(
            report_gate_record
                .gate_decision
                .blocked_reasons
                .iter()
                .map(|reason| format!("report:{reason}")),
        );
        let next_queue = report_gate_record
            .gate_decision
            .next_queue
            .clone()
            .with_repair_first(&progress_repair_tasks);
        let telemetry = agent_run_progress_report_gate_telemetry(
            progress_record.health.status,
            report_gate_record.gate_decision.health_status,
            report_gate_record.is_admitted(),
            admitted,
            requires_repair_first,
            progress_repair_tasks.len(),
            report_gate_record.gate_decision.repair_tasks.len(),
            next_queue.len(),
            blocked_reasons.len(),
        );

        AgentRunProgressReportGateRecord {
            progress_record,
            report_gate_record,
            admitted,
            requires_repair_first,
            progress_repair_tasks,
            next_queue,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunReportHealthGateDecision {
    pub health_status: AgentRunReportHealthStatus,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }

    pub fn summary(&self) -> AgentRunReportHealthGateSummary {
        AgentRunReportHealthGateSummary::from_decision(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunReportHealthGateSummary {
    pub health_status: AgentRunReportHealthStatus,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateSummary {
    pub fn from_decision(decision: &AgentRunReportHealthGateDecision) -> Self {
        let repair_task_ids = decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = decision.next_queue.task_ids();
        let telemetry = agent_run_report_health_gate_summary_telemetry(
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTelomereState {
    pub run_id: String,
    pub remaining_tokens: u32,
    pub remaining_steps: u32,
    pub remaining_messages: u32,
    pub remaining_time_equivalent: Option<u32>,
    pub depletion_reason_codes: Vec<String>,
    pub repeated_repair_streak_count: usize,
    pub loop_risk_signal_count: usize,
    pub senescent: bool,
    pub apoptosis_required: bool,
    pub new_external_call_allowed: bool,
    pub new_file_write_allowed: bool,
    pub new_memory_write_allowed: bool,
    pub new_adaptive_state_write_allowed: bool,
    pub memory_promotion_allowed: bool,
    pub genome_mutation_allowed: bool,
    pub takeover_packet_digest: String,
    pub raw_payload_present: bool,
    pub preview_side_effect_allowed: bool,
    pub telemetry: Vec<String>,
}

impl AgentTelomereState {
    pub fn from_dispatch_summary(
        run_id: impl AsRef<str>,
        summary: &TaskDispatchPlanSummary,
        repeated_repair_streak_count: usize,
    ) -> Self {
        let run_id = run_id.as_ref().to_owned();
        let mut depletion_reason_codes = Vec::new();
        if summary.rejections > 0 {
            depletion_reason_codes.push("dispatch_rejections".to_owned());
        }
        if summary.assignments == 0 && summary.rejections > 0 {
            depletion_reason_codes.push("dispatch_empty_assignments".to_owned());
        }
        if summary.remaining_zero_budget_roles > 0 {
            depletion_reason_codes.push("remaining_zero_budget_roles".to_owned());
        }
        if summary.remaining_partially_depleted_roles > 0 {
            depletion_reason_codes.push("remaining_partially_depleted_roles".to_owned());
        }
        if summary.remaining_token_depleted_roles > 0 {
            depletion_reason_codes.push("remaining_token_depleted_roles".to_owned());
        }
        if summary.remaining_step_depleted_roles > 0 {
            depletion_reason_codes.push("remaining_step_depleted_roles".to_owned());
        }
        if summary.remaining_message_depleted_roles > 0 {
            depletion_reason_codes.push("remaining_message_depleted_roles".to_owned());
        }
        if repeated_repair_streak_count > 0 {
            depletion_reason_codes.push("repeated_repair_pressure".to_owned());
        }

        let loop_risk_signal_count = summary.rejections
            + summary.remaining_zero_budget_roles
            + summary.remaining_partially_depleted_roles
            + repeated_repair_streak_count;
        let senescent = !depletion_reason_codes.is_empty();
        let apoptosis_required = repeated_repair_streak_count >= 2;
        let allows_new_side_effects = !senescent && !apoptosis_required;
        let remaining_tokens = summary.remaining_tokens.to_string();
        let remaining_steps = summary.remaining_steps.to_string();
        let remaining_messages = summary.remaining_messages.to_string();
        let repair_streak = repeated_repair_streak_count.to_string();
        let takeover_packet_digest = agent_preview_digest([
            "issue-501",
            "agent-telomere-state",
            run_id.as_str(),
            remaining_tokens.as_str(),
            remaining_steps.as_str(),
            remaining_messages.as_str(),
            repair_streak.as_str(),
        ]);
        let telemetry = agent_telomere_state_telemetry(
            senescent,
            apoptosis_required,
            summary.remaining_tokens,
            summary.remaining_steps,
            summary.remaining_messages,
            repeated_repair_streak_count,
            loop_risk_signal_count,
            depletion_reason_codes.len(),
        );

        Self {
            run_id,
            remaining_tokens: summary.remaining_tokens,
            remaining_steps: summary.remaining_steps,
            remaining_messages: summary.remaining_messages,
            remaining_time_equivalent: None,
            depletion_reason_codes,
            repeated_repair_streak_count,
            loop_risk_signal_count,
            senescent,
            apoptosis_required,
            new_external_call_allowed: allows_new_side_effects,
            new_file_write_allowed: allows_new_side_effects,
            new_memory_write_allowed: allows_new_side_effects,
            new_adaptive_state_write_allowed: allows_new_side_effects,
            memory_promotion_allowed: allows_new_side_effects,
            genome_mutation_allowed: allows_new_side_effects,
            takeover_packet_digest,
            raw_payload_present: false,
            preview_side_effect_allowed: false,
            telemetry,
        }
    }

    pub fn with_run_report_summary(mut self, summary: &AgentRunReportSummary) -> Self {
        let mut run_pressure = 0usize;
        if summary.unresolved_conflicts > 0 {
            self.depletion_reason_codes
                .push("run_unresolved_conflicts".to_owned());
            run_pressure += summary.unresolved_conflicts;
        }
        if summary.budget_overspends > 0 {
            self.depletion_reason_codes
                .push("run_budget_overspends".to_owned());
            run_pressure += summary.budget_overspends;
        }
        if summary.blocked_side_effects > 0 {
            self.depletion_reason_codes
                .push("run_blocked_side_effects".to_owned());
            run_pressure += summary.blocked_side_effects;
        }
        if run_pressure == 0 {
            return self;
        }

        self.loop_risk_signal_count += run_pressure;
        self.senescent = true;
        self.new_external_call_allowed = false;
        self.new_file_write_allowed = false;
        self.new_memory_write_allowed = false;
        self.new_adaptive_state_write_allowed = false;
        self.memory_promotion_allowed = false;
        self.genome_mutation_allowed = false;
        let reason_count = self.depletion_reason_codes.len().to_string();
        self.takeover_packet_digest = agent_preview_digest([
            "issue-501",
            "agent-telomere-state",
            self.run_id.as_str(),
            reason_count.as_str(),
        ]);
        self.telemetry = agent_telomere_state_telemetry(
            self.senescent,
            self.apoptosis_required,
            self.remaining_tokens,
            self.remaining_steps,
            self.remaining_messages,
            self.repeated_repair_streak_count,
            self.loop_risk_signal_count,
            self.depletion_reason_codes.len(),
        );
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentApoptosisHandoff {
    pub run_id: String,
    pub apoptosis_required: bool,
    pub senescent: bool,
    pub reason_codes: Vec<String>,
    pub next_owner_hint: String,
    pub rollback_anchor_digest: String,
    pub takeover_packet_digest: String,
    pub new_external_call_allowed: bool,
    pub new_file_write_allowed: bool,
    pub new_memory_write_allowed: bool,
    pub new_adaptive_state_write_allowed: bool,
    pub memory_promotion_allowed: bool,
    pub genome_mutation_allowed: bool,
    pub raw_payload_present: bool,
    pub telemetry: Vec<String>,
}

impl AgentApoptosisHandoff {
    pub fn from_telomere_state(state: &AgentTelomereState) -> Self {
        let next_owner_hint = if state.apoptosis_required {
            "scheduler"
        } else if state.senescent {
            "summary_handoff"
        } else {
            "continue"
        }
        .to_owned();
        let rollback_anchor_digest = agent_preview_digest([
            "issue-501",
            "agent-apoptosis-rollback-anchor",
            state.run_id.as_str(),
            state.takeover_packet_digest.as_str(),
        ]);
        let telemetry = vec![
            "agent_apoptosis_handoff=true".to_owned(),
            format!(
                "agent_apoptosis_handoff_apoptosis_required={}",
                state.apoptosis_required
            ),
            format!("agent_apoptosis_handoff_senescent={}", state.senescent),
            format!(
                "agent_apoptosis_handoff_reason_codes={}",
                state.depletion_reason_codes.len()
            ),
            format!("agent_apoptosis_handoff_next_owner={next_owner_hint}"),
            format!(
                "agent_apoptosis_handoff_memory_promotion_allowed={}",
                state.memory_promotion_allowed
            ),
            format!(
                "agent_apoptosis_handoff_genome_mutation_allowed={}",
                state.genome_mutation_allowed
            ),
            "agent_apoptosis_handoff_raw_payload_present=false".to_owned(),
        ];

        Self {
            run_id: state.run_id.clone(),
            apoptosis_required: state.apoptosis_required,
            senescent: state.senescent,
            reason_codes: state.depletion_reason_codes.clone(),
            next_owner_hint,
            rollback_anchor_digest,
            takeover_packet_digest: state.takeover_packet_digest.clone(),
            new_external_call_allowed: state.new_external_call_allowed,
            new_file_write_allowed: state.new_file_write_allowed,
            new_memory_write_allowed: state.new_memory_write_allowed,
            new_adaptive_state_write_allowed: state.new_adaptive_state_write_allowed,
            memory_promotion_allowed: state.memory_promotion_allowed,
            genome_mutation_allowed: state.genome_mutation_allowed,
            raw_payload_present: false,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentRunReportHealthGateHistory {
    summaries: Vec<AgentRunReportHealthGateSummary>,
}

impl AgentRunReportHealthGateHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentRunReportHealthGateSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentRunReportHealthGateSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentRunReportHealthGateSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentRunReportHealthGateSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentRunReportHealthGateDashboard {
        AgentRunReportHealthGateDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentRunReportHealthGateHealthPolicy,
    ) -> AgentRunReportHealthGateHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateDashboard {
    pub total_records: usize,
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
    pub latest_health_status: Option<AgentRunReportHealthStatus>,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateDashboard {
    pub fn from_summaries(summaries: &[AgentRunReportHealthGateSummary]) -> Self {
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
            .filter(|summary| summary.health_status == AgentRunReportHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.health_status == AgentRunReportHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.health_status == AgentRunReportHealthStatus::Repair)
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
            .map(|summary| summary.blocked_reasons.len())
            .sum::<usize>();
        let admission_rate = rate(admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let latest_health_status = summaries.last().map(|summary| summary.health_status);
        let latest_blocked_reasons = summaries
            .last()
            .map(|summary| summary.blocked_reasons.clone())
            .unwrap_or_default();
        let telemetry = agent_run_report_health_gate_dashboard_telemetry(
            total_records,
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
            latest_health_status,
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
            blocked_reasons,
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
        !self.is_empty() && self.repair_first_records == 0 && self.repair_task_count == 0
    }

    pub fn health(
        &self,
        policy: AgentRunReportHealthGateHealthPolicy,
    ) -> AgentRunReportHealthGateHealth {
        AgentRunReportHealthGateHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentRunReportHealthGateHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentRunReportHealthGateHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_admission_rate: 0.67,
            maximum_repair_first_rate: 0.0,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateHealth {
    pub status: AgentRunReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentRunReportHealthGateDashboard,
}

impl AgentRunReportHealthGateHealth {
    pub fn from_dashboard(
        dashboard: AgentRunReportHealthGateDashboard,
        policy: AgentRunReportHealthGateHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("agent_run_report_health_gate_history_empty".to_owned());
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_run_report_health_gate_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentRunReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentRunReportHealthStatus::Watch, watch_reasons)
        } else {
            (AgentRunReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentRunReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateHistoryRecord {
    pub history: AgentRunReportHealthGateHistory,
    pub appended_summary: AgentRunReportHealthGateSummary,
    pub dashboard: AgentRunReportHealthGateDashboard,
    pub health: AgentRunReportHealthGateHealth,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateHistoryRecord {
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
pub struct AgentRunReportHealthGateHistoryRecorder;

impl AgentRunReportHealthGateHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentRunReportHealthGateHistory,
        summary: AgentRunReportHealthGateSummary,
        policy: AgentRunReportHealthGateHealthPolicy,
    ) -> AgentRunReportHealthGateHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = agent_run_report_health_gate_history_record_telemetry(&dashboard, &health);

        AgentRunReportHealthGateHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_gate_record_with_health(
        &self,
        history: AgentRunReportHealthGateHistory,
        record: &AgentRunReportHealthGateRecord,
        policy: AgentRunReportHealthGateHealthPolicy,
    ) -> AgentRunReportHealthGateHistoryRecord {
        self.record_summary_with_health(history, record.gate_summary.clone(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffRecord {
    pub trend_record: AgentRunReportHealthGateHistoryRecord,
    pub gate_decision: AgentRunReportHealthGateDecision,
    pub gate_summary: AgentRunReportHealthGateSummary,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentRunReportHealthGateTrendHandoffSummary {
        AgentRunReportHealthGateTrendHandoffSummary::from_record(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunReportHealthGateTrendHandoffSummary {
    pub trend_health_status: AgentRunReportHealthStatus,
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

impl AgentRunReportHealthGateTrendHandoffSummary {
    pub fn from_record(record: &AgentRunReportHealthGateTrendHandoffRecord) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry = agent_run_report_health_gate_trend_handoff_summary_telemetry(
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
pub struct AgentRunReportHealthGateTrendHandoffHistory {
    summaries: Vec<AgentRunReportHealthGateTrendHandoffSummary>,
}

impl AgentRunReportHealthGateTrendHandoffHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentRunReportHealthGateTrendHandoffSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentRunReportHealthGateTrendHandoffSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentRunReportHealthGateTrendHandoffSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentRunReportHealthGateTrendHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentRunReportHealthGateTrendHandoffDashboard {
        AgentRunReportHealthGateTrendHandoffDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentRunReportHealthGateTrendHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffDashboard {
    pub total_records: usize,
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
    pub latest_trend_health_status: Option<AgentRunReportHealthStatus>,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffDashboard {
    pub fn from_summaries(summaries: &[AgentRunReportHealthGateTrendHandoffSummary]) -> Self {
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
            .filter(|summary| summary.trend_health_status == AgentRunReportHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.trend_health_status == AgentRunReportHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.trend_health_status == AgentRunReportHealthStatus::Repair)
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
            .map(|summary| summary.blocked_reasons.len())
            .sum::<usize>();
        let admission_rate = rate(admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let latest_trend_health_status =
            summaries.last().map(|summary| summary.trend_health_status);
        let latest_blocked_reasons = summaries
            .last()
            .map(|summary| summary.blocked_reasons.clone())
            .unwrap_or_default();
        let telemetry = agent_run_report_health_gate_trend_handoff_dashboard_telemetry(
            total_records,
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
            blocked_reasons,
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
        !self.is_empty()
            && self.repair_first_records == 0
            && self.repair_records == 0
            && self.repair_task_count == 0
    }

    pub fn health(
        &self,
        policy: AgentRunReportHealthGateTrendHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffHealth {
        AgentRunReportHealthGateTrendHandoffHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentRunReportHealthGateTrendHandoffHealthPolicy {
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
pub struct AgentRunReportHealthGateTrendHandoffHealth {
    pub status: AgentRunReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentRunReportHealthGateTrendHandoffDashboard,
}

impl AgentRunReportHealthGateTrendHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentRunReportHealthGateTrendHandoffDashboard,
        policy: AgentRunReportHealthGateTrendHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons
                .push("agent_run_report_health_gate_trend_handoff_history_empty".to_owned());
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentRunReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentRunReportHealthStatus::Watch, watch_reasons)
        } else {
            (AgentRunReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentRunReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffHistoryRecord {
    pub history: AgentRunReportHealthGateTrendHandoffHistory,
    pub appended_summary: AgentRunReportHealthGateTrendHandoffSummary,
    pub dashboard: AgentRunReportHealthGateTrendHandoffDashboard,
    pub health: AgentRunReportHealthGateTrendHandoffHealth,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffHistoryRecord {
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
pub struct AgentRunReportHealthGateTrendHandoffHistoryRecorder;

impl AgentRunReportHealthGateTrendHandoffHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentRunReportHealthGateTrendHandoffHistory,
        summary: AgentRunReportHealthGateTrendHandoffSummary,
        policy: AgentRunReportHealthGateTrendHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = agent_run_report_health_gate_trend_handoff_history_record_telemetry(
            &dashboard, &health,
        );

        AgentRunReportHealthGateTrendHandoffHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_handoff_with_health(
        &self,
        history: AgentRunReportHealthGateTrendHandoffHistory,
        record: &AgentRunReportHealthGateTrendHandoffRecord,
        policy: AgentRunReportHealthGateTrendHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffHistoryRecord {
        self.record_summary_with_health(history, record.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffGateDecision {
    pub requested_admitted: bool,
    pub handoff_health: AgentRunReportHealthGateTrendHandoffHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGateTrendHandoffGate;

impl AgentRunReportHealthGateTrendHandoffGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        handoff: &AgentRunReportHealthGateTrendHandoffRecord,
        history_record: &AgentRunReportHealthGateTrendHandoffHistoryRecord,
    ) -> AgentRunReportHealthGateTrendHandoffGateDecision {
        let requested_admitted = handoff.is_admitted();
        let handoff_health = history_record.health.clone();
        let trend_requires_repair = handoff_health.status == AgentRunReportHealthStatus::Repair;
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
                    agent_run_report_health_gate_trend_handoff_repair_task(
                        run_id.as_ref(),
                        index,
                        reason,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let next_queue = handoff.next_queue().with_repair_first(&repair_tasks);
        let mut blocked_reasons = handoff.gate_decision.blocked_reasons.clone();
        if trend_requires_repair {
            blocked_reasons.extend(handoff_health.reasons.clone());
        }
        let telemetry = agent_run_report_health_gate_trend_handoff_gate_telemetry(
            handoff_health.status,
            requested_admitted,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            &blocked_reasons,
        );

        AgentRunReportHealthGateTrendHandoffGateDecision {
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorRecord {
    pub handoff: AgentRunReportHealthGateTrendHandoffRecord,
    pub history_record: AgentRunReportHealthGateTrendHandoffHistoryRecord,
    pub gate_decision: AgentRunReportHealthGateTrendHandoffGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentRunReportHealthGateTrendHandoffMonitorSummary {
        AgentRunReportHealthGateTrendHandoffMonitorSummary::from_monitor(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGateTrendHandoffMonitor {
    history_recorder: AgentRunReportHealthGateTrendHandoffHistoryRecorder,
    gate: AgentRunReportHealthGateTrendHandoffGate,
}

impl AgentRunReportHealthGateTrendHandoffMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        handoff: AgentRunReportHealthGateTrendHandoffRecord,
        history: AgentRunReportHealthGateTrendHandoffHistory,
        policy: AgentRunReportHealthGateTrendHandoffHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorRecord {
        let history_record = self
            .history_recorder
            .record_handoff_with_health(history, &handoff, policy);
        let gate_decision = self.gate.evaluate(run_id, &handoff, &history_record);
        let telemetry = agent_run_report_health_gate_trend_handoff_monitor_telemetry(
            &handoff,
            &history_record,
            &gate_decision,
        );

        AgentRunReportHealthGateTrendHandoffMonitorRecord {
            handoff,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorSummary {
    pub handoff_health_status: AgentRunReportHealthStatus,
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

impl AgentRunReportHealthGateTrendHandoffMonitorSummary {
    pub fn from_monitor(record: &AgentRunReportHealthGateTrendHandoffMonitorRecord) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry = agent_run_report_health_gate_trend_handoff_monitor_summary_telemetry(
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory {
    summaries: Vec<AgentRunReportHealthGateTrendHandoffMonitorSummary>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentRunReportHealthGateTrendHandoffMonitorSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentRunReportHealthGateTrendHandoffMonitorSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentRunReportHealthGateTrendHandoffMonitorSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentRunReportHealthGateTrendHandoffMonitorSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentRunReportHealthGateTrendHandoffMonitorDashboard {
        AgentRunReportHealthGateTrendHandoffMonitorDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorDashboard {
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
    pub latest_handoff_health_status: Option<AgentRunReportHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorDashboard {
    pub fn from_summaries(
        summaries: &[AgentRunReportHealthGateTrendHandoffMonitorSummary],
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
            .filter(|summary| summary.handoff_health_status == AgentRunReportHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.handoff_health_status == AgentRunReportHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.handoff_health_status == AgentRunReportHealthStatus::Repair)
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
        let telemetry = agent_run_report_health_gate_trend_handoff_monitor_dashboard_telemetry(
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
        policy: AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHealth {
        AgentRunReportHealthGateTrendHandoffMonitorHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy {
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHealth {
    pub status: AgentRunReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentRunReportHealthGateTrendHandoffMonitorDashboard,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHealth {
    pub fn from_dashboard(
        dashboard: AgentRunReportHealthGateTrendHandoffMonitorDashboard,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "agent_run_report_health_gate_trend_handoff_monitor_history_empty".to_owned(),
            );
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentRunReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentRunReportHealthStatus::Watch, watch_reasons)
        } else {
            (AgentRunReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentRunReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecord {
    pub history: AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory,
    pub appended_summary: AgentRunReportHealthGateTrendHandoffMonitorSummary,
    pub dashboard: AgentRunReportHealthGateTrendHandoffMonitorDashboard,
    pub health: AgentRunReportHealthGateTrendHandoffMonitorHealth,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecord {
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecorder;

impl AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory,
        summary: AgentRunReportHealthGateTrendHandoffMonitorSummary,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = agent_run_report_health_gate_trend_handoff_monitor_history_record_telemetry(
            &dashboard, &health,
        );

        AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_monitor_with_health(
        &self,
        history: AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory,
        monitor: &AgentRunReportHealthGateTrendHandoffMonitorRecord,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecord {
        self.record_summary_with_health(history, monitor.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorGateDecision {
    pub requested_admitted: bool,
    pub monitor_health: AgentRunReportHealthGateTrendHandoffMonitorHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorGate;

impl AgentRunReportHealthGateTrendHandoffMonitorGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        monitor: &AgentRunReportHealthGateTrendHandoffMonitorRecord,
        history_record: &AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecord,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorGateDecision {
        let requested_admitted = monitor.is_admitted();
        let monitor_health = history_record.health.clone();
        let monitor_requires_repair = monitor_health.status == AgentRunReportHealthStatus::Repair;
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
                    agent_run_report_health_gate_trend_handoff_monitor_repair_task(
                        run_id.as_ref(),
                        index,
                        reason,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let next_queue = monitor.next_queue().with_repair_first(&repair_tasks);
        let mut blocked_reasons = monitor.gate_decision.blocked_reasons.clone();
        if monitor_requires_repair {
            blocked_reasons.extend(monitor_health.reasons.clone());
        }
        let telemetry = agent_run_report_health_gate_trend_handoff_monitor_gate_telemetry(
            monitor_health.status,
            requested_admitted,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            &blocked_reasons,
        );

        AgentRunReportHealthGateTrendHandoffMonitorGateDecision {
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffRecord {
    pub monitor: AgentRunReportHealthGateTrendHandoffMonitorRecord,
    pub history_record: AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecord,
    pub gate_decision: AgentRunReportHealthGateTrendHandoffMonitorGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary::from_handoff(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoff {
    history_recorder: AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecorder,
    gate: AgentRunReportHealthGateTrendHandoffMonitorGate,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        monitor: AgentRunReportHealthGateTrendHandoffMonitorRecord,
        history: AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffRecord {
        let history_record = self
            .history_recorder
            .record_monitor_with_health(history, &monitor, policy);
        let gate_decision = self.gate.evaluate(run_id, &monitor, &history_record);
        let telemetry = agent_run_report_health_gate_trend_handoff_monitor_handoff_telemetry(
            &monitor,
            &history_record,
            &gate_decision,
        );

        AgentRunReportHealthGateTrendHandoffMonitorHandoffRecord {
            monitor,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary {
    pub monitor_health_status: AgentRunReportHealthStatus,
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

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary {
    pub fn from_handoff(record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffRecord) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_summary_telemetry(
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory {
    summaries: Vec<AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffDashboard {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffDashboard {
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
    pub latest_monitor_health_status: Option<AgentRunReportHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffDashboard {
    pub fn from_summaries(
        summaries: &[AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary],
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
            .filter(|summary| summary.monitor_health_status == AgentRunReportHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.monitor_health_status == AgentRunReportHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.monitor_health_status == AgentRunReportHealthStatus::Repair)
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
            agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_telemetry(
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
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHealth {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy {
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHealth {
    pub status: AgentRunReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentRunReportHealthGateTrendHandoffMonitorHandoffDashboard,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentRunReportHealthGateTrendHandoffMonitorHandoffDashboard,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_empty"
                    .to_owned(),
            );
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentRunReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentRunReportHealthStatus::Watch, watch_reasons)
        } else {
            (AgentRunReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentRunReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
    pub history: AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory,
    pub appended_summary: AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary,
    pub dashboard: AgentRunReportHealthGateTrendHandoffMonitorHandoffDashboard,
    pub health: AgentRunReportHealthGateTrendHandoffMonitorHandoffHealth,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder;

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        summary: AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_telemetry(
                &dashboard, &health,
            );

        AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_handoff_with_health(
        &self,
        history: AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        handoff: &AgentRunReportHealthGateTrendHandoffMonitorHandoffRecord,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
        self.record_summary_with_health(history, handoff.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffGateDecision {
    pub requested_admitted: bool,
    pub handoff_health: AgentRunReportHealthGateTrendHandoffMonitorHandoffHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffGate;

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        handoff: &AgentRunReportHealthGateTrendHandoffMonitorHandoffRecord,
        history_record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffGateDecision {
        let requested_admitted = handoff.is_admitted();
        let handoff_health = history_record.health.clone();
        let handoff_requires_repair = handoff_health.status == AgentRunReportHealthStatus::Repair;
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
                    agent_run_report_health_gate_trend_handoff_monitor_handoff_repair_task(
                        run_id.as_ref(),
                        index,
                        reason,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let next_queue = handoff.next_queue().with_repair_first(&repair_tasks);
        let mut blocked_reasons = handoff.gate_decision.blocked_reasons.clone();
        if handoff_requires_repair {
            blocked_reasons.extend(handoff_health.reasons.clone());
        }
        let telemetry = agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_telemetry(
            handoff_health.status,
            requested_admitted,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            &blocked_reasons,
        );

        AgentRunReportHealthGateTrendHandoffMonitorHandoffGateDecision {
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffRecord {
    pub handoff: AgentRunReportHealthGateTrendHandoffMonitorHandoffRecord,
    pub history_record: AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord,
    pub gate_decision: AgentRunReportHealthGateTrendHandoffMonitorHandoffGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary::from_handoff(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary {
    pub handoff_health_status: AgentRunReportHealthStatus,
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

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary {
    pub fn from_handoff(
        record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffRecord,
    ) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_telemetry(
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory {
    summaries: Vec<AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(
        &mut self,
        summary: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary,
    ) {
        self.summaries.push(summary);
    }

    pub fn latest(
        &self,
    ) -> Option<&AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffDashboard {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffDashboard::from_summaries(
            &self.summaries,
        )
    }

    pub fn health(
        &self,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffDashboard {
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
    pub latest_handoff_health_status: Option<AgentRunReportHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffDashboard {
    pub fn from_summaries(
        summaries: &[AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary],
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
            .filter(|summary| summary.handoff_health_status == AgentRunReportHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.handoff_health_status == AgentRunReportHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.handoff_health_status == AgentRunReportHealthStatus::Repair)
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
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_telemetry(
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
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealth {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy {
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealth {
    pub status: AgentRunReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffDashboard,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffDashboard,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_empty"
                    .to_owned(),
            );
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentRunReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentRunReportHealthStatus::Watch, watch_reasons)
        } else {
            (AgentRunReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentRunReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord {
    pub history: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
    pub appended_summary: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary,
    pub dashboard: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffDashboard,
    pub health: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealth,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord {
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder;

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
        summary: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_telemetry(
                &dashboard, &health,
            );

        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_handoff_with_health(
        &self,
        history: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
        handoff: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffRecord,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord {
        self.record_summary_with_health(history, handoff.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGateDecision {
    pub requested_admitted: bool,
    pub packet_health: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGate;

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        handoff: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffRecord,
        history_record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGateDecision {
        let requested_admitted = handoff.is_admitted();
        let packet_health = history_record.health.clone();
        let packet_requires_repair = packet_health.status == AgentRunReportHealthStatus::Repair;
        let requires_repair_first =
            handoff.gate_decision.requires_repair_first || packet_requires_repair;
        let admitted = requested_admitted && !packet_requires_repair;
        let repair_tasks = if packet_requires_repair {
            packet_health
                .reasons
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, reason)| {
                    agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_repair_task(
                        run_id.as_ref(),
                        index,
                        reason,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let next_queue = handoff.next_queue().with_repair_first(&repair_tasks);
        let mut blocked_reasons = handoff.gate_decision.blocked_reasons.clone();
        if packet_requires_repair {
            blocked_reasons.extend(packet_health.reasons.clone());
        }
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_telemetry(
                packet_health.status,
                requested_admitted,
                admitted,
                requires_repair_first,
                repair_tasks.len(),
                next_queue.len(),
                &blocked_reasons,
            );

        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGateDecision {
            requested_admitted,
            packet_health,
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord {
    pub packet: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffRecord,
    pub history_record:
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord,
    pub gate_decision: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(
        &self,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary::from_admission(
            self,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary {
    pub packet_health_status: AgentRunReportHealthStatus,
    pub requested_admitted: bool,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub packet_records: usize,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary {
    pub fn from_admission(
        record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
    ) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_telemetry(
                record.gate_decision.packet_health.status,
                record.gate_decision.requested_admitted,
                record.gate_decision.admitted,
                record.gate_decision.requires_repair_first,
                record.history_record.dashboard.total_records,
                repair_task_ids.len(),
                next_queue_task_ids.len(),
                record.gate_decision.blocked_reasons.len(),
            );

        Self {
            packet_health_status: record.gate_decision.packet_health.status,
            requested_admitted: record.gate_decision.requested_admitted,
            admitted: record.gate_decision.admitted,
            requires_repair_first: record.gate_decision.requires_repair_first,
            packet_records: record.history_record.dashboard.total_records,
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory {
    summaries: Vec<AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(
        &mut self,
        summary: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary,
    ) {
        self.summaries.push(summary);
    }

    pub fn latest(
        &self,
    ) -> Option<&AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(
        &self,
    ) -> &[AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(
        &self,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard::from_summaries(
            &self.summaries,
        )
    }

    pub fn health(
        &self,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard {
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
    pub latest_packet_health_status: Option<AgentRunReportHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard {
    pub fn from_summaries(
        summaries: &[AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary],
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
            .filter(|summary| summary.packet_health_status == AgentRunReportHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.packet_health_status == AgentRunReportHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.packet_health_status == AgentRunReportHealthStatus::Repair)
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
        let latest_packet_health_status =
            summaries.last().map(|summary| summary.packet_health_status);
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_telemetry(
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
                latest_packet_health_status,
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
            latest_packet_health_status,
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
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy {
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth {
    pub status: AgentRunReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_empty"
                    .to_owned(),
            );
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentRunReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentRunReportHealthStatus::Watch, watch_reasons)
        } else {
            (AgentRunReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentRunReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord {
    pub history: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
    pub appended_summary: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary,
    pub dashboard: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard,
    pub health: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord {
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder;

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
        summary: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_telemetry(
                &dashboard, &health,
            );

        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_admission_with_health(
        &self,
        history: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
        admission: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord {
        self.record_summary_with_health(history, admission.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision {
    pub requested_admitted: bool,
    pub admission_health: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGate;

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        admission: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
        history_record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision {
        let requested_admitted = admission.is_admitted();
        let admission_health = history_record.health.clone();
        let admission_requires_repair =
            admission_health.status == AgentRunReportHealthStatus::Repair;
        let requires_repair_first =
            admission.gate_decision.requires_repair_first || admission_requires_repair;
        let admitted = requested_admitted && !admission_requires_repair;
        let repair_tasks = if admission_requires_repair {
            admission_health
                .reasons
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, reason)| {
                    agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_task(
                        run_id.as_ref(),
                        index,
                        reason,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let next_queue = admission.next_queue().with_repair_first(&repair_tasks);
        let mut blocked_reasons = admission.gate_decision.blocked_reasons.clone();
        if admission_requires_repair {
            blocked_reasons.extend(admission_health.reasons.clone());
        }
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_telemetry(
                admission_health.status,
                requested_admitted,
                admitted,
                requires_repair_first,
                repair_tasks.len(),
                next_queue.len(),
                &blocked_reasons,
            );

        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision {
            requested_admitted,
            admission_health,
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
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord {
    pub admission: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
    pub history_record:
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord,
    pub gate_decision: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(
        &self,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffSummary {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffSummary::from_handoff(
            self,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffSummary {
    pub admission_health_status: AgentRunReportHealthStatus,
    pub requested_admitted: bool,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub admission_records: usize,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffSummary {
    pub fn from_handoff(
        record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord,
    ) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_telemetry(
                record.gate_decision.admission_health.status,
                record.gate_decision.requested_admitted,
                record.gate_decision.admitted,
                record.gate_decision.requires_repair_first,
                record.history_record.dashboard.total_records,
                repair_task_ids.len(),
                next_queue_task_ids.len(),
                record.gate_decision.blocked_reasons.len(),
            );

        Self {
            admission_health_status: record.gate_decision.admission_health.status,
            requested_admitted: record.gate_decision.requested_admitted,
            admitted: record.gate_decision.admitted,
            requires_repair_first: record.gate_decision.requires_repair_first,
            admission_records: record.history_record.dashboard.total_records,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: record.gate_decision.blocked_reasons.len(),
            repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff {
    history_recorder:
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder,
    gate: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGate,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        admission: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
        history: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord {
        let history_record = self
            .history_recorder
            .record_admission_with_health(history, &admission, policy);
        let gate_decision = self.gate.evaluate(run_id, &admission, &history_record);
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_telemetry(
                &admission,
                &history_record,
                &gate_decision,
            );

        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord {
            admission,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff {
    history_recorder:
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder,
    gate: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGate,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        packet: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffRecord,
        history: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord {
        let history_record = self
            .history_recorder
            .record_handoff_with_health(history, &packet, policy);
        let gate_decision = self.gate.evaluate(run_id, &packet, &history_record);
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_telemetry(
                &packet,
                &history_record,
                &gate_decision,
            );

        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord {
            packet,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff {
    history_recorder: AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder,
    gate: AgentRunReportHealthGateTrendHandoffMonitorHandoffGate,
}

impl AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        handoff: AgentRunReportHealthGateTrendHandoffMonitorHandoffRecord,
        history: AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        policy: AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffRecord {
        let history_record = self
            .history_recorder
            .record_handoff_with_health(history, &handoff, policy);
        let gate_decision = self.gate.evaluate(run_id, &handoff, &history_record);
        let telemetry =
            agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_telemetry(
                &handoff,
                &history_record,
                &gate_decision,
            );

        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffRecord {
            handoff,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGateTrendHandoff;

impl AgentRunReportHealthGateTrendHandoff {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_and_gate(
        &self,
        history: AgentRunReportHealthGateHistory,
        summary: AgentRunReportHealthGateSummary,
        policy: AgentRunReportHealthGateHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentRunReportHealthGateTrendHandoffRecord {
        let trend_record = AgentRunReportHealthGateHistoryRecorder::new()
            .record_summary_with_health(history, summary, policy);
        self.gate_record(run_id, trend_record, next_queue)
    }

    pub fn record_gate_record_and_gate(
        &self,
        history: AgentRunReportHealthGateHistory,
        record: &AgentRunReportHealthGateRecord,
        policy: AgentRunReportHealthGateHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentRunReportHealthGateTrendHandoffRecord {
        let trend_record = AgentRunReportHealthGateHistoryRecorder::new()
            .record_gate_record_with_health(history, record, policy);
        self.gate_record(run_id, trend_record, next_queue)
    }

    pub fn gate_record(
        &self,
        run_id: impl AsRef<str>,
        trend_record: AgentRunReportHealthGateHistoryRecord,
        next_queue: &AgentTaskQueue,
    ) -> AgentRunReportHealthGateTrendHandoffRecord {
        let gate_decision = AgentRunReportHealthGateTrendGate::new().evaluate(
            run_id,
            &trend_record.health,
            next_queue,
        );
        let gate_summary = gate_decision.summary();
        let telemetry =
            agent_run_report_health_gate_trend_handoff_telemetry(&trend_record, &gate_summary);

        AgentRunReportHealthGateTrendHandoffRecord {
            trend_record,
            gate_decision,
            gate_summary,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunReportHealthGate;

impl AgentRunReportHealthGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        health: &AgentRunReportHealth,
        next_queue: &AgentTaskQueue,
    ) -> AgentRunReportHealthGateDecision {
        let requires_repair_first = health.status == AgentRunReportHealthStatus::Repair;
        let admitted = !requires_repair_first;
        let repair_tasks = if requires_repair_first {
            health
                .reasons
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, reason)| {
                    agent_run_report_health_repair_task(run_id.as_ref(), index, reason)
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let merged_queue = next_queue.clone().with_repair_first(&repair_tasks);
        let blocked_reasons = if requires_repair_first {
            health.reasons.clone()
        } else {
            Vec::new()
        };
        let telemetry = agent_run_report_health_gate_telemetry(
            health.status,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            merged_queue.len(),
            &blocked_reasons,
        );

        AgentRunReportHealthGateDecision {
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
pub struct AgentRunReportHealthGateTrendGate;

impl AgentRunReportHealthGateTrendGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        health: &AgentRunReportHealthGateHealth,
        next_queue: &AgentTaskQueue,
    ) -> AgentRunReportHealthGateDecision {
        let requires_repair_first = health.status == AgentRunReportHealthStatus::Repair;
        let admitted = !requires_repair_first;
        let repair_tasks = if requires_repair_first {
            health
                .reasons
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, reason)| {
                    agent_run_report_health_gate_trend_repair_task(run_id.as_ref(), index, reason)
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let merged_queue = next_queue.clone().with_repair_first(&repair_tasks);
        let blocked_reasons = if requires_repair_first {
            health.reasons.clone()
        } else {
            Vec::new()
        };
        let telemetry = agent_run_report_health_gate_trend_gate_telemetry(
            health.status,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            merged_queue.len(),
            &blocked_reasons,
        );

        AgentRunReportHealthGateDecision {
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

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunGateDecision {
    pub summary: AgentRunReportSummary,
    pub can_promote_memory_note: bool,
    pub can_write_adaptive_state: bool,
    pub can_dispatch_external_call: bool,
    pub requires_repair_first: bool,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunGateDecision {
    pub fn from_report(report: &AgentRunReport) -> Self {
        let summary = report.summary();
        let mut reasons = Vec::new();

        if summary.unresolved_conflicts > 0 {
            reasons.push(format!(
                "unresolved_conflicts={}",
                summary.unresolved_conflicts
            ));
        }
        if summary.budget_overspends > 0 {
            reasons.push(format!("budget_overspends={}", summary.budget_overspends));
        }
        reasons.extend(
            report
                .side_effects
                .iter()
                .filter(|gate| !gate.allowed)
                .map(|gate| {
                    format!(
                        "side_effect_blocked kind={} reason={}",
                        gate.kind.as_str(),
                        gate.reason
                    )
                }),
        );

        let no_repair_reasons = reasons.is_empty();
        let can_promote_memory_note = summary.memory_note_allowed && no_repair_reasons;
        let can_write_adaptive_state = summary.adaptive_state_allowed && no_repair_reasons;
        let can_dispatch_external_call = summary.external_call_allowed && no_repair_reasons;
        let requires_repair_first = !no_repair_reasons;
        let telemetry = agent_run_gate_telemetry(
            can_promote_memory_note,
            can_write_adaptive_state,
            can_dispatch_external_call,
            requires_repair_first,
            reasons.len(),
            &summary,
        );

        Self {
            summary,
            can_promote_memory_note,
            can_write_adaptive_state,
            can_dispatch_external_call,
            requires_repair_first,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunBudgetOverspend {
    pub task_id: String,
    pub role: AgentRole,
    pub reserved: AgentBudget,
    pub spent: AgentBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunBudgetAudit {
    pub overspends: Vec<RunBudgetOverspend>,
}

impl RunBudgetAudit {
    pub fn has_overspends(&self) -> bool {
        !self.overspends.is_empty()
    }

    pub fn overspend_count(&self) -> usize {
        self.overspends.len()
    }

    pub fn summary(&self) -> RunBudgetAuditSummary {
        RunBudgetAuditSummary::from_audit(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunBudgetAuditSummary {
    pub overspends: usize,
    pub overspent_tokens: u32,
    pub overspent_steps: u32,
    pub overspent_messages: u32,
    pub telemetry: Vec<String>,
}

impl RunBudgetAuditSummary {
    pub fn from_audit(audit: &RunBudgetAudit) -> Self {
        let overspends = audit.overspend_count();
        let overspent_tokens = audit
            .overspends
            .iter()
            .map(|overspend| {
                overspend
                    .spent
                    .tokens
                    .saturating_sub(overspend.reserved.tokens)
            })
            .sum();
        let overspent_steps = audit
            .overspends
            .iter()
            .map(|overspend| {
                overspend
                    .spent
                    .steps
                    .saturating_sub(overspend.reserved.steps)
            })
            .sum();
        let overspent_messages = audit
            .overspends
            .iter()
            .map(|overspend| {
                overspend
                    .spent
                    .messages
                    .saturating_sub(overspend.reserved.messages)
            })
            .sum();
        let telemetry = run_budget_audit_summary_telemetry(
            overspends,
            overspent_tokens,
            overspent_steps,
            overspent_messages,
        );

        Self {
            overspends,
            overspent_tokens,
            overspent_steps,
            overspent_messages,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunLedgerProgress {
    pub assigned_tasks: usize,
    pub reported_tasks: usize,
    pub accepted_results: usize,
    pub rejected_results: usize,
    pub dispatch_rejections: usize,
    pub missing_assigned_tasks: usize,
    pub unassigned_results: usize,
    pub missing_task_ids: Vec<String>,
    pub rejected_task_ids: Vec<String>,
    pub unassigned_task_ids: Vec<String>,
    pub empty_dispatch: bool,
    pub can_close_run: bool,
    pub requires_repair_first: bool,
    pub telemetry: Vec<String>,
}

impl AgentRunLedgerProgress {
    pub fn summary(&self) -> AgentRunLedgerProgressSummary {
        AgentRunLedgerProgressSummary::from_progress(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunLedgerProgressSummary {
    pub assigned_tasks: usize,
    pub reported_tasks: usize,
    pub accepted_results: usize,
    pub rejected_results: usize,
    pub dispatch_rejections: usize,
    pub missing_assigned_tasks: usize,
    pub unassigned_results: usize,
    pub empty_dispatch: bool,
    pub can_close_run: bool,
    pub requires_repair_first: bool,
    pub telemetry: Vec<String>,
}

impl AgentRunLedgerProgressSummary {
    pub fn from_progress(progress: &AgentRunLedgerProgress) -> Self {
        let telemetry = agent_run_ledger_progress_summary_telemetry(
            progress.assigned_tasks,
            progress.reported_tasks,
            progress.accepted_results,
            progress.rejected_results,
            progress.dispatch_rejections,
            progress.missing_assigned_tasks,
            progress.unassigned_results,
            progress.empty_dispatch,
            progress.can_close_run,
            progress.requires_repair_first,
        );

        Self {
            assigned_tasks: progress.assigned_tasks,
            reported_tasks: progress.reported_tasks,
            accepted_results: progress.accepted_results,
            rejected_results: progress.rejected_results,
            dispatch_rejections: progress.dispatch_rejections,
            missing_assigned_tasks: progress.missing_assigned_tasks,
            unassigned_results: progress.unassigned_results,
            empty_dispatch: progress.empty_dispatch,
            can_close_run: progress.can_close_run,
            requires_repair_first: progress.requires_repair_first,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentRunLedgerProgressSummaryHistory {
    summaries: Vec<AgentRunLedgerProgressSummary>,
}

impl AgentRunLedgerProgressSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentRunLedgerProgressSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentRunLedgerProgressSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentRunLedgerProgressSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentRunLedgerProgressSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentRunLedgerProgressDashboard {
        AgentRunLedgerProgressDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentRunLedgerProgressHealthPolicy,
    ) -> AgentRunLedgerProgressHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunLedgerProgressDashboard {
    pub total_records: usize,
    pub closable_records: usize,
    pub repair_first_records: usize,
    pub total_assigned_tasks: usize,
    pub total_reported_tasks: usize,
    pub total_accepted_results: usize,
    pub total_rejected_results: usize,
    pub total_dispatch_rejections: usize,
    pub total_missing_assigned_tasks: usize,
    pub total_unassigned_results: usize,
    pub empty_dispatch_records: usize,
    pub close_rate: f32,
    pub repair_first_rate: f32,
    pub latest_can_close_run: Option<bool>,
    pub telemetry: Vec<String>,
}

impl AgentRunLedgerProgressDashboard {
    pub fn from_summaries(summaries: &[AgentRunLedgerProgressSummary]) -> Self {
        let total_records = summaries.len();
        let closable_records = summaries
            .iter()
            .filter(|summary| summary.can_close_run && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let total_assigned_tasks = summaries
            .iter()
            .map(|summary| summary.assigned_tasks)
            .sum::<usize>();
        let total_reported_tasks = summaries
            .iter()
            .map(|summary| summary.reported_tasks)
            .sum::<usize>();
        let total_accepted_results = summaries
            .iter()
            .map(|summary| summary.accepted_results)
            .sum::<usize>();
        let total_rejected_results = summaries
            .iter()
            .map(|summary| summary.rejected_results)
            .sum::<usize>();
        let total_dispatch_rejections = summaries
            .iter()
            .map(|summary| summary.dispatch_rejections)
            .sum::<usize>();
        let total_missing_assigned_tasks = summaries
            .iter()
            .map(|summary| summary.missing_assigned_tasks)
            .sum::<usize>();
        let total_unassigned_results = summaries
            .iter()
            .map(|summary| summary.unassigned_results)
            .sum::<usize>();
        let empty_dispatch_records = summaries
            .iter()
            .filter(|summary| summary.empty_dispatch)
            .count();
        let close_rate = rate(closable_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let latest_can_close_run = summaries.last().map(|summary| summary.can_close_run);
        let telemetry = agent_run_ledger_progress_dashboard_telemetry(
            total_records,
            closable_records,
            repair_first_records,
            total_assigned_tasks,
            total_reported_tasks,
            total_accepted_results,
            total_rejected_results,
            total_dispatch_rejections,
            total_missing_assigned_tasks,
            total_unassigned_results,
            empty_dispatch_records,
            close_rate,
            repair_first_rate,
            latest_can_close_run,
        );

        Self {
            total_records,
            closable_records,
            repair_first_records,
            total_assigned_tasks,
            total_reported_tasks,
            total_accepted_results,
            total_rejected_results,
            total_dispatch_rejections,
            total_missing_assigned_tasks,
            total_unassigned_results,
            empty_dispatch_records,
            close_rate,
            repair_first_rate,
            latest_can_close_run,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: AgentRunLedgerProgressHealthPolicy,
    ) -> AgentRunLedgerProgressHealth {
        AgentRunLedgerProgressHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentRunLedgerProgressHealthPolicy {
    pub minimum_close_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_rejected_results: usize,
    pub maximum_dispatch_rejections: usize,
    pub maximum_missing_assigned_tasks: usize,
    pub maximum_unassigned_results: usize,
    pub maximum_empty_dispatch_records: usize,
}

impl Default for AgentRunLedgerProgressHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_close_rate: 1.0,
            maximum_repair_first_records: 0,
            maximum_rejected_results: 0,
            maximum_dispatch_rejections: 0,
            maximum_missing_assigned_tasks: 0,
            maximum_unassigned_results: 0,
            maximum_empty_dispatch_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunLedgerProgressHealth {
    pub status: AgentRunReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentRunLedgerProgressDashboard,
}

impl AgentRunLedgerProgressHealth {
    pub fn from_dashboard(
        dashboard: AgentRunLedgerProgressDashboard,
        policy: AgentRunLedgerProgressHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("agent_run_ledger_progress_history_empty".to_owned());
        } else if dashboard.close_rate < policy.minimum_close_rate {
            watch_reasons.push(format!(
                "agent_run_ledger_progress_close_rate={:.3}<{}",
                dashboard.close_rate, policy.minimum_close_rate
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "agent_run_ledger_progress_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }
        if dashboard.total_rejected_results > policy.maximum_rejected_results {
            repair_reasons.push(format!(
                "agent_run_ledger_progress_rejected_results={}>{}",
                dashboard.total_rejected_results, policy.maximum_rejected_results
            ));
        }
        if dashboard.total_dispatch_rejections > policy.maximum_dispatch_rejections {
            repair_reasons.push(format!(
                "agent_run_ledger_progress_dispatch_rejections={}>{}",
                dashboard.total_dispatch_rejections, policy.maximum_dispatch_rejections
            ));
        }
        if dashboard.total_missing_assigned_tasks > policy.maximum_missing_assigned_tasks {
            repair_reasons.push(format!(
                "agent_run_ledger_progress_missing_assigned_tasks={}>{}",
                dashboard.total_missing_assigned_tasks, policy.maximum_missing_assigned_tasks
            ));
        }
        if dashboard.total_unassigned_results > policy.maximum_unassigned_results {
            repair_reasons.push(format!(
                "agent_run_ledger_progress_unassigned_results={}>{}",
                dashboard.total_unassigned_results, policy.maximum_unassigned_results
            ));
        }
        if dashboard.empty_dispatch_records > policy.maximum_empty_dispatch_records {
            repair_reasons.push(format!(
                "agent_run_ledger_progress_empty_dispatch_records={}>{}",
                dashboard.empty_dispatch_records, policy.maximum_empty_dispatch_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentRunReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentRunReportHealthStatus::Watch, watch_reasons)
        } else {
            (AgentRunReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentRunReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentRunReportHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunLedgerProgressSummaryHistoryRecord {
    pub history: AgentRunLedgerProgressSummaryHistory,
    pub appended_summary: AgentRunLedgerProgressSummary,
    pub dashboard: AgentRunLedgerProgressDashboard,
    pub health: AgentRunLedgerProgressHealth,
    pub telemetry: Vec<String>,
}

impl AgentRunLedgerProgressSummaryHistoryRecord {
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
pub struct AgentRunLedgerProgressSummaryHistoryRecorder;

impl AgentRunLedgerProgressSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentRunLedgerProgressSummaryHistory,
        summary: AgentRunLedgerProgressSummary,
        policy: AgentRunLedgerProgressHealthPolicy,
    ) -> AgentRunLedgerProgressSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = agent_run_ledger_progress_history_record_telemetry(&dashboard, &health);

        AgentRunLedgerProgressSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_progress_with_health(
        &self,
        history: AgentRunLedgerProgressSummaryHistory,
        progress: &AgentRunLedgerProgress,
        policy: AgentRunLedgerProgressHealthPolicy,
    ) -> AgentRunLedgerProgressSummaryHistoryRecord {
        self.record_summary_with_health(history, progress.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunLedgerAdmission {
    pub can_build_ledger: bool,
    pub can_admit_side_effects: bool,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentRunLedgerAdmission {
    pub fn from_dispatch_gate(decision: &TaskDispatchGateDecision) -> Self {
        let can_build_ledger = decision.can_dispatch && !decision.requires_repair_first;
        let can_admit_side_effects = can_build_ledger;
        let can_submit_memory_note = can_build_ledger;
        let can_promote_adaptive_state = can_build_ledger;
        let requires_repair_first = !can_build_ledger;
        let reasons = if can_build_ledger {
            Vec::new()
        } else if decision.reasons.is_empty() {
            vec!["run_ledger_dispatch_closed".to_owned()]
        } else {
            decision.reasons.clone()
        };
        let telemetry = agent_run_ledger_admission_telemetry(
            can_build_ledger,
            can_admit_side_effects,
            can_submit_memory_note,
            can_promote_adaptive_state,
            requires_repair_first,
            reasons.len(),
        );

        Self {
            can_build_ledger,
            can_admit_side_effects,
            can_submit_memory_note,
            can_promote_adaptive_state,
            requires_repair_first,
            reasons,
            telemetry,
        }
    }

    pub fn is_admitted(&self) -> bool {
        self.can_build_ledger && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunLedger {
    dispatch: TaskDispatchPlan,
    results: BTreeMap<String, AgentResult>,
}

impl AgentRunLedger {
    pub fn new(dispatch: TaskDispatchPlan) -> Self {
        Self {
            dispatch,
            results: BTreeMap::new(),
        }
    }

    pub fn admission(dispatch_gate: &TaskDispatchGateDecision) -> AgentRunLedgerAdmission {
        AgentRunLedgerAdmission::from_dispatch_gate(dispatch_gate)
    }

    pub fn try_from_dispatch(dispatch: TaskDispatchPlan) -> Result<Self, AgentRunLedgerAdmission> {
        let admission = Self::admission(&dispatch.gate());
        if admission.is_admitted() {
            Ok(Self::new(dispatch))
        } else {
            Err(admission)
        }
    }

    pub fn dispatch(&self) -> &TaskDispatchPlan {
        &self.dispatch
    }

    pub fn record_result(&mut self, result: AgentResult) {
        self.results.insert(result.task_id.clone(), result);
    }

    pub fn result(&self, task_id: &str) -> Option<&AgentResult> {
        self.results.get(task_id)
    }

    pub fn ordered_results(&self) -> Vec<&AgentResult> {
        let mut seen = BTreeSet::new();
        let mut ordered = Vec::new();

        for assignment in &self.dispatch.assignments {
            if let Some(result) = self.results.get(&assignment.task_id) {
                seen.insert(assignment.task_id.clone());
                ordered.push(result);
            }
        }

        for (task_id, result) in &self.results {
            if seen.insert(task_id.clone()) {
                ordered.push(result);
            }
        }

        ordered
    }

    pub fn ordered_messages(&self) -> Vec<AgentMessage> {
        self.ordered_results()
            .into_iter()
            .flat_map(|result| result.messages.clone())
            .collect()
    }

    pub fn progress(&self) -> AgentRunLedgerProgress {
        let assigned_tasks = self.dispatch.assignments.len();
        let reported_tasks = self.results.len();
        let assigned_ids = self
            .dispatch
            .assignments
            .iter()
            .map(|assignment| assignment.task_id.clone())
            .collect::<BTreeSet<_>>();
        let missing_task_ids = self
            .dispatch
            .assignments
            .iter()
            .filter(|assignment| !self.results.contains_key(&assignment.task_id))
            .map(|assignment| assignment.task_id.clone())
            .collect::<Vec<_>>();
        let rejected_task_ids = self
            .ordered_results()
            .into_iter()
            .filter(|result| !result.accepted)
            .map(|result| result.task_id.clone())
            .collect::<Vec<_>>();
        let unassigned_task_ids = self
            .results
            .keys()
            .filter(|task_id| !assigned_ids.contains(*task_id))
            .cloned()
            .collect::<Vec<_>>();
        let accepted_results = self
            .results
            .values()
            .filter(|result| result.accepted)
            .count();
        let rejected_results = rejected_task_ids.len();
        let dispatch_rejections = self.dispatch.rejections.len();
        let missing_assigned_tasks = missing_task_ids.len();
        let unassigned_results = unassigned_task_ids.len();
        let empty_dispatch = assigned_tasks == 0;
        let can_close_run = !empty_dispatch
            && dispatch_rejections == 0
            && missing_assigned_tasks == 0
            && rejected_results == 0
            && unassigned_results == 0;
        let requires_repair_first = !can_close_run;
        let telemetry = agent_run_ledger_progress_telemetry(
            assigned_tasks,
            reported_tasks,
            accepted_results,
            rejected_results,
            dispatch_rejections,
            missing_assigned_tasks,
            unassigned_results,
            empty_dispatch,
            can_close_run,
            requires_repair_first,
        );

        AgentRunLedgerProgress {
            assigned_tasks,
            reported_tasks,
            accepted_results,
            rejected_results,
            dispatch_rejections,
            missing_assigned_tasks,
            unassigned_results,
            missing_task_ids,
            rejected_task_ids,
            unassigned_task_ids,
            empty_dispatch,
            can_close_run,
            requires_repair_first,
            telemetry,
        }
    }

    pub fn aggregation_report(&self) -> AggregationReport {
        MessageAggregator::new().aggregate(self.ordered_messages())
    }

    pub fn conflict_report(&self) -> ConflictReport {
        ConflictResolver::new().mark_conflicts(&self.ordered_messages())
    }

    pub fn conflict_report_with_resolutions(
        &self,
        resolutions: &ConflictResolutionBook,
    ) -> ConflictReport {
        resolutions.resolve_report(&self.conflict_report())
    }

    pub fn budget_audit(&self) -> RunBudgetAudit {
        let mut overspends = Vec::new();
        for assignment in &self.dispatch.assignments {
            let Some(result) = self.results.get(&assignment.task_id) else {
                continue;
            };
            if !assignment.budget_reserved.fits(result.budget_spent) {
                overspends.push(RunBudgetOverspend {
                    task_id: assignment.task_id.clone(),
                    role: assignment.role.clone(),
                    reserved: assignment.budget_reserved,
                    spent: result.budget_spent,
                });
            }
        }
        RunBudgetAudit { overspends }
    }

    pub fn gate_side_effect(
        &self,
        kind: SideEffectKind,
        conflicts: &ConflictReport,
        reflection: Option<&ReflectionLoop>,
    ) -> SideEffectGate {
        let progress = self.progress();
        if !progress.can_close_run {
            return SideEffectGate::block(kind, agent_run_ledger_progress_block_reason(&progress));
        }

        if conflicts.has_unresolved_conflicts() {
            return SideEffectGate::block(
                kind,
                format!(
                    "blocked by {} unresolved conflict(s)",
                    conflicts.unresolved_count()
                ),
            );
        }

        match kind {
            SideEffectKind::MemoryNote => gate_memory_note(reflection),
            SideEffectKind::FileWrite => SideEffectGate::allow(kind, "no unresolved conflicts"),
            SideEffectKind::AdaptiveStateWrite => {
                SideEffectGate::allow(kind, "no unresolved conflicts")
            }
            SideEffectKind::ExternalCall => SideEffectGate::allow(kind, "no unresolved conflicts"),
        }
    }

    pub fn report(&self, reflection: Option<&ReflectionLoop>) -> AgentRunReport {
        self.report_with_resolutions(reflection, &ConflictResolutionBook::new())
    }

    pub fn try_close_report(&self, reflection: Option<&ReflectionLoop>) -> Option<AgentRunReport> {
        if self.progress().can_close_run {
            Some(self.report(reflection))
        } else {
            None
        }
    }

    pub fn report_with_resolutions(
        &self,
        reflection: Option<&ReflectionLoop>,
        resolutions: &ConflictResolutionBook,
    ) -> AgentRunReport {
        let aggregation = self.aggregation_report();
        let conflicts = resolutions.resolve_report(
            &ConflictResolver::new().mark_conflicts(
                &aggregation
                    .messages
                    .iter()
                    .map(|message| message.message.clone())
                    .collect::<Vec<_>>(),
            ),
        );
        let budget_audit = self.budget_audit();
        let side_effects = [
            SideEffectKind::MemoryNote,
            SideEffectKind::FileWrite,
            SideEffectKind::AdaptiveStateWrite,
            SideEffectKind::ExternalCall,
        ]
        .into_iter()
        .map(|kind| self.gate_side_effect(kind, &conflicts, reflection))
        .collect();

        AgentRunReport {
            aggregation,
            conflicts,
            budget_audit,
            side_effects,
        }
    }
}

fn gate_memory_note(reflection: Option<&ReflectionLoop>) -> SideEffectGate {
    let Some(reflection) = reflection else {
        return SideEffectGate::block(
            SideEffectKind::MemoryNote,
            "memory note requires a reflection loop",
        );
    };
    if !reflection.is_complete() {
        return SideEffectGate::block(
            SideEffectKind::MemoryNote,
            "memory note requires a complete reflection loop",
        );
    }
    if reflection.memory_note().is_none() {
        return SideEffectGate::block(
            SideEffectKind::MemoryNote,
            "memory note requires a reflection memory note entry",
        );
    }
    SideEffectGate::allow(
        SideEffectKind::MemoryNote,
        "reflection complete and no unresolved conflicts",
    )
}

fn side_effect_allowed(gates: &[SideEffectGate], kind: SideEffectKind) -> bool {
    gates.iter().any(|gate| gate.kind == kind && gate.allowed)
}

#[allow(clippy::too_many_arguments)]
fn agent_run_ledger_progress_telemetry(
    assigned_tasks: usize,
    reported_tasks: usize,
    accepted_results: usize,
    rejected_results: usize,
    dispatch_rejections: usize,
    missing_assigned_tasks: usize,
    unassigned_results: usize,
    empty_dispatch: bool,
    can_close_run: bool,
    requires_repair_first: bool,
) -> Vec<String> {
    vec![
        "agent_run_ledger_progress=true".to_owned(),
        format!("agent_run_ledger_progress_assigned_tasks={assigned_tasks}"),
        format!("agent_run_ledger_progress_reported_tasks={reported_tasks}"),
        format!("agent_run_ledger_progress_accepted_results={accepted_results}"),
        format!("agent_run_ledger_progress_rejected_results={rejected_results}"),
        format!("agent_run_ledger_progress_dispatch_rejections={dispatch_rejections}"),
        format!("agent_run_ledger_progress_missing_assigned_tasks={missing_assigned_tasks}"),
        format!("agent_run_ledger_progress_unassigned_results={unassigned_results}"),
        format!("agent_run_ledger_progress_empty_dispatch={empty_dispatch}"),
        format!("agent_run_ledger_progress_can_close_run={can_close_run}"),
        format!("agent_run_ledger_progress_requires_repair_first={requires_repair_first}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_run_ledger_progress_summary_telemetry(
    assigned_tasks: usize,
    reported_tasks: usize,
    accepted_results: usize,
    rejected_results: usize,
    dispatch_rejections: usize,
    missing_assigned_tasks: usize,
    unassigned_results: usize,
    empty_dispatch: bool,
    can_close_run: bool,
    requires_repair_first: bool,
) -> Vec<String> {
    vec![
        "agent_run_ledger_progress_summary=true".to_owned(),
        format!("agent_run_ledger_progress_summary_assigned_tasks={assigned_tasks}"),
        format!("agent_run_ledger_progress_summary_reported_tasks={reported_tasks}"),
        format!("agent_run_ledger_progress_summary_accepted_results={accepted_results}"),
        format!("agent_run_ledger_progress_summary_rejected_results={rejected_results}"),
        format!("agent_run_ledger_progress_summary_dispatch_rejections={dispatch_rejections}"),
        format!(
            "agent_run_ledger_progress_summary_missing_assigned_tasks={missing_assigned_tasks}"
        ),
        format!("agent_run_ledger_progress_summary_unassigned_results={unassigned_results}"),
        format!("agent_run_ledger_progress_summary_empty_dispatch={empty_dispatch}"),
        format!("agent_run_ledger_progress_summary_can_close_run={can_close_run}"),
        format!("agent_run_ledger_progress_summary_requires_repair_first={requires_repair_first}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_run_ledger_progress_dashboard_telemetry(
    total_records: usize,
    closable_records: usize,
    repair_first_records: usize,
    total_assigned_tasks: usize,
    total_reported_tasks: usize,
    total_accepted_results: usize,
    total_rejected_results: usize,
    total_dispatch_rejections: usize,
    total_missing_assigned_tasks: usize,
    total_unassigned_results: usize,
    empty_dispatch_records: usize,
    close_rate: f32,
    repair_first_rate: f32,
    latest_can_close_run: Option<bool>,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_ledger_progress_dashboard=true".to_owned(),
        format!("agent_run_ledger_progress_dashboard_records={total_records}"),
        format!("agent_run_ledger_progress_dashboard_closable_records={closable_records}"),
        format!("agent_run_ledger_progress_dashboard_repair_first_records={repair_first_records}"),
        format!("agent_run_ledger_progress_dashboard_assigned_tasks={total_assigned_tasks}"),
        format!("agent_run_ledger_progress_dashboard_reported_tasks={total_reported_tasks}"),
        format!("agent_run_ledger_progress_dashboard_accepted_results={total_accepted_results}"),
        format!("agent_run_ledger_progress_dashboard_rejected_results={total_rejected_results}"),
        format!(
            "agent_run_ledger_progress_dashboard_dispatch_rejections={total_dispatch_rejections}"
        ),
        format!(
            "agent_run_ledger_progress_dashboard_missing_assigned_tasks={total_missing_assigned_tasks}"
        ),
        format!(
            "agent_run_ledger_progress_dashboard_unassigned_results={total_unassigned_results}"
        ),
        format!(
            "agent_run_ledger_progress_dashboard_empty_dispatch_records={empty_dispatch_records}"
        ),
        format!("agent_run_ledger_progress_dashboard_close_rate={close_rate:.3}"),
        format!("agent_run_ledger_progress_dashboard_repair_first_rate={repair_first_rate:.3}"),
    ];
    if let Some(latest_can_close_run) = latest_can_close_run {
        telemetry.push(format!(
            "agent_run_ledger_progress_dashboard_latest_can_close_run={latest_can_close_run}"
        ));
    }
    telemetry
}

fn agent_run_ledger_progress_history_record_telemetry(
    dashboard: &AgentRunLedgerProgressDashboard,
    health: &AgentRunLedgerProgressHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_ledger_progress_history_record=true".to_owned(),
        format!(
            "agent_run_ledger_progress_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_run_ledger_progress_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_run_ledger_progress_history_record_close_rate={:.3}",
            dashboard.close_rate
        ),
        format!(
            "agent_run_ledger_progress_history_record_repair_first_records={}",
            dashboard.repair_first_records
        ),
        format!(
            "agent_run_ledger_progress_history_record_missing_assigned_tasks={}",
            dashboard.total_missing_assigned_tasks
        ),
        format!(
            "agent_run_ledger_progress_history_record_rejected_results={}",
            dashboard.total_rejected_results
        ),
        format!(
            "agent_run_ledger_progress_history_record_dispatch_rejections={}",
            dashboard.total_dispatch_rejections
        ),
        format!(
            "agent_run_ledger_progress_history_record_unassigned_results={}",
            dashboard.total_unassigned_results
        ),
        format!(
            "agent_run_ledger_progress_history_record_empty_dispatch_records={}",
            dashboard.empty_dispatch_records
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_run_ledger_progress_history_record_reason={reason}")),
    );
    telemetry
}

fn agent_run_ledger_progress_block_reason(progress: &AgentRunLedgerProgress) -> String {
    format!(
        "blocked by run ledger progress missing={} rejected={} dispatch_rejections={} unassigned={} empty_dispatch={}",
        progress.missing_assigned_tasks,
        progress.rejected_results,
        progress.dispatch_rejections,
        progress.unassigned_results,
        progress.empty_dispatch
    )
}

fn agent_run_report_health_repair_task(run_id: &str, index: usize, reason: String) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-run-report-health-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair agent run report health: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("agent-run-report-health")
    .with_priority(9)
}

fn agent_run_progress_report_gate_progress_reasons(
    progress_record: &AgentRunLedgerProgressSummaryHistoryRecord,
) -> Vec<String> {
    let mut reasons = progress_record
        .health
        .reasons
        .iter()
        .map(|reason| format!("progress:{reason}"))
        .collect::<Vec<_>>();
    if reasons.is_empty() && progress_record.appended_summary.requires_repair_first {
        reasons.push("progress:agent_run_ledger_progress_current_requires_repair_first".to_owned());
    }
    reasons
}

fn agent_run_progress_report_gate_repair_tasks(
    run_id: &str,
    requires_repair_first: bool,
    reasons: &[String],
) -> Vec<AgentTask> {
    if !requires_repair_first {
        return Vec::new();
    }

    reasons
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!(
                    "agent-run-ledger-progress-repair-{}-{}-{}",
                    stable_id(run_id),
                    index,
                    stable_id(&reason)
                ),
                AgentRole::Reviewer,
                format!("repair agent run ledger progress before report gate: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("agent-run-ledger-progress")
            .with_priority(10)
        })
        .collect()
}

fn agent_run_report_health_gate_trend_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-run-report-health-gate-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair agent run report health gate trend: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("agent-run-report-health-gate")
    .with_priority(9)
}

fn agent_run_report_health_gate_trend_handoff_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-run-report-health-gate-handoff-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair agent run report health gate trend handoff: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("agent-run-report-health-gate-handoff")
    .with_priority(9)
}

fn agent_run_report_health_gate_trend_handoff_monitor_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-run-report-health-gate-handoff-monitor-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair agent run report health gate trend handoff monitor: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("agent-run-report-health-gate-handoff-monitor")
    .with_priority(9)
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-run-report-health-gate-handoff-monitor-handoff-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair agent run report health gate trend handoff monitor handoff: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("agent-run-report-health-gate-handoff-monitor-handoff")
    .with_priority(9)
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-run-report-health-gate-handoff-monitor-handoff-handoff-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!(
            "repair agent run report health gate trend handoff monitor handoff packet: {reason}"
        ),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("agent-run-report-health-gate-handoff-monitor-handoff-handoff")
    .with_priority(9)
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-run-report-health-gate-handoff-monitor-handoff-handoff-handoff-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!(
            "repair agent run report health gate trend handoff monitor final admission: {reason}"
        ),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("agent-run-report-health-gate-handoff-monitor-handoff-handoff-handoff")
    .with_priority(9)
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

#[allow(clippy::too_many_arguments)]
fn agent_run_report_summary_telemetry(
    input_messages: usize,
    unique_messages: usize,
    duplicate_groups: usize,
    unresolved_conflicts: usize,
    budget_overspends: usize,
    side_effects: usize,
    allowed_side_effects: usize,
    blocked_side_effects: usize,
    memory_note_allowed: bool,
    adaptive_state_allowed: bool,
    external_call_allowed: bool,
    all_side_effects_allowed: bool,
) -> Vec<String> {
    vec![
        "agent_run_report_summary=true".to_owned(),
        format!("agent_run_report_summary_input_messages={input_messages}"),
        format!("agent_run_report_summary_unique_messages={unique_messages}"),
        format!("agent_run_report_summary_duplicate_groups={duplicate_groups}"),
        format!("agent_run_report_summary_unresolved_conflicts={unresolved_conflicts}"),
        format!("agent_run_report_summary_budget_overspends={budget_overspends}"),
        format!("agent_run_report_summary_side_effects={side_effects}"),
        format!("agent_run_report_summary_allowed_side_effects={allowed_side_effects}"),
        format!("agent_run_report_summary_blocked_side_effects={blocked_side_effects}"),
        format!("agent_run_report_summary_memory_note_allowed={memory_note_allowed}"),
        format!("agent_run_report_summary_adaptive_state_allowed={adaptive_state_allowed}"),
        format!("agent_run_report_summary_external_call_allowed={external_call_allowed}"),
        format!("agent_run_report_summary_all_side_effects_allowed={all_side_effects_allowed}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_run_report_dashboard_telemetry(
    total_runs: usize,
    clean_runs: usize,
    conflict_runs: usize,
    budget_overspend_runs: usize,
    side_effect_blocked_runs: usize,
    memory_note_admitted_runs: usize,
    adaptive_state_admitted_runs: usize,
    external_call_admitted_runs: usize,
    total_unresolved_conflicts: usize,
    total_budget_overspends: usize,
    total_blocked_side_effects: usize,
    clean_rate: f32,
    memory_note_admission_rate: f32,
    adaptive_state_admission_rate: f32,
    external_call_admission_rate: f32,
    latest_all_side_effects_allowed: Option<bool>,
) -> Vec<String> {
    vec![
        "agent_run_report_dashboard=true".to_owned(),
        format!("agent_run_report_dashboard_runs={total_runs}"),
        format!("agent_run_report_dashboard_clean_runs={clean_runs}"),
        format!("agent_run_report_dashboard_conflict_runs={conflict_runs}"),
        format!("agent_run_report_dashboard_budget_overspend_runs={budget_overspend_runs}"),
        format!("agent_run_report_dashboard_side_effect_blocked_runs={side_effect_blocked_runs}"),
        format!("agent_run_report_dashboard_memory_note_admitted={memory_note_admitted_runs}"),
        format!(
            "agent_run_report_dashboard_adaptive_state_admitted={adaptive_state_admitted_runs}"
        ),
        format!("agent_run_report_dashboard_external_call_admitted={external_call_admitted_runs}"),
        format!("agent_run_report_dashboard_unresolved_conflicts={total_unresolved_conflicts}"),
        format!("agent_run_report_dashboard_budget_overspends={total_budget_overspends}"),
        format!("agent_run_report_dashboard_blocked_side_effects={total_blocked_side_effects}"),
        format!("agent_run_report_dashboard_clean_rate={clean_rate:.3}"),
        format!(
            "agent_run_report_dashboard_memory_note_admission_rate={memory_note_admission_rate:.3}"
        ),
        format!(
            "agent_run_report_dashboard_adaptive_state_admission_rate={adaptive_state_admission_rate:.3}"
        ),
        format!(
            "agent_run_report_dashboard_external_call_admission_rate={external_call_admission_rate:.3}"
        ),
        format!(
            "agent_run_report_dashboard_latest_all_side_effects_allowed={}",
            latest_all_side_effects_allowed
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned())
        ),
    ]
}

fn agent_run_report_history_record_telemetry(
    dashboard: &AgentRunReportDashboard,
    health: &AgentRunReportHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_history_record=true".to_owned(),
        format!(
            "agent_run_report_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_run_report_history_record_runs={}",
            dashboard.total_runs
        ),
        format!(
            "agent_run_report_history_record_clean_rate={:.3}",
            dashboard.clean_rate
        ),
        format!(
            "agent_run_report_history_record_memory_note_admission_rate={:.3}",
            dashboard.memory_note_admission_rate
        ),
        format!(
            "agent_run_report_history_record_adaptive_state_admission_rate={:.3}",
            dashboard.adaptive_state_admission_rate
        ),
        format!(
            "agent_run_report_history_record_conflict_runs={}",
            dashboard.conflict_runs
        ),
        format!(
            "agent_run_report_history_record_budget_overspend_runs={}",
            dashboard.budget_overspend_runs
        ),
        format!(
            "agent_run_report_history_record_side_effect_blocked_runs={}",
            dashboard.side_effect_blocked_runs
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_run_report_history_record_reason={reason}")),
    );
    telemetry
}

fn agent_run_report_health_gate_telemetry(
    health_status: AgentRunReportHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate=true".to_owned(),
        format!(
            "agent_run_report_health_gate_status={}",
            health_status.as_str()
        ),
        format!("agent_run_report_health_gate_admitted={admitted}"),
        format!("agent_run_report_health_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_run_report_health_gate_repair_tasks={repair_tasks}"),
        format!("agent_run_report_health_gate_next_queue_tasks={next_queue_tasks}"),
        format!(
            "agent_run_report_health_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        blocked_reasons
            .iter()
            .map(|reason| format!("agent_run_report_health_gate_reason={reason}")),
    );
    telemetry
}

fn agent_run_report_health_gate_trend_gate_telemetry(
    health_status: AgentRunReportHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_gate=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_gate_status={}",
            health_status.as_str()
        ),
        format!("agent_run_report_health_gate_trend_gate_admitted={admitted}"),
        format!(
            "agent_run_report_health_gate_trend_gate_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_run_report_health_gate_trend_gate_repair_tasks={repair_tasks}"),
        format!("agent_run_report_health_gate_trend_gate_next_queue_tasks={next_queue_tasks}"),
        format!(
            "agent_run_report_health_gate_trend_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        blocked_reasons
            .iter()
            .map(|reason| format!("agent_run_report_health_gate_trend_gate_reason={reason}")),
    );
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_telemetry(
    trend_record: &AgentRunReportHealthGateHistoryRecord,
    gate_summary: &AgentRunReportHealthGateSummary,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_health_status={}",
            trend_record.health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_records={}",
            trend_record.dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_admitted={}",
            gate_summary.admitted
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_requires_repair_first={}",
            gate_summary.requires_repair_first
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_repair_tasks={}",
            gate_summary.repair_tasks
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_next_queue_tasks={}",
            gate_summary.next_queue_tasks
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_blocked_reasons={}",
            gate_summary.blocked_reasons.len()
        ),
    ];
    telemetry.extend(trend_record.telemetry.iter().cloned());
    telemetry.extend(gate_summary.telemetry.iter().cloned());
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_summary_telemetry(
    trend_health_status: AgentRunReportHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    trend_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_trend_handoff_summary=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_summary_status={}",
            trend_health_status.as_str()
        ),
        format!("agent_run_report_health_gate_trend_handoff_summary_admitted={admitted}"),
        format!(
            "agent_run_report_health_gate_trend_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_run_report_health_gate_trend_handoff_summary_records={trend_records}"),
        format!("agent_run_report_health_gate_trend_handoff_summary_repair_tasks={repair_tasks}"),
        format!(
            "agent_run_report_health_gate_trend_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_run_report_health_gate_trend_handoff_dashboard_telemetry(
    total_records: usize,
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
    latest_trend_health_status: Option<AgentRunReportHealthStatus>,
    latest_blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_trend_handoff_dashboard=true".to_owned(),
        format!("agent_run_report_health_gate_trend_handoff_dashboard_records={total_records}"),
        format!("agent_run_report_health_gate_trend_handoff_dashboard_admitted={admitted_records}"),
        format!(
            "agent_run_report_health_gate_trend_handoff_dashboard_repair_first={repair_first_records}"
        ),
        format!("agent_run_report_health_gate_trend_handoff_dashboard_stable={stable_records}"),
        format!("agent_run_report_health_gate_trend_handoff_dashboard_watch={watch_records}"),
        format!("agent_run_report_health_gate_trend_handoff_dashboard_repair={repair_records}"),
        format!(
            "agent_run_report_health_gate_trend_handoff_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_dashboard_latest_status={}",
            latest_trend_health_status
                .map(AgentRunReportHealthStatus::as_str)
                .unwrap_or("none")
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_dashboard_latest_blocked_reasons={latest_blocked_reasons}"
        ),
    ]
}

fn agent_run_report_health_gate_trend_handoff_history_record_telemetry(
    dashboard: &AgentRunReportHealthGateTrendHandoffDashboard,
    health: &AgentRunReportHealthGateTrendHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_history_record=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!("agent_run_report_health_gate_trend_handoff_history_record_reason={reason}")
    }));
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_gate_telemetry(
    handoff_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_gate=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_gate_status={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_gate_requested_admitted={requested_admitted}"
        ),
        format!("agent_run_report_health_gate_trend_handoff_gate_admitted={admitted}"),
        format!(
            "agent_run_report_health_gate_trend_handoff_gate_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_run_report_health_gate_trend_handoff_gate_repair_tasks={repair_tasks}"),
        format!(
            "agent_run_report_health_gate_trend_handoff_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        blocked_reasons.iter().map(|reason| {
            format!("agent_run_report_health_gate_trend_handoff_gate_reason={reason}")
        }),
    );
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_monitor_telemetry(
    handoff: &AgentRunReportHealthGateTrendHandoffRecord,
    history_record: &AgentRunReportHealthGateTrendHandoffHistoryRecord,
    gate_decision: &AgentRunReportHealthGateTrendHandoffGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_status={}",
            handoff.trend_record.health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_monitor_summary_telemetry(
    handoff_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    handoff_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_trend_handoff_monitor_summary=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_summary_status={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_summary_requested_admitted={requested_admitted}"
        ),
        format!("agent_run_report_health_gate_trend_handoff_monitor_summary_admitted={admitted}"),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_summary_records={handoff_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_run_report_health_gate_trend_handoff_monitor_dashboard_telemetry(
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
    latest_handoff_health_status: Option<AgentRunReportHealthStatus>,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_trend_handoff_monitor_dashboard=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_records={total_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_requested_admitted={requested_admitted_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_dashboard_latest_status={}",
            latest_handoff_health_status
                .map(AgentRunReportHealthStatus::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn agent_run_report_health_gate_trend_handoff_monitor_history_record_telemetry(
    dashboard: &AgentRunReportHealthGateTrendHandoffMonitorDashboard,
    health: &AgentRunReportHealthGateTrendHandoffMonitorHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_history_record=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_history_record_repair_records={}",
            dashboard.repair_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_history_record_watch_records={}",
            dashboard.watch_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!("agent_run_report_health_gate_trend_handoff_monitor_history_record_reason={reason}")
    }));
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_monitor_gate_telemetry(
    monitor_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_gate=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_gate_status={}",
            monitor_health_status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_gate_requested_admitted={requested_admitted}"
        ),
        format!("agent_run_report_health_gate_trend_handoff_monitor_gate_admitted={admitted}"),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!("agent_run_report_health_gate_trend_handoff_monitor_gate_reason={reason}")
    }));
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_telemetry(
    monitor: &AgentRunReportHealthGateTrendHandoffMonitorRecord,
    history_record: &AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecord,
    gate_decision: &AgentRunReportHealthGateTrendHandoffMonitorGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(monitor.telemetry.iter().cloned());
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_telemetry(
    handoff_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_gate=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_status={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_admitted={admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!("agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_reason={reason}")
    }));
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_telemetry(
    handoff: &AgentRunReportHealthGateTrendHandoffMonitorHandoffRecord,
    history_record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord,
    gate_decision: &AgentRunReportHealthGateTrendHandoffMonitorHandoffGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(handoff.telemetry.iter().cloned());
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_telemetry(
    handoff_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    handoff_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary=true"
            .to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_status={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_admitted={admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_records={handoff_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_telemetry(
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
    latest_handoff_health_status: Option<AgentRunReportHealthStatus>,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard=true"
            .to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_records={total_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_requested_admitted={requested_admitted_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_latest_status={}",
            latest_handoff_health_status
                .map(AgentRunReportHealthStatus::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_telemetry(
    dashboard: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffDashboard,
    health: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record=true"
            .to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_repair_records={}",
            dashboard.repair_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_watch_records={}",
            dashboard.watch_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_reason={reason}"
        )
    }));
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_telemetry(
    packet_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_status={}",
            packet_health_status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_admitted={admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_reason={reason}"
        )
    }));
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_telemetry(
    packet: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffRecord,
    history_record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord,
    gate_decision: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff=true"
            .to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(packet.telemetry.iter().cloned());
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_telemetry(
    packet_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    packet_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary=true"
            .to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_status={}",
            packet_health_status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_admitted={admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_records={packet_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_telemetry(
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
    latest_packet_health_status: Option<AgentRunReportHealthStatus>,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard=true"
            .to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_records={total_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_requested_admitted={requested_admitted_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_latest_status={}",
            latest_packet_health_status
                .map(AgentRunReportHealthStatus::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_telemetry(
    dashboard: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard,
    health: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record=true"
            .to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_repair_records={}",
            dashboard.repair_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_watch_records={}",
            dashboard.watch_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_reason={reason}"
        )
    }));
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_telemetry(
    admission_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate=true"
            .to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_status={}",
            admission_health_status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_admitted={admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_reason={reason}"
        )
    }));
    telemetry
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_telemetry(
    admission: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
    history_record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord,
    gate_decision: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff=true"
            .to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(admission.telemetry.iter().cloned());
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_telemetry(
    admission_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    admission_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary=true"
            .to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_status={}",
            admission_health_status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_admitted={admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_records={admission_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_summary_telemetry(
    monitor_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    monitor_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_summary=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_summary_status={}",
            monitor_health_status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_summary_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_summary_admitted={admitted}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_summary_records={monitor_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_telemetry(
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
    latest_monitor_health_status: Option<AgentRunReportHealthStatus>,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_records={total_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_requested_admitted={requested_admitted_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_dashboard_latest_status={}",
            latest_monitor_health_status
                .map(AgentRunReportHealthStatus::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_telemetry(
    dashboard: &AgentRunReportHealthGateTrendHandoffMonitorHandoffDashboard,
    health: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record=true".to_owned(),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_repair_records={}",
            dashboard.repair_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_watch_records={}",
            dashboard.watch_records
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_reason={reason}"
        )
    }));
    telemetry
}

fn agent_run_report_health_gate_summary_telemetry(
    health_status: AgentRunReportHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_summary=true".to_owned(),
        format!(
            "agent_run_report_health_gate_summary_status={}",
            health_status.as_str()
        ),
        format!("agent_run_report_health_gate_summary_admitted={admitted}"),
        format!(
            "agent_run_report_health_gate_summary_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_run_report_health_gate_summary_repair_tasks={repair_tasks}"),
        format!("agent_run_report_health_gate_summary_next_queue_tasks={next_queue_tasks}"),
        format!("agent_run_report_health_gate_summary_blocked_reasons={blocked_reasons}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_telomere_state_telemetry(
    senescent: bool,
    apoptosis_required: bool,
    remaining_tokens: u32,
    remaining_steps: u32,
    remaining_messages: u32,
    repeated_repair_streak_count: usize,
    loop_risk_signal_count: usize,
    depletion_reason_codes: usize,
) -> Vec<String> {
    let allows_new_side_effects = !senescent && !apoptosis_required;
    vec![
        "agent_telomere_state=true".to_owned(),
        format!("agent_telomere_state_senescent={senescent}"),
        format!("agent_telomere_state_apoptosis_required={apoptosis_required}"),
        format!("agent_telomere_state_remaining_tokens={remaining_tokens}"),
        format!("agent_telomere_state_remaining_steps={remaining_steps}"),
        format!("agent_telomere_state_remaining_messages={remaining_messages}"),
        format!("agent_telomere_state_repair_streak={repeated_repair_streak_count}"),
        format!("agent_telomere_state_loop_risk_signals={loop_risk_signal_count}"),
        format!("agent_telomere_state_depletion_reason_codes={depletion_reason_codes}"),
        format!("agent_telomere_state_memory_promotion_allowed={allows_new_side_effects}"),
        format!("agent_telomere_state_genome_mutation_allowed={allows_new_side_effects}"),
        "agent_telomere_state_raw_payload_present=false".to_owned(),
        "agent_telomere_state_preview_side_effect_allowed=false".to_owned(),
    ]
}

fn agent_preview_digest<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for part in parts {
        for byte in part.bytes().chain([0xff]) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("redaction-digest:{hash:016x}")
}

#[allow(clippy::too_many_arguments)]
fn agent_run_progress_report_gate_telemetry(
    progress_health_status: AgentRunReportHealthStatus,
    report_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    progress_repair_tasks: usize,
    report_repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_progress_report_gate=true".to_owned(),
        format!(
            "agent_run_progress_report_gate_progress_status={}",
            progress_health_status.as_str()
        ),
        format!(
            "agent_run_progress_report_gate_report_status={}",
            report_health_status.as_str()
        ),
        format!("agent_run_progress_report_gate_requested_admitted={requested_admitted}"),
        format!("agent_run_progress_report_gate_admitted={admitted}"),
        format!("agent_run_progress_report_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_run_progress_report_gate_progress_repair_tasks={progress_repair_tasks}"),
        format!("agent_run_progress_report_gate_report_repair_tasks={report_repair_tasks}"),
        format!("agent_run_progress_report_gate_next_queue_tasks={next_queue_tasks}"),
        format!("agent_run_progress_report_gate_blocked_reasons={blocked_reasons}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_run_progress_report_gate_summary_telemetry(
    progress_health_status: AgentRunReportHealthStatus,
    report_health_status: AgentRunReportHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    progress_repair_tasks: usize,
    report_repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_progress_report_gate_summary=true".to_owned(),
        format!(
            "agent_run_progress_report_gate_summary_progress_status={}",
            progress_health_status.as_str()
        ),
        format!(
            "agent_run_progress_report_gate_summary_report_status={}",
            report_health_status.as_str()
        ),
        format!("agent_run_progress_report_gate_summary_requested_admitted={requested_admitted}"),
        format!("agent_run_progress_report_gate_summary_admitted={admitted}"),
        format!(
            "agent_run_progress_report_gate_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_run_progress_report_gate_summary_progress_repair_tasks={progress_repair_tasks}"
        ),
        format!("agent_run_progress_report_gate_summary_report_repair_tasks={report_repair_tasks}"),
        format!("agent_run_progress_report_gate_summary_next_queue_tasks={next_queue_tasks}"),
        format!("agent_run_progress_report_gate_summary_blocked_reasons={blocked_reasons}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_run_progress_report_gate_dashboard_telemetry(
    total_records: usize,
    requested_admitted_records: usize,
    admitted_records: usize,
    repair_first_records: usize,
    progress_stable_records: usize,
    progress_watch_records: usize,
    progress_repair_records: usize,
    report_stable_records: usize,
    report_watch_records: usize,
    report_repair_records: usize,
    progress_repair_tasks: usize,
    report_repair_tasks: usize,
    total_next_queue_tasks: usize,
    blocked_reasons: usize,
    admission_rate: f32,
    repair_first_rate: f32,
    latest_progress_health_status: Option<AgentRunReportHealthStatus>,
    latest_report_health_status: Option<AgentRunReportHealthStatus>,
) -> Vec<String> {
    vec![
        "agent_run_progress_report_gate_dashboard=true".to_owned(),
        format!("agent_run_progress_report_gate_dashboard_records={total_records}"),
        format!(
            "agent_run_progress_report_gate_dashboard_requested_admitted={requested_admitted_records}"
        ),
        format!("agent_run_progress_report_gate_dashboard_admitted={admitted_records}"),
        format!("agent_run_progress_report_gate_dashboard_repair_first={repair_first_records}"),
        format!(
            "agent_run_progress_report_gate_dashboard_progress_stable={progress_stable_records}"
        ),
        format!("agent_run_progress_report_gate_dashboard_progress_watch={progress_watch_records}"),
        format!(
            "agent_run_progress_report_gate_dashboard_progress_repair={progress_repair_records}"
        ),
        format!("agent_run_progress_report_gate_dashboard_report_stable={report_stable_records}"),
        format!("agent_run_progress_report_gate_dashboard_report_watch={report_watch_records}"),
        format!("agent_run_progress_report_gate_dashboard_report_repair={report_repair_records}"),
        format!(
            "agent_run_progress_report_gate_dashboard_progress_repair_tasks={progress_repair_tasks}"
        ),
        format!(
            "agent_run_progress_report_gate_dashboard_report_repair_tasks={report_repair_tasks}"
        ),
        format!(
            "agent_run_progress_report_gate_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!("agent_run_progress_report_gate_dashboard_blocked_reasons={blocked_reasons}"),
        format!("agent_run_progress_report_gate_dashboard_admission_rate={admission_rate:.3}"),
        format!(
            "agent_run_progress_report_gate_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_run_progress_report_gate_dashboard_latest_progress_status={}",
            latest_progress_health_status
                .map(AgentRunReportHealthStatus::as_str)
                .unwrap_or("none")
        ),
        format!(
            "agent_run_progress_report_gate_dashboard_latest_report_status={}",
            latest_report_health_status
                .map(AgentRunReportHealthStatus::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn agent_run_progress_report_gate_history_record_telemetry(
    dashboard: &AgentRunProgressReportGateDashboard,
    health: &AgentRunProgressReportGateHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_progress_report_gate_history_record=true".to_owned(),
        format!(
            "agent_run_progress_report_gate_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_run_progress_report_gate_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_run_progress_report_gate_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_run_progress_report_gate_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_run_progress_report_gate_history_record_progress_repair={}",
            dashboard.progress_repair_records
        ),
        format!(
            "agent_run_progress_report_gate_history_record_report_repair={}",
            dashboard.report_repair_records
        ),
        format!(
            "agent_run_progress_report_gate_history_record_progress_repair_tasks={}",
            dashboard.progress_repair_tasks
        ),
        format!(
            "agent_run_progress_report_gate_history_record_report_repair_tasks={}",
            dashboard.report_repair_tasks
        ),
        format!(
            "agent_run_progress_report_gate_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_run_progress_report_gate_history_record_reason={reason}")),
    );
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn agent_run_report_health_gate_dashboard_telemetry(
    total_records: usize,
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
    latest_health_status: Option<AgentRunReportHealthStatus>,
    latest_blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_dashboard=true".to_owned(),
        format!("agent_run_report_health_gate_dashboard_records={total_records}"),
        format!("agent_run_report_health_gate_dashboard_admitted={admitted_records}"),
        format!("agent_run_report_health_gate_dashboard_repair_first={repair_first_records}"),
        format!("agent_run_report_health_gate_dashboard_stable={stable_records}"),
        format!("agent_run_report_health_gate_dashboard_watch={watch_records}"),
        format!("agent_run_report_health_gate_dashboard_repair={repair_records}"),
        format!("agent_run_report_health_gate_dashboard_repair_tasks={repair_task_count}"),
        format!("agent_run_report_health_gate_dashboard_next_queue_tasks={total_next_queue_tasks}"),
        format!("agent_run_report_health_gate_dashboard_blocked_reasons={blocked_reasons}"),
        format!("agent_run_report_health_gate_dashboard_admission_rate={admission_rate:.3}"),
        format!("agent_run_report_health_gate_dashboard_repair_first_rate={repair_first_rate:.3}"),
        format!(
            "agent_run_report_health_gate_dashboard_latest_status={}",
            latest_health_status
                .map(AgentRunReportHealthStatus::as_str)
                .unwrap_or("none")
        ),
        format!(
            "agent_run_report_health_gate_dashboard_latest_blocked_reasons={latest_blocked_reasons}"
        ),
    ]
}

fn agent_run_report_health_gate_record_telemetry(
    health_record: &AgentRunReportSummaryHistoryRecord,
    gate_summary: &AgentRunReportHealthGateSummary,
) -> Vec<String> {
    vec![
        "agent_run_report_health_gate_record=true".to_owned(),
        format!(
            "agent_run_report_health_gate_record_status={}",
            health_record.health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_record_runs={}",
            health_record.dashboard.total_runs
        ),
        format!(
            "agent_run_report_health_gate_record_admitted={}",
            gate_summary.admitted
        ),
        format!(
            "agent_run_report_health_gate_record_requires_repair_first={}",
            gate_summary.requires_repair_first
        ),
        format!(
            "agent_run_report_health_gate_record_repair_tasks={}",
            gate_summary.repair_tasks
        ),
        format!(
            "agent_run_report_health_gate_record_next_queue_tasks={}",
            gate_summary.next_queue_tasks
        ),
        format!(
            "agent_run_report_health_gate_record_blocked_reasons={}",
            gate_summary.blocked_reasons.len()
        ),
    ]
}

fn agent_run_report_health_gate_history_record_telemetry(
    dashboard: &AgentRunReportHealthGateDashboard,
    health: &AgentRunReportHealthGateHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_run_report_health_gate_history_record=true".to_owned(),
        format!(
            "agent_run_report_health_gate_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_run_report_health_gate_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_run_report_health_gate_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_run_report_health_gate_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_run_report_health_gate_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_run_report_health_gate_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_run_report_health_gate_history_record_reason={reason}")),
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

fn agent_run_gate_telemetry(
    can_promote_memory_note: bool,
    can_write_adaptive_state: bool,
    can_dispatch_external_call: bool,
    requires_repair_first: bool,
    reasons: usize,
    summary: &AgentRunReportSummary,
) -> Vec<String> {
    vec![
        "agent_run_gate=true".to_owned(),
        format!("agent_run_gate_memory_note={can_promote_memory_note}"),
        format!("agent_run_gate_adaptive_state={can_write_adaptive_state}"),
        format!("agent_run_gate_external_call={can_dispatch_external_call}"),
        format!("agent_run_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_run_gate_reasons={reasons}"),
        format!(
            "agent_run_gate_unresolved_conflicts={}",
            summary.unresolved_conflicts
        ),
        format!(
            "agent_run_gate_budget_overspends={}",
            summary.budget_overspends
        ),
        format!(
            "agent_run_gate_blocked_side_effects={}",
            summary.blocked_side_effects
        ),
    ]
}

fn run_budget_audit_summary_telemetry(
    overspends: usize,
    overspent_tokens: u32,
    overspent_steps: u32,
    overspent_messages: u32,
) -> Vec<String> {
    vec![
        "agent_run_budget_audit_summary=true".to_owned(),
        format!("agent_run_budget_audit_summary_overspends={overspends}"),
        format!("agent_run_budget_audit_summary_overspent_tokens={overspent_tokens}"),
        format!("agent_run_budget_audit_summary_overspent_steps={overspent_steps}"),
        format!("agent_run_budget_audit_summary_overspent_messages={overspent_messages}"),
    ]
}

fn agent_run_ledger_admission_telemetry(
    can_build_ledger: bool,
    can_admit_side_effects: bool,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    reasons: usize,
) -> Vec<String> {
    vec![
        "agent_run_ledger_admission=true".to_owned(),
        format!("agent_run_ledger_admission_can_build_ledger={can_build_ledger}"),
        format!("agent_run_ledger_admission_can_admit_side_effects={can_admit_side_effects}"),
        format!("agent_run_ledger_admission_can_submit_memory_note={can_submit_memory_note}"),
        format!(
            "agent_run_ledger_admission_can_promote_adaptive_state={can_promote_adaptive_state}"
        ),
        format!("agent_run_ledger_admission_requires_repair_first={requires_repair_first}"),
        format!("agent_run_ledger_admission_reasons={reasons}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::{AgentBudget, BudgetLedger, BudgetPolicy};
    use crate::message::{AgentMessage, AgentMessageKind};
    use crate::reflection::{ReflectionLoop, ReflectionStage};
    use crate::schedule::RecursiveAgentScheduler;
    use crate::task::{AgentRole, AgentTask, AgentTaskQueue, DispatchPlanner, TaskAssignment};

    fn run_summary(
        unresolved_conflicts: usize,
        budget_overspends: usize,
        blocked_side_effects: usize,
        memory_note_allowed: bool,
        adaptive_state_allowed: bool,
        external_call_allowed: bool,
    ) -> AgentRunReportSummary {
        AgentRunReportSummary {
            input_messages: 2,
            unique_messages: 2,
            duplicate_groups: 0,
            unresolved_conflicts,
            budget_overspends,
            side_effects: 4,
            allowed_side_effects: 4usize.saturating_sub(blocked_side_effects),
            blocked_side_effects,
            memory_note_allowed,
            adaptive_state_allowed,
            external_call_allowed,
            all_side_effects_allowed: blocked_side_effects == 0,
            telemetry: vec![format!(
                "fixture_run_summary_conflicts={unresolved_conflicts}"
            )],
        }
    }

    fn stable_trend_handoff_monitor_record() -> AgentRunReportHealthGateTrendHandoffMonitorRecord {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let run_gate_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/fixture",
            &queue,
        );
        let handoff = AgentRunReportHealthGateTrendHandoff::new().record_gate_record_and_gate(
            AgentRunReportHealthGateHistory::new(),
            &run_gate_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/fixture-handoff",
            &queue,
        );
        AgentRunReportHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentRunReportHealthGateTrendHandoffHistory::new(),
            AgentRunReportHealthGateTrendHandoffHealthPolicy::default(),
            "run/fixture-monitor",
        )
    }

    fn trend_handoff_monitor_summary(
        status: AgentRunReportHealthStatus,
        requested_admitted: bool,
        admitted: bool,
        requires_repair_first: bool,
        repair_tasks: usize,
        next_queue_tasks: usize,
        blocked_reasons: usize,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorSummary {
        AgentRunReportHealthGateTrendHandoffMonitorSummary {
            handoff_health_status: status,
            requested_admitted,
            admitted,
            requires_repair_first,
            handoff_records: 1,
            repair_tasks,
            next_queue_tasks,
            blocked_reasons,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: Vec::new(),
            telemetry: Vec::new(),
        }
    }

    fn trend_handoff_monitor_handoff_summary(
        status: AgentRunReportHealthStatus,
        requested_admitted: bool,
        admitted: bool,
        requires_repair_first: bool,
        repair_tasks: usize,
        next_queue_tasks: usize,
        blocked_reasons: usize,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary {
            monitor_health_status: status,
            requested_admitted,
            admitted,
            requires_repair_first,
            monitor_records: 1,
            repair_tasks,
            next_queue_tasks,
            blocked_reasons,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: Vec::new(),
            telemetry: Vec::new(),
        }
    }

    fn trend_handoff_monitor_handoff_handoff_summary(
        status: AgentRunReportHealthStatus,
        requested_admitted: bool,
        admitted: bool,
        requires_repair_first: bool,
        repair_tasks: usize,
        next_queue_tasks: usize,
        blocked_reasons: usize,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummary {
            handoff_health_status: status,
            requested_admitted,
            admitted,
            requires_repair_first,
            handoff_records: 1,
            repair_tasks,
            next_queue_tasks,
            blocked_reasons,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: Vec::new(),
            telemetry: Vec::new(),
        }
    }

    fn trend_handoff_monitor_handoff_handoff_handoff_summary(
        status: AgentRunReportHealthStatus,
        requested_admitted: bool,
        admitted: bool,
        requires_repair_first: bool,
        repair_tasks: usize,
        next_queue_tasks: usize,
        blocked_reasons: usize,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary {
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary {
            packet_health_status: status,
            requested_admitted,
            admitted,
            requires_repair_first,
            packet_records: 1,
            repair_tasks,
            next_queue_tasks,
            blocked_reasons,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: Vec::new(),
            telemetry: Vec::new(),
        }
    }

    #[test]
    fn run_report_health_watches_empty_history() {
        let health =
            AgentRunReportSummaryHistory::new().health(AgentRunReportHealthPolicy::default());

        assert_eq!(health.status, AgentRunReportHealthStatus::Watch);
        assert_eq!(health.reasons, vec!["agent_run_report_history_empty"]);
        assert_eq!(health.dashboard.total_runs, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn run_report_history_marks_clean_trend_stable() {
        let history = AgentRunReportSummaryHistory::from_summaries(vec![
            run_summary(0, 0, 0, true, true, true),
            run_summary(0, 0, 0, true, true, true),
        ]);

        let dashboard = history.dashboard();
        let health = dashboard.health(AgentRunReportHealthPolicy::default());

        assert_eq!(dashboard.total_runs, 2);
        assert_eq!(dashboard.clean_runs, 2);
        assert_eq!(dashboard.clean_rate, 1.0);
        assert_eq!(dashboard.memory_note_admission_rate, 1.0);
        assert_eq!(dashboard.adaptive_state_admission_rate, 1.0);
        assert_eq!(dashboard.latest_all_side_effects_allowed, Some(true));
        assert_eq!(health.status, AgentRunReportHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert!(health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_report_dashboard_clean_rate=1.000" })
        );
    }

    #[test]
    fn run_report_history_recorder_repairs_conflict_budget_and_side_effect_trend() {
        let recorder = AgentRunReportSummaryHistoryRecorder::new();
        let clean = run_summary(0, 0, 0, true, true, true);
        let dirty = run_summary(2, 1, 3, false, false, false);

        let first = recorder.record_summary_with_health(
            AgentRunReportSummaryHistory::new(),
            clean,
            AgentRunReportHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            dirty.clone(),
            AgentRunReportHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.appended_summary, dirty);
        assert_eq!(second.dashboard.conflict_runs, 1);
        assert_eq!(second.dashboard.budget_overspend_runs, 1);
        assert_eq!(second.dashboard.side_effect_blocked_runs, 1);
        assert_eq!(second.dashboard.total_unresolved_conflicts, 2);
        assert_eq!(second.dashboard.total_budget_overspends, 1);
        assert_eq!(second.dashboard.total_blocked_side_effects, 3);
        assert_eq!(second.health.status, AgentRunReportHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_run_report_conflict_runs=1>0",
                "agent_run_report_budget_overspend_runs=1>0",
                "agent_run_report_side_effect_blocked_runs=1>0",
                "agent_run_report_clean_rate=0.500<0.67",
                "agent_run_report_memory_note_admission_rate=0.500<0.67",
                "agent_run_report_adaptive_state_admission_rate=0.500<0.67",
            ]
        );
        assert!(
            second
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_report_history_record_status=repair" })
        );
    }

    #[test]
    fn run_ledger_admission_blocks_budget_exhausted_dispatch_before_construction() {
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
        let dispatch_gate = dispatch.gate();

        let admission = AgentRunLedger::admission(&dispatch_gate);

        assert!(!admission.can_build_ledger);
        assert!(!admission.can_admit_side_effects);
        assert!(!admission.can_submit_memory_note);
        assert!(!admission.can_promote_adaptive_state);
        assert!(admission.requires_repair_first);
        assert_eq!(admission.reasons, dispatch_gate.reasons);
        assert!(
            admission
                .reasons
                .iter()
                .any(|reason| reason.contains("insufficient budget"))
        );
        assert!(
            admission
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_ledger_admission_can_build_ledger=false" })
        );
        assert!(matches!(
            AgentRunLedger::try_from_dispatch(dispatch),
            Err(blocked) if blocked == admission
        ));
    }

    #[test]
    fn run_ledger_admission_builds_ledger_for_clean_dispatch() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Coder, AgentBudget::new(12, 2, 2)),
        );
        let task = AgentTask::new(
            "coder-pass",
            AgentRole::Coder,
            "land a clean dispatch",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = planner.plan_with_policy(vec![task], &BudgetPolicy::strict());
        let dispatch_gate = dispatch.gate();

        let admission = AgentRunLedger::admission(&dispatch_gate);
        let ledger = AgentRunLedger::try_from_dispatch(dispatch).expect("ledger should build");

        assert!(admission.is_admitted());
        assert!(admission.can_build_ledger);
        assert!(admission.can_admit_side_effects);
        assert!(admission.can_submit_memory_note);
        assert!(admission.can_promote_adaptive_state);
        assert!(!admission.requires_repair_first);
        assert!(admission.reasons.is_empty());
        assert_eq!(ledger.dispatch().assignments.len(), 1);
        assert_eq!(ledger.progress().assigned_tasks, 1);
        assert!(
            admission
                .telemetry
                .iter()
                .any(|line| line == "agent_run_ledger_admission_can_build_ledger=true")
        );
    }

    #[test]
    fn run_report_health_gate_admits_stable_and_watch_without_repair_tasks() {
        let gate = AgentRunReportHealthGate::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let stable_health = AgentRunReportSummaryHistory::from_summaries(vec![run_summary(
            0, 0, 0, true, true, true,
        )])
        .health(AgentRunReportHealthPolicy::default());
        let watch_health =
            AgentRunReportSummaryHistory::new().health(AgentRunReportHealthPolicy::default());

        let stable = gate.evaluate("run/8", &stable_health, &queue);
        let watch = gate.evaluate("run/8", &watch_health, &queue);

        assert!(stable.is_admitted());
        assert_eq!(stable.health_status, AgentRunReportHealthStatus::Stable);
        assert!(stable.repair_tasks.is_empty());
        assert_eq!(stable.next_queue.task_ids(), vec!["business-task"]);
        assert!(stable.blocked_reasons.is_empty());
        assert!(watch.is_admitted());
        assert_eq!(watch.health_status, AgentRunReportHealthStatus::Watch);
        assert!(watch.repair_tasks.is_empty());
        assert_eq!(watch.next_queue.task_ids(), vec!["business-task"]);
        assert!(watch.blocked_reasons.is_empty());
        assert_eq!(queue.task_ids(), vec!["business-task"]);
    }

    #[test]
    fn run_report_health_gate_blocks_repair_and_merges_repair_queue() {
        let history = AgentRunReportSummaryHistory::from_summaries(vec![
            run_summary(0, 0, 0, true, true, true),
            run_summary(2, 1, 3, false, false, false),
        ]);
        let health = history.health(AgentRunReportHealthPolicy::default());
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);

        let decision = AgentRunReportHealthGate::new().evaluate("run/9", &health, &queue);

        assert!(!decision.admitted);
        assert!(!decision.is_admitted());
        assert!(decision.requires_repair_first);
        assert_eq!(decision.health_status, AgentRunReportHealthStatus::Repair);
        assert_eq!(decision.blocked_reasons, health.reasons);
        assert_eq!(decision.repair_tasks.len(), 6);
        assert_eq!(
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "agent-run-report-health-repair-run-9-0-agent_run_report_conflict_runs-1-0",
                "agent-run-report-health-repair-run-9-1-agent_run_report_budget_overspend_runs-1-0",
                "agent-run-report-health-repair-run-9-2-agent_run_report_side_effect_blocked_runs-1-0",
                "agent-run-report-health-repair-run-9-3-agent_run_report_clean_rate-0-500-0-67",
                "agent-run-report-health-repair-run-9-4-agent_run_report_memory_note_admission_rate-0-500-0-67",
                "agent-run-report-health-repair-run-9-5-agent_run_report_adaptive_state_admission_rate-0-500-0-67",
            ]
        );
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "agent-run-report-health" && task.priority == 9)
        );
        assert_eq!(
            decision.next_queue.task_ids(),
            vec![
                "agent-run-report-health-repair-run-9-0-agent_run_report_conflict_runs-1-0",
                "agent-run-report-health-repair-run-9-1-agent_run_report_budget_overspend_runs-1-0",
                "agent-run-report-health-repair-run-9-2-agent_run_report_side_effect_blocked_runs-1-0",
                "agent-run-report-health-repair-run-9-3-agent_run_report_clean_rate-0-500-0-67",
                "agent-run-report-health-repair-run-9-4-agent_run_report_memory_note_admission_rate-0-500-0-67",
                "agent-run-report-health-repair-run-9-5-agent_run_report_adaptive_state_admission_rate-0-500-0-67",
                "business-task",
            ]
        );
        assert_eq!(queue.task_ids(), vec!["business-task"]);
        assert!(
            decision
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_report_health_gate_requires_repair_first=true" })
        );

        let summary = decision.summary();
        assert_eq!(summary.health_status, AgentRunReportHealthStatus::Repair);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.repair_tasks, 6);
        assert_eq!(summary.next_queue_tasks, 7);
        assert_eq!(
            summary.repair_task_ids,
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(summary.next_queue_task_ids, decision.next_queue.task_ids());
        assert_eq!(summary.blocked_reasons, decision.blocked_reasons);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_report_health_gate_summary_status=repair" })
        );
    }

    #[test]
    fn run_report_health_gate_repair_queue_blocks_high_priority_side_effect_tasks() {
        let history = AgentRunReportSummaryHistory::from_summaries(vec![
            run_summary(0, 0, 0, true, true, true),
            run_summary(1, 0, 2, false, false, true),
        ]);
        let health = history.health(AgentRunReportHealthPolicy::default());
        let queue = AgentTaskQueue::from_tasks(vec![
            AgentTask::new(
                "memory-note",
                AgentRole::MemoryCurator,
                "promote memory note after run repair",
                AgentBudget::new(4, 1, 1),
            )
            .with_priority(10),
            AgentTask::new(
                "side-effect-admission",
                AgentRole::Reviewer,
                "admit side effect after run repair",
                AgentBudget::new(4, 1, 1),
            )
            .with_priority(10),
            AgentTask::new(
                "next-task-promotion",
                AgentRole::Planner,
                "promote next task after run repair",
                AgentBudget::new(4, 1, 1),
            )
            .with_priority(10),
        ]);

        let decision =
            AgentRunReportHealthGate::new().evaluate("run/repair-first", &health, &queue);
        let repair_task_ids = decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let schedule = RecursiveAgentScheduler::new(16).plan(decision.next_queue.tasks());

        assert!(decision.requires_repair_first);
        assert!(!decision.is_admitted());
        assert_eq!(decision.repair_tasks.len(), 5);
        for task_id in [
            "memory-note",
            "next-task-promotion",
            "side-effect-admission",
        ] {
            let task = decision
                .next_queue
                .tasks()
                .into_iter()
                .find(|task| task.id == task_id)
                .expect("business task should remain in the gated queue");
            assert_eq!(task.dependencies, repair_task_ids);
        }
        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.waves[0].task_ids, repair_task_ids);
        assert_eq!(
            schedule.waves[1].task_ids,
            vec![
                "memory-note",
                "next-task-promotion",
                "side-effect-admission",
            ]
        );
    }

    #[test]
    fn run_progress_report_gate_admits_clean_progress_and_clean_report_gate() {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue after run close",
            AgentBudget::new(4, 1, 1),
        )]);
        let progress = AgentRunLedgerProgressSummary {
            assigned_tasks: 1,
            reported_tasks: 1,
            accepted_results: 1,
            rejected_results: 0,
            dispatch_rejections: 0,
            missing_assigned_tasks: 0,
            unassigned_results: 0,
            empty_dispatch: false,
            can_close_run: true,
            requires_repair_first: false,
            telemetry: Vec::new(),
        };
        let progress_record = AgentRunLedgerProgressSummaryHistoryRecorder::new()
            .record_summary_with_health(
                AgentRunLedgerProgressSummaryHistory::new(),
                progress,
                AgentRunLedgerProgressHealthPolicy::default(),
            );
        let report_record = AgentRunReportSummaryHistoryRecorder::new()
            .record_summary_with_health_gate(
                AgentRunReportSummaryHistory::new(),
                run_summary(0, 0, 0, true, true, true),
                AgentRunReportHealthPolicy::default(),
                "run/progress-clean",
                &queue,
            );

        let record = AgentRunProgressReportGate::new().gate(
            "run/progress-clean",
            progress_record,
            report_record,
        );

        assert!(record.is_admitted());
        assert!(record.admitted);
        assert!(!record.requires_repair_first);
        assert!(record.progress_repair_tasks.is_empty());
        assert!(record.blocked_reasons.is_empty());
        assert_eq!(record.next_queue.task_ids(), vec!["business-task"]);
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_progress_report_gate_admitted=true" })
        );

        let summary = record.summary();
        assert_eq!(
            summary.progress_health_status,
            AgentRunReportHealthStatus::Stable
        );
        assert_eq!(
            summary.report_health_status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
    }

    #[test]
    fn run_progress_report_gate_schedules_progress_repair_before_report_repair() {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue after run repair",
            AgentBudget::new(4, 1, 1),
        )]);
        let progress = AgentRunLedgerProgressSummary {
            assigned_tasks: 2,
            reported_tasks: 2,
            accepted_results: 1,
            rejected_results: 1,
            dispatch_rejections: 0,
            missing_assigned_tasks: 1,
            unassigned_results: 1,
            empty_dispatch: false,
            can_close_run: false,
            requires_repair_first: true,
            telemetry: Vec::new(),
        };
        let progress_record = AgentRunLedgerProgressSummaryHistoryRecorder::new()
            .record_summary_with_health(
                AgentRunLedgerProgressSummaryHistory::new(),
                progress,
                AgentRunLedgerProgressHealthPolicy::default(),
            );
        let report_record = AgentRunReportSummaryHistoryRecorder::new()
            .record_summary_with_health_gate(
                AgentRunReportSummaryHistory::new(),
                run_summary(1, 0, 2, false, false, true),
                AgentRunReportHealthPolicy::default(),
                "run/progress-repair",
                &queue,
            );

        let record = AgentRunProgressReportGate::new().gate(
            "run/progress-repair",
            progress_record,
            report_record,
        );
        let progress_repair_ids = record
            .progress_repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let report_repair_ids = record
            .report_gate_record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let schedule = RecursiveAgentScheduler::new(32).plan(record.next_queue.tasks());

        assert!(!record.is_admitted());
        assert!(!record.admitted);
        assert!(record.requires_repair_first);
        assert_eq!(record.progress_repair_tasks.len(), 5);
        assert_eq!(
            record.report_gate_record.gate_decision.repair_tasks.len(),
            5
        );
        assert!(
            record
                .progress_repair_tasks
                .iter()
                .all(|task| task.lane == "agent-run-ledger-progress" && task.priority == 10)
        );
        assert!(record.blocked_reasons.iter().any(|reason| {
            reason == "progress:agent_run_ledger_progress_repair_first_records=1>0"
        }));
        assert!(
            record
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("report:agent_run_report_"))
        );
        assert_eq!(schedule.wave_count(), 3);
        assert_eq!(schedule.waves[0].task_ids, progress_repair_ids);
        assert_eq!(schedule.waves[1].task_ids, report_repair_ids);
        assert_eq!(schedule.waves[2].task_ids, vec!["business-task"]);

        let summary = record.summary();
        assert_eq!(
            summary.progress_health_status,
            AgentRunReportHealthStatus::Repair
        );
        assert_eq!(
            summary.report_health_status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(!summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.progress_repair_tasks, 5);
        assert_eq!(summary.report_repair_tasks, 5);
        assert_eq!(summary.next_queue_tasks, 11);
        assert_eq!(summary.progress_repair_task_ids, progress_repair_ids);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_run_progress_report_gate_summary_requires_repair_first=true"
        }));
    }

    #[test]
    fn run_progress_report_gate_history_watches_empty() {
        let health = AgentRunProgressReportGateSummaryHistory::new()
            .health(AgentRunProgressReportGateHealthPolicy::default());

        assert_eq!(health.status, AgentRunReportHealthStatus::Watch);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["agent_run_progress_report_gate_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_progress_report_gate_dashboard_records=0" })
        );
    }

    #[test]
    fn run_progress_report_gate_history_records_stable_admission() {
        let summary = AgentRunProgressReportGateSummary {
            progress_health_status: AgentRunReportHealthStatus::Stable,
            report_health_status: AgentRunReportHealthStatus::Stable,
            requested_admitted: true,
            admitted: true,
            requires_repair_first: false,
            progress_repair_tasks: 0,
            report_repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            progress_repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };

        let record = AgentRunProgressReportGateSummaryHistoryRecorder::new()
            .record_summary_with_health(
                AgentRunProgressReportGateSummaryHistory::new(),
                summary,
                AgentRunProgressReportGateHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert!(record.appended_summary.admitted);
        assert!(!record.appended_summary.requires_repair_first);
        assert_eq!(record.dashboard.requested_admitted_records, 1);
        assert_eq!(record.dashboard.admitted_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert_eq!(record.dashboard.progress_stable_records, 1);
        assert_eq!(record.dashboard.report_stable_records, 1);
        assert_eq!(record.dashboard.admission_rate, 1.0);
        assert_eq!(
            record.dashboard.latest_progress_health_status,
            Some(AgentRunReportHealthStatus::Stable)
        );
        assert_eq!(
            record.dashboard.latest_report_health_status,
            Some(AgentRunReportHealthStatus::Stable)
        );
        assert_eq!(record.health.status, AgentRunReportHealthStatus::Stable);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record.telemetry.iter().any(|line| {
                line == "agent_run_progress_report_gate_history_record_status=stable"
            })
        );
    }

    #[test]
    fn run_progress_report_gate_history_repairs_progress_and_report_pressure() {
        let summary = AgentRunProgressReportGateSummary {
            progress_health_status: AgentRunReportHealthStatus::Repair,
            report_health_status: AgentRunReportHealthStatus::Repair,
            requested_admitted: false,
            admitted: false,
            requires_repair_first: true,
            progress_repair_tasks: 5,
            report_repair_tasks: 5,
            next_queue_tasks: 11,
            blocked_reasons: 10,
            progress_repair_task_ids: Vec::new(),
            next_queue_task_ids: Vec::new(),
            telemetry: Vec::new(),
        };

        let record = AgentRunProgressReportGateSummaryHistoryRecorder::new()
            .record_summary_with_health(
                AgentRunProgressReportGateSummaryHistory::new(),
                summary,
                AgentRunProgressReportGateHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert!(!record.appended_summary.admitted);
        assert!(record.appended_summary.requires_repair_first);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.progress_repair_records, 1);
        assert_eq!(record.dashboard.report_repair_records, 1);
        assert_eq!(record.dashboard.progress_repair_tasks, 5);
        assert_eq!(record.dashboard.report_repair_tasks, 5);
        assert_eq!(record.dashboard.blocked_reasons, 10);
        assert_eq!(record.dashboard.admission_rate, 0.0);
        assert_eq!(record.health.status, AgentRunReportHealthStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "agent_run_progress_report_gate_repair_first_records=1>0",
                "agent_run_progress_report_gate_progress_repair_records=1>0",
                "agent_run_progress_report_gate_report_repair_records=1>0",
                "agent_run_progress_report_gate_progress_repair_tasks=5>0",
                "agent_run_progress_report_gate_report_repair_tasks=5>0",
                "agent_run_progress_report_gate_blocked_reasons=10>0",
                "agent_run_progress_report_gate_admission_rate=0.000<0.67",
            ]
        );
        assert!(
            record.telemetry.iter().any(|line| {
                line == "agent_run_progress_report_gate_history_record_status=repair"
            })
        );
    }

    #[test]
    fn run_report_history_recorder_gates_clean_health_record() {
        let recorder = AgentRunReportSummaryHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);

        let record = recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/10",
            &queue,
        );

        assert!(record.is_admitted());
        assert_eq!(record.health_record.history.len(), 1);
        assert_eq!(
            record.health_record.health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert_eq!(record.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(record.gate_summary.next_queue_tasks, 1);
        assert!(record.gate_summary.repair_task_ids.is_empty());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_report_health_gate_record_admitted=true" })
        );
    }

    #[test]
    fn run_report_history_recorder_gates_dirty_health_record_to_repair_first() {
        let recorder = AgentRunReportSummaryHistoryRecorder::new();
        let history = AgentRunReportSummaryHistory::from_summaries(vec![run_summary(
            0, 0, 0, true, true, true,
        )]);
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);

        let record = recorder.record_summary_with_health_gate(
            history,
            run_summary(1, 1, 1, false, false, true),
            AgentRunReportHealthPolicy::default(),
            "run/11",
            &queue,
        );

        assert!(!record.is_admitted());
        assert_eq!(record.health_record.history.len(), 2);
        assert_eq!(
            record.health_record.health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(record.gate_decision.requires_repair_first);
        assert_eq!(record.gate_summary.repair_tasks, 6);
        assert_eq!(record.gate_summary.next_queue_tasks, 7);
        assert_eq!(
            record.gate_summary.blocked_reasons,
            record.gate_decision.blocked_reasons
        );
        assert!(
            record
                .next_queue()
                .task_ids()
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_record_requires_repair_first=true"
        }));
    }

    #[test]
    fn run_report_health_gate_history_watches_empty_dashboard() {
        let health = AgentRunReportHealthGateHistory::new()
            .health(AgentRunReportHealthGateHealthPolicy::default());

        assert_eq!(health.status, AgentRunReportHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_run_report_health_gate_history_empty"]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn run_report_health_gate_history_marks_clean_admission_stable() {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let health = AgentRunReportSummaryHistory::from_summaries(vec![run_summary(
            0, 0, 0, true, true, true,
        )])
        .health(AgentRunReportHealthPolicy::default());
        let summary = AgentRunReportHealthGate::new()
            .evaluate("run/12", &health, &queue)
            .summary();
        let history = AgentRunReportHealthGateHistory::from_summaries(vec![summary.clone()]);

        let dashboard = history.dashboard();
        let gate_health = dashboard.health(AgentRunReportHealthGateHealthPolicy::default());

        assert_eq!(dashboard.total_records, 1);
        assert_eq!(dashboard.admitted_records, 1);
        assert_eq!(dashboard.repair_first_records, 0);
        assert_eq!(dashboard.stable_records, 1);
        assert_eq!(dashboard.repair_task_count, 0);
        assert_eq!(dashboard.admission_rate, 1.0);
        assert_eq!(dashboard.latest_health_status, Some(summary.health_status));
        assert_eq!(gate_health.status, AgentRunReportHealthStatus::Stable);
        assert!(gate_health.reasons.is_empty());
        assert!(gate_health.is_stable());
        assert!(gate_health.allows_service_advance());
        assert!(!gate_health.requires_repair_first());
        assert!(dashboard.is_clean());
        assert!(
            dashboard.telemetry.iter().any(|line| {
                line == "agent_run_report_health_gate_dashboard_admission_rate=1.000"
            })
        );
    }

    #[test]
    fn run_report_health_gate_history_recorder_repairs_repair_first_pressure() {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let gate_recorder = AgentRunReportHealthGateHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let clean_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/13",
            &queue,
        );
        let dirty_record = run_recorder.record_summary_with_health_gate(
            clean_record.health_record.history.clone(),
            run_summary(1, 1, 1, false, false, true),
            AgentRunReportHealthPolicy::default(),
            "run/14",
            &queue,
        );

        let first = gate_recorder.record_gate_record_with_health(
            AgentRunReportHealthGateHistory::new(),
            &clean_record,
            AgentRunReportHealthGateHealthPolicy::default(),
        );
        let second = gate_recorder.record_gate_record_with_health(
            first.history,
            &dirty_record,
            AgentRunReportHealthGateHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(
            second.appended_summary.health_status,
            AgentRunReportHealthStatus::Repair
        );
        assert_eq!(second.dashboard.total_records, 2);
        assert_eq!(second.dashboard.admitted_records, 1);
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 6);
        assert_eq!(second.dashboard.blocked_reasons, 6);
        assert_eq!(second.dashboard.repair_first_rate, 0.5);
        assert_eq!(second.health.status, AgentRunReportHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_run_report_health_gate_repair_first_rate=0.500>0",
                "agent_run_report_health_gate_repair_tasks=6>0",
                "agent_run_report_health_gate_blocked_reasons=6>0",
                "agent_run_report_health_gate_admission_rate=0.500<0.67",
            ]
        );
        assert!(
            second.telemetry.iter().any(|line| {
                line == "agent_run_report_health_gate_history_record_status=repair"
            })
        );
    }

    #[test]
    fn run_report_health_gate_trend_gate_admits_stable_and_watch_history() {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let run_health = AgentRunReportSummaryHistory::from_summaries(vec![run_summary(
            0, 0, 0, true, true, true,
        )])
        .health(AgentRunReportHealthPolicy::default());
        let gate_summary = AgentRunReportHealthGate::new()
            .evaluate("run/15", &run_health, &queue)
            .summary();
        let stable_health = AgentRunReportHealthGateHistory::from_summaries(vec![gate_summary])
            .health(AgentRunReportHealthGateHealthPolicy::default());
        let watch_health = AgentRunReportHealthGateHistory::new()
            .health(AgentRunReportHealthGateHealthPolicy::default());

        let stable =
            AgentRunReportHealthGateTrendGate::new().evaluate("run/16", &stable_health, &queue);
        let watch =
            AgentRunReportHealthGateTrendGate::new().evaluate("run/16", &watch_health, &queue);

        assert!(stable.is_admitted());
        assert_eq!(stable.health_status, AgentRunReportHealthStatus::Stable);
        assert!(stable.repair_tasks.is_empty());
        assert_eq!(stable.next_queue.task_ids(), vec!["business-task"]);
        assert!(watch.is_admitted());
        assert_eq!(watch.health_status, AgentRunReportHealthStatus::Watch);
        assert!(watch.repair_tasks.is_empty());
        assert_eq!(watch.next_queue.task_ids(), vec!["business-task"]);
        assert!(watch.blocked_reasons.is_empty());
        assert!(
            watch
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_report_health_gate_trend_gate_status=watch" })
        );
    }

    #[test]
    fn run_report_health_gate_trend_gate_blocks_repair_history() {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let gate_recorder = AgentRunReportHealthGateHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let clean_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/17",
            &queue,
        );
        let dirty_record = run_recorder.record_summary_with_health_gate(
            clean_record.health_record.history.clone(),
            run_summary(1, 1, 1, false, false, true),
            AgentRunReportHealthPolicy::default(),
            "run/18",
            &queue,
        );
        let first = gate_recorder.record_gate_record_with_health(
            AgentRunReportHealthGateHistory::new(),
            &clean_record,
            AgentRunReportHealthGateHealthPolicy::default(),
        );
        let trend_record = gate_recorder.record_gate_record_with_health(
            first.history,
            &dirty_record,
            AgentRunReportHealthGateHealthPolicy::default(),
        );

        let decision = AgentRunReportHealthGateTrendGate::new().evaluate(
            "run/19",
            &trend_record.health,
            &queue,
        );

        assert!(!decision.admitted);
        assert!(!decision.is_admitted());
        assert!(decision.requires_repair_first);
        assert_eq!(decision.health_status, AgentRunReportHealthStatus::Repair);
        assert_eq!(decision.blocked_reasons, trend_record.health.reasons);
        assert_eq!(decision.repair_tasks.len(), 4);
        assert_eq!(
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "agent-run-report-health-gate-repair-run-19-0-agent_run_report_health_gate_repair_first_rate-0-500-0",
                "agent-run-report-health-gate-repair-run-19-1-agent_run_report_health_gate_repair_tasks-6-0",
                "agent-run-report-health-gate-repair-run-19-2-agent_run_report_health_gate_blocked_reasons-6-0",
                "agent-run-report-health-gate-repair-run-19-3-agent_run_report_health_gate_admission_rate-0-500-0-67",
            ]
        );
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| { task.lane == "agent-run-report-health-gate" && task.priority == 9 })
        );
        assert_eq!(
            decision.next_queue.task_ids(),
            vec![
                "agent-run-report-health-gate-repair-run-19-0-agent_run_report_health_gate_repair_first_rate-0-500-0",
                "agent-run-report-health-gate-repair-run-19-1-agent_run_report_health_gate_repair_tasks-6-0",
                "agent-run-report-health-gate-repair-run-19-2-agent_run_report_health_gate_blocked_reasons-6-0",
                "agent-run-report-health-gate-repair-run-19-3-agent_run_report_health_gate_admission_rate-0-500-0-67",
                "business-task",
            ]
        );
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_gate_requires_repair_first=true"
        }));

        let summary = decision.summary();
        assert_eq!(summary.health_status, AgentRunReportHealthStatus::Repair);
        assert_eq!(summary.repair_tasks, 4);
        assert_eq!(summary.next_queue_tasks, 5);
        assert_eq!(summary.blocked_reasons, decision.blocked_reasons);
    }

    #[test]
    fn run_report_health_gate_trend_handoff_records_and_admits_stable_boundary() {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let run_gate_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/20",
            &queue,
        );

        let handoff = AgentRunReportHealthGateTrendHandoff::new().record_gate_record_and_gate(
            AgentRunReportHealthGateHistory::new(),
            &run_gate_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/21",
            &queue,
        );

        assert!(handoff.is_admitted());
        assert_eq!(handoff.trend_record.history.len(), 1);
        assert_eq!(
            handoff.trend_record.health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(handoff.gate_decision.repair_tasks.is_empty());
        assert_eq!(handoff.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(handoff.gate_summary.next_queue_tasks, 1);
        assert!(
            handoff
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_report_health_gate_trend_handoff_admitted=true" })
        );

        let summary = handoff.summary();
        assert_eq!(
            summary.trend_health_status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(summary.admitted);
        assert_eq!(summary.trend_records, 1);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_summary_status=stable"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_records_and_repairs_dirty_boundary() {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let clean_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/22",
            &queue,
        );
        let dirty_record = run_recorder.record_summary_with_health_gate(
            clean_record.health_record.history.clone(),
            run_summary(1, 1, 1, false, false, true),
            AgentRunReportHealthPolicy::default(),
            "run/23",
            &queue,
        );
        let handoff = AgentRunReportHealthGateTrendHandoff::new();
        let stable_handoff = handoff.record_gate_record_and_gate(
            AgentRunReportHealthGateHistory::new(),
            &clean_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/24",
            &queue,
        );

        let repair_handoff = handoff.record_gate_record_and_gate(
            stable_handoff.trend_record.history,
            &dirty_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/25",
            &queue,
        );

        assert!(!repair_handoff.is_admitted());
        assert_eq!(repair_handoff.trend_record.history.len(), 2);
        assert_eq!(
            repair_handoff.trend_record.health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(repair_handoff.gate_decision.requires_repair_first);
        assert_eq!(repair_handoff.gate_decision.repair_tasks.len(), 4);
        assert_eq!(repair_handoff.gate_summary.repair_tasks, 4);
        assert_eq!(repair_handoff.gate_summary.next_queue_tasks, 5);
        assert_eq!(
            repair_handoff.gate_summary.blocked_reasons,
            repair_handoff.gate_decision.blocked_reasons
        );
        assert!(repair_handoff.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_requires_repair_first=true"
        }));

        let summary = repair_handoff.summary();
        assert_eq!(
            summary.trend_health_status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.trend_records, 2);
        assert_eq!(summary.repair_tasks, 4);
        assert_eq!(summary.next_queue_tasks, 5);
        assert_eq!(
            summary.repair_task_ids,
            repair_handoff.gate_summary.repair_task_ids
        );
        assert_eq!(
            summary.next_queue_task_ids,
            repair_handoff.gate_summary.next_queue_task_ids
        );
        assert_eq!(
            summary.blocked_reasons,
            repair_handoff.gate_summary.blocked_reasons
        );
    }

    #[test]
    fn run_report_health_gate_trend_handoff_history_watches_empty_dashboard() {
        let health = AgentRunReportHealthGateTrendHandoffHistory::new()
            .health(AgentRunReportHealthGateTrendHandoffHealthPolicy::default());

        assert_eq!(health.status, AgentRunReportHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_run_report_health_gate_trend_handoff_history_empty"]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn run_report_health_gate_trend_handoff_history_marks_stable_handoff_stable() {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let run_gate_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/26",
            &queue,
        );
        let handoff = AgentRunReportHealthGateTrendHandoff::new().record_gate_record_and_gate(
            AgentRunReportHealthGateHistory::new(),
            &run_gate_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/27",
            &queue,
        );
        let history =
            AgentRunReportHealthGateTrendHandoffHistory::from_summaries(vec![handoff.summary()]);

        let dashboard = history.dashboard();
        let health = dashboard.health(AgentRunReportHealthGateTrendHandoffHealthPolicy::default());

        assert_eq!(dashboard.total_records, 1);
        assert_eq!(dashboard.admitted_records, 1);
        assert_eq!(dashboard.repair_first_records, 0);
        assert_eq!(dashboard.stable_records, 1);
        assert_eq!(dashboard.repair_task_count, 0);
        assert_eq!(dashboard.blocked_reasons, 0);
        assert_eq!(dashboard.admission_rate, 1.0);
        assert!(dashboard.is_clean());
        assert_eq!(health.status, AgentRunReportHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert!(health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(dashboard.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_dashboard_admission_rate=1.000"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_history_recorder_repairs_dirty_handoff() {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let handoff_recorder = AgentRunReportHealthGateTrendHandoffHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let clean_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/28",
            &queue,
        );
        let dirty_record = run_recorder.record_summary_with_health_gate(
            clean_record.health_record.history.clone(),
            run_summary(1, 1, 1, false, false, true),
            AgentRunReportHealthPolicy::default(),
            "run/29",
            &queue,
        );
        let handoff = AgentRunReportHealthGateTrendHandoff::new();
        let stable_handoff = handoff.record_gate_record_and_gate(
            AgentRunReportHealthGateHistory::new(),
            &clean_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/30",
            &queue,
        );
        let repair_handoff = handoff.record_gate_record_and_gate(
            stable_handoff.trend_record.history.clone(),
            &dirty_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/31",
            &queue,
        );
        let first = handoff_recorder.record_handoff_with_health(
            AgentRunReportHealthGateTrendHandoffHistory::new(),
            &stable_handoff,
            AgentRunReportHealthGateTrendHandoffHealthPolicy::default(),
        );
        let second = handoff_recorder.record_handoff_with_health(
            first.history,
            &repair_handoff,
            AgentRunReportHealthGateTrendHandoffHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(
            second.appended_summary.trend_health_status,
            AgentRunReportHealthStatus::Repair
        );
        assert_eq!(second.dashboard.total_records, 2);
        assert_eq!(second.dashboard.admitted_records, 1);
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 4);
        assert_eq!(second.dashboard.blocked_reasons, 4);
        assert_eq!(second.dashboard.repair_first_rate, 0.5);
        assert_eq!(second.health.status, AgentRunReportHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_run_report_health_gate_trend_handoff_repair_first_rate=0.500>0",
                "agent_run_report_health_gate_trend_handoff_repair_records=1>0",
                "agent_run_report_health_gate_trend_handoff_repair_tasks=4>0",
                "agent_run_report_health_gate_trend_handoff_blocked_reasons=4>0",
                "agent_run_report_health_gate_trend_handoff_admission_rate=0.500<0.67",
            ]
        );
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_history_record_status=repair"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_gate_preserves_stable_handoff() {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let handoff_recorder = AgentRunReportHealthGateTrendHandoffHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let run_gate_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/32",
            &queue,
        );
        let handoff = AgentRunReportHealthGateTrendHandoff::new().record_gate_record_and_gate(
            AgentRunReportHealthGateHistory::new(),
            &run_gate_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/33",
            &queue,
        );
        let history_record = handoff_recorder.record_handoff_with_health(
            AgentRunReportHealthGateTrendHandoffHistory::new(),
            &handoff,
            AgentRunReportHealthGateTrendHandoffHealthPolicy::default(),
        );

        let decision = AgentRunReportHealthGateTrendHandoffGate::new().evaluate(
            "run/34",
            &handoff,
            &history_record,
        );

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.handoff_health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_gate_admitted=true"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_gate_repairs_dirty_handoff_history() {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let handoff_recorder = AgentRunReportHealthGateTrendHandoffHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let clean_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/35",
            &queue,
        );
        let dirty_record = run_recorder.record_summary_with_health_gate(
            clean_record.health_record.history.clone(),
            run_summary(1, 1, 1, false, false, true),
            AgentRunReportHealthPolicy::default(),
            "run/36",
            &queue,
        );
        let handoff_builder = AgentRunReportHealthGateTrendHandoff::new();
        let stable_handoff = handoff_builder.record_gate_record_and_gate(
            AgentRunReportHealthGateHistory::new(),
            &clean_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/37",
            &queue,
        );
        let repair_handoff = handoff_builder.record_gate_record_and_gate(
            stable_handoff.trend_record.history.clone(),
            &dirty_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/38",
            &queue,
        );
        let first = handoff_recorder.record_handoff_with_health(
            AgentRunReportHealthGateTrendHandoffHistory::new(),
            &stable_handoff,
            AgentRunReportHealthGateTrendHandoffHealthPolicy::default(),
        );
        let second = handoff_recorder.record_handoff_with_health(
            first.history,
            &repair_handoff,
            AgentRunReportHealthGateTrendHandoffHealthPolicy::default(),
        );

        let decision = AgentRunReportHealthGateTrendHandoffGate::new().evaluate(
            "run/39",
            &repair_handoff,
            &second,
        );

        assert!(!decision.requested_admitted);
        assert!(!decision.is_admitted());
        assert!(decision.requires_repair_first);
        assert_eq!(
            decision.handoff_health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert_eq!(decision.repair_tasks.len(), 5);
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "agent-run-report-health-gate-handoff")
        );
        assert_eq!(
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "agent-run-report-health-gate-handoff-repair-run-39-0-agent_run_report_health_gate_trend_handoff_repair_first_rate-0-500-0",
                "agent-run-report-health-gate-handoff-repair-run-39-1-agent_run_report_health_gate_trend_handoff_repair_records-1-0",
                "agent-run-report-health-gate-handoff-repair-run-39-2-agent_run_report_health_gate_trend_handoff_repair_tasks-4-0",
                "agent-run-report-health-gate-handoff-repair-run-39-3-agent_run_report_health_gate_trend_handoff_blocked_reasons-4-0",
                "agent-run-report-health-gate-handoff-repair-run-39-4-agent_run_report_health_gate_trend_handoff_admission_rate-0-500-0-67",
            ]
        );
        assert_eq!(
            decision
                .next_queue
                .task_ids()
                .into_iter()
                .filter(|id| id.starts_with("agent-run-report-health-gate-handoff-repair"))
                .count(),
            5
        );
        assert_eq!(decision.blocked_reasons.len(), 9);
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_records_stable_handoff() {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let run_gate_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/40",
            &queue,
        );
        let handoff = AgentRunReportHealthGateTrendHandoff::new().record_gate_record_and_gate(
            AgentRunReportHealthGateHistory::new(),
            &run_gate_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/41",
            &queue,
        );

        let monitor_record = AgentRunReportHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentRunReportHealthGateTrendHandoffHistory::new(),
            AgentRunReportHealthGateTrendHandoffHealthPolicy::default(),
            "run/42",
        );

        assert!(monitor_record.is_admitted());
        assert_eq!(
            monitor_record.next_queue().task_ids(),
            vec!["business-task"]
        );
        assert_eq!(monitor_record.history_record.history.len(), 1);
        assert_eq!(
            monitor_record.history_record.health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(monitor_record.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_health_status=stable"
        }));

        let summary = monitor_record.summary();
        assert_eq!(
            summary.handoff_health_status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.handoff_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert_eq!(summary.blocked_reasons, 0);
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_repairs_dirty_handoff_history() {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let clean_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/43",
            &queue,
        );
        let dirty_record = run_recorder.record_summary_with_health_gate(
            clean_record.health_record.history.clone(),
            run_summary(1, 1, 1, false, false, true),
            AgentRunReportHealthPolicy::default(),
            "run/44",
            &queue,
        );
        let handoff_builder = AgentRunReportHealthGateTrendHandoff::new();
        let stable_handoff = handoff_builder.record_gate_record_and_gate(
            AgentRunReportHealthGateHistory::new(),
            &clean_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/45",
            &queue,
        );
        let repair_handoff = handoff_builder.record_gate_record_and_gate(
            stable_handoff.trend_record.history.clone(),
            &dirty_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/46",
            &queue,
        );
        let history = AgentRunReportHealthGateTrendHandoffHistory::from_summaries(vec![
            stable_handoff.summary(),
        ]);

        let monitor_record = AgentRunReportHealthGateTrendHandoffMonitor::new().record_and_gate(
            repair_handoff,
            history,
            AgentRunReportHealthGateTrendHandoffHealthPolicy::default(),
            "run/47",
        );

        assert!(!monitor_record.is_admitted());
        assert!(!monitor_record.gate_decision.requested_admitted);
        assert!(monitor_record.gate_decision.requires_repair_first);
        assert_eq!(
            monitor_record.history_record.health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert_eq!(monitor_record.gate_decision.repair_tasks.len(), 5);
        assert_eq!(
            monitor_record
                .next_queue()
                .task_ids()
                .into_iter()
                .filter(|id| id.starts_with("agent-run-report-health-gate-handoff-repair"))
                .count(),
            5
        );
        assert!(monitor_record.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_requires_repair_first=true"
        }));

        let summary = monitor_record.summary();
        assert_eq!(
            summary.handoff_health_status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(!summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.handoff_records, 2);
        assert_eq!(summary.repair_tasks, 5);
        assert_eq!(summary.next_queue_tasks, 10);
        assert_eq!(summary.blocked_reasons, 9);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_summary_status=repair"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_history_watches_empty() {
        let health = AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new()
            .health(AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default());

        assert_eq!(health.status, AgentRunReportHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_run_report_health_gate_trend_handoff_monitor_history_empty"]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_history_repairs_dirty_trend() {
        let run_recorder = AgentRunReportSummaryHistoryRecorder::new();
        let monitor = AgentRunReportHealthGateTrendHandoffMonitor::new();
        let recorder = AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecorder::new();
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue run report handoff",
            AgentBudget::new(4, 1, 1),
        )]);
        let clean_record = run_recorder.record_summary_with_health_gate(
            AgentRunReportSummaryHistory::new(),
            run_summary(0, 0, 0, true, true, true),
            AgentRunReportHealthPolicy::default(),
            "run/48",
            &queue,
        );
        let dirty_record = run_recorder.record_summary_with_health_gate(
            clean_record.health_record.history.clone(),
            run_summary(1, 1, 1, false, false, true),
            AgentRunReportHealthPolicy::default(),
            "run/49",
            &queue,
        );
        let handoff_builder = AgentRunReportHealthGateTrendHandoff::new();
        let stable_handoff = handoff_builder.record_gate_record_and_gate(
            AgentRunReportHealthGateHistory::new(),
            &clean_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/50",
            &queue,
        );
        let repair_handoff = handoff_builder.record_gate_record_and_gate(
            stable_handoff.trend_record.history.clone(),
            &dirty_record,
            AgentRunReportHealthGateHealthPolicy::default(),
            "run/51",
            &queue,
        );
        let stable_monitor = monitor.record_and_gate(
            stable_handoff,
            AgentRunReportHealthGateTrendHandoffHistory::new(),
            AgentRunReportHealthGateTrendHandoffHealthPolicy::default(),
            "run/52",
        );
        let repair_monitor = monitor.record_and_gate(
            repair_handoff,
            AgentRunReportHealthGateTrendHandoffHistory::from_summaries(vec![
                stable_monitor.handoff.summary(),
            ]),
            AgentRunReportHealthGateTrendHandoffHealthPolicy::default(),
            "run/53",
        );

        let first = recorder.record_monitor_with_health(
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            &stable_monitor,
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
        );
        let second = recorder.record_monitor_with_health(
            first.history,
            &repair_monitor,
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.dashboard.total_records, 2);
        assert_eq!(second.dashboard.requested_admitted_records, 1);
        assert_eq!(second.dashboard.admitted_records, 1);
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 5);
        assert_eq!(second.dashboard.total_next_queue_tasks, 11);
        assert_eq!(second.dashboard.blocked_reasons, 9);
        assert_eq!(second.dashboard.admission_rate, 0.5);
        assert_eq!(second.dashboard.repair_first_rate, 0.5);
        assert_eq!(second.health.status, AgentRunReportHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_run_report_health_gate_trend_handoff_monitor_repair_first_rate=0.500>0",
                "agent_run_report_health_gate_trend_handoff_monitor_repair_records=1>0",
                "agent_run_report_health_gate_trend_handoff_monitor_repair_tasks=5>0",
                "agent_run_report_health_gate_trend_handoff_monitor_blocked_reasons=9>0",
                "agent_run_report_health_gate_trend_handoff_monitor_admission_rate=0.500<0.67",
            ]
        );
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_history_record_status=repair"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_gate_preserves_stable_queue() {
        let monitor_record = stable_trend_handoff_monitor_record();
        let history_record =
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecorder::new()
                .record_monitor_with_health(
                    AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
                    &monitor_record,
                    AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
                );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorGate::new().evaluate(
            "run/54",
            &monitor_record,
            &history_record,
        );

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.monitor_health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_gate_status=stable"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_gate_preserves_watch_queue() {
        let monitor_record = stable_trend_handoff_monitor_record();
        let history_record =
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecorder::new()
                .record_monitor_with_health(
                    AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
                    &monitor_record,
                    AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy {
                        minimum_admission_rate: 1.1,
                        ..AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default()
                    },
                );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorGate::new().evaluate(
            "run/55",
            &monitor_record,
            &history_record,
        );

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.monitor_health.status,
            AgentRunReportHealthStatus::Watch
        );
        assert_eq!(
            decision.monitor_health.reasons,
            vec!["agent_run_report_health_gate_trend_handoff_monitor_admission_rate=1.000<1.1"]
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_gate_status=watch"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_gate_blocks_repair_history() {
        let monitor_record = stable_trend_handoff_monitor_record();
        let recorder = AgentRunReportHealthGateTrendHandoffMonitorSummaryHistoryRecorder::new();
        let stable = trend_handoff_monitor_summary(
            AgentRunReportHealthStatus::Stable,
            true,
            true,
            false,
            0,
            1,
            0,
        );
        let repair = trend_handoff_monitor_summary(
            AgentRunReportHealthStatus::Repair,
            false,
            false,
            true,
            5,
            10,
            9,
        );
        let first = recorder.record_summary_with_health(
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            stable,
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
        );
        let history_record = recorder.record_summary_with_health(
            first.history,
            repair,
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
        );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorGate::new().evaluate(
            "run/56",
            &monitor_record,
            &history_record,
        );

        assert!(decision.requested_admitted);
        assert!(!decision.is_admitted());
        assert!(decision.requires_repair_first);
        assert_eq!(
            decision.monitor_health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert_eq!(decision.repair_tasks.len(), 5);
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "agent-run-report-health-gate-handoff-monitor")
        );
        assert_eq!(
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "agent-run-report-health-gate-handoff-monitor-repair-run-56-0-agent_run_report_health_gate_trend_handoff_monitor_repair_first_rate-0-500-0",
                "agent-run-report-health-gate-handoff-monitor-repair-run-56-1-agent_run_report_health_gate_trend_handoff_monitor_repair_records-1-0",
                "agent-run-report-health-gate-handoff-monitor-repair-run-56-2-agent_run_report_health_gate_trend_handoff_monitor_repair_tasks-5-0",
                "agent-run-report-health-gate-handoff-monitor-repair-run-56-3-agent_run_report_health_gate_trend_handoff_monitor_blocked_reasons-9-0",
                "agent-run-report-health-gate-handoff-monitor-repair-run-56-4-agent_run_report_health_gate_trend_handoff_monitor_admission_rate-0-500-0-67",
            ]
        );
        assert_eq!(
            decision.next_queue.task_ids().first().map(String::as_str),
            Some(
                "agent-run-report-health-gate-handoff-monitor-repair-run-56-0-agent_run_report_health_gate_trend_handoff_monitor_repair_first_rate-0-500-0"
            )
        );
        assert_eq!(decision.next_queue.len(), 6);
        assert!(
            decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert_eq!(decision.blocked_reasons, history_record.health.reasons);
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_records_stable_gate() {
        let monitor_record = stable_trend_handoff_monitor_record();

        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor_record,
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/57",
        );

        assert!(handoff.is_admitted());
        assert_eq!(handoff.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(handoff.history_record.history.len(), 1);
        assert_eq!(
            handoff.history_record.health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(handoff.gate_decision.repair_tasks.is_empty());
        assert!(handoff.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_health_status=stable"
        }));

        let summary = handoff.summary();
        assert_eq!(
            summary.monitor_health_status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.monitor_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert_eq!(summary.blocked_reasons, 0);
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_repairs_dirty_history() {
        let monitor_record = stable_trend_handoff_monitor_record();
        let stable = trend_handoff_monitor_summary(
            AgentRunReportHealthStatus::Stable,
            true,
            true,
            false,
            0,
            1,
            0,
        );
        let repair = trend_handoff_monitor_summary(
            AgentRunReportHealthStatus::Repair,
            false,
            false,
            true,
            5,
            10,
            9,
        );
        let history =
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::from_summaries(vec![
                stable, repair,
            ]);

        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor_record,
            history,
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/58",
        );

        assert!(!handoff.is_admitted());
        assert!(handoff.gate_decision.requires_repair_first);
        assert_eq!(
            handoff.history_record.health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert_eq!(handoff.history_record.history.len(), 3);
        assert_eq!(handoff.gate_decision.repair_tasks.len(), 5);
        assert_eq!(
            handoff
                .next_queue()
                .task_ids()
                .into_iter()
                .filter(|id| {
                    id.starts_with("agent-run-report-health-gate-handoff-monitor-repair")
                })
                .count(),
            5
        );
        assert!(handoff.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_requires_repair_first=true"
        }));

        let summary = handoff.summary();
        assert_eq!(
            summary.monitor_health_status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.monitor_records, 3);
        assert_eq!(summary.repair_tasks, 5);
        assert_eq!(summary.next_queue_tasks, 6);
        assert_eq!(summary.blocked_reasons, 5);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_summary_status=repair"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_health_watches_empty_history() {
        let health = AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new()
            .health(AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default());

        assert_eq!(health.status, AgentRunReportHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec![
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_empty"
                    .to_owned()
            ]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_history_repairs_dirty_trend() {
        let recorder =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder::new();
        let stable = trend_handoff_monitor_handoff_summary(
            AgentRunReportHealthStatus::Stable,
            true,
            true,
            false,
            0,
            1,
            0,
        );
        let repair = trend_handoff_monitor_handoff_summary(
            AgentRunReportHealthStatus::Repair,
            false,
            false,
            true,
            5,
            6,
            5,
        );

        let first = recorder.record_summary_with_health(
            AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
            stable,
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            repair.clone(),
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.appended_summary, repair);
        assert_eq!(second.health.status, AgentRunReportHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_repair_first_rate=0.500>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_repair_records=1>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_repair_tasks=5>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_blocked_reasons=5>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_admission_rate=0.500<0.67",
            ]
        );
        assert_eq!(second.dashboard.requested_admitted_records, 1);
        assert_eq!(second.dashboard.admitted_records, 1);
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 5);
        assert_eq!(second.dashboard.total_next_queue_tasks, 7);
        assert_eq!(second.dashboard.blocked_reasons, 5);
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_history_record_status=repair"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_gate_preserves_stable_queue() {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/59",
        );
        let history_record =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder::new()
                .record_handoff_with_health(
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                    &handoff,
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorHandoffGate::new().evaluate(
            "run/60",
            &handoff,
            &history_record,
        );

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.handoff_health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_status=stable"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_gate_preserves_watch_queue() {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/61",
        );
        let history_record =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder::new()
                .record_handoff_with_health(
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                    &handoff,
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy {
                        minimum_admission_rate: 1.1,
                        ..AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default()
                    },
                );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorHandoffGate::new().evaluate(
            "run/62",
            &handoff,
            &history_record,
        );

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.handoff_health.status,
            AgentRunReportHealthStatus::Watch
        );
        assert_eq!(
            decision.handoff_health.reasons,
            vec![
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_admission_rate=1.000<1.1"
            ]
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_gate_blocks_repair_history() {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/63",
        );
        let recorder =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder::new();
        let first = recorder.record_summary_with_health(
            AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
            trend_handoff_monitor_handoff_summary(
                AgentRunReportHealthStatus::Stable,
                true,
                true,
                false,
                0,
                1,
                0,
            ),
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
        );
        let history_record = recorder.record_summary_with_health(
            first.history,
            trend_handoff_monitor_handoff_summary(
                AgentRunReportHealthStatus::Repair,
                false,
                false,
                true,
                5,
                6,
                5,
            ),
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
        );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorHandoffGate::new().evaluate(
            "run/64",
            &handoff,
            &history_record,
        );

        assert!(decision.requested_admitted);
        assert!(!decision.is_admitted());
        assert_eq!(
            decision.handoff_health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert_eq!(decision.repair_tasks.len(), 5);
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "agent-run-report-health-gate-handoff-monitor-handoff")
        );
        assert!(decision.next_queue.task_ids().first().is_some_and(|id| {
            id.starts_with("agent-run-report-health-gate-handoff-monitor-handoff-repair")
        }));
        assert_eq!(decision.next_queue.len(), 6);
        assert!(
            decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert_eq!(decision.blocked_reasons, history_record.health.reasons);
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_records_stable_gate() {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/65",
        );

        let record = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/66",
            );

        assert!(record.is_admitted());
        assert_eq!(record.history_record.history.len(), 1);
        assert_eq!(
            record.history_record.health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(record.gate_decision.repair_tasks.is_empty());
        assert_eq!(record.next_queue().task_ids(), vec!["business-task"]);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff=true"
        }));
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_admitted=true"
        }));

        let summary = record.summary();
        assert_eq!(
            summary.handoff_health_status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.handoff_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert_eq!(summary.blocked_reasons, 0);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_status=stable"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_repairs_dirty_history() {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/67",
        );
        let dirty_history =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::from_summaries(vec![
                trend_handoff_monitor_handoff_summary(
                    AgentRunReportHealthStatus::Repair,
                    false,
                    false,
                    true,
                    5,
                    6,
                    5,
                ),
            ]);

        let record = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                dirty_history,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/68",
            );

        assert!(!record.is_admitted());
        assert_eq!(record.history_record.history.len(), 2);
        assert_eq!(
            record.history_record.health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(record.gate_decision.requires_repair_first);
        assert_eq!(record.gate_decision.repair_tasks.len(), 5);
        assert_eq!(record.next_queue().len(), 6);
        assert!(
            record
                .gate_decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "agent-run-report-health-gate-handoff-monitor-handoff")
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_requires_repair_first=true"
        }));

        let summary = record.summary();
        assert_eq!(
            summary.handoff_health_status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.handoff_records, 2);
        assert_eq!(summary.repair_tasks, 5);
        assert_eq!(summary.next_queue_tasks, 6);
        assert_eq!(summary.blocked_reasons, 5);
        assert!(summary.repair_task_ids.iter().all(|id| {
            id.starts_with("agent-run-report-health-gate-handoff-monitor-handoff-repair")
        }));
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_summary_requires_repair_first=true"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_health_watches_empty_history() {
        let health = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new()
            .health(
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
            );

        assert_eq!(health.status, AgentRunReportHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec![
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_empty"
                    .to_owned()
            ]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_records_stable_packet()
    {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/69",
        );
        let record = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/70",
            );

        let history_record =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder::new()
                .record_handoff_with_health(
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                    &record,
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(
                    ),
                );

        assert_eq!(history_record.history.len(), 1);
        assert_eq!(
            history_record.appended_summary.handoff_health_status,
            AgentRunReportHealthStatus::Stable
        );
        assert_eq!(history_record.dashboard.total_records, 1);
        assert_eq!(history_record.dashboard.requested_admitted_records, 1);
        assert_eq!(history_record.dashboard.admitted_records, 1);
        assert_eq!(history_record.dashboard.repair_first_records, 0);
        assert_eq!(history_record.dashboard.repair_task_count, 0);
        assert_eq!(history_record.dashboard.total_next_queue_tasks, 1);
        assert_eq!(
            history_record.dashboard.latest_handoff_health_status,
            Some(AgentRunReportHealthStatus::Stable)
        );
        assert_eq!(
            history_record.health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert_eq!(history_record.records(), 1);
        assert!(history_record.allows_service_advance());
        assert!(!history_record.requires_repair_first());
        assert!(history_record.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_status=stable"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_repairs_dirty_trend() {
        let recorder =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder::new();
        let stable = trend_handoff_monitor_handoff_handoff_summary(
            AgentRunReportHealthStatus::Stable,
            true,
            true,
            false,
            0,
            1,
            0,
        );
        let repair = trend_handoff_monitor_handoff_handoff_summary(
            AgentRunReportHealthStatus::Repair,
            false,
            false,
            true,
            5,
            6,
            5,
        );

        let first = recorder.record_summary_with_health(
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
            stable,
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            repair.clone(),
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.appended_summary, repair);
        assert_eq!(second.dashboard.total_records, 2);
        assert_eq!(second.dashboard.requested_admitted_records, 1);
        assert_eq!(second.dashboard.admitted_records, 1);
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 5);
        assert_eq!(second.dashboard.total_next_queue_tasks, 7);
        assert_eq!(second.dashboard.blocked_reasons, 5);
        assert_eq!(second.dashboard.admission_rate, 0.5);
        assert_eq!(second.dashboard.repair_first_rate, 0.5);
        assert_eq!(second.health.status, AgentRunReportHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_repair_first_rate=0.500>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_repair_records=1>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_repair_tasks=5>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_blocked_reasons=5>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_admission_rate=0.500<0.67",
            ]
        );
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_history_record_status=repair"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_preserves_stable_queue() {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/71",
        );
        let packet = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/72",
            );
        let history_record =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder::new()
                .record_handoff_with_health(
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                    &packet,
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(
                    ),
                );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGate::new()
            .evaluate("run/73", &packet, &history_record);

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.packet_health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_status=stable"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_preserves_watch_queue() {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/74",
        );
        let packet = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/75",
            );
        let history_record =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder::new()
                .record_handoff_with_health(
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                    &packet,
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy {
                        minimum_admission_rate: 1.1,
                        ..AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default()
                    },
                );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGate::new()
            .evaluate("run/76", &packet, &history_record);

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.packet_health.status,
            AgentRunReportHealthStatus::Watch
        );
        assert_eq!(
            decision.packet_health.reasons,
            vec![
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_admission_rate=1.000<1.1"
                    .to_owned()
            ]
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_blocks_repair_history() {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/77",
        );
        let packet = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/78",
            );
        let recorder =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder::new();
        let first = recorder.record_summary_with_health(
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
            trend_handoff_monitor_handoff_handoff_summary(
                AgentRunReportHealthStatus::Stable,
                true,
                true,
                false,
                0,
                1,
                0,
            ),
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
        );
        let history_record = recorder.record_summary_with_health(
            first.history,
            trend_handoff_monitor_handoff_handoff_summary(
                AgentRunReportHealthStatus::Repair,
                false,
                false,
                true,
                5,
                6,
                5,
            ),
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
        );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGate::new()
            .evaluate("run/79", &packet, &history_record);

        assert!(decision.requested_admitted);
        assert!(!decision.is_admitted());
        assert_eq!(
            decision.packet_health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert_eq!(decision.repair_tasks.len(), 5);
        assert!(decision.repair_tasks.iter().all(|task| {
            task.lane == "agent-run-report-health-gate-handoff-monitor-handoff-handoff"
        }));
        assert!(decision.next_queue.task_ids().first().is_some_and(|id| {
            id.starts_with("agent-run-report-health-gate-handoff-monitor-handoff-handoff-repair")
        }));
        assert_eq!(decision.next_queue.len(), 6);
        assert!(
            decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert_eq!(decision.blocked_reasons, history_record.health.reasons);
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_records_and_gates_stable_packet()
     {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/80",
        );
        let packet = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/81",
            );

        let admission = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new()
            .record_and_gate(
                packet,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/82",
            );

        assert!(admission.is_admitted());
        assert_eq!(admission.history_record.history.len(), 1);
        assert_eq!(
            admission.history_record.health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert_eq!(
            admission.gate_decision.packet_health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(admission.gate_decision.repair_tasks.is_empty());
        assert_eq!(admission.next_queue().task_ids(), vec!["business-task"]);
        assert!(admission.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff=true"
        }));
        assert!(admission.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_admitted=true"
        }));

        let summary = admission.summary();
        assert_eq!(
            summary.packet_health_status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.packet_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert_eq!(summary.blocked_reasons, 0);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_status=stable"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_blocks_dirty_packet_history()
     {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/83",
        );
        let packet = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/84",
            );
        let dirty_history =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::from_summaries(
                vec![trend_handoff_monitor_handoff_handoff_summary(
                    AgentRunReportHealthStatus::Repair,
                    false,
                    false,
                    true,
                    5,
                    6,
                    5,
                )],
            );

        let admission = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new()
            .record_and_gate(
                packet,
                dirty_history,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/85",
            );

        assert!(!admission.is_admitted());
        assert_eq!(admission.history_record.history.len(), 2);
        assert_eq!(
            admission.history_record.health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(admission.gate_decision.requires_repair_first);
        assert_eq!(admission.gate_decision.repair_tasks.len(), 5);
        assert!(admission.gate_decision.repair_tasks.iter().all(|task| {
            task.lane == "agent-run-report-health-gate-handoff-monitor-handoff-handoff"
        }));
        assert!(admission.next_queue().task_ids().first().is_some_and(|id| {
            id.starts_with("agent-run-report-health-gate-handoff-monitor-handoff-handoff-repair")
        }));
        assert!(
            admission
                .next_queue()
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert_eq!(
            admission.gate_decision.blocked_reasons,
            admission.history_record.health.reasons
        );
        assert!(admission.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_requires_repair_first=true"
        }));

        let summary = admission.summary();
        assert_eq!(
            summary.packet_health_status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.packet_records, 2);
        assert_eq!(summary.repair_tasks, 5);
        assert_eq!(summary.next_queue_tasks, 6);
        assert_eq!(summary.blocked_reasons, 5);
        assert!(summary.repair_task_ids.iter().all(|id| {
            id.starts_with("agent-run-report-health-gate-handoff-monitor-handoff-handoff-repair")
        }));
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_requires_repair_first=true"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_health_watches_empty_history()
     {
        let health =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new()
                .health(
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(),
                );

        assert_eq!(health.status, AgentRunReportHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec![
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_empty"
                    .to_owned()
            ]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_records_stable_admission()
     {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/86",
        );
        let packet = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/87",
            );
        let admission = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new()
            .record_and_gate(
                packet,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/88",
            );

        let history_record =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder::new()
                .record_admission_with_health(
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new(),
                    &admission,
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(),
                );

        assert_eq!(history_record.history.len(), 1);
        assert_eq!(
            history_record.appended_summary.packet_health_status,
            AgentRunReportHealthStatus::Stable
        );
        assert_eq!(history_record.dashboard.total_records, 1);
        assert_eq!(history_record.dashboard.requested_admitted_records, 1);
        assert_eq!(history_record.dashboard.admitted_records, 1);
        assert_eq!(history_record.dashboard.repair_first_records, 0);
        assert_eq!(history_record.dashboard.repair_task_count, 0);
        assert_eq!(history_record.dashboard.total_next_queue_tasks, 1);
        assert_eq!(
            history_record.dashboard.latest_packet_health_status,
            Some(AgentRunReportHealthStatus::Stable)
        );
        assert_eq!(
            history_record.health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert_eq!(history_record.records(), 1);
        assert!(history_record.allows_service_advance());
        assert!(!history_record.requires_repair_first());
        assert!(history_record.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_status=stable"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_repairs_dirty_trend()
     {
        let recorder =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder::new();
        let stable = trend_handoff_monitor_handoff_handoff_handoff_summary(
            AgentRunReportHealthStatus::Stable,
            true,
            true,
            false,
            0,
            1,
            0,
        );
        let repair = trend_handoff_monitor_handoff_handoff_handoff_summary(
            AgentRunReportHealthStatus::Repair,
            false,
            false,
            true,
            5,
            6,
            5,
        );

        let first = recorder.record_summary_with_health(
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new(),
            stable,
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            repair.clone(),
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.appended_summary, repair);
        assert_eq!(second.dashboard.total_records, 2);
        assert_eq!(second.dashboard.requested_admitted_records, 1);
        assert_eq!(second.dashboard.admitted_records, 1);
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 5);
        assert_eq!(second.dashboard.total_next_queue_tasks, 7);
        assert_eq!(second.dashboard.blocked_reasons, 5);
        assert_eq!(second.dashboard.admission_rate, 0.5);
        assert_eq!(second.dashboard.repair_first_rate, 0.5);
        assert_eq!(second.health.status, AgentRunReportHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_first_rate=0.500>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_records=1>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_tasks=5>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_blocked_reasons=5>0",
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_admission_rate=0.500<0.67",
            ]
        );
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_status=repair"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_preserves_stable_queue()
     {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/89",
        );
        let packet = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/90",
            );
        let admission = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new()
            .record_and_gate(
                packet,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/91",
            );
        let history_record =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder::new()
                .record_admission_with_health(
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new(),
                    &admission,
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(),
                );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGate::new()
            .evaluate("run/92", &admission, &history_record);

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.admission_health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_status=stable"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_preserves_watch_queue()
     {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/93",
        );
        let packet = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/94",
            );
        let admission = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new()
            .record_and_gate(
                packet,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/95",
            );
        let history_record =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder::new()
                .record_admission_with_health(
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new(),
                    &admission,
                    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy {
                        minimum_admission_rate: 1.1,
                        ..AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default()
                    },
                );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGate::new()
            .evaluate("run/96", &admission, &history_record);

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert_eq!(
            decision.admission_health.status,
            AgentRunReportHealthStatus::Watch
        );
        assert_eq!(
            decision.admission_health.reasons,
            vec![
                "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_admission_rate=1.000<1.1"
                    .to_owned()
            ]
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_blocks_repair_history()
     {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/97",
        );
        let packet = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/98",
            );
        let admission = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new()
            .record_and_gate(
                packet,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/99",
            );
        let recorder =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder::new();
        let first = recorder.record_summary_with_health(
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new(),
            trend_handoff_monitor_handoff_handoff_handoff_summary(
                AgentRunReportHealthStatus::Stable,
                true,
                true,
                false,
                0,
                1,
                0,
            ),
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(),
        );
        let history_record = recorder.record_summary_with_health(
            first.history,
            trend_handoff_monitor_handoff_handoff_handoff_summary(
                AgentRunReportHealthStatus::Repair,
                false,
                false,
                true,
                5,
                6,
                5,
            ),
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(),
        );

        let decision = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGate::new()
            .evaluate("run/100", &admission, &history_record);

        assert!(decision.requested_admitted);
        assert!(!decision.is_admitted());
        assert_eq!(
            decision.admission_health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert_eq!(decision.repair_tasks.len(), 5);
        assert!(decision.repair_tasks.iter().all(|task| {
            task.lane == "agent-run-report-health-gate-handoff-monitor-handoff-handoff-handoff"
        }));
        assert!(decision.next_queue.task_ids().first().is_some_and(|id| {
            id.starts_with(
                "agent-run-report-health-gate-handoff-monitor-handoff-handoff-handoff-repair",
            )
        }));
        assert_eq!(decision.next_queue.len(), 6);
        assert!(
            decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert_eq!(decision.blocked_reasons, history_record.health.reasons);
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_records_stable_trend()
     {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/101",
        );
        let packet = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/102",
            );
        let admission = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new()
            .record_and_gate(
                packet,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/103",
            );

        let record = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff::new()
            .record_and_gate(
                admission,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(),
                "run/104",
            );

        assert!(record.is_admitted());
        assert_eq!(record.history_record.history.len(), 1);
        assert_eq!(
            record.history_record.health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert_eq!(
            record.gate_decision.admission_health.status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(record.gate_decision.repair_tasks.is_empty());
        assert_eq!(record.next_queue().task_ids(), vec!["business-task"]);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff=true"
        }));
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_admitted=true"
        }));

        let summary = record.summary();
        assert_eq!(
            summary.admission_health_status,
            AgentRunReportHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.admission_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert_eq!(summary.blocked_reasons, 0);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_status=stable"
        }));
    }

    #[test]
    fn run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_repairs_dirty_trend()
     {
        let handoff = AgentRunReportHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            stable_trend_handoff_monitor_record(),
            AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentRunReportHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/105",
        );
        let packet = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                handoff,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/106",
            );
        let admission = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new()
            .record_and_gate(
                packet,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/107",
            );
        let dirty_history =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::from_summaries(
                vec![trend_handoff_monitor_handoff_handoff_handoff_summary(
                    AgentRunReportHealthStatus::Repair,
                    false,
                    false,
                    true,
                    5,
                    6,
                    5,
                )],
            );

        let record = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff::new()
            .record_and_gate(
                admission,
                dirty_history,
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(),
                "run/108",
            );

        assert!(!record.is_admitted());
        assert_eq!(record.history_record.history.len(), 2);
        assert_eq!(
            record.history_record.health.status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(record.gate_decision.requires_repair_first);
        assert_eq!(record.gate_decision.repair_tasks.len(), 5);
        assert!(record.gate_decision.repair_tasks.iter().all(|task| {
            task.lane == "agent-run-report-health-gate-handoff-monitor-handoff-handoff-handoff"
        }));
        assert!(record.next_queue().task_ids().first().is_some_and(|id| {
            id.starts_with(
                "agent-run-report-health-gate-handoff-monitor-handoff-handoff-handoff-repair",
            )
        }));
        assert_eq!(record.next_queue().len(), 6);
        assert!(
            record
                .next_queue()
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert_eq!(
            record.gate_decision.blocked_reasons,
            record.history_record.health.reasons
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_requires_repair_first=true"
        }));

        let summary = record.summary();
        assert_eq!(
            summary.admission_health_status,
            AgentRunReportHealthStatus::Repair
        );
        assert!(summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.admission_records, 2);
        assert_eq!(summary.repair_tasks, 5);
        assert_eq!(summary.next_queue_tasks, 6);
        assert_eq!(summary.blocked_reasons, 5);
        assert!(summary.repair_task_ids.iter().all(|id| {
            id.starts_with(
                "agent-run-report-health-gate-handoff-monitor-handoff-handoff-handoff-repair",
            )
        }));
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_run_report_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_requires_repair_first=true"
        }));
    }

    #[test]
    fn unresolved_conflict_blocks_memory_note_side_effect() {
        let coder = AgentTask::new(
            "coder-task",
            AgentRole::Coder,
            "patch",
            AgentBudget::new(8, 1, 1),
        );
        let reviewer = AgentTask::new(
            "reviewer-task",
            AgentRole::Reviewer,
            "review",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![
                TaskAssignment {
                    task_id: coder.id.clone(),
                    role: coder.role.clone(),
                    lane: coder.lane.clone(),
                    budget_reserved: coder.required_budget,
                },
                TaskAssignment {
                    task_id: reviewer.id.clone(),
                    role: reviewer.role.clone(),
                    lane: reviewer.lane.clone(),
                    budget_reserved: reviewer.required_budget,
                },
            ],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);
        ledger.record_result(AgentResult::accepted(
            &coder,
            "coder approved",
            vec![AgentMessage::new(
                "m-coder",
                AgentRole::Coder,
                AgentMessageKind::Decision,
                "patch",
                "approve patch and proceed",
            )],
            AgentBudget::new(2, 1, 1),
        ));
        ledger.record_result(AgentResult::accepted(
            &reviewer,
            "reviewer blocked",
            vec![AgentMessage::new(
                "m-reviewer",
                AgentRole::Reviewer,
                AgentMessageKind::Risk,
                "patch",
                "block patch until validation passes",
            )],
            AgentBudget::new(2, 1, 1),
        ));
        let conflicts = ledger.conflict_report();
        let mut reflection = ReflectionLoop::new();
        reflection
            .submit(ReflectionStage::Draft, "draft conclusion")
            .unwrap();
        reflection
            .submit(ReflectionStage::Critique, "risk remains")
            .unwrap();
        reflection
            .submit(ReflectionStage::Revision, "hold memory write")
            .unwrap();
        reflection
            .submit(ReflectionStage::MemoryNote, "remember validation gate")
            .unwrap();

        let gate =
            ledger.gate_side_effect(SideEffectKind::MemoryNote, &conflicts, Some(&reflection));

        assert!(!gate.allowed);
        assert_eq!(conflicts.unresolved_count(), 1);
        assert!(gate.reason.contains("unresolved conflict"));

        let report = ledger.report(Some(&reflection));
        let summary = report.summary();
        let run_gate = report.gate();

        assert_eq!(summary.unresolved_conflicts, 1);
        assert_eq!(summary.blocked_side_effects, 4);
        assert!(!summary.memory_note_allowed);
        assert!(run_gate.requires_repair_first);
        assert!(!run_gate.can_promote_memory_note);
        assert!(
            run_gate
                .reasons
                .iter()
                .any(|reason| reason == "unresolved_conflicts=1")
        );
    }

    #[test]
    fn missing_window_result_blocks_all_run_side_effects() {
        let first = AgentTask::new(
            "window-1",
            AgentRole::Custom("window-1".to_owned()),
            "first lane",
            AgentBudget::new(8, 1, 1),
        );
        let second = AgentTask::new(
            "window-2",
            AgentRole::Custom("window-2".to_owned()),
            "second lane",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![
                TaskAssignment {
                    task_id: first.id.clone(),
                    role: first.role.clone(),
                    lane: first.lane.clone(),
                    budget_reserved: first.required_budget,
                },
                TaskAssignment {
                    task_id: second.id.clone(),
                    role: second.role.clone(),
                    lane: second.lane.clone(),
                    budget_reserved: second.required_budget,
                },
            ],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);
        ledger.record_result(AgentResult::accepted(
            &first,
            "first completed",
            vec![AgentMessage::new(
                "m-window-1",
                AgentRole::Reviewer,
                AgentMessageKind::Finding,
                "handoff",
                "first window completed",
            )],
            AgentBudget::new(2, 1, 1),
        ));
        let mut reflection = ReflectionLoop::new();
        reflection.submit(ReflectionStage::Draft, "draft").unwrap();
        reflection
            .submit(ReflectionStage::Critique, "critique")
            .unwrap();
        reflection
            .submit(ReflectionStage::Revision, "revision")
            .unwrap();
        reflection
            .submit(ReflectionStage::MemoryNote, "remember")
            .unwrap();

        let progress = ledger.progress();
        let report = ledger.report(Some(&reflection));
        let summary = report.summary();
        let gate = report.gate();

        assert_eq!(progress.assigned_tasks, 2);
        assert_eq!(progress.reported_tasks, 1);
        assert_eq!(progress.accepted_results, 1);
        assert_eq!(progress.rejected_results, 0);
        assert_eq!(progress.missing_task_ids, vec!["window-2".to_owned()]);
        assert_eq!(progress.unassigned_task_ids, Vec::<String>::new());
        assert!(!progress.can_close_run);
        assert!(progress.requires_repair_first);
        assert!(
            progress
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_ledger_progress_missing_assigned_tasks=1" })
        );
        assert_eq!(report.conflicts.unresolved_count(), 0);
        assert_eq!(summary.blocked_side_effects, 4);
        assert!(!summary.memory_note_allowed);
        assert!(!summary.adaptive_state_allowed);
        assert!(!summary.external_call_allowed);
        assert!(!summary.all_side_effects_allowed);
        assert!(gate.requires_repair_first);
        assert!(!gate.can_promote_memory_note);
        assert!(gate.reasons.iter().any(|reason| {
            reason
                == "side_effect_blocked kind=memory_note reason=blocked by run ledger progress missing=1 rejected=0 dispatch_rejections=0 unassigned=0 empty_dispatch=false"
        }));
    }

    #[test]
    fn budget_exhausted_dispatch_blocks_run_close_side_effects_and_ready_wave() {
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
        let dispatch_gate = dispatch.gate();

        assert!(dispatch.assignments.is_empty());
        assert_eq!(dispatch.rejections.len(), 1);
        assert!(!dispatch_gate.can_dispatch);
        assert!(dispatch_gate.requires_repair_first);
        assert!(
            dispatch_gate
                .reasons
                .iter()
                .any(|reason| reason.contains("insufficient budget"))
        );

        let ledger = AgentRunLedger::new(dispatch);
        let progress = ledger.progress();
        let mut reflection = ReflectionLoop::new();
        reflection.submit(ReflectionStage::Draft, "draft").unwrap();
        reflection
            .submit(ReflectionStage::Critique, "critique")
            .unwrap();
        reflection
            .submit(ReflectionStage::Revision, "revision")
            .unwrap();
        reflection
            .submit(ReflectionStage::MemoryNote, "remember")
            .unwrap();

        assert_eq!(progress.assigned_tasks, 0);
        assert_eq!(progress.dispatch_rejections, 1);
        assert!(progress.empty_dispatch);
        assert!(!progress.can_close_run);
        assert!(progress.requires_repair_first);
        assert!(ledger.try_close_report(Some(&reflection)).is_none());

        let conflicts = ledger.conflict_report();
        let mut side_effects = Vec::new();
        for kind in [
            SideEffectKind::MemoryNote,
            SideEffectKind::FileWrite,
            SideEffectKind::AdaptiveStateWrite,
            SideEffectKind::ExternalCall,
        ] {
            let gate = ledger.gate_side_effect(kind, &conflicts, Some(&reflection));
            assert!(!gate.allowed);
            assert_eq!(
                gate.reason,
                "blocked by run ledger progress missing=0 rejected=0 dispatch_rejections=1 unassigned=0 empty_dispatch=true"
            );
            side_effects.push(gate);
        }
        let blocked_report = AgentRunReport {
            aggregation: ledger.aggregation_report(),
            conflicts,
            budget_audit: ledger.budget_audit(),
            side_effects,
        };
        let blocked_summary = blocked_report.summary();
        let blocked_gate = blocked_report.gate();

        assert_eq!(blocked_summary.side_effects, 4);
        assert_eq!(blocked_summary.blocked_side_effects, 4);
        assert!(!blocked_summary.memory_note_allowed);
        assert!(!blocked_summary.adaptive_state_allowed);
        assert!(!blocked_summary.external_call_allowed);
        assert!(!blocked_summary.all_side_effects_allowed);
        assert!(blocked_gate.requires_repair_first);
        assert!(!blocked_gate.can_promote_memory_note);
        assert!(!blocked_gate.can_write_adaptive_state);
        assert!(!blocked_gate.can_dispatch_external_call);

        let next_queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "normal-follow-up",
            AgentRole::Planner,
            "continue only after budget repair and close gates",
            AgentBudget::new(4, 1, 1),
        )]);
        let progress_record = AgentRunLedgerProgressSummaryHistoryRecorder::new()
            .record_progress_with_health(
                AgentRunLedgerProgressSummaryHistory::new(),
                &progress,
                AgentRunLedgerProgressHealthPolicy::default(),
            );
        let report_record = AgentRunReportSummaryHistoryRecorder::new()
            .record_summary_with_health_gate(
                AgentRunReportSummaryHistory::new(),
                run_summary(0, 0, 4, false, false, false),
                AgentRunReportHealthPolicy::default(),
                "run/budget-exhausted",
                &next_queue,
            );

        let gate_record = AgentRunProgressReportGate::new().gate(
            "run/budget-exhausted",
            progress_record,
            report_record,
        );
        let repair_task_ids = gate_record
            .progress_repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let schedule = RecursiveAgentScheduler::new(8).plan(gate_record.next_queue().tasks());

        assert!(!gate_record.is_admitted());
        assert!(gate_record.requires_repair_first);
        assert!(gate_record.blocked_reasons.iter().any(|reason| {
            reason == "progress:agent_run_ledger_progress_dispatch_rejections=1>0"
        }));
        assert!(gate_record.blocked_reasons.iter().any(|reason| {
            reason == "progress:agent_run_ledger_progress_empty_dispatch_records=1>0"
        }));
        assert!(
            gate_record
                .next_queue
                .task_ids()
                .contains(&"normal-follow-up".to_owned())
        );
        assert_eq!(schedule.waves[0].task_ids, repair_task_ids);
        assert!(
            !schedule.waves[0]
                .task_ids
                .contains(&"normal-follow-up".to_owned())
        );
        assert_eq!(
            schedule.waves.last().map(|wave| wave.task_ids.as_slice()),
            Some(&["normal-follow-up".to_owned()][..])
        );
        assert!(
            schedule.waves[..schedule.wave_count().saturating_sub(1)]
                .iter()
                .all(|wave| !wave.task_ids.contains(&"normal-follow-up".to_owned()))
        );
    }

    #[test]
    fn rejected_or_unassigned_window_result_blocks_run_close() {
        let assigned = AgentTask::new(
            "window-1",
            AgentRole::Custom("window-1".to_owned()),
            "assigned lane",
            AgentBudget::new(8, 1, 1),
        );
        let rogue = AgentTask::new(
            "window-3",
            AgentRole::Custom("window-3".to_owned()),
            "unassigned lane",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![TaskAssignment {
                task_id: assigned.id.clone(),
                role: assigned.role.clone(),
                lane: assigned.lane.clone(),
                budget_reserved: assigned.required_budget,
            }],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);
        ledger.record_result(AgentResult::rejected(&assigned, "assigned window failed"));
        ledger.record_result(AgentResult::accepted(
            &rogue,
            "unassigned window reported",
            vec![AgentMessage::new(
                "m-window-3",
                AgentRole::Reviewer,
                AgentMessageKind::Finding,
                "handoff",
                "unassigned window should not close the run",
            )],
            AgentBudget::new(1, 1, 1),
        ));
        let mut reflection = ReflectionLoop::new();
        reflection.submit(ReflectionStage::Draft, "draft").unwrap();
        reflection
            .submit(ReflectionStage::Critique, "critique")
            .unwrap();
        reflection
            .submit(ReflectionStage::Revision, "revision")
            .unwrap();
        reflection
            .submit(ReflectionStage::MemoryNote, "remember")
            .unwrap();

        let progress = ledger.progress();
        let report = ledger.report(Some(&reflection));

        assert_eq!(progress.assigned_tasks, 1);
        assert_eq!(progress.reported_tasks, 2);
        assert_eq!(progress.accepted_results, 1);
        assert_eq!(progress.rejected_results, 1);
        assert_eq!(progress.missing_task_ids, Vec::<String>::new());
        assert_eq!(progress.rejected_task_ids, vec!["window-1".to_owned()]);
        assert_eq!(progress.unassigned_task_ids, vec!["window-3".to_owned()]);
        assert!(!progress.can_close_run);
        assert!(progress.requires_repair_first);
        assert!(report.side_effects.iter().all(|gate| !gate.allowed));
        assert!(report.side_effects.iter().all(|gate| {
            gate.reason
                == "blocked by run ledger progress missing=0 rejected=1 dispatch_rejections=0 unassigned=1 empty_dispatch=false"
        }));
        assert!(report.gate().requires_repair_first);
    }

    #[test]
    fn run_ledger_progress_history_watches_empty() {
        let health = AgentRunLedgerProgressSummaryHistory::new()
            .health(AgentRunLedgerProgressHealthPolicy::default());

        assert_eq!(health.status, AgentRunReportHealthStatus::Watch);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["agent_run_ledger_progress_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_ledger_progress_dashboard_records=0" })
        );
    }

    #[test]
    fn run_ledger_progress_history_records_stable_close() {
        let task = AgentTask::new(
            "window-1",
            AgentRole::Custom("window-1".to_owned()),
            "complete assigned lane",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![TaskAssignment {
                task_id: task.id.clone(),
                role: task.role.clone(),
                lane: task.lane.clone(),
                budget_reserved: task.required_budget,
            }],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);
        ledger.record_result(AgentResult::accepted(
            &task,
            "window completed",
            vec![AgentMessage::new(
                "m-window-1",
                AgentRole::Reviewer,
                AgentMessageKind::Finding,
                "close",
                "assigned window completed",
            )],
            AgentBudget::new(1, 1, 1),
        ));
        let progress = ledger.progress();

        let record = AgentRunLedgerProgressSummaryHistoryRecorder::new()
            .record_progress_with_health(
                AgentRunLedgerProgressSummaryHistory::new(),
                &progress,
                AgentRunLedgerProgressHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert!(record.appended_summary.can_close_run);
        assert!(!record.appended_summary.requires_repair_first);
        assert_eq!(record.dashboard.closable_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert_eq!(record.dashboard.total_assigned_tasks, 1);
        assert_eq!(record.dashboard.total_reported_tasks, 1);
        assert_eq!(record.dashboard.close_rate, 1.0);
        assert_eq!(record.dashboard.latest_can_close_run, Some(true));
        assert_eq!(record.health.status, AgentRunReportHealthStatus::Stable);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_ledger_progress_history_record_status=stable" })
        );
    }

    #[test]
    fn run_ledger_progress_history_repairs_missing_rejected_and_unassigned_results() {
        let first = AgentTask::new(
            "window-1",
            AgentRole::Custom("window-1".to_owned()),
            "rejected assigned lane",
            AgentBudget::new(8, 1, 1),
        );
        let second = AgentTask::new(
            "window-2",
            AgentRole::Custom("window-2".to_owned()),
            "missing assigned lane",
            AgentBudget::new(8, 1, 1),
        );
        let rogue = AgentTask::new(
            "window-3",
            AgentRole::Custom("window-3".to_owned()),
            "unassigned lane",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![
                TaskAssignment {
                    task_id: first.id.clone(),
                    role: first.role.clone(),
                    lane: first.lane.clone(),
                    budget_reserved: first.required_budget,
                },
                TaskAssignment {
                    task_id: second.id.clone(),
                    role: second.role.clone(),
                    lane: second.lane.clone(),
                    budget_reserved: second.required_budget,
                },
            ],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);
        ledger.record_result(AgentResult::rejected(&first, "window rejected"));
        ledger.record_result(AgentResult::accepted(
            &rogue,
            "unassigned result",
            vec![AgentMessage::new(
                "m-window-3",
                AgentRole::Reviewer,
                AgentMessageKind::Finding,
                "close",
                "unassigned result arrived",
            )],
            AgentBudget::new(1, 1, 1),
        ));
        let progress = ledger.progress();

        let record = AgentRunLedgerProgressSummaryHistoryRecorder::new()
            .record_progress_with_health(
                AgentRunLedgerProgressSummaryHistory::new(),
                &progress,
                AgentRunLedgerProgressHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert!(!record.appended_summary.can_close_run);
        assert!(record.appended_summary.requires_repair_first);
        assert_eq!(record.appended_summary.missing_assigned_tasks, 1);
        assert_eq!(record.appended_summary.rejected_results, 1);
        assert_eq!(record.appended_summary.unassigned_results, 1);
        assert_eq!(record.dashboard.closable_records, 0);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.total_missing_assigned_tasks, 1);
        assert_eq!(record.dashboard.total_rejected_results, 1);
        assert_eq!(record.dashboard.total_unassigned_results, 1);
        assert_eq!(record.dashboard.close_rate, 0.0);
        assert_eq!(record.dashboard.latest_can_close_run, Some(false));
        assert_eq!(record.health.status, AgentRunReportHealthStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "agent_run_ledger_progress_repair_first_records=1>0",
                "agent_run_ledger_progress_rejected_results=1>0",
                "agent_run_ledger_progress_missing_assigned_tasks=1>0",
                "agent_run_ledger_progress_unassigned_results=1>0",
                "agent_run_ledger_progress_close_rate=0.000<1",
            ]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_ledger_progress_history_record_status=repair" })
        );
    }

    #[test]
    fn incomplete_reflection_blocks_only_memory_note_side_effect() {
        let coder = AgentTask::new(
            "coder-task",
            AgentRole::Coder,
            "patch",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![TaskAssignment {
                task_id: coder.id.clone(),
                role: coder.role.clone(),
                lane: coder.lane.clone(),
                budget_reserved: coder.required_budget,
            }],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);
        ledger.record_result(AgentResult::accepted(
            &coder,
            "coder found stable patch",
            vec![AgentMessage::new(
                "m-coder",
                AgentRole::Coder,
                AgentMessageKind::Finding,
                "patch",
                "patch validation passed",
            )],
            AgentBudget::new(2, 1, 1),
        ));
        let mut reflection = ReflectionLoop::new();
        reflection
            .submit(ReflectionStage::Draft, "draft conclusion")
            .unwrap();
        reflection
            .submit(ReflectionStage::Critique, "still needs revision")
            .unwrap();

        let report = ledger.report(Some(&reflection));
        let summary = report.summary();
        let run_gate = report.gate();
        let memory_gate = report
            .side_effects
            .iter()
            .find(|gate| gate.kind == SideEffectKind::MemoryNote)
            .unwrap();

        assert_eq!(report.conflicts.unresolved_count(), 0);
        assert!(!memory_gate.allowed);
        assert_eq!(
            memory_gate.reason,
            "memory note requires a complete reflection loop"
        );
        assert!(
            report
                .side_effects
                .iter()
                .any(|gate| { gate.kind == SideEffectKind::FileWrite && gate.allowed })
        );
        assert!(
            report
                .side_effects
                .iter()
                .any(|gate| { gate.kind == SideEffectKind::AdaptiveStateWrite && gate.allowed })
        );
        assert!(
            report
                .side_effects
                .iter()
                .any(|gate| { gate.kind == SideEffectKind::ExternalCall && gate.allowed })
        );
        assert_eq!(summary.blocked_side_effects, 1);
        assert!(!summary.memory_note_allowed);
        assert!(summary.adaptive_state_allowed);
        assert!(summary.external_call_allowed);
        assert!(run_gate.requires_repair_first);
        assert!(!run_gate.can_promote_memory_note);
        assert!(!run_gate.can_write_adaptive_state);
        assert!(!run_gate.can_dispatch_external_call);
        assert!(run_gate.reasons.iter().any(|reason| {
            reason == "side_effect_blocked kind=memory_note reason=memory note requires a complete reflection loop"
        }));
    }

    #[test]
    fn agent_results_merge_in_dispatch_order_not_completion_order() {
        let first = AgentTask::new(
            "window-1",
            AgentRole::Custom("window-1".to_owned()),
            "first lane",
            AgentBudget::new(8, 1, 1),
        );
        let second = AgentTask::new(
            "window-2",
            AgentRole::Custom("window-2".to_owned()),
            "second lane",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![
                TaskAssignment {
                    task_id: first.id.clone(),
                    role: first.role.clone(),
                    lane: first.lane.clone(),
                    budget_reserved: first.required_budget,
                },
                TaskAssignment {
                    task_id: second.id.clone(),
                    role: second.role.clone(),
                    lane: second.lane.clone(),
                    budget_reserved: second.required_budget,
                },
            ],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);
        ledger.record_result(AgentResult::accepted(
            &second,
            "completed second",
            vec![AgentMessage::new(
                "m2",
                second.role.clone(),
                AgentMessageKind::Finding,
                "order",
                "second result",
            )],
            AgentBudget::new(1, 1, 1),
        ));
        ledger.record_result(AgentResult::accepted(
            &first,
            "completed first",
            vec![AgentMessage::new(
                "m1",
                first.role.clone(),
                AgentMessageKind::Finding,
                "order",
                "first result",
            )],
            AgentBudget::new(1, 1, 1),
        ));

        let ordered_task_ids = ledger
            .ordered_results()
            .into_iter()
            .map(|result| result.task_id.clone())
            .collect::<Vec<_>>();
        let ordered_message_ids = ledger
            .ordered_messages()
            .into_iter()
            .map(|message| message.id)
            .collect::<Vec<_>>();

        assert_eq!(ordered_task_ids, vec!["window-1", "window-2"]);
        assert_eq!(ordered_message_ids, vec!["m1", "m2"]);
    }

    #[test]
    fn agent_results_append_unassigned_results_in_task_id_order() {
        let first = AgentTask::new(
            "window-1",
            AgentRole::Custom("window-1".to_owned()),
            "first lane",
            AgentBudget::new(8, 1, 1),
        );
        let second = AgentTask::new(
            "window-2",
            AgentRole::Custom("window-2".to_owned()),
            "second lane",
            AgentBudget::new(8, 1, 1),
        );
        let loose_a = AgentTask::new(
            "loose-a",
            AgentRole::Custom("loose-a".to_owned()),
            "loose lane a",
            AgentBudget::new(1, 1, 1),
        );
        let loose_b = AgentTask::new(
            "loose-b",
            AgentRole::Custom("loose-b".to_owned()),
            "loose lane b",
            AgentBudget::new(1, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![
                TaskAssignment {
                    task_id: first.id.clone(),
                    role: first.role.clone(),
                    lane: first.lane.clone(),
                    budget_reserved: first.required_budget,
                },
                TaskAssignment {
                    task_id: second.id.clone(),
                    role: second.role.clone(),
                    lane: second.lane.clone(),
                    budget_reserved: second.required_budget,
                },
            ],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);

        for task in [&loose_b, &second, &loose_a, &first] {
            ledger.record_result(AgentResult::accepted(
                task,
                format!("completed {}", task.id),
                Vec::new(),
                AgentBudget::new(1, 1, 1),
            ));
        }

        let ordered_task_ids = ledger
            .ordered_results()
            .into_iter()
            .map(|result| result.task_id.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            ordered_task_ids,
            vec!["window-1", "window-2", "loose-a", "loose-b"]
        );
    }

    #[test]
    fn agent_run_report_keeps_unassigned_aggregation_stable_across_arrival_order() {
        fn build_ledger(reverse_arrival: bool) -> AgentRunLedger {
            let assigned = AgentTask::new(
                "window-1",
                AgentRole::Custom("window-1".to_owned()),
                "assigned lane",
                AgentBudget::new(8, 1, 1),
            );
            let loose_a = AgentTask::new(
                "loose-a",
                AgentRole::Custom("loose-a".to_owned()),
                "loose lane a",
                AgentBudget::new(1, 1, 1),
            );
            let loose_b = AgentTask::new(
                "loose-b",
                AgentRole::Custom("loose-b".to_owned()),
                "loose lane b",
                AgentBudget::new(1, 1, 1),
            );
            let dispatch = TaskDispatchPlan {
                assignments: vec![TaskAssignment {
                    task_id: assigned.id.clone(),
                    role: assigned.role.clone(),
                    lane: assigned.lane.clone(),
                    budget_reserved: assigned.required_budget,
                }],
                ..TaskDispatchPlan::default()
            };
            let assigned_result = AgentResult::accepted(
                &assigned,
                "assigned completed",
                vec![AgentMessage::new(
                    "assigned-message",
                    assigned.role.clone(),
                    AgentMessageKind::Finding,
                    "handoff",
                    "assigned message",
                )],
                AgentBudget::new(1, 1, 1),
            );
            let loose_a_result = AgentResult::accepted(
                &loose_a,
                "loose a reported",
                vec![AgentMessage::new(
                    "loose-a-message",
                    loose_a.role.clone(),
                    AgentMessageKind::Finding,
                    "handoff",
                    "loose a message",
                )],
                AgentBudget::new(1, 1, 1),
            );
            let loose_b_result = AgentResult::accepted(
                &loose_b,
                "loose b reported",
                vec![AgentMessage::new(
                    "loose-b-message",
                    loose_b.role.clone(),
                    AgentMessageKind::Finding,
                    "handoff",
                    "loose b message",
                )],
                AgentBudget::new(1, 1, 1),
            );
            let mut ledger = AgentRunLedger::new(dispatch);
            if reverse_arrival {
                ledger.record_result(loose_b_result);
                ledger.record_result(loose_a_result);
                ledger.record_result(assigned_result);
            } else {
                ledger.record_result(assigned_result);
                ledger.record_result(loose_a_result);
                ledger.record_result(loose_b_result);
            }

            ledger
        }

        let forward_ledger = build_ledger(false);
        let reversed_ledger = build_ledger(true);
        let forward_ordered_task_ids = forward_ledger
            .ordered_results()
            .into_iter()
            .map(|result| result.task_id.clone())
            .collect::<Vec<_>>();
        let reversed_ordered_task_ids = reversed_ledger
            .ordered_results()
            .into_iter()
            .map(|result| result.task_id.clone())
            .collect::<Vec<_>>();
        let forward = forward_ledger.report(None);
        let reversed = reversed_ledger.report(None);
        let aggregated_message_ids = forward
            .aggregation
            .messages
            .iter()
            .map(|message| message.message.id.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            forward_ordered_task_ids,
            vec!["window-1", "loose-a", "loose-b"]
        );
        assert_eq!(reversed_ordered_task_ids, forward_ordered_task_ids);
        assert_eq!(forward.aggregation, reversed.aggregation);
        assert_eq!(forward.summary(), reversed.summary());
        assert_eq!(
            aggregated_message_ids,
            vec!["loose-a-message", "loose-b-message", "assigned-message"]
        );
        assert_eq!(forward.aggregation.input_count, 3);
        assert_eq!(forward.aggregation.unique_count, 3);
        assert_eq!(forward.summary().blocked_side_effects, 4);
        assert!(forward.gate().requires_repair_first);
        assert!(forward.gate().reasons.iter().any(|reason| {
            reason
                == "side_effect_blocked kind=memory_note reason=blocked by run ledger progress missing=0 rejected=0 dispatch_rejections=0 unassigned=2 empty_dispatch=false"
        }));
    }

    #[test]
    fn agent_results_replace_replayed_task_without_duplicate_messages() {
        let first = AgentTask::new(
            "window-1",
            AgentRole::Custom("window-1".to_owned()),
            "first lane",
            AgentBudget::new(8, 1, 1),
        );
        let second = AgentTask::new(
            "window-2",
            AgentRole::Custom("window-2".to_owned()),
            "second lane",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![
                TaskAssignment {
                    task_id: first.id.clone(),
                    role: first.role.clone(),
                    lane: first.lane.clone(),
                    budget_reserved: first.required_budget,
                },
                TaskAssignment {
                    task_id: second.id.clone(),
                    role: second.role.clone(),
                    lane: second.lane.clone(),
                    budget_reserved: second.required_budget,
                },
            ],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);
        ledger.record_result(AgentResult::accepted(
            &first,
            "stale first",
            vec![AgentMessage::new(
                "m1-stale",
                first.role.clone(),
                AgentMessageKind::Finding,
                "order",
                "stale first result",
            )],
            AgentBudget::new(1, 1, 1),
        ));
        ledger.record_result(AgentResult::accepted(
            &second,
            "completed second",
            vec![AgentMessage::new(
                "m2",
                second.role.clone(),
                AgentMessageKind::Finding,
                "order",
                "second result",
            )],
            AgentBudget::new(1, 1, 1),
        ));
        ledger.record_result(AgentResult::accepted(
            &first,
            "replayed first",
            vec![AgentMessage::new(
                "m1-replayed",
                first.role.clone(),
                AgentMessageKind::Finding,
                "order",
                "replayed first result",
            )],
            AgentBudget::new(2, 1, 1),
        ));

        let ordered_task_ids = ledger
            .ordered_results()
            .into_iter()
            .map(|result| result.task_id.clone())
            .collect::<Vec<_>>();
        let ordered_message_ids = ledger
            .ordered_messages()
            .into_iter()
            .map(|message| message.id)
            .collect::<Vec<_>>();
        let first_result = ledger.result("window-1").unwrap();

        assert_eq!(ordered_task_ids, vec!["window-1", "window-2"]);
        assert_eq!(ordered_message_ids, vec!["m1-replayed", "m2"]);
        assert_eq!(first_result.summary, "replayed first");
        assert_eq!(first_result.budget_spent, AgentBudget::new(2, 1, 1));
    }

    #[test]
    fn agent_run_report_aggregation_stays_stable_when_window_results_arrive_reversed() {
        fn build_report(reverse_arrival: bool) -> AgentRunReport {
            let first = AgentTask::new(
                "window-1",
                AgentRole::Custom("window-1".to_owned()),
                "first lane",
                AgentBudget::new(8, 1, 1),
            );
            let second = AgentTask::new(
                "window-2",
                AgentRole::Custom("window-2".to_owned()),
                "second lane",
                AgentBudget::new(8, 1, 1),
            );
            let dispatch = TaskDispatchPlan {
                assignments: vec![
                    TaskAssignment {
                        task_id: first.id.clone(),
                        role: first.role.clone(),
                        lane: first.lane.clone(),
                        budget_reserved: first.required_budget,
                    },
                    TaskAssignment {
                        task_id: second.id.clone(),
                        role: second.role.clone(),
                        lane: second.lane.clone(),
                        budget_reserved: second.required_budget,
                    },
                ],
                ..TaskDispatchPlan::default()
            };
            let first_result = AgentResult::accepted(
                &first,
                "first completed",
                vec![
                    AgentMessage::new(
                        "window-1-note",
                        AgentRole::Reviewer,
                        AgentMessageKind::Finding,
                        "contract",
                        "ownership boundary verified",
                    )
                    .with_evidence("window-1"),
                ],
                AgentBudget::new(1, 1, 1),
            );
            let second_result = AgentResult::accepted(
                &second,
                "second completed",
                vec![
                    AgentMessage::new(
                        "window-2-note",
                        AgentRole::Reviewer,
                        AgentMessageKind::Finding,
                        "contract",
                        "ownership boundary verified",
                    )
                    .with_evidence("window-2"),
                ],
                AgentBudget::new(1, 1, 1),
            );
            let mut ledger = AgentRunLedger::new(dispatch);
            if reverse_arrival {
                ledger.record_result(second_result);
                ledger.record_result(first_result);
            } else {
                ledger.record_result(first_result);
                ledger.record_result(second_result);
            }
            ledger.report(None)
        }

        let forward = build_report(false);
        let reversed = build_report(true);

        assert_eq!(forward.aggregation, reversed.aggregation);
        assert_eq!(forward.summary(), reversed.summary());
        assert_eq!(forward.aggregation.input_count, 2);
        assert_eq!(forward.aggregation.unique_count, 1);
        assert_eq!(forward.aggregation.duplicate_groups, 1);
        assert_eq!(forward.aggregation.messages[0].duplicate_count, 2);
        assert_eq!(
            forward.aggregation.messages[0].source_ids,
            vec!["window-1-note".to_owned(), "window-2-note".to_owned()]
        );
        assert_eq!(
            forward.aggregation.messages[0].message.evidence,
            vec!["window-1".to_owned(), "window-2".to_owned()]
        );
    }

    #[test]
    fn resolved_conflict_allows_memory_note_after_reflection() {
        let coder = AgentTask::new(
            "coder-task",
            AgentRole::Coder,
            "patch",
            AgentBudget::new(8, 1, 1),
        );
        let reviewer = AgentTask::new(
            "reviewer-task",
            AgentRole::Reviewer,
            "review",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![
                TaskAssignment {
                    task_id: coder.id.clone(),
                    role: coder.role.clone(),
                    lane: coder.lane.clone(),
                    budget_reserved: coder.required_budget,
                },
                TaskAssignment {
                    task_id: reviewer.id.clone(),
                    role: reviewer.role.clone(),
                    lane: reviewer.lane.clone(),
                    budget_reserved: reviewer.required_budget,
                },
            ],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);
        ledger.record_result(AgentResult::accepted(
            &coder,
            "coder approved",
            vec![AgentMessage::new(
                "m-coder",
                AgentRole::Coder,
                AgentMessageKind::Decision,
                "memory",
                "approve memory note",
            )],
            AgentBudget::new(2, 1, 1),
        ));
        ledger.record_result(AgentResult::accepted(
            &reviewer,
            "reviewer blocked",
            vec![AgentMessage::new(
                "m-reviewer",
                AgentRole::Reviewer,
                AgentMessageKind::Risk,
                "memory",
                "block memory note until validation passes",
            )],
            AgentBudget::new(2, 1, 1),
        ));
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
            .submit(
                ReflectionStage::MemoryNote,
                "remember validated memory gate",
            )
            .unwrap();
        let resolutions = ConflictResolutionBook::new().with_resolution(
            crate::conflict::ConflictResolution::new(
                "memory",
                vec!["m-coder".to_owned(), "m-reviewer".to_owned()],
                AgentRole::Planner,
                "validation passed and reviewer accepted the memory note",
            ),
        );

        let report = ledger.report_with_resolutions(Some(&reflection), &resolutions);

        assert!(!report.conflicts.has_unresolved_conflicts());
        assert!(
            report
                .side_effects
                .iter()
                .any(|gate| gate.kind == SideEffectKind::MemoryNote && gate.allowed)
        );

        let summary = report.summary();
        let gate = report.gate();

        assert_eq!(summary.unresolved_conflicts, 0);
        assert_eq!(summary.blocked_side_effects, 0);
        assert!(summary.memory_note_allowed);
        assert!(summary.adaptive_state_allowed);
        assert!(summary.external_call_allowed);
        assert!(summary.all_side_effects_allowed);
        assert!(gate.can_promote_memory_note);
        assert!(gate.can_write_adaptive_state);
        assert!(gate.can_dispatch_external_call);
        assert!(!gate.requires_repair_first);
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_run_gate_memory_note=true" })
        );
    }

    #[test]
    fn budget_audit_reports_result_spending_over_reserved_budget() {
        let task = AgentTask::new(
            "coder-task",
            AgentRole::Coder,
            "patch",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![TaskAssignment {
                task_id: task.id.clone(),
                role: task.role.clone(),
                lane: task.lane.clone(),
                budget_reserved: task.required_budget,
            }],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);
        ledger.record_result(AgentResult::accepted(
            &task,
            "overspent",
            Vec::new(),
            AgentBudget::new(9, 1, 1),
        ));

        let audit = ledger.budget_audit();

        assert_eq!(audit.overspend_count(), 1);
        assert_eq!(audit.overspends[0].task_id, "coder-task");
        assert_eq!(audit.overspends[0].reserved, AgentBudget::new(8, 1, 1));
        assert_eq!(audit.overspends[0].spent, AgentBudget::new(9, 1, 1));

        let summary = audit.summary();

        assert_eq!(summary.overspends, 1);
        assert_eq!(summary.overspent_tokens, 1);
        assert_eq!(summary.overspent_steps, 0);
        assert_eq!(summary.overspent_messages, 0);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_run_budget_audit_summary_overspent_tokens=1" })
        );
    }

    #[test]
    fn run_gate_repairs_budget_overspend_even_when_side_effects_are_open() {
        let task = AgentTask::new(
            "coder-task",
            AgentRole::Coder,
            "patch",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = TaskDispatchPlan {
            assignments: vec![TaskAssignment {
                task_id: task.id.clone(),
                role: task.role.clone(),
                lane: task.lane.clone(),
                budget_reserved: task.required_budget,
            }],
            ..TaskDispatchPlan::default()
        };
        let mut ledger = AgentRunLedger::new(dispatch);
        ledger.record_result(AgentResult::accepted(
            &task,
            "overspent clean result",
            vec![AgentMessage::new(
                "m-coder",
                AgentRole::Coder,
                AgentMessageKind::Finding,
                "patch",
                "safe patch",
            )],
            AgentBudget::new(9, 1, 1),
        ));
        let mut reflection = ReflectionLoop::new();
        reflection
            .submit(ReflectionStage::Draft, "draft conclusion")
            .unwrap();
        reflection
            .submit(ReflectionStage::Critique, "budget risk")
            .unwrap();
        reflection
            .submit(ReflectionStage::Revision, "record overspend")
            .unwrap();
        reflection
            .submit(ReflectionStage::MemoryNote, "remember overspend gate")
            .unwrap();

        let report = ledger.report(Some(&reflection));
        let summary = report.summary();
        let gate = report.gate();

        assert_eq!(summary.unresolved_conflicts, 0);
        assert_eq!(summary.budget_overspends, 1);
        assert_eq!(summary.blocked_side_effects, 0);
        assert!(summary.memory_note_allowed);
        assert!(gate.requires_repair_first);
        assert!(!gate.can_promote_memory_note);
        assert!(!gate.can_write_adaptive_state);
        assert!(!gate.can_dispatch_external_call);
        assert!(
            gate.reasons
                .iter()
                .any(|reason| reason == "budget_overspends=1")
        );
    }

    #[test]
    fn telomere_state_blocks_side_effects_when_dispatch_budget_depleted() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Coder, AgentBudget::new(8, 1, 1)),
        );
        let first = AgentTask::new(
            "coder-first",
            AgentRole::Coder,
            "consume coder lane",
            AgentBudget::new(8, 1, 1),
        )
        .with_priority(9);
        let second = AgentTask::new(
            "coder-second",
            AgentRole::Coder,
            "follow-up after exhaustion",
            AgentBudget::new(1, 1, 1),
        );
        let plan = planner.plan_with_policy(vec![second, first], &BudgetPolicy::strict());
        let summary = plan.summary();

        let state = AgentTelomereState::from_dispatch_summary("run/telomere", &summary, 0);
        let handoff = AgentApoptosisHandoff::from_telomere_state(&state);

        assert_eq!(state.remaining_tokens, 0);
        assert_eq!(state.remaining_steps, 0);
        assert_eq!(state.remaining_messages, 0);
        assert!(state.senescent);
        assert!(!state.apoptosis_required);
        assert!(!state.new_external_call_allowed);
        assert!(!state.new_file_write_allowed);
        assert!(!state.new_memory_write_allowed);
        assert!(!state.new_adaptive_state_write_allowed);
        assert!(!state.memory_promotion_allowed);
        assert!(!state.genome_mutation_allowed);
        assert!(!state.raw_payload_present);
        assert!(!state.preview_side_effect_allowed);
        assert!(
            state
                .takeover_packet_digest
                .starts_with("redaction-digest:")
        );
        assert!(
            state
                .depletion_reason_codes
                .contains(&"dispatch_rejections".to_owned())
        );
        assert!(
            state
                .depletion_reason_codes
                .contains(&"remaining_zero_budget_roles".to_owned())
        );
        assert_eq!(handoff.next_owner_hint, "summary_handoff");
        assert!(!handoff.new_external_call_allowed);
        assert!(!handoff.new_file_write_allowed);
        assert!(!handoff.new_memory_write_allowed);
        assert!(!handoff.new_adaptive_state_write_allowed);
        assert!(!handoff.memory_promotion_allowed);
        assert!(!handoff.genome_mutation_allowed);
    }

    #[test]
    fn telomere_state_requires_apoptosis_after_repair_streak_threshold() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(20, 2, 2)),
        );
        let task = AgentTask::new(
            "planner",
            AgentRole::Planner,
            "continue only if repair streak clears",
            AgentBudget::new(4, 1, 1),
        );
        let plan = planner.plan_with_policy(vec![task], &BudgetPolicy::strict());
        let summary = plan.summary();

        let state = AgentTelomereState::from_dispatch_summary("run/apoptosis", &summary, 2);
        let handoff = AgentApoptosisHandoff::from_telomere_state(&state);
        let mut permissive_planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(20, 2, 2)),
        );
        let permissive_plan = permissive_planner.plan_with_policy(
            vec![AgentTask::new(
                "planner-permissive",
                AgentRole::Planner,
                "later permissive policy must not bypass apoptosis",
                AgentBudget::new(1, 1, 1),
            )],
            &BudgetPolicy::permissive(),
        );

        assert!(state.senescent);
        assert!(state.apoptosis_required);
        assert_eq!(state.repeated_repair_streak_count, 2);
        assert!(state.loop_risk_signal_count >= 2);
        assert!(!state.new_external_call_allowed);
        assert!(!state.new_file_write_allowed);
        assert!(!state.new_memory_write_allowed);
        assert!(!state.new_adaptive_state_write_allowed);
        assert!(!state.memory_promotion_allowed);
        assert!(!state.genome_mutation_allowed);
        assert!(
            state
                .depletion_reason_codes
                .contains(&"repeated_repair_pressure".to_owned())
        );
        assert!(handoff.apoptosis_required);
        assert_eq!(handoff.next_owner_hint, "scheduler");
        assert!(
            handoff
                .rollback_anchor_digest
                .starts_with("redaction-digest:")
        );
        assert!(!handoff.raw_payload_present);
        assert!(permissive_plan.gate().can_promote_side_effects);
        assert!(!handoff.new_external_call_allowed);
        assert!(!handoff.new_file_write_allowed);
        assert!(!handoff.new_memory_write_allowed);
        assert!(!handoff.new_adaptive_state_write_allowed);
        assert!(!handoff.memory_promotion_allowed);
        assert!(!handoff.genome_mutation_allowed);
    }

    #[test]
    fn telomere_state_uses_run_health_pressure_to_block_side_effects() {
        let summary = TaskDispatchPlanSummary {
            assignments: 1,
            rejections: 0,
            remaining_roles: 1,
            remaining_tokens: 10,
            remaining_steps: 1,
            remaining_messages: 1,
            remaining_zero_budget_roles: 0,
            remaining_partially_depleted_roles: 0,
            remaining_token_depleted_roles: 0,
            remaining_step_depleted_roles: 0,
            remaining_message_depleted_roles: 0,
            assigned_rate: 1.0,
            rejected_rate: 0.0,
            telemetry: Vec::new(),
        };
        let run_summary = AgentRunReportSummary {
            input_messages: 2,
            unique_messages: 2,
            duplicate_groups: 0,
            unresolved_conflicts: 0,
            budget_overspends: 1,
            side_effects: 2,
            allowed_side_effects: 1,
            blocked_side_effects: 1,
            memory_note_allowed: true,
            adaptive_state_allowed: false,
            external_call_allowed: true,
            all_side_effects_allowed: false,
            telemetry: Vec::new(),
        };

        let state = AgentTelomereState::from_dispatch_summary("run/health", &summary, 0)
            .with_run_report_summary(&run_summary);

        assert!(state.senescent);
        assert!(!state.apoptosis_required);
        assert_eq!(state.loop_risk_signal_count, 2);
        assert!(
            state
                .depletion_reason_codes
                .contains(&"run_budget_overspends".to_owned())
        );
        assert!(
            state
                .depletion_reason_codes
                .contains(&"run_blocked_side_effects".to_owned())
        );
        assert!(!state.new_external_call_allowed);
        assert!(!state.new_adaptive_state_write_allowed);
        assert!(!state.memory_promotion_allowed);
        assert!(!state.genome_mutation_allowed);
    }
}
