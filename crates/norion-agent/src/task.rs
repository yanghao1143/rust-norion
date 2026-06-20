use std::collections::{BTreeMap, BTreeSet};

use crate::budget::{AgentBudget, BudgetError, BudgetLedger, BudgetPolicy, BudgetPolicyError};
use crate::message::AgentMessage;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentRole {
    Planner,
    Researcher,
    Coder,
    Reviewer,
    Tester,
    MemoryCurator,
    Aggregator,
    Reflector,
    Custom(String),
}

impl AgentRole {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Planner => "planner",
            Self::Researcher => "researcher",
            Self::Coder => "coder",
            Self::Reviewer => "reviewer",
            Self::Tester => "tester",
            Self::MemoryCurator => "memory_curator",
            Self::Aggregator => "aggregator",
            Self::Reflector => "reflector",
            Self::Custom(value) => value.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTask {
    pub id: String,
    pub role: AgentRole,
    pub objective: String,
    pub lane: String,
    pub priority: u8,
    pub dependencies: Vec<String>,
    pub required_budget: AgentBudget,
}

impl AgentTask {
    pub fn new(
        id: impl Into<String>,
        role: AgentRole,
        objective: impl Into<String>,
        required_budget: AgentBudget,
    ) -> Self {
        Self {
            id: id.into(),
            role,
            objective: objective.into(),
            lane: "default".to_owned(),
            priority: 5,
            dependencies: Vec::new(),
            required_budget,
        }
    }

    pub fn with_lane(mut self, lane: impl Into<String>) -> Self {
        self.lane = lane.into();
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(10);
        self
    }

    pub fn depends_on(mut self, task_id: impl Into<String>) -> Self {
        self.dependencies.push(task_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentTaskQueue {
    tasks: BTreeMap<String, AgentTask>,
}

impl AgentTaskQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_tasks(tasks: Vec<AgentTask>) -> Self {
        let mut queue = Self::new();
        for task in tasks {
            queue.push(task);
        }
        queue
    }

    pub fn push(&mut self, task: AgentTask) -> Option<AgentTask> {
        self.tasks.insert(task.id.clone(), task)
    }

    pub fn remove(&mut self, task_id: &str) -> Option<AgentTask> {
        self.tasks.remove(task_id)
    }

    pub fn task_ids(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    pub fn tasks(&self) -> Vec<AgentTask> {
        self.tasks.values().cloned().collect()
    }

    pub fn immediate_ready_tasks(&self) -> Vec<AgentTask> {
        self.ready_tasks(&BTreeSet::new())
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn next_queue_tasks(&self) -> Vec<AgentTask> {
        self.tasks()
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn ready_tasks(&self, completed: &BTreeSet<String>) -> Vec<&AgentTask> {
        let mut tasks = self
            .tasks
            .values()
            .filter(|task| dependencies_satisfied(task, completed))
            .collect::<Vec<_>>();
        sort_task_refs(&mut tasks);
        tasks
    }

    pub fn blocked_tasks(&self, completed: &BTreeSet<String>) -> Vec<&AgentTask> {
        let mut tasks = self
            .tasks
            .values()
            .filter(|task| !dependencies_satisfied(task, completed))
            .collect::<Vec<_>>();
        sort_task_refs(&mut tasks);
        tasks
    }

    pub fn drain_ready(&mut self, completed: &BTreeSet<String>) -> Vec<AgentTask> {
        let ready_ids = self
            .ready_tasks(completed)
            .into_iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let mut ready = ready_ids
            .into_iter()
            .filter_map(|task_id| self.tasks.remove(&task_id))
            .collect::<Vec<_>>();
        sort_tasks(&mut ready);
        ready
    }

    pub fn with_repair_first(self, repair_tasks: &[AgentTask]) -> Self {
        if repair_tasks.is_empty() {
            return self;
        }

        let business_task_ids = self.tasks.keys().cloned().collect::<BTreeSet<_>>();
        let mut ordered_repair_tasks = repair_tasks.iter().collect::<Vec<_>>();
        sort_task_refs(&mut ordered_repair_tasks);
        let repair_task_ids = ordered_repair_tasks
            .into_iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let repair_task_id_set = repair_task_ids.iter().cloned().collect::<BTreeSet<_>>();
        let repair_queue_tasks = repair_tasks
            .iter()
            .cloned()
            .map(|mut task| {
                task.dependencies.retain(|dependency| {
                    !business_task_ids.contains(dependency)
                        || repair_task_id_set.contains(dependency)
                });
                task
            })
            .collect::<Vec<_>>();
        let mut merged_queue = Self::from_tasks(repair_queue_tasks);

        for mut task in self.tasks.into_values() {
            if repair_task_id_set.contains(&task.id) {
                continue;
            }

            for repair_task_id in &repair_task_ids {
                if !task
                    .dependencies
                    .iter()
                    .any(|dependency| dependency == repair_task_id)
                {
                    task.dependencies.push(repair_task_id.clone());
                }
            }
            merged_queue.push(task);
        }

        merged_queue
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentResult {
    pub task_id: String,
    pub role: AgentRole,
    pub accepted: bool,
    pub summary: String,
    pub messages: Vec<AgentMessage>,
    pub budget_spent: AgentBudget,
}

impl AgentResult {
    pub fn accepted(
        task: &AgentTask,
        summary: impl Into<String>,
        messages: Vec<AgentMessage>,
        budget_spent: AgentBudget,
    ) -> Self {
        Self {
            task_id: task.id.clone(),
            role: task.role.clone(),
            accepted: true,
            summary: summary.into(),
            messages,
            budget_spent,
        }
    }

    pub fn rejected(task: &AgentTask, summary: impl Into<String>) -> Self {
        Self {
            task_id: task.id.clone(),
            role: task.role.clone(),
            accepted: false,
            summary: summary.into(),
            messages: Vec::new(),
            budget_spent: AgentBudget::zero(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAssignment {
    pub task_id: String,
    pub role: AgentRole,
    pub lane: String,
    pub budget_reserved: AgentBudget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRejection {
    pub task_id: String,
    pub role: AgentRole,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskDispatchPlan {
    pub assignments: Vec<TaskAssignment>,
    pub rejections: Vec<TaskRejection>,
    pub remaining: BTreeMap<AgentRole, AgentBudget>,
}

impl TaskDispatchPlan {
    pub fn summary(&self) -> TaskDispatchPlanSummary {
        TaskDispatchPlanSummary::from_plan(self)
    }

    pub fn gate(&self) -> TaskDispatchGateDecision {
        TaskDispatchGateDecision::from_plan(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskDispatchPlanSummary {
    pub assignments: usize,
    pub rejections: usize,
    pub remaining_roles: usize,
    pub remaining_tokens: u32,
    pub remaining_steps: u32,
    pub remaining_messages: u32,
    pub assigned_rate: f32,
    pub rejected_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskDispatchHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl TaskDispatchHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TaskDispatchPlanSummaryHistory {
    summaries: Vec<TaskDispatchPlanSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskDispatchDashboard {
    pub total_records: usize,
    pub assignments: usize,
    pub rejections: usize,
    pub assigned_records: usize,
    pub rejected_records: usize,
    pub empty_assignment_records: usize,
    pub remaining_roles: usize,
    pub remaining_tokens: u32,
    pub remaining_steps: u32,
    pub remaining_messages: u32,
    pub assignment_rate: f32,
    pub rejection_rate: f32,
    pub assigned_record_rate: f32,
    pub rejected_record_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaskDispatchHealthPolicy {
    pub minimum_assignment_rate: f32,
    pub maximum_rejections: usize,
    pub maximum_rejected_records: usize,
    pub maximum_empty_assignment_records: usize,
    pub minimum_remaining_tokens: u32,
    pub minimum_remaining_steps: u32,
    pub minimum_remaining_messages: u32,
}

impl Default for TaskDispatchHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_assignment_rate: 1.0,
            maximum_rejections: 0,
            maximum_rejected_records: 0,
            maximum_empty_assignment_records: 0,
            minimum_remaining_tokens: 0,
            minimum_remaining_steps: 0,
            minimum_remaining_messages: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskDispatchHealth {
    pub status: TaskDispatchHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: TaskDispatchDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskDispatchPlanSummaryHistoryRecord {
    pub history: TaskDispatchPlanSummaryHistory,
    pub appended_summary: TaskDispatchPlanSummary,
    pub dashboard: TaskDispatchDashboard,
    pub health: TaskDispatchHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TaskDispatchPlanSummaryHistoryRecorder;

impl TaskDispatchPlanSummary {
    pub fn from_plan(plan: &TaskDispatchPlan) -> Self {
        let assignments = plan.assignments.len();
        let rejections = plan.rejections.len();
        let total = assignments + rejections;
        let remaining_roles = plan.remaining.len();
        let remaining_tokens = plan.remaining.values().map(|budget| budget.tokens).sum();
        let remaining_steps = plan.remaining.values().map(|budget| budget.steps).sum();
        let remaining_messages = plan.remaining.values().map(|budget| budget.messages).sum();
        let assigned_rate = rate(assignments, total);
        let rejected_rate = rate(rejections, total);
        let telemetry = task_dispatch_plan_summary_telemetry(
            assignments,
            rejections,
            remaining_roles,
            remaining_tokens,
            remaining_steps,
            remaining_messages,
            assigned_rate,
            rejected_rate,
        );

        Self {
            assignments,
            rejections,
            remaining_roles,
            remaining_tokens,
            remaining_steps,
            remaining_messages,
            assigned_rate,
            rejected_rate,
            telemetry,
        }
    }
}

impl TaskDispatchPlanSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<TaskDispatchPlanSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: TaskDispatchPlanSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&TaskDispatchPlanSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[TaskDispatchPlanSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> TaskDispatchDashboard {
        TaskDispatchDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: TaskDispatchHealthPolicy) -> TaskDispatchHealth {
        self.dashboard().health(policy)
    }
}

impl TaskDispatchDashboard {
    pub fn from_summaries(summaries: &[TaskDispatchPlanSummary]) -> Self {
        let total_records = summaries.len();
        let assignments = summaries
            .iter()
            .map(|summary| summary.assignments)
            .sum::<usize>();
        let rejections = summaries
            .iter()
            .map(|summary| summary.rejections)
            .sum::<usize>();
        let assigned_records = summaries
            .iter()
            .filter(|summary| summary.assignments > 0)
            .count();
        let rejected_records = summaries
            .iter()
            .filter(|summary| summary.rejections > 0)
            .count();
        let empty_assignment_records = summaries
            .iter()
            .filter(|summary| summary.assignments == 0)
            .count();
        let remaining_roles = summaries
            .iter()
            .map(|summary| summary.remaining_roles)
            .sum::<usize>();
        let remaining_tokens = summaries
            .iter()
            .map(|summary| summary.remaining_tokens)
            .sum::<u32>();
        let remaining_steps = summaries
            .iter()
            .map(|summary| summary.remaining_steps)
            .sum::<u32>();
        let remaining_messages = summaries
            .iter()
            .map(|summary| summary.remaining_messages)
            .sum::<u32>();
        let total_tasks = assignments + rejections;
        let assignment_rate = rate(assignments, total_tasks);
        let rejection_rate = rate(rejections, total_tasks);
        let assigned_record_rate = rate(assigned_records, total_records);
        let rejected_record_rate = rate(rejected_records, total_records);
        let telemetry = task_dispatch_dashboard_telemetry(
            total_records,
            assignments,
            rejections,
            assigned_records,
            rejected_records,
            empty_assignment_records,
            remaining_roles,
            remaining_tokens,
            remaining_steps,
            remaining_messages,
            assignment_rate,
            rejection_rate,
            assigned_record_rate,
            rejected_record_rate,
        );

        Self {
            total_records,
            assignments,
            rejections,
            assigned_records,
            rejected_records,
            empty_assignment_records,
            remaining_roles,
            remaining_tokens,
            remaining_steps,
            remaining_messages,
            assignment_rate,
            rejection_rate,
            assigned_record_rate,
            rejected_record_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: TaskDispatchHealthPolicy) -> TaskDispatchHealth {
        TaskDispatchHealth::from_dashboard(self.clone(), policy)
    }
}

impl TaskDispatchHealth {
    pub fn from_dashboard(
        dashboard: TaskDispatchDashboard,
        policy: TaskDispatchHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("task_dispatch_history_empty".to_owned());
        } else if dashboard.assignment_rate < policy.minimum_assignment_rate {
            watch_reasons.push(format!(
                "task_dispatch_assignment_rate={:.3}<{}",
                dashboard.assignment_rate, policy.minimum_assignment_rate
            ));
        }

        if dashboard.rejections > policy.maximum_rejections {
            repair_reasons.push(format!(
                "task_dispatch_rejections={}>{}",
                dashboard.rejections, policy.maximum_rejections
            ));
        }

        if dashboard.rejected_records > policy.maximum_rejected_records {
            repair_reasons.push(format!(
                "task_dispatch_rejected_records={}>{}",
                dashboard.rejected_records, policy.maximum_rejected_records
            ));
        }

        if dashboard.empty_assignment_records > policy.maximum_empty_assignment_records {
            repair_reasons.push(format!(
                "task_dispatch_empty_assignment_records={}>{}",
                dashboard.empty_assignment_records, policy.maximum_empty_assignment_records
            ));
        }

        if !dashboard.is_empty() && dashboard.remaining_tokens < policy.minimum_remaining_tokens {
            watch_reasons.push(format!(
                "task_dispatch_remaining_tokens={}<{}",
                dashboard.remaining_tokens, policy.minimum_remaining_tokens
            ));
        }

        if !dashboard.is_empty() && dashboard.remaining_steps < policy.minimum_remaining_steps {
            watch_reasons.push(format!(
                "task_dispatch_remaining_steps={}<{}",
                dashboard.remaining_steps, policy.minimum_remaining_steps
            ));
        }

        if !dashboard.is_empty() && dashboard.remaining_messages < policy.minimum_remaining_messages
        {
            watch_reasons.push(format!(
                "task_dispatch_remaining_messages={}<{}",
                dashboard.remaining_messages, policy.minimum_remaining_messages
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (TaskDispatchHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (TaskDispatchHealthStatus::Watch, watch_reasons)
        } else {
            (TaskDispatchHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == TaskDispatchHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != TaskDispatchHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == TaskDispatchHealthStatus::Repair
    }
}

impl TaskDispatchPlanSummaryHistoryRecord {
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

impl TaskDispatchPlanSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: TaskDispatchPlanSummaryHistory,
        summary: TaskDispatchPlanSummary,
        policy: TaskDispatchHealthPolicy,
    ) -> TaskDispatchPlanSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = task_dispatch_history_record_telemetry(&dashboard, &health);

        TaskDispatchPlanSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_plan_with_health(
        &self,
        history: TaskDispatchPlanSummaryHistory,
        plan: &TaskDispatchPlan,
        policy: TaskDispatchHealthPolicy,
    ) -> TaskDispatchPlanSummaryHistoryRecord {
        self.record_summary_with_health(history, plan.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskDispatchGateDecision {
    pub summary: TaskDispatchPlanSummary,
    pub can_dispatch: bool,
    pub requires_repair_first: bool,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl TaskDispatchGateDecision {
    pub fn from_plan(plan: &TaskDispatchPlan) -> Self {
        let summary = plan.summary();
        let mut reasons = Vec::new();

        if summary.assignments == 0 {
            reasons.push("dispatch_empty_assignments".to_owned());
        }
        reasons.extend(plan.rejections.iter().map(|rejection| {
            format!(
                "dispatch_rejection task={} role={} reason={}",
                rejection.task_id,
                rejection.role.as_str(),
                rejection.reason
            )
        }));

        let can_dispatch = summary.assignments > 0 && summary.rejections == 0;
        let requires_repair_first = summary.rejections > 0 || summary.assignments == 0;
        let telemetry = task_dispatch_gate_telemetry(
            can_dispatch,
            requires_repair_first,
            reasons.len(),
            &summary,
        );

        Self {
            summary,
            can_dispatch,
            requires_repair_first,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchPlanner {
    ledger: BudgetLedger,
}

impl DispatchPlanner {
    pub fn new(ledger: BudgetLedger) -> Self {
        Self { ledger }
    }

    pub fn plan(&mut self, mut tasks: Vec<AgentTask>) -> TaskDispatchPlan {
        self.plan_inner(&BudgetPolicy::permissive(), &mut tasks)
    }

    pub fn plan_with_policy(
        &mut self,
        mut tasks: Vec<AgentTask>,
        policy: &BudgetPolicy,
    ) -> TaskDispatchPlan {
        self.plan_inner(policy, &mut tasks)
    }

    fn plan_inner(
        &mut self,
        policy: &BudgetPolicy,
        tasks: &mut Vec<AgentTask>,
    ) -> TaskDispatchPlan {
        sort_tasks(tasks);
        let mut assignments = Vec::new();
        let mut rejections = Vec::new();

        for task in tasks.drain(..) {
            if let Err(error) = policy.validate_task(&task) {
                rejections.push(TaskRejection {
                    task_id: task.id,
                    role: task.role,
                    reason: budget_policy_error_summary(&error),
                });
                continue;
            }
            match self.ledger.consume(&task.role, task.required_budget) {
                Ok(()) => assignments.push(TaskAssignment {
                    task_id: task.id,
                    role: task.role,
                    lane: task.lane,
                    budget_reserved: task.required_budget,
                }),
                Err(error) => rejections.push(TaskRejection {
                    task_id: task.id,
                    role: task.role,
                    reason: budget_error_summary(&error),
                }),
            }
        }

        TaskDispatchPlan {
            assignments,
            rejections,
            remaining: self.ledger.snapshot(),
        }
    }
}

fn dependencies_satisfied(task: &AgentTask, completed: &BTreeSet<String>) -> bool {
    task.dependencies
        .iter()
        .all(|dependency| completed.contains(dependency))
}

fn sort_tasks(tasks: &mut [AgentTask]) {
    tasks.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| compare_task_ids(&left.id, &right.id))
    });
}

fn sort_task_refs(tasks: &mut [&AgentTask]) {
    tasks.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| compare_task_ids(&left.id, &right.id))
    });
}

fn compare_task_ids(left: &str, right: &str) -> std::cmp::Ordering {
    match (task_id_numeric_suffix(left), task_id_numeric_suffix(right)) {
        (Some((left_prefix, left_number)), Some((right_prefix, right_number)))
            if left_prefix == right_prefix =>
        {
            left_number.cmp(&right_number).then(left.cmp(right))
        }
        _ => left.cmp(right),
    }
}

fn task_id_numeric_suffix(id: &str) -> Option<(&str, u64)> {
    let split = id
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_ascii_digit())
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);

    if split == 0 || split == id.len() {
        return None;
    }

    id[split..]
        .parse::<u64>()
        .ok()
        .map(|number| (&id[..split], number))
}

fn budget_error_summary(error: &BudgetError) -> String {
    match error {
        BudgetError::MissingRole { role } => {
            format!("missing budget for role {}", role.as_str())
        }
        BudgetError::Insufficient {
            requested,
            remaining,
        } => format!(
            "insufficient budget requested=tokens:{} steps:{} messages:{} remaining=tokens:{} steps:{} messages:{}",
            requested.tokens,
            requested.steps,
            requested.messages,
            remaining.tokens,
            remaining.steps,
            remaining.messages
        ),
    }
}

fn budget_policy_error_summary(error: &BudgetPolicyError) -> String {
    match error {
        BudgetPolicyError::ZeroBudgetTask { role, .. } => {
            format!("zero budget task rejected for role {}", role.as_str())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn task_dispatch_plan_summary_telemetry(
    assignments: usize,
    rejections: usize,
    remaining_roles: usize,
    remaining_tokens: u32,
    remaining_steps: u32,
    remaining_messages: u32,
    assigned_rate: f32,
    rejected_rate: f32,
) -> Vec<String> {
    vec![
        "agent_task_dispatch_plan_summary=true".to_owned(),
        format!("agent_task_dispatch_plan_summary_assignments={assignments}"),
        format!("agent_task_dispatch_plan_summary_rejections={rejections}"),
        format!("agent_task_dispatch_plan_summary_remaining_roles={remaining_roles}"),
        format!("agent_task_dispatch_plan_summary_remaining_tokens={remaining_tokens}"),
        format!("agent_task_dispatch_plan_summary_remaining_steps={remaining_steps}"),
        format!("agent_task_dispatch_plan_summary_remaining_messages={remaining_messages}"),
        format!("agent_task_dispatch_plan_summary_assigned_rate={assigned_rate:.3}"),
        format!("agent_task_dispatch_plan_summary_rejected_rate={rejected_rate:.3}"),
    ]
}

fn task_dispatch_gate_telemetry(
    can_dispatch: bool,
    requires_repair_first: bool,
    reasons: usize,
    summary: &TaskDispatchPlanSummary,
) -> Vec<String> {
    vec![
        "agent_task_dispatch_gate=true".to_owned(),
        format!("agent_task_dispatch_gate_can_dispatch={can_dispatch}"),
        format!("agent_task_dispatch_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_task_dispatch_gate_reasons={reasons}"),
        format!(
            "agent_task_dispatch_gate_assignments={}",
            summary.assignments
        ),
        format!("agent_task_dispatch_gate_rejections={}", summary.rejections),
    ]
}

#[allow(clippy::too_many_arguments)]
fn task_dispatch_dashboard_telemetry(
    total_records: usize,
    assignments: usize,
    rejections: usize,
    assigned_records: usize,
    rejected_records: usize,
    empty_assignment_records: usize,
    remaining_roles: usize,
    remaining_tokens: u32,
    remaining_steps: u32,
    remaining_messages: u32,
    assignment_rate: f32,
    rejection_rate: f32,
    assigned_record_rate: f32,
    rejected_record_rate: f32,
) -> Vec<String> {
    vec![
        "agent_task_dispatch_dashboard=true".to_owned(),
        format!("agent_task_dispatch_dashboard_records={total_records}"),
        format!("agent_task_dispatch_dashboard_assignments={assignments}"),
        format!("agent_task_dispatch_dashboard_rejections={rejections}"),
        format!("agent_task_dispatch_dashboard_assigned_records={assigned_records}"),
        format!("agent_task_dispatch_dashboard_rejected_records={rejected_records}"),
        format!(
            "agent_task_dispatch_dashboard_empty_assignment_records={empty_assignment_records}"
        ),
        format!("agent_task_dispatch_dashboard_remaining_roles={remaining_roles}"),
        format!("agent_task_dispatch_dashboard_remaining_tokens={remaining_tokens}"),
        format!("agent_task_dispatch_dashboard_remaining_steps={remaining_steps}"),
        format!("agent_task_dispatch_dashboard_remaining_messages={remaining_messages}"),
        format!("agent_task_dispatch_dashboard_assignment_rate={assignment_rate:.3}"),
        format!("agent_task_dispatch_dashboard_rejection_rate={rejection_rate:.3}"),
        format!("agent_task_dispatch_dashboard_assigned_record_rate={assigned_record_rate:.3}"),
        format!("agent_task_dispatch_dashboard_rejected_record_rate={rejected_record_rate:.3}"),
    ]
}

fn task_dispatch_history_record_telemetry(
    dashboard: &TaskDispatchDashboard,
    health: &TaskDispatchHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_task_dispatch_history_record=true".to_owned(),
        format!(
            "agent_task_dispatch_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_task_dispatch_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_task_dispatch_history_record_assignments={}",
            dashboard.assignments
        ),
        format!(
            "agent_task_dispatch_history_record_rejections={}",
            dashboard.rejections
        ),
        format!(
            "agent_task_dispatch_history_record_assignment_rate={:.3}",
            dashboard.assignment_rate
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_task_dispatch_history_record_reason={reason}")),
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
    use crate::schedule::RecursiveAgentScheduler;

    #[test]
    fn task_queue_drains_ready_tasks_in_stable_priority_order() {
        let blocked = AgentTask::new(
            "memory",
            AgentRole::MemoryCurator,
            "capture lesson",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("aggregate");
        let lower_priority = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "draft patch",
            AgentBudget::new(8, 1, 1),
        )
        .with_priority(4);
        let higher_priority = AgentTask::new(
            "reviewer",
            AgentRole::Reviewer,
            "review patch",
            AgentBudget::new(8, 1, 1),
        )
        .with_priority(8);
        let mut queue = AgentTaskQueue::from_tasks(vec![blocked, lower_priority, higher_priority]);
        let completed = BTreeSet::new();

        let ready = queue.drain_ready(&completed);
        let blocked = queue.blocked_tasks(&completed);

        assert_eq!(
            ready
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["reviewer", "coder"]
        );
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].id, "memory");
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn task_queue_sorts_numeric_suffixes_naturally_within_priority() {
        let repair_10 = AgentTask::new(
            "adapter-boundary-repair-10",
            AgentRole::Planner,
            "later repair",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(1);
        let repair_2 = AgentTask::new(
            "adapter-boundary-repair-2",
            AgentRole::Planner,
            "middle repair",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(1);
        let repair_1 = AgentTask::new(
            "adapter-boundary-repair-1",
            AgentRole::Planner,
            "first repair",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(1);
        let mut queue = AgentTaskQueue::from_tasks(vec![repair_10, repair_2, repair_1]);

        let ready = queue.drain_ready(&BTreeSet::new());

        assert_eq!(
            ready
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "adapter-boundary-repair-1",
                "adapter-boundary-repair-2",
                "adapter-boundary-repair-10",
            ]
        );
    }

    #[test]
    fn task_queue_tasks_returns_stable_snapshot_without_draining() {
        let queue = AgentTaskQueue::from_tasks(vec![
            AgentTask::new(
                "window-3",
                AgentRole::Coder,
                "implement slice",
                AgentBudget::new(8, 1, 1),
            ),
            AgentTask::new(
                "window-1",
                AgentRole::Planner,
                "plan slice",
                AgentBudget::new(8, 1, 1),
            ),
        ]);

        let snapshot = queue.tasks();

        assert_eq!(
            snapshot
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["window-1", "window-3"]
        );
        assert_eq!(queue.task_ids(), vec!["window-1", "window-3"]);
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn task_queue_adapter_projection_distinguishes_ready_from_next_queue() {
        let follow_up = AgentTask::new(
            "follow-up",
            AgentRole::Planner,
            "plan next queue",
            AgentBudget::new(8, 1, 1),
        )
        .with_priority(3);
        let immediate_repair = AgentTask::new(
            "repair-1",
            AgentRole::Reviewer,
            "repair before side effects",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(9);
        let blocked_memory = AgentTask::new(
            "memory-note",
            AgentRole::MemoryCurator,
            "write memory after repair",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("repair-1")
        .with_priority(10);
        let queue = AgentTaskQueue::from_tasks(vec![blocked_memory, follow_up, immediate_repair]);

        let immediate_ready = queue.immediate_ready_tasks();
        let next_queue = queue.next_queue_tasks();

        assert_eq!(
            immediate_ready
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["repair-1", "follow-up"]
        );
        assert_eq!(
            next_queue
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["follow-up", "memory-note", "repair-1"]
        );
        assert_eq!(queue.len(), 3);
        assert_eq!(
            queue.task_ids(),
            vec!["follow-up", "memory-note", "repair-1"]
        );
    }

    #[test]
    fn task_queue_repair_first_adds_stable_dependencies_before_business_tasks() {
        let repair_b = AgentTask::new(
            "repair-b",
            AgentRole::Reviewer,
            "repair second blocker",
            AgentBudget::new(4, 1, 1),
        )
        .with_lane("repair")
        .with_priority(1);
        let repair_a = AgentTask::new(
            "repair-a",
            AgentRole::Planner,
            "repair first blocker",
            AgentBudget::new(4, 1, 1),
        )
        .with_lane("repair")
        .with_priority(1);
        let memory_note = AgentTask::new(
            "memory-note",
            AgentRole::MemoryCurator,
            "write memory note",
            AgentBudget::new(4, 1, 1),
        )
        .with_priority(10);
        let adaptive_state = AgentTask::new(
            "adaptive-state",
            AgentRole::Reflector,
            "promote adaptive state",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("repair-a")
        .with_priority(9);
        let duplicate_repair = AgentTask::new(
            "repair-a",
            AgentRole::Coder,
            "stale duplicate repair",
            AgentBudget::new(1, 1, 1),
        )
        .with_priority(10);

        let queue = AgentTaskQueue::from_tasks(vec![memory_note, adaptive_state, duplicate_repair])
            .with_repair_first(&[repair_b, repair_a]);

        assert_eq!(
            queue.task_ids(),
            vec!["adaptive-state", "memory-note", "repair-a", "repair-b"]
        );
        let tasks = queue.tasks();
        let memory_note = tasks
            .iter()
            .find(|task| task.id == "memory-note")
            .expect("memory task");
        let adaptive_state = tasks
            .iter()
            .find(|task| task.id == "adaptive-state")
            .expect("adaptive task");
        let repair_a = tasks
            .iter()
            .find(|task| task.id == "repair-a")
            .expect("repair task");

        assert_eq!(memory_note.dependencies, vec!["repair-a", "repair-b"]);
        assert_eq!(adaptive_state.dependencies, vec!["repair-a", "repair-b"]);
        assert_eq!(repair_a.role, AgentRole::Planner);

        let mut active_queue = queue.clone();
        let first_wave = active_queue.drain_ready(&BTreeSet::new());
        assert_eq!(
            first_wave
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["repair-a", "repair-b"]
        );

        let completed = BTreeSet::from(["repair-a".to_owned(), "repair-b".to_owned()]);
        let second_wave = active_queue.ready_tasks(&completed);
        assert_eq!(
            second_wave
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["memory-note", "adaptive-state"]
        );
    }

    #[test]
    fn task_queue_repair_first_strips_reverse_business_edges_before_merge() {
        let business_task = AgentTask::new(
            "business-task",
            AgentRole::Coder,
            "preserve ordinary work after repair",
            AgentBudget::new(8, 1, 1),
        )
        .with_priority(10);
        let repair_task = AgentTask::new(
            "repair-task",
            AgentRole::Reviewer,
            "repair before ordinary work",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("business-task")
        .with_lane("repair")
        .with_priority(1);

        let queue =
            AgentTaskQueue::from_tasks(vec![business_task]).with_repair_first(&[repair_task]);
        let tasks = queue.tasks();
        let merged_repair = tasks
            .iter()
            .find(|task| task.id == "repair-task")
            .expect("repair task should be preserved");
        let merged_business = tasks
            .iter()
            .find(|task| task.id == "business-task")
            .expect("business task should be preserved");

        assert_eq!(queue.task_ids(), vec!["business-task", "repair-task"]);
        assert!(merged_repair.dependencies.is_empty());
        assert_eq!(merged_business.dependencies, vec!["repair-task"]);
        let schedule = RecursiveAgentScheduler::new(4).plan(queue.tasks());
        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.waves[0].task_ids, vec!["repair-task"]);
        assert_eq!(schedule.waves[1].task_ids, vec!["business-task"]);
        assert!(schedule.blocked_task_ids.is_empty());
    }

    #[test]
    fn dispatch_plan_summary_compacts_clean_assignment_budget() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Coder, AgentBudget::new(20, 2, 2)),
        );
        let task = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "implement dispatch row",
            AgentBudget::new(8, 1, 1),
        );

        let plan = planner.plan_with_policy(vec![task], &BudgetPolicy::strict());
        let summary = plan.summary();
        let gate = plan.gate();

        assert_eq!(summary.assignments, 1);
        assert_eq!(summary.rejections, 0);
        assert_eq!(summary.remaining_tokens, 12);
        assert_eq!(summary.remaining_steps, 1);
        assert_eq!(summary.remaining_messages, 1);
        assert_eq!(summary.assigned_rate, 1.0);
        assert_eq!(summary.rejected_rate, 0.0);
        assert!(gate.can_dispatch);
        assert!(!gate.requires_repair_first);
        assert!(gate.reasons.is_empty());
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_task_dispatch_plan_summary_remaining_tokens=12" })
        );
    }

    #[test]
    fn dispatch_gate_repairs_budget_rejections_before_execution() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Reviewer, AgentBudget::new(4, 1, 1)),
        );
        let task = AgentTask::new(
            "reviewer",
            AgentRole::Reviewer,
            "review oversized wave",
            AgentBudget::new(8, 1, 1),
        );

        let plan = planner.plan_with_policy(vec![task], &BudgetPolicy::strict());
        let summary = plan.summary();
        let gate = plan.gate();

        assert_eq!(summary.assignments, 0);
        assert_eq!(summary.rejections, 1);
        assert_eq!(summary.remaining_tokens, 4);
        assert_eq!(summary.rejected_rate, 1.0);
        assert!(!gate.can_dispatch);
        assert!(gate.requires_repair_first);
        assert!(
            gate.reasons
                .iter()
                .any(|reason| reason.contains("insufficient budget"))
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_task_dispatch_gate_requires_repair_first=true" })
        );
    }

    #[test]
    fn dispatch_plan_strict_policy_rejects_zero_budget_before_execution() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1)),
        );
        let task = AgentTask::new(
            "zero-budget-window",
            AgentRole::Planner,
            "coordinate without an explicit budget",
            AgentBudget::zero(),
        );

        let plan = planner.plan_with_policy(vec![task], &BudgetPolicy::strict());
        let summary = plan.summary();
        let gate = plan.gate();

        assert!(plan.assignments.is_empty());
        assert_eq!(plan.rejections.len(), 1);
        assert_eq!(plan.rejections[0].task_id, "zero-budget-window");
        assert_eq!(plan.rejections[0].role, AgentRole::Planner);
        assert_eq!(
            plan.rejections[0].reason,
            "zero budget task rejected for role planner"
        );
        assert_eq!(summary.assignments, 0);
        assert_eq!(summary.rejections, 1);
        assert_eq!(summary.remaining_tokens, 8);
        assert!(!gate.can_dispatch);
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec![
                "dispatch_empty_assignments",
                "dispatch_rejection task=zero-budget-window role=planner reason=zero budget task rejected for role planner",
            ]
        );
    }

    #[test]
    fn dispatch_rejects_later_task_after_role_budget_is_exhausted() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Coder, AgentBudget::new(8, 1, 1)),
        );
        let first = AgentTask::new(
            "exhaust-first",
            AgentRole::Coder,
            "consume coder lane",
            AgentBudget::new(8, 1, 1),
        );
        let second = AgentTask::new(
            "exhaust-second",
            AgentRole::Coder,
            "follow-up coder lane",
            AgentBudget::new(1, 1, 1),
        );

        let plan = planner.plan_with_policy(vec![second, first], &BudgetPolicy::strict());
        let summary = plan.summary();
        let gate = plan.gate();

        assert_eq!(plan.assignments.len(), 1);
        assert_eq!(plan.assignments[0].task_id, "exhaust-first");
        assert_eq!(plan.rejections.len(), 1);
        assert_eq!(plan.rejections[0].task_id, "exhaust-second");
        assert!(plan.rejections[0].reason.contains("insufficient budget"));
        assert_eq!(
            plan.remaining.get(&AgentRole::Coder).copied(),
            Some(AgentBudget::zero())
        );
        assert_eq!(summary.assignments, 1);
        assert_eq!(summary.rejections, 1);
        assert_eq!(summary.remaining_tokens, 0);
        assert!(!gate.can_dispatch);
        assert!(gate.requires_repair_first);
    }

    #[test]
    fn dispatch_budget_exhaustion_isolated_to_depleted_role() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new()
                .with_budget(AgentRole::Coder, AgentBudget::new(8, 1, 1))
                .with_budget(AgentRole::Reviewer, AgentBudget::new(4, 1, 1)),
        );
        let coder_first = AgentTask::new(
            "coder-first",
            AgentRole::Coder,
            "consume coder lane",
            AgentBudget::new(8, 1, 1),
        )
        .with_priority(9);
        let coder_second = AgentTask::new(
            "coder-second",
            AgentRole::Coder,
            "follow-up coder lane",
            AgentBudget::new(1, 1, 1),
        );
        let reviewer = AgentTask::new(
            "reviewer-open",
            AgentRole::Reviewer,
            "review without sharing coder budget",
            AgentBudget::new(4, 1, 1),
        );

        let plan = planner.plan_with_policy(
            vec![coder_second, reviewer, coder_first],
            &BudgetPolicy::strict(),
        );
        let assignment_ids = plan
            .assignments
            .iter()
            .map(|assignment| assignment.task_id.as_str())
            .collect::<Vec<_>>();
        let rejection_ids = plan
            .rejections
            .iter()
            .map(|rejection| rejection.task_id.as_str())
            .collect::<Vec<_>>();
        let summary = plan.summary();
        let gate = plan.gate();

        assert_eq!(assignment_ids, vec!["coder-first", "reviewer-open"]);
        assert_eq!(rejection_ids, vec!["coder-second"]);
        assert_eq!(plan.rejections[0].role, AgentRole::Coder);
        assert!(plan.rejections[0].reason.contains("insufficient budget"));
        assert_eq!(
            plan.remaining.get(&AgentRole::Coder).copied(),
            Some(AgentBudget::zero())
        );
        assert_eq!(
            plan.remaining.get(&AgentRole::Reviewer).copied(),
            Some(AgentBudget::zero())
        );
        assert_eq!(summary.assignments, 2);
        assert_eq!(summary.rejections, 1);
        assert_eq!(summary.assigned_rate, 2.0 / 3.0);
        assert_eq!(summary.rejected_rate, 1.0 / 3.0);
        assert!(!gate.can_dispatch);
        assert!(gate.requires_repair_first);
        assert!(gate.reasons.iter().any(|reason| {
            reason
                == "dispatch_rejection task=coder-second role=coder reason=insufficient budget requested=tokens:1 steps:1 messages:1 remaining=tokens:0 steps:0 messages:0"
        }));
    }

    #[test]
    fn task_dispatch_history_watches_empty() {
        let health =
            TaskDispatchPlanSummaryHistory::new().health(TaskDispatchHealthPolicy::default());

        assert_eq!(health.status, TaskDispatchHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["task_dispatch_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_task_dispatch_dashboard_records=0" })
        );
    }

    #[test]
    fn task_dispatch_history_marks_clean_dispatch_stable() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Coder, AgentBudget::new(20, 2, 2)),
        );
        let task = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "implement stable dispatch",
            AgentBudget::new(8, 1, 1),
        );
        let plan = planner.plan_with_policy(vec![task], &BudgetPolicy::strict());

        let record = TaskDispatchPlanSummaryHistoryRecorder::new().record_plan_with_health(
            TaskDispatchPlanSummaryHistory::new(),
            &plan,
            TaskDispatchHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert_eq!(record.appended_summary.assignments, 1);
        assert_eq!(record.dashboard.assignments, 1);
        assert_eq!(record.dashboard.rejections, 0);
        assert_eq!(record.dashboard.assignment_rate, 1.0);
        assert_eq!(record.dashboard.assigned_record_rate, 1.0);
        assert_eq!(record.health.status, TaskDispatchHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.health.allows_service_advance());
        assert!(!record.health.requires_repair_first());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_task_dispatch_history_record_status=stable" })
        );
    }

    #[test]
    fn task_dispatch_history_repairs_rejection_and_empty_assignment_pressure() {
        let clean = TaskDispatchPlanSummary {
            assignments: 1,
            rejections: 0,
            remaining_roles: 1,
            remaining_tokens: 10,
            remaining_steps: 1,
            remaining_messages: 1,
            assigned_rate: 1.0,
            rejected_rate: 0.0,
            telemetry: Vec::new(),
        };
        let rejected = TaskDispatchPlanSummary {
            assignments: 0,
            rejections: 1,
            remaining_roles: 1,
            remaining_tokens: 4,
            remaining_steps: 1,
            remaining_messages: 1,
            assigned_rate: 0.0,
            rejected_rate: 1.0,
            telemetry: Vec::new(),
        };
        let history = TaskDispatchPlanSummaryHistory::from_summaries(vec![clean]);

        let record = TaskDispatchPlanSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            rejected,
            TaskDispatchHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.assignments, 1);
        assert_eq!(record.dashboard.rejections, 1);
        assert_eq!(record.dashboard.rejected_records, 1);
        assert_eq!(record.dashboard.empty_assignment_records, 1);
        assert_eq!(record.dashboard.assignment_rate, 0.5);
        assert_eq!(record.health.status, TaskDispatchHealthStatus::Repair);
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "task_dispatch_rejections=1>0",
                "task_dispatch_rejected_records=1>0",
                "task_dispatch_empty_assignment_records=1>0",
                "task_dispatch_assignment_rate=0.500<1",
            ]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_task_dispatch_history_record_status=repair" })
        );
    }
}
