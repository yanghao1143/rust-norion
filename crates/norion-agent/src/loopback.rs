use std::collections::BTreeSet;

use crate::cycle::AgentCycleHandoff;
use crate::eval::AgentReportGateDecision;
use crate::task::{AgentTask, AgentTaskQueue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLoopbackPlan {
    pub promote_adaptive_state: bool,
    pub enqueue_tasks: Vec<AgentTask>,
    pub blocked_reasons: Vec<String>,
}

impl AgentLoopbackPlan {
    pub fn next_queue(&self) -> AgentTaskQueue {
        AgentTaskQueue::from_tasks(self.enqueue_tasks.clone())
    }

    pub fn summary(&self) -> AgentLoopbackPlanSummary {
        AgentLoopbackPlanSummary::from_plan(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLoopbackPlanSummary {
    pub promote_adaptive_state: bool,
    pub queued_tasks: usize,
    pub blocked_reasons: usize,
    pub can_schedule_next_wave: bool,
    pub requires_repair_first: bool,
    pub task_ids: Vec<String>,
    pub repair_lanes: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentLoopbackPlanHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AgentLoopbackPlanHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentLoopbackPlanSummaryHistory {
    summaries: Vec<AgentLoopbackPlanSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentLoopbackPlanDashboard {
    pub total_records: usize,
    pub promoted_records: usize,
    pub queued_tasks: usize,
    pub blocked_reasons: usize,
    pub blocked_records: usize,
    pub repair_first_records: usize,
    pub unschedulable_records: usize,
    pub repair_lanes: usize,
    pub promotion_rate: f32,
    pub repair_first_rate: f32,
    pub schedulable_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentLoopbackPlanHealthPolicy {
    pub maximum_blocked_records: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_repair_first_records: usize,
    pub maximum_unschedulable_records: usize,
    pub minimum_schedulable_rate: f32,
}

impl Default for AgentLoopbackPlanHealthPolicy {
    fn default() -> Self {
        Self {
            maximum_blocked_records: 0,
            maximum_blocked_reasons: 0,
            maximum_repair_first_records: 0,
            maximum_unschedulable_records: usize::MAX,
            minimum_schedulable_rate: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentLoopbackPlanHealth {
    pub status: AgentLoopbackPlanHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentLoopbackPlanDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentLoopbackPlanSummaryHistoryRecord {
    pub history: AgentLoopbackPlanSummaryHistory,
    pub appended_summary: AgentLoopbackPlanSummary,
    pub dashboard: AgentLoopbackPlanDashboard,
    pub health: AgentLoopbackPlanHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentLoopbackPlanSummaryHistoryRecorder;

impl AgentLoopbackPlanSummary {
    pub fn from_plan(plan: &AgentLoopbackPlan) -> Self {
        let task_ids = plan
            .enqueue_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let repair_lanes = ordered_unique(
            plan.enqueue_tasks
                .iter()
                .filter(|task| task.lane.contains("repair") || task.lane.contains("eval"))
                .map(|task| task.lane.clone()),
        );
        let queued_tasks = plan.enqueue_tasks.len();
        let blocked_reasons = plan.blocked_reasons.len();
        let can_schedule_next_wave = queued_tasks > 0;
        let requires_repair_first = blocked_reasons > 0;
        let telemetry = loopback_plan_summary_telemetry(
            plan.promote_adaptive_state,
            queued_tasks,
            blocked_reasons,
            can_schedule_next_wave,
            requires_repair_first,
            repair_lanes.len(),
        );

        Self {
            promote_adaptive_state: plan.promote_adaptive_state,
            queued_tasks,
            blocked_reasons,
            can_schedule_next_wave,
            requires_repair_first,
            task_ids,
            repair_lanes,
            telemetry,
        }
    }
}

impl AgentLoopbackPlanSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentLoopbackPlanSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentLoopbackPlanSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentLoopbackPlanSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentLoopbackPlanSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentLoopbackPlanDashboard {
        AgentLoopbackPlanDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AgentLoopbackPlanHealthPolicy) -> AgentLoopbackPlanHealth {
        self.dashboard().health(policy)
    }
}

impl AgentLoopbackPlanDashboard {
    pub fn from_summaries(summaries: &[AgentLoopbackPlanSummary]) -> Self {
        let total_records = summaries.len();
        let promoted_records = summaries
            .iter()
            .filter(|summary| summary.promote_adaptive_state)
            .count();
        let queued_tasks = summaries
            .iter()
            .map(|summary| summary.queued_tasks)
            .sum::<usize>();
        let blocked_reasons = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let blocked_records = summaries
            .iter()
            .filter(|summary| summary.blocked_reasons > 0)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let unschedulable_records = summaries
            .iter()
            .filter(|summary| !summary.can_schedule_next_wave)
            .count();
        let repair_lanes = summaries
            .iter()
            .map(|summary| summary.repair_lanes.len())
            .sum::<usize>();
        let schedulable_records = total_records.saturating_sub(unschedulable_records);
        let promotion_rate = rate(promoted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let schedulable_rate = rate(schedulable_records, total_records);
        let telemetry = loopback_plan_dashboard_telemetry(
            total_records,
            promoted_records,
            queued_tasks,
            blocked_reasons,
            blocked_records,
            repair_first_records,
            unschedulable_records,
            repair_lanes,
            promotion_rate,
            repair_first_rate,
            schedulable_rate,
        );

        Self {
            total_records,
            promoted_records,
            queued_tasks,
            blocked_reasons,
            blocked_records,
            repair_first_records,
            unschedulable_records,
            repair_lanes,
            promotion_rate,
            repair_first_rate,
            schedulable_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: AgentLoopbackPlanHealthPolicy) -> AgentLoopbackPlanHealth {
        AgentLoopbackPlanHealth::from_dashboard(self.clone(), policy)
    }
}

impl AgentLoopbackPlanHealth {
    pub fn from_dashboard(
        dashboard: AgentLoopbackPlanDashboard,
        policy: AgentLoopbackPlanHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("agent_loopback_plan_history_empty".to_owned());
        } else if dashboard.schedulable_rate < policy.minimum_schedulable_rate {
            watch_reasons.push(format!(
                "agent_loopback_plan_schedulable_rate={:.3}<{}",
                dashboard.schedulable_rate, policy.minimum_schedulable_rate
            ));
        }

        if dashboard.blocked_records > policy.maximum_blocked_records {
            repair_reasons.push(format!(
                "agent_loopback_plan_blocked_records={}>{}",
                dashboard.blocked_records, policy.maximum_blocked_records
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_loopback_plan_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "agent_loopback_plan_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.unschedulable_records > policy.maximum_unschedulable_records {
            watch_reasons.push(format!(
                "agent_loopback_plan_unschedulable_records={}>{}",
                dashboard.unschedulable_records, policy.maximum_unschedulable_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentLoopbackPlanHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentLoopbackPlanHealthStatus::Watch, watch_reasons)
        } else {
            (AgentLoopbackPlanHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentLoopbackPlanHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentLoopbackPlanHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentLoopbackPlanHealthStatus::Repair
    }
}

impl AgentLoopbackPlanSummaryHistoryRecord {
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

impl AgentLoopbackPlanSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentLoopbackPlanSummaryHistory,
        summary: AgentLoopbackPlanSummary,
        policy: AgentLoopbackPlanHealthPolicy,
    ) -> AgentLoopbackPlanSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = loopback_plan_history_record_telemetry(&dashboard, &health);

        AgentLoopbackPlanSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_plan_with_health(
        &self,
        history: AgentLoopbackPlanSummaryHistory,
        plan: &AgentLoopbackPlan,
        policy: AgentLoopbackPlanHealthPolicy,
    ) -> AgentLoopbackPlanSummaryHistoryRecord {
        self.record_summary_with_health(history, plan.summary(), policy)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentLoopbackPlanner;

impl AgentLoopbackPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        handoff: &AgentCycleHandoff,
        decision: &AgentReportGateDecision,
    ) -> AgentLoopbackPlan {
        let mut blocked_reasons = decision
            .reasons
            .iter()
            .map(|reason| reason.as_line())
            .collect::<Vec<_>>();
        blocked_reasons.extend(handoff.blocked_reasons.clone());

        let promote_adaptive_state = decision.is_accepted() && handoff.blocked_reasons.is_empty();
        let enqueue_tasks = merge_tasks(
            if decision.is_accepted() {
                Vec::new()
            } else {
                decision.follow_up_tasks.clone()
            },
            handoff.follow_up_tasks.clone(),
        );

        AgentLoopbackPlan {
            promote_adaptive_state,
            enqueue_tasks,
            blocked_reasons,
        }
    }
}

fn merge_tasks(primary: Vec<AgentTask>, secondary: Vec<AgentTask>) -> Vec<AgentTask> {
    let mut seen = BTreeSet::new();
    let mut tasks = Vec::new();

    for task in primary.into_iter().chain(secondary) {
        if seen.insert(task.id.clone()) {
            tasks.push(task);
        }
    }

    tasks
}

fn ordered_unique<I>(items: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut unique = Vec::new();
    for item in items {
        if !unique.iter().any(|existing| existing == &item) {
            unique.push(item);
        }
    }
    unique
}

fn loopback_plan_summary_telemetry(
    promote_adaptive_state: bool,
    queued_tasks: usize,
    blocked_reasons: usize,
    can_schedule_next_wave: bool,
    requires_repair_first: bool,
    repair_lanes: usize,
) -> Vec<String> {
    vec![
        "agent_loopback_plan_summary=true".to_owned(),
        format!("agent_loopback_plan_summary_promote_adaptive_state={promote_adaptive_state}"),
        format!("agent_loopback_plan_summary_queued_tasks={queued_tasks}"),
        format!("agent_loopback_plan_summary_blocked_reasons={blocked_reasons}"),
        format!("agent_loopback_plan_summary_can_schedule_next_wave={can_schedule_next_wave}"),
        format!("agent_loopback_plan_summary_repair_first={requires_repair_first}"),
        format!("agent_loopback_plan_summary_repair_lanes={repair_lanes}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn loopback_plan_dashboard_telemetry(
    total_records: usize,
    promoted_records: usize,
    queued_tasks: usize,
    blocked_reasons: usize,
    blocked_records: usize,
    repair_first_records: usize,
    unschedulable_records: usize,
    repair_lanes: usize,
    promotion_rate: f32,
    repair_first_rate: f32,
    schedulable_rate: f32,
) -> Vec<String> {
    vec![
        "agent_loopback_plan_dashboard=true".to_owned(),
        format!("agent_loopback_plan_dashboard_records={total_records}"),
        format!("agent_loopback_plan_dashboard_promoted_records={promoted_records}"),
        format!("agent_loopback_plan_dashboard_queued_tasks={queued_tasks}"),
        format!("agent_loopback_plan_dashboard_blocked_reasons={blocked_reasons}"),
        format!("agent_loopback_plan_dashboard_blocked_records={blocked_records}"),
        format!("agent_loopback_plan_dashboard_repair_first_records={repair_first_records}"),
        format!("agent_loopback_plan_dashboard_unschedulable_records={unschedulable_records}"),
        format!("agent_loopback_plan_dashboard_repair_lanes={repair_lanes}"),
        format!("agent_loopback_plan_dashboard_promotion_rate={promotion_rate:.3}"),
        format!("agent_loopback_plan_dashboard_repair_first_rate={repair_first_rate:.3}"),
        format!("agent_loopback_plan_dashboard_schedulable_rate={schedulable_rate:.3}"),
    ]
}

fn loopback_plan_history_record_telemetry(
    dashboard: &AgentLoopbackPlanDashboard,
    health: &AgentLoopbackPlanHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_loopback_plan_history_record=true".to_owned(),
        format!(
            "agent_loopback_plan_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_loopback_plan_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_loopback_plan_history_record_promotion_rate={:.3}",
            dashboard.promotion_rate
        ),
        format!(
            "agent_loopback_plan_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_loopback_plan_history_record_reason={reason}")),
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
    use crate::budget::AgentBudget;
    use crate::eval::AgentReportGateReason;
    use crate::ports::MemoryNote;
    use crate::task::AgentRole;

    fn task(id: &str, role: AgentRole) -> AgentTask {
        AgentTask::new(id, role, "next step", AgentBudget::new(8, 1, 1))
    }

    fn task_with_lane(id: &str, role: AgentRole, lane: &str) -> AgentTask {
        task(id, role).with_lane(lane)
    }

    #[test]
    fn loopback_plan_history_watches_empty() {
        let health =
            AgentLoopbackPlanSummaryHistory::new().health(AgentLoopbackPlanHealthPolicy::default());

        assert_eq!(health.status, AgentLoopbackPlanHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_loopback_plan_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| line == "agent_loopback_plan_dashboard_records=0")
        );
    }

    #[test]
    fn loopback_plan_history_marks_clean_promotion_stable() {
        let plan = AgentLoopbackPlan {
            promote_adaptive_state: true,
            enqueue_tasks: vec![task("reinforce-memory", AgentRole::MemoryCurator)],
            blocked_reasons: Vec::new(),
        };

        let record = AgentLoopbackPlanSummaryHistoryRecorder::new().record_plan_with_health(
            AgentLoopbackPlanSummaryHistory::new(),
            &plan,
            AgentLoopbackPlanHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert!(record.appended_summary.promote_adaptive_state);
        assert_eq!(record.dashboard.promoted_records, 1);
        assert_eq!(record.dashboard.queued_tasks, 1);
        assert_eq!(record.dashboard.blocked_records, 0);
        assert_eq!(record.dashboard.promotion_rate, 1.0);
        assert_eq!(record.health.status, AgentLoopbackPlanHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.health.allows_service_advance());
        assert!(!record.health.requires_repair_first());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "agent_loopback_plan_history_record_status=stable")
        );
    }

    #[test]
    fn loopback_plan_history_repairs_blocked_repair_first_pressure() {
        let clean = AgentLoopbackPlanSummary {
            promote_adaptive_state: true,
            queued_tasks: 1,
            blocked_reasons: 0,
            can_schedule_next_wave: true,
            requires_repair_first: false,
            task_ids: vec!["reinforce-memory".to_owned()],
            repair_lanes: Vec::new(),
            telemetry: Vec::new(),
        };
        let blocked = AgentLoopbackPlanSummary {
            promote_adaptive_state: false,
            queued_tasks: 2,
            blocked_reasons: 2,
            can_schedule_next_wave: true,
            requires_repair_first: true,
            task_ids: vec![
                "report-gate-run-review".to_owned(),
                "repair-loop".to_owned(),
            ],
            repair_lanes: vec!["eval-review".to_owned(), "repair-loop".to_owned()],
            telemetry: Vec::new(),
        };
        let history = AgentLoopbackPlanSummaryHistory::from_summaries(vec![clean]);

        let record = AgentLoopbackPlanSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            blocked,
            AgentLoopbackPlanHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.promoted_records, 1);
        assert_eq!(record.dashboard.queued_tasks, 3);
        assert_eq!(record.dashboard.blocked_reasons, 2);
        assert_eq!(record.dashboard.blocked_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.repair_lanes, 2);
        assert_eq!(record.dashboard.repair_first_rate, 0.5);
        assert_eq!(record.health.status, AgentLoopbackPlanHealthStatus::Repair);
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "agent_loopback_plan_blocked_records=1>0".to_owned(),
                "agent_loopback_plan_blocked_reasons=2>0".to_owned(),
                "agent_loopback_plan_repair_first_records=1>0".to_owned(),
            ]
        );
    }

    #[test]
    fn loopback_promotes_state_and_keeps_handoff_tasks_after_clean_gate() {
        let handoff = AgentCycleHandoff {
            memory_notes: vec![MemoryNote::new("agent_cycle", "remember")],
            follow_up_tasks: vec![task("reinforce-memory", AgentRole::MemoryCurator)],
            blocked_reasons: Vec::new(),
        };
        let decision = AgentReportGateDecision {
            accepted: true,
            reasons: Vec::new(),
            follow_up_tasks: Vec::new(),
        };

        let plan = AgentLoopbackPlanner::new().plan(&handoff, &decision);

        assert!(plan.promote_adaptive_state);
        assert!(plan.blocked_reasons.is_empty());
        assert_eq!(plan.enqueue_tasks.len(), 1);
        assert_eq!(plan.enqueue_tasks[0].id, "reinforce-memory");
        assert_eq!(plan.next_queue().task_ids(), vec!["reinforce-memory"]);

        let summary = plan.summary();
        assert!(summary.promote_adaptive_state);
        assert_eq!(summary.queued_tasks, 1);
        assert_eq!(summary.blocked_reasons, 0);
        assert!(summary.can_schedule_next_wave);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.task_ids, vec!["reinforce-memory"]);
        assert!(summary.repair_lanes.is_empty());
    }

    #[test]
    fn loopback_blocks_state_and_prioritizes_gate_repairs() {
        let handoff = AgentCycleHandoff {
            memory_notes: Vec::new(),
            follow_up_tasks: vec![
                task_with_lane("repair-loop", AgentRole::Reviewer, "repair-loop"),
                task_with_lane("report-gate-run-review", AgentRole::Reviewer, "eval-review"),
            ],
            blocked_reasons: vec!["unresolved_conflicts=1".to_owned()],
        };
        let decision = AgentReportGateDecision {
            accepted: false,
            reasons: vec![AgentReportGateReason::new("execution_failures", "1")],
            follow_up_tasks: vec![
                task_with_lane("report-gate-run-review", AgentRole::Reviewer, "eval-review"),
                task_with_lane(
                    "report-gate-run-validation",
                    AgentRole::Tester,
                    "eval-validation",
                ),
            ],
        };

        let plan = AgentLoopbackPlanner::new().plan(&handoff, &decision);
        let task_ids = plan
            .enqueue_tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>();

        assert!(!plan.promote_adaptive_state);
        assert_eq!(
            plan.blocked_reasons,
            vec!["execution_failures=1", "unresolved_conflicts=1"]
        );
        assert_eq!(
            task_ids,
            vec![
                "report-gate-run-review",
                "report-gate-run-validation",
                "repair-loop",
            ]
        );

        let summary = plan.summary();
        assert!(!summary.promote_adaptive_state);
        assert_eq!(summary.queued_tasks, 3);
        assert_eq!(summary.blocked_reasons, 2);
        assert!(summary.can_schedule_next_wave);
        assert!(summary.requires_repair_first);
        assert_eq!(
            summary.task_ids,
            vec![
                "report-gate-run-review",
                "report-gate-run-validation",
                "repair-loop",
            ]
        );
        assert_eq!(
            summary.repair_lanes,
            vec!["eval-review", "eval-validation", "repair-loop"]
        );
    }
}
