use crate::cycle::AgentCycleDispatch;
use crate::ports::{AgentModelRouteRequest, AgentModelRouteRunError, EnginePort, RoutedEnginePort};
use crate::task::{AgentResult, AgentRole};

#[derive(Debug, Clone, PartialEq)]
pub struct AgentWaveExecution {
    pub results: Vec<AgentResult>,
    pub failures: Vec<AgentExecutionFailure>,
}

impl AgentWaveExecution {
    pub fn is_complete(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn summary(&self) -> AgentWaveExecutionSummary {
        AgentWaveExecutionSummary::from_execution(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionFailure {
    pub task_id: String,
    pub role: AgentRole,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentWaveExecutionSummary {
    pub results: usize,
    pub accepted_results: usize,
    pub rejected_results: usize,
    pub failures: usize,
    pub complete: bool,
    pub failed_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentWaveExecutionHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AgentWaveExecutionHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentWaveExecutionSummaryHistory {
    summaries: Vec<AgentWaveExecutionSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentWaveExecutionDashboard {
    pub total_records: usize,
    pub results: usize,
    pub accepted_results: usize,
    pub rejected_results: usize,
    pub failures: usize,
    pub failed_records: usize,
    pub incomplete_records: usize,
    pub empty_records: usize,
    pub complete_record_rate: f32,
    pub failure_record_rate: f32,
    pub accepted_result_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentWaveExecutionHealthPolicy {
    pub maximum_failures: usize,
    pub maximum_failed_records: usize,
    pub maximum_incomplete_records: usize,
    pub maximum_rejected_results: usize,
    pub maximum_empty_records: usize,
    pub minimum_complete_record_rate: f32,
    pub minimum_accepted_result_rate: f32,
}

impl Default for AgentWaveExecutionHealthPolicy {
    fn default() -> Self {
        Self {
            maximum_failures: 0,
            maximum_failed_records: 0,
            maximum_incomplete_records: 0,
            maximum_rejected_results: 0,
            maximum_empty_records: 0,
            minimum_complete_record_rate: 1.0,
            minimum_accepted_result_rate: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentWaveExecutionHealth {
    pub status: AgentWaveExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentWaveExecutionDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentWaveExecutionSummaryHistoryRecord {
    pub history: AgentWaveExecutionSummaryHistory,
    pub appended_summary: AgentWaveExecutionSummary,
    pub dashboard: AgentWaveExecutionDashboard,
    pub health: AgentWaveExecutionHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentWaveExecutionSummaryHistoryRecorder;

impl AgentWaveExecutionSummary {
    pub fn from_execution(execution: &AgentWaveExecution) -> Self {
        let results = execution.results.len();
        let accepted_results = execution
            .results
            .iter()
            .filter(|result| result.accepted)
            .count();
        let rejected_results = results.saturating_sub(accepted_results);
        let failures = execution.failures.len();
        let complete = execution.is_complete();
        let failed_task_ids = execution
            .failures
            .iter()
            .map(|failure| failure.task_id.clone())
            .collect::<Vec<_>>();
        let telemetry = agent_wave_execution_summary_telemetry(
            results,
            accepted_results,
            rejected_results,
            failures,
            complete,
        );

        Self {
            results,
            accepted_results,
            rejected_results,
            failures,
            complete,
            failed_task_ids,
            telemetry,
        }
    }
}

impl AgentWaveExecutionSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentWaveExecutionSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentWaveExecutionSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentWaveExecutionSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentWaveExecutionSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentWaveExecutionDashboard {
        AgentWaveExecutionDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AgentWaveExecutionHealthPolicy) -> AgentWaveExecutionHealth {
        self.dashboard().health(policy)
    }
}

impl AgentWaveExecutionDashboard {
    pub fn from_summaries(summaries: &[AgentWaveExecutionSummary]) -> Self {
        let total_records = summaries.len();
        let results = summaries
            .iter()
            .map(|summary| summary.results)
            .sum::<usize>();
        let accepted_results = summaries
            .iter()
            .map(|summary| summary.accepted_results)
            .sum::<usize>();
        let rejected_results = summaries
            .iter()
            .map(|summary| summary.rejected_results)
            .sum::<usize>();
        let failures = summaries
            .iter()
            .map(|summary| summary.failures)
            .sum::<usize>();
        let failed_records = summaries
            .iter()
            .filter(|summary| summary.failures > 0)
            .count();
        let incomplete_records = summaries.iter().filter(|summary| !summary.complete).count();
        let empty_records = summaries
            .iter()
            .filter(|summary| summary.results == 0 && summary.failures == 0)
            .count();
        let complete_records = total_records.saturating_sub(incomplete_records);
        let complete_record_rate = rate(complete_records, total_records);
        let failure_record_rate = rate(failed_records, total_records);
        let accepted_result_rate = rate(accepted_results, results);
        let telemetry = agent_wave_execution_dashboard_telemetry(
            total_records,
            results,
            accepted_results,
            rejected_results,
            failures,
            failed_records,
            incomplete_records,
            empty_records,
            complete_record_rate,
            failure_record_rate,
            accepted_result_rate,
        );

        Self {
            total_records,
            results,
            accepted_results,
            rejected_results,
            failures,
            failed_records,
            incomplete_records,
            empty_records,
            complete_record_rate,
            failure_record_rate,
            accepted_result_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: AgentWaveExecutionHealthPolicy) -> AgentWaveExecutionHealth {
        AgentWaveExecutionHealth::from_dashboard(self.clone(), policy)
    }
}

impl AgentWaveExecutionHealth {
    pub fn from_dashboard(
        dashboard: AgentWaveExecutionDashboard,
        policy: AgentWaveExecutionHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("agent_wave_execution_history_empty".to_owned());
        } else if dashboard.complete_record_rate < policy.minimum_complete_record_rate {
            watch_reasons.push(format!(
                "agent_wave_execution_complete_record_rate={:.3}<{}",
                dashboard.complete_record_rate, policy.minimum_complete_record_rate
            ));
        }

        if !dashboard.is_empty()
            && dashboard.accepted_result_rate < policy.minimum_accepted_result_rate
        {
            watch_reasons.push(format!(
                "agent_wave_execution_accepted_result_rate={:.3}<{}",
                dashboard.accepted_result_rate, policy.minimum_accepted_result_rate
            ));
        }

        if dashboard.failures > policy.maximum_failures {
            repair_reasons.push(format!(
                "agent_wave_execution_failures={}>{}",
                dashboard.failures, policy.maximum_failures
            ));
        }

        if dashboard.failed_records > policy.maximum_failed_records {
            repair_reasons.push(format!(
                "agent_wave_execution_failed_records={}>{}",
                dashboard.failed_records, policy.maximum_failed_records
            ));
        }

        if dashboard.incomplete_records > policy.maximum_incomplete_records {
            repair_reasons.push(format!(
                "agent_wave_execution_incomplete_records={}>{}",
                dashboard.incomplete_records, policy.maximum_incomplete_records
            ));
        }

        if dashboard.rejected_results > policy.maximum_rejected_results {
            repair_reasons.push(format!(
                "agent_wave_execution_rejected_results={}>{}",
                dashboard.rejected_results, policy.maximum_rejected_results
            ));
        }

        if dashboard.empty_records > policy.maximum_empty_records {
            watch_reasons.push(format!(
                "agent_wave_execution_empty_records={}>{}",
                dashboard.empty_records, policy.maximum_empty_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentWaveExecutionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentWaveExecutionHealthStatus::Watch, watch_reasons)
        } else {
            (AgentWaveExecutionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentWaveExecutionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentWaveExecutionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentWaveExecutionHealthStatus::Repair
    }
}

impl AgentWaveExecutionSummaryHistoryRecord {
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

impl AgentWaveExecutionSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentWaveExecutionSummaryHistory,
        summary: AgentWaveExecutionSummary,
        policy: AgentWaveExecutionHealthPolicy,
    ) -> AgentWaveExecutionSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = agent_wave_execution_history_record_telemetry(&dashboard, &health);

        AgentWaveExecutionSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_execution_with_health(
        &self,
        history: AgentWaveExecutionSummaryHistory,
        execution: &AgentWaveExecution,
        policy: AgentWaveExecutionHealthPolicy,
    ) -> AgentWaveExecutionSummaryHistoryRecord {
        self.record_summary_with_health(history, execution.summary(), policy)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentWaveExecutor;

impl AgentWaveExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute<E>(&self, dispatch: &AgentCycleDispatch, engine: &mut E) -> AgentWaveExecution
    where
        E: EnginePort,
        E::Error: ToString,
    {
        let mut results = Vec::new();
        let mut failures = Vec::new();

        for assignment in &dispatch.dispatch.assignments {
            let Some(task) = dispatch
                .assigned_tasks
                .iter()
                .find(|task| task.id == assignment.task_id)
            else {
                failures.push(AgentExecutionFailure {
                    task_id: assignment.task_id.clone(),
                    role: assignment.role.clone(),
                    reason: "assigned task missing from dispatch task catalog".to_owned(),
                });
                continue;
            };

            match engine.run_task(task) {
                Ok(result) => results.push(result),
                Err(error) => failures.push(AgentExecutionFailure {
                    task_id: assignment.task_id.clone(),
                    role: assignment.role.clone(),
                    reason: error.to_string(),
                }),
            }
        }

        AgentWaveExecution { results, failures }
    }

    pub fn execute_routed<E>(
        &self,
        dispatch: &AgentCycleDispatch,
        engine: &mut E,
        routes: &[AgentModelRouteRequest],
    ) -> AgentWaveExecution
    where
        E: RoutedEnginePort,
        E::Error: ToString,
    {
        let mut results = Vec::new();
        let mut failures = Vec::new();

        for assignment in &dispatch.dispatch.assignments {
            let Some(task) = dispatch
                .assigned_tasks
                .iter()
                .find(|task| task.id == assignment.task_id)
            else {
                failures.push(AgentExecutionFailure {
                    task_id: assignment.task_id.clone(),
                    role: assignment.role.clone(),
                    reason: "assigned task missing from dispatch task catalog".to_owned(),
                });
                continue;
            };

            let Some(route) = routes
                .iter()
                .find(|request| request.task.id == assignment.task_id)
            else {
                failures.push(AgentExecutionFailure {
                    task_id: assignment.task_id.clone(),
                    role: assignment.role.clone(),
                    reason: "assigned task missing Layer B model route proof".to_owned(),
                });
                continue;
            };

            if &route.task != task {
                failures.push(AgentExecutionFailure {
                    task_id: assignment.task_id.clone(),
                    role: assignment.role.clone(),
                    reason: "assigned model route task does not match dispatch task catalog"
                        .to_owned(),
                });
                continue;
            }

            match engine.run_routed_task(route) {
                Ok(result) => results.push(result),
                Err(error) => failures.push(AgentExecutionFailure {
                    task_id: assignment.task_id.clone(),
                    role: assignment.role.clone(),
                    reason: routed_execution_error_reason(error),
                }),
            }
        }

        AgentWaveExecution { results, failures }
    }
}

fn routed_execution_error_reason<E: ToString>(error: AgentModelRouteRunError<E>) -> String {
    match error {
        AgentModelRouteRunError::Route(error) => format!("model route rejected: {error:?}"),
        AgentModelRouteRunError::Engine(error) => error.to_string(),
    }
}

fn agent_wave_execution_summary_telemetry(
    results: usize,
    accepted_results: usize,
    rejected_results: usize,
    failures: usize,
    complete: bool,
) -> Vec<String> {
    vec![
        "agent_wave_execution_summary=true".to_owned(),
        format!("agent_wave_execution_summary_results={results}"),
        format!("agent_wave_execution_summary_accepted_results={accepted_results}"),
        format!("agent_wave_execution_summary_rejected_results={rejected_results}"),
        format!("agent_wave_execution_summary_failures={failures}"),
        format!("agent_wave_execution_summary_complete={complete}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn agent_wave_execution_dashboard_telemetry(
    total_records: usize,
    results: usize,
    accepted_results: usize,
    rejected_results: usize,
    failures: usize,
    failed_records: usize,
    incomplete_records: usize,
    empty_records: usize,
    complete_record_rate: f32,
    failure_record_rate: f32,
    accepted_result_rate: f32,
) -> Vec<String> {
    vec![
        "agent_wave_execution_dashboard=true".to_owned(),
        format!("agent_wave_execution_dashboard_records={total_records}"),
        format!("agent_wave_execution_dashboard_results={results}"),
        format!("agent_wave_execution_dashboard_accepted_results={accepted_results}"),
        format!("agent_wave_execution_dashboard_rejected_results={rejected_results}"),
        format!("agent_wave_execution_dashboard_failures={failures}"),
        format!("agent_wave_execution_dashboard_failed_records={failed_records}"),
        format!("agent_wave_execution_dashboard_incomplete_records={incomplete_records}"),
        format!("agent_wave_execution_dashboard_empty_records={empty_records}"),
        format!("agent_wave_execution_dashboard_complete_record_rate={complete_record_rate:.3}"),
        format!("agent_wave_execution_dashboard_failure_record_rate={failure_record_rate:.3}"),
        format!("agent_wave_execution_dashboard_accepted_result_rate={accepted_result_rate:.3}"),
    ]
}

fn agent_wave_execution_history_record_telemetry(
    dashboard: &AgentWaveExecutionDashboard,
    health: &AgentWaveExecutionHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_wave_execution_history_record=true".to_owned(),
        format!(
            "agent_wave_execution_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_wave_execution_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_wave_execution_history_record_failures={}",
            dashboard.failures
        ),
        format!(
            "agent_wave_execution_history_record_complete_rate={:.3}",
            dashboard.complete_record_rate
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_wave_execution_history_record_reason={reason}")),
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
    use std::collections::BTreeSet;

    use super::*;
    use crate::budget::{AgentBudget, BudgetLedger, BudgetPolicy};
    use crate::cycle::AgentCycleOrchestrator;
    use crate::message::{AgentMessage, AgentMessageKind};
    use crate::ports::{AgentModelRouteProof, AgentModelRouteRequest};
    use crate::task::{AgentRole, AgentTask, AgentTaskQueue, TaskAssignment, TaskDispatchPlan};

    #[derive(Debug, Clone)]
    struct FakeEngine {
        fail_task_id: Option<String>,
        called_task_ids: Vec<String>,
    }

    impl EnginePort for FakeEngine {
        type Error = String;

        fn run_task(&mut self, task: &AgentTask) -> Result<AgentResult, Self::Error> {
            self.called_task_ids.push(task.id.clone());
            if self.fail_task_id.as_deref() == Some(task.id.as_str()) {
                return Err(format!("engine failed {}", task.id));
            }
            Ok(AgentResult::accepted(
                task,
                format!("ran {}", task.id),
                vec![AgentMessage::new(
                    format!("message-{}", task.id),
                    task.role.clone(),
                    AgentMessageKind::Status,
                    "engine",
                    "pass runtime response",
                )],
                AgentBudget::new(1, 1, 1),
            ))
        }
    }

    fn fake_engine(fail_task_id: Option<&str>) -> FakeEngine {
        FakeEngine {
            fail_task_id: fail_task_id.map(str::to_owned),
            called_task_ids: Vec::new(),
        }
    }

    fn route_request(task: AgentTask) -> AgentModelRouteRequest {
        let prompt = format!("prompt for {}", task.id);
        AgentModelRouteRequest::try_new(
            task,
            prompt,
            AgentModelRouteProof::new(
                "model-registry-v1",
                "qwen-local-fast",
                "deterministic-inference-backend",
                "default-model-pool",
            ),
        )
        .unwrap()
    }

    fn single_coder_dispatch() -> AgentCycleDispatch {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "coder",
            AgentRole::Coder,
            "write patch",
            AgentBudget::new(8, 1, 1),
        )]);
        let ledger = BudgetLedger::new().with_budget(AgentRole::Coder, AgentBudget::new(8, 1, 1));
        AgentCycleOrchestrator::new().plan_next_wave(
            queue,
            &BTreeSet::new(),
            ledger,
            &BudgetPolicy::strict(),
            1,
        )
    }

    #[test]
    fn executor_runs_assigned_tasks_in_dispatch_order() {
        let queue = AgentTaskQueue::from_tasks(vec![
            AgentTask::new(
                "coder",
                AgentRole::Coder,
                "write patch",
                AgentBudget::new(8, 1, 1),
            )
            .with_priority(5),
            AgentTask::new(
                "reviewer",
                AgentRole::Reviewer,
                "review patch",
                AgentBudget::new(8, 1, 1),
            )
            .with_priority(8),
        ]);
        let ledger = BudgetLedger::new()
            .with_budget(AgentRole::Coder, AgentBudget::new(8, 1, 1))
            .with_budget(AgentRole::Reviewer, AgentBudget::new(8, 1, 1));
        let dispatch = AgentCycleOrchestrator::new().plan_next_wave(
            queue,
            &BTreeSet::new(),
            ledger,
            &BudgetPolicy::strict(),
            2,
        );
        let mut engine = fake_engine(None);

        let execution = AgentWaveExecutor::new().execute(&dispatch, &mut engine);

        assert!(execution.is_complete());
        assert_eq!(
            execution
                .results
                .iter()
                .map(|result| result.task_id.as_str())
                .collect::<Vec<_>>(),
            vec!["reviewer", "coder"]
        );
    }

    #[test]
    fn executor_records_engine_errors_without_fabricating_results() {
        let queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "coder",
            AgentRole::Coder,
            "write patch",
            AgentBudget::new(8, 1, 1),
        )]);
        let ledger = BudgetLedger::new().with_budget(AgentRole::Coder, AgentBudget::new(8, 1, 1));
        let dispatch = AgentCycleOrchestrator::new().plan_next_wave(
            queue,
            &BTreeSet::new(),
            ledger,
            &BudgetPolicy::strict(),
            1,
        );
        let mut engine = fake_engine(Some("coder"));

        let execution = AgentWaveExecutor::new().execute(&dispatch, &mut engine);

        assert!(!execution.is_complete());
        assert!(execution.results.is_empty());
        assert_eq!(execution.failures.len(), 1);
        assert_eq!(execution.failures[0].reason, "engine failed coder");
    }

    #[test]
    fn executor_records_missing_task_catalog_entries() {
        let dispatch = AgentCycleDispatch {
            wave: crate::schedule::AgentExecutionWave {
                wave: 0,
                task_ids: vec!["ghost".to_owned()],
                parallel_count: 1,
            },
            dispatch: TaskDispatchPlan {
                assignments: vec![TaskAssignment {
                    task_id: "ghost".to_owned(),
                    role: AgentRole::Planner,
                    lane: "default".to_owned(),
                    budget_reserved: AgentBudget::new(1, 1, 1),
                }],
                ..TaskDispatchPlan::default()
            },
            assigned_tasks: Vec::new(),
            blocked_task_ids: Vec::new(),
            remaining_queue: AgentTaskQueue::new(),
        };
        let mut engine = fake_engine(None);

        let execution = AgentWaveExecutor::new().execute(&dispatch, &mut engine);

        assert!(execution.results.is_empty());
        assert_eq!(execution.failures.len(), 1);
        assert_eq!(
            execution.failures[0].reason,
            "assigned task missing from dispatch task catalog"
        );
    }

    #[test]
    fn routed_executor_requires_route_before_engine_call() {
        let dispatch = single_coder_dispatch();
        let mut engine = fake_engine(None);

        let execution = AgentWaveExecutor::new().execute_routed(&dispatch, &mut engine, &[]);

        assert!(execution.results.is_empty());
        assert_eq!(execution.failures.len(), 1);
        assert_eq!(
            execution.failures[0].reason,
            "assigned task missing Layer B model route proof"
        );
        assert!(engine.called_task_ids.is_empty());
    }

    #[test]
    fn routed_executor_rejects_route_task_mismatch_before_engine_call() {
        let dispatch = single_coder_dispatch();
        let mut route = route_request(dispatch.assigned_tasks[0].clone());
        route.task.objective = "different task payload".to_owned();
        let mut engine = fake_engine(None);

        let execution = AgentWaveExecutor::new().execute_routed(&dispatch, &mut engine, &[route]);

        assert!(execution.results.is_empty());
        assert_eq!(execution.failures.len(), 1);
        assert_eq!(
            execution.failures[0].reason,
            "assigned model route task does not match dispatch task catalog"
        );
        assert!(engine.called_task_ids.is_empty());
    }

    #[test]
    fn routed_executor_records_layer_b_route_gate_on_success() {
        let dispatch = single_coder_dispatch();
        let routes = dispatch
            .assigned_tasks
            .iter()
            .cloned()
            .map(route_request)
            .collect::<Vec<_>>();
        let mut engine = fake_engine(None);

        let execution = AgentWaveExecutor::new().execute_routed(&dispatch, &mut engine, &routes);

        assert!(execution.is_complete());
        assert_eq!(engine.called_task_ids, vec!["coder"]);
        assert_eq!(execution.results.len(), 1);
        let route_gate = execution.results[0]
            .messages
            .iter()
            .find(|message| message.topic == "layer_b_model_route")
            .expect("routed execution should record Layer B route gate");
        assert_eq!(route_gate.kind, AgentMessageKind::Gate);
        assert!(
            route_gate
                .content
                .contains("model_registry_id=model-registry-v1")
        );
        assert!(
            route_gate
                .evidence
                .iter()
                .any(|line| line == "agent_model_route_prompt_chars=16")
        );
    }

    #[test]
    fn wave_execution_history_watches_empty() {
        let health = AgentWaveExecutionSummaryHistory::new()
            .health(AgentWaveExecutionHealthPolicy::default());

        assert_eq!(health.status, AgentWaveExecutionHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_wave_execution_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_wave_execution_dashboard_records=0" })
        );
    }

    #[test]
    fn wave_execution_history_marks_clean_execution_stable() {
        let task = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "write patch",
            AgentBudget::new(8, 1, 1),
        );
        let execution = AgentWaveExecution {
            results: vec![AgentResult::accepted(
                &task,
                "ran coder",
                Vec::new(),
                AgentBudget::new(1, 1, 1),
            )],
            failures: Vec::new(),
        };

        let record = AgentWaveExecutionSummaryHistoryRecorder::new().record_execution_with_health(
            AgentWaveExecutionSummaryHistory::new(),
            &execution,
            AgentWaveExecutionHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert_eq!(record.appended_summary.results, 1);
        assert_eq!(record.dashboard.accepted_results, 1);
        assert_eq!(record.dashboard.failures, 0);
        assert_eq!(record.dashboard.complete_record_rate, 1.0);
        assert_eq!(record.health.status, AgentWaveExecutionHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.health.allows_service_advance());
        assert!(!record.health.requires_repair_first());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_wave_execution_history_record_status=stable" })
        );
    }

    #[test]
    fn wave_execution_history_repairs_failures_and_rejected_results() {
        let clean = AgentWaveExecutionSummary {
            results: 1,
            accepted_results: 1,
            rejected_results: 0,
            failures: 0,
            complete: true,
            failed_task_ids: Vec::new(),
            telemetry: Vec::new(),
        };
        let dirty = AgentWaveExecutionSummary {
            results: 1,
            accepted_results: 0,
            rejected_results: 1,
            failures: 1,
            complete: false,
            failed_task_ids: vec!["reviewer".to_owned()],
            telemetry: Vec::new(),
        };
        let history = AgentWaveExecutionSummaryHistory::from_summaries(vec![clean]);

        let record = AgentWaveExecutionSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            dirty,
            AgentWaveExecutionHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.results, 2);
        assert_eq!(record.dashboard.accepted_results, 1);
        assert_eq!(record.dashboard.rejected_results, 1);
        assert_eq!(record.dashboard.failures, 1);
        assert_eq!(record.dashboard.failed_records, 1);
        assert_eq!(record.dashboard.incomplete_records, 1);
        assert_eq!(record.dashboard.complete_record_rate, 0.5);
        assert_eq!(record.health.status, AgentWaveExecutionHealthStatus::Repair);
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "agent_wave_execution_failures=1>0",
                "agent_wave_execution_failed_records=1>0",
                "agent_wave_execution_incomplete_records=1>0",
                "agent_wave_execution_rejected_results=1>0",
                "agent_wave_execution_complete_record_rate=0.500<1",
                "agent_wave_execution_accepted_result_rate=0.500<1",
            ]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_wave_execution_history_record_status=repair" })
        );
    }
}
