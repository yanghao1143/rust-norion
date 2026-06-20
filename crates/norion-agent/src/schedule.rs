use std::collections::BTreeSet;

use crate::budget::AgentBudget;
use crate::task::{AgentRole, AgentTask, AgentTaskQueue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionWave {
    pub wave: usize,
    pub task_ids: Vec<String>,
    pub parallel_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursiveAgentSchedule {
    pub max_parallel_tasks: usize,
    pub waves: Vec<AgentExecutionWave>,
    pub completed_task_ids: Vec<String>,
    pub blocked_task_ids: Vec<String>,
}

impl RecursiveAgentSchedule {
    pub fn wave_count(&self) -> usize {
        self.waves.len()
    }

    pub fn completed_count(&self) -> usize {
        self.completed_task_ids.len()
    }

    pub fn blocked_count(&self) -> usize {
        self.blocked_task_ids.len()
    }

    pub fn has_blocked_tasks(&self) -> bool {
        !self.blocked_task_ids.is_empty()
    }

    pub fn summary(&self) -> RecursiveAgentScheduleSummary {
        RecursiveAgentScheduleSummary::from_schedule(self)
    }

    pub fn gate(&self) -> RecursiveAgentScheduleGateDecision {
        RecursiveAgentScheduleGateDecision::from_schedule(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveAgentScheduleSummary {
    pub max_parallel_tasks: usize,
    pub waves: usize,
    pub completed_tasks: usize,
    pub blocked_tasks: usize,
    pub max_wave_parallelism: usize,
    pub average_wave_parallelism: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecursiveAgentScheduleHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl RecursiveAgentScheduleHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecursiveAgentScheduleSummaryHistory {
    summaries: Vec<RecursiveAgentScheduleSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveAgentScheduleDashboard {
    pub total_records: usize,
    pub schedulable_records: usize,
    pub blocked_records: usize,
    pub empty_wave_records: usize,
    pub waves: usize,
    pub completed_tasks: usize,
    pub blocked_tasks: usize,
    pub max_wave_parallelism: usize,
    pub average_wave_parallelism: f32,
    pub schedule_rate: f32,
    pub blocked_record_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecursiveAgentScheduleHealthPolicy {
    pub minimum_schedule_rate: f32,
    pub maximum_blocked_records: usize,
    pub maximum_blocked_tasks: usize,
    pub maximum_empty_wave_records: usize,
}

impl Default for RecursiveAgentScheduleHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_schedule_rate: 0.67,
            maximum_blocked_records: 0,
            maximum_blocked_tasks: 0,
            maximum_empty_wave_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveAgentScheduleHealth {
    pub status: RecursiveAgentScheduleHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: RecursiveAgentScheduleDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveAgentScheduleSummaryHistoryRecord {
    pub history: RecursiveAgentScheduleSummaryHistory,
    pub appended_summary: RecursiveAgentScheduleSummary,
    pub dashboard: RecursiveAgentScheduleDashboard,
    pub health: RecursiveAgentScheduleHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RecursiveAgentScheduleSummaryHistoryRecorder;

impl RecursiveAgentScheduleSummary {
    pub fn from_schedule(schedule: &RecursiveAgentSchedule) -> Self {
        let waves = schedule.wave_count();
        let completed_tasks = schedule.completed_count();
        let blocked_tasks = schedule.blocked_count();
        let max_wave_parallelism = schedule
            .waves
            .iter()
            .map(|wave| wave.parallel_count)
            .max()
            .unwrap_or(0);
        let average_wave_parallelism = rate(completed_tasks, waves);
        let telemetry = recursive_agent_schedule_summary_telemetry(
            schedule.max_parallel_tasks,
            waves,
            completed_tasks,
            blocked_tasks,
            max_wave_parallelism,
            average_wave_parallelism,
        );

        Self {
            max_parallel_tasks: schedule.max_parallel_tasks,
            waves,
            completed_tasks,
            blocked_tasks,
            max_wave_parallelism,
            average_wave_parallelism,
            telemetry,
        }
    }
}

impl RecursiveAgentScheduleSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<RecursiveAgentScheduleSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: RecursiveAgentScheduleSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&RecursiveAgentScheduleSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[RecursiveAgentScheduleSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> RecursiveAgentScheduleDashboard {
        RecursiveAgentScheduleDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: RecursiveAgentScheduleHealthPolicy,
    ) -> RecursiveAgentScheduleHealth {
        self.dashboard().health(policy)
    }
}

impl RecursiveAgentScheduleDashboard {
    pub fn from_summaries(summaries: &[RecursiveAgentScheduleSummary]) -> Self {
        let total_records = summaries.len();
        let schedulable_records = summaries
            .iter()
            .filter(|summary| summary.waves > 0 && summary.blocked_tasks == 0)
            .count();
        let blocked_records = summaries
            .iter()
            .filter(|summary| summary.blocked_tasks > 0)
            .count();
        let empty_wave_records = summaries
            .iter()
            .filter(|summary| summary.waves == 0)
            .count();
        let waves = summaries.iter().map(|summary| summary.waves).sum::<usize>();
        let completed_tasks = summaries
            .iter()
            .map(|summary| summary.completed_tasks)
            .sum::<usize>();
        let blocked_tasks = summaries
            .iter()
            .map(|summary| summary.blocked_tasks)
            .sum::<usize>();
        let max_wave_parallelism = summaries
            .iter()
            .map(|summary| summary.max_wave_parallelism)
            .max()
            .unwrap_or(0);
        let average_wave_parallelism = rate(completed_tasks, waves);
        let schedule_rate = rate(schedulable_records, total_records);
        let blocked_record_rate = rate(blocked_records, total_records);
        let telemetry = recursive_agent_schedule_dashboard_telemetry(
            total_records,
            schedulable_records,
            blocked_records,
            empty_wave_records,
            waves,
            completed_tasks,
            blocked_tasks,
            max_wave_parallelism,
            average_wave_parallelism,
            schedule_rate,
            blocked_record_rate,
        );

        Self {
            total_records,
            schedulable_records,
            blocked_records,
            empty_wave_records,
            waves,
            completed_tasks,
            blocked_tasks,
            max_wave_parallelism,
            average_wave_parallelism,
            schedule_rate,
            blocked_record_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: RecursiveAgentScheduleHealthPolicy,
    ) -> RecursiveAgentScheduleHealth {
        RecursiveAgentScheduleHealth::from_dashboard(self.clone(), policy)
    }
}

impl RecursiveAgentScheduleHealth {
    pub fn from_dashboard(
        dashboard: RecursiveAgentScheduleDashboard,
        policy: RecursiveAgentScheduleHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("recursive_schedule_history_empty".to_owned());
        } else if dashboard.schedule_rate < policy.minimum_schedule_rate {
            watch_reasons.push(format!(
                "recursive_schedule_rate={:.3}<{}",
                dashboard.schedule_rate, policy.minimum_schedule_rate
            ));
        }

        if dashboard.blocked_records > policy.maximum_blocked_records {
            repair_reasons.push(format!(
                "recursive_schedule_blocked_records={}>{}",
                dashboard.blocked_records, policy.maximum_blocked_records
            ));
        }

        if dashboard.blocked_tasks > policy.maximum_blocked_tasks {
            repair_reasons.push(format!(
                "recursive_schedule_blocked_tasks={}>{}",
                dashboard.blocked_tasks, policy.maximum_blocked_tasks
            ));
        }

        if dashboard.empty_wave_records > policy.maximum_empty_wave_records {
            repair_reasons.push(format!(
                "recursive_schedule_empty_wave_records={}>{}",
                dashboard.empty_wave_records, policy.maximum_empty_wave_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (RecursiveAgentScheduleHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (RecursiveAgentScheduleHealthStatus::Watch, watch_reasons)
        } else {
            (RecursiveAgentScheduleHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == RecursiveAgentScheduleHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != RecursiveAgentScheduleHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == RecursiveAgentScheduleHealthStatus::Repair
    }
}

impl RecursiveAgentScheduleSummaryHistoryRecord {
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

impl RecursiveAgentScheduleSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: RecursiveAgentScheduleSummaryHistory,
        summary: RecursiveAgentScheduleSummary,
        policy: RecursiveAgentScheduleHealthPolicy,
    ) -> RecursiveAgentScheduleSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = recursive_agent_schedule_history_record_telemetry(&dashboard, &health);

        RecursiveAgentScheduleSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_schedule_with_health(
        &self,
        history: RecursiveAgentScheduleSummaryHistory,
        schedule: &RecursiveAgentSchedule,
        policy: RecursiveAgentScheduleHealthPolicy,
    ) -> RecursiveAgentScheduleSummaryHistoryRecord {
        self.record_summary_with_health(history, schedule.summary(), policy)
    }

    pub fn record_schedule_with_health_gate(
        &self,
        history: RecursiveAgentScheduleSummaryHistory,
        schedule: &RecursiveAgentSchedule,
        policy: RecursiveAgentScheduleHealthPolicy,
    ) -> RecursiveAgentScheduleHistoryGateRecord {
        let health_record = self.record_schedule_with_health(history, schedule, policy);
        let gate_decision = RecursiveAgentScheduleHistoryGate::new().gate(schedule, &health_record);
        let telemetry =
            recursive_agent_schedule_history_gate_record_telemetry(&health_record, &gate_decision);

        RecursiveAgentScheduleHistoryGateRecord {
            health_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveAgentScheduleGateDecision {
    pub summary: RecursiveAgentScheduleSummary,
    pub can_dispatch_waves: bool,
    pub requires_repair_first: bool,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl RecursiveAgentScheduleGateDecision {
    pub fn from_schedule(schedule: &RecursiveAgentSchedule) -> Self {
        let summary = schedule.summary();
        let mut reasons = Vec::new();

        if summary.waves == 0 {
            reasons.push("schedule_empty_waves".to_owned());
        }
        if summary.blocked_tasks > 0 {
            reasons.push(format!("schedule_blocked_tasks={}", summary.blocked_tasks));
        }

        let can_dispatch_waves = summary.waves > 0 && summary.blocked_tasks == 0;
        let requires_repair_first = summary.blocked_tasks > 0 || summary.waves == 0;
        let telemetry = recursive_agent_schedule_gate_telemetry(
            can_dispatch_waves,
            requires_repair_first,
            reasons.len(),
            &summary,
        );

        Self {
            summary,
            can_dispatch_waves,
            requires_repair_first,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveAgentScheduleHistoryGateDecision {
    pub schedule_gate: RecursiveAgentScheduleGateDecision,
    pub schedule_health: RecursiveAgentScheduleHealth,
    pub can_dispatch_waves: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl RecursiveAgentScheduleHistoryGateDecision {
    pub fn is_dispatchable(&self) -> bool {
        self.can_dispatch_waves && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveAgentScheduleHistoryGateRecord {
    pub health_record: RecursiveAgentScheduleSummaryHistoryRecord,
    pub gate_decision: RecursiveAgentScheduleHistoryGateDecision,
    pub telemetry: Vec<String>,
}

impl RecursiveAgentScheduleHistoryGateRecord {
    pub fn records(&self) -> usize {
        self.health_record.records()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn can_dispatch_waves(&self) -> bool {
        self.gate_decision.can_dispatch_waves
    }
}

#[derive(Debug, Clone, Default)]
pub struct RecursiveAgentScheduleHistoryGate;

impl RecursiveAgentScheduleHistoryGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        schedule: &RecursiveAgentSchedule,
        history_record: &RecursiveAgentScheduleSummaryHistoryRecord,
    ) -> RecursiveAgentScheduleHistoryGateDecision {
        let schedule_gate = schedule.gate();
        let schedule_health = history_record.health.clone();
        let mut reasons = schedule_gate.reasons.clone();
        extend_ordered_unique(
            &mut reasons,
            schedule_health
                .reasons
                .iter()
                .map(|reason| format!("recursive_schedule_history:{reason}"))
                .collect::<Vec<_>>(),
        );
        let requires_repair_first =
            schedule_gate.requires_repair_first || schedule_health.requires_repair_first();
        let can_dispatch_waves =
            schedule_gate.can_dispatch_waves && schedule_health.allows_service_advance();
        let repair_tasks =
            recursive_agent_schedule_history_gate_repair_tasks(requires_repair_first, &reasons);
        let telemetry = recursive_agent_schedule_history_gate_telemetry(
            can_dispatch_waves,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            &schedule_gate.summary,
            schedule_health.status,
        );

        RecursiveAgentScheduleHistoryGateDecision {
            schedule_gate,
            schedule_health,
            can_dispatch_waves,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursiveAgentScheduler {
    max_parallel_tasks: usize,
}

impl Default for RecursiveAgentScheduler {
    fn default() -> Self {
        Self {
            max_parallel_tasks: 1,
        }
    }
}

impl RecursiveAgentScheduler {
    pub fn new(max_parallel_tasks: usize) -> Self {
        Self {
            max_parallel_tasks: max_parallel_tasks.max(1),
        }
    }

    pub fn max_parallel_tasks(&self) -> usize {
        self.max_parallel_tasks
    }

    pub fn plan(&self, tasks: Vec<AgentTask>) -> RecursiveAgentSchedule {
        let mut queue = AgentTaskQueue::from_tasks(tasks);
        let mut completed = BTreeSet::new();
        let mut waves = Vec::new();

        while !queue.is_empty() {
            let ready_ids = queue
                .ready_tasks(&completed)
                .into_iter()
                .take(self.max_parallel_tasks)
                .map(|task| task.id.clone())
                .collect::<Vec<_>>();
            if ready_ids.is_empty() {
                break;
            }

            let mut wave_task_ids = Vec::new();
            for task_id in ready_ids {
                if queue.remove(&task_id).is_some() {
                    completed.insert(task_id.clone());
                    wave_task_ids.push(task_id);
                }
            }
            waves.push(AgentExecutionWave {
                wave: waves.len(),
                parallel_count: wave_task_ids.len(),
                task_ids: wave_task_ids,
            });
        }

        let completed_task_ids = completed.into_iter().collect::<Vec<_>>();
        let blocked_task_ids = queue.task_ids();

        RecursiveAgentSchedule {
            max_parallel_tasks: self.max_parallel_tasks,
            waves,
            completed_task_ids,
            blocked_task_ids,
        }
    }

    pub fn plan_repair_first(
        &self,
        repair_tasks: Vec<AgentTask>,
        tasks: Vec<AgentTask>,
    ) -> RecursiveAgentSchedule {
        if repair_tasks.is_empty() {
            return self.plan(tasks);
        }

        self.plan(
            AgentTaskQueue::from_tasks(tasks)
                .with_repair_first(&repair_tasks)
                .tasks(),
        )
    }
}

fn recursive_agent_schedule_summary_telemetry(
    max_parallel_tasks: usize,
    waves: usize,
    completed_tasks: usize,
    blocked_tasks: usize,
    max_wave_parallelism: usize,
    average_wave_parallelism: f32,
) -> Vec<String> {
    vec![
        "agent_recursive_schedule_summary=true".to_owned(),
        format!("agent_recursive_schedule_summary_max_parallel_tasks={max_parallel_tasks}"),
        format!("agent_recursive_schedule_summary_waves={waves}"),
        format!("agent_recursive_schedule_summary_completed_tasks={completed_tasks}"),
        format!("agent_recursive_schedule_summary_blocked_tasks={blocked_tasks}"),
        format!("agent_recursive_schedule_summary_max_wave_parallelism={max_wave_parallelism}"),
        format!(
            "agent_recursive_schedule_summary_average_wave_parallelism={average_wave_parallelism:.3}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn recursive_agent_schedule_dashboard_telemetry(
    total_records: usize,
    schedulable_records: usize,
    blocked_records: usize,
    empty_wave_records: usize,
    waves: usize,
    completed_tasks: usize,
    blocked_tasks: usize,
    max_wave_parallelism: usize,
    average_wave_parallelism: f32,
    schedule_rate: f32,
    blocked_record_rate: f32,
) -> Vec<String> {
    vec![
        "agent_recursive_schedule_dashboard=true".to_owned(),
        format!("agent_recursive_schedule_dashboard_records={total_records}"),
        format!("agent_recursive_schedule_dashboard_schedulable={schedulable_records}"),
        format!("agent_recursive_schedule_dashboard_blocked_records={blocked_records}"),
        format!("agent_recursive_schedule_dashboard_empty_wave_records={empty_wave_records}"),
        format!("agent_recursive_schedule_dashboard_waves={waves}"),
        format!("agent_recursive_schedule_dashboard_completed_tasks={completed_tasks}"),
        format!("agent_recursive_schedule_dashboard_blocked_tasks={blocked_tasks}"),
        format!("agent_recursive_schedule_dashboard_max_wave_parallelism={max_wave_parallelism}"),
        format!(
            "agent_recursive_schedule_dashboard_average_wave_parallelism={average_wave_parallelism:.3}"
        ),
        format!("agent_recursive_schedule_dashboard_schedule_rate={schedule_rate:.3}"),
        format!("agent_recursive_schedule_dashboard_blocked_record_rate={blocked_record_rate:.3}"),
    ]
}

fn recursive_agent_schedule_history_record_telemetry(
    dashboard: &RecursiveAgentScheduleDashboard,
    health: &RecursiveAgentScheduleHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_recursive_schedule_history_record=true".to_owned(),
        format!(
            "agent_recursive_schedule_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_recursive_schedule_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_recursive_schedule_history_record_schedule_rate={:.3}",
            dashboard.schedule_rate
        ),
        format!(
            "agent_recursive_schedule_history_record_blocked_tasks={}",
            dashboard.blocked_tasks
        ),
        format!(
            "agent_recursive_schedule_history_record_empty_wave_records={}",
            dashboard.empty_wave_records
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_recursive_schedule_history_record_reason={reason}")),
    );
    telemetry
}

fn recursive_agent_schedule_gate_telemetry(
    can_dispatch_waves: bool,
    requires_repair_first: bool,
    reasons: usize,
    summary: &RecursiveAgentScheduleSummary,
) -> Vec<String> {
    vec![
        "agent_recursive_schedule_gate=true".to_owned(),
        format!("agent_recursive_schedule_gate_can_dispatch_waves={can_dispatch_waves}"),
        format!("agent_recursive_schedule_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_recursive_schedule_gate_reasons={reasons}"),
        format!("agent_recursive_schedule_gate_waves={}", summary.waves),
        format!(
            "agent_recursive_schedule_gate_blocked_tasks={}",
            summary.blocked_tasks
        ),
    ]
}

fn recursive_agent_schedule_history_gate_repair_tasks(
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
                format!("recursive-agent-schedule-repair-{index}"),
                AgentRole::Planner,
                format!("repair recursive agent schedule: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("recursive-agent-schedule-repair")
            .with_priority(1)
        })
        .collect()
}

fn recursive_agent_schedule_history_gate_telemetry(
    can_dispatch_waves: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    summary: &RecursiveAgentScheduleSummary,
    health_status: RecursiveAgentScheduleHealthStatus,
) -> Vec<String> {
    vec![
        "agent_recursive_schedule_history_gate=true".to_owned(),
        format!(
            "agent_recursive_schedule_history_gate_health={}",
            health_status.as_str()
        ),
        format!("agent_recursive_schedule_history_gate_can_dispatch_waves={can_dispatch_waves}"),
        format!(
            "agent_recursive_schedule_history_gate_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_recursive_schedule_history_gate_repair_tasks={repair_tasks}"),
        format!("agent_recursive_schedule_history_gate_reasons={reasons}"),
        format!(
            "agent_recursive_schedule_history_gate_waves={}",
            summary.waves
        ),
        format!(
            "agent_recursive_schedule_history_gate_blocked_tasks={}",
            summary.blocked_tasks
        ),
    ]
}

fn recursive_agent_schedule_history_gate_record_telemetry(
    health_record: &RecursiveAgentScheduleSummaryHistoryRecord,
    gate_decision: &RecursiveAgentScheduleHistoryGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_recursive_schedule_history_gate_record=true".to_owned(),
        format!(
            "agent_recursive_schedule_history_gate_record_health={}",
            health_record.health.status.as_str()
        ),
        format!(
            "agent_recursive_schedule_history_gate_record_records={}",
            health_record.records()
        ),
        format!(
            "agent_recursive_schedule_history_gate_record_can_dispatch_waves={}",
            gate_decision.can_dispatch_waves
        ),
        format!(
            "agent_recursive_schedule_history_gate_record_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_recursive_schedule_history_gate_record_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
    ];
    telemetry.extend(gate_decision.telemetry.iter().cloned());
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
    use crate::aggregate::{
        AggregationConflictReviewer, AggregationHealthPolicy, AggregationSummaryHistory,
    };
    use crate::budget::AgentBudget;
    use crate::collaboration::{AgentWindowOwnership, AgentWindowOwnershipReviewer};
    use crate::conflict::{
        AgentConflict, ConflictReport, ConflictReportHealthPolicy, ConflictReportSummaryHistory,
        ConflictReportSummaryHistoryRecorder,
    };
    use crate::message::{AgentMessage, AgentMessageKind};
    use crate::task::AgentRole;

    #[test]
    fn scheduler_builds_stable_dependency_waves() {
        let planner = AgentTask::new(
            "planner",
            AgentRole::Planner,
            "split work",
            AgentBudget::new(4, 1, 1),
        );
        let coder = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "write patch",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("planner");
        let reviewer = AgentTask::new(
            "reviewer",
            AgentRole::Reviewer,
            "review patch",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("planner");
        let memory = AgentTask::new(
            "memory",
            AgentRole::MemoryCurator,
            "capture lesson",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("reviewer");

        let schedule = RecursiveAgentScheduler::new(2).plan(vec![memory, reviewer, coder, planner]);

        assert_eq!(schedule.wave_count(), 3);
        assert_eq!(schedule.waves[0].task_ids, vec!["planner"]);
        assert_eq!(schedule.waves[1].task_ids, vec!["coder", "reviewer"]);
        assert_eq!(schedule.waves[2].task_ids, vec!["memory"]);
        assert_eq!(schedule.blocked_count(), 0);

        let summary = schedule.summary();
        let gate = schedule.gate();

        assert_eq!(summary.waves, 3);
        assert_eq!(summary.completed_tasks, 4);
        assert_eq!(summary.blocked_tasks, 0);
        assert_eq!(summary.max_wave_parallelism, 2);
        assert_eq!(summary.average_wave_parallelism, 4.0 / 3.0);
        assert!(gate.can_dispatch_waves);
        assert!(!gate.requires_repair_first);
        assert!(gate.reasons.is_empty());
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_recursive_schedule_summary_max_wave_parallelism=2" })
        );
    }

    #[test]
    fn scheduler_respects_priority_when_parallel_slots_are_limited() {
        let low = AgentTask::new(
            "window-low",
            AgentRole::Custom("window-low".to_owned()),
            "low priority follow-up",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(2);
        let high = AgentTask::new(
            "window-high",
            AgentRole::Custom("window-high".to_owned()),
            "high priority repair",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(9);
        let tie_a = AgentTask::new(
            "window-a",
            AgentRole::Custom("window-a".to_owned()),
            "same priority a",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(5);
        let tie_b = AgentTask::new(
            "window-b",
            AgentRole::Custom("window-b".to_owned()),
            "same priority b",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(5);

        let schedule = RecursiveAgentScheduler::new(2).plan(vec![low, tie_b, high, tie_a]);

        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.waves[0].task_ids, vec!["window-high", "window-a"]);
        assert_eq!(schedule.waves[1].task_ids, vec!["window-b", "window-low"]);
        assert_eq!(
            schedule.completed_task_ids,
            vec![
                "window-a".to_owned(),
                "window-b".to_owned(),
                "window-high".to_owned(),
                "window-low".to_owned(),
            ]
        );
        assert!(schedule.blocked_task_ids.is_empty());

        let summary = schedule.summary();
        let gate = schedule.gate();

        assert_eq!(summary.max_wave_parallelism, 2);
        assert_eq!(summary.average_wave_parallelism, 2.0);
        assert!(gate.can_dispatch_waves);
        assert!(!gate.requires_repair_first);
    }

    #[test]
    fn scheduler_places_unresolved_conflict_repairs_before_normal_tasks() {
        let review = AggregationConflictReviewer::new().review_messages(
            vec![
                AgentMessage::new(
                    "coder",
                    AgentRole::Coder,
                    AgentMessageKind::Decision,
                    "memory",
                    "approve memory note promotion and proceed",
                ),
                AgentMessage::new(
                    "reviewer",
                    AgentRole::Reviewer,
                    AgentMessageKind::Risk,
                    "memory",
                    "block memory note promotion until conflict is resolved",
                ),
            ],
            AggregationSummaryHistory::new(),
            AggregationHealthPolicy::default(),
            ConflictReportSummaryHistory::new(),
            ConflictReportHealthPolicy::default(),
        );
        let repair_task_ids = review.repair_task_ids();
        let memory_note = AgentTask::new(
            "memory-note",
            AgentRole::MemoryCurator,
            "write memory note after conflict repair",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(10);
        let side_effect = AgentTask::new(
            "side-effect-admission",
            AgentRole::Reviewer,
            "admit side effect after conflict repair",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(10);
        let next_task = AgentTask::new(
            "next-task-promotion",
            AgentRole::Planner,
            "promote next task after conflict repair",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(10);

        let schedule = RecursiveAgentScheduler::new(10).plan_repair_first(
            review.repair_tasks.clone(),
            vec![memory_note, side_effect, next_task],
        );

        assert!(review.requires_repair_first);
        assert!(!review.can_promote_side_effects);
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
        assert_eq!(schedule.blocked_task_ids, Vec::<String>::new());
        assert!(schedule.gate().can_dispatch_waves);
    }

    #[test]
    fn scheduler_places_conflict_history_gate_repairs_before_business_tasks() {
        let report = ConflictReport {
            messages: vec![
                AgentMessage::new(
                    "coder",
                    AgentRole::Coder,
                    AgentMessageKind::Decision,
                    "memory",
                    "promote the memory note",
                ),
                AgentMessage::new(
                    "reviewer",
                    AgentRole::Reviewer,
                    AgentMessageKind::Risk,
                    "memory",
                    "block the memory note until ownership is resolved",
                ),
            ],
            conflicts: vec![AgentConflict {
                topic: "memory".to_owned(),
                message_ids: vec!["coder".to_owned(), "reviewer".to_owned()],
                roles: vec![AgentRole::Coder, AgentRole::Reviewer],
                summary: "memory side effect has unresolved disagreement".to_owned(),
                resolved: false,
                resolution_hint: "schedule repair before memory or next task promotion".to_owned(),
            }],
        };
        let conflict_record = ConflictReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ConflictReportSummaryHistory::new(),
                &report,
                ConflictReportHealthPolicy::default(),
            );
        let repair_task_ids = conflict_record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let memory_note = AgentTask::new(
            "memory-note",
            AgentRole::MemoryCurator,
            "write memory note only after conflict repair",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(10);
        let next_task = AgentTask::new(
            "next-task-promotion",
            AgentRole::Planner,
            "promote next task only after conflict repair",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(10);

        let schedule = RecursiveAgentScheduler::new(10).plan_repair_first(
            conflict_record.gate_decision.repair_tasks.clone(),
            vec![memory_note, next_task],
        );

        assert!(conflict_record.requires_repair_first());
        assert!(!conflict_record.can_promote_side_effects());
        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.waves[0].task_ids, repair_task_ids);
        assert_eq!(
            schedule.waves[1].task_ids,
            vec!["memory-note", "next-task-promotion"]
        );
        assert_eq!(schedule.blocked_task_ids, Vec::<String>::new());
        assert!(schedule.gate().can_dispatch_waves);
    }

    #[test]
    fn scheduler_places_window_ownership_repairs_before_window_tasks() {
        let review = AgentWindowOwnershipReviewer::new().review(vec![
            AgentWindowOwnership::new("window-1")
                .owns_path("crates/norion-agent/src")
                .changed_path("crates/norion-agent/src/collaboration.rs"),
            AgentWindowOwnership::new("window-2")
                .owns_path("crates/norion-agent/src")
                .changed_path("crates\\norion-agent\\src\\collaboration.rs"),
        ]);
        let repair_task_ids = review.repair_task_ids();
        let window_1_task = AgentTask::new(
            "window-1-normal",
            AgentRole::Custom("window-1".to_owned()),
            "continue window 1 after ownership repair",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(10);
        let window_2_task = AgentTask::new(
            "window-2-normal",
            AgentRole::Custom("window-2".to_owned()),
            "continue window 2 after ownership repair",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(10);

        let schedule = RecursiveAgentScheduler::new(10).plan_repair_first(
            review.repair_tasks.clone(),
            vec![window_2_task, window_1_task],
        );

        assert!(review.requires_repair_first);
        assert!(!review.can_write);
        assert_eq!(
            review.reasons,
            vec![
                "ownership_conflict path=crates/norion-agent/src/collaboration.rs windows=window-1,window-2"
                    .to_owned()
            ]
        );
        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.waves[0].task_ids, repair_task_ids);
        assert_eq!(
            schedule.waves[1].task_ids,
            vec!["window-1-normal", "window-2-normal"]
        );
        assert_eq!(schedule.blocked_task_ids, Vec::<String>::new());
        assert!(schedule.gate().can_dispatch_waves);
    }

    #[test]
    fn scheduler_leaves_cycles_blocked() {
        let left = AgentTask::new(
            "left",
            AgentRole::Custom("left".to_owned()),
            "wait right",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("right");
        let right = AgentTask::new(
            "right",
            AgentRole::Custom("right".to_owned()),
            "wait left",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("left");

        let schedule = RecursiveAgentScheduler::new(2).plan(vec![left, right]);

        assert!(schedule.waves.is_empty());
        assert_eq!(schedule.blocked_task_ids, vec!["left", "right"]);
        assert!(schedule.has_blocked_tasks());

        let summary = schedule.summary();
        let gate = schedule.gate();

        assert_eq!(summary.waves, 0);
        assert_eq!(summary.completed_tasks, 0);
        assert_eq!(summary.blocked_tasks, 2);
        assert_eq!(summary.average_wave_parallelism, 0.0);
        assert!(!gate.can_dispatch_waves);
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec![
                "schedule_empty_waves".to_owned(),
                "schedule_blocked_tasks=2".to_owned()
            ]
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_recursive_schedule_gate_requires_repair_first=true" })
        );
    }

    #[test]
    fn scheduler_blocks_partial_schedule_with_missing_dependency() {
        let planner = AgentTask::new(
            "planner",
            AgentRole::Planner,
            "split work",
            AgentBudget::new(4, 1, 1),
        );
        let coder = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "write patch",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("planner");
        let orphan = AgentTask::new(
            "memory",
            AgentRole::MemoryCurator,
            "remember external review",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("missing-review");

        let schedule = RecursiveAgentScheduler::new(2).plan(vec![orphan, coder, planner]);
        let summary = schedule.summary();
        let gate = schedule.gate();

        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.waves[0].task_ids, vec!["planner"]);
        assert_eq!(schedule.waves[1].task_ids, vec!["coder"]);
        assert_eq!(schedule.completed_task_ids, vec!["coder", "planner"]);
        assert_eq!(schedule.blocked_task_ids, vec!["memory"]);
        assert!(schedule.has_blocked_tasks());
        assert_eq!(summary.waves, 2);
        assert_eq!(summary.completed_tasks, 2);
        assert_eq!(summary.blocked_tasks, 1);
        assert!(!gate.can_dispatch_waves);
        assert!(gate.requires_repair_first);
        assert_eq!(gate.reasons, vec!["schedule_blocked_tasks=1"]);
        assert!(
            gate.telemetry
                .iter()
                .any(|line| line == "agent_recursive_schedule_gate_blocked_tasks=1")
        );
    }

    #[test]
    fn recursive_schedule_history_watches_empty() {
        let health = RecursiveAgentScheduleSummaryHistory::new()
            .health(RecursiveAgentScheduleHealthPolicy::default());

        assert_eq!(health.status, RecursiveAgentScheduleHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["recursive_schedule_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_recursive_schedule_dashboard_records=0" })
        );
    }

    #[test]
    fn recursive_schedule_history_marks_stable_waves() {
        let planner = AgentTask::new(
            "planner",
            AgentRole::Planner,
            "split work",
            AgentBudget::new(4, 1, 1),
        );
        let coder = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "write patch",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("planner");
        let reviewer = AgentTask::new(
            "reviewer",
            AgentRole::Reviewer,
            "review patch",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("planner");
        let schedule = RecursiveAgentScheduler::new(2).plan(vec![reviewer, coder, planner]);

        let record = RecursiveAgentScheduleSummaryHistoryRecorder::new()
            .record_schedule_with_health(
                RecursiveAgentScheduleSummaryHistory::new(),
                &schedule,
                RecursiveAgentScheduleHealthPolicy::default(),
            );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert_eq!(record.dashboard.schedulable_records, 1);
        assert_eq!(record.dashboard.blocked_records, 0);
        assert_eq!(record.dashboard.empty_wave_records, 0);
        assert_eq!(record.dashboard.waves, 2);
        assert_eq!(record.dashboard.completed_tasks, 3);
        assert_eq!(record.dashboard.schedule_rate, 1.0);
        assert_eq!(
            record.health.status,
            RecursiveAgentScheduleHealthStatus::Stable
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
                .any(|line| { line == "agent_recursive_schedule_history_record_status=stable" })
        );
    }

    #[test]
    fn recursive_schedule_history_repairs_blocked_cycles() {
        let clean = RecursiveAgentScheduleSummary {
            max_parallel_tasks: 2,
            waves: 1,
            completed_tasks: 2,
            blocked_tasks: 0,
            max_wave_parallelism: 2,
            average_wave_parallelism: 2.0,
            telemetry: Vec::new(),
        };
        let blocked = RecursiveAgentScheduleSummary {
            max_parallel_tasks: 2,
            waves: 0,
            completed_tasks: 0,
            blocked_tasks: 2,
            max_wave_parallelism: 0,
            average_wave_parallelism: 0.0,
            telemetry: Vec::new(),
        };
        let history = RecursiveAgentScheduleSummaryHistory::from_summaries(vec![clean]);

        let record = RecursiveAgentScheduleSummaryHistoryRecorder::new()
            .record_summary_with_health(
                history,
                blocked,
                RecursiveAgentScheduleHealthPolicy::default(),
            );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.schedulable_records, 1);
        assert_eq!(record.dashboard.blocked_records, 1);
        assert_eq!(record.dashboard.empty_wave_records, 1);
        assert_eq!(record.dashboard.blocked_tasks, 2);
        assert_eq!(record.dashboard.schedule_rate, 0.5);
        assert_eq!(
            record.health.status,
            RecursiveAgentScheduleHealthStatus::Repair
        );
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "recursive_schedule_blocked_records=1>0",
                "recursive_schedule_blocked_tasks=2>0",
                "recursive_schedule_empty_wave_records=1>0",
                "recursive_schedule_rate=0.500<0.67",
            ]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_recursive_schedule_history_record_status=repair" })
        );
    }

    #[test]
    fn recursive_schedule_history_gate_preserves_stable_dispatch() {
        let planner = AgentTask::new(
            "planner",
            AgentRole::Planner,
            "split work",
            AgentBudget::new(4, 1, 1),
        );
        let coder = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "write patch",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("planner");
        let schedule = RecursiveAgentScheduler::new(2).plan(vec![coder, planner]);
        let history_record = RecursiveAgentScheduleSummaryHistoryRecorder::new()
            .record_schedule_with_health(
                RecursiveAgentScheduleSummaryHistory::new(),
                &schedule,
                RecursiveAgentScheduleHealthPolicy::default(),
            );

        let gate = RecursiveAgentScheduleHistoryGate::new().gate(&schedule, &history_record);

        assert!(gate.can_dispatch_waves);
        assert!(gate.is_dispatchable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert!(gate.reasons.is_empty());
        assert_eq!(
            gate.schedule_health.status,
            RecursiveAgentScheduleHealthStatus::Stable
        );
        assert!(gate.telemetry.iter().any(|line| {
            line == "agent_recursive_schedule_history_gate_can_dispatch_waves=true"
        }));
    }

    #[test]
    fn recursive_schedule_history_gate_repairs_dirty_history_before_dispatch() {
        let clean = RecursiveAgentScheduleSummary {
            max_parallel_tasks: 2,
            waves: 1,
            completed_tasks: 2,
            blocked_tasks: 0,
            max_wave_parallelism: 2,
            average_wave_parallelism: 2.0,
            telemetry: Vec::new(),
        };
        let blocked = RecursiveAgentScheduleSummary {
            max_parallel_tasks: 2,
            waves: 0,
            completed_tasks: 0,
            blocked_tasks: 2,
            max_wave_parallelism: 0,
            average_wave_parallelism: 0.0,
            telemetry: Vec::new(),
        };
        let history = RecursiveAgentScheduleSummaryHistory::from_summaries(vec![blocked]);
        let history_record = RecursiveAgentScheduleSummaryHistoryRecorder::new()
            .record_summary_with_health(
                history,
                clean,
                RecursiveAgentScheduleHealthPolicy::default(),
            );
        let planner = AgentTask::new(
            "planner",
            AgentRole::Planner,
            "split work",
            AgentBudget::new(4, 1, 1),
        );
        let schedule = RecursiveAgentScheduler::new(2).plan(vec![planner]);

        let gate = RecursiveAgentScheduleHistoryGate::new().gate(&schedule, &history_record);

        assert!(gate.schedule_gate.can_dispatch_waves);
        assert!(!gate.can_dispatch_waves);
        assert!(!gate.is_dispatchable());
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.schedule_health.status,
            RecursiveAgentScheduleHealthStatus::Repair
        );
        assert_eq!(gate.repair_tasks.len(), 4);
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "recursive-agent-schedule-repair-0",
                "recursive-agent-schedule-repair-1",
                "recursive-agent-schedule-repair-2",
                "recursive-agent-schedule-repair-3",
            ]
        );
        assert_eq!(
            gate.reasons,
            vec![
                "recursive_schedule_history:recursive_schedule_blocked_records=1>0",
                "recursive_schedule_history:recursive_schedule_blocked_tasks=2>0",
                "recursive_schedule_history:recursive_schedule_empty_wave_records=1>0",
                "recursive_schedule_history:recursive_schedule_rate=0.500<0.67",
            ]
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_recursive_schedule_history_gate_repair_tasks=4" })
        );
    }

    #[test]
    fn recursive_schedule_history_recorder_records_and_gates_stable_schedule() {
        let planner = AgentTask::new(
            "planner",
            AgentRole::Planner,
            "split work",
            AgentBudget::new(4, 1, 1),
        );
        let coder = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "write patch",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("planner");
        let schedule = RecursiveAgentScheduler::new(2).plan(vec![coder, planner]);

        let record = RecursiveAgentScheduleSummaryHistoryRecorder::new()
            .record_schedule_with_health_gate(
                RecursiveAgentScheduleSummaryHistory::new(),
                &schedule,
                RecursiveAgentScheduleHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.can_dispatch_waves());
        assert!(record.gate_decision.is_dispatchable());
        assert_eq!(
            record.health_record.health.status,
            RecursiveAgentScheduleHealthStatus::Stable
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_recursive_schedule_history_gate_record_can_dispatch_waves=true"
        }));
    }

    #[test]
    fn recursive_schedule_history_recorder_records_and_gates_repair_first() {
        let blocked = RecursiveAgentScheduleSummary {
            max_parallel_tasks: 2,
            waves: 0,
            completed_tasks: 0,
            blocked_tasks: 2,
            max_wave_parallelism: 0,
            average_wave_parallelism: 0.0,
            telemetry: Vec::new(),
        };
        let planner = AgentTask::new(
            "planner",
            AgentRole::Planner,
            "split work",
            AgentBudget::new(4, 1, 1),
        );
        let schedule = RecursiveAgentScheduler::new(2).plan(vec![planner]);

        let record = RecursiveAgentScheduleSummaryHistoryRecorder::new()
            .record_schedule_with_health_gate(
                RecursiveAgentScheduleSummaryHistory::from_summaries(vec![blocked]),
                &schedule,
                RecursiveAgentScheduleHealthPolicy::default(),
            );

        assert_eq!(record.records(), 2);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(!record.can_dispatch_waves());
        assert_eq!(
            record.health_record.health.status,
            RecursiveAgentScheduleHealthStatus::Repair
        );
        assert_eq!(record.gate_decision.repair_tasks.len(), 4);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_recursive_schedule_history_gate_record_requires_repair_first=true"
        }));
    }
}
