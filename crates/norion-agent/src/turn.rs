use std::collections::BTreeSet;

use crate::budget::{BudgetLedger, BudgetPolicy};
use crate::collaboration::AgentCollaborationAdapterSideEffectAdmission;
use crate::cycle::{AgentCycleEvidence, AgentCycleHandoff, AgentCycleReport};
use crate::eval::AgentReportEvidence;
use crate::ledger::AgentCycleLedger;
use crate::memory::{MemoryHandoffSubmitter, MemorySubmissionReport};
use crate::ports::{AgentModelRouteRequest, EnginePort, MemoryPort};
use crate::run::{SideEffectGate, SideEffectKind};
use crate::service::{
    AgentServiceCommand, AgentServiceCommandPlan, AgentServiceCommandPlanner,
    AgentServiceCommandReceipt,
};
use crate::step::{
    AgentClosedLoopDispatchPreparer, AgentClosedLoopExecutionDashboard,
    AgentClosedLoopExecutionHealth, AgentClosedLoopExecutionHealthPolicy,
    AgentClosedLoopExecutionHealthStatus, AgentClosedLoopExecutionHistory,
    AgentClosedLoopExecutionReport, AgentClosedLoopExecutionSummary, AgentClosedLoopNextTurnMode,
    AgentClosedLoopNextTurnPlan, AgentClosedLoopPreparedCycle, AgentClosedLoopPreparedCycleCloser,
    AgentClosedLoopPreparedExecution, AgentClosedLoopPreparedExecutor, AgentClosedLoopStep,
    AgentClosedLoopStepInput, AgentClosedLoopStepper,
};
use crate::task::{AgentRole, AgentTask, AgentTaskQueue};

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeTurnInput {
    pub history: AgentClosedLoopExecutionHistory,
    pub next_queue: AgentTaskQueue,
    pub completed_task_ids: BTreeSet<String>,
    pub model_routes: Vec<AgentModelRouteRequest>,
    pub budget_ledger: BudgetLedger,
    pub budget_policy: BudgetPolicy,
    pub health_policy: AgentClosedLoopExecutionHealthPolicy,
    pub max_parallel_tasks: usize,
    pub evidence: AgentCycleEvidence,
}

impl AgentClosedLoopRuntimeTurnInput {
    pub fn new(
        history: AgentClosedLoopExecutionHistory,
        next_queue: AgentTaskQueue,
        budget_ledger: BudgetLedger,
        evidence: AgentCycleEvidence,
    ) -> Self {
        Self {
            history,
            next_queue,
            completed_task_ids: BTreeSet::new(),
            model_routes: Vec::new(),
            budget_ledger,
            budget_policy: BudgetPolicy::strict(),
            health_policy: AgentClosedLoopExecutionHealthPolicy::default(),
            max_parallel_tasks: 1,
            evidence,
        }
    }

    pub fn with_completed_task_ids(mut self, completed_task_ids: BTreeSet<String>) -> Self {
        self.completed_task_ids = completed_task_ids;
        self
    }

    pub fn with_model_routes(mut self, model_routes: Vec<AgentModelRouteRequest>) -> Self {
        self.model_routes = model_routes;
        self
    }

    pub fn with_budget_policy(mut self, budget_policy: BudgetPolicy) -> Self {
        self.budget_policy = budget_policy;
        self
    }

    pub fn with_health_policy(
        mut self,
        health_policy: AgentClosedLoopExecutionHealthPolicy,
    ) -> Self {
        self.health_policy = health_policy;
        self
    }

    pub fn with_max_parallel_tasks(mut self, max_parallel_tasks: usize) -> Self {
        self.max_parallel_tasks = max_parallel_tasks.max(1);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeTurn {
    pub prepared_cycle: AgentClosedLoopPreparedCycle,
    pub skipped_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeTurn {
    pub fn has_report(&self) -> bool {
        self.prepared_cycle.has_report()
    }

    pub fn report(&self) -> Option<&AgentCycleReport> {
        self.prepared_cycle.report.as_ref()
    }

    pub fn mode(&self) -> AgentClosedLoopNextTurnMode {
        self.prepared_cycle
            .prepared_execution
            .prepared_dispatch
            .turn_plan
            .mode
    }

    pub fn prepared_execution(&self) -> &AgentClosedLoopPreparedExecution {
        &self.prepared_cycle.prepared_execution
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeTurnRunner {
    dispatch_preparer: AgentClosedLoopDispatchPreparer,
    prepared_executor: AgentClosedLoopPreparedExecutor,
    cycle_closer: AgentClosedLoopPreparedCycleCloser,
}

impl AgentClosedLoopRuntimeTurnRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run<E>(
        &self,
        input: AgentClosedLoopRuntimeTurnInput,
        engine: &mut E,
    ) -> AgentClosedLoopRuntimeTurn
    where
        E: EnginePort,
        E::Error: ToString,
    {
        let turn_plan = input
            .history
            .next_turn_plan(input.next_queue, input.health_policy);
        let prepared_dispatch = self.dispatch_preparer.prepare(
            turn_plan,
            &input.completed_task_ids,
            input.budget_ledger,
            &input.budget_policy,
            input.max_parallel_tasks,
        );
        let prepared_execution = if input.model_routes.is_empty() {
            self.prepared_executor.execute(prepared_dispatch, engine)
        } else {
            self.prepared_executor
                .execute_routed(prepared_dispatch, engine, &input.model_routes)
        };
        let prepared_cycle = self.cycle_closer.close(prepared_execution, input.evidence);
        let skipped_reasons = collect_skipped_reasons(&prepared_cycle);
        let telemetry = runtime_turn_telemetry(&prepared_cycle, &skipped_reasons);

        AgentClosedLoopRuntimeTurn {
            prepared_cycle,
            skipped_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeBusinessInput {
    pub run_id: String,
    pub ledger: AgentCycleLedger,
    pub report_evidence: AgentReportEvidence,
}

impl AgentClosedLoopRuntimeBusinessInput {
    pub fn new(
        run_id: impl Into<String>,
        ledger: AgentCycleLedger,
        report_evidence: AgentReportEvidence,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            ledger,
            report_evidence,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeBusinessTurn {
    pub runtime_turn: AgentClosedLoopRuntimeTurn,
    pub handoff: Option<AgentCycleHandoff>,
    pub memory_submission: Option<MemorySubmissionReport>,
    pub step: Option<AgentClosedLoopStep>,
    pub skipped_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeBusinessTurn {
    pub fn has_step(&self) -> bool {
        self.step.is_some()
    }

    pub fn step(&self) -> Option<&AgentClosedLoopStep> {
        self.step.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct AgentClosedLoopRuntimeBusinessTurnCloser {
    memory_submitter: MemoryHandoffSubmitter,
    stepper: AgentClosedLoopStepper,
}

impl Default for AgentClosedLoopRuntimeBusinessTurnCloser {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentClosedLoopRuntimeBusinessTurnCloser {
    pub fn new() -> Self {
        Self {
            memory_submitter: MemoryHandoffSubmitter::new(),
            stepper: AgentClosedLoopStepper::new(),
        }
    }

    pub fn close<P>(
        &self,
        runtime_turn: AgentClosedLoopRuntimeTurn,
        input: AgentClosedLoopRuntimeBusinessInput,
        memory: &mut P,
    ) -> AgentClosedLoopRuntimeBusinessTurn
    where
        P: MemoryPort,
        P::Error: ToString,
    {
        let Some(report) = runtime_turn.report().cloned() else {
            let skipped_reasons = if runtime_turn.skipped_reasons.is_empty() {
                vec!["runtime_turn_report_missing".to_owned()]
            } else {
                runtime_turn.skipped_reasons.clone()
            };
            let telemetry = business_turn_telemetry(&runtime_turn, false, None, &skipped_reasons);
            return AgentClosedLoopRuntimeBusinessTurn {
                runtime_turn,
                handoff: None,
                memory_submission: None,
                step: None,
                skipped_reasons,
                telemetry,
            };
        };

        let handoff = AgentCycleHandoff::from_report(&report);
        let memory_submission = self.memory_submitter.submit(&handoff, memory);
        let step_input = AgentClosedLoopStepInput::new(
            input.run_id,
            report,
            handoff.clone(),
            input.report_evidence,
            Some(memory_submission.clone()),
        );
        let step = self.stepper.close(&input.ledger, step_input);
        let telemetry = business_turn_telemetry(&runtime_turn, true, Some(&memory_submission), &[]);

        AgentClosedLoopRuntimeBusinessTurn {
            runtime_turn,
            handoff: Some(handoff),
            memory_submission: Some(memory_submission),
            step: Some(step),
            skipped_reasons: Vec::new(),
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceCommandRequest {
    pub business_turn: AgentClosedLoopRuntimeBusinessTurn,
    pub command_plan: Option<AgentServiceCommandPlan>,
    pub skipped_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceCommandRequest {
    pub fn has_commands(&self) -> bool {
        self.command_plan
            .as_ref()
            .is_some_and(|plan| !plan.commands.is_empty())
    }

    pub fn command_kinds(&self) -> Vec<&'static str> {
        self.command_plan
            .as_ref()
            .map(AgentServiceCommandPlan::command_kinds)
            .unwrap_or_default()
    }

    pub fn gate(&self) -> AgentClosedLoopRuntimeServiceCommandGate {
        AgentClosedLoopRuntimeServiceCommandGate::from_request(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceCommandPlanner {
    command_planner: AgentServiceCommandPlanner,
}

impl AgentClosedLoopRuntimeServiceCommandPlanner {
    pub fn new() -> Self {
        Self {
            command_planner: AgentServiceCommandPlanner::new(),
        }
    }

    pub fn plan(
        &self,
        business_turn: AgentClosedLoopRuntimeBusinessTurn,
    ) -> AgentClosedLoopRuntimeServiceCommandRequest {
        let Some(step) = business_turn.step.as_ref() else {
            let skipped_reasons = if business_turn.skipped_reasons.is_empty() {
                vec!["business_turn_step_missing".to_owned()]
            } else {
                business_turn.skipped_reasons.clone()
            };
            let telemetry = service_command_request_telemetry(None, &skipped_reasons);
            return AgentClosedLoopRuntimeServiceCommandRequest {
                business_turn,
                command_plan: None,
                skipped_reasons,
                telemetry,
            };
        };

        let command_plan = self.command_planner.plan(&step.business_plan);
        let telemetry = service_command_request_telemetry(Some(&command_plan), &[]);

        AgentClosedLoopRuntimeServiceCommandRequest {
            business_turn,
            command_plan: Some(command_plan),
            skipped_reasons: Vec::new(),
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClosedLoopRuntimeServiceCommandGateEntry {
    pub command_kind: String,
    pub side_effect: SideEffectKind,
    pub allowed: bool,
    pub reason: String,
}

impl AgentClosedLoopRuntimeServiceCommandGateEntry {
    fn allow(
        command: &AgentServiceCommand,
        side_effect: SideEffectKind,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            command_kind: command.kind().to_owned(),
            side_effect,
            allowed: true,
            reason: reason.into(),
        }
    }

    fn block(
        command: &AgentServiceCommand,
        side_effect: SideEffectKind,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            command_kind: command.kind().to_owned(),
            side_effect,
            allowed: false,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClosedLoopRuntimeServiceCommandGate {
    pub command_count: usize,
    pub entries: Vec<AgentClosedLoopRuntimeServiceCommandGateEntry>,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceCommandGate {
    pub fn from_request(request: &AgentClosedLoopRuntimeServiceCommandRequest) -> Self {
        let Some(command_plan) = request.command_plan.as_ref() else {
            let blocked_reasons = if request.skipped_reasons.is_empty() {
                vec!["service_command_plan_missing".to_owned()]
            } else {
                request.skipped_reasons.clone()
            };
            return Self {
                command_count: 0,
                entries: Vec::new(),
                telemetry: service_command_gate_telemetry(0, true, &blocked_reasons),
                blocked_reasons,
            };
        };

        let entries = command_plan
            .commands
            .iter()
            .map(|command| gate_service_command(command, request.business_turn.step.as_ref()))
            .collect::<Vec<_>>();
        let blocked_reasons = entries
            .iter()
            .filter(|entry| !entry.allowed)
            .map(|entry| {
                format!(
                    "service_command_gate_blocked={}:{}",
                    entry.command_kind, entry.reason
                )
            })
            .collect::<Vec<_>>();
        let telemetry =
            service_command_gate_telemetry(command_plan.commands.len(), false, &blocked_reasons);

        Self {
            command_count: command_plan.commands.len(),
            entries,
            blocked_reasons,
            telemetry,
        }
    }

    pub fn is_allowed(&self) -> bool {
        self.blocked_reasons.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceRequestInput {
    pub runtime_input: AgentClosedLoopRuntimeTurnInput,
    pub business_input: AgentClosedLoopRuntimeBusinessInput,
}

impl AgentClosedLoopRuntimeServiceRequestInput {
    pub fn new(
        runtime_input: AgentClosedLoopRuntimeTurnInput,
        business_input: AgentClosedLoopRuntimeBusinessInput,
    ) -> Self {
        Self {
            runtime_input,
            business_input,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceRequest {
    pub command_request: AgentClosedLoopRuntimeServiceCommandRequest,
    pub prior_history: AgentClosedLoopExecutionHistory,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceRequest {
    pub fn has_commands(&self) -> bool {
        self.command_request.has_commands()
    }

    pub fn command_kinds(&self) -> Vec<&'static str> {
        self.command_request.command_kinds()
    }

    pub fn skipped_reasons(&self) -> &[String] {
        &self.command_request.skipped_reasons
    }

    pub fn command_gate(&self) -> AgentClosedLoopRuntimeServiceCommandGate {
        self.command_request.gate()
    }

    pub fn into_dispatch(self) -> AgentClosedLoopRuntimeServiceDispatch {
        AgentClosedLoopRuntimeServiceDispatch::from_request(self)
    }

    pub fn close_with_receipts(
        self,
        receipts: Vec<AgentServiceCommandReceipt>,
        continuation_input: AgentClosedLoopRuntimeContinuationInput,
    ) -> AgentClosedLoopRuntimeServiceOutcome {
        let Self {
            command_request,
            prior_history,
            ..
        } = self;
        AgentClosedLoopRuntimeServiceOutcomePlanner::new().close(
            command_request,
            &prior_history,
            receipts,
            continuation_input,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceDispatch {
    pub request: AgentClosedLoopRuntimeServiceRequest,
    pub command_gate: AgentClosedLoopRuntimeServiceCommandGate,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceDispatch {
    pub fn from_request(request: AgentClosedLoopRuntimeServiceRequest) -> Self {
        let command_gate = request.command_gate();
        let telemetry = service_dispatch_telemetry(&request, &command_gate);

        Self {
            request,
            command_gate,
            telemetry,
        }
    }

    pub fn is_executable(&self) -> bool {
        self.request.has_commands() && self.command_gate.is_allowed()
    }

    pub fn command_plan(&self) -> Option<&AgentServiceCommandPlan> {
        self.is_executable()
            .then(|| self.request.command_request.command_plan.as_ref())
            .flatten()
    }

    pub fn summary(&self) -> AgentClosedLoopRuntimeServiceDispatchSummary {
        AgentClosedLoopRuntimeServiceDispatchSummary::from_dispatch(self)
    }

    pub fn intake_receipts(
        &self,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceReceiptIntake {
        AgentClosedLoopRuntimeServiceReceiptIntake::from_dispatch(self, receipts)
    }

    pub fn close_with_receipts(
        self,
        receipts: Vec<AgentServiceCommandReceipt>,
        continuation_input: AgentClosedLoopRuntimeContinuationInput,
    ) -> AgentClosedLoopRuntimeServiceOutcome {
        self.request
            .close_with_receipts(receipts, continuation_input)
    }

    pub fn close_with_intake(
        self,
        receipts: Vec<AgentServiceCommandReceipt>,
        continuation_input: AgentClosedLoopRuntimeContinuationInput,
    ) -> AgentClosedLoopRuntimeServiceDispatchOutcome {
        let intake = self.intake_receipts(receipts);
        let outcome = if intake.can_close_outcome() {
            Some(
                self.request
                    .clone()
                    .close_with_receipts(intake.accepted_receipts.clone(), continuation_input),
            )
        } else {
            None
        };
        let repair_plan =
            AgentClosedLoopRuntimeServiceIntakeRepairPlan::from_dispatch_outcome_parts(
                &self,
                &intake,
                outcome.is_some(),
            );
        let telemetry = service_dispatch_outcome_telemetry(&intake, outcome.as_ref(), &repair_plan);

        AgentClosedLoopRuntimeServiceDispatchOutcome {
            dispatch: self,
            intake,
            outcome,
            repair_plan,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClosedLoopRuntimeServiceDispatchSummary {
    pub executable: bool,
    pub command_count: usize,
    pub command_gate_allowed: bool,
    pub side_effect_gate_count: usize,
    pub blocked_side_effect_gate_count: usize,
    pub command_kinds: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceDispatchSummary {
    pub fn from_dispatch(dispatch: &AgentClosedLoopRuntimeServiceDispatch) -> Self {
        let executable = dispatch.is_executable();
        let command_gate_allowed = dispatch.command_gate.is_allowed();
        let side_effect_gate_count = dispatch.command_gate.entries.len();
        let blocked_side_effect_gate_count = dispatch
            .command_gate
            .entries
            .iter()
            .filter(|entry| !entry.allowed)
            .count();
        let command_kinds = dispatch
            .request
            .command_request
            .command_plan
            .as_ref()
            .map(|plan| {
                plan.commands
                    .iter()
                    .map(|command| command.kind().to_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let command_count = command_kinds.len();
        let blocked_reasons = dispatch.command_gate.blocked_reasons.clone();
        let telemetry = service_dispatch_summary_telemetry(
            executable,
            command_count,
            command_gate_allowed,
            side_effect_gate_count,
            blocked_side_effect_gate_count,
            &command_kinds,
            &blocked_reasons,
        );

        Self {
            executable,
            command_count,
            command_gate_allowed,
            side_effect_gate_count,
            blocked_side_effect_gate_count,
            command_kinds,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentClosedLoopRuntimeServiceDispatchSummaryHistory {
    summaries: Vec<AgentClosedLoopRuntimeServiceDispatchSummary>,
}

impl AgentClosedLoopRuntimeServiceDispatchSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentClosedLoopRuntimeServiceDispatchSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentClosedLoopRuntimeServiceDispatchSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentClosedLoopRuntimeServiceDispatchSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentClosedLoopRuntimeServiceDispatchSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentClosedLoopRuntimeServiceDispatchDashboard {
        AgentClosedLoopRuntimeServiceDispatchDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceDispatchHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceDispatchHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceDispatchDashboard {
    pub total_dispatches: usize,
    pub executable_dispatches: usize,
    pub blocked_dispatches: usize,
    pub command_gate_allowed_dispatches: usize,
    pub command_count: usize,
    pub side_effect_gate_count: usize,
    pub blocked_side_effect_gate_count: usize,
    pub blocked_reason_count: usize,
    pub executable_rate: f32,
    pub blocked_side_effect_gate_rate: f32,
    pub latest_executable: Option<bool>,
    pub latest_command_gate_allowed: Option<bool>,
    pub latest_command_kinds: Vec<String>,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceDispatchDashboard {
    pub fn from_summaries(summaries: &[AgentClosedLoopRuntimeServiceDispatchSummary]) -> Self {
        let total_dispatches = summaries.len();
        let executable_dispatches = summaries
            .iter()
            .filter(|summary| summary.executable)
            .count();
        let blocked_dispatches = total_dispatches.saturating_sub(executable_dispatches);
        let command_gate_allowed_dispatches = summaries
            .iter()
            .filter(|summary| summary.command_gate_allowed)
            .count();
        let command_count = summaries
            .iter()
            .map(|summary| summary.command_count)
            .sum::<usize>();
        let side_effect_gate_count = summaries
            .iter()
            .map(|summary| summary.side_effect_gate_count)
            .sum::<usize>();
        let blocked_side_effect_gate_count = summaries
            .iter()
            .map(|summary| summary.blocked_side_effect_gate_count)
            .sum::<usize>();
        let blocked_reason_count = summaries
            .iter()
            .map(|summary| summary.blocked_reasons.len())
            .sum::<usize>();
        let executable_rate = service_run_rate(executable_dispatches, total_dispatches);
        let blocked_side_effect_gate_rate =
            service_run_rate(blocked_side_effect_gate_count, side_effect_gate_count);
        let latest = summaries.last();
        let latest_executable = latest.map(|summary| summary.executable);
        let latest_command_gate_allowed = latest.map(|summary| summary.command_gate_allowed);
        let latest_command_kinds = latest
            .map(|summary| summary.command_kinds.clone())
            .unwrap_or_default();
        let latest_blocked_reasons = latest
            .map(|summary| summary.blocked_reasons.clone())
            .unwrap_or_default();
        let telemetry = service_dispatch_dashboard_telemetry(
            total_dispatches,
            executable_dispatches,
            blocked_dispatches,
            command_gate_allowed_dispatches,
            command_count,
            side_effect_gate_count,
            blocked_side_effect_gate_count,
            blocked_reason_count,
            executable_rate,
            blocked_side_effect_gate_rate,
        );

        Self {
            total_dispatches,
            executable_dispatches,
            blocked_dispatches,
            command_gate_allowed_dispatches,
            command_count,
            side_effect_gate_count,
            blocked_side_effect_gate_count,
            blocked_reason_count,
            executable_rate,
            blocked_side_effect_gate_rate,
            latest_executable,
            latest_command_gate_allowed,
            latest_command_kinds,
            latest_blocked_reasons,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_dispatches == 0
    }

    pub fn is_clean(&self) -> bool {
        self.total_dispatches > 0 && self.blocked_dispatches == 0
    }

    pub fn has_blocked_side_effects(&self) -> bool {
        self.blocked_side_effect_gate_count > 0 || self.blocked_reason_count > 0
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceDispatchHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceDispatchHealth {
        AgentClosedLoopRuntimeServiceDispatchHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceDispatchHealthPolicy {
    pub minimum_executable_rate: f32,
    pub maximum_blocked_dispatches: usize,
    pub maximum_blocked_side_effect_gates: usize,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentClosedLoopRuntimeServiceDispatchHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_executable_rate: 0.67,
            maximum_blocked_dispatches: 0,
            maximum_blocked_side_effect_gates: 0,
            maximum_blocked_reasons: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceDispatchHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentClosedLoopRuntimeServiceDispatchDashboard,
}

impl AgentClosedLoopRuntimeServiceDispatchHealth {
    pub fn from_dashboard(
        dashboard: AgentClosedLoopRuntimeServiceDispatchDashboard,
        policy: AgentClosedLoopRuntimeServiceDispatchHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("runtime_service_dispatch_history_empty".to_owned());
        } else if dashboard.executable_rate < policy.minimum_executable_rate {
            watch_reasons.push(format!(
                "runtime_service_dispatch_executable_rate={:.3}<{}",
                dashboard.executable_rate, policy.minimum_executable_rate
            ));
        }

        if dashboard.blocked_dispatches > policy.maximum_blocked_dispatches {
            repair_reasons.push(format!(
                "runtime_service_dispatch_blocked_dispatches={}>{}",
                dashboard.blocked_dispatches, policy.maximum_blocked_dispatches
            ));
        }

        if dashboard.blocked_side_effect_gate_count > policy.maximum_blocked_side_effect_gates {
            repair_reasons.push(format!(
                "runtime_service_dispatch_blocked_side_effect_gates={}>{}",
                dashboard.blocked_side_effect_gate_count, policy.maximum_blocked_side_effect_gates
            ));
        }

        if dashboard.blocked_reason_count > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "runtime_service_dispatch_blocked_reasons={}>{}",
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
pub struct AgentClosedLoopRuntimeServiceDispatchSummaryHistoryRecord {
    pub history: AgentClosedLoopRuntimeServiceDispatchSummaryHistory,
    pub appended_summary: AgentClosedLoopRuntimeServiceDispatchSummary,
    pub dashboard: AgentClosedLoopRuntimeServiceDispatchDashboard,
    pub health: AgentClosedLoopRuntimeServiceDispatchHealth,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceDispatchSummaryHistoryRecord {
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
pub struct AgentClosedLoopRuntimeServiceDispatchSummaryHistoryRecorder;

impl AgentClosedLoopRuntimeServiceDispatchSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary(
        &self,
        mut history: AgentClosedLoopRuntimeServiceDispatchSummaryHistory,
        summary: AgentClosedLoopRuntimeServiceDispatchSummary,
        policy: AgentClosedLoopRuntimeServiceDispatchHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceDispatchSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_dispatch_history_record_telemetry(&dashboard, &health);

        AgentClosedLoopRuntimeServiceDispatchSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_dispatch(
        &self,
        history: AgentClosedLoopRuntimeServiceDispatchSummaryHistory,
        dispatch: &AgentClosedLoopRuntimeServiceDispatch,
        policy: AgentClosedLoopRuntimeServiceDispatchHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceDispatchSummaryHistoryRecord {
        self.record_summary(history, dispatch.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClosedLoopRuntimeServiceRejectedReceipt {
    pub receipt: AgentServiceCommandReceipt,
    pub reason: String,
}

impl AgentClosedLoopRuntimeServiceRejectedReceipt {
    fn new(receipt: AgentServiceCommandReceipt, reason: impl Into<String>) -> Self {
        Self {
            receipt,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClosedLoopRuntimeServiceReceiptIntake {
    pub executable: bool,
    pub expected_command_kinds: Vec<String>,
    pub accepted_receipts: Vec<AgentServiceCommandReceipt>,
    pub rejected_receipts: Vec<AgentClosedLoopRuntimeServiceRejectedReceipt>,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceReceiptIntake {
    pub fn from_dispatch(
        dispatch: &AgentClosedLoopRuntimeServiceDispatch,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> Self {
        let expected_command_kinds = dispatch
            .request
            .command_request
            .command_plan
            .as_ref()
            .map(|plan| {
                plan.commands
                    .iter()
                    .map(|command| command.kind().to_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !dispatch.is_executable() {
            let rejected_receipts = receipts
                .into_iter()
                .map(|receipt| {
                    AgentClosedLoopRuntimeServiceRejectedReceipt::new(
                        receipt,
                        "service_dispatch_not_executable",
                    )
                })
                .collect::<Vec<_>>();
            let blocked_reasons = if dispatch.command_gate.blocked_reasons.is_empty() {
                vec!["service_dispatch_not_executable".to_owned()]
            } else {
                dispatch.command_gate.blocked_reasons.clone()
            };
            let telemetry = service_receipt_intake_telemetry(
                false,
                expected_command_kinds.len(),
                0,
                rejected_receipts.len(),
                &blocked_reasons,
            );
            return Self {
                executable: false,
                expected_command_kinds,
                accepted_receipts: Vec::new(),
                rejected_receipts,
                blocked_reasons,
                telemetry,
            };
        }

        let mut remaining = expected_command_kinds.clone();
        let mut accepted_receipts = Vec::new();
        let mut rejected_receipts = Vec::new();

        for receipt in receipts {
            if let Some(index) = remaining
                .iter()
                .position(|kind| kind == &receipt.command_kind)
            {
                remaining.remove(index);
                accepted_receipts.push(receipt);
            } else {
                rejected_receipts.push(AgentClosedLoopRuntimeServiceRejectedReceipt::new(
                    receipt,
                    "receipt_command_unexpected_or_duplicate",
                ));
            }
        }

        let blocked_reasons = rejected_receipts
            .iter()
            .map(|rejection| {
                format!(
                    "service_receipt_rejected={}:{}",
                    rejection.receipt.command_kind, rejection.reason
                )
            })
            .collect::<Vec<_>>();
        let telemetry = service_receipt_intake_telemetry(
            true,
            expected_command_kinds.len(),
            accepted_receipts.len(),
            rejected_receipts.len(),
            &blocked_reasons,
        );

        Self {
            executable: true,
            expected_command_kinds,
            accepted_receipts,
            rejected_receipts,
            blocked_reasons,
            telemetry,
        }
    }

    pub fn can_close_outcome(&self) -> bool {
        self.executable && self.rejected_receipts.is_empty() && self.blocked_reasons.is_empty()
    }

    pub fn summary(&self) -> AgentClosedLoopRuntimeServiceReceiptIntakeSummary {
        AgentClosedLoopRuntimeServiceReceiptIntakeSummary::from_intake(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClosedLoopRuntimeServiceReceiptIntakeSummary {
    pub executable: bool,
    pub expected_receipts: usize,
    pub accepted_receipts: usize,
    pub rejected_receipts: usize,
    pub clean: bool,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceReceiptIntakeSummary {
    pub fn from_intake(intake: &AgentClosedLoopRuntimeServiceReceiptIntake) -> Self {
        let executable = intake.executable;
        let expected_receipts = intake.expected_command_kinds.len();
        let accepted_receipts = intake.accepted_receipts.len();
        let rejected_receipts = intake.rejected_receipts.len();
        let clean = intake.can_close_outcome();
        let blocked_reasons = intake.blocked_reasons.clone();
        let telemetry = service_receipt_intake_summary_telemetry(
            executable,
            expected_receipts,
            accepted_receipts,
            rejected_receipts,
            clean,
            blocked_reasons.len(),
        );

        Self {
            executable,
            expected_receipts,
            accepted_receipts,
            rejected_receipts,
            clean,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory {
    summaries: Vec<AgentClosedLoopRuntimeServiceReceiptIntakeSummary>,
}

impl AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentClosedLoopRuntimeServiceReceiptIntakeSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentClosedLoopRuntimeServiceReceiptIntakeSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentClosedLoopRuntimeServiceReceiptIntakeSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentClosedLoopRuntimeServiceReceiptIntakeSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentClosedLoopRuntimeServiceReceiptIntakeDashboard {
        AgentClosedLoopRuntimeServiceReceiptIntakeDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceReceiptIntakeHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceReceiptIntakeDashboard {
    pub total_intakes: usize,
    pub clean_intakes: usize,
    pub dirty_intakes: usize,
    pub executable_intakes: usize,
    pub non_executable_intakes: usize,
    pub expected_receipt_count: usize,
    pub accepted_receipt_count: usize,
    pub rejected_receipt_count: usize,
    pub blocked_reason_count: usize,
    pub clean_rate: f32,
    pub rejection_rate: f32,
    pub latest_clean: Option<bool>,
    pub latest_executable: Option<bool>,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceReceiptIntakeDashboard {
    pub fn from_summaries(summaries: &[AgentClosedLoopRuntimeServiceReceiptIntakeSummary]) -> Self {
        let total_intakes = summaries.len();
        let clean_intakes = summaries.iter().filter(|summary| summary.clean).count();
        let dirty_intakes = total_intakes.saturating_sub(clean_intakes);
        let executable_intakes = summaries
            .iter()
            .filter(|summary| summary.executable)
            .count();
        let non_executable_intakes = total_intakes.saturating_sub(executable_intakes);
        let expected_receipt_count = summaries
            .iter()
            .map(|summary| summary.expected_receipts)
            .sum::<usize>();
        let accepted_receipt_count = summaries
            .iter()
            .map(|summary| summary.accepted_receipts)
            .sum::<usize>();
        let rejected_receipt_count = summaries
            .iter()
            .map(|summary| summary.rejected_receipts)
            .sum::<usize>();
        let blocked_reason_count = summaries
            .iter()
            .map(|summary| summary.blocked_reasons.len())
            .sum::<usize>();
        let clean_rate = service_run_rate(clean_intakes, total_intakes);
        let rejection_rate = service_run_rate(rejected_receipt_count, expected_receipt_count);
        let latest = summaries.last();
        let latest_clean = latest.map(|summary| summary.clean);
        let latest_executable = latest.map(|summary| summary.executable);
        let latest_blocked_reasons = latest
            .map(|summary| summary.blocked_reasons.clone())
            .unwrap_or_default();
        let telemetry = service_receipt_intake_dashboard_telemetry(
            total_intakes,
            clean_intakes,
            dirty_intakes,
            executable_intakes,
            non_executable_intakes,
            expected_receipt_count,
            accepted_receipt_count,
            rejected_receipt_count,
            blocked_reason_count,
            clean_rate,
            rejection_rate,
        );

        Self {
            total_intakes,
            clean_intakes,
            dirty_intakes,
            executable_intakes,
            non_executable_intakes,
            expected_receipt_count,
            accepted_receipt_count,
            rejected_receipt_count,
            blocked_reason_count,
            clean_rate,
            rejection_rate,
            latest_clean,
            latest_executable,
            latest_blocked_reasons,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_intakes == 0
    }

    pub fn is_clean(&self) -> bool {
        self.total_intakes > 0 && self.dirty_intakes == 0
    }

    pub fn has_receipt_drift(&self) -> bool {
        self.rejected_receipt_count > 0 || self.blocked_reason_count > 0
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceReceiptIntakeHealth {
        AgentClosedLoopRuntimeServiceReceiptIntakeHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy {
    pub minimum_clean_rate: f32,
    pub maximum_non_executable_intakes: usize,
    pub maximum_rejected_receipts: usize,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_clean_rate: 0.67,
            maximum_non_executable_intakes: 0,
            maximum_rejected_receipts: 0,
            maximum_blocked_reasons: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceReceiptIntakeHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentClosedLoopRuntimeServiceReceiptIntakeDashboard,
}

impl AgentClosedLoopRuntimeServiceReceiptIntakeHealth {
    pub fn from_dashboard(
        dashboard: AgentClosedLoopRuntimeServiceReceiptIntakeDashboard,
        policy: AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("runtime_service_receipt_intake_history_empty".to_owned());
        } else if dashboard.clean_rate < policy.minimum_clean_rate {
            watch_reasons.push(format!(
                "runtime_service_receipt_intake_clean_rate={:.3}<{}",
                dashboard.clean_rate, policy.minimum_clean_rate
            ));
        }

        if dashboard.non_executable_intakes > policy.maximum_non_executable_intakes {
            repair_reasons.push(format!(
                "runtime_service_receipt_intake_non_executable={}>{}",
                dashboard.non_executable_intakes, policy.maximum_non_executable_intakes
            ));
        }

        if dashboard.rejected_receipt_count > policy.maximum_rejected_receipts {
            repair_reasons.push(format!(
                "runtime_service_receipt_intake_rejected_receipts={}>{}",
                dashboard.rejected_receipt_count, policy.maximum_rejected_receipts
            ));
        }

        if dashboard.blocked_reason_count > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "runtime_service_receipt_intake_blocked_reasons={}>{}",
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
pub struct AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistoryRecord {
    pub history: AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory,
    pub appended_summary: AgentClosedLoopRuntimeServiceReceiptIntakeSummary,
    pub dashboard: AgentClosedLoopRuntimeServiceReceiptIntakeDashboard,
    pub health: AgentClosedLoopRuntimeServiceReceiptIntakeHealth,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistoryRecord {
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
pub struct AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistoryRecorder;

impl AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary(
        &self,
        mut history: AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory,
        summary: AgentClosedLoopRuntimeServiceReceiptIntakeSummary,
        policy: AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_receipt_intake_history_record_telemetry(&dashboard, &health);

        AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_intake(
        &self,
        history: AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory,
        intake: &AgentClosedLoopRuntimeServiceReceiptIntake,
        policy: AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistoryRecord {
        self.record_summary(history, intake.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceDispatchOutcome {
    pub dispatch: AgentClosedLoopRuntimeServiceDispatch,
    pub intake: AgentClosedLoopRuntimeServiceReceiptIntake,
    pub outcome: Option<AgentClosedLoopRuntimeServiceOutcome>,
    pub repair_plan: AgentClosedLoopRuntimeServiceIntakeRepairPlan,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceDispatchOutcome {
    pub fn has_outcome(&self) -> bool {
        self.outcome.is_some()
    }

    pub fn blocked_reasons(&self) -> &[String] {
        &self.intake.blocked_reasons
    }

    pub fn repair_queue(&self) -> AgentTaskQueue {
        self.repair_plan.next_queue.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceDispatchContinuation {
    pub dispatch_outcome: AgentClosedLoopRuntimeServiceDispatchOutcome,
    pub dashboard: AgentClosedLoopExecutionDashboard,
    pub health: AgentClosedLoopExecutionHealth,
    pub next_runtime_input: AgentClosedLoopRuntimeTurnInput,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceDispatchContinuation {
    pub fn next_queue(&self) -> AgentTaskQueue {
        self.next_runtime_input.next_queue.clone()
    }

    pub fn has_closed_outcome(&self) -> bool {
        self.dispatch_outcome.has_outcome()
    }

    pub fn summary(&self) -> AgentClosedLoopRuntimeServiceDispatchContinuationSummary {
        AgentClosedLoopRuntimeServiceDispatchContinuationSummary::from_continuation(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClosedLoopRuntimeServiceDispatchContinuationSummary {
    pub outcome_closed: bool,
    pub intake_clean: bool,
    pub repair_task_count: usize,
    pub health_status: AgentClosedLoopExecutionHealthStatus,
    pub next_queue_tasks: usize,
    pub immediate_ready_tasks: usize,
    pub history_runs: usize,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceDispatchContinuationSummary {
    pub fn from_continuation(
        continuation: &AgentClosedLoopRuntimeServiceDispatchContinuation,
    ) -> Self {
        let outcome_closed = continuation.has_closed_outcome();
        let intake_clean = continuation.dispatch_outcome.intake.can_close_outcome();
        let repair_task_count = continuation.dispatch_outcome.repair_plan.tasks.len();
        let health_status = continuation.health.status;
        let next_queue_tasks = continuation
            .next_runtime_input
            .next_queue
            .next_queue_tasks()
            .len();
        let immediate_ready_tasks = continuation
            .next_runtime_input
            .next_queue
            .immediate_ready_tasks()
            .len();
        let history_runs = continuation.next_runtime_input.history.len();
        let telemetry = service_dispatch_continuation_summary_telemetry(
            outcome_closed,
            intake_clean,
            repair_task_count,
            health_status,
            next_queue_tasks,
            immediate_ready_tasks,
            history_runs,
        );

        Self {
            outcome_closed,
            intake_clean,
            repair_task_count,
            health_status,
            next_queue_tasks,
            immediate_ready_tasks,
            history_runs,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistory {
    summaries: Vec<AgentClosedLoopRuntimeServiceDispatchContinuationSummary>,
}

impl AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentClosedLoopRuntimeServiceDispatchContinuationSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentClosedLoopRuntimeServiceDispatchContinuationSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentClosedLoopRuntimeServiceDispatchContinuationSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentClosedLoopRuntimeServiceDispatchContinuationSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentClosedLoopRuntimeServiceDispatchContinuationDashboard {
        AgentClosedLoopRuntimeServiceDispatchContinuationDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceDispatchContinuationHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceDispatchContinuationDashboard {
    pub total_continuations: usize,
    pub closed_continuations: usize,
    pub blocked_continuations: usize,
    pub intake_clean_continuations: usize,
    pub intake_dirty_continuations: usize,
    pub stable_health_continuations: usize,
    pub watch_health_continuations: usize,
    pub repair_health_continuations: usize,
    pub repair_task_count: usize,
    pub total_next_queue_tasks: usize,
    pub total_immediate_ready_tasks: usize,
    pub latest_history_runs: usize,
    pub closed_rate: f32,
    pub intake_clean_rate: f32,
    pub latest_outcome_closed: Option<bool>,
    pub latest_intake_clean: Option<bool>,
    pub latest_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceDispatchContinuationDashboard {
    pub fn from_summaries(
        summaries: &[AgentClosedLoopRuntimeServiceDispatchContinuationSummary],
    ) -> Self {
        let total_continuations = summaries.len();
        let closed_continuations = summaries
            .iter()
            .filter(|summary| summary.outcome_closed)
            .count();
        let blocked_continuations = total_continuations.saturating_sub(closed_continuations);
        let intake_clean_continuations = summaries
            .iter()
            .filter(|summary| summary.intake_clean)
            .count();
        let intake_dirty_continuations =
            total_continuations.saturating_sub(intake_clean_continuations);
        let stable_health_continuations = summaries
            .iter()
            .filter(|summary| summary.health_status == AgentClosedLoopExecutionHealthStatus::Stable)
            .count();
        let watch_health_continuations = summaries
            .iter()
            .filter(|summary| summary.health_status == AgentClosedLoopExecutionHealthStatus::Watch)
            .count();
        let repair_health_continuations = summaries
            .iter()
            .filter(|summary| summary.health_status == AgentClosedLoopExecutionHealthStatus::Repair)
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_task_count)
            .sum::<usize>();
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let total_immediate_ready_tasks = summaries
            .iter()
            .map(|summary| summary.immediate_ready_tasks)
            .sum::<usize>();
        let latest = summaries.last();
        let latest_history_runs = latest
            .map(|summary| summary.history_runs)
            .unwrap_or_default();
        let closed_rate = service_run_rate(closed_continuations, total_continuations);
        let intake_clean_rate = service_run_rate(intake_clean_continuations, total_continuations);
        let latest_outcome_closed = latest.map(|summary| summary.outcome_closed);
        let latest_intake_clean = latest.map(|summary| summary.intake_clean);
        let latest_health_status = latest.map(|summary| summary.health_status);
        let telemetry = service_dispatch_continuation_dashboard_telemetry(
            total_continuations,
            closed_continuations,
            blocked_continuations,
            intake_clean_continuations,
            intake_dirty_continuations,
            repair_health_continuations,
            repair_task_count,
            total_next_queue_tasks,
            total_immediate_ready_tasks,
            latest_history_runs,
            closed_rate,
            intake_clean_rate,
        );

        Self {
            total_continuations,
            closed_continuations,
            blocked_continuations,
            intake_clean_continuations,
            intake_dirty_continuations,
            stable_health_continuations,
            watch_health_continuations,
            repair_health_continuations,
            repair_task_count,
            total_next_queue_tasks,
            total_immediate_ready_tasks,
            latest_history_runs,
            closed_rate,
            intake_clean_rate,
            latest_outcome_closed,
            latest_intake_clean,
            latest_health_status,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_continuations == 0
    }

    pub fn is_clean(&self) -> bool {
        self.total_continuations > 0
            && self.blocked_continuations == 0
            && self.intake_dirty_continuations == 0
            && self.repair_task_count == 0
    }

    pub fn has_repair_pressure(&self) -> bool {
        self.blocked_continuations > 0
            || self.intake_dirty_continuations > 0
            || self.repair_task_count > 0
            || self.repair_health_continuations > 0
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceDispatchContinuationHealth {
        AgentClosedLoopRuntimeServiceDispatchContinuationHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy {
    pub minimum_closed_rate: f32,
    pub minimum_intake_clean_rate: f32,
    pub maximum_blocked_continuations: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_repair_health_continuations: usize,
}

impl Default for AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_closed_rate: 0.67,
            minimum_intake_clean_rate: 0.67,
            maximum_blocked_continuations: 0,
            maximum_repair_tasks: 0,
            maximum_repair_health_continuations: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceDispatchContinuationHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentClosedLoopRuntimeServiceDispatchContinuationDashboard,
}

impl AgentClosedLoopRuntimeServiceDispatchContinuationHealth {
    pub fn from_dashboard(
        dashboard: AgentClosedLoopRuntimeServiceDispatchContinuationDashboard,
        policy: AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("runtime_service_dispatch_continuation_history_empty".to_owned());
        } else if dashboard.closed_rate < policy.minimum_closed_rate {
            watch_reasons.push(format!(
                "runtime_service_dispatch_continuation_closed_rate={:.3}<{}",
                dashboard.closed_rate, policy.minimum_closed_rate
            ));
        }

        if !dashboard.is_empty() && dashboard.intake_clean_rate < policy.minimum_intake_clean_rate {
            watch_reasons.push(format!(
                "runtime_service_dispatch_continuation_intake_clean_rate={:.3}<{}",
                dashboard.intake_clean_rate, policy.minimum_intake_clean_rate
            ));
        }

        if dashboard.blocked_continuations > policy.maximum_blocked_continuations {
            repair_reasons.push(format!(
                "runtime_service_dispatch_continuation_blocked={}>{}",
                dashboard.blocked_continuations, policy.maximum_blocked_continuations
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "runtime_service_dispatch_continuation_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.repair_health_continuations > policy.maximum_repair_health_continuations {
            repair_reasons.push(format!(
                "runtime_service_dispatch_continuation_repair_health={}>{}",
                dashboard.repair_health_continuations, policy.maximum_repair_health_continuations
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
pub struct AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistoryRecord {
    pub history: AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistory,
    pub appended_summary: AgentClosedLoopRuntimeServiceDispatchContinuationSummary,
    pub dashboard: AgentClosedLoopRuntimeServiceDispatchContinuationDashboard,
    pub health: AgentClosedLoopRuntimeServiceDispatchContinuationHealth,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistoryRecord {
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
pub struct AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistoryRecorder;

impl AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary(
        &self,
        mut history: AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistory,
        summary: AgentClosedLoopRuntimeServiceDispatchContinuationSummary,
        policy: AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_dispatch_continuation_history_record_telemetry(&dashboard, &health);

        AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_continuation(
        &self,
        history: AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistory,
        continuation: &AgentClosedLoopRuntimeServiceDispatchContinuation,
        policy: AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistoryRecord {
        self.record_summary(history, continuation.summary(), policy)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceDispatchContinuationPlanner;

impl AgentClosedLoopRuntimeServiceDispatchContinuationPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        dispatch_outcome: AgentClosedLoopRuntimeServiceDispatchOutcome,
        input: AgentClosedLoopRuntimeContinuationInput,
    ) -> AgentClosedLoopRuntimeServiceDispatchContinuation {
        if let Some(outcome) = dispatch_outcome.outcome.as_ref() {
            let dashboard = outcome.continuation.dashboard.clone();
            let health = outcome.continuation.health.clone();
            let next_runtime_input = outcome.continuation.next_runtime_input.clone();
            let telemetry = service_dispatch_continuation_telemetry(
                &dispatch_outcome,
                &dashboard,
                &health,
                next_runtime_input.next_queue.len(),
            );

            return AgentClosedLoopRuntimeServiceDispatchContinuation {
                dispatch_outcome,
                dashboard,
                health,
                next_runtime_input,
                telemetry,
            };
        }

        let history = dispatch_outcome.dispatch.request.prior_history.clone();
        let dashboard = history.dashboard();
        let health = history.health(input.health_policy);
        let next_queue = dispatch_outcome.repair_queue();
        let next_runtime_input = AgentClosedLoopRuntimeTurnInput {
            history,
            next_queue,
            completed_task_ids: input.completed_task_ids,
            model_routes: input.model_routes,
            budget_ledger: input.budget_ledger,
            budget_policy: input.budget_policy,
            health_policy: input.health_policy,
            max_parallel_tasks: input.max_parallel_tasks,
            evidence: input.evidence,
        };
        let telemetry = service_dispatch_continuation_telemetry(
            &dispatch_outcome,
            &dashboard,
            &health,
            next_runtime_input.next_queue.len(),
        );

        AgentClosedLoopRuntimeServiceDispatchContinuation {
            dispatch_outcome,
            dashboard,
            health,
            next_runtime_input,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceRunInput {
    pub request_input: AgentClosedLoopRuntimeServiceRequestInput,
    pub receipts: Vec<AgentServiceCommandReceipt>,
    pub continuation_input: AgentClosedLoopRuntimeContinuationInput,
}

impl AgentClosedLoopRuntimeServiceRunInput {
    pub fn new(
        request_input: AgentClosedLoopRuntimeServiceRequestInput,
        receipts: Vec<AgentServiceCommandReceipt>,
        continuation_input: AgentClosedLoopRuntimeContinuationInput,
    ) -> Self {
        Self {
            request_input,
            receipts,
            continuation_input,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceRun {
    pub dispatch_continuation: AgentClosedLoopRuntimeServiceDispatchContinuation,
    pub summary: AgentClosedLoopRuntimeServiceDispatchContinuationSummary,
    pub run_summary: AgentClosedLoopRuntimeServiceRunSummary,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceRun {
    pub fn next_runtime_input(&self) -> &AgentClosedLoopRuntimeTurnInput {
        &self.dispatch_continuation.next_runtime_input
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.dispatch_continuation.next_queue()
    }

    pub fn has_closed_outcome(&self) -> bool {
        self.dispatch_continuation.has_closed_outcome()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentClosedLoopRuntimeServiceRunStatus {
    Closed,
    DispatchBlocked,
    IntakeBlocked,
}

impl AgentClosedLoopRuntimeServiceRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::DispatchBlocked => "dispatch_blocked",
            Self::IntakeBlocked => "intake_blocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClosedLoopRuntimeServiceRunSummary {
    pub status: AgentClosedLoopRuntimeServiceRunStatus,
    pub dispatch_executable: bool,
    pub command_count: usize,
    pub command_gate_allowed: bool,
    pub side_effect_gate_count: usize,
    pub blocked_side_effect_gate_count: usize,
    pub command_kinds: Vec<String>,
    pub gate_blocked_reasons: Vec<String>,
    pub outcome_closed: bool,
    pub intake_clean: bool,
    pub intake_blocked_reasons: Vec<String>,
    pub repair_task_count: usize,
    pub health_status: AgentClosedLoopExecutionHealthStatus,
    pub next_queue_tasks: usize,
    pub immediate_ready_tasks: usize,
    pub history_runs: usize,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceRunSummary {
    pub fn from_continuation(
        continuation: &AgentClosedLoopRuntimeServiceDispatchContinuation,
    ) -> Self {
        let dispatch_summary = continuation.dispatch_outcome.dispatch.summary();
        let continuation_summary = continuation.summary();
        let intake_blocked_reasons = continuation.dispatch_outcome.intake.blocked_reasons.clone();
        let status = classify_service_run(&dispatch_summary, &continuation_summary);
        let telemetry = service_run_summary_telemetry(
            &status,
            &dispatch_summary,
            &continuation_summary,
            &intake_blocked_reasons,
        );

        Self {
            status,
            dispatch_executable: dispatch_summary.executable,
            command_count: dispatch_summary.command_count,
            command_gate_allowed: dispatch_summary.command_gate_allowed,
            side_effect_gate_count: dispatch_summary.side_effect_gate_count,
            blocked_side_effect_gate_count: dispatch_summary.blocked_side_effect_gate_count,
            command_kinds: dispatch_summary.command_kinds,
            gate_blocked_reasons: dispatch_summary.blocked_reasons,
            outcome_closed: continuation_summary.outcome_closed,
            intake_clean: continuation_summary.intake_clean,
            intake_blocked_reasons,
            repair_task_count: continuation_summary.repair_task_count,
            health_status: continuation_summary.health_status,
            next_queue_tasks: continuation_summary.next_queue_tasks,
            immediate_ready_tasks: continuation_summary.immediate_ready_tasks,
            history_runs: continuation_summary.history_runs,
            telemetry,
        }
    }

    pub fn blocked_reasons(&self) -> Vec<String> {
        let mut reasons = self.gate_blocked_reasons.clone();
        extend_unique(&mut reasons, &self.intake_blocked_reasons);
        reasons
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentClosedLoopRuntimeServiceRunHistory {
    summaries: Vec<AgentClosedLoopRuntimeServiceRunSummary>,
}

impl AgentClosedLoopRuntimeServiceRunHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentClosedLoopRuntimeServiceRunSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentClosedLoopRuntimeServiceRunSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentClosedLoopRuntimeServiceRunSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentClosedLoopRuntimeServiceRunSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentClosedLoopRuntimeServiceRunDashboard {
        AgentClosedLoopRuntimeServiceRunDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceRunHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceRunDashboard {
    pub total_runs: usize,
    pub closed_runs: usize,
    pub dispatch_blocked_runs: usize,
    pub intake_blocked_runs: usize,
    pub blocked_runs: usize,
    pub closed_rate: f32,
    pub blocked_rate: f32,
    pub command_count: usize,
    pub repair_task_count: usize,
    pub total_next_queue_tasks: usize,
    pub latest_status: Option<AgentClosedLoopRuntimeServiceRunStatus>,
    pub latest_blocked_reasons: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceRunDashboard {
    pub fn from_summaries(summaries: &[AgentClosedLoopRuntimeServiceRunSummary]) -> Self {
        let total_runs = summaries.len();
        let closed_runs = summaries
            .iter()
            .filter(|summary| summary.status == AgentClosedLoopRuntimeServiceRunStatus::Closed)
            .count();
        let dispatch_blocked_runs = summaries
            .iter()
            .filter(|summary| {
                summary.status == AgentClosedLoopRuntimeServiceRunStatus::DispatchBlocked
            })
            .count();
        let intake_blocked_runs = summaries
            .iter()
            .filter(|summary| {
                summary.status == AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
            })
            .count();
        let blocked_runs = dispatch_blocked_runs + intake_blocked_runs;
        let command_count = summaries
            .iter()
            .map(|summary| summary.command_count)
            .sum::<usize>();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_task_count)
            .sum::<usize>();
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let latest = summaries.last();

        Self {
            total_runs,
            closed_runs,
            dispatch_blocked_runs,
            intake_blocked_runs,
            blocked_runs,
            closed_rate: service_run_rate(closed_runs, total_runs),
            blocked_rate: service_run_rate(blocked_runs, total_runs),
            command_count,
            repair_task_count,
            total_next_queue_tasks,
            latest_status: latest.map(|summary| summary.status),
            latest_blocked_reasons: latest
                .map(AgentClosedLoopRuntimeServiceRunSummary::blocked_reasons)
                .unwrap_or_default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_runs == 0
    }

    pub fn is_clean(&self) -> bool {
        self.blocked_runs == 0 && self.repair_task_count == 0
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceRunHealth {
        AgentClosedLoopRuntimeServiceRunHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceRunHealthPolicy {
    pub minimum_closed_rate: f32,
    pub maximum_dispatch_blocked_runs: usize,
    pub maximum_intake_blocked_runs: usize,
    pub maximum_repair_task_count: usize,
}

impl Default for AgentClosedLoopRuntimeServiceRunHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_closed_rate: 0.67,
            maximum_dispatch_blocked_runs: 0,
            maximum_intake_blocked_runs: 0,
            maximum_repair_task_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceRunHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentClosedLoopRuntimeServiceRunDashboard,
}

impl AgentClosedLoopRuntimeServiceRunHealth {
    pub fn from_dashboard(
        dashboard: AgentClosedLoopRuntimeServiceRunDashboard,
        policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("service_run_history_empty".to_owned());
        }

        if !dashboard.is_empty() && dashboard.closed_rate < policy.minimum_closed_rate {
            watch_reasons.push(format!(
                "service_run_closed_rate={:.3}<{}",
                dashboard.closed_rate, policy.minimum_closed_rate
            ));
        }

        if dashboard.dispatch_blocked_runs > policy.maximum_dispatch_blocked_runs {
            watch_reasons.push(format!(
                "service_run_dispatch_blocked_runs={}>{}",
                dashboard.dispatch_blocked_runs, policy.maximum_dispatch_blocked_runs
            ));
        }

        if dashboard.intake_blocked_runs > policy.maximum_intake_blocked_runs {
            repair_reasons.push(format!(
                "service_run_intake_blocked_runs={}>{}",
                dashboard.intake_blocked_runs, policy.maximum_intake_blocked_runs
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_task_count {
            repair_reasons.push(format!(
                "service_run_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_task_count
            ));
        }

        if !dashboard.latest_blocked_reasons.is_empty() {
            watch_reasons.push(format!(
                "service_run_latest_blocked={}",
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

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceRunHistoryRecord {
    pub history: AgentClosedLoopRuntimeServiceRunHistory,
    pub appended_summary: AgentClosedLoopRuntimeServiceRunSummary,
    pub dashboard: AgentClosedLoopRuntimeServiceRunDashboard,
    pub health: AgentClosedLoopRuntimeServiceRunHealth,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceRunHistoryRecord {
    pub fn latest(&self) -> &AgentClosedLoopRuntimeServiceRunSummary {
        &self.appended_summary
    }

    pub fn attempts(&self) -> usize {
        self.history.len()
    }

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
pub struct AgentClosedLoopRuntimeServiceRunHistoryRecorder;

impl AgentClosedLoopRuntimeServiceRunHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record(
        &self,
        mut history: AgentClosedLoopRuntimeServiceRunHistory,
        run: &AgentClosedLoopRuntimeServiceRun,
        policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceRunHistoryRecord {
        let appended_summary = run.run_summary.clone();
        history.push(appended_summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            service_run_history_record_telemetry(&appended_summary, &dashboard, &health);

        AgentClosedLoopRuntimeServiceRunHistoryRecord {
            history,
            appended_summary,
            dashboard,
            health,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServicePreflight {
    pub turn_plan: AgentClosedLoopNextTurnPlan,
    pub service_run_health: AgentClosedLoopRuntimeServiceRunHealth,
    pub mode: AgentClosedLoopNextTurnMode,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServicePreflight {
    pub fn from_parts(
        turn_plan: AgentClosedLoopNextTurnPlan,
        service_run_health: AgentClosedLoopRuntimeServiceRunHealth,
    ) -> Self {
        let mode = service_preflight_mode(&turn_plan, &service_run_health);
        let mut reasons = turn_plan.reasons.clone();
        extend_unique(&mut reasons, &service_run_health.reasons);
        if mode == AgentClosedLoopNextTurnMode::Repair
            && turn_plan.mode != AgentClosedLoopNextTurnMode::Repair
            && service_run_health.status == AgentClosedLoopExecutionHealthStatus::Repair
        {
            extend_unique(
                &mut reasons,
                &["service_run_health_requires_repair".to_owned()],
            );
        }
        let telemetry =
            service_preflight_telemetry(&turn_plan, &service_run_health, mode, &reasons);

        Self {
            turn_plan,
            service_run_health,
            mode,
            reasons,
            telemetry,
        }
    }

    pub fn can_schedule(&self) -> bool {
        self.mode.can_schedule() && !self.turn_plan.next_queue.is_empty()
    }

    pub fn allows_adaptive_evolution(&self) -> bool {
        self.mode.allows_adaptive_evolution()
            && self.turn_plan.allows_adaptive_evolution()
            && self.service_run_health.is_stable()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.mode == AgentClosedLoopNextTurnMode::Repair
    }

    pub fn allows_service_advance(&self) -> bool {
        !self.requires_repair_first()
    }

    pub fn side_effect_admission(&self) -> AgentCollaborationAdapterSideEffectAdmission {
        let requires_repair_first = self.requires_repair_first();
        let can_dispatch_service_commands = self.can_schedule() && !requires_repair_first;
        let can_promote_memory_note = self.mode == AgentClosedLoopNextTurnMode::Continue
            && self.turn_plan.health.is_stable()
            && self.service_run_health.is_stable();
        let can_admit_adaptive_evolution =
            can_promote_memory_note && self.allows_adaptive_evolution();
        let gates = service_preflight_admission_gates(
            can_dispatch_service_commands,
            can_promote_memory_note,
            can_admit_adaptive_evolution,
            requires_repair_first,
        );
        let reasons = service_preflight_admission_reasons(self);
        let telemetry = service_preflight_admission_telemetry(
            self.mode,
            service_preflight_admission_health_status(self),
            can_dispatch_service_commands,
            can_promote_memory_note,
            can_admit_adaptive_evolution,
            requires_repair_first,
            reasons.len(),
        );

        AgentCollaborationAdapterSideEffectAdmission {
            mode: self.mode,
            health_status: service_preflight_admission_health_status(self),
            can_dispatch_service_commands,
            can_promote_memory_note,
            can_admit_adaptive_evolution,
            requires_repair_first,
            gates,
            reasons,
            service_execution_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_closes: 0,
            service_execution_tool_build_command_reason_count: 0,
            telemetry,
        }
    }

    pub fn follow_up_plan(&self) -> AgentClosedLoopRuntimeServicePreflightFollowUpPlan {
        AgentClosedLoopRuntimeServicePreflightFollowUpPlan::from_preflight(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServicePreflightFollowUpPlan {
    pub tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServicePreflightFollowUpPlan {
    pub fn from_preflight(preflight: &AgentClosedLoopRuntimeServicePreflight) -> Self {
        let tasks = if preflight.mode == AgentClosedLoopNextTurnMode::Continue
            || preflight.mode == AgentClosedLoopNextTurnMode::Idle
        {
            Vec::new()
        } else {
            preflight
                .reasons
                .iter()
                .enumerate()
                .map(|(index, reason)| service_preflight_follow_up_task(preflight, index, reason))
                .collect::<Vec<_>>()
        };
        let next_queue = if tasks.is_empty() {
            preflight.turn_plan.next_queue.clone()
        } else {
            preflight
                .turn_plan
                .next_queue
                .clone()
                .with_repair_first(&tasks)
        };
        let telemetry = service_preflight_follow_up_telemetry(preflight.mode, &tasks, &next_queue);

        Self {
            tasks,
            next_queue,
            telemetry,
        }
    }

    pub fn immediate_ready_task_ids(&self) -> Vec<String> {
        self.next_queue
            .immediate_ready_tasks()
            .into_iter()
            .map(|task| task.id)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServicePreflightContinuation {
    pub preflight: AgentClosedLoopRuntimeServicePreflight,
    pub follow_up_plan: AgentClosedLoopRuntimeServicePreflightFollowUpPlan,
    pub next_runtime_input: AgentClosedLoopRuntimeTurnInput,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServicePreflightContinuation {
    pub fn next_queue(&self) -> AgentTaskQueue {
        self.next_runtime_input.next_queue.clone()
    }

    pub fn immediate_ready_task_ids(&self) -> Vec<String> {
        self.next_runtime_input
            .next_queue
            .immediate_ready_tasks()
            .into_iter()
            .map(|task| task.id)
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServicePreflightContinuationPlanner;

impl AgentClosedLoopRuntimeServicePreflightContinuationPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        preflight: AgentClosedLoopRuntimeServicePreflight,
        input: AgentClosedLoopRuntimeContinuationInput,
    ) -> AgentClosedLoopRuntimeServicePreflightContinuation {
        let follow_up_plan = preflight.follow_up_plan();
        let next_runtime_input = AgentClosedLoopRuntimeTurnInput {
            history: preflight.turn_plan.history.clone(),
            next_queue: follow_up_plan.next_queue.clone(),
            completed_task_ids: input.completed_task_ids,
            model_routes: input.model_routes,
            budget_ledger: input.budget_ledger,
            budget_policy: input.budget_policy,
            health_policy: input.health_policy,
            max_parallel_tasks: input.max_parallel_tasks,
            evidence: input.evidence,
        };
        let telemetry = service_preflight_continuation_telemetry(
            &preflight,
            &follow_up_plan,
            &next_runtime_input,
        );

        AgentClosedLoopRuntimeServicePreflightContinuation {
            preflight,
            follow_up_plan,
            next_runtime_input,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServicePreflightPlanner;

impl AgentClosedLoopRuntimeServicePreflightPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        execution_history: AgentClosedLoopExecutionHistory,
        service_run_history: AgentClosedLoopRuntimeServiceRunHistory,
        next_queue: AgentTaskQueue,
        execution_policy: AgentClosedLoopExecutionHealthPolicy,
        service_run_policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
    ) -> AgentClosedLoopRuntimeServicePreflight {
        let turn_plan = execution_history.next_turn_plan(next_queue, execution_policy);
        let service_run_health = service_run_history.health(service_run_policy);
        AgentClosedLoopRuntimeServicePreflight::from_parts(turn_plan, service_run_health)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopState {
    pub execution_history: AgentClosedLoopExecutionHistory,
    pub service_run_history: AgentClosedLoopRuntimeServiceRunHistory,
    pub preflight_continuation: AgentClosedLoopRuntimeServicePreflightContinuation,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopState {
    pub fn summary(&self) -> AgentClosedLoopRuntimeServiceLoopStateSummary {
        AgentClosedLoopRuntimeServiceLoopStateSummary::from_state(self)
    }

    pub fn next_runtime_input(&self) -> &AgentClosedLoopRuntimeTurnInput {
        &self.preflight_continuation.next_runtime_input
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.preflight_continuation.next_queue()
    }

    pub fn mode(&self) -> AgentClosedLoopNextTurnMode {
        self.preflight_continuation.preflight.mode
    }

    pub fn can_schedule(&self) -> bool {
        self.preflight_continuation.preflight.can_schedule()
    }

    pub fn allows_adaptive_evolution(&self) -> bool {
        self.preflight_continuation
            .preflight
            .allows_adaptive_evolution()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.preflight_continuation
            .preflight
            .requires_repair_first()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.preflight_continuation
            .preflight
            .allows_service_advance()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopStateSummary {
    pub mode: AgentClosedLoopNextTurnMode,
    pub execution_health_status: AgentClosedLoopExecutionHealthStatus,
    pub service_run_health_status: AgentClosedLoopExecutionHealthStatus,
    pub side_effect_admission_health_status: AgentClosedLoopExecutionHealthStatus,
    pub can_schedule: bool,
    pub side_effect_dispatch_allowed: bool,
    pub memory_note_allowed: bool,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub execution_history_runs: usize,
    pub service_run_attempts: usize,
    pub preflight_follow_up_tasks: usize,
    pub next_queue_tasks: usize,
    pub immediate_ready_tasks: usize,
    pub side_effect_admission_reasons: usize,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopStateSummary {
    pub fn from_state(state: &AgentClosedLoopRuntimeServiceLoopState) -> Self {
        let preflight = &state.preflight_continuation.preflight;
        let mode = preflight.mode;
        let execution_health_status = preflight.turn_plan.health.status;
        let service_run_health_status = preflight.service_run_health.status;
        let side_effect_admission = preflight.side_effect_admission();
        let side_effect_admission_health_status = side_effect_admission.health_status;
        let can_schedule = state.can_schedule();
        let side_effect_dispatch_allowed = side_effect_admission.can_dispatch_service_commands;
        let memory_note_allowed = side_effect_admission.can_promote_memory_note;
        let allows_adaptive_evolution = state.allows_adaptive_evolution();
        let requires_repair_first = state.requires_repair_first();
        let execution_history_runs = state.execution_history.len();
        let service_run_attempts = state.service_run_history.len();
        let preflight_follow_up_tasks = state.preflight_continuation.follow_up_plan.tasks.len();
        let next_queue_tasks = state
            .next_runtime_input()
            .next_queue
            .next_queue_tasks()
            .len();
        let immediate_ready_tasks = state
            .next_runtime_input()
            .next_queue
            .immediate_ready_tasks()
            .len();
        let side_effect_admission_reasons = side_effect_admission.reasons.len();
        let reasons = preflight.reasons.clone();
        let telemetry = service_loop_state_summary_telemetry(
            mode,
            execution_health_status,
            service_run_health_status,
            side_effect_admission_health_status,
            can_schedule,
            side_effect_dispatch_allowed,
            memory_note_allowed,
            allows_adaptive_evolution,
            requires_repair_first,
            execution_history_runs,
            service_run_attempts,
            preflight_follow_up_tasks,
            next_queue_tasks,
            immediate_ready_tasks,
            side_effect_admission_reasons,
            &reasons,
        );

        Self {
            mode,
            execution_health_status,
            service_run_health_status,
            side_effect_admission_health_status,
            can_schedule,
            side_effect_dispatch_allowed,
            memory_note_allowed,
            allows_adaptive_evolution,
            requires_repair_first,
            execution_history_runs,
            service_run_attempts,
            preflight_follow_up_tasks,
            next_queue_tasks,
            immediate_ready_tasks,
            side_effect_admission_reasons,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopStatePlanner {
    preflight_planner: AgentClosedLoopRuntimeServicePreflightPlanner,
    continuation_planner: AgentClosedLoopRuntimeServicePreflightContinuationPlanner,
}

impl AgentClosedLoopRuntimeServiceLoopStatePlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn plan(
        &self,
        execution_history: AgentClosedLoopExecutionHistory,
        service_run_history: AgentClosedLoopRuntimeServiceRunHistory,
        next_queue: AgentTaskQueue,
        execution_policy: AgentClosedLoopExecutionHealthPolicy,
        service_run_policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
        continuation_input: AgentClosedLoopRuntimeContinuationInput,
    ) -> AgentClosedLoopRuntimeServiceLoopState {
        let preflight = self.preflight_planner.plan(
            execution_history.clone(),
            service_run_history.clone(),
            next_queue,
            execution_policy,
            service_run_policy,
        );
        let preflight_continuation = self
            .continuation_planner
            .plan(preflight, continuation_input);
        let telemetry = service_loop_state_telemetry(
            &execution_history,
            &service_run_history,
            &preflight_continuation,
        );

        AgentClosedLoopRuntimeServiceLoopState {
            execution_history,
            service_run_history,
            preflight_continuation,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopAdvance {
    pub run_record: AgentClosedLoopRuntimeServiceRunHistoryRecord,
    pub loop_state: AgentClosedLoopRuntimeServiceLoopState,
    pub summary: AgentClosedLoopRuntimeServiceLoopStateSummary,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopAdvance {
    pub fn next_runtime_input(&self) -> &AgentClosedLoopRuntimeTurnInput {
        self.loop_state.next_runtime_input()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.loop_state.next_queue()
    }

    pub fn mode(&self) -> AgentClosedLoopNextTurnMode {
        self.loop_state.mode()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.loop_state.requires_repair_first()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.loop_state.allows_service_advance()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopAdvancePlanner {
    history_recorder: AgentClosedLoopRuntimeServiceRunHistoryRecorder,
    loop_state_planner: AgentClosedLoopRuntimeServiceLoopStatePlanner,
}

impl AgentClosedLoopRuntimeServiceLoopAdvancePlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(
        &self,
        service_run_history: AgentClosedLoopRuntimeServiceRunHistory,
        run: &AgentClosedLoopRuntimeServiceRun,
        service_run_policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopAdvance {
        let run_record = self
            .history_recorder
            .record(service_run_history, run, service_run_policy);
        let next_runtime_input = run.next_runtime_input();
        let continuation_input = runtime_continuation_input_from_runtime_input(next_runtime_input);
        let loop_state = self.loop_state_planner.plan(
            next_runtime_input.history.clone(),
            run_record.history.clone(),
            next_runtime_input.next_queue.clone(),
            next_runtime_input.health_policy,
            service_run_policy,
            continuation_input,
        );
        let summary = loop_state.summary();
        let telemetry = service_loop_advance_telemetry(&run_record, &summary);

        AgentClosedLoopRuntimeServiceLoopAdvance {
            run_record,
            loop_state,
            summary,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceRunner {
    request_runner: AgentClosedLoopRuntimeServiceRequestRunner,
    continuation_planner: AgentClosedLoopRuntimeServiceDispatchContinuationPlanner,
}

impl AgentClosedLoopRuntimeServiceRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run<E, P>(
        &self,
        input: AgentClosedLoopRuntimeServiceRunInput,
        engine: &mut E,
        memory: &mut P,
    ) -> AgentClosedLoopRuntimeServiceRun
    where
        E: EnginePort,
        E::Error: ToString,
        P: MemoryPort,
        P::Error: ToString,
    {
        let dispatch = self
            .request_runner
            .run_dispatch(input.request_input, engine, memory);
        let dispatch_outcome =
            dispatch.close_with_intake(input.receipts, input.continuation_input.clone());
        let dispatch_continuation = self
            .continuation_planner
            .plan(dispatch_outcome, input.continuation_input);
        let summary = dispatch_continuation.summary();
        let run_summary =
            AgentClosedLoopRuntimeServiceRunSummary::from_continuation(&dispatch_continuation);
        let telemetry = service_run_telemetry(&dispatch_continuation, &run_summary);

        AgentClosedLoopRuntimeServiceRun {
            dispatch_continuation,
            summary,
            run_summary,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunInput {
    pub service_run_input: AgentClosedLoopRuntimeServiceRunInput,
    pub service_run_history: AgentClosedLoopRuntimeServiceRunHistory,
    pub service_run_policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
}

impl AgentClosedLoopRuntimeServiceLoopRunInput {
    pub fn new(
        service_run_input: AgentClosedLoopRuntimeServiceRunInput,
        service_run_history: AgentClosedLoopRuntimeServiceRunHistory,
        service_run_policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
    ) -> Self {
        Self {
            service_run_input,
            service_run_history,
            service_run_policy,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRun {
    pub run: AgentClosedLoopRuntimeServiceRun,
    pub advance: AgentClosedLoopRuntimeServiceLoopAdvance,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRun {
    pub fn compact_summary(&self) -> AgentClosedLoopRuntimeServiceLoopRunSummary {
        AgentClosedLoopRuntimeServiceLoopRunSummary::from_loop_run(self)
    }

    pub fn next_runtime_input(&self) -> &AgentClosedLoopRuntimeTurnInput {
        self.advance.next_runtime_input()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.advance.next_queue()
    }

    pub fn loop_state(&self) -> &AgentClosedLoopRuntimeServiceLoopState {
        &self.advance.loop_state
    }

    pub fn summary(&self) -> &AgentClosedLoopRuntimeServiceLoopStateSummary {
        &self.advance.summary
    }

    pub fn mode(&self) -> AgentClosedLoopNextTurnMode {
        self.advance.mode()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunSummary {
    pub service_run_status: AgentClosedLoopRuntimeServiceRunStatus,
    pub dispatch_executable: bool,
    pub command_count: usize,
    pub command_gate_allowed: bool,
    pub side_effect_gate_count: usize,
    pub blocked_side_effect_gate_count: usize,
    pub intake_clean: bool,
    pub service_attempts: usize,
    pub mode: AgentClosedLoopNextTurnMode,
    pub execution_health_status: AgentClosedLoopExecutionHealthStatus,
    pub service_run_health_status: AgentClosedLoopExecutionHealthStatus,
    pub side_effect_admission_health_status: AgentClosedLoopExecutionHealthStatus,
    pub can_schedule: bool,
    pub side_effect_dispatch_allowed: bool,
    pub memory_note_allowed: bool,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub execution_history_runs: usize,
    pub preflight_follow_up_tasks: usize,
    pub next_queue_tasks: usize,
    pub side_effect_admission_reasons: usize,
    pub blocked_reasons: Vec<String>,
    pub preflight_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunSummary {
    pub fn from_loop_run(loop_run: &AgentClosedLoopRuntimeServiceLoopRun) -> Self {
        let run_summary = &loop_run.run.run_summary;
        let loop_state_summary = &loop_run.advance.summary;
        let blocked_reasons = run_summary.blocked_reasons();
        let preflight_reasons = loop_state_summary.reasons.clone();
        let telemetry = service_loop_run_summary_telemetry(
            run_summary,
            loop_state_summary,
            loop_run.advance.run_record.attempts(),
            &blocked_reasons,
            &preflight_reasons,
        );

        Self {
            service_run_status: run_summary.status,
            dispatch_executable: run_summary.dispatch_executable,
            command_count: run_summary.command_count,
            command_gate_allowed: run_summary.command_gate_allowed,
            side_effect_gate_count: run_summary.side_effect_gate_count,
            blocked_side_effect_gate_count: run_summary.blocked_side_effect_gate_count,
            intake_clean: run_summary.intake_clean,
            service_attempts: loop_run.advance.run_record.attempts(),
            mode: loop_state_summary.mode,
            execution_health_status: loop_state_summary.execution_health_status,
            service_run_health_status: loop_state_summary.service_run_health_status,
            side_effect_admission_health_status: loop_state_summary
                .side_effect_admission_health_status,
            can_schedule: loop_state_summary.can_schedule,
            side_effect_dispatch_allowed: loop_state_summary.side_effect_dispatch_allowed,
            memory_note_allowed: loop_state_summary.memory_note_allowed,
            allows_adaptive_evolution: loop_state_summary.allows_adaptive_evolution,
            requires_repair_first: loop_state_summary.requires_repair_first,
            execution_history_runs: loop_state_summary.execution_history_runs,
            preflight_follow_up_tasks: loop_state_summary.preflight_follow_up_tasks,
            next_queue_tasks: loop_state_summary.next_queue_tasks,
            side_effect_admission_reasons: loop_state_summary.side_effect_admission_reasons,
            blocked_reasons,
            preflight_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunHistory {
    summaries: Vec<AgentClosedLoopRuntimeServiceLoopRunSummary>,
}

impl AgentClosedLoopRuntimeServiceLoopRunHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentClosedLoopRuntimeServiceLoopRunSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentClosedLoopRuntimeServiceLoopRunSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentClosedLoopRuntimeServiceLoopRunSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentClosedLoopRuntimeServiceLoopRunSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentClosedLoopRuntimeServiceLoopRunDashboard {
        AgentClosedLoopRuntimeServiceLoopRunDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDashboard {
    pub total_runs: usize,
    pub closed_runs: usize,
    pub dispatch_blocked_runs: usize,
    pub intake_blocked_runs: usize,
    pub command_gate_allowed_runs: usize,
    pub repair_first_runs: usize,
    pub side_effect_dispatch_allowed_runs: usize,
    pub memory_note_allowed_runs: usize,
    pub adaptive_allowed_runs: usize,
    pub closed_rate: f32,
    pub command_gate_allowed_rate: f32,
    pub repair_first_rate: f32,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub adaptive_allowed_rate: f32,
    pub command_count: usize,
    pub side_effect_gate_count: usize,
    pub blocked_side_effect_gate_count: usize,
    pub follow_up_task_count: usize,
    pub total_next_queue_tasks: usize,
    pub latest_status: Option<AgentClosedLoopRuntimeServiceRunStatus>,
    pub latest_mode: Option<AgentClosedLoopNextTurnMode>,
    pub latest_blocked_reasons: Vec<String>,
    pub latest_preflight_reasons: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDashboard {
    pub fn from_summaries(summaries: &[AgentClosedLoopRuntimeServiceLoopRunSummary]) -> Self {
        let total_runs = summaries.len();
        let closed_runs = summaries
            .iter()
            .filter(|summary| {
                summary.service_run_status == AgentClosedLoopRuntimeServiceRunStatus::Closed
            })
            .count();
        let dispatch_blocked_runs = summaries
            .iter()
            .filter(|summary| {
                summary.service_run_status
                    == AgentClosedLoopRuntimeServiceRunStatus::DispatchBlocked
            })
            .count();
        let intake_blocked_runs = summaries
            .iter()
            .filter(|summary| {
                summary.service_run_status == AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
            })
            .count();
        let command_gate_allowed_runs = summaries
            .iter()
            .filter(|summary| summary.command_gate_allowed)
            .count();
        let repair_first_runs = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let side_effect_dispatch_allowed_runs = summaries
            .iter()
            .filter(|summary| summary.side_effect_dispatch_allowed)
            .count();
        let memory_note_allowed_runs = summaries
            .iter()
            .filter(|summary| summary.memory_note_allowed)
            .count();
        let adaptive_allowed_runs = summaries
            .iter()
            .filter(|summary| summary.allows_adaptive_evolution)
            .count();
        let command_count = summaries
            .iter()
            .map(|summary| summary.command_count)
            .sum::<usize>();
        let side_effect_gate_count = summaries
            .iter()
            .map(|summary| summary.side_effect_gate_count)
            .sum::<usize>();
        let blocked_side_effect_gate_count = summaries
            .iter()
            .map(|summary| summary.blocked_side_effect_gate_count)
            .sum::<usize>();
        let follow_up_task_count = summaries
            .iter()
            .map(|summary| summary.preflight_follow_up_tasks)
            .sum::<usize>();
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let latest = summaries.last();

        Self {
            total_runs,
            closed_runs,
            dispatch_blocked_runs,
            intake_blocked_runs,
            command_gate_allowed_runs,
            repair_first_runs,
            side_effect_dispatch_allowed_runs,
            memory_note_allowed_runs,
            adaptive_allowed_runs,
            closed_rate: service_run_rate(closed_runs, total_runs),
            command_gate_allowed_rate: service_run_rate(command_gate_allowed_runs, total_runs),
            repair_first_rate: service_run_rate(repair_first_runs, total_runs),
            side_effect_dispatch_allowed_rate: service_run_rate(
                side_effect_dispatch_allowed_runs,
                total_runs,
            ),
            memory_note_allowed_rate: service_run_rate(memory_note_allowed_runs, total_runs),
            adaptive_allowed_rate: service_run_rate(adaptive_allowed_runs, total_runs),
            command_count,
            side_effect_gate_count,
            blocked_side_effect_gate_count,
            follow_up_task_count,
            total_next_queue_tasks,
            latest_status: latest.map(|summary| summary.service_run_status),
            latest_mode: latest.map(|summary| summary.mode),
            latest_blocked_reasons: latest
                .map(|summary| summary.blocked_reasons.clone())
                .unwrap_or_default(),
            latest_preflight_reasons: latest
                .map(|summary| summary.preflight_reasons.clone())
                .unwrap_or_default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_runs == 0
    }

    pub fn is_clean(&self) -> bool {
        self.dispatch_blocked_runs == 0
            && self.intake_blocked_runs == 0
            && self.repair_first_runs == 0
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunHealth {
        AgentClosedLoopRuntimeServiceLoopRunHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunHealthPolicy {
    pub minimum_closed_rate: f32,
    pub minimum_adaptive_allowed_rate: f32,
    pub maximum_dispatch_blocked_runs: usize,
    pub maximum_intake_blocked_runs: usize,
    pub maximum_repair_first_runs: usize,
    pub maximum_follow_up_task_count: usize,
}

impl Default for AgentClosedLoopRuntimeServiceLoopRunHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_closed_rate: 0.67,
            minimum_adaptive_allowed_rate: 0.50,
            maximum_dispatch_blocked_runs: 0,
            maximum_intake_blocked_runs: 0,
            maximum_repair_first_runs: 0,
            maximum_follow_up_task_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentClosedLoopRuntimeServiceLoopRunDashboard,
}

impl AgentClosedLoopRuntimeServiceLoopRunHealth {
    pub fn from_dashboard(
        dashboard: AgentClosedLoopRuntimeServiceLoopRunDashboard,
        policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("service_loop_run_history_empty".to_owned());
        }

        if !dashboard.is_empty() && dashboard.closed_rate < policy.minimum_closed_rate {
            watch_reasons.push(format!(
                "service_loop_run_closed_rate={:.3}<{}",
                dashboard.closed_rate, policy.minimum_closed_rate
            ));
        }

        if !dashboard.is_empty()
            && dashboard.adaptive_allowed_rate < policy.minimum_adaptive_allowed_rate
        {
            watch_reasons.push(format!(
                "service_loop_run_adaptive_allowed_rate={:.3}<{}",
                dashboard.adaptive_allowed_rate, policy.minimum_adaptive_allowed_rate
            ));
        }

        if dashboard.dispatch_blocked_runs > policy.maximum_dispatch_blocked_runs {
            watch_reasons.push(format!(
                "service_loop_run_dispatch_blocked_runs={}>{}",
                dashboard.dispatch_blocked_runs, policy.maximum_dispatch_blocked_runs
            ));
        }

        if dashboard.intake_blocked_runs > policy.maximum_intake_blocked_runs {
            repair_reasons.push(format!(
                "service_loop_run_intake_blocked_runs={}>{}",
                dashboard.intake_blocked_runs, policy.maximum_intake_blocked_runs
            ));
        }

        if dashboard.repair_first_runs > policy.maximum_repair_first_runs {
            repair_reasons.push(format!(
                "service_loop_run_repair_first_runs={}>{}",
                dashboard.repair_first_runs, policy.maximum_repair_first_runs
            ));
        }

        if dashboard.follow_up_task_count > policy.maximum_follow_up_task_count {
            watch_reasons.push(format!(
                "service_loop_run_follow_up_tasks={}>{}",
                dashboard.follow_up_task_count, policy.maximum_follow_up_task_count
            ));
        }

        if !dashboard.latest_blocked_reasons.is_empty() {
            watch_reasons.push(format!(
                "service_loop_run_latest_blocked={}",
                dashboard.latest_blocked_reasons.join(";")
            ));
        }

        if !dashboard.latest_preflight_reasons.is_empty() {
            watch_reasons.push(format!(
                "service_loop_run_latest_preflight={}",
                dashboard.latest_preflight_reasons.join(";")
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
pub struct AgentClosedLoopRuntimeServiceLoopRunHistoryRecord {
    pub history: AgentClosedLoopRuntimeServiceLoopRunHistory,
    pub appended_summary: AgentClosedLoopRuntimeServiceLoopRunSummary,
    pub dashboard: AgentClosedLoopRuntimeServiceLoopRunDashboard,
    pub health: AgentClosedLoopRuntimeServiceLoopRunHealth,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunHistoryRecord {
    pub fn latest(&self) -> &AgentClosedLoopRuntimeServiceLoopRunSummary {
        &self.appended_summary
    }

    pub fn records(&self) -> usize {
        self.history.len()
    }

    pub fn transitions(&self) -> usize {
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
pub struct AgentClosedLoopRuntimeServiceLoopRunHistoryRecorder;

impl AgentClosedLoopRuntimeServiceLoopRunHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record(
        &self,
        mut history: AgentClosedLoopRuntimeServiceLoopRunHistory,
        loop_run: &AgentClosedLoopRuntimeServiceLoopRun,
        policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunHistoryRecord {
        let appended_summary = loop_run.compact_summary();
        history.push(appended_summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            service_loop_run_history_record_telemetry(&appended_summary, &dashboard, &health);

        AgentClosedLoopRuntimeServiceLoopRunHistoryRecord {
            history,
            appended_summary,
            dashboard,
            health,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunControlPlan {
    pub health: AgentClosedLoopRuntimeServiceLoopRunHealth,
    pub mode: AgentClosedLoopNextTurnMode,
    pub next_queue: AgentTaskQueue,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunControlPlan {
    pub fn from_history(
        history: AgentClosedLoopRuntimeServiceLoopRunHistory,
        next_queue: AgentTaskQueue,
        policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
    ) -> Self {
        Self::from_health(history.health(policy), next_queue)
    }

    pub fn from_health(
        health: AgentClosedLoopRuntimeServiceLoopRunHealth,
        next_queue: AgentTaskQueue,
    ) -> Self {
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
        let telemetry =
            service_loop_run_control_plan_telemetry(mode, &health, next_queue.len(), &reasons);

        Self {
            health,
            mode,
            next_queue,
            reasons,
            telemetry,
        }
    }

    pub fn can_schedule(&self) -> bool {
        self.mode.can_schedule() && !self.next_queue.is_empty()
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

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunController;

impl AgentClosedLoopRuntimeServiceLoopRunController {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        history: AgentClosedLoopRuntimeServiceLoopRunHistory,
        next_queue: AgentTaskQueue,
        policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunControlPlan {
        AgentClosedLoopRuntimeServiceLoopRunControlPlan::from_history(history, next_queue, policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunControlRecord {
    pub history_record: AgentClosedLoopRuntimeServiceLoopRunHistoryRecord,
    pub control_plan: AgentClosedLoopRuntimeServiceLoopRunControlPlan,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunControlRecord {
    pub fn summary(&self) -> AgentClosedLoopRuntimeServiceLoopRunControlSummary {
        AgentClosedLoopRuntimeServiceLoopRunControlSummary::from_record(self)
    }

    pub fn latest(&self) -> &AgentClosedLoopRuntimeServiceLoopRunSummary {
        self.history_record.latest()
    }

    pub fn transitions(&self) -> usize {
        self.history_record.transitions()
    }

    pub fn can_schedule(&self) -> bool {
        self.control_plan.can_schedule()
    }

    pub fn allows_adaptive_evolution(&self) -> bool {
        self.control_plan.allows_adaptive_evolution()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.control_plan.requires_repair_first()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.control_plan.allows_service_advance()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunControlSummary {
    pub latest_status: AgentClosedLoopRuntimeServiceRunStatus,
    pub mode: AgentClosedLoopNextTurnMode,
    pub health_status: AgentClosedLoopExecutionHealthStatus,
    pub transitions: usize,
    pub closed_rate: f32,
    pub command_gate_allowed_rate: f32,
    pub repair_first_rate: f32,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub adaptive_allowed_rate: f32,
    pub side_effect_gate_count: usize,
    pub blocked_side_effect_gate_count: usize,
    pub can_schedule: bool,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub next_queue_tasks: usize,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunControlSummary {
    pub fn from_record(record: &AgentClosedLoopRuntimeServiceLoopRunControlRecord) -> Self {
        let latest = record.latest();
        let control_plan = &record.control_plan;
        let dashboard = &record.history_record.dashboard;
        let reasons = control_plan.reasons.clone();
        let telemetry =
            service_loop_run_control_summary_telemetry(latest, control_plan, dashboard, &reasons);

        Self {
            latest_status: latest.service_run_status,
            mode: control_plan.mode,
            health_status: control_plan.health.status,
            transitions: record.transitions(),
            closed_rate: dashboard.closed_rate,
            command_gate_allowed_rate: dashboard.command_gate_allowed_rate,
            repair_first_rate: dashboard.repair_first_rate,
            side_effect_dispatch_allowed_rate: dashboard.side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate: dashboard.memory_note_allowed_rate,
            adaptive_allowed_rate: dashboard.adaptive_allowed_rate,
            side_effect_gate_count: dashboard.side_effect_gate_count,
            blocked_side_effect_gate_count: dashboard.blocked_side_effect_gate_count,
            can_schedule: control_plan.can_schedule(),
            allows_adaptive_evolution: control_plan.allows_adaptive_evolution(),
            requires_repair_first: control_plan.requires_repair_first(),
            next_queue_tasks: control_plan.next_queue.len(),
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory {
    summaries: Vec<AgentClosedLoopRuntimeServiceLoopRunControlSummary>,
}

impl AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentClosedLoopRuntimeServiceLoopRunControlSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentClosedLoopRuntimeServiceLoopRunControlSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentClosedLoopRuntimeServiceLoopRunControlSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentClosedLoopRuntimeServiceLoopRunControlSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentClosedLoopRuntimeServiceLoopRunControlDashboard {
        AgentClosedLoopRuntimeServiceLoopRunControlDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunControlHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunControlDashboard {
    pub total_records: usize,
    pub continue_records: usize,
    pub observe_records: usize,
    pub repair_records: usize,
    pub idle_records: usize,
    pub schedulable_records: usize,
    pub adaptive_allowed_records: usize,
    pub repair_first_records: usize,
    pub schedule_rate: f32,
    pub command_gate_allowed_rate: f32,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub adaptive_allowed_rate: f32,
    pub repair_first_rate: f32,
    pub side_effect_gate_count: usize,
    pub blocked_side_effect_gate_count: usize,
    pub total_next_queue_tasks: usize,
    pub latest_status: Option<AgentClosedLoopRuntimeServiceRunStatus>,
    pub latest_mode: Option<AgentClosedLoopNextTurnMode>,
    pub latest_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
    pub latest_reasons: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunControlDashboard {
    pub fn from_summaries(
        summaries: &[AgentClosedLoopRuntimeServiceLoopRunControlSummary],
    ) -> Self {
        let total_records = summaries.len();
        let continue_records = summaries
            .iter()
            .filter(|summary| summary.mode == AgentClosedLoopNextTurnMode::Continue)
            .count();
        let observe_records = summaries
            .iter()
            .filter(|summary| summary.mode == AgentClosedLoopNextTurnMode::Observe)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.mode == AgentClosedLoopNextTurnMode::Repair)
            .count();
        let idle_records = summaries
            .iter()
            .filter(|summary| summary.mode == AgentClosedLoopNextTurnMode::Idle)
            .count();
        let schedulable_records = summaries
            .iter()
            .filter(|summary| summary.can_schedule)
            .count();
        let adaptive_allowed_records = summaries
            .iter()
            .filter(|summary| summary.allows_adaptive_evolution)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let total_next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let command_gate_allowed_rate =
            average_service_run_rate(summaries, |summary| summary.command_gate_allowed_rate);
        let side_effect_dispatch_allowed_rate = average_service_run_rate(summaries, |summary| {
            summary.side_effect_dispatch_allowed_rate
        });
        let memory_note_allowed_rate =
            average_service_run_rate(summaries, |summary| summary.memory_note_allowed_rate);
        let side_effect_gate_count = summaries
            .iter()
            .map(|summary| summary.side_effect_gate_count)
            .sum::<usize>();
        let blocked_side_effect_gate_count = summaries
            .iter()
            .map(|summary| summary.blocked_side_effect_gate_count)
            .sum::<usize>();
        let latest = summaries.last();

        Self {
            total_records,
            continue_records,
            observe_records,
            repair_records,
            idle_records,
            schedulable_records,
            adaptive_allowed_records,
            repair_first_records,
            schedule_rate: service_run_rate(schedulable_records, total_records),
            command_gate_allowed_rate,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            adaptive_allowed_rate: service_run_rate(adaptive_allowed_records, total_records),
            repair_first_rate: service_run_rate(repair_first_records, total_records),
            side_effect_gate_count,
            blocked_side_effect_gate_count,
            total_next_queue_tasks,
            latest_status: latest.map(|summary| summary.latest_status),
            latest_mode: latest.map(|summary| summary.mode),
            latest_health_status: latest.map(|summary| summary.health_status),
            latest_reasons: latest
                .map(|summary| summary.reasons.clone())
                .unwrap_or_default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn is_clean(&self) -> bool {
        self.repair_records == 0 && self.idle_records == 0
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunControlHealth {
        AgentClosedLoopRuntimeServiceLoopRunControlHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy {
    pub minimum_schedule_rate: f32,
    pub minimum_adaptive_allowed_rate: f32,
    pub maximum_observe_records: usize,
    pub maximum_repair_records: usize,
    pub maximum_idle_records: usize,
    pub maximum_repair_first_records: usize,
}

impl Default for AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_schedule_rate: 0.67,
            minimum_adaptive_allowed_rate: 0.50,
            maximum_observe_records: 1,
            maximum_repair_records: 0,
            maximum_idle_records: 1,
            maximum_repair_first_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunControlHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentClosedLoopRuntimeServiceLoopRunControlDashboard,
}

impl AgentClosedLoopRuntimeServiceLoopRunControlHealth {
    pub fn from_dashboard(
        dashboard: AgentClosedLoopRuntimeServiceLoopRunControlDashboard,
        policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("service_loop_run_control_history_empty".to_owned());
        }

        if !dashboard.is_empty() && dashboard.schedule_rate < policy.minimum_schedule_rate {
            watch_reasons.push(format!(
                "service_loop_run_control_schedule_rate={:.3}<{}",
                dashboard.schedule_rate, policy.minimum_schedule_rate
            ));
        }

        if !dashboard.is_empty()
            && dashboard.adaptive_allowed_rate < policy.minimum_adaptive_allowed_rate
        {
            watch_reasons.push(format!(
                "service_loop_run_control_adaptive_allowed_rate={:.3}<{}",
                dashboard.adaptive_allowed_rate, policy.minimum_adaptive_allowed_rate
            ));
        }

        if dashboard.observe_records > policy.maximum_observe_records {
            watch_reasons.push(format!(
                "service_loop_run_control_observe_records={}>{}",
                dashboard.observe_records, policy.maximum_observe_records
            ));
        }

        if dashboard.idle_records > policy.maximum_idle_records {
            watch_reasons.push(format!(
                "service_loop_run_control_idle_records={}>{}",
                dashboard.idle_records, policy.maximum_idle_records
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "service_loop_run_control_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "service_loop_run_control_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if !dashboard.latest_reasons.is_empty() {
            watch_reasons.push(format!(
                "service_loop_run_control_latest_reason={}",
                dashboard.latest_reasons.join(";")
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
pub struct AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecord {
    pub history: AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory,
    pub appended_summary: AgentClosedLoopRuntimeServiceLoopRunControlSummary,
    pub dashboard: AgentClosedLoopRuntimeServiceLoopRunControlDashboard,
    pub health: AgentClosedLoopRuntimeServiceLoopRunControlHealth,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecord {
    pub fn latest(&self) -> &AgentClosedLoopRuntimeServiceLoopRunControlSummary {
        &self.appended_summary
    }

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
pub struct AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder;

impl AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record(
        &self,
        mut history: AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory,
        summary: AgentClosedLoopRuntimeServiceLoopRunControlSummary,
        policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecord {
        let appended_summary = summary;
        history.push(appended_summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_loop_run_control_summary_history_record_telemetry(
            &appended_summary,
            &dashboard,
            &health,
        );

        AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecord {
            history,
            appended_summary,
            dashboard,
            health,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunMonitor {
    history_recorder: AgentClosedLoopRuntimeServiceLoopRunHistoryRecorder,
    controller: AgentClosedLoopRuntimeServiceLoopRunController,
}

impl AgentClosedLoopRuntimeServiceLoopRunMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_plan(
        &self,
        history: AgentClosedLoopRuntimeServiceLoopRunHistory,
        loop_run: &AgentClosedLoopRuntimeServiceLoopRun,
        policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunControlRecord {
        let history_record = self.history_recorder.record(history, loop_run, policy);
        let control_plan = self.controller.plan(
            history_record.history.clone(),
            loop_run.next_queue(),
            policy,
        );
        let telemetry = service_loop_run_control_record_telemetry(&history_record, &control_plan);

        AgentClosedLoopRuntimeServiceLoopRunControlRecord {
            history_record,
            control_plan,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunner {
    service_runner: AgentClosedLoopRuntimeServiceRunner,
    advance_planner: AgentClosedLoopRuntimeServiceLoopAdvancePlanner,
}

impl AgentClosedLoopRuntimeServiceLoopRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run<E, P>(
        &self,
        input: AgentClosedLoopRuntimeServiceLoopRunInput,
        engine: &mut E,
        memory: &mut P,
    ) -> AgentClosedLoopRuntimeServiceLoopRun
    where
        E: EnginePort,
        E::Error: ToString,
        P: MemoryPort,
        P::Error: ToString,
    {
        let run = self
            .service_runner
            .run(input.service_run_input, engine, memory);
        let advance =
            self.advance_planner
                .advance(input.service_run_history, &run, input.service_run_policy);
        let telemetry = service_loop_run_telemetry(&run, &advance);

        AgentClosedLoopRuntimeServiceLoopRun {
            run,
            advance,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonInput {
    pub loop_run_input: AgentClosedLoopRuntimeServiceLoopRunInput,
    pub loop_run_history: AgentClosedLoopRuntimeServiceLoopRunHistory,
    pub loop_run_health_policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
    pub control_summary_history: AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory,
    pub control_health_policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonInput {
    pub fn new(
        loop_run_input: AgentClosedLoopRuntimeServiceLoopRunInput,
        loop_run_history: AgentClosedLoopRuntimeServiceLoopRunHistory,
        loop_run_health_policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
        control_summary_history: AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory,
        control_health_policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy,
    ) -> Self {
        Self {
            loop_run_input,
            loop_run_history,
            loop_run_health_policy,
            control_summary_history,
            control_health_policy,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRecord {
    pub loop_run: AgentClosedLoopRuntimeServiceLoopRun,
    pub control_record: AgentClosedLoopRuntimeServiceLoopRunControlRecord,
    pub control_summary: AgentClosedLoopRuntimeServiceLoopRunControlSummary,
    pub control_summary_history_record:
        AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecord,
    pub service_run_policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
    pub loop_run_health_policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
    pub control_health_policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRecord {
    pub fn next_runtime_input(&self) -> &AgentClosedLoopRuntimeTurnInput {
        self.loop_run.next_runtime_input()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.loop_run.next_queue()
    }

    pub fn mode(&self) -> AgentClosedLoopNextTurnMode {
        self.control_record.control_plan.mode
    }

    pub fn can_schedule(&self) -> bool {
        self.control_record.can_schedule()
    }

    pub fn allows_adaptive_evolution(&self) -> bool {
        self.control_record.allows_adaptive_evolution()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.control_record.requires_repair_first()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.control_record.allows_service_advance()
    }

    pub fn continuation(&self) -> AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation {
        AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation::from_record(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRunner {
    loop_runner: AgentClosedLoopRuntimeServiceLoopRunner,
    monitor: AgentClosedLoopRuntimeServiceLoopRunMonitor,
    control_summary_recorder: AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run<E, P>(
        &self,
        input: AgentClosedLoopRuntimeServiceLoopRunDaemonInput,
        engine: &mut E,
        memory: &mut P,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRecord
    where
        E: EnginePort,
        E::Error: ToString,
        P: MemoryPort,
        P::Error: ToString,
    {
        let service_run_policy = input.loop_run_input.service_run_policy;
        let loop_run_health_policy = input.loop_run_health_policy;
        let control_health_policy = input.control_health_policy;
        let loop_run = self.loop_runner.run(input.loop_run_input, engine, memory);
        let control_record =
            self.monitor
                .record_and_plan(input.loop_run_history, &loop_run, loop_run_health_policy);
        let control_summary = control_record.summary();
        let control_summary_history_record = self.control_summary_recorder.record(
            input.control_summary_history,
            control_summary.clone(),
            control_health_policy,
        );
        let telemetry = service_loop_run_daemon_record_telemetry(
            &loop_run,
            &control_record,
            &control_summary_history_record,
        );

        AgentClosedLoopRuntimeServiceLoopRunDaemonRecord {
            loop_run,
            control_record,
            control_summary,
            control_summary_history_record,
            service_run_policy,
            loop_run_health_policy,
            control_health_policy,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation {
    pub next_runtime_input: AgentClosedLoopRuntimeTurnInput,
    pub service_run_history: AgentClosedLoopRuntimeServiceRunHistory,
    pub loop_run_history: AgentClosedLoopRuntimeServiceLoopRunHistory,
    pub control_summary_history: AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory,
    pub service_run_policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
    pub loop_run_health_policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
    pub control_health_policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy,
    pub mode: AgentClosedLoopNextTurnMode,
    pub can_schedule: bool,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub transition_health_status: AgentClosedLoopExecutionHealthStatus,
    pub control_health_status: AgentClosedLoopExecutionHealthStatus,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation {
    pub fn from_record(record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRecord) -> Self {
        let mode = record.mode();
        let can_schedule = record.can_schedule();
        let side_effect_dispatch_allowed_rate = record
            .control_summary_history_record
            .dashboard
            .side_effect_dispatch_allowed_rate;
        let memory_note_allowed_rate = record
            .control_summary_history_record
            .dashboard
            .memory_note_allowed_rate;
        let allows_adaptive_evolution = record.allows_adaptive_evolution();
        let requires_repair_first = record.requires_repair_first();
        let transition_health_status = record.control_record.history_record.health.status;
        let control_health_status = record.control_summary_history_record.health.status;
        let telemetry = service_loop_run_daemon_continuation_telemetry(
            record,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            transition_health_status,
            control_health_status,
        );

        Self {
            next_runtime_input: record.next_runtime_input().clone(),
            service_run_history: record.loop_run.advance.run_record.history.clone(),
            loop_run_history: record.control_record.history_record.history.clone(),
            control_summary_history: record.control_summary_history_record.history.clone(),
            service_run_policy: record.service_run_policy,
            loop_run_health_policy: record.loop_run_health_policy,
            control_health_policy: record.control_health_policy,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            transition_health_status,
            control_health_status,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonContinuationPlanner;

impl AgentClosedLoopRuntimeServiceLoopRunDaemonContinuationPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRecord,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation {
        record.continuation()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan {
    pub request_input: AgentClosedLoopRuntimeServiceRequestInput,
    pub continuation_input: AgentClosedLoopRuntimeContinuationInput,
    pub service_run_history: AgentClosedLoopRuntimeServiceRunHistory,
    pub loop_run_history: AgentClosedLoopRuntimeServiceLoopRunHistory,
    pub control_summary_history: AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory,
    pub service_run_policy: AgentClosedLoopRuntimeServiceRunHealthPolicy,
    pub loop_run_health_policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
    pub control_health_policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy,
    pub mode: AgentClosedLoopNextTurnMode,
    pub can_schedule: bool,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan {
    pub fn with_receipts(
        self,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan {
        AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner::new()
            .plan_from_request_plan(self, receipts)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord {
    pub request_plan: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
    pub dispatch: AgentClosedLoopRuntimeServiceDispatch,
    pub dispatch_summary: AgentClosedLoopRuntimeServiceDispatchSummary,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord {
    pub fn summary(&self) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary {
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary::from_record(self)
    }

    pub fn is_executable(&self) -> bool {
        self.dispatch.is_executable()
    }

    pub fn command_plan(&self) -> Option<&AgentServiceCommandPlan> {
        self.dispatch.command_plan()
    }

    pub fn command_kinds(&self) -> Vec<&'static str> {
        self.dispatch.request.command_kinds()
    }

    pub fn skipped_reasons(&self) -> &[String] {
        self.dispatch.request.skipped_reasons()
    }

    pub fn close_with_receipts(
        self,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRecord {
        AgentClosedLoopRuntimeServiceLoopRunDaemonReceiptCloser::new().close(self, receipts)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary {
    pub executable: bool,
    pub command_count: usize,
    pub command_gate_allowed: bool,
    pub side_effect_gate_count: usize,
    pub blocked_side_effect_gate_count: usize,
    pub command_kinds: Vec<String>,
    pub mode: AgentClosedLoopNextTurnMode,
    pub can_schedule: bool,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub service_attempts: usize,
    pub transitions: usize,
    pub control_records: usize,
    pub blocked_reasons: Vec<String>,
    pub skipped_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary {
    pub fn from_record(record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord) -> Self {
        let request_plan = &record.request_plan;
        let dispatch_summary = &record.dispatch_summary;
        let skipped_reasons = record.skipped_reasons().to_vec();
        let telemetry = service_loop_run_daemon_request_summary_telemetry(
            request_plan,
            dispatch_summary,
            &skipped_reasons,
        );

        Self {
            executable: dispatch_summary.executable,
            command_count: dispatch_summary.command_count,
            command_gate_allowed: dispatch_summary.command_gate_allowed,
            side_effect_gate_count: dispatch_summary.side_effect_gate_count,
            blocked_side_effect_gate_count: dispatch_summary.blocked_side_effect_gate_count,
            command_kinds: dispatch_summary.command_kinds.clone(),
            mode: request_plan.mode,
            can_schedule: request_plan.can_schedule,
            side_effect_dispatch_allowed_rate: request_plan.side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate: request_plan.memory_note_allowed_rate,
            allows_adaptive_evolution: request_plan.allows_adaptive_evolution,
            requires_repair_first: request_plan.requires_repair_first,
            service_attempts: request_plan.service_run_history.len(),
            transitions: request_plan.loop_run_history.len(),
            control_records: request_plan.control_summary_history.len(),
            blocked_reasons: dispatch_summary.blocked_reasons.clone(),
            skipped_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory {
    summaries: Vec<AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary) {
        self.summaries.push(summary);
    }

    pub fn summaries(&self) -> &[AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary] {
        &self.summaries
    }

    pub fn latest(&self) -> Option<&AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(&self) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestDashboard {
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestDashboard {
    pub total_records: usize,
    pub executable_records: usize,
    pub command_gate_allowed_records: usize,
    pub blocked_records: usize,
    pub skipped_records: usize,
    pub repair_first_records: usize,
    pub adaptive_allowed_records: usize,
    pub executable_rate: f32,
    pub command_gate_allowed_rate: f32,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub adaptive_allowed_rate: f32,
    pub total_command_count: usize,
    pub side_effect_gate_count: usize,
    pub blocked_side_effect_gate_count: usize,
    pub latest_executable: Option<bool>,
    pub latest_mode: Option<AgentClosedLoopNextTurnMode>,
    pub latest_command_count: Option<usize>,
    pub latest_blocked_reasons: Vec<String>,
    pub latest_skipped_reasons: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestDashboard {
    pub fn from_summaries(
        summaries: &[AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary],
    ) -> Self {
        let total_records = summaries.len();
        let executable_records = summaries
            .iter()
            .filter(|summary| summary.executable)
            .count();
        let command_gate_allowed_records = summaries
            .iter()
            .filter(|summary| summary.command_gate_allowed)
            .count();
        let blocked_records = summaries
            .iter()
            .filter(|summary| !summary.blocked_reasons.is_empty())
            .count();
        let skipped_records = summaries
            .iter()
            .filter(|summary| !summary.skipped_reasons.is_empty())
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let adaptive_allowed_records = summaries
            .iter()
            .filter(|summary| summary.allows_adaptive_evolution)
            .count();
        let total_command_count = summaries
            .iter()
            .map(|summary| summary.command_count)
            .sum::<usize>();
        let side_effect_gate_count = summaries
            .iter()
            .map(|summary| summary.side_effect_gate_count)
            .sum::<usize>();
        let blocked_side_effect_gate_count = summaries
            .iter()
            .map(|summary| summary.blocked_side_effect_gate_count)
            .sum::<usize>();
        let side_effect_dispatch_allowed_rate = average_service_run_rate(summaries, |summary| {
            summary.side_effect_dispatch_allowed_rate
        });
        let memory_note_allowed_rate =
            average_service_run_rate(summaries, |summary| summary.memory_note_allowed_rate);
        let latest = summaries.last();

        Self {
            total_records,
            executable_records,
            command_gate_allowed_records,
            blocked_records,
            skipped_records,
            repair_first_records,
            adaptive_allowed_records,
            executable_rate: service_run_rate(executable_records, total_records),
            command_gate_allowed_rate: service_run_rate(
                command_gate_allowed_records,
                total_records,
            ),
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            adaptive_allowed_rate: service_run_rate(adaptive_allowed_records, total_records),
            total_command_count,
            side_effect_gate_count,
            blocked_side_effect_gate_count,
            latest_executable: latest.map(|summary| summary.executable),
            latest_mode: latest.map(|summary| summary.mode),
            latest_command_count: latest.map(|summary| summary.command_count),
            latest_blocked_reasons: latest
                .map(|summary| summary.blocked_reasons.clone())
                .unwrap_or_default(),
            latest_skipped_reasons: latest
                .map(|summary| summary.skipped_reasons.clone())
                .unwrap_or_default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn is_clean(&self) -> bool {
        self.blocked_records == 0 && self.skipped_records == 0
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth {
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy {
    pub minimum_executable_rate: f32,
    pub minimum_adaptive_allowed_rate: f32,
    pub maximum_blocked_records: usize,
    pub maximum_skipped_records: usize,
    pub maximum_repair_first_records: usize,
}

impl Default for AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_executable_rate: 0.67,
            minimum_adaptive_allowed_rate: 0.50,
            maximum_blocked_records: 0,
            maximum_skipped_records: 0,
            maximum_repair_first_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestDashboard,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth {
    pub fn from_dashboard(
        dashboard: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestDashboard,
        policy: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("service_loop_run_daemon_request_history_empty".to_owned());
        }

        if !dashboard.is_empty() && dashboard.executable_rate < policy.minimum_executable_rate {
            watch_reasons.push(format!(
                "service_loop_run_daemon_request_executable_rate={:.3}<{}",
                dashboard.executable_rate, policy.minimum_executable_rate
            ));
        }

        if !dashboard.is_empty()
            && dashboard.adaptive_allowed_rate < policy.minimum_adaptive_allowed_rate
        {
            watch_reasons.push(format!(
                "service_loop_run_daemon_request_adaptive_allowed_rate={:.3}<{}",
                dashboard.adaptive_allowed_rate, policy.minimum_adaptive_allowed_rate
            ));
        }

        if dashboard.blocked_records > policy.maximum_blocked_records {
            repair_reasons.push(format!(
                "service_loop_run_daemon_request_blocked_records={}>{}",
                dashboard.blocked_records, policy.maximum_blocked_records
            ));
        }

        if dashboard.skipped_records > policy.maximum_skipped_records {
            repair_reasons.push(format!(
                "service_loop_run_daemon_request_skipped_records={}>{}",
                dashboard.skipped_records, policy.maximum_skipped_records
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "service_loop_run_daemon_request_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if !dashboard.latest_blocked_reasons.is_empty() {
            watch_reasons.push(format!(
                "service_loop_run_daemon_request_latest_blocked={}",
                dashboard.latest_blocked_reasons.join(";")
            ));
        }

        if !dashboard.latest_skipped_reasons.is_empty() {
            watch_reasons.push(format!(
                "service_loop_run_daemon_request_latest_skipped={}",
                dashboard.latest_skipped_reasons.join(";")
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
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecord {
    pub history: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory,
    pub appended_summary: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary,
    pub dashboard: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestDashboard,
    pub health: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecord {
    pub fn latest(&self) -> &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary {
        &self.appended_summary
    }

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
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecorder;

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record(
        &self,
        mut history: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory,
        summary: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary,
        policy: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecord {
        let appended_summary = summary;
        history.push(appended_summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = service_loop_run_daemon_request_summary_history_record_telemetry(
            &appended_summary,
            &dashboard,
            &health,
        );

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecord {
            history,
            appended_summary,
            dashboard,
            health,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord {
    pub request_history_record:
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecord,
    pub daemon_record: AgentClosedLoopRuntimeServiceLoopRunDaemonRecord,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord {
    pub fn latest_request_summary(
        &self,
    ) -> &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary {
        self.request_history_record.latest()
    }

    pub fn mode(&self) -> AgentClosedLoopNextTurnMode {
        self.daemon_record.mode()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.daemon_record.requires_repair_first()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.daemon_record.allows_service_advance()
    }

    pub fn summary(
        &self,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary {
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary::from_close_record(
            self,
        )
    }

    pub fn continuation(
        &self,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation {
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation::from_close_record(
            self,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary {
    pub latest_request_executable: bool,
    pub latest_request_command_count: usize,
    pub latest_request_command_gate_allowed: bool,
    pub latest_request_side_effect_gate_count: usize,
    pub latest_request_blocked_side_effect_gate_count: usize,
    pub latest_request_mode: AgentClosedLoopNextTurnMode,
    pub request_health_status: AgentClosedLoopExecutionHealthStatus,
    pub daemon_run_status: AgentClosedLoopRuntimeServiceRunStatus,
    pub daemon_control_health_status: AgentClosedLoopExecutionHealthStatus,
    pub mode: AgentClosedLoopNextTurnMode,
    pub can_schedule: bool,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub request_records: usize,
    pub service_attempts: usize,
    pub transitions: usize,
    pub control_records: usize,
    pub blocked_reasons: Vec<String>,
    pub skipped_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary {
    pub fn from_close_record(
        close_record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord,
    ) -> Self {
        let latest_request_summary = close_record.latest_request_summary();
        let daemon_run_summary = &close_record.daemon_record.loop_run.run.run_summary;
        let mut blocked_reasons = latest_request_summary.blocked_reasons.clone();
        extend_unique(&mut blocked_reasons, &daemon_run_summary.blocked_reasons());
        extend_unique(
            &mut blocked_reasons,
            &close_record
                .daemon_record
                .control_record
                .control_plan
                .reasons,
        );
        let request_health_status = close_record.request_history_record.health.status;
        let daemon_control_health_status = close_record
            .daemon_record
            .control_summary_history_record
            .health
            .status;
        let mode = close_record.mode();
        let can_schedule = close_record.daemon_record.can_schedule();
        let side_effect_dispatch_allowed_rate =
            latest_request_summary.side_effect_dispatch_allowed_rate;
        let memory_note_allowed_rate = latest_request_summary.memory_note_allowed_rate;
        let allows_adaptive_evolution = close_record.daemon_record.allows_adaptive_evolution();
        let requires_repair_first = close_record.requires_repair_first();
        let service_attempts = close_record
            .daemon_record
            .loop_run
            .advance
            .run_record
            .attempts();
        let transitions = close_record.daemon_record.control_record.transitions();
        let control_records = close_record
            .daemon_record
            .control_summary_history_record
            .records();
        let telemetry = service_loop_run_daemon_request_monitored_close_summary_telemetry(
            latest_request_summary,
            request_health_status,
            daemon_run_summary.status,
            daemon_control_health_status,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            close_record.request_history_record.records(),
            service_attempts,
            transitions,
            control_records,
            &blocked_reasons,
            &latest_request_summary.skipped_reasons,
        );

        Self {
            latest_request_executable: latest_request_summary.executable,
            latest_request_command_count: latest_request_summary.command_count,
            latest_request_command_gate_allowed: latest_request_summary.command_gate_allowed,
            latest_request_side_effect_gate_count: latest_request_summary.side_effect_gate_count,
            latest_request_blocked_side_effect_gate_count: latest_request_summary
                .blocked_side_effect_gate_count,
            latest_request_mode: latest_request_summary.mode,
            request_health_status,
            daemon_run_status: daemon_run_summary.status,
            daemon_control_health_status,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            request_records: close_record.request_history_record.records(),
            service_attempts,
            transitions,
            control_records,
            blocked_reasons,
            skipped_reasons: latest_request_summary.skipped_reasons.clone(),
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory {
    summaries: Vec<AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(
        &mut self,
        summary: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary,
    ) {
        self.summaries.push(summary);
    }

    pub fn summaries(
        &self,
    ) -> &[AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary] {
        &self.summaries
    }

    pub fn latest(
        &self,
    ) -> Option<&AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn dashboard(
        &self,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseDashboard {
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseDashboard::from_summaries(
            &self.summaries,
        )
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseDashboard {
    pub total_records: usize,
    pub request_executable_records: usize,
    pub request_command_gate_allowed_records: usize,
    pub daemon_closed_records: usize,
    pub request_repair_records: usize,
    pub daemon_control_repair_records: usize,
    pub repair_first_records: usize,
    pub adaptive_allowed_records: usize,
    pub request_executable_rate: f32,
    pub request_command_gate_allowed_rate: f32,
    pub daemon_closed_rate: f32,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub adaptive_allowed_rate: f32,
    pub request_side_effect_gate_count: usize,
    pub request_blocked_side_effect_gate_count: usize,
    pub latest_request_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
    pub latest_daemon_run_status: Option<AgentClosedLoopRuntimeServiceRunStatus>,
    pub latest_daemon_control_health_status: Option<AgentClosedLoopExecutionHealthStatus>,
    pub latest_mode: Option<AgentClosedLoopNextTurnMode>,
    pub latest_blocked_reasons: Vec<String>,
    pub latest_skipped_reasons: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseDashboard {
    pub fn from_summaries(
        summaries: &[AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary],
    ) -> Self {
        let total_records = summaries.len();
        let request_executable_records = summaries
            .iter()
            .filter(|summary| summary.latest_request_executable)
            .count();
        let request_command_gate_allowed_records = summaries
            .iter()
            .filter(|summary| summary.latest_request_command_gate_allowed)
            .count();
        let daemon_closed_records = summaries
            .iter()
            .filter(|summary| {
                summary.daemon_run_status == AgentClosedLoopRuntimeServiceRunStatus::Closed
            })
            .count();
        let request_repair_records = summaries
            .iter()
            .filter(|summary| {
                summary.request_health_status == AgentClosedLoopExecutionHealthStatus::Repair
            })
            .count();
        let daemon_control_repair_records = summaries
            .iter()
            .filter(|summary| {
                summary.daemon_control_health_status == AgentClosedLoopExecutionHealthStatus::Repair
            })
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let adaptive_allowed_records = summaries
            .iter()
            .filter(|summary| summary.allows_adaptive_evolution)
            .count();
        let request_side_effect_gate_count = summaries
            .iter()
            .map(|summary| summary.latest_request_side_effect_gate_count)
            .sum::<usize>();
        let request_blocked_side_effect_gate_count = summaries
            .iter()
            .map(|summary| summary.latest_request_blocked_side_effect_gate_count)
            .sum::<usize>();
        let side_effect_dispatch_allowed_rate = average_service_run_rate(summaries, |summary| {
            summary.side_effect_dispatch_allowed_rate
        });
        let memory_note_allowed_rate =
            average_service_run_rate(summaries, |summary| summary.memory_note_allowed_rate);
        let latest = summaries.last();

        Self {
            total_records,
            request_executable_records,
            request_command_gate_allowed_records,
            daemon_closed_records,
            request_repair_records,
            daemon_control_repair_records,
            repair_first_records,
            adaptive_allowed_records,
            request_executable_rate: service_run_rate(request_executable_records, total_records),
            request_command_gate_allowed_rate: service_run_rate(
                request_command_gate_allowed_records,
                total_records,
            ),
            daemon_closed_rate: service_run_rate(daemon_closed_records, total_records),
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            adaptive_allowed_rate: service_run_rate(adaptive_allowed_records, total_records),
            request_side_effect_gate_count,
            request_blocked_side_effect_gate_count,
            latest_request_health_status: latest.map(|summary| summary.request_health_status),
            latest_daemon_run_status: latest.map(|summary| summary.daemon_run_status),
            latest_daemon_control_health_status: latest
                .map(|summary| summary.daemon_control_health_status),
            latest_mode: latest.map(|summary| summary.mode),
            latest_blocked_reasons: latest
                .map(|summary| summary.blocked_reasons.clone())
                .unwrap_or_default(),
            latest_skipped_reasons: latest
                .map(|summary| summary.skipped_reasons.clone())
                .unwrap_or_default(),
        }
    }

    pub fn health(
        &self,
        policy: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth {
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy {
    pub minimum_request_executable_rate: f32,
    pub minimum_daemon_closed_rate: f32,
    pub minimum_adaptive_allowed_rate: f32,
    pub maximum_request_repair_records: usize,
    pub maximum_daemon_control_repair_records: usize,
    pub maximum_repair_first_records: usize,
}

impl Default for AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_request_executable_rate: 0.5,
            minimum_daemon_closed_rate: 0.5,
            minimum_adaptive_allowed_rate: 0.25,
            maximum_request_repair_records: 0,
            maximum_daemon_control_repair_records: 0,
            maximum_repair_first_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth {
    pub status: AgentClosedLoopExecutionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseDashboard,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth {
    pub fn from_dashboard(
        dashboard: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseDashboard,
        policy: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.total_records == 0 {
            watch_reasons
                .push("service_loop_run_daemon_request_monitored_close_records=0".to_owned());
        } else {
            if dashboard.request_executable_rate < policy.minimum_request_executable_rate {
                watch_reasons.push(format!(
                    "service_loop_run_daemon_request_monitored_close_executable_rate={:.3}<{}",
                    dashboard.request_executable_rate, policy.minimum_request_executable_rate
                ));
            }

            if dashboard.daemon_closed_rate < policy.minimum_daemon_closed_rate {
                watch_reasons.push(format!(
                    "service_loop_run_daemon_request_monitored_close_closed_rate={:.3}<{}",
                    dashboard.daemon_closed_rate, policy.minimum_daemon_closed_rate
                ));
            }

            if dashboard.adaptive_allowed_rate < policy.minimum_adaptive_allowed_rate {
                watch_reasons.push(format!(
                    "service_loop_run_daemon_request_monitored_close_adaptive_allowed_rate={:.3}<{}",
                    dashboard.adaptive_allowed_rate, policy.minimum_adaptive_allowed_rate
                ));
            }
        }

        if dashboard.request_repair_records > policy.maximum_request_repair_records {
            repair_reasons.push(format!(
                "service_loop_run_daemon_request_monitored_close_request_repair_records={}>{}",
                dashboard.request_repair_records, policy.maximum_request_repair_records
            ));
        }

        if dashboard.daemon_control_repair_records > policy.maximum_daemon_control_repair_records {
            repair_reasons.push(format!(
                "service_loop_run_daemon_request_monitored_close_control_repair_records={}>{}",
                dashboard.daemon_control_repair_records,
                policy.maximum_daemon_control_repair_records
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "service_loop_run_daemon_request_monitored_close_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if !dashboard.latest_blocked_reasons.is_empty() {
            watch_reasons.push(format!(
                "service_loop_run_daemon_request_monitored_close_latest_blocked={}",
                dashboard.latest_blocked_reasons.join(";")
            ));
        }

        if !dashboard.latest_skipped_reasons.is_empty() {
            watch_reasons.push(format!(
                "service_loop_run_daemon_request_monitored_close_latest_skipped={}",
                dashboard.latest_skipped_reasons.join(";")
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
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecord {
    pub history: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory,
    pub appended_summary: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary,
    pub dashboard: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseDashboard,
    pub health: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecord {
    pub fn latest(
        &self,
    ) -> &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary {
        &self.appended_summary
    }

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
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder;

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record(
        &self,
        mut history: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory,
        summary: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary,
        policy: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecord {
        let appended_summary = summary;
        history.push(appended_summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            service_loop_run_daemon_request_monitored_close_summary_history_record_telemetry(
                &appended_summary,
                &dashboard,
                &health,
            );

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecord {
            history,
            appended_summary,
            dashboard,
            health,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation {
    pub monitored_continuation:
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation,
    pub monitored_close_summary_history:
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory,
    pub monitored_close_health:
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth,
    pub request_health_status: AgentClosedLoopExecutionHealthStatus,
    pub daemon_control_health_status: AgentClosedLoopExecutionHealthStatus,
    pub mode: AgentClosedLoopNextTurnMode,
    pub can_schedule: bool,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation {
    pub fn from_records(
        close_record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord,
        close_history_record:
            &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecord,
    ) -> Self {
        let monitored_continuation = close_record.continuation();
        let monitored_close_summary_history = close_history_record.history.clone();
        let monitored_close_health = close_history_record.health.clone();
        let request_health_status = monitored_continuation.request_health.status;
        let daemon_control_health_status = monitored_continuation.daemon_control_health.status;
        let mode = monitored_continuation.mode;
        let can_schedule = monitored_continuation.can_schedule;
        let side_effect_dispatch_allowed_rate =
            monitored_continuation.side_effect_dispatch_allowed_rate;
        let memory_note_allowed_rate = monitored_continuation.memory_note_allowed_rate;
        let allows_adaptive_evolution = monitored_continuation.allows_adaptive_evolution;
        let requires_repair_first = monitored_continuation.requires_repair_first;
        let telemetry = service_loop_run_daemon_request_monitored_close_continuation_telemetry(
            &monitored_close_health,
            request_health_status,
            daemon_control_health_status,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            monitored_continuation.request_summary_history.len(),
            monitored_close_summary_history.len(),
        );

        Self {
            monitored_continuation,
            monitored_close_summary_history,
            monitored_close_health,
            request_health_status,
            daemon_control_health_status,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuationPlanner;

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuationPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        close_record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord,
        close_history_record:
            &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecord,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation {
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation::from_records(
            close_record,
            close_history_record,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation {
    pub daemon_continuation: AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
    pub request_summary_history: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory,
    pub request_health: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth,
    pub daemon_control_health: AgentClosedLoopRuntimeServiceLoopRunControlHealth,
    pub mode: AgentClosedLoopNextTurnMode,
    pub can_schedule: bool,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation {
    pub fn from_close_record(
        close_record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord,
    ) -> Self {
        let daemon_continuation = close_record.daemon_record.continuation();
        let request_summary_history = close_record.request_history_record.history.clone();
        let request_health = close_record.request_history_record.health.clone();
        let daemon_control_health = close_record
            .daemon_record
            .control_summary_history_record
            .health
            .clone();
        let mode = close_record.mode();
        let can_schedule = close_record.daemon_record.can_schedule();
        let side_effect_dispatch_allowed_rate =
            daemon_continuation.side_effect_dispatch_allowed_rate;
        let memory_note_allowed_rate = daemon_continuation.memory_note_allowed_rate;
        let allows_adaptive_evolution = close_record.daemon_record.allows_adaptive_evolution();
        let requires_repair_first = close_record.requires_repair_first();
        let telemetry = service_loop_run_daemon_request_monitored_continuation_telemetry(
            &request_health,
            &daemon_control_health,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            request_summary_history.len(),
            daemon_continuation.control_summary_history.len(),
        );

        Self {
            daemon_continuation,
            request_summary_history,
            request_health,
            daemon_control_health,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuationPlanner;

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuationPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        close_record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation {
        close_record.continuation()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan {
    pub request_plan: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
    pub request_summary_history: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory,
    pub request_health: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth,
    pub daemon_control_health: AgentClosedLoopRuntimeServiceLoopRunControlHealth,
    pub mode: AgentClosedLoopNextTurnMode,
    pub can_schedule: bool,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan {
    pub fn request_history(
        &self,
    ) -> &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory {
        &self.request_summary_history
    }

    pub fn into_request_parts(
        self,
    ) -> (
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory,
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
    ) {
        (self.request_summary_history, self.request_plan)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlanner {
    request_planner: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn plan(
        &self,
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation,
        business_input: AgentClosedLoopRuntimeBusinessInput,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan {
        let request_plan = self
            .request_planner
            .plan(&continuation.daemon_continuation, business_input);
        let side_effect_dispatch_allowed_rate = request_plan.side_effect_dispatch_allowed_rate;
        let memory_note_allowed_rate = request_plan.memory_note_allowed_rate;
        let telemetry =
            service_loop_run_daemon_request_monitored_plan_telemetry(continuation, &request_plan);

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan {
            request_plan,
            request_summary_history: continuation.request_summary_history.clone(),
            request_health: continuation.request_health.clone(),
            daemon_control_health: continuation.daemon_control_health.clone(),
            mode: continuation.mode,
            can_schedule: continuation.can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution: continuation.allows_adaptive_evolution,
            requires_repair_first: continuation.requires_repair_first,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan {
    pub monitored_plan: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
    pub monitored_close_summary_history:
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory,
    pub monitored_close_health:
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth,
    pub request_health_status: AgentClosedLoopExecutionHealthStatus,
    pub daemon_control_health_status: AgentClosedLoopExecutionHealthStatus,
    pub mode: AgentClosedLoopNextTurnMode,
    pub can_schedule: bool,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan {
    pub fn request_history(
        &self,
    ) -> &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory {
        self.monitored_plan.request_history()
    }

    pub fn monitored_close_history(
        &self,
    ) -> &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory {
        &self.monitored_close_summary_history
    }

    pub fn into_monitored_parts(
        self,
    ) -> (
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory,
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
    ) {
        (self.monitored_close_summary_history, self.monitored_plan)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlanner {
    monitored_planner: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlanner,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn plan(
        &self,
        close_continuation:
            &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation,
        business_input: AgentClosedLoopRuntimeBusinessInput,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan {
        let monitored_plan = self
            .monitored_planner
            .plan(&close_continuation.monitored_continuation, business_input);
        let telemetry = service_loop_run_daemon_request_monitored_close_plan_telemetry(
            close_continuation,
            &monitored_plan,
        );

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan {
            monitored_plan,
            monitored_close_summary_history: close_continuation
                .monitored_close_summary_history
                .clone(),
            monitored_close_health: close_continuation.monitored_close_health.clone(),
            request_health_status: close_continuation.request_health_status,
            daemon_control_health_status: close_continuation.daemon_control_health_status,
            mode: close_continuation.mode,
            can_schedule: close_continuation.can_schedule,
            side_effect_dispatch_allowed_rate: close_continuation.side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate: close_continuation.memory_note_allowed_rate,
            allows_adaptive_evolution: close_continuation.allows_adaptive_evolution,
            requires_repair_first: close_continuation.requires_repair_first,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord {
    pub monitored_close_plan: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
    pub monitored_record: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord {
    pub fn request_history(
        &self,
    ) -> &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory {
        self.monitored_record.request_history()
    }

    pub fn monitored_close_history(
        &self,
    ) -> &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory {
        self.monitored_close_plan.monitored_close_history()
    }

    pub fn is_executable(&self) -> bool {
        self.monitored_record.is_executable()
    }

    pub fn command_plan(&self) -> Option<&AgentServiceCommandPlan> {
        self.monitored_record.command_plan()
    }

    pub fn summary(
        &self,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunSummary {
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunSummary::from_record(self)
    }

    pub fn close_with_receipts(
        self,
        request_health_policy: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy,
        monitored_close_health_policy:
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation {
        let AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord {
            monitored_close_plan,
            monitored_record,
            ..
        } = self;
        let monitored_close_summary_history = monitored_close_plan.monitored_close_summary_history;
        let close_record = monitored_record.close_with_receipts(request_health_policy, receipts);
        let close_history_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder::new()
                .record(
                    monitored_close_summary_history,
                    close_record.summary(),
                    monitored_close_health_policy,
                );

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuationPlanner::new()
            .plan(&close_record, &close_history_record)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunSummary {
    pub executable: bool,
    pub command_count: usize,
    pub command_gate_allowed: bool,
    pub side_effect_gate_count: usize,
    pub blocked_side_effect_gate_count: usize,
    pub command_kinds: Vec<String>,
    pub request_records: usize,
    pub monitored_close_records: usize,
    pub monitored_close_health_status: AgentClosedLoopExecutionHealthStatus,
    pub request_health_status: AgentClosedLoopExecutionHealthStatus,
    pub daemon_control_health_status: AgentClosedLoopExecutionHealthStatus,
    pub mode: AgentClosedLoopNextTurnMode,
    pub can_schedule: bool,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub allows_adaptive_evolution: bool,
    pub requires_repair_first: bool,
    pub blocked_reasons: Vec<String>,
    pub skipped_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunSummary {
    pub fn from_record(
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord,
    ) -> Self {
        let request_summary = record.monitored_record.summary();
        let monitored_close_plan = &record.monitored_close_plan;
        let telemetry = service_loop_run_daemon_request_monitored_close_run_summary_telemetry(
            &request_summary,
            monitored_close_plan,
        );

        Self {
            executable: request_summary.executable,
            command_count: request_summary.command_count,
            command_gate_allowed: request_summary.command_gate_allowed,
            side_effect_gate_count: request_summary.side_effect_gate_count,
            blocked_side_effect_gate_count: request_summary.blocked_side_effect_gate_count,
            command_kinds: request_summary.command_kinds,
            request_records: monitored_close_plan.request_history().len(),
            monitored_close_records: monitored_close_plan.monitored_close_history().len(),
            monitored_close_health_status: monitored_close_plan.monitored_close_health.status,
            request_health_status: monitored_close_plan.request_health_status,
            daemon_control_health_status: monitored_close_plan.daemon_control_health_status,
            mode: monitored_close_plan.mode,
            can_schedule: monitored_close_plan.can_schedule,
            side_effect_dispatch_allowed_rate: monitored_close_plan
                .side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate: monitored_close_plan.memory_note_allowed_rate,
            allows_adaptive_evolution: monitored_close_plan.allows_adaptive_evolution,
            requires_repair_first: monitored_close_plan.requires_repair_first,
            blocked_reasons: request_summary.blocked_reasons,
            skipped_reasons: request_summary.skipped_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunner {
    monitored_runner: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRunner,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run<E, P>(
        &self,
        monitored_close_plan: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
        engine: &mut E,
        memory: &mut P,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord
    where
        E: EnginePort,
        E::Error: ToString,
        P: MemoryPort,
        P::Error: ToString,
    {
        let monitored_record =
            self.monitored_runner
                .run(monitored_close_plan.monitored_plan.clone(), engine, memory);
        let telemetry = service_loop_run_daemon_request_monitored_close_run_record_telemetry(
            &monitored_close_plan,
            &monitored_record,
        );

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord {
            monitored_close_plan,
            monitored_record,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord {
    pub monitored_plan: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
    pub request_record: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord {
    pub fn request_history(
        &self,
    ) -> &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory {
        &self.monitored_plan.request_summary_history
    }

    pub fn summary(&self) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary {
        self.request_record.summary()
    }

    pub fn is_executable(&self) -> bool {
        self.request_record.is_executable()
    }

    pub fn command_plan(&self) -> Option<&AgentServiceCommandPlan> {
        self.request_record.command_plan()
    }

    pub fn close_with_receipts(
        self,
        request_health_policy: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord {
        let AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord {
            monitored_plan,
            request_record,
            ..
        } = self;
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
            monitored_plan.request_summary_history,
            request_record,
            request_health_policy,
            receipts,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRunner {
    request_runner: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run<E, P>(
        &self,
        monitored_plan: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
        engine: &mut E,
        memory: &mut P,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord
    where
        E: EnginePort,
        E::Error: ToString,
        P: MemoryPort,
        P::Error: ToString,
    {
        let request_record =
            self.request_runner
                .run(monitored_plan.request_plan.clone(), engine, memory);
        let telemetry = service_loop_run_daemon_request_monitored_record_telemetry(
            &monitored_plan,
            &request_record,
        );

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord {
            monitored_plan,
            request_record,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser {
    request_summary_recorder:
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecorder,
    receipt_closer: AgentClosedLoopRuntimeServiceLoopRunDaemonReceiptCloser,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn close(
        &self,
        request_history: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory,
        request_record: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord,
        request_health_policy: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord {
        let request_summary = request_record.summary();
        let request_history_record = self.request_summary_recorder.record(
            request_history,
            request_summary,
            request_health_policy,
        );
        let daemon_record = self.receipt_closer.close(request_record, receipts);
        let telemetry = service_loop_run_daemon_request_monitored_close_telemetry(
            &request_history_record,
            &daemon_record,
        );

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord {
            request_history_record,
            daemon_record,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner;

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
        business_input: AgentClosedLoopRuntimeBusinessInput,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan {
        let continuation_input =
            runtime_continuation_input_from_runtime_input(&continuation.next_runtime_input);
        self.plan_with_continuation_input(continuation, business_input, continuation_input)
    }

    pub fn plan_with_continuation_input(
        &self,
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
        business_input: AgentClosedLoopRuntimeBusinessInput,
        continuation_input: AgentClosedLoopRuntimeContinuationInput,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan {
        let request_input = AgentClosedLoopRuntimeServiceRequestInput::new(
            continuation.next_runtime_input.clone(),
            business_input,
        );
        let telemetry = service_loop_run_daemon_request_plan_telemetry(
            continuation,
            &request_input,
            &continuation_input,
        );

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan {
            request_input,
            continuation_input,
            service_run_history: continuation.service_run_history.clone(),
            loop_run_history: continuation.loop_run_history.clone(),
            control_summary_history: continuation.control_summary_history.clone(),
            service_run_policy: continuation.service_run_policy,
            loop_run_health_policy: continuation.loop_run_health_policy,
            control_health_policy: continuation.control_health_policy,
            mode: continuation.mode,
            can_schedule: continuation.can_schedule,
            side_effect_dispatch_allowed_rate: continuation.side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate: continuation.memory_note_allowed_rate,
            allows_adaptive_evolution: continuation.allows_adaptive_evolution,
            requires_repair_first: continuation.requires_repair_first,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner {
    request_runner: AgentClosedLoopRuntimeServiceRequestRunner,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run<E, P>(
        &self,
        request_plan: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
        engine: &mut E,
        memory: &mut P,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord
    where
        E: EnginePort,
        E::Error: ToString,
        P: MemoryPort,
        P::Error: ToString,
    {
        let request = self
            .request_runner
            .run(request_plan.request_input.clone(), engine, memory);
        let dispatch = request.into_dispatch();
        let dispatch_summary = dispatch.summary();
        let telemetry = service_loop_run_daemon_request_record_telemetry(
            &request_plan,
            &dispatch,
            &dispatch_summary,
        );

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord {
            request_plan,
            dispatch,
            dispatch_summary,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonReceiptCloser {
    continuation_planner: AgentClosedLoopRuntimeServiceDispatchContinuationPlanner,
    advance_planner: AgentClosedLoopRuntimeServiceLoopAdvancePlanner,
    monitor: AgentClosedLoopRuntimeServiceLoopRunMonitor,
    control_summary_recorder: AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder,
}

impl AgentClosedLoopRuntimeServiceLoopRunDaemonReceiptCloser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn close(
        &self,
        request_record: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRecord {
        let request_record_telemetry = request_record.telemetry.clone();
        let AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord {
            request_plan,
            dispatch,
            ..
        } = request_record;
        let AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan {
            continuation_input,
            service_run_history,
            loop_run_history,
            control_summary_history,
            service_run_policy,
            loop_run_health_policy,
            control_health_policy,
            ..
        } = request_plan;
        let dispatch_outcome = dispatch.close_with_intake(receipts, continuation_input.clone());
        let dispatch_continuation = self
            .continuation_planner
            .plan(dispatch_outcome, continuation_input);
        let summary = dispatch_continuation.summary();
        let run_summary =
            AgentClosedLoopRuntimeServiceRunSummary::from_continuation(&dispatch_continuation);
        let run_telemetry = service_run_telemetry(&dispatch_continuation, &run_summary);
        let run = AgentClosedLoopRuntimeServiceRun {
            dispatch_continuation,
            summary,
            run_summary,
            telemetry: run_telemetry,
        };
        let advance = self
            .advance_planner
            .advance(service_run_history, &run, service_run_policy);
        let loop_run_telemetry = service_loop_run_telemetry(&run, &advance);
        let loop_run = AgentClosedLoopRuntimeServiceLoopRun {
            run,
            advance,
            telemetry: loop_run_telemetry,
        };
        let control_record =
            self.monitor
                .record_and_plan(loop_run_history, &loop_run, loop_run_health_policy);
        let control_summary = control_record.summary();
        let control_summary_history_record = self.control_summary_recorder.record(
            control_summary_history,
            control_summary.clone(),
            control_health_policy,
        );
        let mut telemetry = request_record_telemetry;
        telemetry.extend(service_loop_run_daemon_record_telemetry(
            &loop_run,
            &control_record,
            &control_summary_history_record,
        ));
        telemetry.push("service_loop_run_daemon_receipt_close=true".to_owned());

        AgentClosedLoopRuntimeServiceLoopRunDaemonRecord {
            loop_run,
            control_record,
            control_summary,
            control_summary_history_record,
            service_run_policy,
            loop_run_health_policy,
            control_health_policy,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan {
    pub input: AgentClosedLoopRuntimeServiceLoopRunDaemonInput,
    pub side_effect_dispatch_allowed_rate: f32,
    pub memory_note_allowed_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner;

impl AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
        business_input: AgentClosedLoopRuntimeBusinessInput,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan {
        let continuation_input =
            runtime_continuation_input_from_runtime_input(&continuation.next_runtime_input);
        self.plan_with_continuation_input(
            continuation,
            business_input,
            receipts,
            continuation_input,
        )
    }

    pub fn plan_from_request_plan(
        &self,
        request_plan: AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan {
        let receipt_count = receipts.len();
        let side_effect_dispatch_allowed_rate = request_plan.side_effect_dispatch_allowed_rate;
        let memory_note_allowed_rate = request_plan.memory_note_allowed_rate;
        let request_telemetry =
            service_loop_run_daemon_input_plan_from_request_telemetry(&request_plan, receipt_count);
        let service_run_input = AgentClosedLoopRuntimeServiceRunInput::new(
            request_plan.request_input,
            receipts,
            request_plan.continuation_input,
        );
        let loop_run_input = AgentClosedLoopRuntimeServiceLoopRunInput::new(
            service_run_input,
            request_plan.service_run_history,
            request_plan.service_run_policy,
        );
        let input = AgentClosedLoopRuntimeServiceLoopRunDaemonInput::new(
            loop_run_input,
            request_plan.loop_run_history,
            request_plan.loop_run_health_policy,
            request_plan.control_summary_history,
            request_plan.control_health_policy,
        );
        let mut telemetry = request_telemetry;
        telemetry.extend(service_loop_run_daemon_input_plan_state_telemetry(
            request_plan.mode,
            request_plan.can_schedule,
            request_plan.allows_adaptive_evolution,
            request_plan.requires_repair_first,
            receipt_count,
            &input,
        ));

        AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan {
            input,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            telemetry,
        }
    }

    pub fn plan_with_continuation_input(
        &self,
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
        business_input: AgentClosedLoopRuntimeBusinessInput,
        receipts: Vec<AgentServiceCommandReceipt>,
        continuation_input: AgentClosedLoopRuntimeContinuationInput,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan {
        let receipt_count = receipts.len();
        let request_input = AgentClosedLoopRuntimeServiceRequestInput::new(
            continuation.next_runtime_input.clone(),
            business_input,
        );
        let service_run_input =
            AgentClosedLoopRuntimeServiceRunInput::new(request_input, receipts, continuation_input);
        let loop_run_input = AgentClosedLoopRuntimeServiceLoopRunInput::new(
            service_run_input,
            continuation.service_run_history.clone(),
            continuation.service_run_policy,
        );
        let input = AgentClosedLoopRuntimeServiceLoopRunDaemonInput::new(
            loop_run_input,
            continuation.loop_run_history.clone(),
            continuation.loop_run_health_policy,
            continuation.control_summary_history.clone(),
            continuation.control_health_policy,
        );
        let telemetry =
            service_loop_run_daemon_input_plan_telemetry(continuation, receipt_count, &input);

        AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan {
            input,
            side_effect_dispatch_allowed_rate: continuation.side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate: continuation.memory_note_allowed_rate,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceIntakeRepairPlan {
    pub tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceIntakeRepairPlan {
    pub fn from_dispatch_outcome_parts(
        dispatch: &AgentClosedLoopRuntimeServiceDispatch,
        intake: &AgentClosedLoopRuntimeServiceReceiptIntake,
        has_outcome: bool,
    ) -> Self {
        if has_outcome || intake.blocked_reasons.is_empty() {
            return Self {
                tasks: Vec::new(),
                next_queue: AgentTaskQueue::new(),
                telemetry: vec!["service_intake_repair_tasks=0".to_owned()],
            };
        }

        let run_id = dispatch_run_id(dispatch);
        let tasks = intake
            .blocked_reasons
            .iter()
            .enumerate()
            .map(|(index, reason)| service_intake_repair_task(&run_id, index, reason))
            .collect::<Vec<_>>();
        let next_queue = AgentTaskQueue::from_tasks(tasks.clone());
        let telemetry = vec![format!("service_intake_repair_tasks={}", tasks.len())];

        Self {
            tasks,
            next_queue,
            telemetry,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentClosedLoopRuntimeServiceRequestRunner {
    turn_runner: AgentClosedLoopRuntimeTurnRunner,
    business_turn_closer: AgentClosedLoopRuntimeBusinessTurnCloser,
    command_planner: AgentClosedLoopRuntimeServiceCommandPlanner,
}

impl Default for AgentClosedLoopRuntimeServiceRequestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentClosedLoopRuntimeServiceRequestRunner {
    pub fn new() -> Self {
        Self {
            turn_runner: AgentClosedLoopRuntimeTurnRunner::new(),
            business_turn_closer: AgentClosedLoopRuntimeBusinessTurnCloser::new(),
            command_planner: AgentClosedLoopRuntimeServiceCommandPlanner::new(),
        }
    }

    pub fn run<E, P>(
        &self,
        input: AgentClosedLoopRuntimeServiceRequestInput,
        engine: &mut E,
        memory: &mut P,
    ) -> AgentClosedLoopRuntimeServiceRequest
    where
        E: EnginePort,
        E::Error: ToString,
        P: MemoryPort,
        P::Error: ToString,
    {
        let prior_history = input.runtime_input.history.clone();
        let runtime_turn = self.turn_runner.run(input.runtime_input, engine);
        let business_turn =
            self.business_turn_closer
                .close(runtime_turn, input.business_input, memory);
        let command_request = self.command_planner.plan(business_turn);
        let telemetry = service_request_telemetry(&command_request, &prior_history);

        AgentClosedLoopRuntimeServiceRequest {
            command_request,
            prior_history,
            telemetry,
        }
    }

    pub fn run_dispatch<E, P>(
        &self,
        input: AgentClosedLoopRuntimeServiceRequestInput,
        engine: &mut E,
        memory: &mut P,
    ) -> AgentClosedLoopRuntimeServiceDispatch
    where
        E: EnginePort,
        E::Error: ToString,
        P: MemoryPort,
        P::Error: ToString,
    {
        self.run(input, engine, memory).into_dispatch()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceTurn {
    pub business_turn: AgentClosedLoopRuntimeBusinessTurn,
    pub execution_report: Option<AgentClosedLoopExecutionReport>,
    pub summary: Option<AgentClosedLoopExecutionSummary>,
    pub updated_history: AgentClosedLoopExecutionHistory,
    pub skipped_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceTurn {
    pub fn has_execution_report(&self) -> bool {
        self.execution_report.is_some()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.execution_report
            .as_ref()
            .map(AgentClosedLoopExecutionReport::next_queue)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceTurnCloser;

impl AgentClosedLoopRuntimeServiceTurnCloser {
    pub fn new() -> Self {
        Self
    }

    pub fn close(
        &self,
        business_turn: AgentClosedLoopRuntimeBusinessTurn,
        history: &AgentClosedLoopExecutionHistory,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceTurn {
        let Some(step) = business_turn.step.clone() else {
            let skipped_reasons = if business_turn.skipped_reasons.is_empty() {
                vec!["business_turn_step_missing".to_owned()]
            } else {
                business_turn.skipped_reasons.clone()
            };
            let telemetry = service_turn_telemetry(&business_turn, None, &skipped_reasons);
            return AgentClosedLoopRuntimeServiceTurn {
                business_turn,
                execution_report: None,
                summary: None,
                updated_history: history.clone(),
                skipped_reasons,
                telemetry,
            };
        };

        let execution_report = step.close_service_execution(receipts);
        let summary = execution_report.summary();
        let mut updated_history = history.clone();
        updated_history.push(summary.clone());
        let telemetry = service_turn_telemetry(&business_turn, Some(&summary), &[]);

        AgentClosedLoopRuntimeServiceTurn {
            business_turn,
            execution_report: Some(execution_report),
            summary: Some(summary),
            updated_history,
            skipped_reasons: Vec::new(),
            telemetry,
        }
    }

    pub fn close_request(
        &self,
        request: AgentClosedLoopRuntimeServiceCommandRequest,
        history: &AgentClosedLoopExecutionHistory,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceTurn {
        let mut service_turn = self.close(request.business_turn, history, receipts);
        service_turn.telemetry.extend(request.telemetry);
        service_turn
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeContinuationInput {
    pub completed_task_ids: BTreeSet<String>,
    pub model_routes: Vec<AgentModelRouteRequest>,
    pub budget_ledger: BudgetLedger,
    pub budget_policy: BudgetPolicy,
    pub health_policy: AgentClosedLoopExecutionHealthPolicy,
    pub max_parallel_tasks: usize,
    pub evidence: AgentCycleEvidence,
}

impl AgentClosedLoopRuntimeContinuationInput {
    pub fn new(budget_ledger: BudgetLedger, evidence: AgentCycleEvidence) -> Self {
        Self {
            completed_task_ids: BTreeSet::new(),
            model_routes: Vec::new(),
            budget_ledger,
            budget_policy: BudgetPolicy::strict(),
            health_policy: AgentClosedLoopExecutionHealthPolicy::default(),
            max_parallel_tasks: 1,
            evidence,
        }
    }

    pub fn with_completed_task_ids(mut self, completed_task_ids: BTreeSet<String>) -> Self {
        self.completed_task_ids = completed_task_ids;
        self
    }

    pub fn with_model_routes(mut self, model_routes: Vec<AgentModelRouteRequest>) -> Self {
        self.model_routes = model_routes;
        self
    }

    pub fn with_budget_policy(mut self, budget_policy: BudgetPolicy) -> Self {
        self.budget_policy = budget_policy;
        self
    }

    pub fn with_health_policy(
        mut self,
        health_policy: AgentClosedLoopExecutionHealthPolicy,
    ) -> Self {
        self.health_policy = health_policy;
        self
    }

    pub fn with_max_parallel_tasks(mut self, max_parallel_tasks: usize) -> Self {
        self.max_parallel_tasks = max_parallel_tasks.max(1);
        self
    }
}

fn runtime_continuation_input_from_runtime_input(
    input: &AgentClosedLoopRuntimeTurnInput,
) -> AgentClosedLoopRuntimeContinuationInput {
    AgentClosedLoopRuntimeContinuationInput {
        completed_task_ids: input.completed_task_ids.clone(),
        model_routes: input.model_routes.clone(),
        budget_ledger: input.budget_ledger.clone(),
        budget_policy: input.budget_policy.clone(),
        health_policy: input.health_policy,
        max_parallel_tasks: input.max_parallel_tasks,
        evidence: input.evidence.clone(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeContinuation {
    pub service_turn: AgentClosedLoopRuntimeServiceTurn,
    pub dashboard: AgentClosedLoopExecutionDashboard,
    pub health: AgentClosedLoopExecutionHealth,
    pub next_runtime_input: AgentClosedLoopRuntimeTurnInput,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeContinuation {
    pub fn next_queue(&self) -> AgentTaskQueue {
        self.next_runtime_input.next_queue.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeContinuationPlanner;

impl AgentClosedLoopRuntimeContinuationPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        service_turn: AgentClosedLoopRuntimeServiceTurn,
        input: AgentClosedLoopRuntimeContinuationInput,
    ) -> AgentClosedLoopRuntimeContinuation {
        let next_queue = service_turn.next_queue();
        let dashboard = service_turn.updated_history.dashboard();
        let health = service_turn.updated_history.health(input.health_policy);
        let next_runtime_input = AgentClosedLoopRuntimeTurnInput {
            history: service_turn.updated_history.clone(),
            next_queue,
            completed_task_ids: input.completed_task_ids,
            model_routes: input.model_routes,
            budget_ledger: input.budget_ledger,
            budget_policy: input.budget_policy,
            health_policy: input.health_policy,
            max_parallel_tasks: input.max_parallel_tasks,
            evidence: input.evidence,
        };
        let telemetry = continuation_telemetry(&service_turn, &dashboard, &health);

        AgentClosedLoopRuntimeContinuation {
            service_turn,
            dashboard,
            health,
            next_runtime_input,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceOutcome {
    pub command_request: AgentClosedLoopRuntimeServiceCommandRequest,
    pub service_turn: AgentClosedLoopRuntimeServiceTurn,
    pub continuation: AgentClosedLoopRuntimeContinuation,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceOutcome {
    pub fn next_runtime_input(&self) -> &AgentClosedLoopRuntimeTurnInput {
        &self.continuation.next_runtime_input
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.continuation.next_queue()
    }

    pub fn summary(&self) -> AgentClosedLoopRuntimeServiceOutcomeSummary {
        AgentClosedLoopRuntimeServiceOutcomeSummary::from_outcome(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentClosedLoopRuntimeServiceOutcomeSummary {
    pub runtime_mode: AgentClosedLoopNextTurnMode,
    pub command_count: usize,
    pub command_gate_allowed: bool,
    pub side_effect_gate_count: usize,
    pub blocked_side_effect_gate_count: usize,
    pub service_executed: bool,
    pub service_clean: bool,
    pub health_status: AgentClosedLoopExecutionHealthStatus,
    pub next_queue_tasks: usize,
    pub command_gate_blocked_reasons: Vec<String>,
    pub skipped_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentClosedLoopRuntimeServiceOutcomeSummary {
    pub fn from_outcome(outcome: &AgentClosedLoopRuntimeServiceOutcome) -> Self {
        let mut skipped_reasons = outcome.command_request.skipped_reasons.clone();
        extend_unique(&mut skipped_reasons, &outcome.service_turn.skipped_reasons);
        let command_count = outcome
            .command_request
            .command_plan
            .as_ref()
            .map(|plan| plan.commands.len())
            .unwrap_or_default();
        let command_gate = outcome.command_request.gate();
        let command_gate_allowed = command_gate.is_allowed();
        let side_effect_gate_count = command_gate.entries.len();
        let blocked_side_effect_gate_count = command_gate
            .entries
            .iter()
            .filter(|entry| !entry.allowed)
            .count();
        let command_gate_blocked_reasons = command_gate.blocked_reasons.clone();
        let service_executed = outcome.service_turn.has_execution_report();
        let service_clean = outcome
            .service_turn
            .summary
            .as_ref()
            .is_some_and(|summary| summary.service_clean);
        let health_status = outcome.continuation.health.status;
        let next_queue_tasks = outcome.next_queue().len();
        let runtime_mode = outcome.command_request.business_turn.runtime_turn.mode();
        let telemetry = service_outcome_summary_telemetry(
            runtime_mode,
            command_count,
            command_gate_allowed,
            side_effect_gate_count,
            blocked_side_effect_gate_count,
            service_executed,
            service_clean,
            health_status,
            next_queue_tasks,
            &command_gate_blocked_reasons,
            &skipped_reasons,
        );

        Self {
            runtime_mode,
            command_count,
            command_gate_allowed,
            side_effect_gate_count,
            blocked_side_effect_gate_count,
            service_executed,
            service_clean,
            health_status,
            next_queue_tasks,
            command_gate_blocked_reasons,
            skipped_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentClosedLoopRuntimeServiceOutcomePlanner {
    service_turn_closer: AgentClosedLoopRuntimeServiceTurnCloser,
    continuation_planner: AgentClosedLoopRuntimeContinuationPlanner,
}

impl AgentClosedLoopRuntimeServiceOutcomePlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn close(
        &self,
        command_request: AgentClosedLoopRuntimeServiceCommandRequest,
        history: &AgentClosedLoopExecutionHistory,
        receipts: Vec<AgentServiceCommandReceipt>,
        continuation_input: AgentClosedLoopRuntimeContinuationInput,
    ) -> AgentClosedLoopRuntimeServiceOutcome {
        let service_turn =
            self.service_turn_closer
                .close_request(command_request.clone(), history, receipts);
        let continuation = self
            .continuation_planner
            .plan(service_turn.clone(), continuation_input);
        let telemetry = service_outcome_telemetry(&command_request, &service_turn, &continuation);

        AgentClosedLoopRuntimeServiceOutcome {
            command_request,
            service_turn,
            continuation,
            telemetry,
        }
    }
}

fn collect_skipped_reasons(prepared_cycle: &AgentClosedLoopPreparedCycle) -> Vec<String> {
    let mut reasons = Vec::new();
    extend_unique(
        &mut reasons,
        &prepared_cycle
            .prepared_execution
            .prepared_dispatch
            .skipped_reasons,
    );
    extend_unique(
        &mut reasons,
        &prepared_cycle.prepared_execution.skipped_reasons,
    );
    extend_unique(&mut reasons, &prepared_cycle.skipped_reasons);
    reasons
}

fn service_outcome_telemetry(
    command_request: &AgentClosedLoopRuntimeServiceCommandRequest,
    service_turn: &AgentClosedLoopRuntimeServiceTurn,
    continuation: &AgentClosedLoopRuntimeContinuation,
) -> Vec<String> {
    let mut telemetry = command_request.telemetry.clone();
    telemetry.extend(service_turn.telemetry.clone());
    telemetry.extend(continuation.telemetry.clone());
    telemetry.push(format!(
        "service_outcome_commands={}",
        command_request
            .command_plan
            .as_ref()
            .map(|plan| plan.commands.len())
            .unwrap_or_default()
    ));
    telemetry.push(format!(
        "service_outcome_execution={}",
        service_turn.has_execution_report()
    ));
    telemetry.push(format!(
        "service_outcome_history_runs={}",
        continuation.dashboard.total_runs
    ));
    telemetry.push(format!(
        "service_outcome_health={}",
        continuation.health.status.as_str()
    ));
    telemetry
}

fn gate_service_command(
    command: &AgentServiceCommand,
    step: Option<&AgentClosedLoopStep>,
) -> AgentClosedLoopRuntimeServiceCommandGateEntry {
    match command {
        AgentServiceCommand::PromoteAdaptiveState(_) => {
            let Some(step) = step else {
                return AgentClosedLoopRuntimeServiceCommandGateEntry::block(
                    command,
                    SideEffectKind::AdaptiveStateWrite,
                    "closed_loop_step_missing",
                );
            };
            if step.business_plan.can_promote_adaptive_state() {
                AgentClosedLoopRuntimeServiceCommandGateEntry::allow(
                    command,
                    SideEffectKind::AdaptiveStateWrite,
                    "business_loop_promote_admitted",
                )
            } else {
                AgentClosedLoopRuntimeServiceCommandGateEntry::block(
                    command,
                    SideEffectKind::AdaptiveStateWrite,
                    "business_loop_promote_not_admitted",
                )
            }
        }
        AgentServiceCommand::HoldBusinessLoop { .. } => {
            AgentClosedLoopRuntimeServiceCommandGateEntry::allow(
                command,
                SideEffectKind::AdaptiveStateWrite,
                "business_loop_hold_command",
            )
        }
        AgentServiceCommand::OpenRepairMode { .. } => {
            AgentClosedLoopRuntimeServiceCommandGateEntry::allow(
                command,
                SideEffectKind::AdaptiveStateWrite,
                "business_loop_repair_command",
            )
        }
        AgentServiceCommand::RunRustValidation { commands, .. } => {
            if commands.is_empty() {
                AgentClosedLoopRuntimeServiceCommandGateEntry::block(
                    command,
                    SideEffectKind::ExternalCall,
                    "rust_validation_requires_non_empty_commands",
                )
            } else {
                AgentClosedLoopRuntimeServiceCommandGateEntry::allow(
                    command,
                    SideEffectKind::ExternalCall,
                    "rust_validation_commands_planned",
                )
            }
        }
        AgentServiceCommand::EnqueueTasks(queue) => {
            if queue.is_empty() {
                AgentClosedLoopRuntimeServiceCommandGateEntry::block(
                    command,
                    SideEffectKind::ExternalCall,
                    "enqueue_tasks_requires_non_empty_queue",
                )
            } else {
                AgentClosedLoopRuntimeServiceCommandGateEntry::allow(
                    command,
                    SideEffectKind::ExternalCall,
                    "business_loop_queue_available",
                )
            }
        }
        AgentServiceCommand::EmitTelemetry(line) => {
            if line.is_empty() {
                AgentClosedLoopRuntimeServiceCommandGateEntry::block(
                    command,
                    SideEffectKind::ExternalCall,
                    "telemetry_line_empty",
                )
            } else {
                AgentClosedLoopRuntimeServiceCommandGateEntry::allow(
                    command,
                    SideEffectKind::ExternalCall,
                    "telemetry_line_present",
                )
            }
        }
    }
}

fn service_command_gate_telemetry(
    command_count: usize,
    plan_missing: bool,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!("service_command_gate_commands={command_count}"),
        format!(
            "service_command_gate_allowed={}",
            blocked_reasons.is_empty()
        ),
    ];
    if plan_missing {
        telemetry.push("service_command_gate_plan_missing=true".to_owned());
    }
    telemetry.extend(
        blocked_reasons
            .iter()
            .map(|reason| format!("service_command_gate_blocked_reason={reason}")),
    );
    telemetry
}

fn service_dispatch_telemetry(
    request: &AgentClosedLoopRuntimeServiceRequest,
    command_gate: &AgentClosedLoopRuntimeServiceCommandGate,
) -> Vec<String> {
    let mut telemetry = request.telemetry.clone();
    telemetry.extend(command_gate.telemetry.clone());
    telemetry.push(format!(
        "service_dispatch_executable={}",
        request.has_commands() && command_gate.is_allowed()
    ));
    telemetry.push(format!(
        "service_dispatch_blockers={}",
        command_gate.blocked_reasons.len()
    ));
    telemetry
}

fn service_dispatch_summary_telemetry(
    executable: bool,
    command_count: usize,
    command_gate_allowed: bool,
    side_effect_gate_count: usize,
    blocked_side_effect_gate_count: usize,
    command_kinds: &[String],
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!("service_dispatch_summary_executable={executable}"),
        format!("service_dispatch_summary_commands={command_count}"),
        format!("service_dispatch_summary_command_gate_allowed={command_gate_allowed}"),
        format!("service_dispatch_summary_side_effect_gates={side_effect_gate_count}"),
        format!(
            "service_dispatch_summary_blocked_side_effect_gates={blocked_side_effect_gate_count}"
        ),
    ];
    telemetry.extend(
        command_kinds
            .iter()
            .map(|kind| format!("service_dispatch_summary_command={kind}")),
    );
    telemetry.extend(
        blocked_reasons
            .iter()
            .map(|reason| format!("service_dispatch_summary_blocked={reason}")),
    );
    telemetry
}

fn service_dispatch_dashboard_telemetry(
    total_dispatches: usize,
    executable_dispatches: usize,
    blocked_dispatches: usize,
    command_gate_allowed_dispatches: usize,
    command_count: usize,
    side_effect_gate_count: usize,
    blocked_side_effect_gate_count: usize,
    blocked_reason_count: usize,
    executable_rate: f32,
    blocked_side_effect_gate_rate: f32,
) -> Vec<String> {
    vec![
        "service_dispatch_dashboard=true".to_owned(),
        format!("service_dispatch_dashboard_dispatches={total_dispatches}"),
        format!("service_dispatch_dashboard_executable={executable_dispatches}"),
        format!("service_dispatch_dashboard_blocked={blocked_dispatches}"),
        format!(
            "service_dispatch_dashboard_command_gate_allowed={command_gate_allowed_dispatches}"
        ),
        format!("service_dispatch_dashboard_commands={command_count}"),
        format!("service_dispatch_dashboard_side_effect_gates={side_effect_gate_count}"),
        format!(
            "service_dispatch_dashboard_blocked_side_effect_gates={blocked_side_effect_gate_count}"
        ),
        format!("service_dispatch_dashboard_blocked_reasons={blocked_reason_count}"),
        format!("service_dispatch_dashboard_executable_rate={executable_rate:.3}"),
        format!(
            "service_dispatch_dashboard_blocked_side_effect_gate_rate={blocked_side_effect_gate_rate:.3}"
        ),
    ]
}

fn service_dispatch_history_record_telemetry(
    dashboard: &AgentClosedLoopRuntimeServiceDispatchDashboard,
    health: &AgentClosedLoopRuntimeServiceDispatchHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "service_dispatch_history_record=true".to_owned(),
        format!(
            "service_dispatch_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "service_dispatch_history_record_dispatches={}",
            dashboard.total_dispatches
        ),
        format!(
            "service_dispatch_history_record_executable_rate={:.3}",
            dashboard.executable_rate
        ),
        format!(
            "service_dispatch_history_record_blocked_dispatches={}",
            dashboard.blocked_dispatches
        ),
        format!(
            "service_dispatch_history_record_blocked_side_effect_gates={}",
            dashboard.blocked_side_effect_gate_count
        ),
        format!(
            "service_dispatch_history_record_blocked_reasons={}",
            dashboard.blocked_reason_count
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("service_dispatch_history_record_reason={reason}")),
    );
    telemetry
}

fn service_receipt_intake_telemetry(
    executable: bool,
    expected_count: usize,
    accepted_count: usize,
    rejected_count: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!("service_receipt_intake_executable={executable}"),
        format!("service_receipt_intake_expected={expected_count}"),
        format!("service_receipt_intake_accepted={accepted_count}"),
        format!("service_receipt_intake_rejected={rejected_count}"),
        format!(
            "service_receipt_intake_clean={}",
            executable && rejected_count == 0 && blocked_reasons.is_empty()
        ),
    ];
    telemetry.extend(
        blocked_reasons
            .iter()
            .map(|reason| format!("service_receipt_intake_blocked={reason}")),
    );
    telemetry
}

fn service_receipt_intake_summary_telemetry(
    executable: bool,
    expected_receipts: usize,
    accepted_receipts: usize,
    rejected_receipts: usize,
    clean: bool,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "service_receipt_intake_summary=true".to_owned(),
        format!("service_receipt_intake_summary_executable={executable}"),
        format!("service_receipt_intake_summary_expected={expected_receipts}"),
        format!("service_receipt_intake_summary_accepted={accepted_receipts}"),
        format!("service_receipt_intake_summary_rejected={rejected_receipts}"),
        format!("service_receipt_intake_summary_clean={clean}"),
        format!("service_receipt_intake_summary_blocked_reasons={blocked_reasons}"),
    ]
}

fn service_receipt_intake_dashboard_telemetry(
    total_intakes: usize,
    clean_intakes: usize,
    dirty_intakes: usize,
    executable_intakes: usize,
    non_executable_intakes: usize,
    expected_receipt_count: usize,
    accepted_receipt_count: usize,
    rejected_receipt_count: usize,
    blocked_reason_count: usize,
    clean_rate: f32,
    rejection_rate: f32,
) -> Vec<String> {
    vec![
        "service_receipt_intake_dashboard=true".to_owned(),
        format!("service_receipt_intake_dashboard_intakes={total_intakes}"),
        format!("service_receipt_intake_dashboard_clean={clean_intakes}"),
        format!("service_receipt_intake_dashboard_dirty={dirty_intakes}"),
        format!("service_receipt_intake_dashboard_executable={executable_intakes}"),
        format!("service_receipt_intake_dashboard_non_executable={non_executable_intakes}"),
        format!("service_receipt_intake_dashboard_expected={expected_receipt_count}"),
        format!("service_receipt_intake_dashboard_accepted={accepted_receipt_count}"),
        format!("service_receipt_intake_dashboard_rejected={rejected_receipt_count}"),
        format!("service_receipt_intake_dashboard_blocked_reasons={blocked_reason_count}"),
        format!("service_receipt_intake_dashboard_clean_rate={clean_rate:.3}"),
        format!("service_receipt_intake_dashboard_rejection_rate={rejection_rate:.3}"),
    ]
}

fn service_receipt_intake_history_record_telemetry(
    dashboard: &AgentClosedLoopRuntimeServiceReceiptIntakeDashboard,
    health: &AgentClosedLoopRuntimeServiceReceiptIntakeHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "service_receipt_intake_history_record=true".to_owned(),
        format!(
            "service_receipt_intake_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "service_receipt_intake_history_record_intakes={}",
            dashboard.total_intakes
        ),
        format!(
            "service_receipt_intake_history_record_clean_rate={:.3}",
            dashboard.clean_rate
        ),
        format!(
            "service_receipt_intake_history_record_non_executable={}",
            dashboard.non_executable_intakes
        ),
        format!(
            "service_receipt_intake_history_record_rejected={}",
            dashboard.rejected_receipt_count
        ),
        format!(
            "service_receipt_intake_history_record_blocked_reasons={}",
            dashboard.blocked_reason_count
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("service_receipt_intake_history_record_reason={reason}")),
    );
    telemetry
}

fn service_dispatch_outcome_telemetry(
    intake: &AgentClosedLoopRuntimeServiceReceiptIntake,
    outcome: Option<&AgentClosedLoopRuntimeServiceOutcome>,
    repair_plan: &AgentClosedLoopRuntimeServiceIntakeRepairPlan,
) -> Vec<String> {
    let mut telemetry = intake.telemetry.clone();
    telemetry.push(format!("service_dispatch_outcome={}", outcome.is_some()));
    if let Some(outcome) = outcome {
        telemetry.push(format!(
            "service_dispatch_outcome_health={}",
            outcome.continuation.health.status.as_str()
        ));
        telemetry.push(format!(
            "service_dispatch_outcome_next_queue_tasks={}",
            outcome.next_queue().len()
        ));
    }
    telemetry.extend(repair_plan.telemetry.clone());
    telemetry
}

fn service_dispatch_continuation_telemetry(
    dispatch_outcome: &AgentClosedLoopRuntimeServiceDispatchOutcome,
    dashboard: &AgentClosedLoopExecutionDashboard,
    health: &AgentClosedLoopExecutionHealth,
    next_queue_tasks: usize,
) -> Vec<String> {
    let mut telemetry = dispatch_outcome.telemetry.clone();
    telemetry.push(format!(
        "service_dispatch_continuation_outcome={}",
        dispatch_outcome.has_outcome()
    ));
    telemetry.push(format!(
        "service_dispatch_continuation_runs={}",
        dashboard.total_runs
    ));
    telemetry.push(format!(
        "service_dispatch_continuation_health={}",
        health.status.as_str()
    ));
    telemetry.push(format!(
        "service_dispatch_continuation_next_queue_tasks={next_queue_tasks}"
    ));
    telemetry
}

fn service_dispatch_continuation_summary_telemetry(
    outcome_closed: bool,
    intake_clean: bool,
    repair_task_count: usize,
    health_status: AgentClosedLoopExecutionHealthStatus,
    next_queue_tasks: usize,
    immediate_ready_tasks: usize,
    history_runs: usize,
) -> Vec<String> {
    vec![
        format!("service_dispatch_continuation_summary_outcome_closed={outcome_closed}"),
        format!("service_dispatch_continuation_summary_intake_clean={intake_clean}"),
        format!("service_dispatch_continuation_summary_repair_tasks={repair_task_count}"),
        format!(
            "service_dispatch_continuation_summary_health={}",
            health_status.as_str()
        ),
        format!("service_dispatch_continuation_summary_next_queue_tasks={next_queue_tasks}"),
        format!(
            "service_dispatch_continuation_summary_immediate_ready_tasks={immediate_ready_tasks}"
        ),
        format!("service_dispatch_continuation_summary_history_runs={history_runs}"),
    ]
}

fn service_dispatch_continuation_dashboard_telemetry(
    total_continuations: usize,
    closed_continuations: usize,
    blocked_continuations: usize,
    intake_clean_continuations: usize,
    intake_dirty_continuations: usize,
    repair_health_continuations: usize,
    repair_task_count: usize,
    total_next_queue_tasks: usize,
    total_immediate_ready_tasks: usize,
    latest_history_runs: usize,
    closed_rate: f32,
    intake_clean_rate: f32,
) -> Vec<String> {
    vec![
        "service_dispatch_continuation_dashboard=true".to_owned(),
        format!("service_dispatch_continuation_dashboard_continuations={total_continuations}"),
        format!("service_dispatch_continuation_dashboard_closed={closed_continuations}"),
        format!("service_dispatch_continuation_dashboard_blocked={blocked_continuations}"),
        format!(
            "service_dispatch_continuation_dashboard_intake_clean={intake_clean_continuations}"
        ),
        format!(
            "service_dispatch_continuation_dashboard_intake_dirty={intake_dirty_continuations}"
        ),
        format!(
            "service_dispatch_continuation_dashboard_repair_health={repair_health_continuations}"
        ),
        format!("service_dispatch_continuation_dashboard_repair_tasks={repair_task_count}"),
        format!(
            "service_dispatch_continuation_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "service_dispatch_continuation_dashboard_immediate_ready_tasks={total_immediate_ready_tasks}"
        ),
        format!(
            "service_dispatch_continuation_dashboard_latest_history_runs={latest_history_runs}"
        ),
        format!("service_dispatch_continuation_dashboard_closed_rate={closed_rate:.3}"),
        format!("service_dispatch_continuation_dashboard_intake_clean_rate={intake_clean_rate:.3}"),
    ]
}

fn service_dispatch_continuation_history_record_telemetry(
    dashboard: &AgentClosedLoopRuntimeServiceDispatchContinuationDashboard,
    health: &AgentClosedLoopRuntimeServiceDispatchContinuationHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "service_dispatch_continuation_history_record=true".to_owned(),
        format!(
            "service_dispatch_continuation_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "service_dispatch_continuation_history_record_continuations={}",
            dashboard.total_continuations
        ),
        format!(
            "service_dispatch_continuation_history_record_closed_rate={:.3}",
            dashboard.closed_rate
        ),
        format!(
            "service_dispatch_continuation_history_record_intake_clean_rate={:.3}",
            dashboard.intake_clean_rate
        ),
        format!(
            "service_dispatch_continuation_history_record_blocked={}",
            dashboard.blocked_continuations
        ),
        format!(
            "service_dispatch_continuation_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("service_dispatch_continuation_history_record_reason={reason}")),
    );
    telemetry
}

fn service_run_telemetry(
    dispatch_continuation: &AgentClosedLoopRuntimeServiceDispatchContinuation,
    summary: &AgentClosedLoopRuntimeServiceRunSummary,
) -> Vec<String> {
    let mut telemetry = dispatch_continuation.telemetry.clone();
    telemetry.extend(summary.telemetry.clone());
    telemetry.push(format!(
        "service_run_outcome_closed={}",
        summary.outcome_closed
    ));
    telemetry.push(format!("service_run_intake_clean={}", summary.intake_clean));
    telemetry.push(format!(
        "service_run_repair_tasks={}",
        summary.repair_task_count
    ));
    telemetry.push(format!(
        "service_run_health={}",
        summary.health_status.as_str()
    ));
    telemetry.push(format!(
        "service_run_next_queue_tasks={}",
        summary.next_queue_tasks
    ));
    telemetry
}

fn service_run_summary_telemetry(
    status: &AgentClosedLoopRuntimeServiceRunStatus,
    dispatch_summary: &AgentClosedLoopRuntimeServiceDispatchSummary,
    continuation_summary: &AgentClosedLoopRuntimeServiceDispatchContinuationSummary,
    intake_blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!("service_run_summary_status={}", status.as_str()),
        format!(
            "service_run_summary_dispatch_executable={}",
            dispatch_summary.executable
        ),
        format!(
            "service_run_summary_commands={}",
            dispatch_summary.command_count
        ),
        format!(
            "service_run_summary_command_gate_allowed={}",
            dispatch_summary.command_gate_allowed
        ),
        format!(
            "service_run_summary_side_effect_gates={}",
            dispatch_summary.side_effect_gate_count
        ),
        format!(
            "service_run_summary_blocked_side_effect_gates={}",
            dispatch_summary.blocked_side_effect_gate_count
        ),
        format!(
            "service_run_summary_outcome_closed={}",
            continuation_summary.outcome_closed
        ),
        format!(
            "service_run_summary_intake_clean={}",
            continuation_summary.intake_clean
        ),
        format!(
            "service_run_summary_repair_tasks={}",
            continuation_summary.repair_task_count
        ),
        format!(
            "service_run_summary_health={}",
            continuation_summary.health_status.as_str()
        ),
        format!(
            "service_run_summary_next_queue_tasks={}",
            continuation_summary.next_queue_tasks
        ),
        format!(
            "service_run_summary_immediate_ready_tasks={}",
            continuation_summary.immediate_ready_tasks
        ),
        format!(
            "service_run_summary_history_runs={}",
            continuation_summary.history_runs
        ),
    ];
    telemetry.extend(
        dispatch_summary
            .command_kinds
            .iter()
            .map(|kind| format!("service_run_summary_command={kind}")),
    );
    telemetry.extend(
        dispatch_summary
            .blocked_reasons
            .iter()
            .map(|reason| format!("service_run_summary_gate_blocked={reason}")),
    );
    telemetry.extend(
        intake_blocked_reasons
            .iter()
            .map(|reason| format!("service_run_summary_intake_blocked={reason}")),
    );
    telemetry
}

fn classify_service_run(
    dispatch_summary: &AgentClosedLoopRuntimeServiceDispatchSummary,
    continuation_summary: &AgentClosedLoopRuntimeServiceDispatchContinuationSummary,
) -> AgentClosedLoopRuntimeServiceRunStatus {
    if continuation_summary.outcome_closed && continuation_summary.intake_clean {
        AgentClosedLoopRuntimeServiceRunStatus::Closed
    } else if !dispatch_summary.executable {
        AgentClosedLoopRuntimeServiceRunStatus::DispatchBlocked
    } else {
        AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
    }
}

fn service_run_history_record_telemetry(
    summary: &AgentClosedLoopRuntimeServiceRunSummary,
    dashboard: &AgentClosedLoopRuntimeServiceRunDashboard,
    health: &AgentClosedLoopRuntimeServiceRunHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        format!(
            "service_run_history_record_status={}",
            summary.status.as_str()
        ),
        format!(
            "service_run_history_record_attempts={}",
            dashboard.total_runs
        ),
        format!(
            "service_run_history_record_closed_runs={}",
            dashboard.closed_runs
        ),
        format!(
            "service_run_history_record_blocked_runs={}",
            dashboard.blocked_runs
        ),
        format!(
            "service_run_history_record_health={}",
            health.status.as_str()
        ),
        format!(
            "service_run_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "service_run_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("service_run_history_record_reason={reason}")),
    );
    telemetry
}

fn service_run_rate(count: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        count as f32 / total as f32
    }
}

fn average_service_run_rate<T>(items: &[T], rate: impl Fn(&T) -> f32) -> f32 {
    if items.is_empty() {
        0.0
    } else {
        items.iter().map(rate).sum::<f32>() / items.len() as f32
    }
}

fn service_preflight_mode(
    turn_plan: &AgentClosedLoopNextTurnPlan,
    service_run_health: &AgentClosedLoopRuntimeServiceRunHealth,
) -> AgentClosedLoopNextTurnMode {
    if turn_plan.mode == AgentClosedLoopNextTurnMode::Idle {
        AgentClosedLoopNextTurnMode::Idle
    } else if turn_plan.mode == AgentClosedLoopNextTurnMode::Repair
        || service_run_health.status == AgentClosedLoopExecutionHealthStatus::Repair
    {
        AgentClosedLoopNextTurnMode::Repair
    } else if turn_plan.mode == AgentClosedLoopNextTurnMode::Observe
        || service_run_health.status == AgentClosedLoopExecutionHealthStatus::Watch
    {
        AgentClosedLoopNextTurnMode::Observe
    } else {
        AgentClosedLoopNextTurnMode::Continue
    }
}

fn service_preflight_telemetry(
    turn_plan: &AgentClosedLoopNextTurnPlan,
    service_run_health: &AgentClosedLoopRuntimeServiceRunHealth,
    mode: AgentClosedLoopNextTurnMode,
    reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!("service_preflight_mode={}", mode.as_str()),
        format!("service_preflight_turn_mode={}", turn_plan.mode.as_str()),
        format!(
            "service_preflight_execution_health={}",
            turn_plan.health.status.as_str()
        ),
        format!(
            "service_preflight_service_run_health={}",
            service_run_health.status.as_str()
        ),
        format!(
            "service_preflight_next_queue_tasks={}",
            turn_plan.next_queue.len()
        ),
        format!(
            "service_preflight_service_run_attempts={}",
            service_run_health.dashboard.total_runs
        ),
    ];
    telemetry.extend(
        reasons
            .iter()
            .map(|reason| format!("service_preflight_reason={reason}")),
    );
    telemetry
}

fn service_preflight_admission_health_status(
    preflight: &AgentClosedLoopRuntimeServicePreflight,
) -> AgentClosedLoopExecutionHealthStatus {
    match preflight.mode {
        AgentClosedLoopNextTurnMode::Continue => AgentClosedLoopExecutionHealthStatus::Stable,
        AgentClosedLoopNextTurnMode::Observe | AgentClosedLoopNextTurnMode::Idle => {
            AgentClosedLoopExecutionHealthStatus::Watch
        }
        AgentClosedLoopNextTurnMode::Repair => AgentClosedLoopExecutionHealthStatus::Repair,
    }
}

fn service_preflight_admission_gates(
    can_dispatch_service_commands: bool,
    can_promote_memory_note: bool,
    can_admit_adaptive_evolution: bool,
    requires_repair_first: bool,
) -> Vec<SideEffectGate> {
    let dispatch_reason = if can_dispatch_service_commands {
        "service_preflight_allows_dispatch"
    } else if requires_repair_first {
        "service_preflight_requires_repair_first"
    } else {
        "service_preflight_not_schedulable"
    };
    let memory_reason = if can_promote_memory_note {
        "service_preflight_allows_memory_note"
    } else if requires_repair_first {
        "service_preflight_requires_repair_first"
    } else {
        "service_preflight_memory_note_not_stable"
    };
    let adaptive_reason = if can_admit_adaptive_evolution {
        "service_preflight_allows_adaptive_evolution"
    } else if requires_repair_first {
        "service_preflight_requires_repair_first"
    } else {
        "service_preflight_adaptive_evolution_not_stable"
    };

    vec![
        service_preflight_admission_gate(
            SideEffectKind::ExternalCall,
            can_dispatch_service_commands,
            dispatch_reason,
        ),
        service_preflight_admission_gate(
            SideEffectKind::MemoryNote,
            can_promote_memory_note,
            memory_reason,
        ),
        service_preflight_admission_gate(
            SideEffectKind::AdaptiveStateWrite,
            can_admit_adaptive_evolution,
            adaptive_reason,
        ),
    ]
}

fn service_preflight_admission_gate(
    kind: SideEffectKind,
    allowed: bool,
    reason: &str,
) -> SideEffectGate {
    if allowed {
        SideEffectGate::allow(kind, reason)
    } else {
        SideEffectGate::block(kind, reason)
    }
}

fn service_preflight_admission_reasons(
    preflight: &AgentClosedLoopRuntimeServicePreflight,
) -> Vec<String> {
    let mut reasons = preflight.reasons.clone();
    match preflight.mode {
        AgentClosedLoopNextTurnMode::Continue => {}
        AgentClosedLoopNextTurnMode::Observe => {
            extend_unique(
                &mut reasons,
                &["service_preflight_admission_observe".to_owned()],
            );
        }
        AgentClosedLoopNextTurnMode::Repair => {
            extend_unique(
                &mut reasons,
                &["service_preflight_admission_repair_first".to_owned()],
            );
        }
        AgentClosedLoopNextTurnMode::Idle => {
            extend_unique(
                &mut reasons,
                &["service_preflight_admission_idle".to_owned()],
            );
        }
    }
    reasons
}

#[allow(clippy::too_many_arguments)]
fn service_preflight_admission_telemetry(
    mode: AgentClosedLoopNextTurnMode,
    health_status: AgentClosedLoopExecutionHealthStatus,
    can_dispatch_service_commands: bool,
    can_promote_memory_note: bool,
    can_admit_adaptive_evolution: bool,
    requires_repair_first: bool,
    reasons: usize,
) -> Vec<String> {
    vec![
        "service_preflight_side_effect_admission=true".to_owned(),
        format!(
            "service_preflight_side_effect_admission_mode={}",
            mode.as_str()
        ),
        format!(
            "service_preflight_side_effect_admission_health={}",
            health_status.as_str()
        ),
        format!(
            "service_preflight_side_effect_admission_can_dispatch={can_dispatch_service_commands}"
        ),
        format!(
            "service_preflight_side_effect_admission_can_promote_memory_note={can_promote_memory_note}"
        ),
        format!(
            "service_preflight_side_effect_admission_can_admit_adaptive_evolution={can_admit_adaptive_evolution}"
        ),
        format!(
            "service_preflight_side_effect_admission_requires_repair_first={requires_repair_first}"
        ),
        format!("service_preflight_side_effect_admission_reasons={reasons}"),
    ]
}

fn service_preflight_follow_up_telemetry(
    mode: AgentClosedLoopNextTurnMode,
    tasks: &[AgentTask],
    next_queue: &AgentTaskQueue,
) -> Vec<String> {
    let immediate_ready_tasks = next_queue.immediate_ready_tasks().len();
    vec![
        format!("service_preflight_follow_up_mode={}", mode.as_str()),
        format!("service_preflight_follow_up_tasks={}", tasks.len()),
        format!("service_preflight_follow_up_immediate_ready_tasks={immediate_ready_tasks}"),
        format!(
            "service_preflight_follow_up_next_queue_tasks={}",
            next_queue.next_queue_tasks().len()
        ),
    ]
}

fn service_preflight_continuation_telemetry(
    preflight: &AgentClosedLoopRuntimeServicePreflight,
    follow_up_plan: &AgentClosedLoopRuntimeServicePreflightFollowUpPlan,
    next_runtime_input: &AgentClosedLoopRuntimeTurnInput,
) -> Vec<String> {
    let immediate_ready_tasks = next_runtime_input.next_queue.immediate_ready_tasks().len();
    let mut telemetry = preflight.telemetry.clone();
    telemetry.extend(follow_up_plan.telemetry.clone());
    telemetry.push(format!(
        "service_preflight_continuation_mode={}",
        preflight.mode.as_str()
    ));
    telemetry.push(format!(
        "service_preflight_continuation_follow_up_tasks={}",
        follow_up_plan.tasks.len()
    ));
    telemetry.push(format!(
        "service_preflight_continuation_next_queue_tasks={}",
        next_runtime_input.next_queue.next_queue_tasks().len()
    ));
    telemetry.push(format!(
        "service_preflight_continuation_immediate_ready_tasks={immediate_ready_tasks}"
    ));
    telemetry.push(format!(
        "service_preflight_continuation_history_runs={}",
        next_runtime_input.history.len()
    ));
    telemetry
}

fn service_loop_state_telemetry(
    execution_history: &AgentClosedLoopExecutionHistory,
    service_run_history: &AgentClosedLoopRuntimeServiceRunHistory,
    preflight_continuation: &AgentClosedLoopRuntimeServicePreflightContinuation,
) -> Vec<String> {
    let mut telemetry = preflight_continuation.telemetry.clone();
    telemetry.push(format!(
        "service_loop_state_mode={}",
        preflight_continuation.preflight.mode.as_str()
    ));
    telemetry.push(format!(
        "service_loop_state_execution_history_runs={}",
        execution_history.len()
    ));
    telemetry.push(format!(
        "service_loop_state_service_run_attempts={}",
        service_run_history.len()
    ));
    telemetry.push(format!(
        "service_loop_state_next_queue_tasks={}",
        preflight_continuation.next_runtime_input.next_queue.len()
    ));
    telemetry.push(format!(
        "service_loop_state_follow_up_tasks={}",
        preflight_continuation.follow_up_plan.tasks.len()
    ));
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn service_loop_state_summary_telemetry(
    mode: AgentClosedLoopNextTurnMode,
    execution_health_status: AgentClosedLoopExecutionHealthStatus,
    service_run_health_status: AgentClosedLoopExecutionHealthStatus,
    side_effect_admission_health_status: AgentClosedLoopExecutionHealthStatus,
    can_schedule: bool,
    side_effect_dispatch_allowed: bool,
    memory_note_allowed: bool,
    allows_adaptive_evolution: bool,
    requires_repair_first: bool,
    execution_history_runs: usize,
    service_run_attempts: usize,
    preflight_follow_up_tasks: usize,
    next_queue_tasks: usize,
    immediate_ready_tasks: usize,
    side_effect_admission_reasons: usize,
    reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!("service_loop_state_summary_mode={}", mode.as_str()),
        format!(
            "service_loop_state_summary_execution_health={}",
            execution_health_status.as_str()
        ),
        format!(
            "service_loop_state_summary_service_run_health={}",
            service_run_health_status.as_str()
        ),
        format!(
            "service_loop_state_summary_side_effect_admission_health={}",
            side_effect_admission_health_status.as_str()
        ),
        format!("service_loop_state_summary_can_schedule={can_schedule}"),
        format!(
            "service_loop_state_summary_side_effect_dispatch_allowed={side_effect_dispatch_allowed}"
        ),
        format!("service_loop_state_summary_memory_note_allowed={memory_note_allowed}"),
        format!("service_loop_state_summary_allows_adaptive_evolution={allows_adaptive_evolution}"),
        format!("service_loop_state_summary_requires_repair_first={requires_repair_first}"),
        format!("service_loop_state_summary_execution_history_runs={execution_history_runs}"),
        format!("service_loop_state_summary_service_run_attempts={service_run_attempts}"),
        format!("service_loop_state_summary_follow_up_tasks={preflight_follow_up_tasks}"),
        format!("service_loop_state_summary_next_queue_tasks={next_queue_tasks}"),
        format!("service_loop_state_summary_immediate_ready_tasks={immediate_ready_tasks}"),
        format!(
            "service_loop_state_summary_side_effect_admission_reasons={side_effect_admission_reasons}"
        ),
    ];
    telemetry.extend(
        reasons
            .iter()
            .map(|reason| format!("service_loop_state_summary_reason={reason}")),
    );
    telemetry
}

fn service_loop_advance_telemetry(
    run_record: &AgentClosedLoopRuntimeServiceRunHistoryRecord,
    summary: &AgentClosedLoopRuntimeServiceLoopStateSummary,
) -> Vec<String> {
    let mut telemetry = run_record.telemetry.clone();
    telemetry.extend(summary.telemetry.clone());
    telemetry.push(format!(
        "service_loop_advance_run_status={}",
        run_record.appended_summary.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_advance_attempts={}",
        run_record.attempts()
    ));
    telemetry.push(format!(
        "service_loop_advance_mode={}",
        summary.mode.as_str()
    ));
    telemetry.push(format!(
        "service_loop_advance_requires_repair_first={}",
        summary.requires_repair_first
    ));
    telemetry
}

fn service_loop_run_telemetry(
    run: &AgentClosedLoopRuntimeServiceRun,
    advance: &AgentClosedLoopRuntimeServiceLoopAdvance,
) -> Vec<String> {
    let mut telemetry = run.telemetry.clone();
    telemetry.extend(advance.telemetry.clone());
    telemetry.push(format!(
        "service_loop_run_status={}",
        run.run_summary.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_mode={}",
        advance.summary.mode.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_attempts={}",
        advance.run_record.attempts()
    ));
    telemetry.push(format!(
        "service_loop_run_next_queue_tasks={}",
        advance.next_runtime_input().next_queue.len()
    ));
    telemetry
}

fn service_loop_run_summary_telemetry(
    run_summary: &AgentClosedLoopRuntimeServiceRunSummary,
    loop_state_summary: &AgentClosedLoopRuntimeServiceLoopStateSummary,
    service_attempts: usize,
    blocked_reasons: &[String],
    preflight_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!(
            "service_loop_run_summary_status={}",
            run_summary.status.as_str()
        ),
        format!(
            "service_loop_run_summary_dispatch_executable={}",
            run_summary.dispatch_executable
        ),
        format!(
            "service_loop_run_summary_intake_clean={}",
            run_summary.intake_clean
        ),
        format!(
            "service_loop_run_summary_commands={}",
            run_summary.command_count
        ),
        format!(
            "service_loop_run_summary_command_gate_allowed={}",
            run_summary.command_gate_allowed
        ),
        format!(
            "service_loop_run_summary_side_effect_gates={}",
            run_summary.side_effect_gate_count
        ),
        format!(
            "service_loop_run_summary_blocked_side_effect_gates={}",
            run_summary.blocked_side_effect_gate_count
        ),
        format!("service_loop_run_summary_attempts={service_attempts}"),
        format!(
            "service_loop_run_summary_mode={}",
            loop_state_summary.mode.as_str()
        ),
        format!(
            "service_loop_run_summary_execution_health={}",
            loop_state_summary.execution_health_status.as_str()
        ),
        format!(
            "service_loop_run_summary_service_run_health={}",
            loop_state_summary.service_run_health_status.as_str()
        ),
        format!(
            "service_loop_run_summary_side_effect_admission_health={}",
            loop_state_summary
                .side_effect_admission_health_status
                .as_str()
        ),
        format!(
            "service_loop_run_summary_side_effect_dispatch_allowed={}",
            loop_state_summary.side_effect_dispatch_allowed
        ),
        format!(
            "service_loop_run_summary_memory_note_allowed={}",
            loop_state_summary.memory_note_allowed
        ),
        format!(
            "service_loop_run_summary_side_effect_admission_reasons={}",
            loop_state_summary.side_effect_admission_reasons
        ),
        format!(
            "service_loop_run_summary_requires_repair_first={}",
            loop_state_summary.requires_repair_first
        ),
        format!(
            "service_loop_run_summary_follow_up_tasks={}",
            loop_state_summary.preflight_follow_up_tasks
        ),
        format!(
            "service_loop_run_summary_next_queue_tasks={}",
            loop_state_summary.next_queue_tasks
        ),
    ];
    telemetry.extend(
        blocked_reasons
            .iter()
            .map(|reason| format!("service_loop_run_summary_blocked={reason}")),
    );
    telemetry.extend(
        preflight_reasons
            .iter()
            .map(|reason| format!("service_loop_run_summary_preflight_reason={reason}")),
    );
    telemetry
}

fn service_loop_run_history_record_telemetry(
    summary: &AgentClosedLoopRuntimeServiceLoopRunSummary,
    dashboard: &AgentClosedLoopRuntimeServiceLoopRunDashboard,
    health: &AgentClosedLoopRuntimeServiceLoopRunHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        format!(
            "service_loop_run_history_record_status={}",
            summary.service_run_status.as_str()
        ),
        format!(
            "service_loop_run_history_record_mode={}",
            summary.mode.as_str()
        ),
        format!(
            "service_loop_run_history_record_transitions={}",
            dashboard.total_runs
        ),
        format!(
            "service_loop_run_history_record_closed_runs={}",
            dashboard.closed_runs
        ),
        format!(
            "service_loop_run_history_record_repair_first_runs={}",
            dashboard.repair_first_runs
        ),
        format!(
            "service_loop_run_history_record_command_gate_allowed_runs={}",
            dashboard.command_gate_allowed_runs
        ),
        format!(
            "service_loop_run_history_record_side_effect_gates={}",
            dashboard.side_effect_gate_count
        ),
        format!(
            "service_loop_run_history_record_blocked_side_effect_gates={}",
            dashboard.blocked_side_effect_gate_count
        ),
        format!(
            "service_loop_run_history_record_side_effect_dispatch_allowed_runs={}",
            dashboard.side_effect_dispatch_allowed_runs
        ),
        format!(
            "service_loop_run_history_record_memory_note_allowed_runs={}",
            dashboard.memory_note_allowed_runs
        ),
        format!(
            "service_loop_run_history_record_adaptive_allowed_runs={}",
            dashboard.adaptive_allowed_runs
        ),
        format!(
            "service_loop_run_history_record_health={}",
            health.status.as_str()
        ),
        format!(
            "service_loop_run_history_record_follow_up_tasks={}",
            dashboard.follow_up_task_count
        ),
        format!(
            "service_loop_run_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("service_loop_run_history_record_reason={reason}")),
    );
    telemetry
}

fn service_loop_run_control_plan_telemetry(
    mode: AgentClosedLoopNextTurnMode,
    health: &AgentClosedLoopRuntimeServiceLoopRunHealth,
    next_queue_tasks: usize,
    reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!("service_loop_run_control_mode={}", mode.as_str()),
        format!("service_loop_run_control_health={}", health.status.as_str()),
        format!(
            "service_loop_run_control_transitions={}",
            health.dashboard.total_runs
        ),
        format!("service_loop_run_control_next_queue_tasks={next_queue_tasks}"),
        format!(
            "service_loop_run_control_can_schedule={}",
            mode.can_schedule() && next_queue_tasks > 0
        ),
        format!(
            "service_loop_run_control_adaptive_allowed={}",
            mode.allows_adaptive_evolution()
        ),
    ];
    telemetry.extend(
        reasons
            .iter()
            .map(|reason| format!("service_loop_run_control_reason={reason}")),
    );
    telemetry
}

fn service_loop_run_control_record_telemetry(
    history_record: &AgentClosedLoopRuntimeServiceLoopRunHistoryRecord,
    control_plan: &AgentClosedLoopRuntimeServiceLoopRunControlPlan,
) -> Vec<String> {
    let mut telemetry = history_record.telemetry.clone();
    telemetry.extend(control_plan.telemetry.clone());
    telemetry.push(format!(
        "service_loop_run_control_record_status={}",
        history_record.appended_summary.service_run_status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_control_record_transitions={}",
        history_record.transitions()
    ));
    telemetry.push(format!(
        "service_loop_run_control_record_mode={}",
        control_plan.mode.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_control_record_can_schedule={}",
        control_plan.can_schedule()
    ));
    telemetry.push(format!(
        "service_loop_run_control_record_adaptive_allowed={}",
        control_plan.allows_adaptive_evolution()
    ));
    telemetry
}

fn service_loop_run_control_summary_telemetry(
    latest: &AgentClosedLoopRuntimeServiceLoopRunSummary,
    control_plan: &AgentClosedLoopRuntimeServiceLoopRunControlPlan,
    dashboard: &AgentClosedLoopRuntimeServiceLoopRunDashboard,
    reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!(
            "service_loop_run_control_summary_status={}",
            latest.service_run_status.as_str()
        ),
        format!(
            "service_loop_run_control_summary_mode={}",
            control_plan.mode.as_str()
        ),
        format!(
            "service_loop_run_control_summary_health={}",
            control_plan.health.status.as_str()
        ),
        format!(
            "service_loop_run_control_summary_transitions={}",
            dashboard.total_runs
        ),
        format!(
            "service_loop_run_control_summary_closed_rate={:.3}",
            dashboard.closed_rate
        ),
        format!(
            "service_loop_run_control_summary_command_gate_allowed_rate={:.3}",
            dashboard.command_gate_allowed_rate
        ),
        format!(
            "service_loop_run_control_summary_side_effect_gates={}",
            dashboard.side_effect_gate_count
        ),
        format!(
            "service_loop_run_control_summary_blocked_side_effect_gates={}",
            dashboard.blocked_side_effect_gate_count
        ),
        format!(
            "service_loop_run_control_summary_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "service_loop_run_control_summary_side_effect_dispatch_allowed_rate={:.3}",
            dashboard.side_effect_dispatch_allowed_rate
        ),
        format!(
            "service_loop_run_control_summary_memory_note_allowed_rate={:.3}",
            dashboard.memory_note_allowed_rate
        ),
        format!(
            "service_loop_run_control_summary_adaptive_allowed_rate={:.3}",
            dashboard.adaptive_allowed_rate
        ),
        format!(
            "service_loop_run_control_summary_can_schedule={}",
            control_plan.can_schedule()
        ),
        format!(
            "service_loop_run_control_summary_adaptive_allowed={}",
            control_plan.allows_adaptive_evolution()
        ),
        format!(
            "service_loop_run_control_summary_next_queue_tasks={}",
            control_plan.next_queue.len()
        ),
    ];
    telemetry.extend(
        reasons
            .iter()
            .map(|reason| format!("service_loop_run_control_summary_reason={reason}")),
    );
    telemetry
}

fn service_loop_run_control_summary_history_record_telemetry(
    summary: &AgentClosedLoopRuntimeServiceLoopRunControlSummary,
    dashboard: &AgentClosedLoopRuntimeServiceLoopRunControlDashboard,
    health: &AgentClosedLoopRuntimeServiceLoopRunControlHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        format!(
            "service_loop_run_control_summary_history_record_status={}",
            summary.latest_status.as_str()
        ),
        format!(
            "service_loop_run_control_summary_history_record_mode={}",
            summary.mode.as_str()
        ),
        format!(
            "service_loop_run_control_summary_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "service_loop_run_control_summary_history_record_schedule_rate={:.3}",
            dashboard.schedule_rate
        ),
        format!(
            "service_loop_run_control_summary_history_record_command_gate_allowed_rate={:.3}",
            dashboard.command_gate_allowed_rate
        ),
        format!(
            "service_loop_run_control_summary_history_record_side_effect_gates={}",
            dashboard.side_effect_gate_count
        ),
        format!(
            "service_loop_run_control_summary_history_record_blocked_side_effect_gates={}",
            dashboard.blocked_side_effect_gate_count
        ),
        format!(
            "service_loop_run_control_summary_history_record_side_effect_dispatch_allowed_rate={:.3}",
            dashboard.side_effect_dispatch_allowed_rate
        ),
        format!(
            "service_loop_run_control_summary_history_record_memory_note_allowed_rate={:.3}",
            dashboard.memory_note_allowed_rate
        ),
        format!(
            "service_loop_run_control_summary_history_record_adaptive_allowed_rate={:.3}",
            dashboard.adaptive_allowed_rate
        ),
        format!(
            "service_loop_run_control_summary_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "service_loop_run_control_summary_history_record_health={}",
            health.status.as_str()
        ),
        format!(
            "service_loop_run_control_summary_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(
        health.reasons.iter().map(|reason| {
            format!("service_loop_run_control_summary_history_record_reason={reason}")
        }),
    );
    telemetry
}

fn service_loop_run_daemon_record_telemetry(
    loop_run: &AgentClosedLoopRuntimeServiceLoopRun,
    control_record: &AgentClosedLoopRuntimeServiceLoopRunControlRecord,
    control_summary_history_record: &AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecord,
) -> Vec<String> {
    let mut telemetry = loop_run.telemetry.clone();
    telemetry.extend(control_record.telemetry.clone());
    telemetry.extend(control_summary_history_record.telemetry.clone());
    telemetry.push(format!(
        "service_loop_run_daemon_record_status={}",
        loop_run.run.run_summary.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_record_mode={}",
        control_record.control_plan.mode.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_record_transition_health={}",
        control_record.history_record.health.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_record_control_health={}",
        control_summary_history_record.health.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_record_transitions={}",
        control_record.transitions()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_record_control_records={}",
        control_summary_history_record.records()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_record_can_schedule={}",
        control_record.can_schedule()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_record_adaptive_allowed={}",
        control_record.allows_adaptive_evolution()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_record_next_queue_tasks={}",
        control_record.control_plan.next_queue.len()
    ));
    telemetry
}

fn service_loop_run_daemon_continuation_telemetry(
    record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRecord,
    mode: AgentClosedLoopNextTurnMode,
    can_schedule: bool,
    side_effect_dispatch_allowed_rate: f32,
    memory_note_allowed_rate: f32,
    allows_adaptive_evolution: bool,
    requires_repair_first: bool,
    transition_health_status: AgentClosedLoopExecutionHealthStatus,
    control_health_status: AgentClosedLoopExecutionHealthStatus,
) -> Vec<String> {
    vec![
        format!(
            "service_loop_run_daemon_continuation_mode={}",
            mode.as_str()
        ),
        format!("service_loop_run_daemon_continuation_can_schedule={can_schedule}"),
        format!(
            "service_loop_run_daemon_continuation_side_effect_dispatch_allowed_rate={:.3}",
            side_effect_dispatch_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_continuation_memory_note_allowed_rate={:.3}",
            memory_note_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_continuation_adaptive_allowed={allows_adaptive_evolution}"
        ),
        format!(
            "service_loop_run_daemon_continuation_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "service_loop_run_daemon_continuation_transition_health={}",
            transition_health_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_continuation_control_health={}",
            control_health_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_continuation_service_attempts={}",
            record.loop_run.advance.run_record.attempts()
        ),
        format!(
            "service_loop_run_daemon_continuation_transitions={}",
            record.control_record.transitions()
        ),
        format!(
            "service_loop_run_daemon_continuation_control_records={}",
            record.control_summary_history_record.records()
        ),
        format!(
            "service_loop_run_daemon_continuation_next_queue_tasks={}",
            record.next_queue().len()
        ),
    ]
}

fn service_loop_run_daemon_input_plan_telemetry(
    continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
    receipt_count: usize,
    input: &AgentClosedLoopRuntimeServiceLoopRunDaemonInput,
) -> Vec<String> {
    let mut telemetry = vec![
        format!(
            "service_loop_run_daemon_input_plan_transition_health={}",
            continuation.transition_health_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_input_plan_control_health={}",
            continuation.control_health_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_input_plan_side_effect_dispatch_allowed_rate={:.3}",
            continuation.side_effect_dispatch_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_input_plan_memory_note_allowed_rate={:.3}",
            continuation.memory_note_allowed_rate
        ),
    ];
    telemetry.extend(service_loop_run_daemon_input_plan_state_telemetry(
        continuation.mode,
        continuation.can_schedule,
        continuation.allows_adaptive_evolution,
        continuation.requires_repair_first,
        receipt_count,
        input,
    ));
    telemetry
}

fn service_loop_run_daemon_input_plan_state_telemetry(
    mode: AgentClosedLoopNextTurnMode,
    can_schedule: bool,
    allows_adaptive_evolution: bool,
    requires_repair_first: bool,
    receipt_count: usize,
    input: &AgentClosedLoopRuntimeServiceLoopRunDaemonInput,
) -> Vec<String> {
    vec![
        format!("service_loop_run_daemon_input_plan_mode={}", mode.as_str()),
        format!(
            "service_loop_run_daemon_input_plan_can_schedule={}",
            can_schedule
        ),
        format!(
            "service_loop_run_daemon_input_plan_adaptive_allowed={}",
            allows_adaptive_evolution
        ),
        format!(
            "service_loop_run_daemon_input_plan_requires_repair_first={}",
            requires_repair_first
        ),
        format!("service_loop_run_daemon_input_plan_receipts={receipt_count}"),
        format!(
            "service_loop_run_daemon_input_plan_service_attempts={}",
            input.loop_run_input.service_run_history.len()
        ),
        format!(
            "service_loop_run_daemon_input_plan_transitions={}",
            input.loop_run_history.len()
        ),
        format!(
            "service_loop_run_daemon_input_plan_control_records={}",
            input.control_summary_history.len()
        ),
        format!(
            "service_loop_run_daemon_input_plan_next_queue_tasks={}",
            input
                .loop_run_input
                .service_run_input
                .request_input
                .runtime_input
                .next_queue
                .len()
        ),
    ]
}

fn service_loop_run_daemon_request_plan_telemetry(
    continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
    request_input: &AgentClosedLoopRuntimeServiceRequestInput,
    continuation_input: &AgentClosedLoopRuntimeContinuationInput,
) -> Vec<String> {
    vec![
        format!(
            "service_loop_run_daemon_request_plan_mode={}",
            continuation.mode.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_plan_transition_health={}",
            continuation.transition_health_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_plan_control_health={}",
            continuation.control_health_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_plan_can_schedule={}",
            continuation.can_schedule
        ),
        format!(
            "service_loop_run_daemon_request_plan_side_effect_dispatch_allowed_rate={:.3}",
            continuation.side_effect_dispatch_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_plan_memory_note_allowed_rate={:.3}",
            continuation.memory_note_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_plan_adaptive_allowed={}",
            continuation.allows_adaptive_evolution
        ),
        format!(
            "service_loop_run_daemon_request_plan_requires_repair_first={}",
            continuation.requires_repair_first
        ),
        format!(
            "service_loop_run_daemon_request_plan_next_queue_tasks={}",
            request_input.runtime_input.next_queue.len()
        ),
        format!(
            "service_loop_run_daemon_request_plan_service_attempts={}",
            continuation.service_run_history.len()
        ),
        format!(
            "service_loop_run_daemon_request_plan_transitions={}",
            continuation.loop_run_history.len()
        ),
        format!(
            "service_loop_run_daemon_request_plan_control_records={}",
            continuation.control_summary_history.len()
        ),
        format!(
            "service_loop_run_daemon_request_plan_completed_tasks={}",
            continuation_input.completed_task_ids.len()
        ),
        format!(
            "service_loop_run_daemon_request_plan_max_parallel={}",
            continuation_input.max_parallel_tasks
        ),
    ]
}

fn service_loop_run_daemon_request_record_telemetry(
    request_plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
    dispatch: &AgentClosedLoopRuntimeServiceDispatch,
    dispatch_summary: &AgentClosedLoopRuntimeServiceDispatchSummary,
) -> Vec<String> {
    let mut telemetry = request_plan.telemetry.clone();
    telemetry.extend(dispatch.telemetry.clone());
    telemetry.push(format!(
        "service_loop_run_daemon_request_record_executable={}",
        dispatch.is_executable()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_record_command_count={}",
        dispatch_summary.command_count
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_record_command_gate_allowed={}",
        dispatch_summary.command_gate_allowed
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_record_side_effect_gates={}",
        dispatch_summary.side_effect_gate_count
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_record_blocked_side_effect_gates={}",
        dispatch_summary.blocked_side_effect_gate_count
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_record_service_attempts={}",
        request_plan.service_run_history.len()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_record_transitions={}",
        request_plan.loop_run_history.len()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_record_control_records={}",
        request_plan.control_summary_history.len()
    ));
    telemetry.extend(
        dispatch_summary
            .blocked_reasons
            .iter()
            .map(|reason| format!("service_loop_run_daemon_request_record_blocked={reason}")),
    );
    telemetry.extend(
        dispatch
            .request
            .skipped_reasons()
            .iter()
            .map(|reason| format!("service_loop_run_daemon_request_record_skipped={reason}")),
    );
    telemetry
}

fn service_loop_run_daemon_request_summary_telemetry(
    request_plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
    dispatch_summary: &AgentClosedLoopRuntimeServiceDispatchSummary,
    skipped_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!(
            "service_loop_run_daemon_request_summary_executable={}",
            dispatch_summary.executable
        ),
        format!(
            "service_loop_run_daemon_request_summary_command_count={}",
            dispatch_summary.command_count
        ),
        format!(
            "service_loop_run_daemon_request_summary_command_gate_allowed={}",
            dispatch_summary.command_gate_allowed
        ),
        format!(
            "service_loop_run_daemon_request_summary_side_effect_gates={}",
            dispatch_summary.side_effect_gate_count
        ),
        format!(
            "service_loop_run_daemon_request_summary_blocked_side_effect_gates={}",
            dispatch_summary.blocked_side_effect_gate_count
        ),
        format!(
            "service_loop_run_daemon_request_summary_mode={}",
            request_plan.mode.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_summary_can_schedule={}",
            request_plan.can_schedule
        ),
        format!(
            "service_loop_run_daemon_request_summary_side_effect_dispatch_allowed_rate={:.3}",
            request_plan.side_effect_dispatch_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_summary_memory_note_allowed_rate={:.3}",
            request_plan.memory_note_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_summary_adaptive_allowed={}",
            request_plan.allows_adaptive_evolution
        ),
        format!(
            "service_loop_run_daemon_request_summary_requires_repair_first={}",
            request_plan.requires_repair_first
        ),
        format!(
            "service_loop_run_daemon_request_summary_service_attempts={}",
            request_plan.service_run_history.len()
        ),
        format!(
            "service_loop_run_daemon_request_summary_transitions={}",
            request_plan.loop_run_history.len()
        ),
        format!(
            "service_loop_run_daemon_request_summary_control_records={}",
            request_plan.control_summary_history.len()
        ),
    ];
    telemetry.extend(
        dispatch_summary
            .blocked_reasons
            .iter()
            .map(|reason| format!("service_loop_run_daemon_request_summary_blocked={reason}")),
    );
    telemetry.extend(
        skipped_reasons
            .iter()
            .map(|reason| format!("service_loop_run_daemon_request_summary_skipped={reason}")),
    );
    telemetry
}

fn service_loop_run_daemon_request_summary_history_record_telemetry(
    summary: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary,
    dashboard: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestDashboard,
    health: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        format!(
            "service_loop_run_daemon_request_summary_history_record_executable={}",
            summary.executable
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_executable_rate={:.3}",
            dashboard.executable_rate
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_command_gate_allowed_rate={:.3}",
            dashboard.command_gate_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_side_effect_gates={}",
            dashboard.side_effect_gate_count
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_blocked_side_effect_gates={}",
            dashboard.blocked_side_effect_gate_count
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_side_effect_dispatch_allowed_rate={:.3}",
            dashboard.side_effect_dispatch_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_memory_note_allowed_rate={:.3}",
            dashboard.memory_note_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_adaptive_allowed_rate={:.3}",
            dashboard.adaptive_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_blocked_records={}",
            dashboard.blocked_records
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_skipped_records={}",
            dashboard.skipped_records
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_command_count={}",
            dashboard.total_command_count
        ),
        format!(
            "service_loop_run_daemon_request_summary_history_record_health={}",
            health.status.as_str()
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!("service_loop_run_daemon_request_summary_history_record_reason={reason}")
    }));
    telemetry
}

fn service_loop_run_daemon_request_monitored_close_telemetry(
    request_history_record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecord,
    daemon_record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRecord,
) -> Vec<String> {
    let mut telemetry = request_history_record.telemetry.clone();
    telemetry.extend(daemon_record.telemetry.clone());
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_request_health={}",
        request_history_record.health.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_daemon_mode={}",
        daemon_record.mode().as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_daemon_control_health={}",
        daemon_record
            .control_summary_history_record
            .health
            .status
            .as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_request_records={}",
        request_history_record.records()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_control_records={}",
        daemon_record.control_summary_history_record.records()
    ));
    telemetry
}

fn service_loop_run_daemon_request_monitored_close_summary_telemetry(
    latest_request_summary: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary,
    request_health_status: AgentClosedLoopExecutionHealthStatus,
    daemon_run_status: AgentClosedLoopRuntimeServiceRunStatus,
    daemon_control_health_status: AgentClosedLoopExecutionHealthStatus,
    mode: AgentClosedLoopNextTurnMode,
    can_schedule: bool,
    side_effect_dispatch_allowed_rate: f32,
    memory_note_allowed_rate: f32,
    allows_adaptive_evolution: bool,
    requires_repair_first: bool,
    request_records: usize,
    service_attempts: usize,
    transitions: usize,
    control_records: usize,
    blocked_reasons: &[String],
    skipped_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_request_executable={}",
            latest_request_summary.executable
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_request_commands={}",
            latest_request_summary.command_count
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_request_command_gate_allowed={}",
            latest_request_summary.command_gate_allowed
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_request_side_effect_gates={}",
            latest_request_summary.side_effect_gate_count
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_request_blocked_side_effect_gates={}",
            latest_request_summary.blocked_side_effect_gate_count
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_request_mode={}",
            latest_request_summary.mode.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_request_health={}",
            request_health_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_daemon_run_status={}",
            daemon_run_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_daemon_control_health={}",
            daemon_control_health_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_mode={}",
            mode.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_can_schedule={can_schedule}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_side_effect_dispatch_allowed_rate={side_effect_dispatch_allowed_rate:.3}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_memory_note_allowed_rate={memory_note_allowed_rate:.3}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_adaptive_allowed={allows_adaptive_evolution}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_request_records={request_records}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_service_attempts={service_attempts}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_transitions={transitions}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_control_records={control_records}"
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!("service_loop_run_daemon_request_monitored_close_summary_blocked={reason}")
    }));
    telemetry.extend(skipped_reasons.iter().map(|reason| {
        format!("service_loop_run_daemon_request_monitored_close_summary_skipped={reason}")
    }));
    telemetry
}

fn service_loop_run_daemon_request_monitored_close_summary_history_record_telemetry(
    summary: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary,
    dashboard: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseDashboard,
    health: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_request_executable={}",
            summary.latest_request_executable
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_daemon_status={}",
            summary.daemon_run_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_request_executable_rate={:.3}",
            dashboard.request_executable_rate
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_request_command_gate_allowed_rate={:.3}",
            dashboard.request_command_gate_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_request_side_effect_gates={}",
            dashboard.request_side_effect_gate_count
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_request_blocked_side_effect_gates={}",
            dashboard.request_blocked_side_effect_gate_count
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_daemon_closed_rate={:.3}",
            dashboard.daemon_closed_rate
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_side_effect_dispatch_allowed_rate={:.3}",
            dashboard.side_effect_dispatch_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_memory_note_allowed_rate={:.3}",
            dashboard.memory_note_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_adaptive_allowed_rate={:.3}",
            dashboard.adaptive_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_request_repairs={}",
            dashboard.request_repair_records
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_control_repairs={}",
            dashboard.daemon_control_repair_records
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_repair_first={}",
            dashboard.repair_first_records
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_health={}",
            health.status.as_str()
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "service_loop_run_daemon_request_monitored_close_summary_history_record_reason={reason}"
        )
    }));
    telemetry
}

fn service_loop_run_daemon_request_monitored_close_continuation_telemetry(
    monitored_close_health: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth,
    request_health_status: AgentClosedLoopExecutionHealthStatus,
    daemon_control_health_status: AgentClosedLoopExecutionHealthStatus,
    mode: AgentClosedLoopNextTurnMode,
    can_schedule: bool,
    side_effect_dispatch_allowed_rate: f32,
    memory_note_allowed_rate: f32,
    allows_adaptive_evolution: bool,
    requires_repair_first: bool,
    request_records: usize,
    monitored_close_records: usize,
) -> Vec<String> {
    let mut telemetry = vec![
        format!(
            "service_loop_run_daemon_request_monitored_close_continuation_close_health={}",
            monitored_close_health.status.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_continuation_request_health={}",
            request_health_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_continuation_daemon_control_health={}",
            daemon_control_health_status.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_continuation_mode={}",
            mode.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_continuation_can_schedule={can_schedule}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_continuation_side_effect_dispatch_allowed_rate={side_effect_dispatch_allowed_rate:.3}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_continuation_memory_note_allowed_rate={memory_note_allowed_rate:.3}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_continuation_adaptive_allowed={allows_adaptive_evolution}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_continuation_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_continuation_request_records={request_records}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_close_continuation_close_records={monitored_close_records}"
        ),
    ];
    telemetry.extend(monitored_close_health.reasons.iter().map(|reason| {
        format!("service_loop_run_daemon_request_monitored_close_continuation_reason={reason}")
    }));
    telemetry
}

fn service_loop_run_daemon_request_monitored_continuation_telemetry(
    request_health: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth,
    daemon_control_health: &AgentClosedLoopRuntimeServiceLoopRunControlHealth,
    mode: AgentClosedLoopNextTurnMode,
    can_schedule: bool,
    side_effect_dispatch_allowed_rate: f32,
    memory_note_allowed_rate: f32,
    allows_adaptive_evolution: bool,
    requires_repair_first: bool,
    request_records: usize,
    control_records: usize,
) -> Vec<String> {
    vec![
        format!(
            "service_loop_run_daemon_request_monitored_continuation_request_health={}",
            request_health.status.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_continuation_daemon_control_health={}",
            daemon_control_health.status.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_continuation_mode={}",
            mode.as_str()
        ),
        format!(
            "service_loop_run_daemon_request_monitored_continuation_can_schedule={can_schedule}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_continuation_side_effect_dispatch_allowed_rate={side_effect_dispatch_allowed_rate:.3}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_continuation_memory_note_allowed_rate={memory_note_allowed_rate:.3}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_continuation_adaptive_allowed={allows_adaptive_evolution}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_continuation_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_continuation_request_records={request_records}"
        ),
        format!(
            "service_loop_run_daemon_request_monitored_continuation_control_records={control_records}"
        ),
    ]
}

fn service_loop_run_daemon_request_monitored_plan_telemetry(
    continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation,
    request_plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
) -> Vec<String> {
    let mut telemetry = request_plan.telemetry.clone();
    telemetry.push("service_loop_run_daemon_request_monitored_plan=true".to_owned());
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_plan_request_health={}",
        continuation.request_health.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_plan_daemon_control_health={}",
        continuation.daemon_control_health.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_plan_mode={}",
        continuation.mode.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_plan_request_records={}",
        continuation.request_summary_history.len()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_plan_control_records={}",
        continuation
            .daemon_continuation
            .control_summary_history
            .len()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_plan_can_schedule={}",
        continuation.can_schedule
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_plan_side_effect_dispatch_allowed_rate={:.3}",
        continuation.side_effect_dispatch_allowed_rate
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_plan_memory_note_allowed_rate={:.3}",
        continuation.memory_note_allowed_rate
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_plan_adaptive_allowed={}",
        continuation.allows_adaptive_evolution
    ));
    telemetry
}

fn service_loop_run_daemon_request_monitored_record_telemetry(
    monitored_plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
    request_record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord,
) -> Vec<String> {
    let mut telemetry = monitored_plan.telemetry.clone();
    telemetry.extend(request_record.telemetry.clone());
    telemetry.push("service_loop_run_daemon_request_monitored_record=true".to_owned());
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_record_executable={}",
        request_record.is_executable()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_record_request_records={}",
        monitored_plan.request_summary_history.len()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_record_request_health={}",
        monitored_plan.request_health.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_record_daemon_control_health={}",
        monitored_plan.daemon_control_health.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_record_side_effect_dispatch_allowed_rate={:.3}",
        monitored_plan.side_effect_dispatch_allowed_rate
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_record_memory_note_allowed_rate={:.3}",
        monitored_plan.memory_note_allowed_rate
    ));
    telemetry
}

fn service_loop_run_daemon_request_monitored_close_plan_telemetry(
    close_continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation,
    monitored_plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
) -> Vec<String> {
    let mut telemetry = monitored_plan.telemetry.clone();
    telemetry.extend(close_continuation.telemetry.clone());
    telemetry.push("service_loop_run_daemon_request_monitored_close_plan=true".to_owned());
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_plan_close_health={}",
        close_continuation.monitored_close_health.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_plan_request_health={}",
        close_continuation.request_health_status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_plan_daemon_control_health={}",
        close_continuation.daemon_control_health_status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_plan_mode={}",
        close_continuation.mode.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_plan_request_records={}",
        monitored_plan.request_summary_history.len()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_plan_close_records={}",
        close_continuation.monitored_close_summary_history.len()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_plan_can_schedule={}",
        close_continuation.can_schedule
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_plan_side_effect_dispatch_allowed_rate={:.3}",
        close_continuation.side_effect_dispatch_allowed_rate
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_plan_memory_note_allowed_rate={:.3}",
        close_continuation.memory_note_allowed_rate
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_plan_adaptive_allowed={}",
        close_continuation.allows_adaptive_evolution
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_plan_requires_repair_first={}",
        close_continuation.requires_repair_first
    ));
    telemetry
}

fn service_loop_run_daemon_request_monitored_close_run_record_telemetry(
    monitored_close_plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
    monitored_record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord,
) -> Vec<String> {
    let mut telemetry = monitored_close_plan.telemetry.clone();
    telemetry.extend(monitored_record.telemetry.clone());
    telemetry.push("service_loop_run_daemon_request_monitored_close_run_record=true".to_owned());
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_record_executable={}",
        monitored_record.is_executable()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_record_request_records={}",
        monitored_close_plan.request_history().len()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_record_close_records={}",
        monitored_close_plan.monitored_close_summary_history.len()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_record_close_health={}",
        monitored_close_plan.monitored_close_health.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_record_side_effect_dispatch_allowed_rate={:.3}",
        monitored_close_plan.side_effect_dispatch_allowed_rate
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_record_memory_note_allowed_rate={:.3}",
        monitored_close_plan.memory_note_allowed_rate
    ));
    telemetry
}

fn service_loop_run_daemon_request_monitored_close_run_summary_telemetry(
    request_summary: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary,
    monitored_close_plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
) -> Vec<String> {
    let mut telemetry = request_summary.telemetry.clone();
    telemetry.push("service_loop_run_daemon_request_monitored_close_run_summary=true".to_owned());
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_executable={}",
        request_summary.executable
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_commands={}",
        request_summary.command_count
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_command_gate_allowed={}",
        request_summary.command_gate_allowed
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_side_effect_gates={}",
        request_summary.side_effect_gate_count
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_blocked_side_effect_gates={}",
        request_summary.blocked_side_effect_gate_count
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_request_records={}",
        monitored_close_plan.request_history().len()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_close_records={}",
        monitored_close_plan.monitored_close_history().len()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_close_health={}",
        monitored_close_plan.monitored_close_health.status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_request_health={}",
        monitored_close_plan.request_health_status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_daemon_control_health={}",
        monitored_close_plan.daemon_control_health_status.as_str()
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_side_effect_dispatch_allowed_rate={:.3}",
        monitored_close_plan.side_effect_dispatch_allowed_rate
    ));
    telemetry.push(format!(
        "service_loop_run_daemon_request_monitored_close_run_summary_memory_note_allowed_rate={:.3}",
        monitored_close_plan.memory_note_allowed_rate
    ));
    telemetry
}

fn service_loop_run_daemon_input_plan_from_request_telemetry(
    request_plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
    receipt_count: usize,
) -> Vec<String> {
    vec![
        "service_loop_run_daemon_input_plan_from_request=true".to_owned(),
        format!("service_loop_run_daemon_input_plan_request_receipts={receipt_count}"),
        format!(
            "service_loop_run_daemon_input_plan_request_mode={}",
            request_plan.mode.as_str()
        ),
        format!(
            "service_loop_run_daemon_input_plan_request_can_schedule={}",
            request_plan.can_schedule
        ),
        format!(
            "service_loop_run_daemon_input_plan_request_side_effect_dispatch_allowed_rate={:.3}",
            request_plan.side_effect_dispatch_allowed_rate
        ),
        format!(
            "service_loop_run_daemon_input_plan_request_memory_note_allowed_rate={:.3}",
            request_plan.memory_note_allowed_rate
        ),
    ]
}

fn service_preflight_follow_up_task(
    preflight: &AgentClosedLoopRuntimeServicePreflight,
    index: usize,
    reason: &str,
) -> AgentTask {
    let kind = match preflight.mode {
        AgentClosedLoopNextTurnMode::Repair => "repair",
        AgentClosedLoopNextTurnMode::Observe => "observe",
        AgentClosedLoopNextTurnMode::Continue => "continue",
        AgentClosedLoopNextTurnMode::Idle => "idle",
    };
    AgentTask::new(
        format!(
            "service-preflight-{}-{}-{}",
            kind,
            index,
            stable_id(service_preflight_reason_key(reason))
        ),
        service_preflight_follow_up_role(reason),
        format!("{kind} service preflight: {reason}"),
        crate::budget::AgentBudget::new(16, 1, 1),
    )
    .with_lane("service-preflight")
    .with_priority(service_preflight_follow_up_priority(preflight.mode))
}

fn service_preflight_follow_up_priority(mode: AgentClosedLoopNextTurnMode) -> u8 {
    match mode {
        AgentClosedLoopNextTurnMode::Repair => 10,
        AgentClosedLoopNextTurnMode::Observe => 8,
        AgentClosedLoopNextTurnMode::Continue | AgentClosedLoopNextTurnMode::Idle => 5,
    }
}

fn service_preflight_follow_up_role(reason: &str) -> AgentRole {
    if reason.contains("intake") || reason.contains("unexpected") {
        AgentRole::Aggregator
    } else if reason.contains("closed_rate") || reason.contains("history_empty") {
        AgentRole::Tester
    } else if reason.contains("dispatch") || reason.contains("latest_blocked") {
        AgentRole::Reviewer
    } else {
        AgentRole::Planner
    }
}

fn service_preflight_reason_key(reason: &str) -> &str {
    reason
        .strip_prefix("service_run_")
        .unwrap_or(reason)
        .split(['=', '>', '<', ';'])
        .next()
        .unwrap_or(reason)
}

fn dispatch_run_id(dispatch: &AgentClosedLoopRuntimeServiceDispatch) -> String {
    dispatch
        .request
        .command_request
        .business_turn
        .step
        .as_ref()
        .map(|step| step.record.run_id.clone())
        .or_else(|| {
            dispatch
                .request
                .prior_history
                .latest()
                .map(|summary| summary.run_id.clone())
        })
        .unwrap_or_else(|| "runtime-service".to_owned())
}

fn service_intake_repair_task(run_id: &str, index: usize, reason: &str) -> AgentTask {
    AgentTask::new(
        format!(
            "service-intake-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(service_intake_reason_key(reason))
        ),
        service_intake_repair_role(reason),
        format!("repair service receipt intake: {reason}"),
        crate::budget::AgentBudget::new(16, 1, 1),
    )
    .with_lane("service-intake")
    .with_priority(9)
}

fn service_intake_repair_role(reason: &str) -> AgentRole {
    if reason.contains("enqueue_tasks") {
        AgentRole::Planner
    } else if reason.contains("emit_telemetry") || reason.contains("unexpected") {
        AgentRole::Aggregator
    } else {
        AgentRole::Reviewer
    }
}

fn service_intake_reason_key(reason: &str) -> &str {
    reason
        .strip_prefix("service_command_gate_blocked=")
        .or_else(|| reason.strip_prefix("service_receipt_rejected="))
        .unwrap_or(reason)
        .split(':')
        .next()
        .unwrap_or(reason)
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
    if id.is_empty() {
        "runtime-service".to_owned()
    } else {
        id
    }
}

fn service_outcome_summary_telemetry(
    runtime_mode: AgentClosedLoopNextTurnMode,
    command_count: usize,
    command_gate_allowed: bool,
    side_effect_gate_count: usize,
    blocked_side_effect_gate_count: usize,
    service_executed: bool,
    service_clean: bool,
    health_status: AgentClosedLoopExecutionHealthStatus,
    next_queue_tasks: usize,
    command_gate_blocked_reasons: &[String],
    skipped_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        format!("service_outcome_summary_mode={}", runtime_mode.as_str()),
        format!("service_outcome_summary_commands={command_count}"),
        format!("service_outcome_summary_command_gate_allowed={command_gate_allowed}"),
        format!("service_outcome_summary_side_effect_gates={side_effect_gate_count}"),
        format!(
            "service_outcome_summary_blocked_side_effect_gates={blocked_side_effect_gate_count}"
        ),
        format!("service_outcome_summary_executed={service_executed}"),
        format!("service_outcome_summary_clean={service_clean}"),
        format!("service_outcome_summary_health={}", health_status.as_str()),
        format!("service_outcome_summary_next_queue_tasks={next_queue_tasks}"),
    ];
    telemetry.extend(
        command_gate_blocked_reasons
            .iter()
            .map(|reason| format!("service_outcome_summary_command_gate_blocked={reason}")),
    );
    telemetry.extend(
        skipped_reasons
            .iter()
            .map(|reason| format!("service_outcome_summary_skipped={reason}")),
    );
    telemetry
}

fn service_request_telemetry(
    command_request: &AgentClosedLoopRuntimeServiceCommandRequest,
    prior_history: &AgentClosedLoopExecutionHistory,
) -> Vec<String> {
    let mut telemetry = command_request.business_turn.telemetry.clone();
    telemetry.extend(command_request.telemetry.clone());
    telemetry.push(format!(
        "service_request_commands={}",
        command_request
            .command_plan
            .as_ref()
            .map(|plan| plan.commands.len())
            .unwrap_or_default()
    ));
    telemetry.push(format!(
        "service_request_history_runs={}",
        prior_history.len()
    ));
    telemetry.push(format!(
        "service_request_ready={}",
        command_request.has_commands()
    ));
    telemetry.extend(
        command_request
            .skipped_reasons
            .iter()
            .map(|reason| format!("service_request_skipped={reason}")),
    );
    telemetry
}

fn continuation_telemetry(
    service_turn: &AgentClosedLoopRuntimeServiceTurn,
    dashboard: &AgentClosedLoopExecutionDashboard,
    health: &AgentClosedLoopExecutionHealth,
) -> Vec<String> {
    let mut telemetry = service_turn.telemetry.clone();
    telemetry.push(format!("continuation_runs={}", dashboard.total_runs));
    telemetry.push(format!(
        "continuation_clean_rate={:.3}",
        dashboard.clean_rate
    ));
    telemetry.push(format!("continuation_health={}", health.status.as_str()));
    telemetry.push(format!(
        "continuation_next_queue_tasks={}",
        service_turn.next_queue().len()
    ));
    telemetry
}

fn service_turn_telemetry(
    business_turn: &AgentClosedLoopRuntimeBusinessTurn,
    summary: Option<&AgentClosedLoopExecutionSummary>,
    skipped_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = business_turn.telemetry.clone();
    telemetry.push(format!("service_turn_execution={}", summary.is_some()));
    if let Some(summary) = summary {
        telemetry.push(format!("service_turn_clean={}", summary.clean));
        telemetry.push(format!("service_turn_commands={}", summary.command_count));
        telemetry.push(format!(
            "service_turn_missing_commands={}",
            summary.missing_command_count
        ));
        telemetry.push(format!(
            "service_turn_failed_commands={}",
            summary.failed_command_count
        ));
        telemetry.push(format!(
            "service_turn_next_queue_tasks={}",
            summary.next_queue_tasks
        ));
    }
    telemetry.extend(
        skipped_reasons
            .iter()
            .map(|reason| format!("service_turn_skipped={reason}")),
    );
    telemetry
}

fn service_command_request_telemetry(
    command_plan: Option<&AgentServiceCommandPlan>,
    skipped_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = Vec::new();
    telemetry.push(format!(
        "service_command_request={}",
        command_plan.is_some()
    ));
    telemetry.push(format!(
        "service_command_count={}",
        command_plan
            .map(|plan| plan.commands.len())
            .unwrap_or_default()
    ));
    telemetry.extend(
        skipped_reasons
            .iter()
            .map(|reason| format!("service_command_request_skipped={reason}")),
    );
    telemetry
}

fn business_turn_telemetry(
    runtime_turn: &AgentClosedLoopRuntimeTurn,
    has_step: bool,
    memory_submission: Option<&MemorySubmissionReport>,
    skipped_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = runtime_turn.telemetry.clone();
    telemetry.push(format!("business_turn_step={has_step}"));
    if let Some(submission) = memory_submission {
        telemetry.push(format!(
            "business_turn_memory_submitted={}",
            submission.submitted.len()
        ));
        telemetry.push(format!(
            "business_turn_memory_failures={}",
            submission.failures.len()
        ));
        telemetry.push(format!(
            "business_turn_memory_blockers={}",
            submission.blocked_reasons.len()
        ));
    }
    telemetry.extend(
        skipped_reasons
            .iter()
            .map(|reason| format!("business_turn_skipped={reason}")),
    );
    telemetry
}

fn runtime_turn_telemetry(
    prepared_cycle: &AgentClosedLoopPreparedCycle,
    skipped_reasons: &[String],
) -> Vec<String> {
    let prepared_execution = &prepared_cycle.prepared_execution;
    let turn_plan = &prepared_execution.prepared_dispatch.turn_plan;
    let mut telemetry = turn_plan.telemetry.clone();
    telemetry.push(format!("runtime_turn_mode={}", turn_plan.mode.as_str()));
    telemetry.push(format!(
        "runtime_turn_report={}",
        prepared_cycle.has_report()
    ));
    telemetry.push(format!(
        "runtime_turn_results={}",
        prepared_execution.result_count()
    ));
    telemetry.push(format!(
        "runtime_turn_failures={}",
        prepared_execution.failure_count()
    ));
    telemetry.extend(
        skipped_reasons
            .iter()
            .map(|reason| format!("runtime_turn_skipped={reason}")),
    );
    telemetry
}

fn extend_unique(target: &mut Vec<String>, source: &[String]) {
    for reason in source {
        if !target.contains(reason) {
            target.push(reason.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::AgentBudget;
    use crate::eval::AgentReportEvidence;
    use crate::ledger::AgentCycleLedgerAdmissionStatus;
    use crate::message::{AgentMessage, AgentMessageKind};
    use crate::ports::{
        AgentModelRouteProof, AgentModelRouteRequest, MemoryNote, MemoryPort, MemoryRecallRequest,
        MemoryRecord,
    };
    use crate::reflection::{ReflectionLoop, ReflectionStage};
    use crate::run::SideEffectKind;
    use crate::service::{
        AgentRustValidationCommand, AgentServiceCommand, AgentServiceCommandPlan,
        AgentServiceCommandPlanner, AgentServiceCommandReceipt, AgentServiceCommandStatus,
    };
    use crate::task::{AgentResult, AgentRole, AgentTask};

    #[derive(Debug, Clone)]
    struct CountingEngine {
        calls: usize,
        fail: bool,
    }

    impl EnginePort for CountingEngine {
        type Error = String;

        fn run_task(&mut self, task: &AgentTask) -> Result<AgentResult, Self::Error> {
            self.calls += 1;
            if self.fail {
                return Err(format!("engine failed {}", task.id));
            }
            Ok(AgentResult::accepted(
                task,
                format!("ran {}", task.id),
                vec![AgentMessage::new(
                    format!("message-{}", task.id),
                    task.role.clone(),
                    AgentMessageKind::Status,
                    "runtime",
                    "runtime ok",
                )],
                AgentBudget::new(1, 1, 1),
            ))
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

    #[derive(Debug, Clone, Default)]
    struct FakeMemory {
        fail: bool,
        submitted: Vec<MemoryNote>,
    }

    impl MemoryPort for FakeMemory {
        type Error = String;

        fn recall(
            &self,
            _request: &MemoryRecallRequest,
            _limit: usize,
        ) -> Result<Vec<MemoryRecord>, Self::Error> {
            Ok(Vec::new())
        }

        fn propose_note(&mut self, note: MemoryNote) -> Result<(), Self::Error> {
            if self.fail {
                return Err(format!("memory rejected {}", note.topic));
            }
            self.submitted.push(note);
            Ok(())
        }
    }

    fn history() -> AgentClosedLoopExecutionHistory {
        AgentClosedLoopExecutionHistory::from_summaries(vec![
            crate::step::AgentClosedLoopExecutionSummary {
                run_id: "run-1".to_owned(),
                clean: true,
                report_accepted: true,
                loopback_promoted: true,
                service_clean: true,
                reward_total: 0.90,
                admission_status: AgentCycleLedgerAdmissionStatus::Promote,
                command_count: 2,
                missing_command_count: 0,
                failed_command_count: 0,
                skipped_command_count: 0,
                next_queue_tasks: 1,
                next_queue_task_ids: vec!["runtime-turn".to_owned()],
                blocked_reasons: Vec::new(),
            },
        ])
    }

    fn queue() -> AgentTaskQueue {
        AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "runtime-turn",
            AgentRole::Planner,
            "run next runtime turn",
            AgentBudget::new(8, 1, 1),
        )])
    }

    fn budget() -> BudgetLedger {
        BudgetLedger::new().with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1))
    }

    fn complete_reflection() -> ReflectionLoop {
        let mut reflection = ReflectionLoop::new();
        reflection
            .submit(ReflectionStage::Draft, "draft accepted")
            .unwrap();
        reflection
            .submit(ReflectionStage::Critique, "no blocker")
            .unwrap();
        reflection
            .submit(ReflectionStage::Revision, "keep lesson")
            .unwrap();
        reflection
            .submit(ReflectionStage::MemoryNote, "remember runtime turn")
            .unwrap();
        reflection
    }

    fn clean_runtime_input() -> AgentClosedLoopRuntimeTurnInput {
        AgentClosedLoopRuntimeTurnInput::new(
            history(),
            queue(),
            budget(),
            AgentCycleEvidence {
                quality: 0.94,
                validation_passed: true,
                runtime_response_ok: true,
                reflection: Some(complete_reflection()),
                ..AgentCycleEvidence::default()
            },
        )
    }

    fn report_evidence() -> AgentReportEvidence {
        AgentReportEvidence::new(true, true)
            .with_validation_ref("eval:validation:pass")
            .with_runtime_ref("runtime:ok")
    }

    fn clean_business_turn() -> AgentClosedLoopRuntimeBusinessTurn {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn =
            AgentClosedLoopRuntimeTurnRunner::new().run(clean_runtime_input(), &mut engine);
        let mut memory = FakeMemory::default();
        AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-1",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        )
    }

    fn clean_service_run(run_id: &str) -> AgentClosedLoopRuntimeServiceRun {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let receipt_plan =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let receipts = receipt_plan
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();

        AgentClosedLoopRuntimeServiceRunner::new().run(
            AgentClosedLoopRuntimeServiceRunInput::new(
                AgentClosedLoopRuntimeServiceRequestInput::new(
                    clean_runtime_input(),
                    AgentClosedLoopRuntimeBusinessInput::new(
                        run_id,
                        crate::ledger::AgentCycleLedger::new(),
                        report_evidence(),
                    ),
                ),
                receipts,
                AgentClosedLoopRuntimeContinuationInput::new(
                    budget(),
                    AgentCycleEvidence::default(),
                ),
            ),
            &mut engine,
            &mut memory,
        )
    }

    fn clean_service_run_summary(run_id: &str) -> AgentClosedLoopRuntimeServiceRunSummary {
        clean_service_run(run_id).run_summary
    }

    fn intake_blocked_service_run(run_id: &str) -> AgentClosedLoopRuntimeServiceRun {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let receipt_plan =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let mut receipts = receipt_plan
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        receipts.push(AgentServiceCommandReceipt::new(
            "unexpected_command",
            AgentServiceCommandStatus::Applied,
            "rogue executor output",
        ));

        AgentClosedLoopRuntimeServiceRunner::new().run(
            AgentClosedLoopRuntimeServiceRunInput::new(
                AgentClosedLoopRuntimeServiceRequestInput::new(
                    clean_runtime_input(),
                    AgentClosedLoopRuntimeBusinessInput::new(
                        run_id,
                        crate::ledger::AgentCycleLedger::new(),
                        report_evidence(),
                    ),
                ),
                receipts,
                AgentClosedLoopRuntimeContinuationInput::new(
                    budget(),
                    AgentCycleEvidence::default(),
                ),
            ),
            &mut engine,
            &mut memory,
        )
    }

    fn intake_blocked_service_run_summary(run_id: &str) -> AgentClosedLoopRuntimeServiceRunSummary {
        intake_blocked_service_run(run_id).run_summary
    }

    fn clean_service_loop_run(run_id: &str) -> AgentClosedLoopRuntimeServiceLoopRun {
        let run = clean_service_run(run_id);
        let advance = AgentClosedLoopRuntimeServiceLoopAdvancePlanner::new().advance(
            AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
                clean_service_run_summary("run-service-loop-history-prior-clean"),
            ]),
            &run,
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );
        AgentClosedLoopRuntimeServiceLoopRun {
            run,
            advance,
            telemetry: Vec::new(),
        }
    }

    fn clean_service_loop_run_summary(run_id: &str) -> AgentClosedLoopRuntimeServiceLoopRunSummary {
        clean_service_loop_run(run_id).compact_summary()
    }

    fn clean_service_loop_run_receipts() -> Vec<AgentServiceCommandReceipt> {
        let receipt_plan =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        receipt_plan
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>()
    }

    fn service_loop_run_input_with_receipts(
        run_id: &str,
        receipts: Vec<AgentServiceCommandReceipt>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunInput {
        AgentClosedLoopRuntimeServiceLoopRunInput::new(
            AgentClosedLoopRuntimeServiceRunInput::new(
                AgentClosedLoopRuntimeServiceRequestInput::new(
                    clean_runtime_input(),
                    AgentClosedLoopRuntimeBusinessInput::new(
                        run_id,
                        crate::ledger::AgentCycleLedger::new(),
                        report_evidence(),
                    ),
                ),
                receipts,
                AgentClosedLoopRuntimeContinuationInput::new(
                    budget(),
                    AgentCycleEvidence::default(),
                ),
            ),
            AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
                clean_service_run_summary("run-service-loop-daemon-service-prior"),
            ]),
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        )
    }

    fn clean_daemon_continuation(
        run_id: &str,
        engine: &mut CountingEngine,
        memory: &mut FakeMemory,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation {
        let input = AgentClosedLoopRuntimeServiceLoopRunDaemonInput::new(
            service_loop_run_input_with_receipts(run_id, clean_service_loop_run_receipts()),
            AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                clean_service_loop_run_summary("run-service-loop-daemon-helper-prior"),
            ]),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::new(),
            AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default(),
        );

        AgentClosedLoopRuntimeServiceLoopRunDaemonRunner::new()
            .run(input, engine, memory)
            .continuation()
    }

    fn daemon_continuation_for_runtime_input(
        runtime_input: AgentClosedLoopRuntimeTurnInput,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation {
        AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation {
            next_runtime_input: runtime_input,
            service_run_history: AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
                clean_service_run_summary("run-service-loop-daemon-request-prior-service"),
            ]),
            loop_run_history: AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                clean_service_loop_run_summary("run-service-loop-daemon-request-prior-transition"),
            ]),
            control_summary_history: AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::new(
            ),
            service_run_policy: AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
            loop_run_health_policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            control_health_policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default(
            ),
            mode: AgentClosedLoopNextTurnMode::Continue,
            can_schedule: true,
            side_effect_dispatch_allowed_rate: 0.0,
            memory_note_allowed_rate: 0.0,
            allows_adaptive_evolution: true,
            requires_repair_first: false,
            transition_health_status: AgentClosedLoopExecutionHealthStatus::Stable,
            control_health_status: AgentClosedLoopExecutionHealthStatus::Stable,
            telemetry: Vec::new(),
        }
    }

    fn intake_blocked_service_loop_run(run_id: &str) -> AgentClosedLoopRuntimeServiceLoopRun {
        let run = intake_blocked_service_run(run_id);
        let advance = AgentClosedLoopRuntimeServiceLoopAdvancePlanner::new().advance(
            AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
                clean_service_run_summary("run-service-loop-history-prior-clean"),
            ]),
            &run,
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );
        AgentClosedLoopRuntimeServiceLoopRun {
            run,
            advance,
            telemetry: Vec::new(),
        }
    }

    fn intake_blocked_service_loop_run_summary(
        run_id: &str,
    ) -> AgentClosedLoopRuntimeServiceLoopRunSummary {
        intake_blocked_service_loop_run(run_id).compact_summary()
    }

    #[test]
    fn runtime_turn_runner_executes_and_closes_report() {
        let input = AgentClosedLoopRuntimeTurnInput::new(
            history(),
            queue(),
            budget(),
            AgentCycleEvidence {
                quality: 0.80,
                validation_passed: true,
                runtime_response_ok: true,
                ..AgentCycleEvidence::default()
            },
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };

        let turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);

        assert_eq!(engine.calls, 1);
        assert_eq!(turn.mode(), AgentClosedLoopNextTurnMode::Continue);
        assert!(turn.has_report());
        assert!(turn.skipped_reasons.is_empty());
        assert_eq!(turn.prepared_execution().result_count(), 1);
        assert_eq!(turn.report().unwrap().dispatch.assignments.len(), 1);
        assert!(
            turn.telemetry
                .iter()
                .any(|line| line == "runtime_turn_report=true")
        );
    }

    #[test]
    fn runtime_turn_runner_uses_supplied_layer_b_routes() {
        let input = clean_runtime_input().with_model_routes(vec![route_request(AgentTask::new(
            "runtime-turn",
            AgentRole::Planner,
            "run next runtime turn",
            AgentBudget::new(8, 1, 1),
        ))]);
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };

        let turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);

        assert_eq!(engine.calls, 1);
        assert!(turn.has_report());
        let execution = turn.prepared_execution().execution.as_ref().unwrap();
        let route_gate = execution.results[0]
            .messages
            .iter()
            .find(|message| message.topic == "layer_b_model_route")
            .expect("runtime turn should carry Layer B route gate");
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
                .any(|line| line == "agent_model_route_prompt_chars=23")
        );
    }

    #[test]
    fn runtime_turn_runner_blocks_missing_supplied_layer_b_route_before_engine_call() {
        let input = clean_runtime_input().with_model_routes(vec![route_request(AgentTask::new(
            "other-runtime-turn",
            AgentRole::Planner,
            "not the dispatched task",
            AgentBudget::new(8, 1, 1),
        ))]);
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };

        let turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);

        assert_eq!(engine.calls, 0);
        assert!(turn.has_report());
        assert_eq!(turn.prepared_execution().failure_count(), 1);
        assert_eq!(
            turn.report().unwrap().execution_failures[0].reason,
            "assigned task missing Layer B model route proof"
        );
        assert!(
            turn.telemetry
                .iter()
                .any(|line| line == "runtime_turn_failures=1")
        );
    }

    #[test]
    fn runtime_turn_runner_idles_without_engine_or_report() {
        let input = AgentClosedLoopRuntimeTurnInput::new(
            history(),
            AgentTaskQueue::new(),
            budget(),
            AgentCycleEvidence::default(),
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };

        let turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);

        assert_eq!(engine.calls, 0);
        assert_eq!(turn.mode(), AgentClosedLoopNextTurnMode::Idle);
        assert!(!turn.has_report());
        assert_eq!(turn.skipped_reasons, vec!["next_queue_empty"]);
        assert!(
            turn.telemetry
                .iter()
                .any(|line| line == "runtime_turn_skipped=next_queue_empty")
        );
    }

    #[test]
    fn runtime_turn_runner_preserves_engine_failure_in_report() {
        let input = AgentClosedLoopRuntimeTurnInput::new(
            history(),
            queue(),
            budget(),
            AgentCycleEvidence {
                quality: 0.80,
                validation_passed: true,
                runtime_response_ok: true,
                ..AgentCycleEvidence::default()
            },
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: true,
        };

        let turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);

        assert_eq!(engine.calls, 1);
        assert!(turn.has_report());
        assert_eq!(turn.prepared_execution().failure_count(), 1);
        assert_eq!(turn.report().unwrap().execution_failures.len(), 1);
        assert_eq!(
            turn.report().unwrap().execution_failures[0].reason,
            "engine failed runtime-turn"
        );
        assert!(
            turn.telemetry
                .iter()
                .any(|line| line == "runtime_turn_failures=1")
        );
    }

    #[test]
    fn runtime_business_turn_closer_submits_memory_and_closes_step() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn =
            AgentClosedLoopRuntimeTurnRunner::new().run(clean_runtime_input(), &mut engine);
        let mut memory = FakeMemory::default();

        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-business-1",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        );

        assert_eq!(memory.submitted.len(), 1);
        assert!(business_turn.has_step());
        assert!(business_turn.skipped_reasons.is_empty());
        assert_eq!(
            business_turn
                .memory_submission
                .as_ref()
                .unwrap()
                .submitted
                .len(),
            1
        );
        assert!(business_turn.step().unwrap().report_decision.is_accepted());
        assert!(
            business_turn
                .telemetry
                .iter()
                .any(|line| line == "business_turn_memory_submitted=1")
        );
    }

    #[test]
    fn runtime_business_turn_closer_skips_when_runtime_turn_has_no_report() {
        let input = AgentClosedLoopRuntimeTurnInput::new(
            history(),
            AgentTaskQueue::new(),
            budget(),
            AgentCycleEvidence::default(),
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);
        let mut memory = FakeMemory::default();

        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-business-2",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        );

        assert_eq!(engine.calls, 0);
        assert!(memory.submitted.is_empty());
        assert!(!business_turn.has_step());
        assert_eq!(business_turn.handoff, None);
        assert_eq!(business_turn.skipped_reasons, vec!["next_queue_empty"]);
        assert!(
            business_turn
                .telemetry
                .iter()
                .any(|line| line == "business_turn_skipped=next_queue_empty")
        );
    }

    #[test]
    fn runtime_business_turn_closer_preserves_memory_failure_in_step() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn =
            AgentClosedLoopRuntimeTurnRunner::new().run(clean_runtime_input(), &mut engine);
        let mut memory = FakeMemory {
            fail: true,
            submitted: Vec::new(),
        };

        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-business-3",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        );

        assert!(memory.submitted.is_empty());
        assert!(business_turn.has_step());
        assert_eq!(
            business_turn
                .memory_submission
                .as_ref()
                .unwrap()
                .failures
                .len(),
            1
        );
        let step = business_turn.step().unwrap();
        assert!(!step.report_decision.is_accepted());
        assert!(
            step.report_decision
                .reasons
                .iter()
                .any(|reason| reason.code == "memory_submission_failures")
        );
        assert!(
            business_turn
                .telemetry
                .iter()
                .any(|line| line == "business_turn_memory_failures=1")
        );
    }

    #[test]
    fn runtime_service_command_planner_exposes_command_request_for_business_turn() {
        let business_turn = clean_business_turn();

        let request = AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(business_turn);

        assert!(request.has_commands());
        assert!(request.skipped_reasons.is_empty());
        assert_eq!(request.command_kinds()[0], "promote_adaptive_state");
        assert!(
            request
                .telemetry
                .iter()
                .any(|line| line == "service_command_request=true")
        );
    }

    #[test]
    fn runtime_service_command_gate_allows_admitted_commands() {
        let request =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());

        let gate = request.gate();

        assert!(gate.is_allowed());
        assert_eq!(gate.command_count, request.command_kinds().len());
        assert_eq!(gate.entries[0].command_kind, "promote_adaptive_state");
        assert_eq!(
            gate.entries[0].side_effect,
            SideEffectKind::AdaptiveStateWrite
        );
        assert!(gate.entries.iter().all(|entry| entry.allowed));
        assert!(
            gate.telemetry
                .iter()
                .any(|line| line == "service_command_gate_allowed=true")
        );
    }

    #[test]
    fn runtime_service_command_gate_allows_planned_rust_validation() {
        let command = AgentServiceCommand::RunRustValidation {
            commands: vec![AgentRustValidationCommand::Check],
            reasons: vec!["tool_build_blocked_cycles=1>0".to_owned()],
        };

        let gate = gate_service_command(&command, None);

        assert!(gate.allowed);
        assert_eq!(gate.command_kind, "run_rust_validation");
        assert_eq!(gate.side_effect, SideEffectKind::ExternalCall);
        assert_eq!(gate.reason, "rust_validation_commands_planned");

        let empty_command = AgentServiceCommand::RunRustValidation {
            commands: Vec::new(),
            reasons: vec!["tool_build_blocked_cycles=1>0".to_owned()],
        };
        let blocked = gate_service_command(&empty_command, None);

        assert!(!blocked.allowed);
        assert_eq!(
            blocked.reason,
            "rust_validation_requires_non_empty_commands"
        );
    }

    #[test]
    fn runtime_service_command_planner_skips_business_turn_without_step() {
        let input = AgentClosedLoopRuntimeTurnInput::new(
            history(),
            AgentTaskQueue::new(),
            budget(),
            AgentCycleEvidence::default(),
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);
        let mut memory = FakeMemory::default();
        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-request-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        );

        let request = AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(business_turn);

        assert!(!request.has_commands());
        assert!(request.command_plan.is_none());
        assert_eq!(request.skipped_reasons, vec!["next_queue_empty"]);
        assert!(
            request
                .telemetry
                .iter()
                .any(|line| line == "service_command_request_skipped=next_queue_empty")
        );
    }

    #[test]
    fn runtime_service_command_gate_blocks_skipped_request() {
        let input = AgentClosedLoopRuntimeTurnInput::new(
            history(),
            AgentTaskQueue::new(),
            budget(),
            AgentCycleEvidence::default(),
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);
        let mut memory = FakeMemory::default();
        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-gate-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        );
        let request = AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(business_turn);

        let gate = request.gate();

        assert!(!gate.is_allowed());
        assert_eq!(gate.command_count, 0);
        assert!(gate.entries.is_empty());
        assert_eq!(gate.blocked_reasons, vec!["next_queue_empty".to_owned()]);
        assert!(
            gate.telemetry
                .iter()
                .any(|line| line == "service_command_gate_plan_missing=true")
        );
    }

    #[test]
    fn runtime_service_command_gate_blocks_unadmitted_adaptive_write() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn =
            AgentClosedLoopRuntimeTurnRunner::new().run(clean_runtime_input(), &mut engine);
        let mut memory = FakeMemory {
            fail: true,
            submitted: Vec::new(),
        };
        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-gate-block",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        );
        assert!(
            !business_turn
                .step()
                .unwrap()
                .business_plan
                .can_promote_adaptive_state()
        );
        let mut request = AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(business_turn);
        request.command_plan = Some(AgentServiceCommandPlan {
            commands: vec![AgentServiceCommand::PromoteAdaptiveState(
                crate::control::AdaptiveStateCandidate {
                    run_id: "tampered-run".to_owned(),
                    reward_total: 0.99,
                    acceptance_rate: 1.0,
                    average_reward_total: 0.99,
                    evidence_refs: vec!["tampered:evidence".to_owned()],
                },
            )],
        });

        let gate = request.gate();

        assert!(!gate.is_allowed());
        assert_eq!(gate.command_count, 1);
        assert_eq!(gate.entries[0].command_kind, "promote_adaptive_state");
        assert!(!gate.entries[0].allowed);
        assert_eq!(gate.entries[0].reason, "business_loop_promote_not_admitted");
        assert_eq!(
            gate.blocked_reasons,
            vec![
                "service_command_gate_blocked=promote_adaptive_state:business_loop_promote_not_admitted"
                    .to_owned()
            ]
        );

        let dispatch = AgentClosedLoopRuntimeServiceRequest {
            command_request: request,
            prior_history: history(),
            telemetry: Vec::new(),
        }
        .into_dispatch();
        let summary = dispatch.summary();

        assert!(!summary.executable);
        assert!(!summary.command_gate_allowed);
        assert_eq!(summary.side_effect_gate_count, 1);
        assert_eq!(summary.blocked_side_effect_gate_count, 1);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "service_dispatch_summary_blocked_side_effect_gates=1" })
        );
    }

    #[test]
    fn runtime_service_request_runner_reaches_command_boundary_without_receipts() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-request-1",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );

        let request =
            AgentClosedLoopRuntimeServiceRequestRunner::new().run(input, &mut engine, &mut memory);

        assert_eq!(engine.calls, 1);
        assert_eq!(memory.submitted.len(), 1);
        assert!(request.has_commands());
        assert_eq!(request.command_kinds()[0], "promote_adaptive_state");
        assert_eq!(request.prior_history.len(), history().len());
        assert!(request.command_request.business_turn.has_step());
        assert!(
            request
                .telemetry
                .iter()
                .any(|line| line == "service_request_ready=true")
        );
        assert!(
            request
                .telemetry
                .iter()
                .any(|line| line == "service_request_history_runs=1")
        );
    }

    #[test]
    fn runtime_service_dispatch_exposes_executable_command_plan_after_gate() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-1",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );

        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );

        assert!(dispatch.is_executable());
        assert!(dispatch.command_gate.is_allowed());
        assert_eq!(
            dispatch.command_plan().unwrap().command_kinds()[0],
            "promote_adaptive_state"
        );
        assert!(
            dispatch
                .telemetry
                .iter()
                .any(|line| line == "service_dispatch_executable=true")
        );
    }

    #[test]
    fn runtime_service_dispatch_summary_compacts_executable_dispatch() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-summary-clean",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );

        let summary = dispatch.summary();

        assert!(summary.executable);
        assert_eq!(
            summary.command_count,
            dispatch.command_plan().unwrap().commands.len()
        );
        assert!(summary.command_gate_allowed);
        assert_eq!(summary.side_effect_gate_count, summary.command_count);
        assert_eq!(summary.blocked_side_effect_gate_count, 0);
        assert_eq!(summary.command_kinds[0], "promote_adaptive_state");
        assert!(summary.blocked_reasons.is_empty());
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "service_dispatch_summary_executable=true")
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "service_dispatch_summary_command_gate_allowed=true" })
        );
    }

    #[test]
    fn runtime_service_dispatch_history_empty_is_watch() {
        let health = AgentClosedLoopRuntimeServiceDispatchSummaryHistory::new()
            .health(AgentClosedLoopRuntimeServiceDispatchHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["runtime_service_dispatch_history_empty"]
        );
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(health.dashboard.total_dispatches, 0);
    }

    #[test]
    fn runtime_service_dispatch_history_marks_executable_dispatch_stable() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-history-clean",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );

        let record = AgentClosedLoopRuntimeServiceDispatchSummaryHistoryRecorder::new()
            .record_dispatch(
                AgentClosedLoopRuntimeServiceDispatchSummaryHistory::new(),
                &dispatch,
                AgentClosedLoopRuntimeServiceDispatchHealthPolicy::default(),
            );

        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.health.is_stable());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.total_dispatches, 1);
        assert_eq!(record.dashboard.executable_dispatches, 1);
        assert_eq!(record.dashboard.blocked_dispatches, 0);
        assert_eq!(record.dashboard.blocked_side_effect_gate_count, 0);
        assert_eq!(
            record.dashboard.latest_command_kinds[0],
            "promote_adaptive_state"
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "service_dispatch_history_record_status=stable" })
        );
    }

    #[test]
    fn runtime_service_dispatch_history_watches_low_executable_rate_when_blocks_allowed() {
        let clean = AgentClosedLoopRuntimeServiceDispatchSummary {
            executable: true,
            command_count: 2,
            command_gate_allowed: true,
            side_effect_gate_count: 2,
            blocked_side_effect_gate_count: 0,
            command_kinds: vec![
                "promote_adaptive_state".to_owned(),
                "emit_telemetry".to_owned(),
            ],
            blocked_reasons: Vec::new(),
            telemetry: Vec::new(),
        };
        let blocked = AgentClosedLoopRuntimeServiceDispatchSummary {
            executable: false,
            command_count: 1,
            command_gate_allowed: false,
            side_effect_gate_count: 1,
            blocked_side_effect_gate_count: 1,
            command_kinds: vec!["promote_adaptive_state".to_owned()],
            blocked_reasons: vec![
                "service_command_gate_blocked=promote_adaptive_state:blocked".to_owned(),
            ],
            telemetry: Vec::new(),
        };
        let policy = AgentClosedLoopRuntimeServiceDispatchHealthPolicy {
            maximum_blocked_dispatches: usize::MAX,
            maximum_blocked_side_effect_gates: usize::MAX,
            maximum_blocked_reasons: usize::MAX,
            ..AgentClosedLoopRuntimeServiceDispatchHealthPolicy::default()
        };

        let health = AgentClosedLoopRuntimeServiceDispatchSummaryHistory::from_summaries(vec![
            clean, blocked,
        ])
        .health(policy);

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.dashboard.executable_rate, 0.5);
        assert_eq!(
            health.reasons,
            vec!["runtime_service_dispatch_executable_rate=0.500<0.67"]
        );
    }

    #[test]
    fn runtime_service_dispatch_history_repairs_blocked_dispatch_and_preserves_order() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let clean_input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-history-clean-prior",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let clean_dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            clean_input,
            &mut engine,
            &mut memory,
        );
        let runtime_turn =
            AgentClosedLoopRuntimeTurnRunner::new().run(clean_runtime_input(), &mut engine);
        let mut failing_memory = FakeMemory {
            fail: true,
            submitted: Vec::new(),
        };
        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-history-blocked",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut failing_memory,
        );
        let mut request = AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(business_turn);
        request.command_plan = Some(AgentServiceCommandPlan {
            commands: vec![AgentServiceCommand::PromoteAdaptiveState(
                crate::control::AdaptiveStateCandidate {
                    run_id: "tampered-run".to_owned(),
                    reward_total: 0.99,
                    acceptance_rate: 1.0,
                    average_reward_total: 0.99,
                    evidence_refs: vec!["tampered:evidence".to_owned()],
                },
            )],
        });
        let blocked_dispatch = AgentClosedLoopRuntimeServiceRequest {
            command_request: request,
            prior_history: history(),
            telemetry: Vec::new(),
        }
        .into_dispatch();
        let recorder = AgentClosedLoopRuntimeServiceDispatchSummaryHistoryRecorder::new();
        let first = recorder.record_dispatch(
            AgentClosedLoopRuntimeServiceDispatchSummaryHistory::new(),
            &clean_dispatch,
            AgentClosedLoopRuntimeServiceDispatchHealthPolicy::default(),
        );
        let second = recorder.record_dispatch(
            first.history,
            &blocked_dispatch,
            AgentClosedLoopRuntimeServiceDispatchHealthPolicy::default(),
        );

        assert_eq!(
            second.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(second.dashboard.total_dispatches, 2);
        assert_eq!(second.dashboard.executable_dispatches, 1);
        assert_eq!(second.dashboard.blocked_dispatches, 1);
        assert_eq!(second.dashboard.blocked_side_effect_gate_count, 1);
        assert_eq!(second.dashboard.blocked_reason_count, 1);
        assert_eq!(
            second
                .history
                .summaries()
                .iter()
                .map(|summary| summary.executable)
                .collect::<Vec<_>>(),
            vec![true, false]
        );
        assert_eq!(
            second.dashboard.latest_blocked_reasons,
            vec![
                "service_command_gate_blocked=promote_adaptive_state:business_loop_promote_not_admitted"
            ]
        );
        assert!(
            second
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "runtime_service_dispatch_blocked_dispatches=1>0" })
        );
    }

    #[test]
    fn runtime_service_dispatch_intake_closes_clean_receipts() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-intake-clean",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );
        let receipts = dispatch
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();

        let dispatch_outcome = dispatch.close_with_intake(
            receipts,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        assert!(dispatch_outcome.has_outcome());
        assert!(dispatch_outcome.intake.can_close_outcome());
        assert!(dispatch_outcome.intake.rejected_receipts.is_empty());
        assert!(dispatch_outcome.repair_plan.tasks.is_empty());
        assert!(dispatch_outcome.repair_queue().is_empty());
        assert_eq!(
            dispatch_outcome
                .outcome
                .as_ref()
                .unwrap()
                .continuation
                .health
                .status
                .as_str(),
            "stable"
        );
        assert!(
            dispatch_outcome
                .telemetry
                .iter()
                .any(|line| line == "service_dispatch_outcome=true")
        );
    }

    #[test]
    fn runtime_service_receipt_intake_history_empty_is_watch() {
        let health = AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory::new()
            .health(AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["runtime_service_receipt_intake_history_empty"]
        );
        assert_eq!(health.dashboard.total_intakes, 0);
    }

    #[test]
    fn runtime_service_receipt_intake_history_marks_clean_intake_stable() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-intake-history-clean",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );
        let receipts = dispatch
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let intake = dispatch.intake_receipts(receipts);

        let record = AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistoryRecorder::new()
            .record_intake(
                AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory::new(),
                &intake,
                AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy::default(),
            );

        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.health.is_stable());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.total_intakes, 1);
        assert_eq!(record.dashboard.clean_intakes, 1);
        assert_eq!(record.dashboard.rejected_receipt_count, 0);
        assert_eq!(record.dashboard.blocked_reason_count, 0);
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "service_receipt_intake_history_record_status=stable" })
        );
    }

    #[test]
    fn runtime_service_receipt_intake_history_watches_low_clean_rate_when_drift_allowed() {
        let clean = AgentClosedLoopRuntimeServiceReceiptIntakeSummary {
            executable: true,
            expected_receipts: 2,
            accepted_receipts: 2,
            rejected_receipts: 0,
            clean: true,
            blocked_reasons: Vec::new(),
            telemetry: Vec::new(),
        };
        let dirty = AgentClosedLoopRuntimeServiceReceiptIntakeSummary {
            executable: true,
            expected_receipts: 2,
            accepted_receipts: 1,
            rejected_receipts: 1,
            clean: false,
            blocked_reasons: vec![
                "service_receipt_rejected=unexpected:receipt_command_unexpected_or_duplicate"
                    .to_owned(),
            ],
            telemetry: Vec::new(),
        };
        let policy = AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy {
            maximum_rejected_receipts: usize::MAX,
            maximum_blocked_reasons: usize::MAX,
            ..AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy::default()
        };

        let health =
            AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory::from_summaries(vec![
                clean, dirty,
            ])
            .health(policy);

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.dashboard.clean_rate, 0.5);
        assert_eq!(
            health.reasons,
            vec!["runtime_service_receipt_intake_clean_rate=0.500<0.67"]
        );
    }

    #[test]
    fn runtime_service_receipt_intake_history_repairs_rejected_receipts_and_preserves_order() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let clean_input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-intake-history-clean-prior",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let clean_dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            clean_input,
            &mut engine,
            &mut memory,
        );
        let clean_receipts = clean_dispatch
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let clean_intake = clean_dispatch.intake_receipts(clean_receipts);
        let dirty_input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-intake-history-dirty",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let dirty_dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            dirty_input,
            &mut engine,
            &mut memory,
        );
        let mut dirty_receipts = dirty_dispatch
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        dirty_receipts.push(AgentServiceCommandReceipt::new(
            "unexpected_command",
            AgentServiceCommandStatus::Applied,
            "rogue executor output",
        ));
        let dirty_intake = dirty_dispatch.intake_receipts(dirty_receipts);
        let recorder = AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistoryRecorder::new();
        let first = recorder.record_intake(
            AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory::new(),
            &clean_intake,
            AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy::default(),
        );
        let second = recorder.record_intake(
            first.history,
            &dirty_intake,
            AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy::default(),
        );

        assert_eq!(
            second.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(second.dashboard.total_intakes, 2);
        assert_eq!(second.dashboard.clean_intakes, 1);
        assert_eq!(second.dashboard.dirty_intakes, 1);
        assert_eq!(second.dashboard.rejected_receipt_count, 1);
        assert_eq!(second.dashboard.blocked_reason_count, 1);
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
                "service_receipt_rejected=unexpected_command:receipt_command_unexpected_or_duplicate"
            ]
        );
        assert!(
            second
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "runtime_service_receipt_intake_rejected_receipts=1>0" })
        );
    }

    #[test]
    fn runtime_service_receipt_intake_history_repairs_non_executable_intake() {
        let input = AgentClosedLoopRuntimeTurnInput::new(
            history(),
            AgentTaskQueue::new(),
            budget(),
            AgentCycleEvidence::default(),
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);
        let mut memory = FakeMemory::default();
        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-intake-history-blocked",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        );
        let request = AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(business_turn);
        let dispatch = AgentClosedLoopRuntimeServiceRequest {
            command_request: request,
            prior_history: history(),
            telemetry: Vec::new(),
        }
        .into_dispatch();
        let intake = dispatch.intake_receipts(vec![AgentServiceCommandReceipt::new(
            "unexpected_command",
            AgentServiceCommandStatus::Applied,
            "blocked executor output",
        )]);

        let health =
            AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory::from_summaries(vec![
                intake.summary(),
            ])
            .health(AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Repair);
        assert_eq!(health.dashboard.non_executable_intakes, 1);
        assert_eq!(health.dashboard.rejected_receipt_count, 1);
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| { reason == "runtime_service_receipt_intake_non_executable=1>0" })
        );
    }

    #[test]
    fn runtime_service_dispatch_continuation_uses_closed_outcome_input() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-continuation-clean",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );
        let receipts = dispatch
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let dispatch_outcome = dispatch.close_with_intake(
            receipts,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default())
                .with_max_parallel_tasks(4),
        );
        let expected_history_len = dispatch_outcome
            .outcome
            .as_ref()
            .unwrap()
            .next_runtime_input()
            .history
            .len();

        let continuation = AgentClosedLoopRuntimeServiceDispatchContinuationPlanner::new().plan(
            dispatch_outcome,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        assert!(continuation.has_closed_outcome());
        assert_eq!(
            continuation.next_runtime_input.history.len(),
            expected_history_len
        );
        assert_eq!(continuation.next_runtime_input.max_parallel_tasks, 4);
        assert_eq!(
            continuation.next_queue().len(),
            continuation
                .dispatch_outcome
                .outcome
                .as_ref()
                .unwrap()
                .next_queue()
                .len()
        );
        assert!(
            continuation
                .telemetry
                .iter()
                .any(|line| line == "service_dispatch_continuation_outcome=true")
        );
    }

    #[test]
    fn runtime_service_dispatch_continuation_history_empty_is_watch() {
        let health = AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistory::new()
            .health(AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["runtime_service_dispatch_continuation_history_empty"]
        );
        assert_eq!(health.dashboard.total_continuations, 0);
    }

    #[test]
    fn runtime_service_dispatch_continuation_history_marks_clean_continuation_stable() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-continuation-history-clean",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );
        let receipts = dispatch
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let dispatch_outcome = dispatch.close_with_intake(
            receipts,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );
        let continuation = AgentClosedLoopRuntimeServiceDispatchContinuationPlanner::new().plan(
            dispatch_outcome,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        let record = AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistoryRecorder::new()
            .record_continuation(
                AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistory::new(),
                &continuation,
                AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy::default(),
            );

        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.health.is_stable());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.total_continuations, 1);
        assert_eq!(record.dashboard.closed_continuations, 1);
        assert_eq!(record.dashboard.blocked_continuations, 0);
        assert_eq!(record.dashboard.intake_dirty_continuations, 0);
        assert_eq!(record.dashboard.repair_task_count, 0);
        assert_eq!(record.dashboard.latest_outcome_closed, Some(true));
        assert_eq!(record.dashboard.latest_intake_clean, Some(true));
        assert!(
            record.telemetry.iter().any(|line| {
                line == "service_dispatch_continuation_history_record_status=stable"
            })
        );
    }

    #[test]
    fn runtime_service_dispatch_continuation_history_watches_low_closed_rate_when_repair_allowed() {
        let clean = AgentClosedLoopRuntimeServiceDispatchContinuationSummary {
            outcome_closed: true,
            intake_clean: true,
            repair_task_count: 0,
            health_status: AgentClosedLoopExecutionHealthStatus::Stable,
            next_queue_tasks: 1,
            immediate_ready_tasks: 1,
            history_runs: 2,
            telemetry: Vec::new(),
        };
        let blocked = AgentClosedLoopRuntimeServiceDispatchContinuationSummary {
            outcome_closed: false,
            intake_clean: true,
            repair_task_count: 0,
            health_status: AgentClosedLoopExecutionHealthStatus::Stable,
            next_queue_tasks: 1,
            immediate_ready_tasks: 0,
            history_runs: 2,
            telemetry: Vec::new(),
        };
        let policy = AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy {
            maximum_blocked_continuations: usize::MAX,
            maximum_repair_tasks: usize::MAX,
            maximum_repair_health_continuations: usize::MAX,
            ..AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy::default()
        };

        let health =
            AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistory::from_summaries(vec![
                clean, blocked,
            ])
            .health(policy);

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.dashboard.closed_rate, 0.5);
        assert_eq!(health.dashboard.intake_clean_rate, 1.0);
        assert_eq!(health.dashboard.total_next_queue_tasks, 2);
        assert_eq!(health.dashboard.total_immediate_ready_tasks, 1);
        assert_eq!(
            health.reasons,
            vec!["runtime_service_dispatch_continuation_closed_rate=0.500<0.67"]
        );
    }

    #[test]
    fn runtime_service_dispatch_continuation_history_repairs_blocked_continuation_and_preserves_order()
     {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let clean_input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-continuation-history-clean-prior",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let clean_dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            clean_input,
            &mut engine,
            &mut memory,
        );
        let clean_receipts = clean_dispatch
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let clean_outcome = clean_dispatch.close_with_intake(
            clean_receipts,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );
        let clean_continuation = AgentClosedLoopRuntimeServiceDispatchContinuationPlanner::new()
            .plan(
                clean_outcome,
                AgentClosedLoopRuntimeContinuationInput::new(
                    budget(),
                    AgentCycleEvidence::default(),
                ),
            );
        let blocked_input = AgentClosedLoopRuntimeServiceRequestInput::new(
            AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-continuation-history-blocked",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let blocked_dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            blocked_input,
            &mut engine,
            &mut memory,
        );
        let blocked_outcome = blocked_dispatch.close_with_intake(
            vec![AgentServiceCommandReceipt::new(
                "hold_business_loop",
                AgentServiceCommandStatus::Applied,
                "should not have run",
            )],
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );
        let blocked_continuation = AgentClosedLoopRuntimeServiceDispatchContinuationPlanner::new()
            .plan(
                blocked_outcome,
                AgentClosedLoopRuntimeContinuationInput::new(
                    budget(),
                    AgentCycleEvidence::default(),
                ),
            );
        let recorder =
            AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistoryRecorder::new();
        let first = recorder.record_continuation(
            AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistory::new(),
            &clean_continuation,
            AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy::default(),
        );
        let second = recorder.record_continuation(
            first.history,
            &blocked_continuation,
            AgentClosedLoopRuntimeServiceDispatchContinuationHealthPolicy::default(),
        );

        assert_eq!(
            second.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(second.dashboard.total_continuations, 2);
        assert_eq!(second.dashboard.closed_continuations, 1);
        assert_eq!(second.dashboard.blocked_continuations, 1);
        assert_eq!(second.dashboard.intake_dirty_continuations, 1);
        assert_eq!(second.dashboard.repair_task_count, 1);
        assert_eq!(second.dashboard.latest_outcome_closed, Some(false));
        assert_eq!(second.dashboard.latest_intake_clean, Some(false));
        assert_eq!(
            second
                .history
                .summaries()
                .iter()
                .map(|summary| summary.outcome_closed)
                .collect::<Vec<_>>(),
            vec![true, false]
        );
        assert_eq!(
            second
                .history
                .summaries()
                .iter()
                .map(|summary| summary.repair_task_count)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert!(
            second
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "runtime_service_dispatch_continuation_blocked=1>0" })
        );
        assert!(
            blocked_continuation.next_queue().task_ids()
                == vec!["service-intake-run-1-0-next_queue_empty"]
        );
    }

    #[test]
    fn runtime_service_request_runner_skips_idle_without_engine_or_memory_calls() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-request-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );

        let request =
            AgentClosedLoopRuntimeServiceRequestRunner::new().run(input, &mut engine, &mut memory);

        assert_eq!(engine.calls, 0);
        assert!(memory.submitted.is_empty());
        assert!(!request.has_commands());
        assert_eq!(request.skipped_reasons(), &["next_queue_empty".to_owned()]);
        assert_eq!(request.prior_history.len(), history().len());
        assert!(
            request
                .telemetry
                .iter()
                .any(|line| line == "service_request_ready=false")
        );
        assert!(
            request
                .telemetry
                .iter()
                .any(|line| line == "service_request_skipped=next_queue_empty")
        );
    }

    #[test]
    fn runtime_service_dispatch_blocks_idle_request_before_command_execution() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );

        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );

        assert!(!dispatch.is_executable());
        assert!(dispatch.command_plan().is_none());
        assert_eq!(
            dispatch.command_gate.blocked_reasons,
            vec!["next_queue_empty".to_owned()]
        );
        assert!(
            dispatch
                .telemetry
                .iter()
                .any(|line| line == "service_dispatch_executable=false")
        );
    }

    #[test]
    fn runtime_service_dispatch_summary_preserves_blocked_dispatch_reasons() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-summary-blocked",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );

        let summary = dispatch.summary();

        assert!(!summary.executable);
        assert_eq!(summary.command_count, 0);
        assert!(!summary.command_gate_allowed);
        assert_eq!(summary.side_effect_gate_count, 0);
        assert_eq!(summary.blocked_side_effect_gate_count, 0);
        assert!(summary.command_kinds.is_empty());
        assert_eq!(summary.blocked_reasons, vec!["next_queue_empty"]);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "service_dispatch_summary_blocked=next_queue_empty")
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "service_dispatch_summary_command_gate_allowed=false" })
        );
    }

    #[test]
    fn runtime_service_dispatch_intake_rejects_receipts_for_blocked_dispatch() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-intake-blocked",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );
        let receipts = vec![AgentServiceCommandReceipt::new(
            "hold_business_loop",
            AgentServiceCommandStatus::Applied,
            "should not have run",
        )];

        let dispatch_outcome = dispatch.close_with_intake(
            receipts,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        assert!(!dispatch_outcome.has_outcome());
        assert!(!dispatch_outcome.intake.can_close_outcome());
        assert_eq!(dispatch_outcome.intake.accepted_receipts.len(), 0);
        assert_eq!(dispatch_outcome.intake.rejected_receipts.len(), 1);
        assert_eq!(
            dispatch_outcome.intake.rejected_receipts[0].reason,
            "service_dispatch_not_executable"
        );
        assert_eq!(
            dispatch_outcome.blocked_reasons(),
            ["next_queue_empty".to_owned()].as_slice()
        );
        assert_eq!(
            dispatch_outcome.repair_queue().task_ids(),
            vec!["service-intake-run-1-0-next_queue_empty"]
        );
        assert_eq!(
            dispatch_outcome.repair_plan.tasks[0].role,
            AgentRole::Reviewer
        );
        assert!(
            dispatch_outcome
                .telemetry
                .iter()
                .any(|line| line == "service_intake_repair_tasks=1")
        );
    }

    #[test]
    fn runtime_service_dispatch_continuation_routes_blocked_intake_repairs() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-dispatch-continuation-blocked",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );
        let prior_history_len = dispatch.request.prior_history.len();
        let dispatch_outcome = dispatch.close_with_intake(
            vec![AgentServiceCommandReceipt::new(
                "hold_business_loop",
                AgentServiceCommandStatus::Applied,
                "should not have run",
            )],
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );
        let repair_budget =
            BudgetLedger::new().with_budget(AgentRole::Reviewer, AgentBudget::new(16, 1, 1));

        let continuation = AgentClosedLoopRuntimeServiceDispatchContinuationPlanner::new().plan(
            dispatch_outcome,
            AgentClosedLoopRuntimeContinuationInput::new(
                repair_budget.clone(),
                AgentCycleEvidence::default(),
            )
            .with_max_parallel_tasks(2),
        );

        assert!(!continuation.has_closed_outcome());
        assert_eq!(
            continuation.next_runtime_input.history.len(),
            prior_history_len
        );
        assert_eq!(
            continuation.next_queue().task_ids(),
            vec!["service-intake-run-1-0-next_queue_empty"]
        );
        assert_eq!(continuation.next_runtime_input.budget_ledger, repair_budget);
        assert_eq!(continuation.next_runtime_input.max_parallel_tasks, 2);
        assert!(
            continuation
                .telemetry
                .iter()
                .any(|line| line == "service_dispatch_continuation_outcome=false")
        );
    }

    #[test]
    fn runtime_service_runner_closes_clean_receipts_into_next_runtime_input() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let receipt_plan =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let receipts = receipt_plan
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let input = AgentClosedLoopRuntimeServiceRunInput::new(
            AgentClosedLoopRuntimeServiceRequestInput::new(
                clean_runtime_input(),
                AgentClosedLoopRuntimeBusinessInput::new(
                    "run-service-run-clean",
                    crate::ledger::AgentCycleLedger::new(),
                    report_evidence(),
                ),
            ),
            receipts,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default())
                .with_max_parallel_tasks(3),
        );

        let run = AgentClosedLoopRuntimeServiceRunner::new().run(input, &mut engine, &mut memory);

        assert_eq!(engine.calls, 1);
        assert_eq!(memory.submitted.len(), 1);
        assert!(run.has_closed_outcome());
        assert!(run.summary.outcome_closed);
        assert!(run.summary.intake_clean);
        assert_eq!(run.summary.repair_task_count, 0);
        assert_eq!(run.summary.health_status.as_str(), "stable");
        assert!(run.run_summary.dispatch_executable);
        assert_eq!(
            run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert!(run.run_summary.outcome_closed);
        assert!(run.run_summary.intake_clean);
        assert_eq!(
            run.run_summary.command_count,
            receipt_plan.command_kinds().len()
        );
        assert!(run.run_summary.command_gate_allowed);
        assert_eq!(
            run.run_summary.side_effect_gate_count,
            run.run_summary.command_count
        );
        assert_eq!(run.run_summary.blocked_side_effect_gate_count, 0);
        assert_eq!(run.run_summary.command_kinds[0], "promote_adaptive_state");
        assert!(run.run_summary.gate_blocked_reasons.is_empty());
        assert!(run.run_summary.intake_blocked_reasons.is_empty());
        assert_eq!(run.next_runtime_input().history.len(), history().len() + 1);
        assert_eq!(run.next_runtime_input().max_parallel_tasks, 3);
        assert_eq!(
            run.summary.history_runs,
            run.next_runtime_input().history.len()
        );
        assert!(
            run.telemetry
                .iter()
                .any(|line| line == "service_run_outcome_closed=true")
        );
        assert!(
            run.run_summary
                .telemetry
                .iter()
                .any(|line| line == "service_run_summary_dispatch_executable=true")
        );
        assert!(
            run.run_summary
                .telemetry
                .iter()
                .any(|line| { line == "service_run_summary_command_gate_allowed=true" })
        );
        assert!(
            run.run_summary
                .telemetry
                .iter()
                .any(|line| line == "service_run_summary_status=closed")
        );
    }

    #[test]
    fn runtime_service_runner_routes_blocked_dispatch_to_intake_repair_queue() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRunInput::new(
            AgentClosedLoopRuntimeServiceRequestInput::new(
                AgentClosedLoopRuntimeTurnInput::new(
                    history(),
                    AgentTaskQueue::new(),
                    budget(),
                    AgentCycleEvidence::default(),
                ),
                AgentClosedLoopRuntimeBusinessInput::new(
                    "run-service-run-blocked",
                    crate::ledger::AgentCycleLedger::new(),
                    report_evidence(),
                ),
            ),
            vec![AgentServiceCommandReceipt::new(
                "hold_business_loop",
                AgentServiceCommandStatus::Applied,
                "should not have run",
            )],
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        let run = AgentClosedLoopRuntimeServiceRunner::new().run(input, &mut engine, &mut memory);

        assert_eq!(engine.calls, 0);
        assert!(memory.submitted.is_empty());
        assert!(!run.has_closed_outcome());
        assert!(!run.summary.outcome_closed);
        assert!(!run.summary.intake_clean);
        assert_eq!(run.summary.repair_task_count, 1);
        assert!(!run.run_summary.dispatch_executable);
        assert_eq!(
            run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::DispatchBlocked
        );
        assert!(!run.run_summary.outcome_closed);
        assert!(!run.run_summary.intake_clean);
        assert_eq!(run.run_summary.command_count, 0);
        assert!(!run.run_summary.command_gate_allowed);
        assert_eq!(run.run_summary.side_effect_gate_count, 0);
        assert_eq!(run.run_summary.blocked_side_effect_gate_count, 0);
        assert_eq!(
            run.run_summary.gate_blocked_reasons,
            vec!["next_queue_empty"]
        );
        assert_eq!(
            run.run_summary.intake_blocked_reasons,
            vec!["next_queue_empty"]
        );
        assert_eq!(run.next_runtime_input().history.len(), history().len());
        assert_eq!(
            run.next_queue().task_ids(),
            vec!["service-intake-run-1-0-next_queue_empty"]
        );
        assert!(
            run.telemetry
                .iter()
                .any(|line| line == "service_run_repair_tasks=1")
        );
        assert!(
            run.run_summary
                .telemetry
                .iter()
                .any(|line| line == "service_run_summary_intake_blocked=next_queue_empty")
        );
        assert!(
            run.run_summary
                .telemetry
                .iter()
                .any(|line| line == "service_run_summary_status=dispatch_blocked")
        );
        assert!(
            run.run_summary
                .telemetry
                .iter()
                .any(|line| { line == "service_run_summary_command_gate_allowed=false" })
        );
    }

    #[test]
    fn runtime_service_runner_marks_unexpected_receipt_as_intake_blocked() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let receipt_plan =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let mut receipts = receipt_plan
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        receipts.push(AgentServiceCommandReceipt::new(
            "unexpected_command",
            AgentServiceCommandStatus::Applied,
            "rogue executor output",
        ));
        let input = AgentClosedLoopRuntimeServiceRunInput::new(
            AgentClosedLoopRuntimeServiceRequestInput::new(
                clean_runtime_input(),
                AgentClosedLoopRuntimeBusinessInput::new(
                    "run-service-run-intake-blocked",
                    crate::ledger::AgentCycleLedger::new(),
                    report_evidence(),
                ),
            ),
            receipts,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        let run = AgentClosedLoopRuntimeServiceRunner::new().run(input, &mut engine, &mut memory);

        assert_eq!(engine.calls, 1);
        assert!(run.run_summary.dispatch_executable);
        assert_eq!(
            run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
        );
        assert!(!run.run_summary.outcome_closed);
        assert!(!run.run_summary.intake_clean);
        assert_eq!(run.run_summary.repair_task_count, 1);
        assert_eq!(
            run.run_summary.intake_blocked_reasons,
            vec![
                "service_receipt_rejected=unexpected_command:receipt_command_unexpected_or_duplicate"
                    .to_owned()
            ]
        );
        assert!(
            run.run_summary
                .telemetry
                .iter()
                .any(|line| line == "service_run_summary_status=intake_blocked")
        );
    }

    #[test]
    fn runtime_service_run_history_summarizes_closed_and_blocked_runs() {
        let mut clean_engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut clean_memory = FakeMemory::default();
        let receipt_plan =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let clean_receipts = receipt_plan
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let clean_run = AgentClosedLoopRuntimeServiceRunner::new().run(
            AgentClosedLoopRuntimeServiceRunInput::new(
                AgentClosedLoopRuntimeServiceRequestInput::new(
                    clean_runtime_input(),
                    AgentClosedLoopRuntimeBusinessInput::new(
                        "run-service-history-clean",
                        crate::ledger::AgentCycleLedger::new(),
                        report_evidence(),
                    ),
                ),
                clean_receipts.clone(),
                AgentClosedLoopRuntimeContinuationInput::new(
                    budget(),
                    AgentCycleEvidence::default(),
                ),
            ),
            &mut clean_engine,
            &mut clean_memory,
        );

        let mut blocked_engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut blocked_memory = FakeMemory::default();
        let dispatch_blocked_run = AgentClosedLoopRuntimeServiceRunner::new().run(
            AgentClosedLoopRuntimeServiceRunInput::new(
                AgentClosedLoopRuntimeServiceRequestInput::new(
                    AgentClosedLoopRuntimeTurnInput::new(
                        history(),
                        AgentTaskQueue::new(),
                        budget(),
                        AgentCycleEvidence::default(),
                    ),
                    AgentClosedLoopRuntimeBusinessInput::new(
                        "run-service-history-dispatch-blocked",
                        crate::ledger::AgentCycleLedger::new(),
                        report_evidence(),
                    ),
                ),
                vec![AgentServiceCommandReceipt::new(
                    "hold_business_loop",
                    AgentServiceCommandStatus::Applied,
                    "should not have run",
                )],
                AgentClosedLoopRuntimeContinuationInput::new(
                    budget(),
                    AgentCycleEvidence::default(),
                ),
            ),
            &mut blocked_engine,
            &mut blocked_memory,
        );

        let mut intake_engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut intake_memory = FakeMemory::default();
        let mut intake_receipts = clean_receipts;
        intake_receipts.push(AgentServiceCommandReceipt::new(
            "unexpected_command",
            AgentServiceCommandStatus::Applied,
            "rogue executor output",
        ));
        let intake_blocked_run = AgentClosedLoopRuntimeServiceRunner::new().run(
            AgentClosedLoopRuntimeServiceRunInput::new(
                AgentClosedLoopRuntimeServiceRequestInput::new(
                    clean_runtime_input(),
                    AgentClosedLoopRuntimeBusinessInput::new(
                        "run-service-history-intake-blocked",
                        crate::ledger::AgentCycleLedger::new(),
                        report_evidence(),
                    ),
                ),
                intake_receipts,
                AgentClosedLoopRuntimeContinuationInput::new(
                    budget(),
                    AgentCycleEvidence::default(),
                ),
            ),
            &mut intake_engine,
            &mut intake_memory,
        );
        let history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_run.run_summary.clone(),
            dispatch_blocked_run.run_summary.clone(),
            intake_blocked_run.run_summary.clone(),
        ]);

        let dashboard = history.dashboard();

        assert_eq!(history.len(), 3);
        assert_eq!(
            history.latest().unwrap().status,
            AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
        );
        assert_eq!(dashboard.total_runs, 3);
        assert_eq!(dashboard.closed_runs, 1);
        assert_eq!(dashboard.dispatch_blocked_runs, 1);
        assert_eq!(dashboard.intake_blocked_runs, 1);
        assert_eq!(dashboard.blocked_runs, 2);
        assert!((dashboard.closed_rate - 0.333).abs() < 0.01);
        assert!((dashboard.blocked_rate - 0.666).abs() < 0.01);
        assert_eq!(
            dashboard.command_count,
            clean_run.run_summary.command_count + intake_blocked_run.run_summary.command_count
        );
        assert_eq!(dashboard.repair_task_count, 2);
        assert_eq!(
            dashboard.latest_status,
            Some(AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked)
        );
        assert!(
            dashboard
                .latest_blocked_reasons
                .iter()
                .any(|reason| reason.contains("unexpected_command"))
        );
        assert!(!dashboard.is_clean());
    }

    #[test]
    fn runtime_service_run_history_handles_empty_dashboard() {
        let history = AgentClosedLoopRuntimeServiceRunHistory::new();

        let dashboard = history.dashboard();

        assert!(history.is_empty());
        assert!(history.latest().is_none());
        assert!(dashboard.is_empty());
        assert!(dashboard.is_clean());
        assert_eq!(dashboard.total_runs, 0);
        assert_eq!(dashboard.closed_rate, 0.0);
        assert_eq!(dashboard.blocked_rate, 0.0);
        assert_eq!(dashboard.latest_status, None);
        assert!(dashboard.latest_blocked_reasons.is_empty());
    }

    #[test]
    fn runtime_service_run_history_recorder_appends_clean_attempt() {
        let prior_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-record-prior-clean"),
        ]);
        let run = clean_service_run("run-service-record-clean");

        let record = AgentClosedLoopRuntimeServiceRunHistoryRecorder::new().record(
            prior_history,
            &run,
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        assert_eq!(record.attempts(), 2);
        assert_eq!(
            record.latest().status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert_eq!(record.appended_summary, run.run_summary);
        assert_eq!(record.dashboard.closed_runs, 2);
        assert_eq!(record.dashboard.blocked_runs, 0);
        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.health.is_stable());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "service_run_history_record_status=closed")
        );
    }

    #[test]
    fn runtime_service_run_history_recorder_surfaces_intake_repair_health() {
        let prior_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-record-prior-clean"),
        ]);
        let run = intake_blocked_service_run("run-service-record-intake");

        let record = AgentClosedLoopRuntimeServiceRunHistoryRecorder::new().record(
            prior_history,
            &run,
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        assert_eq!(record.attempts(), 2);
        assert_eq!(
            record.latest().status,
            AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
        );
        assert_eq!(record.dashboard.intake_blocked_runs, 1);
        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(
            record
                .latest()
                .blocked_reasons()
                .iter()
                .any(|reason| reason.contains("unexpected_command"))
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "service_run_history_record_health=repair")
        );
        assert!(
            record.telemetry.iter().any(|line| line
                == "service_run_history_record_reason=service_run_intake_blocked_runs=1>0")
        );
    }

    #[test]
    fn runtime_service_run_health_marks_empty_history_watch() {
        let health = AgentClosedLoopRuntimeServiceRunHistory::new()
            .health(AgentClosedLoopRuntimeServiceRunHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.reasons, vec!["service_run_history_empty"]);
        assert!(health.dashboard.is_empty());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(!health.is_stable());
    }

    #[test]
    fn runtime_service_run_health_marks_clean_history_stable() {
        let history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-health-clean-1"),
            clean_service_run_summary("run-service-health-clean-2"),
        ]);

        let health = history.health(AgentClosedLoopRuntimeServiceRunHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert_eq!(health.dashboard.total_runs, 2);
        assert_eq!(health.dashboard.closed_runs, 2);
        assert!(health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn runtime_service_run_health_repairs_intake_drift() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let receipt_plan =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let mut receipts = receipt_plan
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        receipts.push(AgentServiceCommandReceipt::new(
            "unexpected_command",
            AgentServiceCommandStatus::Applied,
            "rogue executor output",
        ));
        let intake_blocked = AgentClosedLoopRuntimeServiceRunner::new()
            .run(
                AgentClosedLoopRuntimeServiceRunInput::new(
                    AgentClosedLoopRuntimeServiceRequestInput::new(
                        clean_runtime_input(),
                        AgentClosedLoopRuntimeBusinessInput::new(
                            "run-service-health-intake",
                            crate::ledger::AgentCycleLedger::new(),
                            report_evidence(),
                        ),
                    ),
                    receipts,
                    AgentClosedLoopRuntimeContinuationInput::new(
                        budget(),
                        AgentCycleEvidence::default(),
                    ),
                ),
                &mut engine,
                &mut memory,
            )
            .run_summary;
        let history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-health-clean"),
            intake_blocked,
        ]);

        let health = history.health(AgentClosedLoopRuntimeServiceRunHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Repair);
        assert!(!health.allows_service_advance());
        assert!(health.requires_repair_first());
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "service_run_intake_blocked_runs=1>0")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "service_run_repair_tasks=1>0")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason.starts_with("service_run_latest_blocked="))
        );
        assert!(!health.is_stable());
    }

    #[test]
    fn runtime_service_preflight_continues_when_execution_and_service_run_are_stable() {
        let service_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-preflight-clean-1"),
            clean_service_run_summary("run-service-preflight-clean-2"),
        ]);

        let preflight = AgentClosedLoopRuntimeServicePreflightPlanner::new().plan(
            history(),
            service_history,
            queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        assert_eq!(preflight.mode, AgentClosedLoopNextTurnMode::Continue);
        assert!(preflight.can_schedule());
        assert!(preflight.allows_adaptive_evolution());
        assert!(!preflight.requires_repair_first());
        assert!(preflight.reasons.is_empty());
        assert!(
            preflight
                .telemetry
                .iter()
                .any(|line| line == "service_preflight_mode=continue")
        );

        let follow_up = preflight.follow_up_plan();

        assert!(follow_up.tasks.is_empty());
        assert_eq!(follow_up.next_queue.task_ids(), queue().task_ids());
        assert!(
            follow_up
                .telemetry
                .iter()
                .any(|line| line == "service_preflight_follow_up_tasks=0")
        );
    }

    #[test]
    fn runtime_service_preflight_projects_stable_side_effect_admission() {
        let service_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-preflight-admission-clean-1"),
            clean_service_run_summary("run-service-preflight-admission-clean-2"),
        ]);
        let preflight = AgentClosedLoopRuntimeServicePreflightPlanner::new().plan(
            history(),
            service_history,
            queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        let admission = preflight.side_effect_admission();

        assert_eq!(admission.mode, AgentClosedLoopNextTurnMode::Continue);
        assert_eq!(
            admission.health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(admission.can_dispatch_service_commands);
        assert!(admission.can_promote_memory_note);
        assert!(admission.can_admit_adaptive_evolution);
        assert!(!admission.requires_repair_first);
        assert!(
            admission
                .service_dispatch_gate()
                .is_some_and(|gate| gate.allowed)
        );
        assert!(
            admission
                .memory_note_gate()
                .is_some_and(|gate| gate.allowed)
        );
        assert!(
            admission
                .adaptive_state_gate()
                .is_some_and(|gate| gate.allowed)
        );
        assert!(
            admission
                .telemetry
                .iter()
                .any(|line| { line == "service_preflight_side_effect_admission_mode=continue" })
        );
    }

    #[test]
    fn runtime_service_preflight_observes_empty_service_run_history() {
        let preflight = AgentClosedLoopRuntimeServicePreflightPlanner::new().plan(
            history(),
            AgentClosedLoopRuntimeServiceRunHistory::new(),
            queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        assert_eq!(preflight.mode, AgentClosedLoopNextTurnMode::Observe);
        assert!(preflight.can_schedule());
        assert!(!preflight.allows_adaptive_evolution());
        assert_eq!(preflight.reasons, vec!["service_run_history_empty"]);
        assert!(
            preflight
                .telemetry
                .iter()
                .any(|line| line == "service_preflight_service_run_health=watch")
        );

        let follow_up = preflight.follow_up_plan();

        assert_eq!(follow_up.tasks.len(), 1);
        assert_eq!(follow_up.tasks[0].lane, "service-preflight");
        assert_eq!(follow_up.tasks[0].role, AgentRole::Tester);
        assert_eq!(follow_up.tasks[0].priority, 8);
        assert_eq!(follow_up.next_queue.len(), queue().len() + 1);
        assert_eq!(
            follow_up.immediate_ready_task_ids(),
            vec!["service-preflight-observe-0-history_empty".to_owned()]
        );
        assert!(
            follow_up
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "service-preflight-observe-0-history_empty")
        );
        let mut next_queue = follow_up.next_queue.clone();
        let follow_up_wave = next_queue.drain_ready(&BTreeSet::new());
        assert_eq!(
            follow_up_wave
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["service-preflight-observe-0-history_empty"]
        );
        let completed_follow_up = follow_up_wave
            .iter()
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();
        let business_wave_ids = next_queue
            .ready_tasks(&completed_follow_up)
            .iter()
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();
        let requested_ready_ids = queue()
            .immediate_ready_tasks()
            .into_iter()
            .map(|task| task.id)
            .collect::<BTreeSet<_>>();
        assert_eq!(business_wave_ids, requested_ready_ids);
    }

    #[test]
    fn runtime_service_preflight_projects_observe_side_effect_admission() {
        let preflight = AgentClosedLoopRuntimeServicePreflightPlanner::new().plan(
            history(),
            AgentClosedLoopRuntimeServiceRunHistory::new(),
            queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        let admission = preflight.side_effect_admission();

        assert_eq!(admission.mode, AgentClosedLoopNextTurnMode::Observe);
        assert_eq!(
            admission.health_status,
            AgentClosedLoopExecutionHealthStatus::Watch
        );
        assert!(admission.can_dispatch_service_commands);
        assert!(!admission.can_promote_memory_note);
        assert!(!admission.can_admit_adaptive_evolution);
        assert!(!admission.requires_repair_first);
        assert!(
            admission
                .service_dispatch_gate()
                .is_some_and(|gate| gate.allowed)
        );
        assert!(
            admission
                .memory_note_gate()
                .is_some_and(|gate| !gate.allowed)
        );
        assert!(
            admission
                .reasons
                .iter()
                .any(|reason| reason == "service_preflight_admission_observe")
        );
    }

    #[test]
    fn runtime_service_preflight_repairs_service_run_drift_before_scheduling() {
        let service_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-preflight-clean"),
            intake_blocked_service_run_summary("run-service-preflight-intake"),
        ]);

        let preflight = AgentClosedLoopRuntimeServicePreflightPlanner::new().plan(
            history(),
            service_history,
            queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        assert_eq!(preflight.mode, AgentClosedLoopNextTurnMode::Repair);
        assert!(preflight.can_schedule());
        assert!(!preflight.allows_adaptive_evolution());
        assert!(preflight.requires_repair_first());
        assert!(
            preflight
                .reasons
                .iter()
                .any(|reason| reason == "service_run_health_requires_repair")
        );
        assert!(
            preflight
                .reasons
                .iter()
                .any(|reason| reason == "service_run_intake_blocked_runs=1>0")
        );
        assert!(
            preflight
                .telemetry
                .iter()
                .any(|line| line == "service_preflight_mode=repair")
        );

        let follow_up = preflight.follow_up_plan();

        assert!(!follow_up.tasks.is_empty());
        assert!(
            follow_up
                .tasks
                .iter()
                .all(|task| task.lane == "service-preflight")
        );
        assert!(follow_up.tasks.iter().all(|task| task.priority == 10));
        assert!(
            follow_up
                .tasks
                .iter()
                .any(|task| task.role == AgentRole::Aggregator)
        );
        assert_eq!(
            follow_up.next_queue.len(),
            queue().len() + follow_up.tasks.len()
        );
        assert!(
            follow_up
                .telemetry
                .iter()
                .any(|line| line == "service_preflight_follow_up_mode=repair")
        );
    }

    #[test]
    fn runtime_service_preflight_projects_repair_side_effect_admission() {
        let service_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-preflight-admission-clean"),
            intake_blocked_service_run_summary("run-service-preflight-admission-intake"),
        ]);
        let preflight = AgentClosedLoopRuntimeServicePreflightPlanner::new().plan(
            history(),
            service_history,
            queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        let admission = preflight.side_effect_admission();

        assert_eq!(admission.mode, AgentClosedLoopNextTurnMode::Repair);
        assert_eq!(
            admission.health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!admission.can_dispatch_service_commands);
        assert!(!admission.can_promote_memory_note);
        assert!(!admission.can_admit_adaptive_evolution);
        assert!(admission.requires_repair_first);
        assert!(
            admission
                .service_dispatch_gate()
                .is_some_and(|gate| !gate.allowed)
        );
        assert!(
            admission
                .memory_note_gate()
                .is_some_and(|gate| !gate.allowed)
        );
        assert!(
            admission
                .adaptive_state_gate()
                .is_some_and(|gate| !gate.allowed)
        );
        assert!(
            admission
                .reasons
                .iter()
                .any(|reason| reason == "service_preflight_admission_repair_first")
        );
    }

    #[test]
    fn runtime_service_preflight_continuation_merges_observe_tasks_into_runtime_input() {
        let preflight = AgentClosedLoopRuntimeServicePreflightPlanner::new().plan(
            history(),
            AgentClosedLoopRuntimeServiceRunHistory::new(),
            queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );
        let next_budget = BudgetLedger::new()
            .with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1))
            .with_budget(AgentRole::Tester, AgentBudget::new(16, 1, 1));

        let continuation = AgentClosedLoopRuntimeServicePreflightContinuationPlanner::new().plan(
            preflight,
            AgentClosedLoopRuntimeContinuationInput::new(
                next_budget.clone(),
                AgentCycleEvidence::default(),
            )
            .with_max_parallel_tasks(2),
        );

        assert_eq!(
            continuation.preflight.mode,
            AgentClosedLoopNextTurnMode::Observe
        );
        assert_eq!(continuation.follow_up_plan.tasks.len(), 1);
        assert_eq!(
            continuation.next_runtime_input.history.len(),
            history().len()
        );
        assert_eq!(continuation.next_runtime_input.budget_ledger, next_budget);
        assert_eq!(continuation.next_runtime_input.max_parallel_tasks, 2);
        assert_eq!(
            continuation.next_runtime_input.next_queue.len(),
            queue().len() + 1
        );
        assert_eq!(
            continuation.immediate_ready_task_ids(),
            vec!["service-preflight-observe-0-history_empty".to_owned()]
        );
        assert!(
            continuation
                .next_queue()
                .task_ids()
                .iter()
                .any(|id| id == "service-preflight-observe-0-history_empty")
        );
        let mut next_queue = continuation.next_runtime_input.next_queue.clone();
        let follow_up_wave = next_queue.drain_ready(&BTreeSet::new());
        assert_eq!(
            follow_up_wave
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["service-preflight-observe-0-history_empty"]
        );
        let completed_follow_up = follow_up_wave
            .iter()
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();
        let business_wave_ids = next_queue
            .ready_tasks(&completed_follow_up)
            .iter()
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();
        let requested_ready_ids = queue()
            .immediate_ready_tasks()
            .into_iter()
            .map(|task| task.id)
            .collect::<BTreeSet<_>>();
        assert_eq!(business_wave_ids, requested_ready_ids);
        assert!(
            continuation
                .telemetry
                .iter()
                .any(|line| line == "service_preflight_continuation_mode=observe")
        );
        assert!(
            continuation
                .telemetry
                .iter()
                .any(|line| line == "service_preflight_continuation_immediate_ready_tasks=1")
        );
    }

    #[test]
    fn runtime_service_preflight_continuation_preserves_repair_runtime_inputs() {
        let service_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-preflight-continuation-clean"),
            intake_blocked_service_run_summary("run-service-preflight-continuation-intake"),
        ]);
        let preflight = AgentClosedLoopRuntimeServicePreflightPlanner::new().plan(
            history(),
            service_history,
            queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );
        let next_budget = BudgetLedger::new()
            .with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1))
            .with_budget(AgentRole::Aggregator, AgentBudget::new(32, 2, 2));
        let evidence = AgentCycleEvidence {
            quality: 0.71,
            validation_passed: false,
            runtime_response_ok: false,
            ..AgentCycleEvidence::default()
        };

        let continuation = AgentClosedLoopRuntimeServicePreflightContinuationPlanner::new().plan(
            preflight,
            AgentClosedLoopRuntimeContinuationInput::new(next_budget.clone(), evidence.clone())
                .with_max_parallel_tasks(3),
        );

        assert_eq!(
            continuation.preflight.mode,
            AgentClosedLoopNextTurnMode::Repair
        );
        assert!(continuation.preflight.requires_repair_first());
        assert!(!continuation.preflight.allows_service_advance());
        assert!(!continuation.follow_up_plan.tasks.is_empty());
        assert_eq!(continuation.next_runtime_input.budget_ledger, next_budget);
        assert_eq!(continuation.next_runtime_input.evidence, evidence);
        assert_eq!(continuation.next_runtime_input.max_parallel_tasks, 3);
        assert_eq!(
            continuation.next_runtime_input.next_queue.len(),
            queue().len() + continuation.follow_up_plan.tasks.len()
        );
        assert_eq!(
            continuation.immediate_ready_task_ids().len(),
            continuation.follow_up_plan.tasks.len()
        );
        assert!(
            continuation
                .immediate_ready_task_ids()
                .iter()
                .all(|task_id| { task_id.starts_with("service-preflight-repair-") })
        );
        assert!(
            continuation
                .follow_up_plan
                .tasks
                .iter()
                .any(|task| task.role == AgentRole::Aggregator)
        );
        let mut next_queue = continuation.next_runtime_input.next_queue.clone();
        let repair_wave = next_queue.drain_ready(&BTreeSet::new());
        assert_eq!(repair_wave.len(), continuation.follow_up_plan.tasks.len());
        assert!(
            repair_wave
                .iter()
                .all(|task| task.id.starts_with("service-preflight-repair-"))
        );
        let completed_repairs = repair_wave
            .iter()
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();
        let business_wave_ids = next_queue
            .ready_tasks(&completed_repairs)
            .iter()
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();
        let requested_ready_ids = queue()
            .immediate_ready_tasks()
            .into_iter()
            .map(|task| task.id)
            .collect::<BTreeSet<_>>();
        assert_eq!(business_wave_ids, requested_ready_ids);
        assert!(
            continuation
                .telemetry
                .iter()
                .any(|line| line == "service_preflight_continuation_mode=repair")
        );
        assert!(continuation.telemetry.iter().any(|line| {
            line == &format!(
                "service_preflight_continuation_immediate_ready_tasks={}",
                continuation.follow_up_plan.tasks.len()
            )
        }));
    }

    #[test]
    fn runtime_service_loop_state_planner_packages_clean_next_runtime_input() {
        let service_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-loop-state-clean-1"),
            clean_service_run_summary("run-service-loop-state-clean-2"),
        ]);

        let state = AgentClosedLoopRuntimeServiceLoopStatePlanner::new().plan(
            history(),
            service_history.clone(),
            queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        assert_eq!(state.mode(), AgentClosedLoopNextTurnMode::Continue);
        assert!(state.can_schedule());
        assert!(state.allows_adaptive_evolution());
        assert!(!state.requires_repair_first());
        assert!(state.allows_service_advance());
        assert_eq!(state.execution_history.len(), history().len());
        assert_eq!(state.service_run_history.len(), service_history.len());
        assert_eq!(state.next_runtime_input().history.len(), history().len());
        assert_eq!(state.next_queue().task_ids(), queue().task_ids());
        assert!(state.preflight_continuation.follow_up_plan.tasks.is_empty());
        assert!(
            state
                .telemetry
                .iter()
                .any(|line| line == "service_loop_state_mode=continue")
        );

        let summary = state.summary();
        assert_eq!(summary.mode, AgentClosedLoopNextTurnMode::Continue);
        assert_eq!(
            summary.execution_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            summary.service_run_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(summary.can_schedule);
        assert_eq!(
            summary.side_effect_admission_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(summary.side_effect_dispatch_allowed);
        assert!(summary.memory_note_allowed);
        assert!(summary.allows_adaptive_evolution);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.side_effect_admission_reasons, 0);
        assert_eq!(summary.execution_history_runs, history().len());
        assert_eq!(summary.service_run_attempts, service_history.len());
        assert_eq!(summary.preflight_follow_up_tasks, 0);
        assert_eq!(summary.next_queue_tasks, queue().len());
        assert_eq!(
            summary.immediate_ready_tasks,
            state.next_queue().immediate_ready_tasks().len()
        );
        assert!(summary.reasons.is_empty());
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "service_loop_state_summary_mode=continue")
        );
    }

    #[test]
    fn runtime_service_loop_state_planner_prioritizes_preflight_repairs() {
        let service_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-loop-state-clean"),
            intake_blocked_service_run_summary("run-service-loop-state-intake"),
        ]);
        let next_budget = BudgetLedger::new()
            .with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1))
            .with_budget(AgentRole::Aggregator, AgentBudget::new(32, 2, 2));

        let state = AgentClosedLoopRuntimeServiceLoopStatePlanner::new().plan(
            history(),
            service_history,
            queue(),
            AgentClosedLoopExecutionHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
            AgentClosedLoopRuntimeContinuationInput::new(
                next_budget.clone(),
                AgentCycleEvidence::default(),
            )
            .with_max_parallel_tasks(4),
        );

        assert_eq!(state.mode(), AgentClosedLoopNextTurnMode::Repair);
        assert!(state.requires_repair_first());
        assert!(!state.allows_service_advance());
        assert!(!state.allows_adaptive_evolution());
        assert_eq!(state.next_runtime_input().budget_ledger, next_budget);
        assert_eq!(state.next_runtime_input().max_parallel_tasks, 4);
        assert_eq!(
            state.next_runtime_input().next_queue.len(),
            queue().len() + state.preflight_continuation.follow_up_plan.tasks.len()
        );
        assert!(
            state
                .preflight_continuation
                .follow_up_plan
                .tasks
                .iter()
                .any(|task| task.role == AgentRole::Aggregator)
        );
        assert!(
            state
                .telemetry
                .iter()
                .any(|line| line == "service_loop_state_mode=repair")
        );

        let summary = state.summary();
        assert_eq!(summary.mode, AgentClosedLoopNextTurnMode::Repair);
        assert_eq!(
            summary.service_run_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            summary.side_effect_admission_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!summary.side_effect_dispatch_allowed);
        assert!(!summary.memory_note_allowed);
        assert!(!summary.allows_adaptive_evolution);
        assert!(summary.requires_repair_first);
        assert!(summary.side_effect_admission_reasons > summary.reasons.len());
        assert_eq!(
            summary.preflight_follow_up_tasks,
            state.preflight_continuation.follow_up_plan.tasks.len()
        );
        assert_eq!(
            summary.next_queue_tasks,
            state.next_runtime_input().next_queue.len()
        );
        assert_eq!(
            summary.immediate_ready_tasks,
            state
                .next_runtime_input()
                .next_queue
                .immediate_ready_tasks()
                .len()
        );
        assert!(
            summary
                .reasons
                .iter()
                .any(|reason| reason == "service_run_health_requires_repair")
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "service_loop_state_summary_requires_repair_first=true")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "service_loop_state_summary_side_effect_dispatch_allowed=false"
        }));
    }

    #[test]
    fn runtime_service_loop_advance_records_clean_run_and_continues() {
        let prior_service_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-loop-advance-prior-clean"),
        ]);
        let run = clean_service_run("run-service-loop-advance-clean");
        let run_next_input = run.next_runtime_input().clone();

        let advance = AgentClosedLoopRuntimeServiceLoopAdvancePlanner::new().advance(
            prior_service_history,
            &run,
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        assert_eq!(advance.run_record.attempts(), 2);
        assert_eq!(
            advance.run_record.latest().status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert_eq!(advance.loop_state.service_run_history.len(), 2);
        assert_eq!(advance.loop_state.execution_history, run_next_input.history);
        assert_eq!(
            advance.next_runtime_input().budget_ledger,
            run_next_input.budget_ledger
        );
        assert_eq!(
            advance.next_runtime_input().max_parallel_tasks,
            run_next_input.max_parallel_tasks
        );
        assert_eq!(advance.mode(), AgentClosedLoopNextTurnMode::Continue);
        assert!(advance.summary.can_schedule);
        assert!(advance.summary.allows_adaptive_evolution);
        assert!(!advance.requires_repair_first());
        assert!(advance.allows_service_advance());
        assert!(
            advance
                .telemetry
                .iter()
                .any(|line| line == "service_loop_advance_mode=continue")
        );
    }

    #[test]
    fn runtime_service_loop_advance_records_intake_drift_as_repair_state() {
        let prior_service_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-loop-advance-prior-clean"),
        ]);
        let run = intake_blocked_service_run("run-service-loop-advance-intake");
        let run_next_history_len = run.next_runtime_input().history.len();

        let advance = AgentClosedLoopRuntimeServiceLoopAdvancePlanner::new().advance(
            prior_service_history,
            &run,
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        assert_eq!(advance.run_record.attempts(), 2);
        assert_eq!(
            advance.run_record.latest().status,
            AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
        );
        assert_eq!(
            advance.run_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            advance.loop_state.execution_history.len(),
            run_next_history_len
        );
        assert_eq!(advance.mode(), AgentClosedLoopNextTurnMode::Repair);
        assert!(advance.requires_repair_first());
        assert!(!advance.allows_service_advance());
        assert!(!advance.summary.allows_adaptive_evolution);
        assert!(
            advance
                .summary
                .reasons
                .iter()
                .any(|reason| reason == "service_run_health_requires_repair")
        );
        assert!(
            advance
                .run_record
                .latest()
                .blocked_reasons()
                .iter()
                .any(|reason| reason.contains("unexpected_command"))
        );
        assert!(
            advance
                .telemetry
                .iter()
                .any(|line| line == "service_loop_advance_requires_repair_first=true")
        );
    }

    #[test]
    fn runtime_service_loop_runner_runs_clean_receipts_into_advance() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let receipt_plan =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let receipts = receipt_plan
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let prior_service_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-loop-runner-prior-clean"),
        ]);
        let next_budget = BudgetLedger::new()
            .with_budget(AgentRole::Planner, AgentBudget::new(8, 1, 1))
            .with_budget(AgentRole::Tester, AgentBudget::new(16, 1, 1));
        let completed_task_ids = BTreeSet::from(["runtime-turn".to_owned()]);
        let input = AgentClosedLoopRuntimeServiceLoopRunInput::new(
            AgentClosedLoopRuntimeServiceRunInput::new(
                AgentClosedLoopRuntimeServiceRequestInput::new(
                    clean_runtime_input(),
                    AgentClosedLoopRuntimeBusinessInput::new(
                        "run-service-loop-runner-clean",
                        crate::ledger::AgentCycleLedger::new(),
                        report_evidence(),
                    ),
                ),
                receipts,
                AgentClosedLoopRuntimeContinuationInput::new(
                    next_budget.clone(),
                    AgentCycleEvidence::default(),
                )
                .with_completed_task_ids(completed_task_ids.clone())
                .with_max_parallel_tasks(3),
            ),
            prior_service_history,
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        let loop_run =
            AgentClosedLoopRuntimeServiceLoopRunner::new().run(input, &mut engine, &mut memory);

        assert_eq!(engine.calls, 1);
        assert_eq!(memory.submitted.len(), 1);
        assert_eq!(
            loop_run.run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert_eq!(loop_run.advance.run_record.attempts(), 2);
        assert_eq!(loop_run.mode(), AgentClosedLoopNextTurnMode::Continue);
        assert_eq!(loop_run.next_runtime_input().budget_ledger, next_budget);
        assert_eq!(
            loop_run.next_runtime_input().completed_task_ids,
            completed_task_ids
        );
        assert_eq!(loop_run.next_runtime_input().max_parallel_tasks, 3);
        assert!(loop_run.summary().allows_adaptive_evolution);
        assert!(
            loop_run
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_status=closed")
        );
        assert!(
            loop_run
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_mode=continue")
        );

        let summary = loop_run.compact_summary();
        assert_eq!(
            summary.service_run_status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert!(summary.dispatch_executable);
        assert!(summary.command_gate_allowed);
        assert_eq!(summary.side_effect_gate_count, summary.command_count);
        assert_eq!(summary.blocked_side_effect_gate_count, 0);
        assert!(summary.intake_clean);
        assert_eq!(summary.service_attempts, 2);
        assert_eq!(summary.mode, AgentClosedLoopNextTurnMode::Continue);
        assert_eq!(
            summary.service_run_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            summary.side_effect_admission_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(summary.can_schedule);
        assert!(summary.side_effect_dispatch_allowed);
        assert!(summary.memory_note_allowed);
        assert!(summary.allows_adaptive_evolution);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.side_effect_admission_reasons, 0);
        assert!(summary.blocked_reasons.is_empty());
        assert!(summary.preflight_reasons.is_empty());
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_summary_status=closed")
        );
        assert!(
            summary.telemetry.iter().any(|line| {
                line == "service_loop_run_summary_side_effect_dispatch_allowed=true"
            })
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "service_loop_run_summary_command_gate_allowed=true" })
        );
    }

    #[test]
    fn runtime_service_loop_runner_routes_intake_drift_to_repair_state() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let receipt_plan =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let mut receipts = receipt_plan
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        receipts.push(AgentServiceCommandReceipt::new(
            "unexpected_command",
            AgentServiceCommandStatus::Applied,
            "rogue executor output",
        ));
        let prior_service_history = AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
            clean_service_run_summary("run-service-loop-runner-prior-clean"),
        ]);
        let input = AgentClosedLoopRuntimeServiceLoopRunInput::new(
            AgentClosedLoopRuntimeServiceRunInput::new(
                AgentClosedLoopRuntimeServiceRequestInput::new(
                    clean_runtime_input(),
                    AgentClosedLoopRuntimeBusinessInput::new(
                        "run-service-loop-runner-intake",
                        crate::ledger::AgentCycleLedger::new(),
                        report_evidence(),
                    ),
                ),
                receipts,
                AgentClosedLoopRuntimeContinuationInput::new(
                    budget(),
                    AgentCycleEvidence::default(),
                ),
            ),
            prior_service_history,
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
        );

        let loop_run =
            AgentClosedLoopRuntimeServiceLoopRunner::new().run(input, &mut engine, &mut memory);

        assert_eq!(engine.calls, 1);
        assert_eq!(
            loop_run.run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
        );
        assert_eq!(
            loop_run.advance.run_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(loop_run.mode(), AgentClosedLoopNextTurnMode::Repair);
        assert!(loop_run.advance.requires_repair_first());
        assert_eq!(
            loop_run.loop_state().execution_history.len(),
            clean_runtime_input().history.len()
        );
        assert!(
            loop_run
                .next_queue()
                .task_ids()
                .iter()
                .any(|id| id.starts_with("service-preflight-repair-"))
        );
        assert!(
            loop_run
                .run
                .run_summary
                .blocked_reasons()
                .iter()
                .any(|reason| reason.contains("unexpected_command"))
        );
        assert!(
            loop_run
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_mode=repair")
        );

        let summary = loop_run.compact_summary();
        assert_eq!(
            summary.service_run_status,
            AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
        );
        assert!(summary.dispatch_executable);
        assert!(summary.command_gate_allowed);
        assert_eq!(summary.side_effect_gate_count, summary.command_count);
        assert_eq!(summary.blocked_side_effect_gate_count, 0);
        assert!(!summary.intake_clean);
        assert_eq!(summary.mode, AgentClosedLoopNextTurnMode::Repair);
        assert_eq!(
            summary.service_run_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(summary.requires_repair_first);
        assert!(!summary.allows_adaptive_evolution);
        assert!(
            summary
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains("unexpected_command"))
        );
        assert!(
            summary
                .preflight_reasons
                .iter()
                .any(|reason| reason == "service_run_health_requires_repair")
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_summary_mode=repair")
        );
    }

    #[test]
    fn runtime_service_loop_run_history_handles_empty_dashboard() {
        let history = AgentClosedLoopRuntimeServiceLoopRunHistory::new();

        let dashboard = history.dashboard();
        let health = history.health(AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default());

        assert!(history.is_empty());
        assert!(history.latest().is_none());
        assert!(dashboard.is_empty());
        assert!(dashboard.is_clean());
        assert_eq!(dashboard.total_runs, 0);
        assert_eq!(dashboard.closed_rate, 0.0);
        assert_eq!(dashboard.command_gate_allowed_rate, 0.0);
        assert_eq!(dashboard.repair_first_rate, 0.0);
        assert_eq!(dashboard.side_effect_dispatch_allowed_rate, 0.0);
        assert_eq!(dashboard.memory_note_allowed_rate, 0.0);
        assert_eq!(dashboard.adaptive_allowed_rate, 0.0);
        assert_eq!(dashboard.side_effect_gate_count, 0);
        assert_eq!(dashboard.blocked_side_effect_gate_count, 0);
        assert_eq!(dashboard.latest_status, None);
        assert_eq!(dashboard.latest_mode, None);
        assert!(dashboard.latest_blocked_reasons.is_empty());
        assert!(dashboard.latest_preflight_reasons.is_empty());
        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(health.reasons, vec!["service_loop_run_history_empty"]);
        assert!(health.dashboard.is_empty());
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn runtime_service_loop_run_health_marks_clean_transitions_stable() {
        let history = AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
            clean_service_loop_run_summary("run-service-loop-health-clean-1"),
            clean_service_loop_run_summary("run-service-loop-health-clean-2"),
        ]);

        let health = history.health(AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert_eq!(health.dashboard.total_runs, 2);
        assert_eq!(health.dashboard.closed_runs, 2);
        assert_eq!(health.dashboard.command_gate_allowed_runs, 2);
        assert_eq!(health.dashboard.repair_first_runs, 0);
        assert_eq!(health.dashboard.side_effect_dispatch_allowed_runs, 2);
        assert_eq!(health.dashboard.memory_note_allowed_runs, 2);
        assert_eq!(health.dashboard.adaptive_allowed_runs, 2);
        assert_eq!(
            health.dashboard.side_effect_gate_count,
            health.dashboard.command_count
        );
        assert_eq!(health.dashboard.blocked_side_effect_gate_count, 0);
        assert!(health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn runtime_service_loop_run_history_summarizes_clean_and_repair_transitions() {
        let clean = clean_service_loop_run_summary("run-service-loop-history-clean");
        let intake_blocked =
            intake_blocked_service_loop_run_summary("run-service-loop-history-intake");
        let mut history = AgentClosedLoopRuntimeServiceLoopRunHistory::new();

        history.push(clean.clone());
        history.push(intake_blocked.clone());
        let dashboard = history.dashboard();

        assert_eq!(history.len(), 2);
        assert_eq!(history.summaries()[0], clean);
        assert_eq!(history.latest(), Some(&intake_blocked));
        assert_eq!(dashboard.total_runs, 2);
        assert_eq!(dashboard.closed_runs, 1);
        assert_eq!(dashboard.dispatch_blocked_runs, 0);
        assert_eq!(dashboard.intake_blocked_runs, 1);
        assert_eq!(dashboard.command_gate_allowed_runs, 2);
        assert_eq!(dashboard.repair_first_runs, 1);
        assert_eq!(dashboard.side_effect_dispatch_allowed_runs, 1);
        assert_eq!(dashboard.memory_note_allowed_runs, 1);
        assert_eq!(dashboard.adaptive_allowed_runs, 1);
        assert!((dashboard.closed_rate - 0.5).abs() < 0.01);
        assert!((dashboard.command_gate_allowed_rate - 1.0).abs() < 0.01);
        assert!((dashboard.repair_first_rate - 0.5).abs() < 0.01);
        assert!((dashboard.side_effect_dispatch_allowed_rate - 0.5).abs() < 0.01);
        assert!((dashboard.memory_note_allowed_rate - 0.5).abs() < 0.01);
        assert!((dashboard.adaptive_allowed_rate - 0.5).abs() < 0.01);
        assert_eq!(
            dashboard.command_count,
            clean.command_count + intake_blocked.command_count
        );
        assert_eq!(
            dashboard.side_effect_gate_count,
            clean.side_effect_gate_count + intake_blocked.side_effect_gate_count
        );
        assert_eq!(
            dashboard.blocked_side_effect_gate_count,
            clean.blocked_side_effect_gate_count + intake_blocked.blocked_side_effect_gate_count
        );
        assert_eq!(
            dashboard.follow_up_task_count,
            clean.preflight_follow_up_tasks + intake_blocked.preflight_follow_up_tasks
        );
        assert_eq!(
            dashboard.total_next_queue_tasks,
            clean.next_queue_tasks + intake_blocked.next_queue_tasks
        );
        assert_eq!(
            dashboard.latest_status,
            Some(AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked)
        );
        assert_eq!(
            dashboard.latest_mode,
            Some(AgentClosedLoopNextTurnMode::Repair)
        );
        assert!(
            dashboard
                .latest_blocked_reasons
                .iter()
                .any(|reason| reason.contains("unexpected_command"))
        );
        assert!(
            dashboard
                .latest_preflight_reasons
                .iter()
                .any(|reason| reason == "service_run_health_requires_repair")
        );
        assert!(!dashboard.is_clean());
    }

    #[test]
    fn runtime_service_loop_run_health_repairs_intake_and_repair_first_drift() {
        let history = AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
            clean_service_loop_run_summary("run-service-loop-health-clean"),
            intake_blocked_service_loop_run_summary("run-service-loop-health-intake"),
        ]);

        let health = history.health(AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Repair);
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "service_loop_run_intake_blocked_runs=1>0")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "service_loop_run_repair_first_runs=1>0")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "service_loop_run_closed_rate=0.500<0.67")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason.starts_with("service_loop_run_latest_preflight="))
        );
        assert_eq!(health.dashboard.intake_blocked_runs, 1);
        assert_eq!(health.dashboard.repair_first_runs, 1);
        assert!(!health.is_stable());
        assert!(!health.allows_service_advance());
        assert!(health.requires_repair_first());
    }

    #[test]
    fn runtime_service_loop_run_history_recorder_appends_clean_transition() {
        let prior_history = AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
            clean_service_loop_run_summary("run-service-loop-record-prior"),
        ]);
        let loop_run = clean_service_loop_run("run-service-loop-record-clean");

        let record = AgentClosedLoopRuntimeServiceLoopRunHistoryRecorder::new().record(
            prior_history,
            &loop_run,
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );

        assert_eq!(record.records(), 2);
        assert_eq!(record.transitions(), 2);
        assert_eq!(
            record.latest().service_run_status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert_eq!(record.dashboard.closed_runs, 2);
        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_history_record_health=stable")
        );
    }

    #[test]
    fn runtime_service_loop_run_history_recorder_surfaces_repair_transition() {
        let prior_history = AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
            clean_service_loop_run_summary("run-service-loop-record-clean"),
        ]);
        let loop_run = intake_blocked_service_loop_run("run-service-loop-record-intake");

        let record = AgentClosedLoopRuntimeServiceLoopRunHistoryRecorder::new().record(
            prior_history,
            &loop_run,
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );

        assert_eq!(record.records(), 2);
        assert_eq!(record.transitions(), 2);
        assert_eq!(
            record.latest().service_run_status,
            AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
        );
        assert_eq!(record.dashboard.intake_blocked_runs, 1);
        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_history_record_health=repair")
        );
        assert!(
            record.telemetry.iter().any(|line| {
                line == "service_loop_run_history_record_reason=service_loop_run_intake_blocked_runs=1>0"
            })
        );
    }

    #[test]
    fn runtime_service_loop_run_control_continues_clean_transition_history() {
        let history = AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
            clean_service_loop_run_summary("run-service-loop-control-clean-1"),
            clean_service_loop_run_summary("run-service-loop-control-clean-2"),
        ]);

        let plan = AgentClosedLoopRuntimeServiceLoopRunController::new().plan(
            history,
            queue(),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Continue);
        assert_eq!(
            plan.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(plan.can_schedule());
        assert!(plan.allows_adaptive_evolution());
        assert!(!plan.requires_repair_first());
        assert!(plan.allows_service_advance());
        assert!(plan.reasons.is_empty());
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "service_loop_run_control_mode=continue")
        );
    }

    #[test]
    fn runtime_service_loop_run_control_repairs_transition_drift_first() {
        let history = AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
            clean_service_loop_run_summary("run-service-loop-control-clean"),
            intake_blocked_service_loop_run_summary("run-service-loop-control-intake"),
        ]);

        let plan = AgentClosedLoopRuntimeServiceLoopRunController::new().plan(
            history,
            queue(),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Repair);
        assert_eq!(
            plan.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(plan.can_schedule());
        assert!(!plan.allows_adaptive_evolution());
        assert!(plan.requires_repair_first());
        assert!(!plan.allows_service_advance());
        assert!(
            plan.reasons
                .iter()
                .any(|reason| reason == "service_loop_run_intake_blocked_runs=1>0")
        );
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "service_loop_run_control_adaptive_allowed=false")
        );
    }

    #[test]
    fn runtime_service_loop_run_control_idles_empty_queue_without_adaptive() {
        let history = AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
            clean_service_loop_run_summary("run-service-loop-control-idle"),
        ]);

        let plan = AgentClosedLoopRuntimeServiceLoopRunControlPlan::from_history(
            history,
            AgentTaskQueue::new(),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );

        assert_eq!(plan.mode, AgentClosedLoopNextTurnMode::Idle);
        assert_eq!(
            plan.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(!plan.can_schedule());
        assert!(!plan.allows_adaptive_evolution());
        assert!(!plan.requires_repair_first());
        assert_eq!(plan.reasons, vec!["next_queue_empty"]);
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "service_loop_run_control_can_schedule=false")
        );
    }

    #[test]
    fn runtime_service_loop_run_monitor_records_and_continues_clean_transition() {
        let prior_history = AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
            clean_service_loop_run_summary("run-service-loop-monitor-prior"),
        ]);
        let loop_run = clean_service_loop_run("run-service-loop-monitor-clean");

        let record = AgentClosedLoopRuntimeServiceLoopRunMonitor::new().record_and_plan(
            prior_history,
            &loop_run,
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );

        assert_eq!(record.transitions(), 2);
        assert_eq!(
            record.latest().service_run_status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert_eq!(
            record.control_plan.mode,
            AgentClosedLoopNextTurnMode::Continue
        );
        assert!(record.can_schedule());
        assert!(record.allows_adaptive_evolution());
        assert!(!record.requires_repair_first());
        assert!(record.allows_service_advance());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_control_record_mode=continue")
        );
    }

    #[test]
    fn runtime_service_loop_run_control_summary_compacts_clean_monitor_record() {
        let prior_history = AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
            clean_service_loop_run_summary("run-service-loop-summary-prior"),
        ]);
        let loop_run = clean_service_loop_run("run-service-loop-summary-clean");
        let record = AgentClosedLoopRuntimeServiceLoopRunMonitor::new().record_and_plan(
            prior_history,
            &loop_run,
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );

        let summary = record.summary();

        assert_eq!(
            summary.latest_status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert_eq!(summary.mode, AgentClosedLoopNextTurnMode::Continue);
        assert_eq!(
            summary.health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(summary.transitions, 2);
        assert_eq!(summary.closed_rate, 1.0);
        assert_eq!(summary.command_gate_allowed_rate, 1.0);
        assert_eq!(summary.repair_first_rate, 0.0);
        assert_eq!(summary.side_effect_dispatch_allowed_rate, 1.0);
        assert_eq!(summary.memory_note_allowed_rate, 1.0);
        assert_eq!(summary.adaptive_allowed_rate, 1.0);
        assert_eq!(
            summary.side_effect_gate_count,
            record.history_record.dashboard.command_count
        );
        assert_eq!(summary.blocked_side_effect_gate_count, 0);
        assert!(summary.can_schedule);
        assert!(summary.allows_adaptive_evolution);
        assert!(!summary.requires_repair_first);
        assert_eq!(
            summary.next_queue_tasks,
            record.control_plan.next_queue.len()
        );
        assert!(summary.reasons.is_empty());
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_control_summary_mode=continue")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "service_loop_run_control_summary_side_effect_dispatch_allowed_rate=1.000"
        }));
        assert!(summary.telemetry.iter().any(|line| {
            line == "service_loop_run_control_summary_command_gate_allowed_rate=1.000"
        }));
    }

    #[test]
    fn runtime_service_loop_run_monitor_records_and_repairs_drift() {
        let prior_history = AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
            clean_service_loop_run_summary("run-service-loop-monitor-clean"),
        ]);
        let loop_run = intake_blocked_service_loop_run("run-service-loop-monitor-intake");

        let record = AgentClosedLoopRuntimeServiceLoopRunMonitor::new().record_and_plan(
            prior_history,
            &loop_run,
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );

        assert_eq!(record.transitions(), 2);
        assert_eq!(
            record.latest().service_run_status,
            AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
        );
        assert_eq!(
            record.control_plan.mode,
            AgentClosedLoopNextTurnMode::Repair
        );
        assert!(record.can_schedule());
        assert!(!record.allows_adaptive_evolution());
        assert!(record.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(
            record
                .control_plan
                .reasons
                .iter()
                .any(|reason| reason == "service_loop_run_intake_blocked_runs=1>0")
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_control_record_mode=repair")
        );
    }

    #[test]
    fn runtime_service_loop_run_control_summary_compacts_repair_monitor_record() {
        let prior_history = AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
            clean_service_loop_run_summary("run-service-loop-summary-clean"),
        ]);
        let loop_run = intake_blocked_service_loop_run("run-service-loop-summary-intake");
        let record = AgentClosedLoopRuntimeServiceLoopRunMonitor::new().record_and_plan(
            prior_history,
            &loop_run,
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );

        let summary = record.summary();

        assert_eq!(
            summary.latest_status,
            AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
        );
        assert_eq!(summary.mode, AgentClosedLoopNextTurnMode::Repair);
        assert_eq!(
            summary.health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(summary.transitions, 2);
        assert!((summary.closed_rate - 0.5).abs() < 0.01);
        assert!((summary.command_gate_allowed_rate - 1.0).abs() < 0.01);
        assert!((summary.repair_first_rate - 0.5).abs() < 0.01);
        assert!((summary.side_effect_dispatch_allowed_rate - 0.5).abs() < 0.01);
        assert!((summary.memory_note_allowed_rate - 0.5).abs() < 0.01);
        assert!((summary.adaptive_allowed_rate - 0.5).abs() < 0.01);
        assert_eq!(
            summary.side_effect_gate_count,
            record.history_record.dashboard.command_count
        );
        assert_eq!(summary.blocked_side_effect_gate_count, 0);
        assert!(summary.can_schedule);
        assert!(!summary.allows_adaptive_evolution);
        assert!(summary.requires_repair_first);
        assert!(
            summary
                .reasons
                .iter()
                .any(|reason| reason == "service_loop_run_intake_blocked_runs=1>0")
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_control_summary_mode=repair")
        );
    }

    #[test]
    fn runtime_service_loop_run_control_summary_history_handles_empty_dashboard() {
        let history = AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::new();

        let dashboard = history.dashboard();
        let health =
            history.health(AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default());

        assert!(history.is_empty());
        assert!(history.latest().is_none());
        assert!(dashboard.is_empty());
        assert!(dashboard.is_clean());
        assert_eq!(dashboard.total_records, 0);
        assert_eq!(dashboard.schedule_rate, 0.0);
        assert_eq!(dashboard.command_gate_allowed_rate, 0.0);
        assert_eq!(dashboard.side_effect_dispatch_allowed_rate, 0.0);
        assert_eq!(dashboard.memory_note_allowed_rate, 0.0);
        assert_eq!(dashboard.adaptive_allowed_rate, 0.0);
        assert_eq!(dashboard.repair_first_rate, 0.0);
        assert_eq!(dashboard.side_effect_gate_count, 0);
        assert_eq!(dashboard.blocked_side_effect_gate_count, 0);
        assert_eq!(dashboard.latest_status, None);
        assert_eq!(dashboard.latest_mode, None);
        assert_eq!(dashboard.latest_health_status, None);
        assert!(dashboard.latest_reasons.is_empty());
        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["service_loop_run_control_history_empty"]
        );
        assert!(health.dashboard.is_empty());
        assert!(!health.is_stable());
    }

    #[test]
    fn runtime_service_loop_run_control_health_marks_clean_history_stable() {
        let summaries = vec![
            AgentClosedLoopRuntimeServiceLoopRunMonitor::new()
                .record_and_plan(
                    AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                        clean_service_loop_run_summary("run-service-loop-control-health-prior-1"),
                    ]),
                    &clean_service_loop_run("run-service-loop-control-health-clean-1"),
                    AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
                )
                .summary(),
            AgentClosedLoopRuntimeServiceLoopRunMonitor::new()
                .record_and_plan(
                    AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                        clean_service_loop_run_summary("run-service-loop-control-health-prior-2"),
                    ]),
                    &clean_service_loop_run("run-service-loop-control-health-clean-2"),
                    AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
                )
                .summary(),
        ];
        let history =
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::from_summaries(summaries);

        let health =
            history.health(AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert_eq!(health.dashboard.total_records, 2);
        assert_eq!(health.dashboard.continue_records, 2);
        assert_eq!(health.dashboard.repair_records, 0);
        assert!(health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn runtime_service_loop_run_control_summary_history_summarizes_daemon_pressure() {
        let clean_record = AgentClosedLoopRuntimeServiceLoopRunMonitor::new().record_and_plan(
            AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                clean_service_loop_run_summary("run-service-loop-summary-history-prior"),
            ]),
            &clean_service_loop_run("run-service-loop-summary-history-clean"),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );
        let repair_record = AgentClosedLoopRuntimeServiceLoopRunMonitor::new().record_and_plan(
            AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                clean_service_loop_run_summary("run-service-loop-summary-history-clean"),
            ]),
            &intake_blocked_service_loop_run("run-service-loop-summary-history-intake"),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );
        let clean = clean_record.summary();
        let repair = repair_record.summary();
        let mut idle = clean.clone();
        idle.mode = AgentClosedLoopNextTurnMode::Idle;
        idle.can_schedule = false;
        idle.allows_adaptive_evolution = false;
        idle.requires_repair_first = false;
        idle.next_queue_tasks = 0;
        idle.reasons = vec!["next_queue_empty".to_owned()];
        let history =
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::from_summaries(vec![
                clean.clone(),
                repair.clone(),
                idle.clone(),
            ]);

        let dashboard = history.dashboard();

        assert_eq!(history.len(), 3);
        assert_eq!(history.summaries()[0], clean);
        assert_eq!(history.latest(), Some(&idle));
        assert_eq!(dashboard.total_records, 3);
        assert_eq!(dashboard.continue_records, 1);
        assert_eq!(dashboard.observe_records, 0);
        assert_eq!(dashboard.repair_records, 1);
        assert_eq!(dashboard.idle_records, 1);
        assert_eq!(dashboard.schedulable_records, 2);
        assert_eq!(dashboard.adaptive_allowed_records, 1);
        assert_eq!(dashboard.repair_first_records, 1);
        assert!((dashboard.schedule_rate - 0.666).abs() < 0.01);
        assert!((dashboard.command_gate_allowed_rate - 1.0).abs() < 0.01);
        assert!((dashboard.side_effect_dispatch_allowed_rate - 0.833).abs() < 0.01);
        assert!((dashboard.memory_note_allowed_rate - 0.833).abs() < 0.01);
        assert!((dashboard.adaptive_allowed_rate - 0.333).abs() < 0.01);
        assert!((dashboard.repair_first_rate - 0.333).abs() < 0.01);
        assert_eq!(
            dashboard.side_effect_gate_count,
            clean.side_effect_gate_count
                + repair.side_effect_gate_count
                + idle.side_effect_gate_count
        );
        assert_eq!(
            dashboard.blocked_side_effect_gate_count,
            clean.blocked_side_effect_gate_count
                + repair.blocked_side_effect_gate_count
                + idle.blocked_side_effect_gate_count
        );
        assert_eq!(
            dashboard.total_next_queue_tasks,
            clean.next_queue_tasks + repair.next_queue_tasks
        );
        assert_eq!(
            dashboard.latest_status,
            Some(AgentClosedLoopRuntimeServiceRunStatus::Closed)
        );
        assert_eq!(
            dashboard.latest_mode,
            Some(AgentClosedLoopNextTurnMode::Idle)
        );
        assert_eq!(
            dashboard.latest_health_status,
            Some(AgentClosedLoopExecutionHealthStatus::Stable)
        );
        assert_eq!(dashboard.latest_reasons, vec!["next_queue_empty"]);
        assert!(!dashboard.is_clean());
    }

    #[test]
    fn runtime_service_loop_run_control_health_repairs_daemon_pressure() {
        let clean_record = AgentClosedLoopRuntimeServiceLoopRunMonitor::new().record_and_plan(
            AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                clean_service_loop_run_summary("run-service-loop-control-health-clean"),
            ]),
            &clean_service_loop_run("run-service-loop-control-health-clean-next"),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );
        let repair_record = AgentClosedLoopRuntimeServiceLoopRunMonitor::new().record_and_plan(
            AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                clean_service_loop_run_summary("run-service-loop-control-health-repair-prior"),
            ]),
            &intake_blocked_service_loop_run("run-service-loop-control-health-repair"),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
        );
        let mut idle = clean_record.summary();
        idle.mode = AgentClosedLoopNextTurnMode::Idle;
        idle.can_schedule = false;
        idle.allows_adaptive_evolution = false;
        idle.next_queue_tasks = 0;
        idle.reasons = vec!["next_queue_empty".to_owned()];
        let history =
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::from_summaries(vec![
                clean_record.summary(),
                repair_record.summary(),
                idle,
            ]);

        let health =
            history.health(AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default());

        assert_eq!(health.status, AgentClosedLoopExecutionHealthStatus::Repair);
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "service_loop_run_control_repair_records=1>0")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "service_loop_run_control_repair_first_records=1>0")
        );
        assert!(
            health
                .reasons
                .iter()
                .any(|reason| reason == "service_loop_run_control_latest_reason=next_queue_empty")
        );
        assert_eq!(health.dashboard.repair_records, 1);
        assert_eq!(health.dashboard.idle_records, 1);
        assert!(!health.is_stable());
        assert!(!health.allows_service_advance());
        assert!(health.requires_repair_first());
    }

    #[test]
    fn runtime_service_loop_run_control_summary_history_recorder_appends_clean_record() {
        let prior_summary = AgentClosedLoopRuntimeServiceLoopRunMonitor::new()
            .record_and_plan(
                AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                    clean_service_loop_run_summary("run-service-loop-control-record-prior"),
                ]),
                &clean_service_loop_run("run-service-loop-control-record-prior-clean"),
                AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            )
            .summary();
        let next_summary = AgentClosedLoopRuntimeServiceLoopRunMonitor::new()
            .record_and_plan(
                AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                    clean_service_loop_run_summary("run-service-loop-control-record-next"),
                ]),
                &clean_service_loop_run("run-service-loop-control-record-next-clean"),
                AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            )
            .summary();
        let history =
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::from_summaries(vec![
                prior_summary,
            ]);

        let record = AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder::new()
            .record(
                history,
                next_summary,
                AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default(),
            );

        assert_eq!(record.records(), 2);
        assert_eq!(record.latest().mode, AgentClosedLoopNextTurnMode::Continue);
        assert_eq!(record.dashboard.continue_records, 2);
        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.telemetry.iter().any(|line| {
            line == "service_loop_run_control_summary_history_record_health=stable"
        }));
    }

    #[test]
    fn runtime_service_loop_run_control_summary_history_recorder_surfaces_repair_health() {
        let prior_summary = AgentClosedLoopRuntimeServiceLoopRunMonitor::new()
            .record_and_plan(
                AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                    clean_service_loop_run_summary("run-service-loop-control-record-clean"),
                ]),
                &clean_service_loop_run("run-service-loop-control-record-clean-next"),
                AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            )
            .summary();
        let repair_summary = AgentClosedLoopRuntimeServiceLoopRunMonitor::new()
            .record_and_plan(
                AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                    clean_service_loop_run_summary("run-service-loop-control-record-repair-prior"),
                ]),
                &intake_blocked_service_loop_run("run-service-loop-control-record-repair"),
                AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            )
            .summary();
        let history =
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::from_summaries(vec![
                prior_summary,
            ]);

        let record = AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder::new()
            .record(
                history,
                repair_summary,
                AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default(),
            );

        assert_eq!(record.records(), 2);
        assert_eq!(record.latest().mode, AgentClosedLoopNextTurnMode::Repair);
        assert_eq!(record.dashboard.repair_records, 1);
        assert_eq!(
            record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_control_summary_history_record_health=repair")
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "service_loop_run_control_summary_history_record_reason=service_loop_run_control_repair_records=1>0"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_runner_records_clean_control_history() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceLoopRunDaemonInput::new(
            service_loop_run_input_with_receipts(
                "run-service-loop-daemon-clean",
                clean_service_loop_run_receipts(),
            ),
            AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                clean_service_loop_run_summary("run-service-loop-daemon-transition-prior"),
            ]),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::new(),
            AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default(),
        );

        let record = AgentClosedLoopRuntimeServiceLoopRunDaemonRunner::new().run(
            input,
            &mut engine,
            &mut memory,
        );

        assert_eq!(engine.calls, 1);
        assert_eq!(memory.submitted.len(), 1);
        assert_eq!(
            record.loop_run.run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert_eq!(record.mode(), AgentClosedLoopNextTurnMode::Continue);
        assert!(record.can_schedule());
        assert!(record.allows_adaptive_evolution());
        assert!(!record.requires_repair_first());
        assert!(record.allows_service_advance());
        assert_eq!(record.control_record.transitions(), 2);
        assert_eq!(record.control_summary.transitions, 2);
        assert_eq!(
            record.control_summary.health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            record.control_summary_history_record.latest(),
            &record.control_summary
        );
        assert_eq!(record.control_summary_history_record.records(), 1);
        assert_eq!(
            record.control_summary_history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            record
                .control_summary_history_record
                .dashboard
                .continue_records,
            1
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_daemon_record_control_health=stable")
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_daemon_record_mode=continue")
        );
    }

    #[test]
    fn runtime_service_loop_run_daemon_runner_surfaces_receipt_drift_as_repair_control() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let mut receipts = clean_service_loop_run_receipts();
        receipts.push(AgentServiceCommandReceipt::new(
            "unexpected_command",
            AgentServiceCommandStatus::Applied,
            "rogue executor output",
        ));
        let input = AgentClosedLoopRuntimeServiceLoopRunDaemonInput::new(
            service_loop_run_input_with_receipts("run-service-loop-daemon-intake", receipts),
            AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                clean_service_loop_run_summary("run-service-loop-daemon-repair-prior"),
            ]),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::new(),
            AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default(),
        );

        let record = AgentClosedLoopRuntimeServiceLoopRunDaemonRunner::new().run(
            input,
            &mut engine,
            &mut memory,
        );

        assert_eq!(engine.calls, 1);
        assert_eq!(
            record.loop_run.run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::IntakeBlocked
        );
        assert_eq!(record.mode(), AgentClosedLoopNextTurnMode::Repair);
        assert!(record.can_schedule());
        assert!(!record.allows_adaptive_evolution());
        assert!(record.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert_eq!(
            record.control_record.history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            record.control_summary.health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            record.control_summary_history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            record
                .control_summary_history_record
                .dashboard
                .repair_records,
            1
        );
        assert!(
            record
                .control_summary
                .reasons
                .iter()
                .any(|reason| reason.contains("service_loop_run_repair_first_runs"))
        );
        assert!(
            record
                .loop_run
                .run
                .run_summary
                .blocked_reasons()
                .iter()
                .any(|reason| reason.contains("unexpected_command"))
        );
        assert!(
            record
                .control_summary_history_record
                .health
                .reasons
                .iter()
                .any(|reason| reason == "service_loop_run_control_repair_records=1>0")
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_daemon_record_control_health=repair")
        );
    }

    #[test]
    fn runtime_service_loop_run_daemon_continuation_packages_next_state() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let service_run_policy = AgentClosedLoopRuntimeServiceRunHealthPolicy {
            minimum_closed_rate: 0.50,
            ..AgentClosedLoopRuntimeServiceRunHealthPolicy::default()
        };
        let loop_run_health_policy = AgentClosedLoopRuntimeServiceLoopRunHealthPolicy {
            minimum_adaptive_allowed_rate: 0.25,
            ..AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default()
        };
        let control_health_policy = AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy {
            minimum_schedule_rate: 0.50,
            ..AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default()
        };
        let input = AgentClosedLoopRuntimeServiceLoopRunDaemonInput::new(
            AgentClosedLoopRuntimeServiceLoopRunInput::new(
                AgentClosedLoopRuntimeServiceRunInput::new(
                    AgentClosedLoopRuntimeServiceRequestInput::new(
                        clean_runtime_input(),
                        AgentClosedLoopRuntimeBusinessInput::new(
                            "run-service-loop-daemon-continuation",
                            crate::ledger::AgentCycleLedger::new(),
                            report_evidence(),
                        ),
                    ),
                    clean_service_loop_run_receipts(),
                    AgentClosedLoopRuntimeContinuationInput::new(
                        budget(),
                        AgentCycleEvidence::default(),
                    ),
                ),
                AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
                    clean_service_run_summary("run-service-loop-daemon-continuation-service"),
                ]),
                service_run_policy,
            ),
            AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                clean_service_loop_run_summary("run-service-loop-daemon-continuation-transition"),
            ]),
            loop_run_health_policy,
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::new(),
            control_health_policy,
        );

        let record = AgentClosedLoopRuntimeServiceLoopRunDaemonRunner::new().run(
            input,
            &mut engine,
            &mut memory,
        );
        let continuation =
            AgentClosedLoopRuntimeServiceLoopRunDaemonContinuationPlanner::new().plan(&record);

        assert_eq!(
            continuation.next_runtime_input,
            record.next_runtime_input().clone()
        );
        assert_eq!(
            continuation.service_run_history,
            record.loop_run.advance.run_record.history
        );
        assert_eq!(
            continuation.loop_run_history,
            record.control_record.history_record.history
        );
        assert_eq!(
            continuation.control_summary_history,
            record.control_summary_history_record.history
        );
        assert_eq!(continuation.service_run_policy, service_run_policy);
        assert_eq!(continuation.loop_run_health_policy, loop_run_health_policy);
        assert_eq!(continuation.control_health_policy, control_health_policy);
        assert_eq!(continuation.mode, AgentClosedLoopNextTurnMode::Continue);
        assert!(continuation.can_schedule);
        assert_eq!(continuation.side_effect_dispatch_allowed_rate, 1.0);
        assert_eq!(continuation.memory_note_allowed_rate, 1.0);
        assert!(continuation.allows_adaptive_evolution);
        assert!(!continuation.requires_repair_first);
        assert_eq!(
            continuation.transition_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            continuation.control_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(continuation.service_run_history.len(), 2);
        assert_eq!(continuation.loop_run_history.len(), 2);
        assert_eq!(continuation.control_summary_history.len(), 1);
        assert!(
            continuation.telemetry.iter().any(|line| {
                line == "service_loop_run_daemon_continuation_control_health=stable"
            })
        );
        assert!(continuation.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_continuation_side_effect_dispatch_allowed_rate=1.000"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_input_planner_builds_next_daemon_input() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let first_input = AgentClosedLoopRuntimeServiceLoopRunDaemonInput::new(
            service_loop_run_input_with_receipts(
                "run-service-loop-daemon-input-plan-first",
                clean_service_loop_run_receipts(),
            ),
            AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                clean_service_loop_run_summary("run-service-loop-daemon-input-plan-prior"),
            ]),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::new(),
            AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default(),
        );
        let first_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRunner::new().run(
            first_input,
            &mut engine,
            &mut memory,
        );
        let continuation = first_record.continuation();
        let next_receipts = clean_service_loop_run_receipts();
        let receipt_count = next_receipts.len();

        let plan = AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-input-plan-next",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            next_receipts,
        );

        assert_eq!(
            plan.input
                .loop_run_input
                .service_run_input
                .request_input
                .runtime_input,
            continuation.next_runtime_input
        );
        assert_eq!(
            plan.input
                .loop_run_input
                .service_run_input
                .request_input
                .business_input
                .run_id,
            "run-service-loop-daemon-input-plan-next"
        );
        assert_eq!(
            plan.input.loop_run_input.service_run_input.receipts.len(),
            receipt_count
        );
        assert_eq!(
            plan.input.loop_run_input.service_run_history,
            continuation.service_run_history
        );
        assert_eq!(plan.input.loop_run_history, continuation.loop_run_history);
        assert_eq!(
            plan.input.control_summary_history,
            continuation.control_summary_history
        );
        assert_eq!(
            plan.input.loop_run_input.service_run_policy,
            continuation.service_run_policy
        );
        assert_eq!(
            plan.input.loop_run_health_policy,
            continuation.loop_run_health_policy
        );
        assert_eq!(
            plan.input.control_health_policy,
            continuation.control_health_policy
        );
        assert_eq!(
            plan.side_effect_dispatch_allowed_rate,
            continuation.side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            plan.memory_note_allowed_rate,
            continuation.memory_note_allowed_rate
        );
        assert!(
            plan.telemetry
                .iter()
                .any(|line| { line == "service_loop_run_daemon_input_plan_control_health=stable" })
        );
        assert!(plan.telemetry.iter().any(|line| {
            line == &format!("service_loop_run_daemon_input_plan_receipts={receipt_count}")
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_input_planner_preserves_model_routes() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let mut continuation = clean_daemon_continuation(
            "run-service-loop-daemon-input-plan-route-first",
            &mut engine,
            &mut memory,
        );
        let route = route_request(continuation.next_runtime_input.next_queue.tasks()[0].clone());
        continuation.next_runtime_input.model_routes = vec![route.clone()];

        let plan = AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-input-plan-route-next",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            clean_service_loop_run_receipts(),
        );

        assert_eq!(
            plan.input
                .loop_run_input
                .service_run_input
                .request_input
                .runtime_input
                .model_routes,
            vec![route.clone()]
        );
        assert_eq!(
            plan.input
                .loop_run_input
                .service_run_input
                .continuation_input
                .model_routes,
            vec![route]
        );
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_planner_prepares_side_effect_boundary() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation = clean_daemon_continuation(
            "run-service-loop-daemon-request-plan-first",
            &mut engine,
            &mut memory,
        );
        let engine_calls_after_continuation = engine.calls;
        let memory_submissions_after_continuation = memory.submitted.len();

        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-plan-next",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );

        assert_eq!(engine.calls, engine_calls_after_continuation);
        assert_eq!(
            memory.submitted.len(),
            memory_submissions_after_continuation
        );
        assert_eq!(
            request_plan.request_input.runtime_input,
            continuation.next_runtime_input
        );
        assert_eq!(
            request_plan.request_input.business_input.run_id,
            "run-service-loop-daemon-request-plan-next"
        );
        assert_eq!(
            request_plan.service_run_history,
            continuation.service_run_history
        );
        assert_eq!(request_plan.loop_run_history, continuation.loop_run_history);
        assert_eq!(
            request_plan.control_summary_history,
            continuation.control_summary_history
        );
        assert_eq!(
            request_plan.service_run_policy,
            continuation.service_run_policy
        );
        assert_eq!(
            request_plan.loop_run_health_policy,
            continuation.loop_run_health_policy
        );
        assert_eq!(
            request_plan.control_health_policy,
            continuation.control_health_policy
        );
        assert_eq!(request_plan.mode, continuation.mode);
        assert_eq!(request_plan.can_schedule, continuation.can_schedule);
        assert_eq!(
            request_plan.side_effect_dispatch_allowed_rate,
            continuation.side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            request_plan.memory_note_allowed_rate,
            continuation.memory_note_allowed_rate
        );
        assert_eq!(
            request_plan.allows_adaptive_evolution,
            continuation.allows_adaptive_evolution
        );
        assert_eq!(
            request_plan.requires_repair_first,
            continuation.requires_repair_first
        );
        assert_eq!(
            request_plan.continuation_input.completed_task_ids,
            continuation.next_runtime_input.completed_task_ids
        );
        assert_eq!(
            request_plan.continuation_input.budget_ledger,
            continuation.next_runtime_input.budget_ledger
        );
        assert!(
            request_plan.telemetry.iter().any(|line| {
                line == "service_loop_run_daemon_request_plan_control_health=stable"
            })
        );
        assert!(request_plan.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_plan_memory_note_allowed_rate=1.000"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_plan_materializes_input_with_receipts() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation = clean_daemon_continuation(
            "run-service-loop-daemon-request-materialize-first",
            &mut engine,
            &mut memory,
        );
        let receipts = clean_service_loop_run_receipts();
        let receipt_count = receipts.len();
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-materialize-next",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );

        let expected_dispatch_rate = request_plan.side_effect_dispatch_allowed_rate;
        let expected_memory_rate = request_plan.memory_note_allowed_rate;
        let input_plan = request_plan.with_receipts(receipts);

        assert_eq!(
            input_plan
                .input
                .loop_run_input
                .service_run_input
                .request_input
                .business_input
                .run_id,
            "run-service-loop-daemon-request-materialize-next"
        );
        assert_eq!(
            input_plan
                .input
                .loop_run_input
                .service_run_input
                .receipts
                .len(),
            receipt_count
        );
        assert_eq!(
            input_plan.input.loop_run_input.service_run_history,
            continuation.service_run_history
        );
        assert_eq!(
            input_plan.input.loop_run_history,
            continuation.loop_run_history
        );
        assert_eq!(
            input_plan.input.control_summary_history,
            continuation.control_summary_history
        );
        assert_eq!(
            input_plan.side_effect_dispatch_allowed_rate,
            expected_dispatch_rate
        );
        assert_eq!(input_plan.memory_note_allowed_rate, expected_memory_rate);
        assert!(
            input_plan
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_daemon_input_plan_from_request=true")
        );
        assert!(input_plan.telemetry.iter().any(|line| {
            line == &format!("service_loop_run_daemon_input_plan_request_receipts={receipt_count}")
        }));
        assert!(input_plan.telemetry.iter().any(|line| {
            line == &format!("service_loop_run_daemon_input_plan_receipts={receipt_count}")
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_runner_closes_clean_receipts_without_rerun() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation = daemon_continuation_for_runtime_input(clean_runtime_input());
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-record-clean",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );

        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );

        assert_eq!(engine.calls, 1);
        assert_eq!(memory.submitted.len(), 1);
        assert!(request_record.is_executable());
        assert!(request_record.command_plan().is_some());
        assert!(request_record.skipped_reasons().is_empty());
        assert!(request_record.dispatch_summary.command_count > 0);
        assert!(request_record.dispatch_summary.command_gate_allowed);
        assert_eq!(
            request_record.dispatch_summary.side_effect_gate_count,
            request_record.dispatch_summary.command_count
        );
        assert_eq!(
            request_record
                .dispatch_summary
                .blocked_side_effect_gate_count,
            0
        );
        assert!(
            request_record
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_daemon_request_record_executable=true")
        );
        assert!(request_record.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_record_command_gate_allowed=true"
        }));

        let receipts = request_record
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let daemon_record = request_record.close_with_receipts(receipts);

        assert_eq!(engine.calls, 1);
        assert_eq!(memory.submitted.len(), 1);
        assert_eq!(
            daemon_record.loop_run.run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert_eq!(daemon_record.mode(), AgentClosedLoopNextTurnMode::Continue);
        assert_eq!(daemon_record.loop_run.advance.run_record.attempts(), 2);
        assert_eq!(daemon_record.control_record.transitions(), 2);
        assert_eq!(daemon_record.control_summary_history_record.records(), 1);
        assert_eq!(
            daemon_record.control_summary_history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(
            daemon_record
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_daemon_receipt_close=true")
        );
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_runner_preserves_skipped_gate() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation =
            daemon_continuation_for_runtime_input(AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ));
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-record-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );

        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );

        assert_eq!(engine.calls, 0);
        assert_eq!(memory.submitted.len(), 0);
        assert!(!request_record.is_executable());
        assert!(request_record.command_plan().is_none());
        assert!(!request_record.dispatch_summary.command_gate_allowed);
        assert_eq!(request_record.dispatch_summary.side_effect_gate_count, 0);
        assert_eq!(
            request_record
                .dispatch_summary
                .blocked_side_effect_gate_count,
            0
        );
        assert_eq!(
            request_record.skipped_reasons(),
            &["next_queue_empty".to_owned()]
        );
        assert!(
            request_record
                .telemetry
                .iter()
                .any(|line| line == "service_loop_run_daemon_request_record_executable=false")
        );

        let daemon_record = request_record.close_with_receipts(Vec::new());

        assert_eq!(engine.calls, 0);
        assert_eq!(memory.submitted.len(), 0);
        assert_eq!(
            daemon_record.loop_run.run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::DispatchBlocked
        );
        assert_eq!(daemon_record.mode(), AgentClosedLoopNextTurnMode::Repair);
        assert!(daemon_record.requires_repair_first());
        assert_eq!(
            daemon_record.control_summary_history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(
            daemon_record
                .loop_run
                .run
                .run_summary
                .blocked_reasons()
                .iter()
                .any(|reason| reason == "next_queue_empty")
        );
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_summary_history_marks_clean_boundary_stable() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation = daemon_continuation_for_runtime_input(clean_runtime_input());
        let mut request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new()
            .plan(
                &continuation,
                AgentClosedLoopRuntimeBusinessInput::new(
                    "run-service-loop-daemon-request-summary-clean",
                    crate::ledger::AgentCycleLedger::new(),
                    report_evidence(),
                ),
            );
        request_plan.side_effect_dispatch_allowed_rate = 1.0;
        request_plan.memory_note_allowed_rate = 1.0;
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );

        let summary = request_record.summary();
        let history_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecorder::new().record(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                summary.clone(),
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
            );

        assert!(summary.executable);
        assert!(summary.command_count > 0);
        assert!(summary.command_gate_allowed);
        assert_eq!(summary.side_effect_gate_count, summary.command_count);
        assert_eq!(summary.blocked_side_effect_gate_count, 0);
        assert_eq!(summary.side_effect_dispatch_allowed_rate, 1.0);
        assert_eq!(summary.memory_note_allowed_rate, 1.0);
        assert!(summary.blocked_reasons.is_empty());
        assert!(summary.skipped_reasons.is_empty());
        assert_eq!(history_record.records(), 1);
        assert_eq!(history_record.latest(), &summary);
        assert_eq!(history_record.dashboard.executable_records, 1);
        assert_eq!(history_record.dashboard.command_gate_allowed_records, 1);
        assert_eq!(history_record.dashboard.blocked_records, 0);
        assert_eq!(history_record.dashboard.skipped_records, 0);
        assert_eq!(history_record.dashboard.command_gate_allowed_rate, 1.0);
        assert_eq!(
            history_record.dashboard.side_effect_gate_count,
            summary.side_effect_gate_count
        );
        assert_eq!(history_record.dashboard.blocked_side_effect_gate_count, 0);
        assert_eq!(
            history_record.dashboard.side_effect_dispatch_allowed_rate,
            1.0
        );
        assert_eq!(history_record.dashboard.memory_note_allowed_rate, 1.0);
        assert_eq!(
            history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(history_record.allows_service_advance());
        assert!(!history_record.requires_repair_first());
        assert!(history_record.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_summary_history_record_health=stable"
        }));
        assert!(history_record.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_summary_history_record_side_effect_dispatch_allowed_rate=1.000"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_summary_history_repairs_skipped_boundary() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation =
            daemon_continuation_for_runtime_input(AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ));
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-summary-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );

        let summary = request_record.summary();
        let history_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecorder::new().record(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                summary.clone(),
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
            );

        assert!(!summary.executable);
        assert_eq!(summary.command_count, 0);
        assert!(!summary.command_gate_allowed);
        assert_eq!(summary.side_effect_gate_count, 0);
        assert_eq!(summary.blocked_side_effect_gate_count, 0);
        assert_eq!(summary.side_effect_dispatch_allowed_rate, 0.0);
        assert_eq!(summary.memory_note_allowed_rate, 0.0);
        assert_eq!(summary.skipped_reasons, vec!["next_queue_empty"]);
        assert_eq!(history_record.dashboard.executable_records, 0);
        assert_eq!(history_record.dashboard.command_gate_allowed_records, 0);
        assert_eq!(history_record.dashboard.skipped_records, 1);
        assert_eq!(history_record.dashboard.command_gate_allowed_rate, 0.0);
        assert_eq!(history_record.dashboard.side_effect_gate_count, 0);
        assert_eq!(history_record.dashboard.blocked_side_effect_gate_count, 0);
        assert_eq!(
            history_record.dashboard.side_effect_dispatch_allowed_rate,
            0.0
        );
        assert_eq!(history_record.dashboard.memory_note_allowed_rate, 0.0);
        assert_eq!(
            history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!history_record.allows_service_advance());
        assert!(history_record.requires_repair_first());
        assert!(
            history_record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "service_loop_run_daemon_request_skipped_records=1>0" })
        );
        assert!(history_record.health.reasons.iter().any(|reason| {
            reason == "service_loop_run_daemon_request_latest_skipped=next_queue_empty"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_summary_history_repairs_repair_first_task_missing() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let repair_history = AgentClosedLoopExecutionHistory::from_summaries(vec![
            AgentClosedLoopExecutionSummary {
                run_id: "run-service-loop-daemon-request-summary-repair-prior".to_owned(),
                clean: false,
                report_accepted: true,
                loopback_promoted: true,
                service_clean: false,
                reward_total: 0.42,
                admission_status: AgentCycleLedgerAdmissionStatus::Repair,
                command_count: 2,
                missing_command_count: 0,
                failed_command_count: 0,
                skipped_command_count: 0,
                next_queue_tasks: 1,
                next_queue_task_ids: vec!["runtime-turn".to_owned()],
                blocked_reasons: vec!["repair_admission".to_owned()],
            },
        ]);
        let continuation =
            daemon_continuation_for_runtime_input(AgentClosedLoopRuntimeTurnInput::new(
                repair_history,
                queue(),
                budget(),
                AgentCycleEvidence::default(),
            ));
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-summary-repair-first-missing",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );

        let summary = request_record.summary();
        let history_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecorder::new().record(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                summary.clone(),
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
            );

        assert_eq!(engine.calls, 0);
        assert_eq!(memory.submitted.len(), 0);
        assert!(!request_record.is_executable());
        assert!(request_record.command_plan().is_none());
        assert_eq!(
            request_record.skipped_reasons(),
            &["repair_first_task_missing".to_owned()]
        );
        assert!(!summary.executable);
        assert_eq!(summary.command_count, 0);
        assert!(!summary.command_gate_allowed);
        assert_eq!(summary.side_effect_gate_count, 0);
        assert_eq!(summary.blocked_side_effect_gate_count, 0);
        assert_eq!(summary.skipped_reasons, vec!["repair_first_task_missing"]);
        assert_eq!(history_record.dashboard.skipped_records, 1);
        assert_eq!(
            history_record.dashboard.latest_skipped_reasons,
            vec!["repair_first_task_missing"]
        );
        assert_eq!(
            history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(
            history_record
                .health
                .reasons
                .iter()
                .any(|reason| reason == "service_loop_run_daemon_request_skipped_records=1>0")
        );
        assert!(history_record.health.reasons.iter().any(|reason| {
            reason == "service_loop_run_daemon_request_latest_skipped=repair_first_task_missing"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_close_records_clean_request_and_daemon() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation = daemon_continuation_for_runtime_input(clean_runtime_input());
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-clean",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );
        let receipts = request_record
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();

        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                receipts,
            );

        assert_eq!(engine.calls, 1);
        assert_eq!(memory.submitted.len(), 1);
        assert!(close_record.latest_request_summary().executable);
        assert_eq!(
            close_record.request_history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            close_record.daemon_record.loop_run.run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert_eq!(close_record.mode(), AgentClosedLoopNextTurnMode::Continue);
        assert!(close_record.allows_service_advance());
        assert_eq!(
            close_record
                .daemon_record
                .control_summary_history_record
                .health
                .status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        let summary = close_record.summary();
        assert!(summary.latest_request_executable);
        assert_eq!(
            summary.latest_request_command_count,
            close_record.latest_request_summary().command_count
        );
        assert_eq!(
            summary.latest_request_command_gate_allowed,
            close_record.latest_request_summary().command_gate_allowed
        );
        assert_eq!(
            summary.latest_request_side_effect_gate_count,
            close_record.latest_request_summary().side_effect_gate_count
        );
        assert_eq!(summary.latest_request_blocked_side_effect_gate_count, 0);
        assert_eq!(
            summary.request_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            summary.daemon_run_status,
            AgentClosedLoopRuntimeServiceRunStatus::Closed
        );
        assert_eq!(
            summary.daemon_control_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(summary.mode, AgentClosedLoopNextTurnMode::Continue);
        assert!(summary.can_schedule);
        assert_eq!(
            summary.side_effect_dispatch_allowed_rate,
            close_record
                .latest_request_summary()
                .side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            summary.memory_note_allowed_rate,
            close_record
                .latest_request_summary()
                .memory_note_allowed_rate
        );
        assert!(summary.allows_adaptive_evolution);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.request_records, 1);
        assert_eq!(
            summary.service_attempts,
            close_record
                .daemon_record
                .loop_run
                .advance
                .run_record
                .attempts()
        );
        assert_eq!(
            summary.transitions,
            close_record.daemon_record.control_record.transitions()
        );
        assert_eq!(
            summary.control_records,
            close_record
                .daemon_record
                .control_summary_history_record
                .records()
        );
        assert!(summary.blocked_reasons.is_empty());
        assert!(summary.skipped_reasons.is_empty());
        assert!(summary.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_close_summary_request_health=stable"
        }));
        assert!(summary.telemetry.iter().any(|line| {
            line
                == "service_loop_run_daemon_request_monitored_close_summary_request_command_gate_allowed=true"
        }));
        let dispatch_rate_line = format!(
            "service_loop_run_daemon_request_monitored_close_summary_side_effect_dispatch_allowed_rate={:.3}",
            close_record
                .latest_request_summary()
                .side_effect_dispatch_allowed_rate
        );
        let memory_rate_line = format!(
            "service_loop_run_daemon_request_monitored_close_summary_memory_note_allowed_rate={:.3}",
            close_record
                .latest_request_summary()
                .memory_note_allowed_rate
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == &dispatch_rate_line })
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == &memory_rate_line })
        );
        assert!(close_record.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_close_request_health=stable"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_close_surfaces_skipped_request() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation =
            daemon_continuation_for_runtime_input(AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ));
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );

        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                Vec::new(),
            );

        assert_eq!(engine.calls, 0);
        assert_eq!(memory.submitted.len(), 0);
        assert!(!close_record.latest_request_summary().executable);
        assert_eq!(
            close_record.latest_request_summary().skipped_reasons,
            vec!["next_queue_empty"]
        );
        assert_eq!(
            close_record.request_history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            close_record.daemon_record.loop_run.run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::DispatchBlocked
        );
        assert_eq!(close_record.mode(), AgentClosedLoopNextTurnMode::Repair);
        assert!(close_record.requires_repair_first());
        assert!(!close_record.allows_service_advance());
        let summary = close_record.summary();
        assert!(!summary.latest_request_executable);
        assert!(!summary.latest_request_command_gate_allowed);
        assert_eq!(summary.latest_request_side_effect_gate_count, 0);
        assert_eq!(summary.latest_request_blocked_side_effect_gate_count, 0);
        assert_eq!(
            summary.request_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            summary.daemon_run_status,
            AgentClosedLoopRuntimeServiceRunStatus::DispatchBlocked
        );
        assert_eq!(
            summary.daemon_control_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(summary.mode, AgentClosedLoopNextTurnMode::Repair);
        assert!(summary.can_schedule);
        assert_eq!(summary.side_effect_dispatch_allowed_rate, 0.0);
        assert_eq!(summary.memory_note_allowed_rate, 0.0);
        assert!(!summary.allows_adaptive_evolution);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.request_records, 1);
        assert_eq!(
            summary.service_attempts,
            close_record
                .daemon_record
                .loop_run
                .advance
                .run_record
                .attempts()
        );
        assert_eq!(
            summary.transitions,
            close_record.daemon_record.control_record.transitions()
        );
        assert_eq!(
            summary.control_records,
            close_record
                .daemon_record
                .control_summary_history_record
                .records()
        );
        assert_eq!(summary.skipped_reasons, vec!["next_queue_empty"]);
        assert!(summary.blocked_reasons.iter().any(|reason| {
            reason.contains("service_loop_run_dispatch_blocked_rate")
                || reason == "next_queue_empty"
        }));
        assert!(summary.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_close_summary_requires_repair_first=true"
        }));
        assert!(close_record.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_close_request_health=repair"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_close_summary_history_marks_clean_stable()
    {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation = daemon_continuation_for_runtime_input(clean_runtime_input());
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-close-history-clean",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );
        let receipts = request_record
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                receipts,
            );

        let history_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder::new()
                .record(
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory::new(),
                    close_record.summary(),
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy::default(),
                );

        assert_eq!(history_record.records(), 1);
        assert!(history_record.latest().latest_request_executable);
        assert_eq!(history_record.dashboard.total_records, 1);
        assert_eq!(history_record.dashboard.request_executable_records, 1);
        assert_eq!(
            history_record
                .dashboard
                .request_command_gate_allowed_records,
            1
        );
        assert_eq!(history_record.dashboard.daemon_closed_records, 1);
        assert_eq!(
            history_record.dashboard.request_command_gate_allowed_rate,
            1.0
        );
        assert_eq!(
            history_record.dashboard.request_side_effect_gate_count,
            history_record
                .latest()
                .latest_request_side_effect_gate_count
        );
        assert_eq!(
            history_record
                .dashboard
                .request_blocked_side_effect_gate_count,
            0
        );
        assert_eq!(
            history_record.dashboard.side_effect_dispatch_allowed_rate,
            history_record.latest().side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            history_record.dashboard.memory_note_allowed_rate,
            history_record.latest().memory_note_allowed_rate
        );
        assert_eq!(
            history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert!(history_record.health.is_stable());
        assert!(history_record.allows_service_advance());
        assert!(!history_record.requires_repair_first());
        assert!(history_record.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_close_summary_history_record_health=stable"
        }));
        assert!(history_record.telemetry.iter().any(|line| {
            line
                == "service_loop_run_daemon_request_monitored_close_summary_history_record_request_command_gate_allowed_rate=1.000"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_close_summary_history_repairs_skips() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation =
            daemon_continuation_for_runtime_input(AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ));
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-close-history-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );
        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                Vec::new(),
            );

        let history_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder::new()
                .record(
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory::new(),
                    close_record.summary(),
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy::default(),
                );

        assert_eq!(history_record.records(), 1);
        assert!(!history_record.latest().latest_request_executable);
        assert_eq!(history_record.dashboard.request_executable_records, 0);
        assert_eq!(
            history_record
                .dashboard
                .request_command_gate_allowed_records,
            0
        );
        assert_eq!(history_record.dashboard.daemon_closed_records, 0);
        assert_eq!(history_record.dashboard.request_repair_records, 1);
        assert_eq!(history_record.dashboard.daemon_control_repair_records, 1);
        assert_eq!(history_record.dashboard.repair_first_records, 1);
        assert_eq!(
            history_record.dashboard.side_effect_dispatch_allowed_rate,
            0.0
        );
        assert_eq!(
            history_record.dashboard.request_command_gate_allowed_rate,
            0.0
        );
        assert_eq!(history_record.dashboard.request_side_effect_gate_count, 0);
        assert_eq!(
            history_record
                .dashboard
                .request_blocked_side_effect_gate_count,
            0
        );
        assert_eq!(history_record.dashboard.memory_note_allowed_rate, 0.0);
        assert_eq!(
            history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!history_record.allows_service_advance());
        assert!(history_record.requires_repair_first());
        assert!(history_record.health.reasons.iter().any(|reason| {
            reason == "service_loop_run_daemon_request_monitored_close_request_repair_records=1>0"
        }));
        assert!(history_record.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_close_summary_history_record_health=repair"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_close_continuation_packages_clean_history()
    {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation = daemon_continuation_for_runtime_input(clean_runtime_input());
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-close-continuation-clean",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );
        let receipts = request_record
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                receipts,
            );
        let close_history_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder::new()
                .record(
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory::new(),
                    close_record.summary(),
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy::default(),
                );

        let close_continuation =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuationPlanner::new()
                .plan(&close_record, &close_history_record);

        assert_eq!(
            close_continuation.monitored_continuation,
            close_record.continuation()
        );
        assert_eq!(
            close_continuation.monitored_close_summary_history,
            close_history_record.history
        );
        assert_eq!(
            close_continuation.monitored_close_health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            close_continuation.request_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            close_continuation.daemon_control_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            close_continuation.mode,
            AgentClosedLoopNextTurnMode::Continue
        );
        assert!(close_continuation.can_schedule);
        assert_eq!(
            close_continuation.side_effect_dispatch_allowed_rate,
            close_continuation
                .monitored_continuation
                .side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            close_continuation.memory_note_allowed_rate,
            close_continuation
                .monitored_continuation
                .memory_note_allowed_rate
        );
        assert!(close_continuation.allows_adaptive_evolution);
        assert!(!close_continuation.requires_repair_first);
        assert!(close_continuation.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_close_continuation_close_health=stable"
        }));
        let dispatch_rate_line = format!(
            "service_loop_run_daemon_request_monitored_close_continuation_side_effect_dispatch_allowed_rate={:.3}",
            close_continuation.side_effect_dispatch_allowed_rate
        );
        assert!(
            close_continuation
                .telemetry
                .iter()
                .any(|line| { line == &dispatch_rate_line })
        );
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_close_continuation_preserves_repair_health()
     {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation =
            daemon_continuation_for_runtime_input(AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ));
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-close-continuation-repair",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );
        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                Vec::new(),
            );
        let close_history_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder::new()
                .record(
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory::new(),
                    close_record.summary(),
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy::default(),
                );

        let close_continuation =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation::from_records(
                &close_record,
                &close_history_record,
            );

        assert_eq!(close_continuation.monitored_close_summary_history.len(), 1);
        assert_eq!(
            close_continuation.monitored_close_health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            close_continuation.request_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            close_continuation.daemon_control_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(close_continuation.mode, AgentClosedLoopNextTurnMode::Repair);
        assert!(close_continuation.can_schedule);
        assert_eq!(
            close_continuation.side_effect_dispatch_allowed_rate,
            close_continuation
                .monitored_continuation
                .side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            close_continuation.memory_note_allowed_rate,
            close_continuation
                .monitored_continuation
                .memory_note_allowed_rate
        );
        assert!(!close_continuation.allows_adaptive_evolution);
        assert!(close_continuation.requires_repair_first);
        assert!(close_continuation.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_close_continuation_close_health=repair"
        }));
        assert!(close_continuation.telemetry.iter().any(|line| {
            line.contains(
                "service_loop_run_daemon_request_monitored_close_continuation_reason=service_loop_run_daemon_request_monitored_close_request_repair_records=1>0",
            )
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_continuation_packages_clean_state() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation = daemon_continuation_for_runtime_input(clean_runtime_input());
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-continuation-clean",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );
        let receipts = request_record
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                receipts,
            );

        let monitored_continuation =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuationPlanner::new()
                .plan(&close_record);

        assert_eq!(
            monitored_continuation.request_summary_history,
            close_record.request_history_record.history
        );
        assert_eq!(
            monitored_continuation.request_health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            monitored_continuation.daemon_control_health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            monitored_continuation.daemon_continuation,
            close_record.daemon_record.continuation()
        );
        assert_eq!(
            monitored_continuation.mode,
            AgentClosedLoopNextTurnMode::Continue
        );
        assert!(monitored_continuation.can_schedule);
        assert_eq!(
            monitored_continuation.side_effect_dispatch_allowed_rate,
            monitored_continuation
                .daemon_continuation
                .side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            monitored_continuation.memory_note_allowed_rate,
            monitored_continuation
                .daemon_continuation
                .memory_note_allowed_rate
        );
        assert!(monitored_continuation.allows_adaptive_evolution);
        assert!(!monitored_continuation.requires_repair_first);
        assert!(monitored_continuation.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_continuation_request_health=stable"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_plan_preserves_request_history() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation = daemon_continuation_for_runtime_input(clean_runtime_input());
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-plan-first",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );
        let receipts = request_record
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                receipts,
            );
        let monitored_continuation = close_record.continuation();

        let monitored_plan =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlanner::new().plan(
                &monitored_continuation,
                AgentClosedLoopRuntimeBusinessInput::new(
                    "run-service-loop-daemon-request-monitored-plan-next",
                    crate::ledger::AgentCycleLedger::new(),
                    report_evidence(),
                ),
            );

        assert_eq!(monitored_plan.request_history().len(), 1);
        assert_eq!(
            monitored_plan.request_summary_history,
            monitored_continuation.request_summary_history
        );
        assert_eq!(
            monitored_plan.request_health,
            monitored_continuation.request_health
        );
        assert_eq!(
            monitored_plan.daemon_control_health,
            monitored_continuation.daemon_control_health
        );
        assert_eq!(
            monitored_plan.request_plan.control_summary_history,
            monitored_continuation
                .daemon_continuation
                .control_summary_history
        );
        assert_eq!(
            monitored_plan.side_effect_dispatch_allowed_rate,
            monitored_continuation.side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            monitored_plan.memory_note_allowed_rate,
            monitored_continuation.memory_note_allowed_rate
        );
        assert!(
            monitored_plan
                .telemetry
                .iter()
                .any(|line| { line == "service_loop_run_daemon_request_monitored_plan=true" })
        );
        let dispatch_rate_line = format!(
            "service_loop_run_daemon_request_monitored_plan_side_effect_dispatch_allowed_rate={:.3}",
            monitored_continuation.side_effect_dispatch_allowed_rate
        );
        assert!(
            monitored_plan
                .telemetry
                .iter()
                .any(|line| { line == &dispatch_rate_line })
        );

        let (request_history, request_plan) = monitored_plan.into_request_parts();
        let next_request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new()
            .run(request_plan, &mut engine, &mut memory);
        let receipts = next_request_record
            .command_plan()
            .map(|command_plan| {
                command_plan
                    .commands
                    .iter()
                    .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let next_close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                request_history,
                next_request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                receipts,
            );

        assert_eq!(next_close_record.request_history_record.records(), 2);
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_close_plan_preserves_histories() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation = daemon_continuation_for_runtime_input(clean_runtime_input());
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-close-plan-first",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );
        let receipts = request_record
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                receipts,
            );
        let close_history_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder::new()
                .record(
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory::new(),
                    close_record.summary(),
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy::default(),
                );
        let close_continuation =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuationPlanner::new()
                .plan(&close_record, &close_history_record);

        let close_plan =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlanner::new().plan(
                &close_continuation,
                AgentClosedLoopRuntimeBusinessInput::new(
                    "run-service-loop-daemon-request-monitored-close-plan-next",
                    crate::ledger::AgentCycleLedger::new(),
                    report_evidence(),
                ),
            );

        assert_eq!(close_plan.request_history().len(), 1);
        assert_eq!(close_plan.monitored_close_history().len(), 1);
        assert_eq!(
            close_plan.monitored_plan.request_summary_history,
            close_continuation
                .monitored_continuation
                .request_summary_history
        );
        assert_eq!(
            close_plan.monitored_close_summary_history,
            close_continuation.monitored_close_summary_history
        );
        assert_eq!(
            close_plan.monitored_close_health.status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            close_plan.request_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(
            close_plan.daemon_control_health_status,
            AgentClosedLoopExecutionHealthStatus::Stable
        );
        assert_eq!(close_plan.mode, AgentClosedLoopNextTurnMode::Continue);
        assert!(close_plan.can_schedule);
        assert_eq!(
            close_plan.side_effect_dispatch_allowed_rate,
            close_continuation.side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            close_plan.memory_note_allowed_rate,
            close_continuation.memory_note_allowed_rate
        );
        assert!(close_plan.allows_adaptive_evolution);
        assert!(!close_plan.requires_repair_first);
        assert!(
            close_plan.telemetry.iter().any(|line| {
                line == "service_loop_run_daemon_request_monitored_close_plan=true"
            })
        );

        let (close_history, monitored_plan) = close_plan.into_monitored_parts();

        assert_eq!(close_history.len(), 1);
        assert_eq!(monitored_plan.request_history().len(), 1);
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_close_runner_preserves_repair_trend() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation =
            daemon_continuation_for_runtime_input(AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ));
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-close-runner-first",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );
        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                Vec::new(),
            );
        let close_history_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder::new()
                .record(
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory::new(),
                    close_record.summary(),
                    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy::default(),
                );
        let close_continuation =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation::from_records(
                &close_record,
                &close_history_record,
            );
        let close_plan =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlanner::new().plan(
                &close_continuation,
                AgentClosedLoopRuntimeBusinessInput::new(
                    "run-service-loop-daemon-request-monitored-close-runner-next",
                    crate::ledger::AgentCycleLedger::new(),
                    report_evidence(),
                ),
            );

        let close_run_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunner::new().run(
                close_plan,
                &mut engine,
                &mut memory,
            );

        assert_eq!(close_run_record.request_history().len(), 1);
        assert_eq!(close_run_record.monitored_close_history().len(), 1);
        assert!(
            !close_run_record
                .monitored_close_plan
                .allows_adaptive_evolution
        );
        assert!(close_run_record.monitored_close_plan.requires_repair_first);
        assert!(close_run_record.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_close_run_record=true"
        }));
        let dispatch_rate_line = format!(
            "service_loop_run_daemon_request_monitored_close_run_record_side_effect_dispatch_allowed_rate={:.3}",
            close_run_record
                .monitored_close_plan
                .side_effect_dispatch_allowed_rate
        );
        assert!(
            close_run_record
                .telemetry
                .iter()
                .any(|line| { line == &dispatch_rate_line })
        );
        let close_run_summary = close_run_record.summary();
        let request_summary = close_run_record.monitored_record.summary();

        assert_eq!(
            close_run_summary.command_gate_allowed,
            request_summary.command_gate_allowed
        );
        assert_eq!(
            close_run_summary.side_effect_gate_count,
            request_summary.side_effect_gate_count
        );
        assert_eq!(
            close_run_summary.blocked_side_effect_gate_count,
            request_summary.blocked_side_effect_gate_count
        );
        assert_eq!(close_run_summary.request_records, 1);
        assert_eq!(close_run_summary.monitored_close_records, 1);
        assert_eq!(
            close_run_summary.monitored_close_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            close_run_summary.request_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            close_run_summary.daemon_control_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(!close_run_summary.allows_adaptive_evolution);
        assert!(close_run_summary.requires_repair_first);
        assert_eq!(
            close_run_summary.side_effect_dispatch_allowed_rate,
            close_run_record
                .monitored_close_plan
                .side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            close_run_summary.memory_note_allowed_rate,
            close_run_record
                .monitored_close_plan
                .memory_note_allowed_rate
        );
        assert!(close_run_summary.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_close_run_summary=true"
        }));
        let gate_line = format!(
            "service_loop_run_daemon_request_monitored_close_run_summary_command_gate_allowed={}",
            request_summary.command_gate_allowed
        );
        assert!(
            close_run_summary
                .telemetry
                .iter()
                .any(|line| { line == &gate_line })
        );
        let dispatch_rate_line = format!(
            "service_loop_run_daemon_request_monitored_close_run_summary_side_effect_dispatch_allowed_rate={:.3}",
            close_run_record
                .monitored_close_plan
                .side_effect_dispatch_allowed_rate
        );
        assert!(
            close_run_summary
                .telemetry
                .iter()
                .any(|line| { line == &dispatch_rate_line })
        );

        let next_close_continuation = close_run_record.close_with_receipts(
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealthPolicy::default(),
            Vec::new(),
        );

        assert_eq!(
            next_close_continuation
                .monitored_close_summary_history
                .len(),
            2
        );
        assert_eq!(
            next_close_continuation.monitored_close_health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            next_close_continuation.request_health_status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            next_close_continuation.side_effect_dispatch_allowed_rate,
            next_close_continuation
                .monitored_continuation
                .side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            next_close_continuation.memory_note_allowed_rate,
            next_close_continuation
                .monitored_continuation
                .memory_note_allowed_rate
        );
        assert!(!next_close_continuation.allows_adaptive_evolution);
        assert!(next_close_continuation.requires_repair_first);
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_runner_keeps_history_until_close() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation = daemon_continuation_for_runtime_input(clean_runtime_input());
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-runner-first",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );
        let receipts = request_record
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                receipts,
            );
        let monitored_plan =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlanner::new().plan(
                &close_record.continuation(),
                AgentClosedLoopRuntimeBusinessInput::new(
                    "run-service-loop-daemon-request-monitored-runner-next",
                    crate::ledger::AgentCycleLedger::new(),
                    report_evidence(),
                ),
            );

        let monitored_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRunner::new().run(
                monitored_plan,
                &mut engine,
                &mut memory,
            );

        assert_eq!(monitored_record.request_history().len(), 1);
        assert!(
            monitored_record
                .telemetry
                .iter()
                .any(|line| { line == "service_loop_run_daemon_request_monitored_record=true" })
        );
        let dispatch_rate_line = format!(
            "service_loop_run_daemon_request_monitored_record_side_effect_dispatch_allowed_rate={:.3}",
            monitored_record
                .monitored_plan
                .side_effect_dispatch_allowed_rate
        );
        assert!(
            monitored_record
                .telemetry
                .iter()
                .any(|line| { line == &dispatch_rate_line })
        );
        let receipts = monitored_record
            .command_plan()
            .map(|command_plan| {
                command_plan
                    .commands
                    .iter()
                    .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let next_close_record = monitored_record.close_with_receipts(
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
            receipts,
        );

        assert_eq!(next_close_record.request_history_record.records(), 2);
    }

    #[test]
    fn runtime_service_loop_run_daemon_request_monitored_continuation_preserves_repair_state() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let continuation =
            daemon_continuation_for_runtime_input(AgentClosedLoopRuntimeTurnInput::new(
                history(),
                AgentTaskQueue::new(),
                budget(),
                AgentCycleEvidence::default(),
            ));
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-request-monitored-continuation-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request_record = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner::new().run(
            request_plan,
            &mut engine,
            &mut memory,
        );
        let close_record =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser::new().close(
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new(),
                request_record,
                AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealthPolicy::default(),
                Vec::new(),
            );

        let monitored_continuation = close_record.continuation();

        assert_eq!(monitored_continuation.request_summary_history.len(), 1);
        assert_eq!(
            monitored_continuation.request_health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            monitored_continuation.daemon_control_health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert_eq!(
            monitored_continuation.mode,
            AgentClosedLoopNextTurnMode::Repair
        );
        assert!(monitored_continuation.can_schedule);
        assert!(!monitored_continuation.allows_adaptive_evolution);
        assert!(monitored_continuation.requires_repair_first);
        assert!(
            monitored_continuation
                .daemon_continuation
                .next_runtime_input
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id.starts_with("service-preflight-repair-"))
        );
        assert!(monitored_continuation.telemetry.iter().any(|line| {
            line == "service_loop_run_daemon_request_monitored_continuation_request_health=repair"
        }));
    }

    #[test]
    fn runtime_service_loop_run_daemon_input_plan_records_next_transition_gate() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let runner = AgentClosedLoopRuntimeServiceLoopRunDaemonRunner::new();
        let first_input = AgentClosedLoopRuntimeServiceLoopRunDaemonInput::new(
            service_loop_run_input_with_receipts(
                "run-service-loop-daemon-input-plan-chain-first",
                clean_service_loop_run_receipts(),
            ),
            AgentClosedLoopRuntimeServiceLoopRunHistory::from_summaries(vec![
                clean_service_loop_run_summary("run-service-loop-daemon-input-plan-chain-prior"),
            ]),
            AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::new(),
            AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default(),
        );
        let first_record = runner.run(first_input, &mut engine, &mut memory);
        let plan = AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner::new().plan(
            &first_record.continuation(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-loop-daemon-input-plan-chain-next",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            clean_service_loop_run_receipts(),
        );
        let continuation = first_record.continuation();
        assert_eq!(
            plan.side_effect_dispatch_allowed_rate,
            continuation.side_effect_dispatch_allowed_rate
        );
        assert_eq!(
            plan.memory_note_allowed_rate,
            continuation.memory_note_allowed_rate
        );

        let second_record = runner.run(plan.input, &mut engine, &mut memory);

        assert_eq!(engine.calls, 1);
        assert_eq!(memory.submitted.len(), 1);
        assert_eq!(
            second_record.loop_run.run.run_summary.status,
            AgentClosedLoopRuntimeServiceRunStatus::DispatchBlocked
        );
        assert_eq!(second_record.mode(), AgentClosedLoopNextTurnMode::Repair);
        assert_eq!(second_record.loop_run.advance.run_record.attempts(), 3);
        assert_eq!(second_record.control_record.transitions(), 3);
        assert_eq!(second_record.control_summary_history_record.records(), 2);
        assert_eq!(
            second_record.control_summary_history_record.health.status,
            AgentClosedLoopExecutionHealthStatus::Repair
        );
        assert!(
            second_record
                .loop_run
                .run
                .run_summary
                .blocked_reasons()
                .iter()
                .any(|reason| !reason.is_empty())
        );
        assert!(second_record.can_schedule());
        assert!(!second_record.allows_adaptive_evolution());
        assert!(second_record.requires_repair_first());
    }

    #[test]
    fn runtime_service_dispatch_intake_rejects_unexpected_receipts() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-intake-unexpected",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let dispatch = AgentClosedLoopRuntimeServiceRequestRunner::new().run_dispatch(
            input,
            &mut engine,
            &mut memory,
        );
        let mut receipts = dispatch
            .command_plan()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        receipts.push(AgentServiceCommandReceipt::new(
            "unexpected_command",
            AgentServiceCommandStatus::Applied,
            "rogue executor output",
        ));

        let dispatch_outcome = dispatch.close_with_intake(
            receipts,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        assert!(!dispatch_outcome.has_outcome());
        assert!(!dispatch_outcome.intake.can_close_outcome());
        assert_eq!(dispatch_outcome.intake.rejected_receipts.len(), 1);
        assert_eq!(
            dispatch_outcome.intake.rejected_receipts[0].reason,
            "receipt_command_unexpected_or_duplicate"
        );
        assert_eq!(
            dispatch_outcome.blocked_reasons(),
            [String::from(
                "service_receipt_rejected=unexpected_command:receipt_command_unexpected_or_duplicate"
            )]
            .as_slice()
        );
        assert_eq!(
            dispatch_outcome.repair_queue().task_ids(),
            vec!["service-intake-run-service-intake-unexpected-0-unexpected_command"]
        );
        assert_eq!(
            dispatch_outcome.repair_plan.tasks[0].role,
            AgentRole::Aggregator
        );
        assert!(
            dispatch_outcome
                .telemetry
                .iter()
                .any(|line| line == "service_dispatch_outcome=false")
        );
    }

    #[test]
    fn runtime_service_request_closes_receipts_into_outcome() {
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let mut memory = FakeMemory::default();
        let input = AgentClosedLoopRuntimeServiceRequestInput::new(
            clean_runtime_input(),
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-request-close",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
        );
        let request =
            AgentClosedLoopRuntimeServiceRequestRunner::new().run(input, &mut engine, &mut memory);
        let prior_history_len = request.prior_history.len();
        let receipts = request
            .command_request
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();

        let outcome = request.close_with_receipts(
            receipts,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        assert!(outcome.service_turn.summary.as_ref().unwrap().clean);
        assert_eq!(
            outcome.next_runtime_input().history.len(),
            prior_history_len + 1
        );
        assert_eq!(outcome.continuation.health.status.as_str(), "stable");
        assert!(
            outcome
                .telemetry
                .iter()
                .any(|line| line == "service_outcome_health=stable")
        );
    }

    #[test]
    fn runtime_service_turn_closer_records_clean_service_execution_and_history() {
        let business_turn = clean_business_turn();
        let step = business_turn.step().unwrap();
        let planned_next_queue_tasks = step.business_plan.next_queue.len();
        let command_plan = AgentServiceCommandPlanner::new().plan(&step.business_plan);
        let receipts = command_plan
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let history = history();

        let service_turn =
            AgentClosedLoopRuntimeServiceTurnCloser::new().close(business_turn, &history, receipts);

        assert!(service_turn.has_execution_report());
        assert!(service_turn.skipped_reasons.is_empty());
        assert_eq!(service_turn.updated_history.len(), history.len() + 1);
        assert!(service_turn.summary.as_ref().unwrap().clean);
        assert_eq!(service_turn.next_queue().len(), planned_next_queue_tasks);
        assert!(
            service_turn
                .telemetry
                .iter()
                .any(|line| line == "service_turn_execution=true")
        );
    }

    #[test]
    fn runtime_service_turn_closer_consumes_command_request_and_updates_history() {
        let request =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let receipts = request
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let history = history();

        let service_turn = AgentClosedLoopRuntimeServiceTurnCloser::new()
            .close_request(request, &history, receipts);

        assert!(service_turn.has_execution_report());
        assert_eq!(service_turn.updated_history.len(), history.len() + 1);
        assert!(service_turn.summary.as_ref().unwrap().clean);
        assert!(
            service_turn
                .telemetry
                .iter()
                .any(|line| line == "service_command_request=true")
        );
    }

    #[test]
    fn runtime_continuation_planner_builds_next_runtime_input_from_service_turn() {
        let request =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let receipts = request
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let history = history();
        let service_turn = AgentClosedLoopRuntimeServiceTurnCloser::new()
            .close_request(request, &history, receipts);
        let next_budget =
            BudgetLedger::new().with_budget(AgentRole::MemoryCurator, AgentBudget::new(16, 1, 1));

        let continuation = AgentClosedLoopRuntimeContinuationPlanner::new().plan(
            service_turn,
            AgentClosedLoopRuntimeContinuationInput::new(
                next_budget.clone(),
                AgentCycleEvidence::default(),
            )
            .with_max_parallel_tasks(3),
        );

        assert_eq!(continuation.dashboard.total_runs, history.len() + 1);
        assert_eq!(continuation.health.status.as_str(), "stable");
        assert_eq!(
            continuation.next_runtime_input.history.len(),
            continuation.dashboard.total_runs
        );
        assert_eq!(
            continuation.next_runtime_input.next_queue.len(),
            continuation.next_queue().len()
        );
        assert_eq!(
            continuation.next_queue().immediate_ready_tasks().len(),
            continuation
                .next_runtime_input
                .next_queue
                .immediate_ready_tasks()
                .len()
        );
        assert_eq!(continuation.next_runtime_input.max_parallel_tasks, 3);
        assert_eq!(continuation.next_runtime_input.budget_ledger, next_budget);
        assert!(
            continuation
                .telemetry
                .iter()
                .any(|line| line == "continuation_health=stable")
        );
    }

    #[test]
    fn runtime_continuation_planner_reflects_dirty_service_health() {
        let request =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let history = history();
        let service_turn = AgentClosedLoopRuntimeServiceTurnCloser::new().close_request(
            request,
            &history,
            Vec::new(),
        );

        let continuation = AgentClosedLoopRuntimeContinuationPlanner::new().plan(
            service_turn,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        assert_eq!(continuation.dashboard.service_dirty_runs, 1);
        assert_eq!(continuation.health.status.as_str(), "repair");
        assert!(
            continuation
                .health
                .reasons
                .iter()
                .any(|reason| reason.starts_with("service_failure_pressure="))
        );
        assert_eq!(
            continuation.next_runtime_input.next_queue.len(),
            continuation.service_turn.next_queue().len()
        );
        assert_eq!(
            continuation.next_queue().immediate_ready_tasks().len(),
            continuation
                .next_runtime_input
                .next_queue
                .immediate_ready_tasks()
                .len()
        );
    }

    #[test]
    fn runtime_continuation_planner_keeps_history_when_service_turn_is_skipped() {
        let input = AgentClosedLoopRuntimeTurnInput::new(
            history(),
            AgentTaskQueue::new(),
            budget(),
            AgentCycleEvidence::default(),
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);
        let mut memory = FakeMemory::default();
        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-continuation-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        );
        let history = history();
        let service_turn = AgentClosedLoopRuntimeServiceTurnCloser::new().close(
            business_turn,
            &history,
            Vec::new(),
        );

        let continuation = AgentClosedLoopRuntimeContinuationPlanner::new().plan(
            service_turn,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        assert_eq!(continuation.next_runtime_input.history, history);
        assert!(continuation.next_runtime_input.next_queue.is_empty());
        assert_eq!(continuation.dashboard.total_runs, 1);
        assert!(
            continuation
                .telemetry
                .iter()
                .any(|line| line == "continuation_next_queue_tasks=0")
        );
    }

    #[test]
    fn runtime_service_outcome_planner_closes_applied_receipts_into_continuation() {
        let request =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let receipts = request
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let history = history();

        let outcome = AgentClosedLoopRuntimeServiceOutcomePlanner::new().close(
            request,
            &history,
            receipts,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        assert!(outcome.service_turn.has_execution_report());
        assert!(outcome.service_turn.summary.as_ref().unwrap().clean);
        assert_eq!(outcome.continuation.dashboard.total_runs, history.len() + 1);
        assert_eq!(outcome.continuation.health.status.as_str(), "stable");
        assert_eq!(
            outcome.next_runtime_input().history.len(),
            outcome.continuation.dashboard.total_runs
        );
        assert!(
            outcome
                .telemetry
                .iter()
                .any(|line| line == "service_outcome_health=stable")
        );
    }

    #[test]
    fn runtime_service_outcome_summary_compacts_clean_outcome() {
        let request =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let receipts = request
            .command_plan
            .as_ref()
            .unwrap()
            .commands
            .iter()
            .map(|command| AgentServiceCommandReceipt::applied(command, "ok"))
            .collect::<Vec<_>>();
        let command_count = receipts.len();
        let history = history();
        let outcome = AgentClosedLoopRuntimeServiceOutcomePlanner::new().close(
            request,
            &history,
            receipts,
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        let summary = outcome.summary();

        assert_eq!(summary.runtime_mode, AgentClosedLoopNextTurnMode::Continue);
        assert_eq!(summary.command_count, command_count);
        assert!(summary.command_gate_allowed);
        assert_eq!(summary.side_effect_gate_count, command_count);
        assert_eq!(summary.blocked_side_effect_gate_count, 0);
        assert!(summary.command_gate_blocked_reasons.is_empty());
        assert!(summary.service_executed);
        assert!(summary.service_clean);
        assert_eq!(summary.health_status.as_str(), "stable");
        assert_eq!(summary.next_queue_tasks, outcome.next_queue().len());
        assert!(summary.skipped_reasons.is_empty());
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "service_outcome_summary_clean=true")
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "service_outcome_summary_command_gate_allowed=true")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == &format!("service_outcome_summary_side_effect_gates={command_count}")
        }));
    }

    #[test]
    fn runtime_service_outcome_planner_closes_missing_receipts_into_repair_continuation() {
        let request =
            AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(clean_business_turn());
        let history = history();

        let outcome = AgentClosedLoopRuntimeServiceOutcomePlanner::new().close(
            request,
            &history,
            Vec::new(),
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        assert!(outcome.service_turn.has_execution_report());
        assert!(!outcome.service_turn.summary.as_ref().unwrap().clean);
        assert_eq!(outcome.continuation.health.status.as_str(), "repair");
        assert_eq!(
            outcome.next_queue().len(),
            outcome.service_turn.next_queue().len()
        );
        assert!(
            outcome
                .telemetry
                .iter()
                .any(|line| line == "service_outcome_health=repair")
        );
    }

    #[test]
    fn runtime_service_outcome_summary_preserves_skipped_request_reason() {
        let input = AgentClosedLoopRuntimeTurnInput::new(
            history(),
            AgentTaskQueue::new(),
            budget(),
            AgentCycleEvidence::default(),
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);
        let mut memory = FakeMemory::default();
        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-outcome-summary-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        );
        let request = AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(business_turn);
        let history = history();
        let outcome = AgentClosedLoopRuntimeServiceOutcomePlanner::new().close(
            request,
            &history,
            Vec::new(),
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        let summary = outcome.summary();

        assert_eq!(summary.runtime_mode, AgentClosedLoopNextTurnMode::Idle);
        assert_eq!(summary.command_count, 0);
        assert!(!summary.command_gate_allowed);
        assert_eq!(summary.side_effect_gate_count, 0);
        assert_eq!(summary.blocked_side_effect_gate_count, 0);
        assert_eq!(
            summary.command_gate_blocked_reasons,
            vec!["next_queue_empty".to_owned()]
        );
        assert!(!summary.service_executed);
        assert!(!summary.service_clean);
        assert_eq!(summary.health_status.as_str(), "stable");
        assert_eq!(summary.next_queue_tasks, 0);
        assert_eq!(summary.skipped_reasons, vec!["next_queue_empty".to_owned()]);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "service_outcome_summary_skipped=next_queue_empty")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "service_outcome_summary_command_gate_blocked=next_queue_empty"
        }));
    }

    #[test]
    fn runtime_service_outcome_planner_preserves_skipped_request_without_history_append() {
        let input = AgentClosedLoopRuntimeTurnInput::new(
            history(),
            AgentTaskQueue::new(),
            budget(),
            AgentCycleEvidence::default(),
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);
        let mut memory = FakeMemory::default();
        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-outcome-skip",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        );
        let request = AgentClosedLoopRuntimeServiceCommandPlanner::new().plan(business_turn);
        let history = history();

        let outcome = AgentClosedLoopRuntimeServiceOutcomePlanner::new().close(
            request,
            &history,
            Vec::new(),
            AgentClosedLoopRuntimeContinuationInput::new(budget(), AgentCycleEvidence::default()),
        );

        assert!(!outcome.command_request.has_commands());
        assert!(!outcome.service_turn.has_execution_report());
        assert_eq!(outcome.next_runtime_input().history, history);
        assert!(outcome.next_queue().is_empty());
        assert!(
            outcome
                .telemetry
                .iter()
                .any(|line| line == "service_outcome_execution=false")
        );
    }

    #[test]
    fn runtime_service_turn_closer_turns_missing_receipts_into_history_and_queue() {
        let business_turn = clean_business_turn();
        let planned_next_queue_tasks = business_turn.step().unwrap().business_plan.next_queue.len();
        let history = history();

        let service_turn = AgentClosedLoopRuntimeServiceTurnCloser::new().close(
            business_turn,
            &history,
            Vec::new(),
        );

        assert!(service_turn.has_execution_report());
        assert_eq!(service_turn.updated_history.len(), history.len() + 1);
        let summary = service_turn.summary.as_ref().unwrap();
        assert!(!summary.clean);
        assert_eq!(summary.missing_command_count, summary.command_count);
        assert_eq!(
            summary.next_queue_tasks,
            summary.command_count + planned_next_queue_tasks
        );
        assert_eq!(service_turn.next_queue().len(), summary.next_queue_tasks);
        assert!(
            service_turn
                .telemetry
                .iter()
                .any(|line| line.starts_with("service_turn_missing_commands="))
        );
    }

    #[test]
    fn runtime_service_turn_closer_skips_business_turn_without_step() {
        let input = AgentClosedLoopRuntimeTurnInput::new(
            history(),
            AgentTaskQueue::new(),
            budget(),
            AgentCycleEvidence::default(),
        );
        let mut engine = CountingEngine {
            calls: 0,
            fail: false,
        };
        let runtime_turn = AgentClosedLoopRuntimeTurnRunner::new().run(input, &mut engine);
        let mut memory = FakeMemory::default();
        let business_turn = AgentClosedLoopRuntimeBusinessTurnCloser::new().close(
            runtime_turn,
            AgentClosedLoopRuntimeBusinessInput::new(
                "run-service-2",
                crate::ledger::AgentCycleLedger::new(),
                report_evidence(),
            ),
            &mut memory,
        );
        let history = history();

        let service_turn = AgentClosedLoopRuntimeServiceTurnCloser::new().close(
            business_turn,
            &history,
            Vec::new(),
        );

        assert!(!service_turn.has_execution_report());
        assert_eq!(service_turn.updated_history, history);
        assert_eq!(service_turn.skipped_reasons, vec!["next_queue_empty"]);
        assert!(
            service_turn
                .telemetry
                .iter()
                .any(|line| line == "service_turn_skipped=next_queue_empty")
        );
    }
}
