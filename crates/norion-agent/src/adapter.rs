use crate::aggregate::AggregationConflictReviewTrendGateDecision;
use crate::budget::{AgentBudget, BudgetLedgerHistoryGateRecord};
use crate::collaboration::AgentCollaborationAdapterSideEffectAdmission;
use crate::conflict::ConflictReportHistoryGateRecord;
use crate::cycle::AgentCycleReport;
use crate::eval::{
    AgentReportGateDecision, AgentReportGateHealthGateHealthPolicy,
    AgentReportGateHealthGateRecord, AgentReportGateHealthGateSummaryHistory,
    AgentReportGateHealthGateTrendHandoff, AgentReportGateHealthGateTrendHandoffHealthPolicy,
    AgentReportGateHealthGateTrendHandoffHistory, AgentReportGateHealthGateTrendHandoffMonitor,
    AgentReportGateHealthGateTrendHandoffMonitorHandoff,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoff,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoff,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord,
    AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory,
    AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
    AgentReportGateHealthGateTrendHandoffMonitorRecord,
    AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory,
    AgentReportGateHealthGateTrendHandoffRecord, AgentReportGateHealthPolicy,
    AgentReportGateHistoryRecord, AgentReportGateHistoryRecorder, AgentReportGateReason,
    AgentReportGateSummaryHistory,
};
use crate::evolution::{
    EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord, EvolutionAdmissionRecord,
    ProcessRewardReportHistoryGateRecord, ReflectionRewardAdmissionRecord,
    ToolsmithPlanHistoryGateRecord,
};
use crate::memory::{MemoryPromotionGateDecision, MemorySubmissionGateDecision};
use crate::ports::ToolBuildReportHistoryGateRecord;
use crate::reflection::ReflectionLoopHistoryGateRecord;
use crate::run::{
    AgentRunLedgerAdmission, AgentRunLedgerProgress,
    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision,
    AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord,
    AgentRunReportHealthStatus,
};
use crate::schedule::RecursiveAgentScheduleHistoryGateRecord;
use crate::service::{
    AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision,
    AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoffRecord,
};
use crate::step::{AgentClosedLoopExecutionHealthStatus, AgentClosedLoopNextTurnMode};
use crate::task::{AgentRole, AgentTask, AgentTaskQueue, TaskDispatchGateDecision};
use crate::turn::{
    AgentClosedLoopRuntimeServiceLoopAdvance, AgentClosedLoopRuntimeServiceLoopRunControlPlan,
    AgentClosedLoopRuntimeServiceLoopRunControlRecord,
    AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
    AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan,
    AgentClosedLoopRuntimeServiceLoopRunDaemonRecord,
    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation,
    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord,
    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord,
    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
    AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord,
    AgentClosedLoopRuntimeServiceLoopState, AgentClosedLoopRuntimeServicePreflight,
    AgentClosedLoopRuntimeServicePreflightContinuation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentAdapterBoundaryOwner {
    NorionCore,
    NorionMemory,
    ServiceAdapter,
    EvalReporting,
}

impl AgentAdapterBoundaryOwner {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NorionCore => "norion_core",
            Self::NorionMemory => "norion_memory",
            Self::ServiceAdapter => "service_adapter",
            Self::EvalReporting => "eval_reporting",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentAdapterBoundaryStatus {
    Stable,
    Watch,
    Repair,
}

impl AgentAdapterBoundaryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAdapterBoundaryGate {
    pub owner: AgentAdapterBoundaryOwner,
    pub status: AgentAdapterBoundaryStatus,
    pub dispatch_allowed: bool,
    pub memory_note_allowed: bool,
    pub adaptive_state_allowed: bool,
    pub service_command_allowed: bool,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub blocked_reasons: Vec<String>,
}

impl AgentAdapterBoundaryGate {
    pub fn stable(owner: AgentAdapterBoundaryOwner) -> Self {
        Self {
            owner,
            status: AgentAdapterBoundaryStatus::Stable,
            dispatch_allowed: true,
            memory_note_allowed: true,
            adaptive_state_allowed: true,
            service_command_allowed: true,
            service_execution_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_closes: 0,
            service_execution_tool_build_command_reason_count: 0,
            blocked_reasons: Vec::new(),
        }
    }

    pub fn watch(owner: AgentAdapterBoundaryOwner, reason: impl Into<String>) -> Self {
        Self {
            owner,
            status: AgentAdapterBoundaryStatus::Watch,
            dispatch_allowed: true,
            memory_note_allowed: false,
            adaptive_state_allowed: false,
            service_command_allowed: true,
            service_execution_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_closes: 0,
            service_execution_tool_build_command_reason_count: 0,
            blocked_reasons: vec![reason.into()],
        }
    }

    pub fn repair(owner: AgentAdapterBoundaryOwner, reason: impl Into<String>) -> Self {
        Self {
            owner,
            status: AgentAdapterBoundaryStatus::Repair,
            dispatch_allowed: false,
            memory_note_allowed: false,
            adaptive_state_allowed: false,
            service_command_allowed: false,
            service_execution_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_closes: 0,
            service_execution_tool_build_command_reason_count: 0,
            blocked_reasons: vec![reason.into()],
        }
    }

    pub fn with_dispatch_allowed(mut self, allowed: bool) -> Self {
        self.dispatch_allowed = allowed;
        self
    }

    pub fn with_memory_note_allowed(mut self, allowed: bool) -> Self {
        self.memory_note_allowed = allowed;
        self
    }

    pub fn with_adaptive_state_allowed(mut self, allowed: bool) -> Self {
        self.adaptive_state_allowed = allowed;
        self
    }

    pub fn with_service_command_allowed(mut self, allowed: bool) -> Self {
        self.service_command_allowed = allowed;
        self
    }

    pub fn with_blocked_reason(mut self, reason: impl Into<String>) -> Self {
        self.blocked_reasons.push(reason.into());
        self
    }

    pub fn from_dispatch_gate(decision: &TaskDispatchGateDecision) -> Self {
        let can_dispatch = decision.is_dispatchable();
        let can_promote_side_effects = decision.is_side_effect_safe();
        let mut gate = if decision.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::NorionCore,
                first_reason_or(&decision.reasons, "dispatch_requires_repair_first"),
            )
        } else if can_dispatch && can_promote_side_effects {
            Self::stable(AgentAdapterBoundaryOwner::NorionCore)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::NorionCore,
                first_reason_or(&decision.reasons, "dispatch_observe_only"),
            )
        }
        .with_dispatch_allowed(can_dispatch)
        .with_service_command_allowed(can_dispatch)
        .with_memory_note_allowed(can_promote_side_effects)
        .with_adaptive_state_allowed(can_promote_side_effects);

        extend_ordered_unique(&mut gate.blocked_reasons, decision.reasons.clone());
        gate
    }

    pub fn from_run_progress(progress: &AgentRunLedgerProgress) -> Self {
        if progress.can_close_run {
            return Self::stable(AgentAdapterBoundaryOwner::NorionCore);
        }

        let reasons = adapter_run_progress_reasons(progress);
        let mut gate = Self::repair(
            AgentAdapterBoundaryOwner::NorionCore,
            first_reason_or(&reasons, "run_progress_requires_repair_first"),
        );
        gate.blocked_reasons = reasons;
        gate
    }

    pub fn from_run_ledger_admission(admission: &AgentRunLedgerAdmission) -> Self {
        let reasons = adapter_run_ledger_admission_reasons(admission);
        let mut gate = if admission.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::NorionCore,
                first_reason_or(&reasons, "run_ledger_admission_requires_repair_first"),
            )
        } else if admission.is_admitted() {
            Self::stable(AgentAdapterBoundaryOwner::NorionCore)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::NorionCore,
                first_reason_or(&reasons, "run_ledger_admission_observe_only"),
            )
        }
        .with_dispatch_allowed(admission.can_build_ledger)
        .with_service_command_allowed(admission.can_admit_side_effects)
        .with_memory_note_allowed(admission.can_submit_memory_note)
        .with_adaptive_state_allowed(admission.can_promote_adaptive_state);

        gate.blocked_reasons = reasons;
        gate
    }

    pub fn from_dispatch_and_run_progress(
        dispatch: &TaskDispatchGateDecision,
        progress: &AgentRunLedgerProgress,
    ) -> Self {
        let mut gate = Self::from_dispatch_gate(dispatch);
        extend_ordered_unique(&mut gate.blocked_reasons, dispatch.reasons.clone());
        let progress_gate = Self::from_run_progress(progress);

        if progress_gate.status == AgentAdapterBoundaryStatus::Repair {
            gate.status = AgentAdapterBoundaryStatus::Repair;
            gate.dispatch_allowed = false;
            gate.memory_note_allowed = false;
            gate.adaptive_state_allowed = false;
            gate.service_command_allowed = false;
        }
        extend_ordered_unique(&mut gate.blocked_reasons, progress_gate.blocked_reasons);
        gate
    }

    pub fn from_dispatch_and_run_ledger_admission(
        dispatch: &TaskDispatchGateDecision,
        admission: &AgentRunLedgerAdmission,
    ) -> Self {
        let mut gate = Self::from_dispatch_gate(dispatch);
        extend_ordered_unique(&mut gate.blocked_reasons, dispatch.reasons.clone());
        let admission_gate = Self::from_run_ledger_admission(admission);

        if admission_gate.status == AgentAdapterBoundaryStatus::Repair {
            gate.status = AgentAdapterBoundaryStatus::Repair;
        } else if admission_gate.status == AgentAdapterBoundaryStatus::Watch
            && gate.status == AgentAdapterBoundaryStatus::Stable
        {
            gate.status = AgentAdapterBoundaryStatus::Watch;
        }

        gate.dispatch_allowed &= admission.can_build_ledger;
        gate.service_command_allowed &= admission.can_admit_side_effects;
        gate.memory_note_allowed &= admission.can_submit_memory_note;
        gate.adaptive_state_allowed &= admission.can_promote_adaptive_state;

        if admission.requires_repair_first {
            gate.dispatch_allowed = false;
            gate.service_command_allowed = false;
            gate.memory_note_allowed = false;
            gate.adaptive_state_allowed = false;
        }

        extend_ordered_unique(&mut gate.blocked_reasons, admission_gate.blocked_reasons);
        gate
    }

    pub fn from_memory_submission_gate(decision: &MemorySubmissionGateDecision) -> Self {
        if decision.requires_repair_first {
            return Self::repair(
                AgentAdapterBoundaryOwner::NorionMemory,
                first_reason_or(&decision.reasons, "memory_submission_requires_repair_first"),
            );
        }

        let mut gate = if decision.can_commit_submitted_notes {
            Self::stable(AgentAdapterBoundaryOwner::NorionMemory)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::NorionMemory,
                first_reason_or(&decision.reasons, "memory_submission_observe_only"),
            )
        };
        gate.memory_note_allowed = decision.can_commit_submitted_notes;
        gate.adaptive_state_allowed = decision.can_commit_submitted_notes;
        gate
    }

    pub fn from_memory_promotion_gate(decision: &MemoryPromotionGateDecision) -> Self {
        if decision.requires_repair_first {
            return Self::repair(
                AgentAdapterBoundaryOwner::NorionMemory,
                first_reason_or(&decision.reasons, "memory_promotion_requires_repair_first"),
            );
        }

        let mut gate = if decision.can_submit_memory {
            Self::stable(AgentAdapterBoundaryOwner::NorionMemory)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::NorionMemory,
                first_reason_or(&decision.reasons, "memory_promotion_observe_only"),
            )
        };
        gate.memory_note_allowed = decision.can_submit_memory;
        gate.adaptive_state_allowed = decision.can_submit_memory;
        gate
    }

    pub fn from_report_gate(decision: &AgentReportGateDecision) -> Self {
        if decision.is_accepted() {
            Self::stable(AgentAdapterBoundaryOwner::EvalReporting)
        } else {
            let reasons = decision
                .reasons
                .iter()
                .map(|reason| reason.as_line())
                .collect::<Vec<_>>();
            Self::repair(
                AgentAdapterBoundaryOwner::EvalReporting,
                first_reason_or(&reasons, "report_gate_rejected"),
            )
        }
    }

    pub fn from_conflict_report_history_gate(record: &ConflictReportHistoryGateRecord) -> Self {
        let decision = &record.gate_decision;
        let mut gate = if decision.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::EvalReporting,
                first_reason_or(
                    &decision.reasons,
                    "conflict_report_history_requires_repair_first",
                ),
            )
        } else if decision.can_forward_report && decision.can_promote_side_effects {
            Self::stable(AgentAdapterBoundaryOwner::EvalReporting)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::EvalReporting,
                first_reason_or(&decision.reasons, "conflict_report_history_observe"),
            )
        }
        .with_dispatch_allowed(decision.can_forward_report)
        .with_service_command_allowed(decision.can_forward_report)
        .with_memory_note_allowed(decision.can_promote_side_effects)
        .with_adaptive_state_allowed(decision.can_promote_side_effects);

        extend_ordered_unique(&mut gate.blocked_reasons, decision.reasons.clone());
        gate
    }

    pub fn from_aggregation_conflict_review_trend_gate(
        decision: &AggregationConflictReviewTrendGateDecision,
    ) -> Self {
        let mut gate = if decision.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::EvalReporting,
                first_reason_or(
                    &decision.reasons,
                    "aggregation_conflict_review_requires_repair_first",
                ),
            )
        } else if decision.can_forward_messages && decision.can_promote_side_effects {
            Self::stable(AgentAdapterBoundaryOwner::EvalReporting)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::EvalReporting,
                first_reason_or(&decision.reasons, "aggregation_conflict_review_observe"),
            )
        }
        .with_dispatch_allowed(decision.can_forward_messages)
        .with_service_command_allowed(decision.can_forward_messages)
        .with_memory_note_allowed(decision.can_promote_side_effects)
        .with_adaptive_state_allowed(decision.can_promote_side_effects);

        extend_ordered_unique(&mut gate.blocked_reasons, decision.reasons.clone());
        gate
    }

    pub fn from_service_admission(
        admission: &AgentCollaborationAdapterSideEffectAdmission,
    ) -> Self {
        let mut gate = if admission.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &admission.reasons,
                    "service_admission_requires_repair_first",
                ),
            )
        } else if admission.can_dispatch_service_commands
            && admission.can_promote_memory_note
            && admission.can_admit_adaptive_evolution
        {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(&admission.reasons, "service_admission_observe_only"),
            )
        };

        gate.dispatch_allowed = admission.can_dispatch_service_commands;
        gate.service_command_allowed = admission.can_dispatch_service_commands;
        gate.memory_note_allowed = admission.can_promote_memory_note;
        gate.adaptive_state_allowed = admission.can_admit_adaptive_evolution;
        gate.service_execution_command_reason_count =
            admission.service_execution_command_reason_count;
        gate.service_execution_memory_promotion_command_reason_count =
            admission.service_execution_memory_promotion_command_reason_count;
        gate.service_execution_memory_promotion_command_reason_closes =
            admission.service_execution_memory_promotion_command_reason_closes;
        gate.service_execution_tool_build_command_reason_count =
            admission.service_execution_tool_build_command_reason_count;
        gate
    }

    pub fn from_evolution_admission_handoff_history(
        record: &EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord,
    ) -> Self {
        let admission =
            AgentCollaborationAdapterSideEffectAdmission::from_evolution_admission_handoff_history(
                record,
            );

        Self::from_service_admission(&admission)
    }

    pub fn from_evolution_admission(record: &EvolutionAdmissionRecord) -> Self {
        let decision = &record.decision;
        let admitted = decision.is_admitted();
        let mut gate = if decision.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &decision.blocked_reasons,
                    "evolution_admission_requires_repair_first",
                ),
            )
        } else if admitted {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(&decision.blocked_reasons, "evolution_admission_observe"),
            )
        }
        .with_dispatch_allowed(admitted)
        .with_service_command_allowed(decision.can_promote_ready_proposals)
        .with_memory_note_allowed(false)
        .with_adaptive_state_allowed(decision.can_promote_adaptive_state);

        extend_ordered_unique(&mut gate.blocked_reasons, decision.blocked_reasons.clone());
        gate
    }

    pub fn from_budget_ledger_history_gate(record: &BudgetLedgerHistoryGateRecord) -> Self {
        let decision = &record.gate_decision;
        let can_dispatch = decision.is_dispatchable();
        let can_promote_side_effects = decision.is_side_effect_safe();
        let mut gate = if decision.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::NorionCore,
                first_reason_or(
                    &decision.reasons,
                    "budget_ledger_history_requires_repair_first",
                ),
            )
        } else if can_dispatch && can_promote_side_effects {
            Self::stable(AgentAdapterBoundaryOwner::NorionCore)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::NorionCore,
                first_reason_or(&decision.reasons, "budget_ledger_history_observe"),
            )
        }
        .with_dispatch_allowed(can_dispatch)
        .with_service_command_allowed(can_dispatch)
        .with_memory_note_allowed(can_promote_side_effects)
        .with_adaptive_state_allowed(can_promote_side_effects);

        extend_ordered_unique(&mut gate.blocked_reasons, decision.reasons.clone());
        gate
    }

    pub fn from_recursive_schedule_history_gate(
        record: &RecursiveAgentScheduleHistoryGateRecord,
    ) -> Self {
        let decision = &record.gate_decision;
        let can_dispatch = decision.is_dispatchable();
        let mut gate = if decision.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::NorionCore,
                first_reason_or(
                    &decision.reasons,
                    "recursive_schedule_history_requires_repair_first",
                ),
            )
        } else if can_dispatch {
            Self::stable(AgentAdapterBoundaryOwner::NorionCore)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::NorionCore,
                first_reason_or(&decision.reasons, "recursive_schedule_history_observe"),
            )
        }
        .with_dispatch_allowed(can_dispatch)
        .with_service_command_allowed(can_dispatch)
        .with_memory_note_allowed(can_dispatch)
        .with_adaptive_state_allowed(can_dispatch);

        extend_ordered_unique(&mut gate.blocked_reasons, decision.reasons.clone());
        gate
    }

    pub fn from_reflection_loop_history_gate(record: &ReflectionLoopHistoryGateRecord) -> Self {
        let decision = &record.gate_decision;
        let can_continue_or_promote =
            decision.can_continue_reflection || decision.can_promote_memory_note;
        let can_promote_memory_note = decision.is_memory_promotable();
        let mut gate = if decision.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::NorionMemory,
                first_reason_or(
                    &decision.reasons,
                    "reflection_loop_history_requires_repair_first",
                ),
            )
        } else if can_promote_memory_note {
            Self::stable(AgentAdapterBoundaryOwner::NorionMemory)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::NorionMemory,
                first_reason_or(&decision.reasons, "reflection_loop_history_observe"),
            )
        }
        .with_dispatch_allowed(can_continue_or_promote)
        .with_service_command_allowed(can_continue_or_promote)
        .with_memory_note_allowed(can_promote_memory_note)
        .with_adaptive_state_allowed(false);

        extend_ordered_unique(&mut gate.blocked_reasons, decision.reasons.clone());
        gate
    }

    pub fn from_tool_build_report_history_gate(record: &ToolBuildReportHistoryGateRecord) -> Self {
        let decision = &record.gate_decision;
        let mut gate = if decision.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &decision.reasons,
                    "tool_build_report_history_requires_repair_first",
                ),
            )
        } else if decision.can_open_tool_build_boundary {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                "tool_build_report_history_observe",
            )
        }
        .with_dispatch_allowed(decision.can_open_tool_build_boundary)
        .with_service_command_allowed(decision.can_open_tool_build_boundary)
        .with_memory_note_allowed(decision.can_promote_memory_note)
        .with_adaptive_state_allowed(decision.can_promote_adaptive_state);

        extend_ordered_unique(&mut gate.blocked_reasons, decision.reasons.clone());
        gate
    }

    pub fn from_toolsmith_plan_history_gate(record: &ToolsmithPlanHistoryGateRecord) -> Self {
        let decision = &record.gate_decision;
        let can_promote = decision.is_promotable();
        let mut gate = if decision.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &decision.reasons,
                    "toolsmith_plan_history_requires_repair_first",
                ),
            )
        } else if can_promote {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(&decision.reasons, "toolsmith_plan_history_observe"),
            )
        }
        .with_dispatch_allowed(can_promote)
        .with_service_command_allowed(can_promote)
        .with_memory_note_allowed(can_promote)
        .with_adaptive_state_allowed(can_promote);

        extend_ordered_unique(&mut gate.blocked_reasons, decision.reasons.clone());
        gate
    }

    pub fn from_process_reward_report_history_gate(
        record: &ProcessRewardReportHistoryGateRecord,
    ) -> Self {
        let decision = &record.gate_decision;
        let mut gate = if decision.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::EvalReporting,
                first_reason_or(
                    &decision.reasons,
                    "process_reward_report_history_requires_repair_first",
                ),
            )
        } else if decision.can_promote_evolution_signals {
            Self::stable(AgentAdapterBoundaryOwner::EvalReporting)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::EvalReporting,
                first_reason_or(&decision.reasons, "process_reward_report_history_observe"),
            )
        }
        .with_adaptive_state_allowed(decision.can_promote_evolution_signals);

        extend_ordered_unique(&mut gate.blocked_reasons, decision.reasons.clone());
        gate
    }

    pub fn from_reflection_reward_admission(record: &ReflectionRewardAdmissionRecord) -> Self {
        let mut gate = if record.requires_repair_first {
            Self::repair(
                AgentAdapterBoundaryOwner::EvalReporting,
                first_reason_or(
                    &record.blocked_reasons,
                    "reflection_reward_admission_requires_repair_first",
                ),
            )
        } else if record.is_admitted() {
            Self::stable(AgentAdapterBoundaryOwner::EvalReporting)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::EvalReporting,
                first_reason_or(
                    &record.blocked_reasons,
                    "reflection_reward_admission_observe",
                ),
            )
        }
        .with_dispatch_allowed(
            record.can_continue_reflection
                || record.can_promote_evolution_signals
                || record.can_reinforce_process,
        )
        .with_service_command_allowed(
            record.can_continue_reflection
                || record.can_promote_evolution_signals
                || record.can_reinforce_process,
        )
        .with_memory_note_allowed(record.can_promote_memory_note)
        .with_adaptive_state_allowed(record.can_promote_evolution_signals);

        extend_ordered_unique(&mut gate.blocked_reasons, record.blocked_reasons.clone());
        gate
    }

    pub fn from_run_report_final_gate_decision(
        decision: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision,
    ) -> Self {
        let mut reasons = decision.blocked_reasons.clone();
        extend_ordered_unique(&mut reasons, decision.admission_health.reasons.clone());

        let mut gate = if decision.requires_repair_first
            || decision.admission_health.status == AgentRunReportHealthStatus::Repair
        {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(&reasons, "run_report_final_requires_repair_first"),
            )
        } else if decision.is_admitted()
            && decision.admission_health.status == AgentRunReportHealthStatus::Stable
        {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(&reasons, "run_report_final_observe_only"),
            )
        };

        gate.blocked_reasons = reasons;
        gate
    }

    pub fn from_run_report_final_handoff(
        record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord,
    ) -> Self {
        Self::from_run_report_final_gate_decision(&record.gate_decision)
    }

    pub fn from_service_execution_final_gate_decision(
        decision: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision,
    ) -> Self {
        let mut reasons = decision.blocked_reasons.clone();
        extend_ordered_unique(&mut reasons, decision.handoff_health.reasons.clone());

        let mut gate = if decision.requires_repair_first
            || decision.handoff_health.status == AgentClosedLoopExecutionHealthStatus::Repair
        {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(&reasons, "service_execution_final_requires_repair_first"),
            )
        } else if decision.is_admitted()
            && decision.handoff_health.status == AgentClosedLoopExecutionHealthStatus::Stable
        {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(&reasons, "service_execution_final_observe_only"),
            )
        };

        gate.blocked_reasons = reasons;
        gate
    }

    pub fn from_service_execution_final_handoff(
        record: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoffRecord,
    ) -> Self {
        Self::from_service_execution_final_gate_decision(&record.gate_decision)
    }

    pub fn from_runtime_service_loop_control_plan(
        plan: &AgentClosedLoopRuntimeServiceLoopRunControlPlan,
    ) -> Self {
        let mut gate = if plan.requires_repair_first()
            || plan.health.status == AgentClosedLoopExecutionHealthStatus::Repair
        {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(&plan.reasons, "runtime_service_loop_requires_repair_first"),
            )
        } else if plan.mode == AgentClosedLoopNextTurnMode::Continue
            && plan.health.status == AgentClosedLoopExecutionHealthStatus::Stable
        {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(&plan.reasons, "runtime_service_loop_observe_only"),
            )
        };

        gate.dispatch_allowed = plan.can_schedule();
        gate.service_command_allowed = plan.can_schedule();
        gate.memory_note_allowed = plan.mode.allows_adaptive_evolution()
            && plan.health.status == AgentClosedLoopExecutionHealthStatus::Stable;
        gate.adaptive_state_allowed = plan.allows_adaptive_evolution()
            && plan.health.status == AgentClosedLoopExecutionHealthStatus::Stable;
        gate.blocked_reasons = plan.reasons.clone();
        gate
    }

    pub fn from_runtime_service_loop_control_record(
        record: &AgentClosedLoopRuntimeServiceLoopRunControlRecord,
    ) -> Self {
        Self::from_runtime_service_loop_control_plan(&record.control_plan)
    }

    pub fn from_runtime_service_preflight(
        preflight: &AgentClosedLoopRuntimeServicePreflight,
    ) -> Self {
        Self::from_service_admission(&preflight.side_effect_admission())
    }

    pub fn from_runtime_service_preflight_continuation(
        continuation: &AgentClosedLoopRuntimeServicePreflightContinuation,
    ) -> Self {
        Self::from_runtime_service_preflight(&continuation.preflight)
    }

    pub fn from_runtime_service_loop_state(state: &AgentClosedLoopRuntimeServiceLoopState) -> Self {
        let mut gate =
            Self::from_runtime_service_preflight_continuation(&state.preflight_continuation);
        extend_ordered_unique(
            &mut gate.blocked_reasons,
            state
                .preflight_continuation
                .preflight
                .side_effect_admission()
                .reasons,
        );
        gate
    }

    pub fn from_runtime_service_loop_advance(
        advance: &AgentClosedLoopRuntimeServiceLoopAdvance,
    ) -> Self {
        Self::from_runtime_service_loop_state(&advance.loop_state)
    }

    pub fn from_runtime_service_loop_daemon_record(
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRecord,
    ) -> Self {
        Self::from_runtime_service_loop_daemon_continuation(&record.continuation())
    }

    pub fn from_runtime_service_loop_daemon_continuation(
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
    ) -> Self {
        let reasons = runtime_service_loop_daemon_continuation_reasons(continuation);
        let is_repair = continuation.requires_repair_first
            || continuation.mode == AgentClosedLoopNextTurnMode::Repair
            || continuation.transition_health_status
                == AgentClosedLoopExecutionHealthStatus::Repair
            || continuation.control_health_status == AgentClosedLoopExecutionHealthStatus::Repair;
        let is_stable = continuation.mode == AgentClosedLoopNextTurnMode::Continue
            && continuation.transition_health_status
                == AgentClosedLoopExecutionHealthStatus::Stable
            && continuation.control_health_status == AgentClosedLoopExecutionHealthStatus::Stable
            && continuation.can_schedule
            && continuation.side_effect_dispatch_allowed_rate > 0.0
            && continuation.memory_note_allowed_rate > 0.0
            && continuation.allows_adaptive_evolution;

        let mut gate = if is_repair {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &reasons,
                    "runtime_service_loop_daemon_requires_repair_first",
                ),
            )
        } else if is_stable {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(&reasons, "runtime_service_loop_daemon_observe_only"),
            )
        };

        gate.dispatch_allowed = !is_repair
            && continuation.can_schedule
            && continuation.side_effect_dispatch_allowed_rate > 0.0;
        gate.service_command_allowed = gate.dispatch_allowed;
        gate.memory_note_allowed = is_stable
            && continuation.mode.allows_adaptive_evolution()
            && continuation.memory_note_allowed_rate > 0.0;
        gate.adaptive_state_allowed = is_stable && continuation.allows_adaptive_evolution;
        gate.blocked_reasons = reasons;
        gate
    }

    pub fn from_runtime_service_loop_daemon_input_plan(
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan,
    ) -> Self {
        let reasons = runtime_service_loop_daemon_input_plan_reasons(plan);
        let can_observe_service = !runtime_service_loop_daemon_input_plan_next_queue(plan)
            .is_empty()
            && plan.side_effect_dispatch_allowed_rate > 0.0;
        let mut gate = Self::watch(
            AgentAdapterBoundaryOwner::ServiceAdapter,
            first_reason_or(
                &reasons,
                "runtime_service_loop_daemon_input_plan_observe_only",
            ),
        );

        gate.dispatch_allowed = can_observe_service;
        gate.service_command_allowed = can_observe_service;
        gate.memory_note_allowed = false;
        gate.adaptive_state_allowed = false;
        gate.blocked_reasons = reasons;
        gate
    }

    pub fn from_runtime_service_loop_daemon_request_plan(
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
    ) -> Self {
        let reasons = runtime_service_loop_daemon_request_reasons(plan);
        let is_repair =
            plan.requires_repair_first || plan.mode == AgentClosedLoopNextTurnMode::Repair;
        let is_stable = plan.mode == AgentClosedLoopNextTurnMode::Continue
            && plan.can_schedule
            && plan.side_effect_dispatch_allowed_rate > 0.0
            && plan.memory_note_allowed_rate > 0.0
            && plan.allows_adaptive_evolution;

        let mut gate = if is_repair {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &reasons,
                    "runtime_service_loop_daemon_request_requires_repair_first",
                ),
            )
        } else if is_stable {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(&reasons, "runtime_service_loop_daemon_request_observe_only"),
            )
        };

        gate.dispatch_allowed =
            !is_repair && plan.can_schedule && plan.side_effect_dispatch_allowed_rate > 0.0;
        gate.service_command_allowed = gate.dispatch_allowed;
        gate.memory_note_allowed = is_stable
            && plan.mode.allows_adaptive_evolution()
            && plan.memory_note_allowed_rate > 0.0;
        gate.adaptive_state_allowed = is_stable && plan.allows_adaptive_evolution;
        gate.blocked_reasons = reasons;
        gate
    }

    pub fn from_runtime_service_loop_daemon_request_record(
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord,
    ) -> Self {
        Self::from_runtime_service_loop_daemon_request_plan(&record.request_plan)
    }

    pub fn from_runtime_service_loop_daemon_request_monitored_plan(
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
    ) -> Self {
        let reasons = runtime_service_loop_daemon_request_monitored_reasons(plan);
        let is_repair = plan.requires_repair_first
            || plan.request_health.status == AgentClosedLoopExecutionHealthStatus::Repair
            || plan.daemon_control_health.status == AgentClosedLoopExecutionHealthStatus::Repair;
        let is_stable = plan.mode == AgentClosedLoopNextTurnMode::Continue
            && plan.request_health.status == AgentClosedLoopExecutionHealthStatus::Stable
            && plan.daemon_control_health.status == AgentClosedLoopExecutionHealthStatus::Stable
            && plan.can_schedule
            && plan.side_effect_dispatch_allowed_rate > 0.0
            && plan.memory_note_allowed_rate > 0.0
            && plan.allows_adaptive_evolution;

        let mut gate = if is_repair {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &reasons,
                    "runtime_service_loop_daemon_request_monitored_requires_repair_first",
                ),
            )
        } else if is_stable {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &reasons,
                    "runtime_service_loop_daemon_request_monitored_observe_only",
                ),
            )
        };

        gate.dispatch_allowed =
            !is_repair && plan.can_schedule && plan.side_effect_dispatch_allowed_rate > 0.0;
        gate.service_command_allowed = gate.dispatch_allowed;
        gate.memory_note_allowed = is_stable
            && plan.mode.allows_adaptive_evolution()
            && plan.memory_note_allowed_rate > 0.0;
        gate.adaptive_state_allowed = is_stable && plan.allows_adaptive_evolution;
        gate.blocked_reasons = reasons;
        gate
    }

    pub fn from_runtime_service_loop_daemon_request_monitored_record(
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord,
    ) -> Self {
        Self::from_runtime_service_loop_daemon_request_monitored_plan(&record.monitored_plan)
    }

    pub fn from_runtime_service_loop_daemon_request_monitored_close_plan(
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
    ) -> Self {
        let reasons = runtime_service_loop_daemon_request_monitored_close_reasons(plan);
        let is_repair = plan.requires_repair_first
            || plan.monitored_close_health.status == AgentClosedLoopExecutionHealthStatus::Repair
            || plan.request_health_status == AgentClosedLoopExecutionHealthStatus::Repair
            || plan.daemon_control_health_status == AgentClosedLoopExecutionHealthStatus::Repair;
        let is_stable = plan.mode == AgentClosedLoopNextTurnMode::Continue
            && plan.monitored_close_health.status == AgentClosedLoopExecutionHealthStatus::Stable
            && plan.request_health_status == AgentClosedLoopExecutionHealthStatus::Stable
            && plan.daemon_control_health_status == AgentClosedLoopExecutionHealthStatus::Stable
            && plan.can_schedule
            && plan.side_effect_dispatch_allowed_rate > 0.0
            && plan.memory_note_allowed_rate > 0.0
            && plan.allows_adaptive_evolution;

        let mut gate = if is_repair {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &reasons,
                    "runtime_service_loop_daemon_request_monitored_close_requires_repair_first",
                ),
            )
        } else if is_stable {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &reasons,
                    "runtime_service_loop_daemon_request_monitored_close_observe_only",
                ),
            )
        };

        gate.dispatch_allowed =
            !is_repair && plan.can_schedule && plan.side_effect_dispatch_allowed_rate > 0.0;
        gate.service_command_allowed = gate.dispatch_allowed;
        gate.memory_note_allowed = is_stable
            && plan.mode.allows_adaptive_evolution()
            && plan.memory_note_allowed_rate > 0.0;
        gate.adaptive_state_allowed = is_stable && plan.allows_adaptive_evolution;
        gate.blocked_reasons = reasons;
        gate
    }

    pub fn from_runtime_service_loop_daemon_request_monitored_close_run_record(
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord,
    ) -> Self {
        Self::from_runtime_service_loop_daemon_request_monitored_close_plan(
            &record.monitored_close_plan,
        )
    }

    pub fn from_runtime_service_loop_daemon_request_monitored_close_continuation(
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation,
    ) -> Self {
        let reasons =
            runtime_service_loop_daemon_request_monitored_close_continuation_reasons(continuation);
        let is_repair = continuation.requires_repair_first
            || continuation.monitored_close_health.status
                == AgentClosedLoopExecutionHealthStatus::Repair
            || continuation.request_health_status == AgentClosedLoopExecutionHealthStatus::Repair
            || continuation.daemon_control_health_status
                == AgentClosedLoopExecutionHealthStatus::Repair;
        let is_stable = continuation.mode == AgentClosedLoopNextTurnMode::Continue
            && continuation.monitored_close_health.status
                == AgentClosedLoopExecutionHealthStatus::Stable
            && continuation.request_health_status == AgentClosedLoopExecutionHealthStatus::Stable
            && continuation.daemon_control_health_status
                == AgentClosedLoopExecutionHealthStatus::Stable
            && continuation.can_schedule
            && continuation.side_effect_dispatch_allowed_rate > 0.0
            && continuation.memory_note_allowed_rate > 0.0
            && continuation.allows_adaptive_evolution;

        let mut gate = if is_repair {
            Self::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &reasons,
                    "runtime_service_loop_daemon_request_monitored_close_continuation_requires_repair_first",
                ),
            )
        } else if is_stable {
            Self::stable(AgentAdapterBoundaryOwner::ServiceAdapter)
        } else {
            Self::watch(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                first_reason_or(
                    &reasons,
                    "runtime_service_loop_daemon_request_monitored_close_continuation_observe_only",
                ),
            )
        };

        gate.dispatch_allowed = !is_repair
            && continuation.can_schedule
            && continuation.side_effect_dispatch_allowed_rate > 0.0;
        gate.service_command_allowed = gate.dispatch_allowed;
        gate.memory_note_allowed = is_stable
            && continuation.mode.allows_adaptive_evolution()
            && continuation.memory_note_allowed_rate > 0.0;
        gate.adaptive_state_allowed = is_stable && continuation.allows_adaptive_evolution;
        gate.blocked_reasons = reasons;
        gate
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAdapterBoundarySnapshot {
    pub gates: Vec<AgentAdapterBoundaryGate>,
    pub next_queue_task_ids: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundarySnapshot {
    pub fn from_boundary_gates(
        next_queue: &AgentTaskQueue,
        dispatch_gate: &TaskDispatchGateDecision,
        memory_submission_gate: &MemorySubmissionGateDecision,
        report_gate: &AgentReportGateDecision,
        service_admission: &AgentCollaborationAdapterSideEffectAdmission,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![
                AgentAdapterBoundaryGate::from_dispatch_gate(dispatch_gate),
                AgentAdapterBoundaryGate::from_memory_submission_gate(memory_submission_gate),
                AgentAdapterBoundaryGate::from_report_gate(report_gate),
                AgentAdapterBoundaryGate::from_service_admission(service_admission),
            ],
        )
    }

    pub fn from_dispatch_and_run_progress(
        next_queue: &AgentTaskQueue,
        dispatch_gate: &TaskDispatchGateDecision,
        progress: &AgentRunLedgerProgress,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_dispatch_and_run_progress(
                dispatch_gate,
                progress,
            )],
        )
    }

    pub fn from_dispatch_and_run_ledger_admission(
        next_queue: &AgentTaskQueue,
        dispatch_gate: &TaskDispatchGateDecision,
        admission: &AgentRunLedgerAdmission,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![
                AgentAdapterBoundaryGate::from_dispatch_and_run_ledger_admission(
                    dispatch_gate,
                    admission,
                ),
            ],
        )
    }

    pub fn from_cycle_report(next_queue: &AgentTaskQueue, report: &AgentCycleReport) -> Self {
        let dispatch_gate = report.dispatch.gate();
        Self::from_dispatch_and_run_ledger_admission(
            next_queue,
            &dispatch_gate,
            &report.run_ledger_admission,
        )
    }

    pub fn from_evolution_admission_handoff_history(
        next_queue: &AgentTaskQueue,
        record: &EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_evolution_admission_handoff_history(record)],
        )
    }

    pub fn from_evolution_admission(
        next_queue: &AgentTaskQueue,
        record: &EvolutionAdmissionRecord,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_evolution_admission(record)],
        )
    }

    pub fn from_budget_ledger_history_gate(
        next_queue: &AgentTaskQueue,
        record: &BudgetLedgerHistoryGateRecord,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_budget_ledger_history_gate(
                record,
            )],
        )
    }

    pub fn from_recursive_schedule_history_gate(
        next_queue: &AgentTaskQueue,
        record: &RecursiveAgentScheduleHistoryGateRecord,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_recursive_schedule_history_gate(record)],
        )
    }

    pub fn from_reflection_loop_history_gate(
        next_queue: &AgentTaskQueue,
        record: &ReflectionLoopHistoryGateRecord,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_reflection_loop_history_gate(
                record,
            )],
        )
    }

    pub fn from_tool_build_report_history_gate(
        next_queue: &AgentTaskQueue,
        record: &ToolBuildReportHistoryGateRecord,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_tool_build_report_history_gate(record)],
        )
    }

    pub fn from_toolsmith_plan_history_gate(
        next_queue: &AgentTaskQueue,
        record: &ToolsmithPlanHistoryGateRecord,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_toolsmith_plan_history_gate(
                record,
            )],
        )
    }

    pub fn from_process_reward_report_history_gate(
        next_queue: &AgentTaskQueue,
        record: &ProcessRewardReportHistoryGateRecord,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_process_reward_report_history_gate(record)],
        )
    }

    pub fn from_reflection_reward_admission(
        next_queue: &AgentTaskQueue,
        record: &ReflectionRewardAdmissionRecord,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_reflection_reward_admission(
                record,
            )],
        )
    }

    pub fn from_conflict_report_history_gate(
        next_queue: &AgentTaskQueue,
        record: &ConflictReportHistoryGateRecord,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_conflict_report_history_gate(
                record,
            )],
        )
    }

    pub fn from_aggregation_conflict_review_trend_gate(
        next_queue: &AgentTaskQueue,
        decision: &AggregationConflictReviewTrendGateDecision,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![AgentAdapterBoundaryGate::from_aggregation_conflict_review_trend_gate(decision)],
        )
    }

    pub fn from_report_and_tool_build_gates(
        next_queue: &AgentTaskQueue,
        report_gate: &AgentReportGateDecision,
        tool_build_record: &ToolBuildReportHistoryGateRecord,
    ) -> Self {
        Self::from_gates(
            next_queue,
            vec![
                AgentAdapterBoundaryGate::from_report_gate(report_gate),
                AgentAdapterBoundaryGate::from_tool_build_report_history_gate(tool_build_record),
            ],
        )
    }

    pub fn from_run_report_final_gate_decision(
        decision: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision,
    ) -> Self {
        Self::from_gates(
            &decision.next_queue,
            vec![AgentAdapterBoundaryGate::from_run_report_final_gate_decision(decision)],
        )
    }

    pub fn from_run_report_final_handoff(
        record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord,
    ) -> Self {
        Self::from_gates(
            &record.gate_decision.next_queue,
            vec![AgentAdapterBoundaryGate::from_run_report_final_handoff(
                record,
            )],
        )
    }

    pub fn from_service_execution_final_gate_decision(
        decision: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision,
    ) -> Self {
        Self::from_gates(
            &decision.next_queue,
            vec![AgentAdapterBoundaryGate::from_service_execution_final_gate_decision(decision)],
        )
    }

    pub fn from_service_execution_final_handoff(
        record: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoffRecord,
    ) -> Self {
        Self::from_gates(
            &record.gate_decision.next_queue,
            vec![AgentAdapterBoundaryGate::from_service_execution_final_handoff(record)],
        )
    }

    pub fn from_runtime_service_loop_control_plan(
        plan: &AgentClosedLoopRuntimeServiceLoopRunControlPlan,
    ) -> Self {
        Self::from_gates(
            &plan.next_queue,
            vec![AgentAdapterBoundaryGate::from_runtime_service_loop_control_plan(plan)],
        )
    }

    pub fn from_runtime_service_loop_control_record(
        record: &AgentClosedLoopRuntimeServiceLoopRunControlRecord,
    ) -> Self {
        Self::from_gates(
            &record.control_plan.next_queue,
            vec![AgentAdapterBoundaryGate::from_runtime_service_loop_control_record(record)],
        )
    }

    pub fn from_runtime_service_preflight(
        preflight: &AgentClosedLoopRuntimeServicePreflight,
    ) -> Self {
        Self::from_gates(
            &preflight.turn_plan.next_queue,
            vec![AgentAdapterBoundaryGate::from_runtime_service_preflight(
                preflight,
            )],
        )
    }

    pub fn from_runtime_service_preflight_continuation(
        continuation: &AgentClosedLoopRuntimeServicePreflightContinuation,
    ) -> Self {
        Self::from_gates(
            &continuation.next_runtime_input.next_queue,
            vec![
                AgentAdapterBoundaryGate::from_runtime_service_preflight_continuation(continuation),
            ],
        )
    }

    pub fn from_runtime_service_loop_state(state: &AgentClosedLoopRuntimeServiceLoopState) -> Self {
        Self::from_gates(
            &state.next_runtime_input().next_queue,
            vec![AgentAdapterBoundaryGate::from_runtime_service_loop_state(
                state,
            )],
        )
    }

    pub fn from_runtime_service_loop_advance(
        advance: &AgentClosedLoopRuntimeServiceLoopAdvance,
    ) -> Self {
        Self::from_gates(
            &advance.next_runtime_input().next_queue,
            vec![AgentAdapterBoundaryGate::from_runtime_service_loop_advance(
                advance,
            )],
        )
    }

    pub fn from_runtime_service_loop_daemon_record(
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRecord,
    ) -> Self {
        Self::from_gates(
            runtime_service_loop_daemon_record_next_queue(record),
            vec![AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_record(record)],
        )
    }

    pub fn from_runtime_service_loop_daemon_continuation(
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
    ) -> Self {
        Self::from_gates(
            runtime_service_loop_daemon_continuation_next_queue(continuation),
            vec![
                AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_continuation(
                    continuation,
                ),
            ],
        )
    }

    pub fn from_runtime_service_loop_daemon_input_plan(
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan,
    ) -> Self {
        Self::from_gates(
            runtime_service_loop_daemon_input_plan_next_queue(plan),
            vec![AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_input_plan(plan)],
        )
    }

    pub fn from_runtime_service_loop_daemon_request_plan(
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
    ) -> Self {
        Self::from_gates(
            runtime_service_loop_daemon_request_next_queue(plan),
            vec![AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_plan(plan)],
        )
    }

    pub fn from_runtime_service_loop_daemon_request_record(
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord,
    ) -> Self {
        Self::from_gates(
            runtime_service_loop_daemon_request_next_queue(&record.request_plan),
            vec![AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_record(record)],
        )
    }

    pub fn from_runtime_service_loop_daemon_request_monitored_plan(
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
    ) -> Self {
        Self::from_gates(
            runtime_service_loop_daemon_request_monitored_next_queue(plan),
            vec![
                AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_plan(
                    plan,
                ),
            ],
        )
    }

    pub fn from_runtime_service_loop_daemon_request_monitored_record(
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord,
    ) -> Self {
        Self::from_gates(
            runtime_service_loop_daemon_request_monitored_next_queue(&record.monitored_plan),
            vec![
                AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_record(
                    record,
                ),
            ],
        )
    }

    pub fn from_runtime_service_loop_daemon_request_monitored_close_plan(
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
    ) -> Self {
        Self::from_gates(
            runtime_service_loop_daemon_request_monitored_close_next_queue(plan),
            vec![
                AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_close_plan(
                    plan,
                ),
            ],
        )
    }

    pub fn from_runtime_service_loop_daemon_request_monitored_close_run_record(
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord,
    ) -> Self {
        Self::from_gates(
            runtime_service_loop_daemon_request_monitored_close_next_queue(
                &record.monitored_close_plan,
            ),
            vec![
                AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_close_run_record(
                    record,
                ),
            ],
        )
    }

    pub fn from_runtime_service_loop_daemon_request_monitored_close_continuation(
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation,
    ) -> Self {
        Self::from_gates(
            runtime_service_loop_daemon_request_monitored_close_continuation_next_queue(
                continuation,
            ),
            vec![
                AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_close_continuation(
                    continuation,
                ),
            ],
        )
    }

    pub fn from_gates(
        next_queue: &AgentTaskQueue,
        mut gates: Vec<AgentAdapterBoundaryGate>,
    ) -> Self {
        gates.sort_by(|left, right| left.owner.cmp(&right.owner));
        let next_queue_task_ids = next_queue.task_ids();
        let blocked_reasons = gates
            .iter()
            .flat_map(|gate| {
                gate.blocked_reasons
                    .iter()
                    .map(move |reason| format!("{}:{reason}", gate.owner.as_str()))
            })
            .collect::<Vec<_>>();
        let telemetry = adapter_boundary_snapshot_telemetry(&gates, &next_queue_task_ids);

        Self {
            gates,
            next_queue_task_ids,
            blocked_reasons,
            telemetry,
        }
    }

    pub fn summary(&self) -> AgentAdapterBoundarySummary {
        AgentAdapterBoundarySummary::from_snapshot(self)
    }

    pub fn status(&self) -> AgentAdapterBoundaryStatus {
        if self
            .gates
            .iter()
            .any(|gate| gate.status == AgentAdapterBoundaryStatus::Repair)
        {
            AgentAdapterBoundaryStatus::Repair
        } else if self
            .gates
            .iter()
            .any(|gate| gate.status == AgentAdapterBoundaryStatus::Watch)
        {
            AgentAdapterBoundaryStatus::Watch
        } else {
            AgentAdapterBoundaryStatus::Stable
        }
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status() == AgentAdapterBoundaryStatus::Repair
    }

    pub fn allows_service_advance(&self) -> bool {
        !self.requires_repair_first()
    }

    pub fn all_stable(&self) -> bool {
        !self.gates.is_empty()
            && self
                .gates
                .iter()
                .all(|gate| gate.status == AgentAdapterBoundaryStatus::Stable)
    }

    pub fn can_dispatch_core(&self) -> bool {
        self.allows_service_advance() && self.gates.iter().all(|gate| gate.dispatch_allowed)
    }

    pub fn can_execute_service_commands(&self) -> bool {
        self.allows_service_advance() && self.gates.iter().all(|gate| gate.service_command_allowed)
    }

    pub fn can_submit_memory_note(&self) -> bool {
        self.all_stable() && self.gates.iter().all(|gate| gate.memory_note_allowed)
    }

    pub fn can_promote_adaptive_state(&self) -> bool {
        self.all_stable() && self.gates.iter().all(|gate| gate.adaptive_state_allowed)
    }

    pub fn service_execution_command_reason_count(&self) -> usize {
        self.gates
            .iter()
            .map(|gate| gate.service_execution_command_reason_count)
            .sum()
    }

    pub fn service_execution_memory_promotion_command_reason_count(&self) -> usize {
        self.gates
            .iter()
            .map(|gate| gate.service_execution_memory_promotion_command_reason_count)
            .sum()
    }

    pub fn service_execution_memory_promotion_command_reason_closes(&self) -> usize {
        self.gates
            .iter()
            .map(|gate| gate.service_execution_memory_promotion_command_reason_closes)
            .sum()
    }

    pub fn service_execution_tool_build_command_reason_count(&self) -> usize {
        self.gates
            .iter()
            .map(|gate| gate.service_execution_tool_build_command_reason_count)
            .sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAdapterBoundarySummary {
    pub status: AgentAdapterBoundaryStatus,
    pub owners: usize,
    pub stable_owners: usize,
    pub watch_owners: usize,
    pub repair_owners: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub can_dispatch_core: bool,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub can_execute_service_commands: bool,
    pub requires_repair_first: bool,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundarySummary {
    pub fn from_snapshot(snapshot: &AgentAdapterBoundarySnapshot) -> Self {
        let status = snapshot.status();
        let owners = snapshot.gates.len();
        let stable_owners = snapshot
            .gates
            .iter()
            .filter(|gate| gate.status == AgentAdapterBoundaryStatus::Stable)
            .count();
        let watch_owners = snapshot
            .gates
            .iter()
            .filter(|gate| gate.status == AgentAdapterBoundaryStatus::Watch)
            .count();
        let repair_owners = snapshot
            .gates
            .iter()
            .filter(|gate| gate.status == AgentAdapterBoundaryStatus::Repair)
            .count();
        let next_queue_tasks = snapshot.next_queue_task_ids.len();
        let blocked_reasons = snapshot.blocked_reasons.len();
        let can_dispatch_core = snapshot.can_dispatch_core();
        let can_submit_memory_note = snapshot.can_submit_memory_note();
        let can_promote_adaptive_state = snapshot.can_promote_adaptive_state();
        let can_execute_service_commands = snapshot.can_execute_service_commands();
        let requires_repair_first = snapshot.requires_repair_first();
        let service_execution_command_reason_count =
            snapshot.service_execution_command_reason_count();
        let service_execution_memory_promotion_command_reason_count =
            snapshot.service_execution_memory_promotion_command_reason_count();
        let service_execution_memory_promotion_command_reason_closes =
            snapshot.service_execution_memory_promotion_command_reason_closes();
        let service_execution_tool_build_command_reason_count =
            snapshot.service_execution_tool_build_command_reason_count();
        let telemetry = adapter_boundary_summary_telemetry(
            status,
            owners,
            stable_owners,
            watch_owners,
            repair_owners,
            next_queue_tasks,
            blocked_reasons,
            can_dispatch_core,
            can_submit_memory_note,
            can_promote_adaptive_state,
            can_execute_service_commands,
            requires_repair_first,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
        );

        Self {
            status,
            owners,
            stable_owners,
            watch_owners,
            repair_owners,
            next_queue_tasks,
            blocked_reasons,
            can_dispatch_core,
            can_submit_memory_note,
            can_promote_adaptive_state,
            can_execute_service_commands,
            requires_repair_first,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentAdapterBoundarySummaryHistory {
    summaries: Vec<AgentAdapterBoundarySummary>,
}

impl AgentAdapterBoundarySummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentAdapterBoundarySummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentAdapterBoundarySummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentAdapterBoundarySummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentAdapterBoundarySummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentAdapterBoundaryDashboard {
        AgentAdapterBoundaryDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AgentAdapterBoundaryHealthPolicy) -> AgentAdapterBoundaryHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryDashboard {
    pub total_records: usize,
    pub stable_records: usize,
    pub watch_records: usize,
    pub repair_records: usize,
    pub repair_first_records: usize,
    pub memory_promotable_records: usize,
    pub adaptive_promotable_records: usize,
    pub service_command_records: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub stable_rate: f32,
    pub memory_promotion_rate: f32,
    pub adaptive_promotion_rate: f32,
    pub service_command_rate: f32,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryDashboard {
    pub fn from_summaries(summaries: &[AgentAdapterBoundarySummary]) -> Self {
        let total_records = summaries.len();
        let stable_records = summaries
            .iter()
            .filter(|summary| summary.status == AgentAdapterBoundaryStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.status == AgentAdapterBoundaryStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.status == AgentAdapterBoundaryStatus::Repair)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let memory_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_submit_memory_note)
            .count();
        let adaptive_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let service_command_records = summaries
            .iter()
            .filter(|summary| summary.can_execute_service_commands)
            .count();
        let next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reasons = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let service_execution_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_closes = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_closes)
            .sum::<usize>();
        let service_execution_tool_build_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_tool_build_command_reason_count)
            .sum::<usize>();
        let stable_rate = rate(stable_records, total_records);
        let memory_promotion_rate = rate(memory_promotable_records, total_records);
        let adaptive_promotion_rate = rate(adaptive_promotable_records, total_records);
        let service_command_rate = rate(service_command_records, total_records);
        let telemetry = adapter_boundary_dashboard_telemetry(
            total_records,
            stable_records,
            watch_records,
            repair_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            service_command_records,
            next_queue_tasks,
            blocked_reasons,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            stable_rate,
            memory_promotion_rate,
            adaptive_promotion_rate,
            service_command_rate,
        );

        Self {
            total_records,
            stable_records,
            watch_records,
            repair_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            service_command_records,
            next_queue_tasks,
            blocked_reasons,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            stable_rate,
            memory_promotion_rate,
            adaptive_promotion_rate,
            service_command_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: AgentAdapterBoundaryHealthPolicy) -> AgentAdapterBoundaryHealth {
        AgentAdapterBoundaryHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentAdapterBoundaryHealthPolicy {
    pub minimum_stable_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_first_records: usize,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentAdapterBoundaryHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_stable_rate: 0.67,
            maximum_repair_records: 0,
            maximum_repair_first_records: 0,
            maximum_blocked_reasons: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHealth {
    pub status: AgentAdapterBoundaryStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentAdapterBoundaryDashboard,
}

impl AgentAdapterBoundaryHealth {
    pub fn from_dashboard(
        dashboard: AgentAdapterBoundaryDashboard,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("adapter_boundary_history_empty".to_owned());
        } else if dashboard.stable_rate < policy.minimum_stable_rate {
            watch_reasons.push(format!(
                "adapter_boundary_stable_rate={:.3}<{}",
                dashboard.stable_rate, policy.minimum_stable_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "adapter_boundary_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "adapter_boundary_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            watch_reasons.push(format!(
                "adapter_boundary_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentAdapterBoundaryStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentAdapterBoundaryStatus::Watch, watch_reasons)
        } else {
            (AgentAdapterBoundaryStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentAdapterBoundaryStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundarySummaryHistoryRecord {
    pub history: AgentAdapterBoundarySummaryHistory,
    pub appended_summary: AgentAdapterBoundarySummary,
    pub dashboard: AgentAdapterBoundaryDashboard,
    pub health: AgentAdapterBoundaryHealth,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundarySummaryHistoryRecord {
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
pub struct AgentAdapterBoundaryRecord {
    pub snapshot: AgentAdapterBoundarySnapshot,
    pub history_record: AgentAdapterBoundarySummaryHistoryRecord,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryRecord {
    pub fn summary(&self) -> &AgentAdapterBoundarySummary {
        &self.history_record.appended_summary
    }

    pub fn records(&self) -> usize {
        self.history_record.records()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.history_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.history_record.requires_repair_first()
    }

    pub fn can_submit_memory_note(&self) -> bool {
        self.snapshot.can_submit_memory_note() && self.history_record.health.is_stable()
    }

    pub fn can_promote_adaptive_state(&self) -> bool {
        self.snapshot.can_promote_adaptive_state() && self.history_record.health.is_stable()
    }

    pub fn can_execute_service_commands(&self) -> bool {
        self.snapshot.can_execute_service_commands()
            && self.history_record.health.allows_service_advance()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoff {
    pub boundary_record: AgentAdapterBoundaryRecord,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoff {
    pub fn from_record_and_queue(
        boundary_record: AgentAdapterBoundaryRecord,
        next_queue: &AgentTaskQueue,
    ) -> Self {
        let requires_repair_first = boundary_record.requires_repair_first();
        let blocked_reasons = adapter_boundary_handoff_blocked_reasons(&boundary_record);
        let repair_tasks = adapter_boundary_repair_tasks(requires_repair_first, &blocked_reasons);
        let next_queue = next_queue.clone().with_repair_first(&repair_tasks);
        let admitted = !requires_repair_first;
        let telemetry = adapter_boundary_handoff_telemetry(
            &boundary_record,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            blocked_reasons.len(),
        );
        let service_execution_command_reason_count = boundary_record
            .summary()
            .service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count = boundary_record
            .summary()
            .service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes = boundary_record
            .summary()
            .service_execution_memory_promotion_command_reason_closes;
        let service_execution_tool_build_command_reason_count = boundary_record
            .summary()
            .service_execution_tool_build_command_reason_count;

        Self {
            boundary_record,
            admitted,
            requires_repair_first,
            repair_tasks,
            next_queue,
            blocked_reasons,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry,
        }
    }

    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }

    pub fn can_submit_memory_note(&self) -> bool {
        self.is_admitted() && self.boundary_record.can_submit_memory_note()
    }

    pub fn can_promote_adaptive_state(&self) -> bool {
        self.is_admitted() && self.boundary_record.can_promote_adaptive_state()
    }

    pub fn can_execute_service_commands(&self) -> bool {
        self.is_admitted() && self.boundary_record.can_execute_service_commands()
    }

    pub fn summary(&self) -> AgentAdapterBoundaryHandoffSummary {
        AgentAdapterBoundaryHandoffSummary::from_handoff(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAdapterBoundaryHandoffSummary {
    pub snapshot_status: AgentAdapterBoundaryStatus,
    pub health_status: AgentAdapterBoundaryStatus,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffSummary {
    pub fn from_handoff(handoff: &AgentAdapterBoundaryHandoff) -> Self {
        let snapshot_status = handoff.boundary_record.snapshot.status();
        let health_status = handoff.boundary_record.history_record.health.status;
        let repair_task_ids = handoff
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = handoff.next_queue.task_ids();
        let telemetry = adapter_boundary_handoff_summary_telemetry(
            snapshot_status,
            health_status,
            handoff.admitted,
            handoff.requires_repair_first,
            handoff.can_submit_memory_note(),
            handoff.can_promote_adaptive_state(),
            repair_task_ids.len(),
            next_queue_task_ids.len(),
            handoff.blocked_reasons.len(),
            handoff
                .boundary_record
                .summary()
                .service_execution_command_reason_count,
            handoff
                .boundary_record
                .summary()
                .service_execution_memory_promotion_command_reason_count,
            handoff
                .boundary_record
                .summary()
                .service_execution_memory_promotion_command_reason_closes,
            handoff
                .boundary_record
                .summary()
                .service_execution_tool_build_command_reason_count,
        );

        Self {
            snapshot_status,
            health_status,
            admitted: handoff.admitted,
            requires_repair_first: handoff.requires_repair_first,
            can_submit_memory_note: handoff.can_submit_memory_note(),
            can_promote_adaptive_state: handoff.can_promote_adaptive_state(),
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: handoff.blocked_reasons.len(),
            service_execution_command_reason_count: handoff
                .boundary_record
                .summary()
                .service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: handoff
                .boundary_record
                .summary()
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: handoff
                .boundary_record
                .summary()
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: handoff
                .boundary_record
                .summary()
                .service_execution_tool_build_command_reason_count,
            repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentAdapterBoundaryHandoffSummaryHistory {
    summaries: Vec<AgentAdapterBoundaryHandoffSummary>,
}

impl AgentAdapterBoundaryHandoffSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentAdapterBoundaryHandoffSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentAdapterBoundaryHandoffSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentAdapterBoundaryHandoffSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentAdapterBoundaryHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentAdapterBoundaryHandoffDashboard {
        AgentAdapterBoundaryHandoffDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffDashboard {
    pub total_records: usize,
    pub admitted_records: usize,
    pub repair_first_records: usize,
    pub memory_promotable_records: usize,
    pub adaptive_promotable_records: usize,
    pub repair_task_count: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub latest_snapshot_status: Option<AgentAdapterBoundaryStatus>,
    pub latest_health_status: Option<AgentAdapterBoundaryStatus>,
    pub admitted_rate: f32,
    pub repair_first_rate: f32,
    pub memory_promotion_rate: f32,
    pub adaptive_promotion_rate: f32,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffDashboard {
    pub fn from_summaries(summaries: &[AgentAdapterBoundaryHandoffSummary]) -> Self {
        let total_records = summaries.len();
        let admitted_records = summaries
            .iter()
            .filter(|summary| summary.admitted && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let memory_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_submit_memory_note)
            .count();
        let adaptive_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reasons = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let service_execution_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_closes = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_closes)
            .sum::<usize>();
        let service_execution_tool_build_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_tool_build_command_reason_count)
            .sum::<usize>();
        let latest_snapshot_status = summaries.last().map(|summary| summary.snapshot_status);
        let latest_health_status = summaries.last().map(|summary| summary.health_status);
        let admitted_rate = rate(admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let memory_promotion_rate = rate(memory_promotable_records, total_records);
        let adaptive_promotion_rate = rate(adaptive_promotable_records, total_records);
        let telemetry = adapter_boundary_handoff_dashboard_telemetry(
            total_records,
            admitted_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            repair_task_count,
            next_queue_tasks,
            blocked_reasons,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            admitted_rate,
            repair_first_rate,
            memory_promotion_rate,
            adaptive_promotion_rate,
        );

        Self {
            total_records,
            admitted_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            repair_task_count,
            next_queue_tasks,
            blocked_reasons,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            latest_snapshot_status,
            latest_health_status,
            admitted_rate,
            repair_first_rate,
            memory_promotion_rate,
            adaptive_promotion_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffHealth {
        AgentAdapterBoundaryHandoffHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentAdapterBoundaryHandoffHealthPolicy {
    pub minimum_admitted_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentAdapterBoundaryHandoffHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_admitted_rate: 0.67,
            maximum_repair_first_records: 0,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffHealth {
    pub status: AgentAdapterBoundaryStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentAdapterBoundaryHandoffDashboard,
}

impl AgentAdapterBoundaryHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentAdapterBoundaryHandoffDashboard,
        policy: AgentAdapterBoundaryHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("adapter_boundary_handoff_history_empty".to_owned());
        } else if dashboard.admitted_rate < policy.minimum_admitted_rate {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_admitted_rate={:.3}<{}",
                dashboard.admitted_rate, policy.minimum_admitted_rate
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentAdapterBoundaryStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentAdapterBoundaryStatus::Watch, watch_reasons)
        } else {
            (AgentAdapterBoundaryStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentAdapterBoundaryStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffHistoryRecord {
    pub history: AgentAdapterBoundaryHandoffSummaryHistory,
    pub appended_summary: AgentAdapterBoundaryHandoffSummary,
    pub dashboard: AgentAdapterBoundaryHandoffDashboard,
    pub health: AgentAdapterBoundaryHandoffHealth,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffHistoryRecord {
    pub fn records(&self) -> usize {
        self.history.len()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.health.requires_repair_first()
    }

    pub fn report_gate_decision(&self, run_id: impl AsRef<str>) -> AgentReportGateDecision {
        let reasons = adapter_boundary_handoff_report_gate_reasons(self);
        let repair_first = self.requires_repair_first();
        let follow_up_tasks = if repair_first {
            adapter_boundary_handoff_report_gate_tasks(run_id.as_ref(), &reasons)
        } else {
            Vec::new()
        };

        AgentReportGateDecision {
            accepted: reasons.is_empty(),
            reasons,
            follow_up_tasks,
        }
    }

    pub fn record_report_gate_with_health(
        &self,
        history: AgentReportGateSummaryHistory,
        policy: AgentReportGateHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentReportGateHistoryRecord {
        let decision = self.report_gate_decision(run_id);

        AgentReportGateHistoryRecorder::new()
            .record_decision_with_health(history, &decision, policy)
    }

    pub fn record_report_gate_with_health_gate(
        &self,
        history: AgentReportGateSummaryHistory,
        policy: AgentReportGateHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateRecord {
        let decision = self.report_gate_decision(run_id.as_ref());

        AgentReportGateHistoryRecorder::new()
            .record_decision_with_health_gate(history, &decision, policy, run_id, next_queue)
    }

    pub fn record_report_gate_trend_handoff(
        &self,
        report_history: AgentReportGateSummaryHistory,
        report_policy: AgentReportGateHealthPolicy,
        gate_history: AgentReportGateHealthGateSummaryHistory,
        gate_policy: AgentReportGateHealthGateHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateTrendHandoffRecord {
        let run_id = run_id.as_ref();
        let gate_record = self.record_report_gate_with_health_gate(
            report_history,
            report_policy,
            run_id,
            next_queue,
        );

        AgentReportGateHealthGateTrendHandoff::new().record_gate_record_and_gate(
            gate_history,
            &gate_record,
            gate_policy,
            run_id,
            next_queue,
        )
    }

    pub fn record_report_gate_trend_handoff_monitor(
        &self,
        report_history: AgentReportGateSummaryHistory,
        report_policy: AgentReportGateHealthPolicy,
        gate_history: AgentReportGateHealthGateSummaryHistory,
        gate_policy: AgentReportGateHealthGateHealthPolicy,
        handoff_history: AgentReportGateHealthGateTrendHandoffHistory,
        handoff_policy: AgentReportGateHealthGateTrendHandoffHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorRecord {
        let run_id = run_id.as_ref();
        let handoff = self.record_report_gate_trend_handoff(
            report_history,
            report_policy,
            gate_history,
            gate_policy,
            run_id,
            next_queue,
        );

        AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            handoff_history,
            handoff_policy,
            run_id,
        )
    }

    pub fn record_report_gate_trend_handoff_monitor_handoff(
        &self,
        report_history: AgentReportGateSummaryHistory,
        report_policy: AgentReportGateHealthPolicy,
        gate_history: AgentReportGateHealthGateSummaryHistory,
        gate_policy: AgentReportGateHealthGateHealthPolicy,
        handoff_history: AgentReportGateHealthGateTrendHandoffHistory,
        handoff_policy: AgentReportGateHealthGateTrendHandoffHealthPolicy,
        monitor_history: AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory,
        monitor_policy: AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord {
        let run_id = run_id.as_ref();
        let monitor = self.record_report_gate_trend_handoff_monitor(
            report_history,
            report_policy,
            gate_history,
            gate_policy,
            handoff_history,
            handoff_policy,
            run_id,
            next_queue,
        );

        AgentReportGateHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor,
            monitor_history,
            monitor_policy,
            run_id,
        )
    }

    pub fn record_report_gate_trend_handoff_monitor_handoff_handoff(
        &self,
        report_history: AgentReportGateSummaryHistory,
        report_policy: AgentReportGateHealthPolicy,
        gate_history: AgentReportGateHealthGateSummaryHistory,
        gate_policy: AgentReportGateHealthGateHealthPolicy,
        handoff_history: AgentReportGateHealthGateTrendHandoffHistory,
        handoff_policy: AgentReportGateHealthGateTrendHandoffHealthPolicy,
        monitor_history: AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory,
        monitor_policy: AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
        monitor_handoff_history: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        monitor_handoff_policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord {
        let run_id = run_id.as_ref();
        let handoff = self.record_report_gate_trend_handoff_monitor_handoff(
            report_history,
            report_policy,
            gate_history,
            gate_policy,
            handoff_history,
            handoff_policy,
            monitor_history,
            monitor_policy,
            run_id,
            next_queue,
        );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoff::new().record_and_gate(
            handoff,
            monitor_handoff_history,
            monitor_handoff_policy,
            run_id,
        )
    }

    pub fn record_report_gate_trend_handoff_monitor_handoff_handoff_handoff(
        &self,
        report_history: AgentReportGateSummaryHistory,
        report_policy: AgentReportGateHealthPolicy,
        gate_history: AgentReportGateHealthGateSummaryHistory,
        gate_policy: AgentReportGateHealthGateHealthPolicy,
        handoff_history: AgentReportGateHealthGateTrendHandoffHistory,
        handoff_policy: AgentReportGateHealthGateTrendHandoffHealthPolicy,
        monitor_history: AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory,
        monitor_policy: AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
        monitor_handoff_history: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        monitor_handoff_policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy,
        monitor_handoff_handoff_history: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
        monitor_handoff_handoff_policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord {
        let run_id = run_id.as_ref();
        let packet = self.record_report_gate_trend_handoff_monitor_handoff_handoff(
            report_history,
            report_policy,
            gate_history,
            gate_policy,
            handoff_history,
            handoff_policy,
            monitor_history,
            monitor_policy,
            monitor_handoff_history,
            monitor_handoff_policy,
            run_id,
            next_queue,
        );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new().record_and_gate(
            packet,
            monitor_handoff_handoff_history,
            monitor_handoff_handoff_policy,
            run_id,
        )
    }

    pub fn record_report_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff(
        &self,
        report_history: AgentReportGateSummaryHistory,
        report_policy: AgentReportGateHealthPolicy,
        gate_history: AgentReportGateHealthGateSummaryHistory,
        gate_policy: AgentReportGateHealthGateHealthPolicy,
        handoff_history: AgentReportGateHealthGateTrendHandoffHistory,
        handoff_policy: AgentReportGateHealthGateTrendHandoffHealthPolicy,
        monitor_history: AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory,
        monitor_policy: AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
        monitor_handoff_history: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        monitor_handoff_policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy,
        monitor_handoff_handoff_history: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
        monitor_handoff_handoff_policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
        monitor_handoff_handoff_handoff_history: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
        monitor_handoff_handoff_handoff_policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord {
        let run_id = run_id.as_ref();
        let admission = self.record_report_gate_trend_handoff_monitor_handoff_handoff_handoff(
            report_history,
            report_policy,
            gate_history,
            gate_policy,
            handoff_history,
            handoff_policy,
            monitor_history,
            monitor_policy,
            monitor_handoff_history,
            monitor_handoff_policy,
            monitor_handoff_handoff_history,
            monitor_handoff_handoff_policy,
            run_id,
            next_queue,
        );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff::new()
            .record_and_gate(
                admission,
                monitor_handoff_handoff_handoff_history,
                monitor_handoff_handoff_handoff_policy,
                run_id,
            )
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffHistoryRecorder;

impl AgentAdapterBoundaryHandoffHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentAdapterBoundaryHandoffSummaryHistory,
        summary: AgentAdapterBoundaryHandoffSummary,
        policy: AgentAdapterBoundaryHandoffHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = adapter_boundary_handoff_history_record_telemetry(&dashboard, &health);

        AgentAdapterBoundaryHandoffHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_handoff_with_health(
        &self,
        history: AgentAdapterBoundaryHandoffSummaryHistory,
        handoff: &AgentAdapterBoundaryHandoff,
        policy: AgentAdapterBoundaryHandoffHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffHistoryRecord {
        self.record_summary_with_health(history, handoff.summary(), policy)
    }

    pub fn record_evolution_admission_handoff_history_with_health(
        &self,
        boundary_history: AgentAdapterBoundarySummaryHistory,
        handoff_history: AgentAdapterBoundaryHandoffSummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord,
        boundary_policy: AgentAdapterBoundaryHealthPolicy,
        handoff_policy: AgentAdapterBoundaryHandoffHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffHistoryRecord {
        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_evolution_admission_handoff_history_handoff_with_health(
                boundary_history,
                next_queue,
                record,
                boundary_policy,
            );

        self.record_handoff_with_health(handoff_history, &handoff, handoff_policy)
    }

    pub fn record_tool_build_report_history_gate_with_health(
        &self,
        boundary_history: AgentAdapterBoundarySummaryHistory,
        handoff_history: AgentAdapterBoundaryHandoffSummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &ToolBuildReportHistoryGateRecord,
        boundary_policy: AgentAdapterBoundaryHealthPolicy,
        handoff_policy: AgentAdapterBoundaryHandoffHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffHistoryRecord {
        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_tool_build_report_history_gate_handoff_with_health(
                boundary_history,
                next_queue,
                record,
                boundary_policy,
            );

        self.record_handoff_with_health(handoff_history, &handoff, handoff_policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendGateDecision {
    pub requested_admitted: bool,
    pub effective_admitted: bool,
    pub handoff_health: AgentAdapterBoundaryHandoffHealth,
    pub requires_repair_first: bool,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.effective_admitted && !self.requires_repair_first
    }

    pub fn summary(&self) -> AgentAdapterBoundaryHandoffTrendGateSummary {
        AgentAdapterBoundaryHandoffTrendGateSummary::from_decision(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAdapterBoundaryHandoffTrendGateSummary {
    pub handoff_health_status: AgentAdapterBoundaryStatus,
    pub requested_admitted: bool,
    pub effective_admitted: bool,
    pub requires_repair_first: bool,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendGateSummary {
    pub fn from_decision(decision: &AgentAdapterBoundaryHandoffTrendGateDecision) -> Self {
        let repair_task_ids = decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = decision.next_queue.task_ids();
        let telemetry = adapter_boundary_handoff_trend_gate_summary_telemetry(
            decision.handoff_health.status,
            decision.requested_admitted,
            decision.effective_admitted,
            decision.requires_repair_first,
            decision.can_submit_memory_note,
            decision.can_promote_adaptive_state,
            repair_task_ids.len(),
            next_queue_task_ids.len(),
            decision.blocked_reasons.len(),
            decision.service_execution_command_reason_count,
            decision.service_execution_memory_promotion_command_reason_count,
            decision.service_execution_memory_promotion_command_reason_closes,
            decision.service_execution_tool_build_command_reason_count,
        );

        Self {
            handoff_health_status: decision.handoff_health.status,
            requested_admitted: decision.requested_admitted,
            effective_admitted: decision.effective_admitted,
            requires_repair_first: decision.requires_repair_first,
            can_submit_memory_note: decision.can_submit_memory_note,
            can_promote_adaptive_state: decision.can_promote_adaptive_state,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: decision.blocked_reasons.len(),
            service_execution_command_reason_count: decision.service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: decision
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: decision
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: decision
                .service_execution_tool_build_command_reason_count,
            repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentAdapterBoundaryHandoffTrendGateSummaryHistory {
    summaries: Vec<AgentAdapterBoundaryHandoffTrendGateSummary>,
}

impl AgentAdapterBoundaryHandoffTrendGateSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentAdapterBoundaryHandoffTrendGateSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentAdapterBoundaryHandoffTrendGateSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentAdapterBoundaryHandoffTrendGateSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentAdapterBoundaryHandoffTrendGateSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentAdapterBoundaryHandoffTrendGateDashboard {
        AgentAdapterBoundaryHandoffTrendGateDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendGateHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendGateDashboard {
    pub total_records: usize,
    pub requested_admitted_records: usize,
    pub effective_admitted_records: usize,
    pub repair_first_records: usize,
    pub memory_promotable_records: usize,
    pub adaptive_promotable_records: usize,
    pub repair_task_count: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub latest_handoff_health_status: Option<AgentAdapterBoundaryStatus>,
    pub effective_admitted_rate: f32,
    pub repair_first_rate: f32,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendGateDashboard {
    pub fn from_summaries(summaries: &[AgentAdapterBoundaryHandoffTrendGateSummary]) -> Self {
        let total_records = summaries.len();
        let requested_admitted_records = summaries
            .iter()
            .filter(|summary| summary.requested_admitted)
            .count();
        let effective_admitted_records = summaries
            .iter()
            .filter(|summary| summary.effective_admitted && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let memory_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_submit_memory_note)
            .count();
        let adaptive_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reasons = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let service_execution_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_closes = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_closes)
            .sum::<usize>();
        let service_execution_tool_build_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_tool_build_command_reason_count)
            .sum::<usize>();
        let latest_handoff_health_status = summaries
            .last()
            .map(|summary| summary.handoff_health_status);
        let effective_admitted_rate = rate(effective_admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let telemetry = adapter_boundary_handoff_trend_gate_dashboard_telemetry(
            total_records,
            requested_admitted_records,
            effective_admitted_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            repair_task_count,
            next_queue_tasks,
            blocked_reasons,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            effective_admitted_rate,
            repair_first_rate,
        );

        Self {
            total_records,
            requested_admitted_records,
            effective_admitted_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            repair_task_count,
            next_queue_tasks,
            blocked_reasons,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            latest_handoff_health_status,
            effective_admitted_rate,
            repair_first_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendGateHealth {
        AgentAdapterBoundaryHandoffTrendGateHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendGateHealthPolicy {
    pub minimum_effective_admitted_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentAdapterBoundaryHandoffTrendGateHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_effective_admitted_rate: 0.67,
            maximum_repair_first_records: 0,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendGateHealth {
    pub status: AgentAdapterBoundaryStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentAdapterBoundaryHandoffTrendGateDashboard,
}

impl AgentAdapterBoundaryHandoffTrendGateHealth {
    pub fn from_dashboard(
        dashboard: AgentAdapterBoundaryHandoffTrendGateDashboard,
        policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("adapter_boundary_handoff_trend_gate_history_empty".to_owned());
        } else if dashboard.effective_admitted_rate < policy.minimum_effective_admitted_rate {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_trend_gate_effective_admitted_rate={:.3}<{}",
                dashboard.effective_admitted_rate, policy.minimum_effective_admitted_rate
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_trend_gate_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_trend_gate_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_trend_gate_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentAdapterBoundaryStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentAdapterBoundaryStatus::Watch, watch_reasons)
        } else {
            (AgentAdapterBoundaryStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentAdapterBoundaryStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendGateHistoryRecord {
    pub history: AgentAdapterBoundaryHandoffTrendGateSummaryHistory,
    pub appended_summary: AgentAdapterBoundaryHandoffTrendGateSummary,
    pub dashboard: AgentAdapterBoundaryHandoffTrendGateDashboard,
    pub health: AgentAdapterBoundaryHandoffTrendGateHealth,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendGateHistoryRecord {
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
pub struct AgentAdapterBoundaryHandoffTrendAdmission {
    pub decision: AgentAdapterBoundaryHandoffTrendGateDecision,
    pub history_record: AgentAdapterBoundaryHandoffTrendGateHistoryRecord,
    pub effective_admitted: bool,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub history_repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmission {
    pub fn is_admitted(&self) -> bool {
        self.effective_admitted && !self.requires_repair_first
    }

    pub fn summary(&self) -> AgentAdapterBoundaryHandoffTrendAdmissionSummary {
        AgentAdapterBoundaryHandoffTrendAdmissionSummary::from_admission(self)
    }

    pub fn records(&self) -> usize {
        self.history_record.records()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.history_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionSummary {
    pub trend_health_status: AgentAdapterBoundaryStatus,
    pub requested_admitted: bool,
    pub effective_admitted: bool,
    pub requires_repair_first: bool,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub decision_repair_tasks: usize,
    pub history_repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub records: usize,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub decision_repair_task_ids: Vec<String>,
    pub history_repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionSummary {
    pub fn from_admission(admission: &AgentAdapterBoundaryHandoffTrendAdmission) -> Self {
        let decision_repair_task_ids = admission
            .decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let history_repair_task_ids = admission
            .history_repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = admission.next_queue.task_ids();
        let telemetry = adapter_boundary_handoff_trend_admission_summary_telemetry(
            admission.history_record.health.status,
            admission.decision.requested_admitted,
            admission.effective_admitted,
            admission.requires_repair_first,
            admission.can_submit_memory_note,
            admission.can_promote_adaptive_state,
            decision_repair_task_ids.len(),
            history_repair_task_ids.len(),
            next_queue_task_ids.len(),
            admission.blocked_reasons.len(),
            admission.records(),
            admission.service_execution_command_reason_count,
            admission.service_execution_memory_promotion_command_reason_count,
            admission.service_execution_memory_promotion_command_reason_closes,
            admission.service_execution_tool_build_command_reason_count,
        );

        Self {
            trend_health_status: admission.history_record.health.status,
            requested_admitted: admission.decision.requested_admitted,
            effective_admitted: admission.effective_admitted,
            requires_repair_first: admission.requires_repair_first,
            can_submit_memory_note: admission.can_submit_memory_note,
            can_promote_adaptive_state: admission.can_promote_adaptive_state,
            decision_repair_tasks: decision_repair_task_ids.len(),
            history_repair_tasks: history_repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: admission.blocked_reasons.len(),
            records: admission.records(),
            service_execution_command_reason_count: admission
                .service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: admission
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: admission
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: admission
                .service_execution_tool_build_command_reason_count,
            decision_repair_task_ids,
            history_repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory {
    summaries: Vec<AgentAdapterBoundaryHandoffTrendAdmissionSummary>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentAdapterBoundaryHandoffTrendAdmissionSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentAdapterBoundaryHandoffTrendAdmissionSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentAdapterBoundaryHandoffTrendAdmissionSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentAdapterBoundaryHandoffTrendAdmissionSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentAdapterBoundaryHandoffTrendAdmissionDashboard {
        AgentAdapterBoundaryHandoffTrendAdmissionDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionDashboard {
    pub total_records: usize,
    pub requested_admitted_records: usize,
    pub effective_admitted_records: usize,
    pub repair_first_records: usize,
    pub memory_promotable_records: usize,
    pub adaptive_promotable_records: usize,
    pub decision_repair_task_count: usize,
    pub history_repair_task_count: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub latest_trend_health_status: Option<AgentAdapterBoundaryStatus>,
    pub effective_admitted_rate: f32,
    pub repair_first_rate: f32,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionDashboard {
    pub fn from_summaries(summaries: &[AgentAdapterBoundaryHandoffTrendAdmissionSummary]) -> Self {
        let total_records = summaries.len();
        let requested_admitted_records = summaries
            .iter()
            .filter(|summary| summary.requested_admitted)
            .count();
        let effective_admitted_records = summaries
            .iter()
            .filter(|summary| summary.effective_admitted && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let memory_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_submit_memory_note)
            .count();
        let adaptive_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let decision_repair_task_count = summaries
            .iter()
            .map(|summary| summary.decision_repair_tasks)
            .sum::<usize>();
        let history_repair_task_count = summaries
            .iter()
            .map(|summary| summary.history_repair_tasks)
            .sum::<usize>();
        let next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reasons = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let service_execution_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_closes = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_closes)
            .sum::<usize>();
        let service_execution_tool_build_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_tool_build_command_reason_count)
            .sum::<usize>();
        let latest_trend_health_status =
            summaries.last().map(|summary| summary.trend_health_status);
        let effective_admitted_rate = rate(effective_admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let telemetry = adapter_boundary_handoff_trend_admission_dashboard_telemetry(
            total_records,
            requested_admitted_records,
            effective_admitted_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            decision_repair_task_count,
            history_repair_task_count,
            next_queue_tasks,
            blocked_reasons,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            effective_admitted_rate,
            repair_first_rate,
        );

        Self {
            total_records,
            requested_admitted_records,
            effective_admitted_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            decision_repair_task_count,
            history_repair_task_count,
            next_queue_tasks,
            blocked_reasons,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            latest_trend_health_status,
            effective_admitted_rate,
            repair_first_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionHealth {
        AgentAdapterBoundaryHandoffTrendAdmissionHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy {
    pub minimum_effective_admitted_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_history_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_effective_admitted_rate: 0.67,
            maximum_repair_first_records: 0,
            maximum_history_repair_tasks: 0,
            maximum_blocked_reasons: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionHealth {
    pub status: AgentAdapterBoundaryStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentAdapterBoundaryHandoffTrendAdmissionDashboard,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionHealth {
    pub fn from_dashboard(
        dashboard: AgentAdapterBoundaryHandoffTrendAdmissionDashboard,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("adapter_boundary_handoff_trend_admission_history_empty".to_owned());
        } else if dashboard.effective_admitted_rate < policy.minimum_effective_admitted_rate {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_effective_admitted_rate={:.3}<{}",
                dashboard.effective_admitted_rate, policy.minimum_effective_admitted_rate
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.history_repair_task_count > policy.maximum_history_repair_tasks {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_history_repair_tasks={}>{}",
                dashboard.history_repair_task_count, policy.maximum_history_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentAdapterBoundaryStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentAdapterBoundaryStatus::Watch, watch_reasons)
        } else {
            (AgentAdapterBoundaryStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentAdapterBoundaryStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecord {
    pub history: AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory,
    pub appended_summary: AgentAdapterBoundaryHandoffTrendAdmissionSummary,
    pub dashboard: AgentAdapterBoundaryHandoffTrendAdmissionDashboard,
    pub health: AgentAdapterBoundaryHandoffTrendAdmissionHealth,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecord {
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
pub struct AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder;

impl AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory,
        summary: AgentAdapterBoundaryHandoffTrendAdmissionSummary,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            adapter_boundary_handoff_trend_admission_history_record_telemetry(&dashboard, &health);

        AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_admission_with_health(
        &self,
        history: AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory,
        admission: &AgentAdapterBoundaryHandoffTrendAdmission,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecord {
        self.record_summary_with_health(history, admission.summary(), policy)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionGate;

impl AgentAdapterBoundaryHandoffTrendAdmissionGate {
    pub fn new() -> Self {
        Self
    }

    pub fn admit(
        &self,
        handoff: &AgentAdapterBoundaryHandoff,
        handoff_history_record: &AgentAdapterBoundaryHandoffHistoryRecord,
        trend_history: AgentAdapterBoundaryHandoffTrendGateSummaryHistory,
        trend_policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmission {
        let decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(handoff, handoff_history_record);
        let history_record = AgentAdapterBoundaryHandoffTrendGateHistoryRecorder::new()
            .record_decision_with_health(trend_history, &decision, trend_policy);
        let mut blocked_reasons = decision.blocked_reasons.clone();
        extend_ordered_unique(
            &mut blocked_reasons,
            history_record
                .health
                .reasons
                .iter()
                .map(|reason| format!("trend_gate_history:{reason}"))
                .collect(),
        );
        let history_repair_tasks = adapter_boundary_handoff_trend_gate_repair_tasks(
            &history_record.health,
            &blocked_reasons,
        );
        let mut next_queue = decision.next_queue.clone();
        for task in &history_repair_tasks {
            next_queue.push(task.clone());
        }
        let effective_admitted = decision.is_admitted() && history_record.allows_service_advance();
        let can_submit_memory_note = effective_admitted
            && history_record.health.is_stable()
            && decision.can_submit_memory_note;
        let can_promote_adaptive_state = effective_admitted
            && history_record.health.is_stable()
            && decision.can_promote_adaptive_state;
        let requires_repair_first =
            decision.requires_repair_first || history_record.requires_repair_first();
        let service_execution_command_reason_count =
            decision.service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count =
            decision.service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes =
            decision.service_execution_memory_promotion_command_reason_closes;
        let service_execution_tool_build_command_reason_count =
            decision.service_execution_tool_build_command_reason_count;
        let telemetry = adapter_boundary_handoff_trend_admission_telemetry(
            decision.requested_admitted,
            effective_admitted,
            history_record.health.status,
            requires_repair_first,
            can_submit_memory_note,
            can_promote_adaptive_state,
            history_repair_tasks.len(),
            next_queue.len(),
            blocked_reasons.len(),
            history_record.records(),
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
        );

        AgentAdapterBoundaryHandoffTrendAdmission {
            decision,
            history_record,
            effective_admitted,
            can_submit_memory_note,
            can_promote_adaptive_state,
            requires_repair_first,
            history_repair_tasks,
            next_queue,
            blocked_reasons,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionMonitorRecord {
    pub admission: AgentAdapterBoundaryHandoffTrendAdmission,
    pub history_record: AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecord,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionMonitorRecord {
    pub fn records(&self) -> usize {
        self.history_record.records()
    }

    pub fn is_admitted(&self) -> bool {
        self.admission.is_admitted() && self.history_record.health.is_stable()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.history_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.admission.requires_repair_first() || self.history_record.requires_repair_first()
    }

    pub fn next_queue(&self) -> &AgentTaskQueue {
        &self.admission.next_queue
    }

    pub fn continuation(
        &self,
        trend_policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
        admission_policy: AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionContinuation {
        AgentAdapterBoundaryHandoffTrendAdmissionContinuation::from_monitor_record(
            self,
            trend_policy,
            admission_policy,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionMonitor;

impl AgentAdapterBoundaryHandoffTrendAdmissionMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn monitor(
        &self,
        handoff: &AgentAdapterBoundaryHandoff,
        handoff_history_record: &AgentAdapterBoundaryHandoffHistoryRecord,
        trend_history: AgentAdapterBoundaryHandoffTrendGateSummaryHistory,
        trend_policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
        admission_history: AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory,
        admission_policy: AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionMonitorRecord {
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            handoff,
            handoff_history_record,
            trend_history,
            trend_policy,
        );
        let history_record = AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
            .record_admission_with_health(admission_history, &admission, admission_policy);
        let telemetry =
            adapter_boundary_handoff_trend_admission_monitor_telemetry(&admission, &history_record);

        AgentAdapterBoundaryHandoffTrendAdmissionMonitorRecord {
            service_execution_command_reason_count: admission
                .service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: admission
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: admission
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: admission
                .service_execution_tool_build_command_reason_count,
            admission,
            history_record,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionContinuation {
    pub next_queue: AgentTaskQueue,
    pub trend_history: AgentAdapterBoundaryHandoffTrendGateSummaryHistory,
    pub admission_history: AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory,
    pub trend_policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
    pub admission_policy: AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy,
    pub trend_health_status: AgentAdapterBoundaryStatus,
    pub admission_health_status: AgentAdapterBoundaryStatus,
    pub effective_admitted: bool,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionContinuation {
    pub fn from_monitor_record(
        record: &AgentAdapterBoundaryHandoffTrendAdmissionMonitorRecord,
        trend_policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
        admission_policy: AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy,
    ) -> Self {
        let effective_admitted = record.is_admitted();
        let can_submit_memory_note = effective_admitted && record.admission.can_submit_memory_note;
        let can_promote_adaptive_state =
            effective_admitted && record.admission.can_promote_adaptive_state;
        let requires_repair_first = record.requires_repair_first();
        let next_queue = record.admission.next_queue.clone();
        let trend_history = record.admission.history_record.history.clone();
        let admission_history = record.history_record.history.clone();
        let trend_health_status = record.admission.history_record.health.status;
        let admission_health_status = record.history_record.health.status;
        let service_execution_command_reason_count = record.service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count =
            record.service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes =
            record.service_execution_memory_promotion_command_reason_closes;
        let service_execution_tool_build_command_reason_count =
            record.service_execution_tool_build_command_reason_count;
        let telemetry = adapter_boundary_handoff_trend_admission_continuation_telemetry(
            trend_health_status,
            admission_health_status,
            effective_admitted,
            can_submit_memory_note,
            can_promote_adaptive_state,
            requires_repair_first,
            next_queue.len(),
            trend_history.len(),
            admission_history.len(),
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
        );

        Self {
            next_queue,
            trend_history,
            admission_history,
            trend_policy,
            admission_policy,
            trend_health_status,
            admission_health_status,
            effective_admitted,
            can_submit_memory_note,
            can_promote_adaptive_state,
            requires_repair_first,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry,
        }
    }

    pub fn is_admitted(&self) -> bool {
        self.effective_admitted && !self.requires_repair_first
    }

    pub fn resume_plan(&self) -> AgentAdapterBoundaryHandoffTrendAdmissionResumePlan {
        AgentAdapterBoundaryHandoffTrendAdmissionResumePlan::from_continuation(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionContinuationPlanner;

impl AgentAdapterBoundaryHandoffTrendAdmissionContinuationPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        record: &AgentAdapterBoundaryHandoffTrendAdmissionMonitorRecord,
        trend_policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
        admission_policy: AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionContinuation {
        record.continuation(trend_policy, admission_policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumePlan {
    pub prior_queue: AgentTaskQueue,
    pub trend_history: AgentAdapterBoundaryHandoffTrendGateSummaryHistory,
    pub admission_history: AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory,
    pub trend_policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
    pub admission_policy: AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy,
    pub prior_trend_health_status: AgentAdapterBoundaryStatus,
    pub prior_admission_health_status: AgentAdapterBoundaryStatus,
    pub prior_effective_admitted: bool,
    pub prior_can_submit_memory_note: bool,
    pub prior_can_promote_adaptive_state: bool,
    pub prior_requires_repair_first: bool,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumePlan {
    pub fn from_continuation(
        continuation: &AgentAdapterBoundaryHandoffTrendAdmissionContinuation,
    ) -> Self {
        let telemetry = adapter_boundary_handoff_trend_admission_resume_plan_telemetry(
            continuation.trend_health_status,
            continuation.admission_health_status,
            continuation.effective_admitted,
            continuation.can_submit_memory_note,
            continuation.can_promote_adaptive_state,
            continuation.requires_repair_first,
            continuation.next_queue.len(),
            continuation.trend_history.len(),
            continuation.admission_history.len(),
            continuation.service_execution_command_reason_count,
            continuation.service_execution_memory_promotion_command_reason_count,
            continuation.service_execution_memory_promotion_command_reason_closes,
            continuation.service_execution_tool_build_command_reason_count,
        );

        Self {
            prior_queue: continuation.next_queue.clone(),
            trend_history: continuation.trend_history.clone(),
            admission_history: continuation.admission_history.clone(),
            trend_policy: continuation.trend_policy,
            admission_policy: continuation.admission_policy,
            prior_trend_health_status: continuation.trend_health_status,
            prior_admission_health_status: continuation.admission_health_status,
            prior_effective_admitted: continuation.effective_admitted,
            prior_can_submit_memory_note: continuation.can_submit_memory_note,
            prior_can_promote_adaptive_state: continuation.can_promote_adaptive_state,
            prior_requires_repair_first: continuation.requires_repair_first,
            service_execution_command_reason_count: continuation
                .service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: continuation
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: continuation
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: continuation
                .service_execution_tool_build_command_reason_count,
            telemetry,
        }
    }

    pub fn monitor_next(
        &self,
        handoff: &AgentAdapterBoundaryHandoff,
        handoff_history_record: &AgentAdapterBoundaryHandoffHistoryRecord,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionMonitorRecord {
        AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            handoff,
            handoff_history_record,
            self.trend_history.clone(),
            self.trend_policy,
            self.admission_history.clone(),
            self.admission_policy,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumePlanner;

impl AgentAdapterBoundaryHandoffTrendAdmissionResumePlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        continuation: &AgentAdapterBoundaryHandoffTrendAdmissionContinuation,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumePlan {
        continuation.resume_plan()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeRecord {
    pub resume_plan: AgentAdapterBoundaryHandoffTrendAdmissionResumePlan,
    pub monitor_record: AgentAdapterBoundaryHandoffTrendAdmissionMonitorRecord,
    pub continuation: AgentAdapterBoundaryHandoffTrendAdmissionContinuation,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeRecord {
    pub fn summary(&self) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary {
        AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary::from_record(self)
    }

    pub fn is_admitted(&self) -> bool {
        self.continuation.is_admitted()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.continuation.requires_repair_first
    }

    pub fn next_queue(&self) -> &AgentTaskQueue {
        &self.continuation.next_queue
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary {
    pub prior_trend_health_status: AgentAdapterBoundaryStatus,
    pub prior_admission_health_status: AgentAdapterBoundaryStatus,
    pub next_trend_health_status: AgentAdapterBoundaryStatus,
    pub next_admission_health_status: AgentAdapterBoundaryStatus,
    pub effective_admitted: bool,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub prior_queue_tasks: usize,
    pub next_queue_tasks: usize,
    pub prior_trend_records: usize,
    pub prior_admission_records: usize,
    pub next_trend_records: usize,
    pub next_admission_records: usize,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary {
    pub fn from_record(record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeRecord) -> Self {
        let next_queue_task_ids = record.continuation.next_queue.task_ids();
        let telemetry = adapter_boundary_handoff_trend_admission_resume_summary_telemetry(
            record.resume_plan.prior_trend_health_status,
            record.resume_plan.prior_admission_health_status,
            record.continuation.trend_health_status,
            record.continuation.admission_health_status,
            record.continuation.is_admitted(),
            record.continuation.can_submit_memory_note,
            record.continuation.can_promote_adaptive_state,
            record.continuation.requires_repair_first,
            record.resume_plan.prior_queue.len(),
            next_queue_task_ids.len(),
            record.resume_plan.trend_history.len(),
            record.resume_plan.admission_history.len(),
            record.continuation.trend_history.len(),
            record.continuation.admission_history.len(),
            record.service_execution_command_reason_count,
            record.service_execution_memory_promotion_command_reason_count,
            record.service_execution_memory_promotion_command_reason_closes,
            record.service_execution_tool_build_command_reason_count,
        );

        Self {
            prior_trend_health_status: record.resume_plan.prior_trend_health_status,
            prior_admission_health_status: record.resume_plan.prior_admission_health_status,
            next_trend_health_status: record.continuation.trend_health_status,
            next_admission_health_status: record.continuation.admission_health_status,
            effective_admitted: record.continuation.is_admitted(),
            can_submit_memory_note: record.continuation.can_submit_memory_note,
            can_promote_adaptive_state: record.continuation.can_promote_adaptive_state,
            requires_repair_first: record.continuation.requires_repair_first,
            prior_queue_tasks: record.resume_plan.prior_queue.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            prior_trend_records: record.resume_plan.trend_history.len(),
            prior_admission_records: record.resume_plan.admission_history.len(),
            next_trend_records: record.continuation.trend_history.len(),
            next_admission_records: record.continuation.admission_history.len(),
            service_execution_command_reason_count: record.service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: record
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: record
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: record
                .service_execution_tool_build_command_reason_count,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory {
    summaries: Vec<AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeDashboard {
        AgentAdapterBoundaryHandoffTrendAdmissionResumeDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeDashboard {
    pub total_records: usize,
    pub effective_admitted_records: usize,
    pub repair_first_records: usize,
    pub memory_promotable_records: usize,
    pub adaptive_promotable_records: usize,
    pub next_queue_tasks: usize,
    pub latest_next_trend_health_status: Option<AgentAdapterBoundaryStatus>,
    pub latest_next_admission_health_status: Option<AgentAdapterBoundaryStatus>,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub effective_admitted_rate: f32,
    pub repair_first_rate: f32,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeDashboard {
    pub fn from_summaries(
        summaries: &[AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary],
    ) -> Self {
        let total_records = summaries.len();
        let effective_admitted_records = summaries
            .iter()
            .filter(|summary| summary.effective_admitted && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let memory_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_submit_memory_note)
            .count();
        let adaptive_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let latest_next_trend_health_status = summaries
            .last()
            .map(|summary| summary.next_trend_health_status);
        let latest_next_admission_health_status = summaries
            .last()
            .map(|summary| summary.next_admission_health_status);
        let service_execution_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_closes = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_closes)
            .sum::<usize>();
        let service_execution_tool_build_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_tool_build_command_reason_count)
            .sum::<usize>();
        let effective_admitted_rate = rate(effective_admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let telemetry = adapter_boundary_handoff_trend_admission_resume_dashboard_telemetry(
            total_records,
            effective_admitted_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            next_queue_tasks,
            effective_admitted_rate,
            repair_first_rate,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
        );

        Self {
            total_records,
            effective_admitted_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            next_queue_tasks,
            latest_next_trend_health_status,
            latest_next_admission_health_status,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            effective_admitted_rate,
            repair_first_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeHealth {
        AgentAdapterBoundaryHandoffTrendAdmissionResumeHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy {
    pub minimum_effective_admitted_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_next_queue_tasks: usize,
}

impl Default for AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_effective_admitted_rate: 0.67,
            maximum_repair_first_records: 0,
            maximum_next_queue_tasks: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeHealth {
    pub status: AgentAdapterBoundaryStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentAdapterBoundaryHandoffTrendAdmissionResumeDashboard,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeHealth {
    pub fn from_dashboard(
        dashboard: AgentAdapterBoundaryHandoffTrendAdmissionResumeDashboard,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons
                .push("adapter_boundary_handoff_trend_admission_resume_history_empty".to_owned());
        } else if dashboard.effective_admitted_rate < policy.minimum_effective_admitted_rate {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_effective_admitted_rate={:.3}<{}",
                dashboard.effective_admitted_rate, policy.minimum_effective_admitted_rate
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.next_queue_tasks > policy.maximum_next_queue_tasks {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_next_queue_tasks={}>{}",
                dashboard.next_queue_tasks, policy.maximum_next_queue_tasks
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentAdapterBoundaryStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentAdapterBoundaryStatus::Watch, watch_reasons)
        } else {
            (AgentAdapterBoundaryStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentAdapterBoundaryStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecord {
    pub history: AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory,
    pub appended_summary: AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary,
    pub dashboard: AgentAdapterBoundaryHandoffTrendAdmissionResumeDashboard,
    pub health: AgentAdapterBoundaryHandoffTrendAdmissionResumeHealth,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecord {
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
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder;

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory,
        summary: AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = adapter_boundary_handoff_trend_admission_resume_history_record_telemetry(
            &dashboard, &health,
        );

        AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_resume_with_health(
        &self,
        history: AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory,
        record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeRecord,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecord {
        self.record_summary_with_health(history, record.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDecision {
    pub requested_admitted: bool,
    pub effective_admitted: bool,
    pub resume_health: AgentAdapterBoundaryHandoffTrendAdmissionResumeHealth,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.effective_admitted && !self.requires_repair_first
    }

    pub fn summary(&self) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary {
        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary::from_decision(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary {
    pub resume_health_status: AgentAdapterBoundaryStatus,
    pub requested_admitted: bool,
    pub effective_admitted: bool,
    pub requires_repair_first: bool,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary {
    pub fn from_decision(
        decision: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDecision,
    ) -> Self {
        let repair_task_ids = decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = decision.next_queue.task_ids();
        let telemetry = adapter_boundary_handoff_trend_admission_resume_gate_summary_telemetry(
            decision.resume_health.status,
            decision.requested_admitted,
            decision.effective_admitted,
            decision.requires_repair_first,
            decision.can_submit_memory_note,
            decision.can_promote_adaptive_state,
            repair_task_ids.len(),
            next_queue_task_ids.len(),
            decision.blocked_reasons.len(),
            decision.service_execution_command_reason_count,
            decision.service_execution_memory_promotion_command_reason_count,
            decision.service_execution_memory_promotion_command_reason_closes,
            decision.service_execution_tool_build_command_reason_count,
        );

        Self {
            resume_health_status: decision.resume_health.status,
            requested_admitted: decision.requested_admitted,
            effective_admitted: decision.effective_admitted,
            requires_repair_first: decision.requires_repair_first,
            can_submit_memory_note: decision.can_submit_memory_note,
            can_promote_adaptive_state: decision.can_promote_adaptive_state,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: decision.blocked_reasons.len(),
            service_execution_command_reason_count: decision.service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: decision
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: decision
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: decision
                .service_execution_tool_build_command_reason_count,
            repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory {
    summaries: Vec<AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDashboard {
        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDashboard::from_summaries(
            &self.summaries,
        )
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDashboard {
    pub total_records: usize,
    pub requested_admitted_records: usize,
    pub effective_admitted_records: usize,
    pub repair_first_records: usize,
    pub memory_promotable_records: usize,
    pub adaptive_promotable_records: usize,
    pub repair_task_count: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub latest_resume_health_status: Option<AgentAdapterBoundaryStatus>,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub effective_admitted_rate: f32,
    pub repair_first_rate: f32,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDashboard {
    pub fn from_summaries(
        summaries: &[AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary],
    ) -> Self {
        let total_records = summaries.len();
        let requested_admitted_records = summaries
            .iter()
            .filter(|summary| summary.requested_admitted)
            .count();
        let effective_admitted_records = summaries
            .iter()
            .filter(|summary| summary.effective_admitted && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let memory_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_submit_memory_note)
            .count();
        let adaptive_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reasons = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let latest_resume_health_status =
            summaries.last().map(|summary| summary.resume_health_status);
        let service_execution_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_closes = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_closes)
            .sum::<usize>();
        let service_execution_tool_build_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_tool_build_command_reason_count)
            .sum::<usize>();
        let effective_admitted_rate = rate(effective_admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let telemetry = adapter_boundary_handoff_trend_admission_resume_gate_dashboard_telemetry(
            total_records,
            requested_admitted_records,
            effective_admitted_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            repair_task_count,
            next_queue_tasks,
            blocked_reasons,
            effective_admitted_rate,
            repair_first_rate,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
        );

        Self {
            total_records,
            requested_admitted_records,
            effective_admitted_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            repair_task_count,
            next_queue_tasks,
            blocked_reasons,
            latest_resume_health_status,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            effective_admitted_rate,
            repair_first_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealth {
        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy {
    pub minimum_effective_admitted_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_effective_admitted_rate: 0.67,
            maximum_repair_first_records: 0,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealth {
    pub status: AgentAdapterBoundaryStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDashboard,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealth {
    pub fn from_dashboard(
        dashboard: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDashboard,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "adapter_boundary_handoff_trend_admission_resume_gate_history_empty".to_owned(),
            );
        } else if dashboard.effective_admitted_rate < policy.minimum_effective_admitted_rate {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_gate_effective_admitted_rate={:.3}<{}",
                dashboard.effective_admitted_rate, policy.minimum_effective_admitted_rate
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_gate_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_gate_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_gate_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentAdapterBoundaryStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentAdapterBoundaryStatus::Watch, watch_reasons)
        } else {
            (AgentAdapterBoundaryStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentAdapterBoundaryStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecord {
    pub history: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory,
    pub appended_summary: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary,
    pub dashboard: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDashboard,
    pub health: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealth,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecord {
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
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecorder;

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory,
        summary: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            adapter_boundary_handoff_trend_admission_resume_gate_history_record_telemetry(
                &dashboard, &health,
            );

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_decision_with_health(
        &self,
        history: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory,
        decision: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDecision,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecord {
        self.record_summary_with_health(history, decision.summary(), policy)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGate;

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        resume_record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeRecord,
        history_record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecord,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDecision {
        let requested_admitted = resume_record.is_admitted();
        let resume_health = history_record.health.clone();
        let blocked_reasons = resume_health
            .reasons
            .iter()
            .map(|reason| format!("resume_history:{reason}"))
            .collect::<Vec<_>>();
        let requires_repair_first =
            resume_record.requires_repair_first() || resume_health.requires_repair_first();
        let repair_tasks = adapter_boundary_handoff_trend_admission_resume_repair_tasks(
            &resume_health,
            &blocked_reasons,
        );
        let mut next_queue = resume_record.continuation.next_queue.clone();
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let effective_admitted =
            requested_admitted && resume_health.allows_service_advance() && !requires_repair_first;
        let can_submit_memory_note = effective_admitted
            && resume_health.is_stable()
            && resume_record.continuation.can_submit_memory_note;
        let can_promote_adaptive_state = effective_admitted
            && resume_health.is_stable()
            && resume_record.continuation.can_promote_adaptive_state;
        let telemetry = adapter_boundary_handoff_trend_admission_resume_gate_telemetry(
            requested_admitted,
            effective_admitted,
            resume_health.status,
            can_submit_memory_note,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            blocked_reasons.len(),
            history_record
                .dashboard
                .service_execution_command_reason_count,
            history_record
                .dashboard
                .service_execution_memory_promotion_command_reason_count,
            history_record
                .dashboard
                .service_execution_memory_promotion_command_reason_closes,
            history_record
                .dashboard
                .service_execution_tool_build_command_reason_count,
        );

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDecision {
            requested_admitted,
            effective_admitted,
            resume_health,
            can_submit_memory_note,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks,
            next_queue,
            blocked_reasons,
            service_execution_command_reason_count: history_record
                .dashboard
                .service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: history_record
                .dashboard
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: history_record
                .dashboard
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: history_record
                .dashboard
                .service_execution_tool_build_command_reason_count,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorRecord {
    pub decision: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDecision,
    pub history_record: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecord,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorRecord {
    pub fn records(&self) -> usize {
        self.history_record.records()
    }

    pub fn is_admitted(&self) -> bool {
        self.decision.is_admitted() && self.history_record.health.is_stable()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.history_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.decision.requires_repair_first || self.history_record.requires_repair_first()
    }

    pub fn can_submit_memory_note(&self) -> bool {
        self.decision.can_submit_memory_note && self.history_record.health.is_stable()
    }

    pub fn can_promote_adaptive_state(&self) -> bool {
        self.decision.can_promote_adaptive_state && self.history_record.health.is_stable()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGateDecision {
    pub requested_admitted: bool,
    pub effective_admitted: bool,
    pub gate_health: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealth,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.effective_admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGate;

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        monitor_record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorRecord,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGateDecision {
        let requested_admitted = monitor_record.decision.is_admitted();
        let gate_health = monitor_record.history_record.health.clone();
        let blocked_reasons = gate_health
            .reasons
            .iter()
            .map(|reason| format!("resume_gate_history:{reason}"))
            .collect::<Vec<_>>();
        let requires_repair_first =
            monitor_record.decision.requires_repair_first || gate_health.requires_repair_first();
        let repair_tasks =
            adapter_boundary_handoff_trend_admission_resume_gate_monitor_repair_tasks(
                &gate_health,
                &blocked_reasons,
            );
        let mut next_queue = monitor_record.decision.next_queue.clone();
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let effective_admitted =
            requested_admitted && gate_health.allows_service_advance() && !requires_repair_first;
        let can_submit_memory_note = effective_admitted
            && gate_health.is_stable()
            && monitor_record.decision.can_submit_memory_note;
        let can_promote_adaptive_state = effective_admitted
            && gate_health.is_stable()
            && monitor_record.decision.can_promote_adaptive_state;
        let telemetry = adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_telemetry(
            requested_admitted,
            effective_admitted,
            gate_health.status,
            can_submit_memory_note,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            blocked_reasons.len(),
            monitor_record.service_execution_command_reason_count,
            monitor_record.service_execution_memory_promotion_command_reason_count,
            monitor_record.service_execution_memory_promotion_command_reason_closes,
            monitor_record.service_execution_tool_build_command_reason_count,
        );

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGateDecision {
            requested_admitted,
            effective_admitted,
            gate_health,
            can_submit_memory_note,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks,
            next_queue,
            blocked_reasons,
            service_execution_command_reason_count: monitor_record
                .service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: monitor_record
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: monitor_record
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: monitor_record
                .service_execution_tool_build_command_reason_count,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitor;

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn monitor(
        &self,
        resume_record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeRecord,
        resume_history_record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecord,
        gate_history: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory,
        gate_policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorRecord {
        let decision = AgentAdapterBoundaryHandoffTrendAdmissionResumeGate::new()
            .gate(resume_record, resume_history_record);
        let history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecorder::new()
                .record_decision_with_health(gate_history, &decision, gate_policy);
        let telemetry = adapter_boundary_handoff_trend_admission_resume_gate_monitor_telemetry(
            &decision,
            &history_record,
        );
        let service_execution_command_reason_count = history_record
            .dashboard
            .service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count = history_record
            .dashboard
            .service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes = history_record
            .dashboard
            .service_execution_memory_promotion_command_reason_closes;
        let service_execution_tool_build_command_reason_count = history_record
            .dashboard
            .service_execution_tool_build_command_reason_count;

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorRecord {
            decision,
            history_record,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord {
    pub monitor_record: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorRecord,
    pub gate_decision: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGateDecision,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord {
    pub fn records(&self) -> usize {
        self.monitor_record.records()
    }

    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.gate_decision.gate_health.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn can_submit_memory_note(&self) -> bool {
        self.gate_decision.can_submit_memory_note
    }

    pub fn can_promote_adaptive_state(&self) -> bool {
        self.gate_decision.can_promote_adaptive_state
    }

    pub fn summary(
        &self,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary {
        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary::from_handoff(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary {
    pub gate_health_status: AgentAdapterBoundaryStatus,
    pub requested_admitted: bool,
    pub effective_admitted: bool,
    pub requires_repair_first: bool,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub records: usize,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary {
    pub fn from_handoff(
        record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord,
    ) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry =
            adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_telemetry(
                record.gate_decision.gate_health.status,
                record.gate_decision.requested_admitted,
                record.gate_decision.is_admitted(),
                record.gate_decision.requires_repair_first,
                record.gate_decision.can_submit_memory_note,
                record.gate_decision.can_promote_adaptive_state,
                record.records(),
                repair_task_ids.len(),
                next_queue_task_ids.len(),
                record.gate_decision.blocked_reasons.len(),
                record.service_execution_command_reason_count,
                record.service_execution_memory_promotion_command_reason_count,
                record.service_execution_memory_promotion_command_reason_closes,
                record.service_execution_tool_build_command_reason_count,
            );

        Self {
            gate_health_status: record.gate_decision.gate_health.status,
            requested_admitted: record.gate_decision.requested_admitted,
            effective_admitted: record.gate_decision.is_admitted(),
            requires_repair_first: record.gate_decision.requires_repair_first,
            can_submit_memory_note: record.gate_decision.can_submit_memory_note,
            can_promote_adaptive_state: record.gate_decision.can_promote_adaptive_state,
            records: record.records(),
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: record.gate_decision.blocked_reasons.len(),
            service_execution_command_reason_count: record.service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: record
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: record
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: record
                .service_execution_tool_build_command_reason_count,
            repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory {
    summaries: Vec<AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(
        &mut self,
        summary: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary,
    ) {
        self.summaries.push(summary);
    }

    pub fn latest(
        &self,
    ) -> Option<&AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary> {
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
    ) -> &[AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(
        &self,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffDashboard {
        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffDashboard::from_summaries(
            &self.summaries,
        )
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffDashboard {
    pub total_records: usize,
    pub requested_admitted_records: usize,
    pub effective_admitted_records: usize,
    pub repair_first_records: usize,
    pub memory_promotable_records: usize,
    pub adaptive_promotable_records: usize,
    pub stable_records: usize,
    pub watch_records: usize,
    pub repair_records: usize,
    pub recorded_gate_rows: usize,
    pub repair_task_count: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub latest_gate_health_status: Option<AgentAdapterBoundaryStatus>,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub service_execution_tool_build_command_reason_count: usize,
    pub effective_admitted_rate: f32,
    pub repair_first_rate: f32,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffDashboard {
    pub fn from_summaries(
        summaries: &[AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary],
    ) -> Self {
        let total_records = summaries.len();
        let requested_admitted_records = summaries
            .iter()
            .filter(|summary| summary.requested_admitted)
            .count();
        let effective_admitted_records = summaries
            .iter()
            .filter(|summary| summary.effective_admitted && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let memory_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_submit_memory_note)
            .count();
        let adaptive_promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let stable_records = summaries
            .iter()
            .filter(|summary| summary.gate_health_status == AgentAdapterBoundaryStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.gate_health_status == AgentAdapterBoundaryStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.gate_health_status == AgentAdapterBoundaryStatus::Repair)
            .count();
        let recorded_gate_rows = summaries
            .iter()
            .map(|summary| summary.records)
            .sum::<usize>();
        let repair_task_count = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let blocked_reasons = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let latest_gate_health_status = summaries.last().map(|summary| summary.gate_health_status);
        let service_execution_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_count)
            .sum::<usize>();
        let service_execution_memory_promotion_command_reason_closes = summaries
            .iter()
            .map(|summary| summary.service_execution_memory_promotion_command_reason_closes)
            .sum::<usize>();
        let service_execution_tool_build_command_reason_count = summaries
            .iter()
            .map(|summary| summary.service_execution_tool_build_command_reason_count)
            .sum::<usize>();
        let effective_admitted_rate = rate(effective_admitted_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let telemetry =
            adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_telemetry(
                total_records,
                requested_admitted_records,
                effective_admitted_records,
                repair_first_records,
                memory_promotable_records,
                adaptive_promotable_records,
                stable_records,
                watch_records,
                repair_records,
                recorded_gate_rows,
                repair_task_count,
                next_queue_tasks,
                blocked_reasons,
                effective_admitted_rate,
                repair_first_rate,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
        );

        Self {
            total_records,
            requested_admitted_records,
            effective_admitted_records,
            repair_first_records,
            memory_promotable_records,
            adaptive_promotable_records,
            stable_records,
            watch_records,
            repair_records,
            recorded_gate_rows,
            repair_task_count,
            next_queue_tasks,
            blocked_reasons,
            latest_gate_health_status,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            effective_admitted_rate,
            repair_first_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealth {
        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy {
    pub minimum_effective_admitted_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_effective_admitted_rate: 0.67,
            maximum_repair_first_records: 0,
            maximum_repair_records: 0,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: 0,
            maximum_watch_records: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealth {
    pub status: AgentAdapterBoundaryStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffDashboard,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffDashboard,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_empty"
                    .to_owned(),
            );
        } else if dashboard.effective_admitted_rate < policy.minimum_effective_admitted_rate {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_effective_admitted_rate={:.3}<{}",
                dashboard.effective_admitted_rate, policy.minimum_effective_admitted_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentAdapterBoundaryStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentAdapterBoundaryStatus::Watch, watch_reasons)
        } else {
            (AgentAdapterBoundaryStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentAdapterBoundaryStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentAdapterBoundaryStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecord {
    pub history: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory,
    pub appended_summary: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary,
    pub dashboard: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffDashboard,
    pub health: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealth,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecord {
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
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecorder;

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory,
        summary: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_record_telemetry(
                &dashboard, &health,
            );

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_handoff_with_health(
        &self,
        history: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory,
        handoff: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecord {
        self.record_summary_with_health(history, handoff.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGateDecision {
    pub requested_admitted: bool,
    pub effective_admitted: bool,
    pub handoff_health: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealth,
    pub can_submit_memory_note: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.effective_admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGate;

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        handoff: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord,
        history_record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecord,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGateDecision {
        let requested_admitted = handoff.is_admitted();
        let handoff_health = history_record.health.clone();
        let mut blocked_reasons = handoff.gate_decision.blocked_reasons.clone();
        extend_ordered_unique(
            &mut blocked_reasons,
            handoff_health
                .reasons
                .iter()
                .map(|reason| format!("resume_gate_monitor_handoff_history:{reason}"))
                .collect::<Vec<_>>(),
        );
        let requires_repair_first =
            handoff.requires_repair_first() || handoff_health.requires_repair_first();
        let repair_tasks =
            adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_tasks(
                &handoff_health,
                &blocked_reasons,
            );
        let next_queue = handoff
            .gate_decision
            .next_queue
            .clone()
            .with_repair_first(&repair_tasks);
        let effective_admitted =
            requested_admitted && handoff_health.allows_service_advance() && !requires_repair_first;
        let can_submit_memory_note =
            effective_admitted && handoff_health.is_stable() && handoff.can_submit_memory_note();
        let can_promote_adaptive_state = effective_admitted
            && handoff_health.is_stable()
            && handoff.can_promote_adaptive_state();
        let telemetry =
            adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_telemetry(
                handoff_health.status,
                requested_admitted,
                effective_admitted,
                can_submit_memory_note,
                can_promote_adaptive_state,
                requires_repair_first,
                repair_tasks.len(),
                next_queue.len(),
                blocked_reasons.len(),
                history_record
                    .dashboard
                    .service_execution_command_reason_count,
                history_record
                    .dashboard
                    .service_execution_memory_promotion_command_reason_count,
                history_record
                    .dashboard
                    .service_execution_memory_promotion_command_reason_closes,
            );

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGateDecision {
            requested_admitted,
            effective_admitted,
            handoff_health,
            can_submit_memory_note,
            can_promote_adaptive_state,
            requires_repair_first,
            repair_tasks,
            next_queue,
            blocked_reasons,
            service_execution_command_reason_count: history_record
                .dashboard
                .service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: history_record
                .dashboard
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: history_record
                .dashboard
                .service_execution_memory_promotion_command_reason_closes,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoffRecord {
    pub handoff: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord,
    pub history_record:
        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecord,
    pub gate_decision:
        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGateDecision,
    pub service_execution_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_count: usize,
    pub service_execution_memory_promotion_command_reason_closes: usize,
    pub telemetry: Vec<String>,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoffRecord {
    pub fn records(&self) -> usize {
        self.history_record.records()
    }

    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.gate_decision.handoff_health.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn can_submit_memory_note(&self) -> bool {
        self.gate_decision.can_submit_memory_note
    }

    pub fn can_promote_adaptive_state(&self) -> bool {
        self.gate_decision.can_promote_adaptive_state
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff {
    history_recorder:
        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecorder,
    gate: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGate,
}

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        handoff: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord,
        history: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory,
        policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoffRecord {
        let history_record = self
            .history_recorder
            .record_handoff_with_health(history, &handoff, policy);
        let gate_decision = self.gate.gate(&handoff, &history_record);
        let telemetry =
            adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_telemetry(
                &handoff,
                &history_record,
                &gate_decision,
            );
        let service_execution_command_reason_count =
            gate_decision.service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count =
            gate_decision.service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes =
            gate_decision.service_execution_memory_promotion_command_reason_closes;

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoffRecord {
            handoff,
            history_record,
            gate_decision,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff;

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff {
    pub fn new() -> Self {
        Self
    }

    pub fn record_and_gate(
        &self,
        resume_record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeRecord,
        resume_history_record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecord,
        gate_history: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory,
        gate_policy: AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord {
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitor::new()
            .monitor(
                resume_record,
                resume_history_record,
                gate_history,
                gate_policy,
            );
        let gate_decision = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGate::new()
            .gate(&monitor_record);
        let telemetry =
            adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_telemetry(
                &monitor_record,
                &gate_decision,
            );
        let service_execution_command_reason_count =
            gate_decision.service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count =
            gate_decision.service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes =
            gate_decision.service_execution_memory_promotion_command_reason_closes;
        let service_execution_tool_build_command_reason_count =
            gate_decision.service_execution_tool_build_command_reason_count;

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord {
            monitor_record,
            gate_decision,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner;

impl AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner {
    pub fn new() -> Self {
        Self
    }

    pub fn run(
        &self,
        continuation: &AgentAdapterBoundaryHandoffTrendAdmissionContinuation,
        handoff: &AgentAdapterBoundaryHandoff,
        handoff_history_record: &AgentAdapterBoundaryHandoffHistoryRecord,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeRecord {
        let resume_plan = continuation.resume_plan();
        let monitor_record = resume_plan.monitor_next(handoff, handoff_history_record);
        let next_continuation =
            monitor_record.continuation(resume_plan.trend_policy, resume_plan.admission_policy);
        let telemetry = adapter_boundary_handoff_trend_admission_resume_record_telemetry(
            &resume_plan,
            &monitor_record,
            &next_continuation,
        );

        AgentAdapterBoundaryHandoffTrendAdmissionResumeRecord {
            resume_plan,
            monitor_record,
            service_execution_command_reason_count: next_continuation
                .service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: next_continuation
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: next_continuation
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: next_continuation
                .service_execution_tool_build_command_reason_count,
            continuation: next_continuation,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendGateHistoryRecorder;

impl AgentAdapterBoundaryHandoffTrendGateHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentAdapterBoundaryHandoffTrendGateSummaryHistory,
        summary: AgentAdapterBoundaryHandoffTrendGateSummary,
        policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendGateHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            adapter_boundary_handoff_trend_gate_history_record_telemetry(&dashboard, &health);

        AgentAdapterBoundaryHandoffTrendGateHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_decision_with_health(
        &self,
        history: AgentAdapterBoundaryHandoffTrendGateSummaryHistory,
        decision: &AgentAdapterBoundaryHandoffTrendGateDecision,
        policy: AgentAdapterBoundaryHandoffTrendGateHealthPolicy,
    ) -> AgentAdapterBoundaryHandoffTrendGateHistoryRecord {
        self.record_summary_with_health(history, decision.summary(), policy)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundaryHandoffTrendGate;

impl AgentAdapterBoundaryHandoffTrendGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        handoff: &AgentAdapterBoundaryHandoff,
        history_record: &AgentAdapterBoundaryHandoffHistoryRecord,
    ) -> AgentAdapterBoundaryHandoffTrendGateDecision {
        let requested_admitted = handoff.is_admitted();
        let handoff_health = history_record.health.clone();
        let requires_repair_first =
            handoff.requires_repair_first || handoff_health.requires_repair_first();
        let mut blocked_reasons = handoff.blocked_reasons.clone();
        extend_ordered_unique(
            &mut blocked_reasons,
            handoff_health
                .reasons
                .iter()
                .map(|reason| format!("handoff_history:{reason}"))
                .collect(),
        );
        let repair_tasks =
            adapter_boundary_handoff_trend_repair_tasks(&handoff_health, &blocked_reasons);
        let mut next_queue = handoff.next_queue.clone();
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let effective_admitted = requested_admitted && handoff_health.allows_service_advance();
        let can_submit_memory_note =
            effective_admitted && handoff_health.is_stable() && handoff.can_submit_memory_note();
        let can_promote_adaptive_state = effective_admitted
            && handoff_health.is_stable()
            && handoff.can_promote_adaptive_state();
        let telemetry = adapter_boundary_handoff_trend_gate_telemetry(
            requested_admitted,
            effective_admitted,
            handoff_health.status,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            blocked_reasons.len(),
            can_submit_memory_note,
            can_promote_adaptive_state,
            history_record
                .dashboard
                .service_execution_command_reason_count,
            history_record
                .dashboard
                .service_execution_memory_promotion_command_reason_count,
            history_record
                .dashboard
                .service_execution_memory_promotion_command_reason_closes,
            history_record
                .dashboard
                .service_execution_tool_build_command_reason_count,
        );

        AgentAdapterBoundaryHandoffTrendGateDecision {
            requested_admitted,
            effective_admitted,
            handoff_health,
            requires_repair_first,
            can_submit_memory_note,
            can_promote_adaptive_state,
            repair_tasks,
            next_queue,
            blocked_reasons,
            service_execution_command_reason_count: history_record
                .dashboard
                .service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count: history_record
                .dashboard
                .service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes: history_record
                .dashboard
                .service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count: history_record
                .dashboard
                .service_execution_tool_build_command_reason_count,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentAdapterBoundarySummaryHistoryRecorder;

impl AgentAdapterBoundarySummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentAdapterBoundarySummaryHistory,
        summary: AgentAdapterBoundarySummary,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundarySummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = adapter_boundary_history_record_telemetry(&dashboard, &health);

        AgentAdapterBoundarySummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_snapshot_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        snapshot: &AgentAdapterBoundarySnapshot,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundarySummaryHistoryRecord {
        self.record_summary_with_health(history, snapshot.summary(), policy)
    }

    pub fn record_snapshot_boundary_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        snapshot: AgentAdapterBoundarySnapshot,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let history_record = self.record_snapshot_with_health(history, &snapshot, policy);
        let telemetry = adapter_boundary_record_telemetry(&snapshot, &history_record);

        AgentAdapterBoundaryRecord {
            snapshot,
            history_record,
            telemetry,
        }
    }

    pub fn record_boundary_gates_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        dispatch_gate: &TaskDispatchGateDecision,
        memory_submission_gate: &MemorySubmissionGateDecision,
        report_gate: &AgentReportGateDecision,
        service_admission: &AgentCollaborationAdapterSideEffectAdmission,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_boundary_gates(
            next_queue,
            dispatch_gate,
            memory_submission_gate,
            report_gate,
            service_admission,
        );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_dispatch_and_run_ledger_admission_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        dispatch_gate: &TaskDispatchGateDecision,
        admission: &AgentRunLedgerAdmission,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_dispatch_and_run_ledger_admission(
            next_queue,
            dispatch_gate,
            admission,
        );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_cycle_report_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        report: &AgentCycleReport,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_cycle_report(next_queue, report);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_evolution_admission_handoff_history_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_evolution_admission_handoff_history(
            next_queue, record,
        );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_evolution_admission_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &EvolutionAdmissionRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_evolution_admission(next_queue, record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_budget_ledger_history_gate_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &BudgetLedgerHistoryGateRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_budget_ledger_history_gate(next_queue, record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_recursive_schedule_history_gate_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &RecursiveAgentScheduleHistoryGateRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_recursive_schedule_history_gate(next_queue, record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_reflection_loop_history_gate_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &ReflectionLoopHistoryGateRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_reflection_loop_history_gate(next_queue, record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_tool_build_report_history_gate_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &ToolBuildReportHistoryGateRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_tool_build_report_history_gate(next_queue, record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_toolsmith_plan_history_gate_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &ToolsmithPlanHistoryGateRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_toolsmith_plan_history_gate(next_queue, record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_process_reward_report_history_gate_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &ProcessRewardReportHistoryGateRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_process_reward_report_history_gate(
            next_queue, record,
        );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_reflection_reward_admission_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &ReflectionRewardAdmissionRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_reflection_reward_admission(next_queue, record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_conflict_report_history_gate_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &ConflictReportHistoryGateRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_conflict_report_history_gate(next_queue, record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_aggregation_conflict_review_trend_gate_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        decision: &AggregationConflictReviewTrendGateDecision,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_aggregation_conflict_review_trend_gate(
            next_queue, decision,
        );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_report_and_tool_build_gates_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        report_gate: &AgentReportGateDecision,
        tool_build_record: &ToolBuildReportHistoryGateRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_report_and_tool_build_gates(
            next_queue,
            report_gate,
            tool_build_record,
        );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_run_report_final_gate_decision_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        decision: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_run_report_final_gate_decision(decision);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_run_report_final_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_run_report_final_handoff(record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_service_execution_final_gate_decision_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        decision: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_service_execution_final_gate_decision(decision);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_service_execution_final_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoffRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_service_execution_final_handoff(record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_control_plan_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        plan: &AgentClosedLoopRuntimeServiceLoopRunControlPlan,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_control_plan(plan);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_control_record_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentClosedLoopRuntimeServiceLoopRunControlRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_control_record(record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_preflight_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        preflight: &AgentClosedLoopRuntimeServicePreflight,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_preflight(preflight);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_preflight_continuation_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        continuation: &AgentClosedLoopRuntimeServicePreflightContinuation,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_preflight_continuation(continuation);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_state_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        state: &AgentClosedLoopRuntimeServiceLoopState,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_state(state);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_advance_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        advance: &AgentClosedLoopRuntimeServiceLoopAdvance,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_advance(advance);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_daemon_record_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_record(record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_daemon_continuation_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_continuation(
            continuation,
        );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_daemon_input_plan_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_input_plan(plan);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_daemon_request_plan_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_plan(plan);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_daemon_request_record_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_record(record);

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_daemon_request_monitored_plan_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_monitored_plan(
                plan,
            );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_daemon_request_monitored_record_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_monitored_record(
                record,
            );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_daemon_request_monitored_close_plan_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_monitored_close_plan(
                plan,
            );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_daemon_request_monitored_close_run_record_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_monitored_close_run_record(
            record,
        );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_runtime_service_loop_daemon_request_monitored_close_continuation_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryRecord {
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_monitored_close_continuation(
                continuation,
            );

        self.record_snapshot_boundary_with_health(history, snapshot, policy)
    }

    pub fn record_boundary_gates_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        dispatch_gate: &TaskDispatchGateDecision,
        memory_submission_gate: &MemorySubmissionGateDecision,
        report_gate: &AgentReportGateDecision,
        service_admission: &AgentCollaborationAdapterSideEffectAdmission,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self.record_boundary_gates_with_health(
            history,
            next_queue,
            dispatch_gate,
            memory_submission_gate,
            report_gate,
            service_admission,
            policy,
        );

        AgentAdapterBoundaryHandoff::from_record_and_queue(boundary_record, next_queue)
    }

    pub fn record_run_report_final_gate_decision_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        decision: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record =
            self.record_run_report_final_gate_decision_with_health(history, decision, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(boundary_record, &decision.next_queue)
    }

    pub fn record_run_report_final_handoff_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record =
            self.record_run_report_final_handoff_with_health(history, record, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            &record.gate_decision.next_queue,
        )
    }

    pub fn record_service_execution_final_gate_decision_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        decision: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self
            .record_service_execution_final_gate_decision_with_health(history, decision, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(boundary_record, &decision.next_queue)
    }

    pub fn record_service_execution_final_handoff_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoffRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record =
            self.record_service_execution_final_handoff_with_health(history, record, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            &record.gate_decision.next_queue,
        )
    }

    pub fn record_runtime_service_loop_control_plan_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        plan: &AgentClosedLoopRuntimeServiceLoopRunControlPlan,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record =
            self.record_runtime_service_loop_control_plan_with_health(history, plan, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(boundary_record, &plan.next_queue)
    }

    pub fn record_runtime_service_loop_control_record_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentClosedLoopRuntimeServiceLoopRunControlRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record =
            self.record_runtime_service_loop_control_record_with_health(history, record, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            &record.control_plan.next_queue,
        )
    }

    pub fn record_runtime_service_preflight_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        preflight: &AgentClosedLoopRuntimeServicePreflight,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record =
            self.record_runtime_service_preflight_with_health(history, preflight, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            &preflight.turn_plan.next_queue,
        )
    }

    pub fn record_runtime_service_preflight_continuation_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        continuation: &AgentClosedLoopRuntimeServicePreflightContinuation,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self.record_runtime_service_preflight_continuation_with_health(
            history,
            continuation,
            policy,
        );

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            &continuation.next_runtime_input.next_queue,
        )
    }

    pub fn record_runtime_service_loop_state_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        state: &AgentClosedLoopRuntimeServiceLoopState,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record =
            self.record_runtime_service_loop_state_with_health(history, state, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            &state.next_runtime_input().next_queue,
        )
    }

    pub fn record_runtime_service_loop_advance_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        advance: &AgentClosedLoopRuntimeServiceLoopAdvance,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record =
            self.record_runtime_service_loop_advance_with_health(history, advance, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            &advance.next_runtime_input().next_queue,
        )
    }

    pub fn record_runtime_service_loop_daemon_record_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record =
            self.record_runtime_service_loop_daemon_record_with_health(history, record, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            runtime_service_loop_daemon_record_next_queue(record),
        )
    }

    pub fn record_runtime_service_loop_daemon_continuation_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self.record_runtime_service_loop_daemon_continuation_with_health(
            history,
            continuation,
            policy,
        );

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            runtime_service_loop_daemon_continuation_next_queue(continuation),
        )
    }

    pub fn record_runtime_service_loop_daemon_input_plan_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record =
            self.record_runtime_service_loop_daemon_input_plan_with_health(history, plan, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            runtime_service_loop_daemon_input_plan_next_queue(plan),
        )
    }

    pub fn record_runtime_service_loop_daemon_request_plan_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record =
            self.record_runtime_service_loop_daemon_request_plan_with_health(history, plan, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            runtime_service_loop_daemon_request_next_queue(plan),
        )
    }

    pub fn record_runtime_service_loop_daemon_request_record_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self
            .record_runtime_service_loop_daemon_request_record_with_health(history, record, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            runtime_service_loop_daemon_request_next_queue(&record.request_plan),
        )
    }

    pub fn record_runtime_service_loop_daemon_request_monitored_plan_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self
            .record_runtime_service_loop_daemon_request_monitored_plan_with_health(
                history, plan, policy,
            );

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            runtime_service_loop_daemon_request_monitored_next_queue(plan),
        )
    }

    pub fn record_runtime_service_loop_daemon_request_monitored_record_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self
            .record_runtime_service_loop_daemon_request_monitored_record_with_health(
                history, record, policy,
            );

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            runtime_service_loop_daemon_request_monitored_next_queue(&record.monitored_plan),
        )
    }

    pub fn record_runtime_service_loop_daemon_request_monitored_close_plan_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self
            .record_runtime_service_loop_daemon_request_monitored_close_plan_with_health(
                history, plan, policy,
            );

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            runtime_service_loop_daemon_request_monitored_close_next_queue(plan),
        )
    }

    pub fn record_runtime_service_loop_daemon_request_monitored_close_run_record_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self
            .record_runtime_service_loop_daemon_request_monitored_close_run_record_with_health(
                history, record, policy,
            );

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            runtime_service_loop_daemon_request_monitored_close_next_queue(
                &record.monitored_close_plan,
            ),
        )
    }

    pub fn record_runtime_service_loop_daemon_request_monitored_close_continuation_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self
            .record_runtime_service_loop_daemon_request_monitored_close_continuation_with_health(
                history,
                continuation,
                policy,
            );

        AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record,
            runtime_service_loop_daemon_request_monitored_close_continuation_next_queue(
                continuation,
            ),
        )
    }

    pub fn record_evolution_admission_handoff_history_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self.record_evolution_admission_handoff_history_with_health(
            history, next_queue, record, policy,
        );

        AgentAdapterBoundaryHandoff::from_record_and_queue(boundary_record, next_queue)
    }

    pub fn record_tool_build_report_history_gate_handoff_with_health(
        &self,
        history: AgentAdapterBoundarySummaryHistory,
        next_queue: &AgentTaskQueue,
        record: &ToolBuildReportHistoryGateRecord,
        policy: AgentAdapterBoundaryHealthPolicy,
    ) -> AgentAdapterBoundaryHandoff {
        let boundary_record = self
            .record_tool_build_report_history_gate_with_health(history, next_queue, record, policy);

        AgentAdapterBoundaryHandoff::from_record_and_queue(boundary_record, next_queue)
    }
}

fn adapter_boundary_snapshot_telemetry(
    gates: &[AgentAdapterBoundaryGate],
    next_queue_task_ids: &[String],
) -> Vec<String> {
    let service_execution_command_reason_count = gates
        .iter()
        .map(|gate| gate.service_execution_command_reason_count)
        .sum::<usize>();
    let service_execution_memory_promotion_command_reason_count = gates
        .iter()
        .map(|gate| gate.service_execution_memory_promotion_command_reason_count)
        .sum::<usize>();
    let service_execution_memory_promotion_command_reason_closes = gates
        .iter()
        .map(|gate| gate.service_execution_memory_promotion_command_reason_closes)
        .sum::<usize>();
    let service_execution_tool_build_command_reason_count = gates
        .iter()
        .map(|gate| gate.service_execution_tool_build_command_reason_count)
        .sum::<usize>();
    let mut telemetry = vec![
        "agent_adapter_boundary_snapshot=true".to_owned(),
        format!("agent_adapter_boundary_snapshot_gates={}", gates.len()),
        format!(
            "agent_adapter_boundary_snapshot_next_queue_tasks={}",
            next_queue_task_ids.len()
        ),
        format!(
            "agent_adapter_boundary_snapshot_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_snapshot_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_snapshot_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_snapshot_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ];
    telemetry.extend(gates.iter().map(|gate| {
        format!(
            "agent_adapter_boundary_snapshot_gate={} status={}",
            gate.owner.as_str(),
            gate.status.as_str()
        )
    }));
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_summary_telemetry(
    status: AgentAdapterBoundaryStatus,
    owners: usize,
    stable_owners: usize,
    watch_owners: usize,
    repair_owners: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    can_dispatch_core: bool,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    can_execute_service_commands: bool,
    requires_repair_first: bool,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_summary=true".to_owned(),
        format!("agent_adapter_boundary_summary_status={}", status.as_str()),
        format!("agent_adapter_boundary_summary_owners={owners}"),
        format!("agent_adapter_boundary_summary_stable_owners={stable_owners}"),
        format!("agent_adapter_boundary_summary_watch_owners={watch_owners}"),
        format!("agent_adapter_boundary_summary_repair_owners={repair_owners}"),
        format!("agent_adapter_boundary_summary_next_queue_tasks={next_queue_tasks}"),
        format!("agent_adapter_boundary_summary_blocked_reasons={blocked_reasons}"),
        format!("agent_adapter_boundary_summary_dispatch={can_dispatch_core}"),
        format!("agent_adapter_boundary_summary_memory_note={can_submit_memory_note}"),
        format!("agent_adapter_boundary_summary_adaptive_state={can_promote_adaptive_state}"),
        format!("agent_adapter_boundary_summary_service_command={can_execute_service_commands}"),
        format!("agent_adapter_boundary_summary_repair_first={requires_repair_first}"),
        format!(
            "agent_adapter_boundary_summary_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_summary_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_summary_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_summary_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_dashboard_telemetry(
    total_records: usize,
    stable_records: usize,
    watch_records: usize,
    repair_records: usize,
    repair_first_records: usize,
    memory_promotable_records: usize,
    adaptive_promotable_records: usize,
    service_command_records: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
    stable_rate: f32,
    memory_promotion_rate: f32,
    adaptive_promotion_rate: f32,
    service_command_rate: f32,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_dashboard=true".to_owned(),
        format!("agent_adapter_boundary_dashboard_records={total_records}"),
        format!("agent_adapter_boundary_dashboard_stable_records={stable_records}"),
        format!("agent_adapter_boundary_dashboard_watch_records={watch_records}"),
        format!("agent_adapter_boundary_dashboard_repair_records={repair_records}"),
        format!("agent_adapter_boundary_dashboard_repair_first_records={repair_first_records}"),
        format!("agent_adapter_boundary_dashboard_memory_promotable={memory_promotable_records}"),
        format!(
            "agent_adapter_boundary_dashboard_adaptive_promotable={adaptive_promotable_records}"
        ),
        format!("agent_adapter_boundary_dashboard_service_command={service_command_records}"),
        format!("agent_adapter_boundary_dashboard_next_queue_tasks={next_queue_tasks}"),
        format!("agent_adapter_boundary_dashboard_blocked_reasons={blocked_reasons}"),
        format!(
            "agent_adapter_boundary_dashboard_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_dashboard_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_dashboard_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_dashboard_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
        format!("agent_adapter_boundary_dashboard_stable_rate={stable_rate:.3}"),
        format!(
            "agent_adapter_boundary_dashboard_memory_promotion_rate={memory_promotion_rate:.3}"
        ),
        format!(
            "agent_adapter_boundary_dashboard_adaptive_promotion_rate={adaptive_promotion_rate:.3}"
        ),
        format!("agent_adapter_boundary_dashboard_service_command_rate={service_command_rate:.3}"),
    ]
}

fn adapter_boundary_history_record_telemetry(
    dashboard: &AgentAdapterBoundaryDashboard,
    health: &AgentAdapterBoundaryHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_adapter_boundary_history_record=true".to_owned(),
        format!(
            "agent_adapter_boundary_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_adapter_boundary_history_record_stable_rate={:.3}",
            dashboard.stable_rate
        ),
        format!(
            "agent_adapter_boundary_history_record_repair_first_records={}",
            dashboard.repair_first_records
        ),
        format!(
            "agent_adapter_boundary_history_record_service_command_reasons={}",
            dashboard.service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_history_record_service_memory_promotion_command_reasons={}",
            dashboard.service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_history_record_service_memory_promotion_command_reason_closes={}",
            dashboard.service_execution_memory_promotion_command_reason_closes
        ),
        format!(
            "agent_adapter_boundary_history_record_service_tool_build_command_reasons={}",
            dashboard.service_execution_tool_build_command_reason_count
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_adapter_boundary_history_record_reason={reason}")),
    );
    telemetry
}

fn adapter_boundary_record_telemetry(
    snapshot: &AgentAdapterBoundarySnapshot,
    history_record: &AgentAdapterBoundarySummaryHistoryRecord,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_record=true".to_owned(),
        format!(
            "agent_adapter_boundary_record_snapshot_status={}",
            snapshot.status().as_str()
        ),
        format!(
            "agent_adapter_boundary_record_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_record_records={}",
            history_record.records()
        ),
        format!(
            "agent_adapter_boundary_record_memory_note={}",
            snapshot.can_submit_memory_note() && history_record.health.is_stable()
        ),
        format!(
            "agent_adapter_boundary_record_adaptive_state={}",
            snapshot.can_promote_adaptive_state() && history_record.health.is_stable()
        ),
        format!(
            "agent_adapter_boundary_record_service_command_reasons={}",
            history_record
                .appended_summary
                .service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_record_service_memory_promotion_command_reasons={}",
            history_record
                .appended_summary
                .service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_record_service_memory_promotion_command_reason_closes={}",
            history_record
                .appended_summary
                .service_execution_memory_promotion_command_reason_closes
        ),
        format!(
            "agent_adapter_boundary_record_service_tool_build_command_reasons={}",
            history_record
                .appended_summary
                .service_execution_tool_build_command_reason_count
        ),
    ]
}

fn adapter_boundary_handoff_blocked_reasons(
    boundary_record: &AgentAdapterBoundaryRecord,
) -> Vec<String> {
    let mut reasons = Vec::new();
    extend_ordered_unique(
        &mut reasons,
        boundary_record.snapshot.blocked_reasons.clone(),
    );
    extend_ordered_unique(
        &mut reasons,
        boundary_record
            .history_record
            .health
            .reasons
            .iter()
            .map(|reason| format!("history:{reason}"))
            .collect(),
    );
    reasons
}

fn adapter_boundary_repair_tasks(
    requires_repair_first: bool,
    blocked_reasons: &[String],
) -> Vec<AgentTask> {
    if !requires_repair_first {
        return Vec::new();
    }

    blocked_reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!("adapter-boundary-repair-{index}"),
                adapter_boundary_repair_role(reason),
                format!("repair adapter boundary: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("adapter-boundary-repair")
            .with_priority(1)
        })
        .collect()
}

fn adapter_boundary_handoff_trend_repair_tasks(
    handoff_health: &AgentAdapterBoundaryHandoffHealth,
    blocked_reasons: &[String],
) -> Vec<AgentTask> {
    if !handoff_health.requires_repair_first() {
        return Vec::new();
    }

    blocked_reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!("adapter-boundary-handoff-trend-repair-{index}"),
                adapter_boundary_repair_role(reason),
                format!("repair adapter handoff trend: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("adapter-boundary-handoff-trend-repair")
            .with_priority(1)
        })
        .collect()
}

fn adapter_boundary_handoff_trend_gate_repair_tasks(
    trend_health: &AgentAdapterBoundaryHandoffTrendGateHealth,
    blocked_reasons: &[String],
) -> Vec<AgentTask> {
    if !trend_health.requires_repair_first() {
        return Vec::new();
    }

    blocked_reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!("adapter-boundary-handoff-trend-gate-repair-{index}"),
                adapter_boundary_repair_role(reason),
                format!("repair adapter handoff trend gate: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("adapter-boundary-handoff-trend-gate-repair")
            .with_priority(1)
        })
        .collect()
}

fn adapter_boundary_handoff_trend_admission_resume_repair_tasks(
    resume_health: &AgentAdapterBoundaryHandoffTrendAdmissionResumeHealth,
    blocked_reasons: &[String],
) -> Vec<AgentTask> {
    if !resume_health.requires_repair_first() {
        return Vec::new();
    }

    blocked_reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!("adapter-boundary-handoff-trend-admission-resume-repair-{index}"),
                adapter_boundary_repair_role(reason),
                format!("repair adapter handoff trend admission resume: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("adapter-boundary-handoff-trend-admission-resume-repair")
            .with_priority(1)
        })
        .collect()
}

fn adapter_boundary_handoff_trend_admission_resume_gate_monitor_repair_tasks(
    gate_health: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealth,
    blocked_reasons: &[String],
) -> Vec<AgentTask> {
    if !gate_health.requires_repair_first() {
        return Vec::new();
    }

    blocked_reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!(
                    "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-{index}"
                ),
                adapter_boundary_repair_role(reason),
                format!("repair adapter handoff trend admission resume gate monitor: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair")
            .with_priority(1)
        })
        .collect()
}

fn adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_tasks(
    handoff_health: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealth,
    blocked_reasons: &[String],
) -> Vec<AgentTask> {
    if !handoff_health.requires_repair_first() {
        return Vec::new();
    }

    blocked_reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!(
                    "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-{index}"
                ),
                adapter_boundary_repair_role(reason),
                format!(
                    "repair adapter handoff trend admission resume gate monitor handoff: {reason}"
                ),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane(
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair",
            )
            .with_priority(1)
        })
        .collect()
}

fn adapter_boundary_repair_role(reason: &str) -> AgentRole {
    if reason.starts_with("norion_memory:") {
        AgentRole::MemoryCurator
    } else if reason.starts_with("eval_reporting:") {
        AgentRole::Reviewer
    } else if reason.starts_with("service_adapter:") {
        AgentRole::Tester
    } else if reason.starts_with("norion_core:") {
        AgentRole::Planner
    } else {
        AgentRole::Reviewer
    }
}

fn adapter_boundary_handoff_report_gate_reasons(
    record: &AgentAdapterBoundaryHandoffHistoryRecord,
) -> Vec<AgentReportGateReason> {
    let summary = &record.appended_summary;
    let mut reasons = Vec::new();

    if record.health.status != AgentAdapterBoundaryStatus::Stable {
        reasons.push(AgentReportGateReason::new(
            "adapter_boundary_handoff_health_status",
            record.health.status.as_str(),
        ));
    }
    if !summary.admitted {
        reasons.push(AgentReportGateReason::new(
            "adapter_boundary_handoff_not_admitted",
            summary.health_status.as_str(),
        ));
    }
    if summary.requires_repair_first {
        reasons.push(AgentReportGateReason::new(
            "adapter_boundary_handoff_repair_first",
            "true",
        ));
    }
    if !summary.can_submit_memory_note {
        reasons.push(AgentReportGateReason::new(
            "adapter_boundary_handoff_memory_note_closed",
            summary.snapshot_status.as_str(),
        ));
    }
    if !summary.can_promote_adaptive_state {
        reasons.push(AgentReportGateReason::new(
            "adapter_boundary_handoff_adaptive_state_closed",
            summary.snapshot_status.as_str(),
        ));
    }
    if summary.repair_tasks > 0 {
        reasons.push(AgentReportGateReason::new(
            "adapter_boundary_handoff_repair_tasks",
            summary.repair_tasks.to_string(),
        ));
    }
    reasons.extend(record.health.reasons.iter().map(|reason| {
        AgentReportGateReason::new("adapter_boundary_handoff_health_reason", reason.clone())
    }));

    reasons
}

fn adapter_boundary_handoff_report_gate_tasks(
    run_id: &str,
    reasons: &[AgentReportGateReason],
) -> Vec<AgentTask> {
    reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!(
                    "adapter-boundary-eval-report-{}-{}-{}",
                    adapter_boundary_stable_id(run_id),
                    index,
                    adapter_boundary_stable_id(&reason.code)
                ),
                adapter_boundary_report_gate_role(reason.code.as_str()),
                format!("repair adapter boundary eval report: {}", reason.as_line()),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("eval-adapter-boundary")
            .with_priority(8)
        })
        .collect()
}

fn adapter_boundary_report_gate_role(code: &str) -> AgentRole {
    if code.contains("memory_note") {
        AgentRole::MemoryCurator
    } else if code.contains("repair") {
        AgentRole::Tester
    } else {
        AgentRole::Reviewer
    }
}

fn adapter_boundary_stable_id(raw: &str) -> String {
    let id = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let id = id
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if id.is_empty() {
        "unknown".to_owned()
    } else {
        id
    }
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_gate_telemetry(
    requested_admitted: bool,
    effective_admitted: bool,
    handoff_health_status: AgentAdapterBoundaryStatus,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_gate=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_health={}",
            handoff_health_status.as_str()
        ),
        format!("agent_adapter_boundary_handoff_trend_gate_requested={requested_admitted}"),
        format!("agent_adapter_boundary_handoff_trend_gate_effective={effective_admitted}"),
        format!("agent_adapter_boundary_handoff_trend_gate_repair_first={requires_repair_first}"),
        format!("agent_adapter_boundary_handoff_trend_gate_repair_tasks={repair_tasks}"),
        format!("agent_adapter_boundary_handoff_trend_gate_next_queue_tasks={next_queue_tasks}"),
        format!("agent_adapter_boundary_handoff_trend_gate_blocked_reasons={blocked_reasons}"),
        format!("agent_adapter_boundary_handoff_trend_gate_memory_note={can_submit_memory_note}"),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_telemetry(
    requested_admitted: bool,
    effective_admitted: bool,
    trend_health_status: AgentAdapterBoundaryStatus,
    requires_repair_first: bool,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    history_repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    records: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_health={}",
            trend_health_status.as_str()
        ),
        format!("agent_adapter_boundary_handoff_trend_admission_records={records}"),
        format!("agent_adapter_boundary_handoff_trend_admission_requested={requested_admitted}"),
        format!("agent_adapter_boundary_handoff_trend_admission_effective={effective_admitted}"),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_memory_note={can_submit_memory_note}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_history_repair_tasks={history_repair_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_next_queue_tasks={next_queue_tasks}"
        ),
        format!("agent_adapter_boundary_handoff_trend_admission_blocked_reasons={blocked_reasons}"),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_summary_telemetry(
    trend_health_status: AgentAdapterBoundaryStatus,
    requested_admitted: bool,
    effective_admitted: bool,
    requires_repair_first: bool,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    decision_repair_tasks: usize,
    history_repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    records: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_summary=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_health={}",
            trend_health_status.as_str()
        ),
        format!("agent_adapter_boundary_handoff_trend_admission_summary_records={records}"),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_requested={requested_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_effective={effective_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_memory_note={can_submit_memory_note}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_decision_repair_tasks={decision_repair_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_history_repair_tasks={history_repair_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_summary_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_dashboard_telemetry(
    total_records: usize,
    requested_admitted_records: usize,
    effective_admitted_records: usize,
    repair_first_records: usize,
    memory_promotable_records: usize,
    adaptive_promotable_records: usize,
    decision_repair_task_count: usize,
    history_repair_task_count: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
    effective_admitted_rate: f32,
    repair_first_rate: f32,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_dashboard=true".to_owned(),
        format!("agent_adapter_boundary_handoff_trend_admission_dashboard_records={total_records}"),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_requested={requested_admitted_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_effective={effective_admitted_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_memory_promotable={memory_promotable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_adaptive_promotable={adaptive_promotable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_decision_repair_tasks={decision_repair_task_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_history_repair_tasks={history_repair_task_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_effective_rate={effective_admitted_rate:.3}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
    ]
}

fn adapter_boundary_handoff_trend_admission_history_record_telemetry(
    dashboard: &AgentAdapterBoundaryHandoffTrendAdmissionDashboard,
    health: &AgentAdapterBoundaryHandoffTrendAdmissionHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_adapter_boundary_handoff_trend_admission_history_record=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_history_record_effective_rate={:.3}",
            dashboard.effective_admitted_rate
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_history_record_history_repair_tasks={}",
            dashboard.history_repair_task_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_history_record_service_command_reasons={}",
            dashboard.service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_history_record_service_memory_promotion_command_reasons={}",
            dashboard.service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_history_record_service_memory_promotion_command_reason_closes={}",
            dashboard.service_execution_memory_promotion_command_reason_closes
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_history_record_service_tool_build_command_reasons={}",
            dashboard.service_execution_tool_build_command_reason_count
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!("agent_adapter_boundary_handoff_trend_admission_history_record_reason={reason}")
    }));
    telemetry
}

fn adapter_boundary_handoff_trend_admission_monitor_telemetry(
    admission: &AgentAdapterBoundaryHandoffTrendAdmission,
    history_record: &AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecord,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_monitor=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_health={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_records={}",
            history_record.records()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_effective={}",
            admission.is_admitted() && history_record.health.is_stable()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_repair_first={}",
            admission.requires_repair_first() || history_record.requires_repair_first()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_memory_note={}",
            admission.can_submit_memory_note && history_record.health.is_stable()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_adaptive_state={}",
            admission.can_promote_adaptive_state && history_record.health.is_stable()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_next_queue_tasks={}",
            admission.next_queue.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_history_repair_tasks={}",
            admission.history_repair_tasks.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_service_command_reasons={}",
            admission.service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_service_memory_promotion_command_reasons={}",
            admission.service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_service_memory_promotion_command_reason_closes={}",
            admission.service_execution_memory_promotion_command_reason_closes
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_monitor_service_tool_build_command_reasons={}",
            admission.service_execution_tool_build_command_reason_count
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_continuation_telemetry(
    trend_health_status: AgentAdapterBoundaryStatus,
    admission_health_status: AgentAdapterBoundaryStatus,
    effective_admitted: bool,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    next_queue_tasks: usize,
    trend_records: usize,
    admission_records: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_continuation=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_trend_health={}",
            trend_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_admission_health={}",
            admission_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_effective={effective_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_memory_note={can_submit_memory_note}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_trend_records={trend_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_admission_records={admission_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_continuation_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_resume_plan_telemetry(
    prior_trend_health_status: AgentAdapterBoundaryStatus,
    prior_admission_health_status: AgentAdapterBoundaryStatus,
    prior_effective_admitted: bool,
    prior_can_submit_memory_note: bool,
    prior_can_promote_adaptive_state: bool,
    prior_requires_repair_first: bool,
    prior_queue_tasks: usize,
    trend_records: usize,
    admission_records: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_plan=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_trend_health={}",
            prior_trend_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_admission_health={}",
            prior_admission_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_effective={prior_effective_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_memory_note={prior_can_submit_memory_note}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_adaptive_state={prior_can_promote_adaptive_state}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_repair_first={prior_requires_repair_first}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_prior_queue_tasks={prior_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_trend_records={trend_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_admission_records={admission_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_plan_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

fn adapter_boundary_handoff_trend_admission_resume_record_telemetry(
    resume_plan: &AgentAdapterBoundaryHandoffTrendAdmissionResumePlan,
    monitor_record: &AgentAdapterBoundaryHandoffTrendAdmissionMonitorRecord,
    continuation: &AgentAdapterBoundaryHandoffTrendAdmissionContinuation,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_record=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_prior_trend_records={}",
            resume_plan.trend_history.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_prior_admission_records={}",
            resume_plan.admission_history.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_monitor_records={}",
            monitor_record.records()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_next_trend_records={}",
            continuation.trend_history.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_next_admission_records={}",
            continuation.admission_history.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_effective={}",
            continuation.is_admitted()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_repair_first={}",
            continuation.requires_repair_first
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_next_queue_tasks={}",
            continuation.next_queue.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_service_command_reasons={}",
            continuation.service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_service_memory_promotion_command_reasons={}",
            continuation.service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_service_memory_promotion_command_reason_closes={}",
            continuation.service_execution_memory_promotion_command_reason_closes
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_record_service_tool_build_command_reasons={}",
            continuation.service_execution_tool_build_command_reason_count
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_resume_summary_telemetry(
    prior_trend_health_status: AgentAdapterBoundaryStatus,
    prior_admission_health_status: AgentAdapterBoundaryStatus,
    next_trend_health_status: AgentAdapterBoundaryStatus,
    next_admission_health_status: AgentAdapterBoundaryStatus,
    effective_admitted: bool,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    prior_queue_tasks: usize,
    next_queue_tasks: usize,
    prior_trend_records: usize,
    prior_admission_records: usize,
    next_trend_records: usize,
    next_admission_records: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_summary=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_prior_trend_health={}",
            prior_trend_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_prior_admission_health={}",
            prior_admission_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_next_trend_health={}",
            next_trend_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_next_admission_health={}",
            next_admission_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_effective={effective_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_memory_note={can_submit_memory_note}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_prior_queue_tasks={prior_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_prior_trend_records={prior_trend_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_prior_admission_records={prior_admission_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_next_trend_records={next_trend_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_next_admission_records={next_admission_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_summary_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_resume_dashboard_telemetry(
    total_records: usize,
    effective_admitted_records: usize,
    repair_first_records: usize,
    memory_promotable_records: usize,
    adaptive_promotable_records: usize,
    next_queue_tasks: usize,
    effective_admitted_rate: f32,
    repair_first_rate: f32,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_dashboard=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_records={total_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_effective={effective_admitted_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_memory_promotable={memory_promotable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_adaptive_promotable={adaptive_promotable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_effective_rate={effective_admitted_rate:.3}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_dashboard_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

fn adapter_boundary_handoff_trend_admission_resume_history_record_telemetry(
    dashboard: &AgentAdapterBoundaryHandoffTrendAdmissionResumeDashboard,
    health: &AgentAdapterBoundaryHandoffTrendAdmissionResumeHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_history_record=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_history_record_effective_rate={:.3}",
            dashboard.effective_admitted_rate
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_history_record_repair_first={}",
            dashboard.repair_first_records
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_history_record_service_command_reasons={}",
            dashboard.service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_history_record_service_memory_promotion_command_reasons={}",
            dashboard.service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_history_record_service_memory_promotion_command_reason_closes={}",
            dashboard.service_execution_memory_promotion_command_reason_closes
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_history_record_service_tool_build_command_reasons={}",
            dashboard.service_execution_tool_build_command_reason_count
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_history_record_reason={reason}"
        )
    }));
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_resume_gate_telemetry(
    requested_admitted: bool,
    effective_admitted: bool,
    resume_health_status: AgentAdapterBoundaryStatus,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_health={}",
            resume_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_requested={requested_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_effective={effective_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_memory_note={can_submit_memory_note}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_resume_gate_summary_telemetry(
    resume_health_status: AgentAdapterBoundaryStatus,
    requested_admitted: bool,
    effective_admitted: bool,
    requires_repair_first: bool,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_health={}",
            resume_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_requested={requested_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_effective={effective_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_memory_note={can_submit_memory_note}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_resume_gate_dashboard_telemetry(
    total_records: usize,
    requested_admitted_records: usize,
    effective_admitted_records: usize,
    repair_first_records: usize,
    memory_promotable_records: usize,
    adaptive_promotable_records: usize,
    repair_task_count: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    effective_admitted_rate: f32,
    repair_first_rate: f32,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_records={total_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_requested={requested_admitted_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_effective={effective_admitted_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_memory_promotable={memory_promotable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_adaptive_promotable={adaptive_promotable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_effective_rate={effective_admitted_rate:.3}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_dashboard_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

fn adapter_boundary_handoff_trend_admission_resume_gate_history_record_telemetry(
    dashboard: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDashboard,
    health: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate_history_record=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_history_record_effective_rate={:.3}",
            dashboard.effective_admitted_rate
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_history_record_service_command_reasons={}",
            dashboard.service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_history_record_service_memory_promotion_command_reasons={}",
            dashboard.service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_history_record_service_memory_promotion_command_reason_closes={}",
            dashboard.service_execution_memory_promotion_command_reason_closes
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_history_record_service_tool_build_command_reasons={}",
            dashboard.service_execution_tool_build_command_reason_count
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_history_record_reason={reason}"
        )
    }));
    telemetry
}

fn adapter_boundary_handoff_trend_admission_resume_gate_monitor_telemetry(
    decision: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDecision,
    history_record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecord,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_health={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_records={}",
            history_record.records()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_effective={}",
            decision.is_admitted() && history_record.health.is_stable()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_repair_first={}",
            decision.requires_repair_first || history_record.requires_repair_first()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_memory_note={}",
            decision.can_submit_memory_note && history_record.health.is_stable()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_adaptive_state={}",
            decision.can_promote_adaptive_state && history_record.health.is_stable()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_next_queue_tasks={}",
            decision.next_queue.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_repair_tasks={}",
            decision.repair_tasks.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_service_command_reasons={}",
            decision.service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_service_memory_promotion_command_reasons={}",
            decision.service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_service_memory_promotion_command_reason_closes={}",
            decision.service_execution_memory_promotion_command_reason_closes
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_service_tool_build_command_reasons={}",
            decision.service_execution_tool_build_command_reason_count
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_telemetry(
    requested_admitted: bool,
    effective_admitted: bool,
    gate_health_status: AgentAdapterBoundaryStatus,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_health={}",
            gate_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_requested={requested_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_effective={effective_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_memory_note={can_submit_memory_note}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

fn adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_telemetry(
    monitor_record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorRecord,
    gate_decision: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGateDecision,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff=true"
            .to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_health={}",
            gate_decision.gate_health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_records={}",
            monitor_record.records()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_effective={}",
            gate_decision.is_admitted()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_memory_note={}",
            gate_decision.can_submit_memory_note
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_adaptive_state={}",
            gate_decision.can_promote_adaptive_state
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_service_command_reasons={}",
            gate_decision.service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_service_memory_promotion_command_reasons={}",
            gate_decision.service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_service_memory_promotion_command_reason_closes={}",
            gate_decision.service_execution_memory_promotion_command_reason_closes
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_telemetry(
    gate_health_status: AgentAdapterBoundaryStatus,
    requested_admitted: bool,
    effective_admitted: bool,
    requires_repair_first: bool,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary=true"
            .to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_health={}",
            gate_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_requested={requested_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_effective={effective_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_memory_note={can_submit_memory_note}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_records={records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_telemetry(
    total_records: usize,
    requested_admitted_records: usize,
    effective_admitted_records: usize,
    repair_first_records: usize,
    memory_promotable_records: usize,
    adaptive_promotable_records: usize,
    stable_records: usize,
    watch_records: usize,
    repair_records: usize,
    recorded_gate_rows: usize,
    repair_task_count: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    effective_admitted_rate: f32,
    repair_first_rate: f32,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard=true"
            .to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_records={total_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_requested={requested_admitted_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_effective={effective_admitted_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_memory_promotable={memory_promotable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_adaptive_promotable={adaptive_promotable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_gate_rows={recorded_gate_rows}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_effective_rate={effective_admitted_rate:.3}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_dashboard_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

fn adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_record_telemetry(
    dashboard: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffDashboard,
    health: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_record=true"
            .to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_record_effective_rate={:.3}",
            dashboard.effective_admitted_rate
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_record_reason={reason}"
        )
    }));
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_telemetry(
    handoff_health_status: AgentAdapterBoundaryStatus,
    requested_admitted: bool,
    effective_admitted: bool,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate=true"
            .to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_health={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_requested={requested_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_effective={effective_admitted}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_memory_note={can_submit_memory_note}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_adaptive_state={can_promote_adaptive_state}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
    ]
}

fn adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_telemetry(
    handoff: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord,
    history_record: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecord,
    gate_decision: &AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGateDecision,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff=true"
            .to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_health={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_records={}",
            history_record.records()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_requested={}",
            gate_decision.requested_admitted
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_effective={}",
            gate_decision.is_admitted()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_memory_note={}",
            gate_decision.can_submit_memory_note
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_adaptive_state={}",
            gate_decision.can_promote_adaptive_state
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_source_records={}",
            handoff.records()
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_gate_summary_telemetry(
    handoff_health_status: AgentAdapterBoundaryStatus,
    requested_admitted: bool,
    effective_admitted: bool,
    requires_repair_first: bool,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_gate_summary=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_summary_health={}",
            handoff_health_status.as_str()
        ),
        format!("agent_adapter_boundary_handoff_trend_gate_summary_requested={requested_admitted}"),
        format!("agent_adapter_boundary_handoff_trend_gate_summary_effective={effective_admitted}"),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_summary_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_summary_memory_note={can_submit_memory_note}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_summary_adaptive_state={can_promote_adaptive_state}"
        ),
        format!("agent_adapter_boundary_handoff_trend_gate_summary_repair_tasks={repair_tasks}"),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_summary_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_summary_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_summary_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_summary_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_summary_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_trend_gate_dashboard_telemetry(
    total_records: usize,
    requested_admitted_records: usize,
    effective_admitted_records: usize,
    repair_first_records: usize,
    memory_promotable_records: usize,
    adaptive_promotable_records: usize,
    repair_task_count: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
    effective_admitted_rate: f32,
    repair_first_rate: f32,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_trend_gate_dashboard=true".to_owned(),
        format!("agent_adapter_boundary_handoff_trend_gate_dashboard_records={total_records}"),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_requested={requested_admitted_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_effective={effective_admitted_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_memory_promotable={memory_promotable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_adaptive_promotable={adaptive_promotable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_effective_rate={effective_admitted_rate:.3}"
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
    ]
}

fn adapter_boundary_handoff_trend_gate_history_record_telemetry(
    dashboard: &AgentAdapterBoundaryHandoffTrendGateDashboard,
    health: &AgentAdapterBoundaryHandoffTrendGateHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_adapter_boundary_handoff_trend_gate_history_record=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_history_record_effective_rate={:.3}",
            dashboard.effective_admitted_rate
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_history_record_service_command_reasons={}",
            dashboard.service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_history_record_service_memory_promotion_command_reasons={}",
            dashboard.service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_history_record_service_memory_promotion_command_reason_closes={}",
            dashboard.service_execution_memory_promotion_command_reason_closes
        ),
        format!(
            "agent_adapter_boundary_handoff_trend_gate_history_record_service_tool_build_command_reasons={}",
            dashboard.service_execution_tool_build_command_reason_count
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!("agent_adapter_boundary_handoff_trend_gate_history_record_reason={reason}")
    }));
    telemetry
}

fn adapter_boundary_handoff_telemetry(
    boundary_record: &AgentAdapterBoundaryRecord,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_snapshot_status={}",
            boundary_record.snapshot.status().as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_health_status={}",
            boundary_record.history_record.health.status.as_str()
        ),
        format!("agent_adapter_boundary_handoff_admitted={admitted}"),
        format!("agent_adapter_boundary_handoff_repair_first={requires_repair_first}"),
        format!("agent_adapter_boundary_handoff_repair_tasks={repair_tasks}"),
        format!("agent_adapter_boundary_handoff_next_queue_tasks={next_queue_tasks}"),
        format!("agent_adapter_boundary_handoff_blocked_reasons={blocked_reasons}"),
        format!(
            "agent_adapter_boundary_handoff_service_command_reasons={}",
            boundary_record
                .summary()
                .service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_service_memory_promotion_command_reasons={}",
            boundary_record
                .summary()
                .service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_service_memory_promotion_command_reason_closes={}",
            boundary_record
                .summary()
                .service_execution_memory_promotion_command_reason_closes
        ),
        format!(
            "agent_adapter_boundary_handoff_service_tool_build_command_reasons={}",
            boundary_record
                .summary()
                .service_execution_tool_build_command_reason_count
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_summary_telemetry(
    snapshot_status: AgentAdapterBoundaryStatus,
    health_status: AgentAdapterBoundaryStatus,
    admitted: bool,
    requires_repair_first: bool,
    can_submit_memory_note: bool,
    can_promote_adaptive_state: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_summary=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_summary_snapshot_status={}",
            snapshot_status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_summary_health_status={}",
            health_status.as_str()
        ),
        format!("agent_adapter_boundary_handoff_summary_admitted={admitted}"),
        format!("agent_adapter_boundary_handoff_summary_repair_first={requires_repair_first}"),
        format!("agent_adapter_boundary_handoff_summary_memory_note={can_submit_memory_note}"),
        format!(
            "agent_adapter_boundary_handoff_summary_adaptive_state={can_promote_adaptive_state}"
        ),
        format!("agent_adapter_boundary_handoff_summary_repair_tasks={repair_tasks}"),
        format!("agent_adapter_boundary_handoff_summary_next_queue_tasks={next_queue_tasks}"),
        format!("agent_adapter_boundary_handoff_summary_blocked_reasons={blocked_reasons}"),
        format!(
            "agent_adapter_boundary_handoff_summary_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_summary_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_summary_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_summary_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn adapter_boundary_handoff_dashboard_telemetry(
    total_records: usize,
    admitted_records: usize,
    repair_first_records: usize,
    memory_promotable_records: usize,
    adaptive_promotable_records: usize,
    repair_task_count: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
    service_execution_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_count: usize,
    service_execution_memory_promotion_command_reason_closes: usize,
    service_execution_tool_build_command_reason_count: usize,
    admitted_rate: f32,
    repair_first_rate: f32,
    memory_promotion_rate: f32,
    adaptive_promotion_rate: f32,
) -> Vec<String> {
    vec![
        "agent_adapter_boundary_handoff_dashboard=true".to_owned(),
        format!("agent_adapter_boundary_handoff_dashboard_records={total_records}"),
        format!("agent_adapter_boundary_handoff_dashboard_admitted={admitted_records}"),
        format!("agent_adapter_boundary_handoff_dashboard_repair_first={repair_first_records}"),
        format!(
            "agent_adapter_boundary_handoff_dashboard_memory_promotable={memory_promotable_records}"
        ),
        format!(
            "agent_adapter_boundary_handoff_dashboard_adaptive_promotable={adaptive_promotable_records}"
        ),
        format!("agent_adapter_boundary_handoff_dashboard_repair_tasks={repair_task_count}"),
        format!("agent_adapter_boundary_handoff_dashboard_next_queue_tasks={next_queue_tasks}"),
        format!("agent_adapter_boundary_handoff_dashboard_blocked_reasons={blocked_reasons}"),
        format!(
            "agent_adapter_boundary_handoff_dashboard_service_command_reasons={service_execution_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_dashboard_service_memory_promotion_command_reasons={service_execution_memory_promotion_command_reason_count}"
        ),
        format!(
            "agent_adapter_boundary_handoff_dashboard_service_memory_promotion_command_reason_closes={service_execution_memory_promotion_command_reason_closes}"
        ),
        format!(
            "agent_adapter_boundary_handoff_dashboard_service_tool_build_command_reasons={service_execution_tool_build_command_reason_count}"
        ),
        format!("agent_adapter_boundary_handoff_dashboard_admitted_rate={admitted_rate:.3}"),
        format!(
            "agent_adapter_boundary_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_adapter_boundary_handoff_dashboard_memory_promotion_rate={memory_promotion_rate:.3}"
        ),
        format!(
            "agent_adapter_boundary_handoff_dashboard_adaptive_promotion_rate={adaptive_promotion_rate:.3}"
        ),
    ]
}

fn adapter_boundary_handoff_history_record_telemetry(
    dashboard: &AgentAdapterBoundaryHandoffDashboard,
    health: &AgentAdapterBoundaryHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_adapter_boundary_handoff_history_record=true".to_owned(),
        format!(
            "agent_adapter_boundary_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_adapter_boundary_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_adapter_boundary_handoff_history_record_admitted_rate={:.3}",
            dashboard.admitted_rate
        ),
        format!(
            "agent_adapter_boundary_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_adapter_boundary_handoff_history_record_service_command_reasons={}",
            dashboard.service_execution_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_history_record_service_memory_promotion_command_reasons={}",
            dashboard.service_execution_memory_promotion_command_reason_count
        ),
        format!(
            "agent_adapter_boundary_handoff_history_record_service_memory_promotion_command_reason_closes={}",
            dashboard.service_execution_memory_promotion_command_reason_closes
        ),
        format!(
            "agent_adapter_boundary_handoff_history_record_service_tool_build_command_reasons={}",
            dashboard.service_execution_tool_build_command_reason_count
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_adapter_boundary_handoff_history_record_reason={reason}")),
    );
    telemetry
}

fn extend_ordered_unique(target: &mut Vec<String>, items: Vec<String>) {
    for item in items {
        if !target.contains(&item) {
            target.push(item);
        }
    }
}

fn adapter_run_progress_reasons(progress: &AgentRunLedgerProgress) -> Vec<String> {
    let mut reasons = Vec::new();
    if progress.empty_dispatch {
        reasons.push("run_progress_empty_dispatch".to_owned());
    }
    if progress.dispatch_rejections > 0 {
        reasons.push(format!(
            "run_progress_dispatch_rejections={}",
            progress.dispatch_rejections
        ));
    }
    if progress.missing_assigned_tasks > 0 {
        reasons.push(format!(
            "run_progress_missing_assigned_tasks={}",
            progress.missing_assigned_tasks
        ));
    }
    if progress.rejected_results > 0 {
        reasons.push(format!(
            "run_progress_rejected_results={}",
            progress.rejected_results
        ));
    }
    if progress.unassigned_results > 0 {
        reasons.push(format!(
            "run_progress_unassigned_results={}",
            progress.unassigned_results
        ));
    }
    reasons
}

fn adapter_run_ledger_admission_reasons(admission: &AgentRunLedgerAdmission) -> Vec<String> {
    if admission.reasons.is_empty() {
        if admission.requires_repair_first {
            vec!["run_ledger_admission_dispatch_closed".to_owned()]
        } else {
            Vec::new()
        }
    } else {
        admission
            .reasons
            .iter()
            .map(|reason| format!("run_ledger_admission:{reason}"))
            .collect()
    }
}

fn rate(count: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        count as f32 / total as f32
    }
}

fn runtime_service_loop_daemon_record_next_queue(
    record: &AgentClosedLoopRuntimeServiceLoopRunDaemonRecord,
) -> &AgentTaskQueue {
    &record.next_runtime_input().next_queue
}

fn runtime_service_loop_daemon_continuation_next_queue(
    continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
) -> &AgentTaskQueue {
    &continuation.next_runtime_input.next_queue
}

fn runtime_service_loop_daemon_input_plan_next_queue(
    plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan,
) -> &AgentTaskQueue {
    &plan
        .input
        .loop_run_input
        .service_run_input
        .request_input
        .runtime_input
        .next_queue
}

fn runtime_service_loop_daemon_continuation_reasons(
    continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
) -> Vec<String> {
    let mut reasons = Vec::new();

    if continuation.requires_repair_first {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_requires_repair_first".to_owned()],
        );
    }

    if continuation.mode == AgentClosedLoopNextTurnMode::Repair {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_mode_repair".to_owned()],
        );
    }

    match continuation.transition_health_status {
        AgentClosedLoopExecutionHealthStatus::Stable => {}
        AgentClosedLoopExecutionHealthStatus::Watch => extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_transition_watch".to_owned()],
        ),
        AgentClosedLoopExecutionHealthStatus::Repair => extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_transition_repair".to_owned()],
        ),
    }

    match continuation.control_health_status {
        AgentClosedLoopExecutionHealthStatus::Stable => {}
        AgentClosedLoopExecutionHealthStatus::Watch => extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_control_watch".to_owned()],
        ),
        AgentClosedLoopExecutionHealthStatus::Repair => extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_control_repair".to_owned()],
        ),
    }

    if !continuation.can_schedule {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_unschedulable".to_owned()],
        );
    }

    if continuation.side_effect_dispatch_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_side_effect_dispatch_closed".to_owned()],
        );
    }

    if continuation.memory_note_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_memory_note_closed".to_owned()],
        );
    }

    if !continuation.allows_adaptive_evolution {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_adaptive_closed".to_owned()],
        );
    }

    reasons
}

fn runtime_service_loop_daemon_input_plan_reasons(
    plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan,
) -> Vec<String> {
    let mut reasons = vec!["runtime_service_loop_daemon_input_plan_observe_only".to_owned()];

    if runtime_service_loop_daemon_input_plan_next_queue(plan).is_empty() {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_input_plan_next_queue_empty".to_owned()],
        );
    }

    if plan.side_effect_dispatch_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_input_plan_side_effect_dispatch_closed".to_owned()],
        );
    }

    if plan.memory_note_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_input_plan_memory_note_closed".to_owned()],
        );
    }

    reasons
}

fn runtime_service_loop_daemon_request_next_queue(
    plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
) -> &AgentTaskQueue {
    &plan.request_input.runtime_input.next_queue
}

fn runtime_service_loop_daemon_request_reasons(
    plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
) -> Vec<String> {
    let mut reasons = Vec::new();

    if plan.requires_repair_first {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_requires_repair_first".to_owned()],
        );
    }

    if plan.mode == AgentClosedLoopNextTurnMode::Repair {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_mode_repair".to_owned()],
        );
    }

    if !plan.can_schedule {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_unschedulable".to_owned()],
        );
    }

    if plan.side_effect_dispatch_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_side_effect_dispatch_closed".to_owned()],
        );
    }

    if plan.memory_note_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_memory_note_closed".to_owned()],
        );
    }

    if !plan.allows_adaptive_evolution {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_adaptive_closed".to_owned()],
        );
    }

    reasons
}

fn runtime_service_loop_daemon_request_monitored_next_queue(
    plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
) -> &AgentTaskQueue {
    &plan.request_plan.request_input.runtime_input.next_queue
}

fn runtime_service_loop_daemon_request_monitored_reasons(
    plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
) -> Vec<String> {
    let mut reasons = plan.request_health.reasons.clone();
    extend_ordered_unique(&mut reasons, plan.daemon_control_health.reasons.clone());

    if plan.requires_repair_first {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_requires_repair_first".to_owned()],
        );
    }

    match plan.request_health.status {
        AgentClosedLoopExecutionHealthStatus::Stable => {}
        AgentClosedLoopExecutionHealthStatus::Watch => extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_request_watch".to_owned()],
        ),
        AgentClosedLoopExecutionHealthStatus::Repair => extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_request_repair".to_owned()],
        ),
    }

    match plan.daemon_control_health.status {
        AgentClosedLoopExecutionHealthStatus::Stable => {}
        AgentClosedLoopExecutionHealthStatus::Watch => extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_daemon_control_watch".to_owned()],
        ),
        AgentClosedLoopExecutionHealthStatus::Repair => extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_daemon_control_repair".to_owned()],
        ),
    }

    if !plan.can_schedule {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_unschedulable".to_owned()],
        );
    }

    if plan.side_effect_dispatch_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_side_effect_dispatch_closed"
                    .to_owned(),
            ],
        );
    }

    if plan.memory_note_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_memory_note_closed".to_owned()],
        );
    }

    if !plan.allows_adaptive_evolution {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_adaptive_closed".to_owned()],
        );
    }

    reasons
}

fn runtime_service_loop_daemon_request_monitored_close_next_queue(
    plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
) -> &AgentTaskQueue {
    &plan
        .monitored_plan
        .request_plan
        .request_input
        .runtime_input
        .next_queue
}

fn runtime_service_loop_daemon_request_monitored_close_continuation_next_queue(
    continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation,
) -> &AgentTaskQueue {
    &continuation
        .monitored_continuation
        .daemon_continuation
        .next_runtime_input
        .next_queue
}

fn runtime_service_loop_daemon_request_monitored_close_reasons(
    plan: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
) -> Vec<String> {
    let mut reasons = plan.monitored_close_health.reasons.clone();

    if plan.requires_repair_first {
        extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_requires_repair_first"
                    .to_owned(),
            ],
        );
    }

    match plan.request_health_status {
        AgentClosedLoopExecutionHealthStatus::Stable => {}
        AgentClosedLoopExecutionHealthStatus::Watch => extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_close_request_watch".to_owned()],
        ),
        AgentClosedLoopExecutionHealthStatus::Repair => extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_close_request_repair".to_owned()],
        ),
    }

    match plan.daemon_control_health_status {
        AgentClosedLoopExecutionHealthStatus::Stable => {}
        AgentClosedLoopExecutionHealthStatus::Watch => extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_daemon_control_watch"
                    .to_owned(),
            ],
        ),
        AgentClosedLoopExecutionHealthStatus::Repair => extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_daemon_control_repair"
                    .to_owned(),
            ],
        ),
    }

    if !plan.can_schedule {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_close_unschedulable".to_owned()],
        );
    }

    if plan.side_effect_dispatch_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_side_effect_dispatch_closed"
                    .to_owned(),
            ],
        );
    }

    if plan.memory_note_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_memory_note_closed".to_owned(),
            ],
        );
    }

    if !plan.allows_adaptive_evolution {
        extend_ordered_unique(
            &mut reasons,
            vec!["runtime_service_loop_daemon_request_monitored_close_adaptive_closed".to_owned()],
        );
    }

    reasons
}

fn runtime_service_loop_daemon_request_monitored_close_continuation_reasons(
    continuation: &AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation,
) -> Vec<String> {
    let mut reasons = continuation.monitored_close_health.reasons.clone();

    if continuation.requires_repair_first {
        extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_continuation_requires_repair_first"
                    .to_owned(),
            ],
        );
    }

    match continuation.request_health_status {
        AgentClosedLoopExecutionHealthStatus::Stable => {}
        AgentClosedLoopExecutionHealthStatus::Watch => extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_continuation_request_watch"
                    .to_owned(),
            ],
        ),
        AgentClosedLoopExecutionHealthStatus::Repair => extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_continuation_request_repair"
                    .to_owned(),
            ],
        ),
    }

    match continuation.daemon_control_health_status {
        AgentClosedLoopExecutionHealthStatus::Stable => {}
        AgentClosedLoopExecutionHealthStatus::Watch => extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_continuation_daemon_control_watch"
                    .to_owned(),
            ],
        ),
        AgentClosedLoopExecutionHealthStatus::Repair => extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_continuation_daemon_control_repair"
                    .to_owned(),
            ],
        ),
    }

    if !continuation.can_schedule {
        extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_continuation_unschedulable"
                    .to_owned(),
            ],
        );
    }

    if continuation.side_effect_dispatch_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_continuation_side_effect_dispatch_closed"
                    .to_owned(),
            ],
        );
    }

    if continuation.memory_note_allowed_rate <= 0.0 {
        extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_continuation_memory_note_closed"
                    .to_owned(),
            ],
        );
    }

    if !continuation.allows_adaptive_evolution {
        extend_ordered_unique(
            &mut reasons,
            vec![
                "runtime_service_loop_daemon_request_monitored_close_continuation_adaptive_closed"
                    .to_owned(),
            ],
        );
    }

    reasons
}

fn first_reason_or(reasons: &[String], fallback: &str) -> String {
    reasons
        .first()
        .cloned()
        .unwrap_or_else(|| fallback.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::{
        AggregationConflictReviewHealthPolicy, AggregationConflictReviewSummaryHistory,
        AggregationConflictReviewSummaryHistoryRecorder, AggregationConflictReviewTrendGate,
        AggregationConflictReviewer, AggregationHealthPolicy, AggregationReport,
        AggregationSummaryHistory,
    };
    use crate::budget::{
        AgentBudget, BudgetLedger, BudgetLedgerHealthPolicy, BudgetLedgerSummary,
        BudgetLedgerSummaryHistory, BudgetLedgerSummaryHistoryRecorder, BudgetPolicy,
    };
    use crate::conflict::{AgentConflict, ConflictReport, ConflictReportSummaryHistoryRecorder};
    use crate::cycle::{AgentCycleEvidence, AgentCycleReport};
    use crate::eval::{
        AgentReportEvidence, AgentReportGateHealthGateHealthPolicy,
        AgentReportGateHealthGateSummaryHistory, AgentReportGateHealthGateTrendHandoffHealthPolicy,
        AgentReportGateHealthGateTrendHandoffHistory,
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy,
        AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
        AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory, AgentReportGateHealthPolicy,
        AgentReportGateHealthStatus, AgentReportGateReason, AgentReportGateSummaryHistory,
    };
    use crate::evolution::{
        EvolutionAdmissionGate, EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy,
        EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord,
        EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder,
        EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary,
        EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory,
        EvolutionAdmissionHealthStatus, EvolutionSignal, ProcessRewardComponents,
        ProcessRewardReport, ProcessRewardReportHealthPolicy, ProcessRewardReportSummaryHistory,
        ProcessRewardReportSummaryHistoryRecorder, ReflectionRewardAdmissionGate, RewardAction,
        ToolBuildStatus, ToolIntent, ToolProposal, ToolsmithPlan, ToolsmithPlanHealthPolicy,
        ToolsmithPlanSummaryHistory, ToolsmithPlanSummaryHistoryRecorder,
    };
    use crate::memory::{
        MemoryPromotionGate, MemorySubmissionHealth, MemorySubmissionHealthPolicy,
        MemorySubmissionReport, MemorySubmissionSummaryHistory,
        MemorySubmissionSummaryHistoryRecorder,
    };
    use crate::message::{AgentMessage, AgentMessageKind};
    use crate::ports::{
        MemoryNote, ToolBuildReport, ToolBuildReportHealthPolicy, ToolBuildReportSummaryHistory,
        ToolBuildReportSummaryHistoryRecorder, ToolBuildRequest,
    };
    use crate::reflection::{
        ReflectionLoop, ReflectionLoopHealthPolicy, ReflectionLoopSummary,
        ReflectionLoopSummaryHistory, ReflectionLoopSummaryHistoryRecorder, ReflectionStage,
    };
    use crate::run::{
        AgentRunLedger, AgentRunLedgerAdmission, AgentRunReport,
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision,
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary,
        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
        AgentRunReportHealthStatus, RunBudgetAudit, SideEffectGate, SideEffectKind,
    };
    use crate::schedule::{
        RecursiveAgentScheduleHealthPolicy, RecursiveAgentScheduleSummary,
        RecursiveAgentScheduleSummaryHistory, RecursiveAgentScheduleSummaryHistoryRecorder,
        RecursiveAgentScheduler,
    };
    use crate::service::{
        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision,
        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy,
        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary,
        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory,
    };
    use crate::step::{
        AgentClosedLoopExecutionHealthPolicy, AgentClosedLoopExecutionHealthStatus,
        AgentClosedLoopExecutionHistory, AgentClosedLoopExecutionSummary,
        AgentClosedLoopNextTurnMode, AgentClosedLoopNextTurnPlan,
    };
    use crate::task::{
        AgentRole, AgentTask, DispatchPlanner, TaskAssignment, TaskDispatchPlan,
        TaskDispatchPlanSummary,
    };
    use crate::turn::{
        AgentClosedLoopRuntimeBusinessInput, AgentClosedLoopRuntimeContinuationInput,
        AgentClosedLoopRuntimeServiceLoopRunControlHealth,
        AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy,
        AgentClosedLoopRuntimeServiceLoopRunControlPlan,
        AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory,
        AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation,
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth,
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation,
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth,
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan,
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory,
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation,
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan,
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan,
        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory,
        AgentClosedLoopRuntimeServiceLoopRunDashboard, AgentClosedLoopRuntimeServiceLoopRunHealth,
        AgentClosedLoopRuntimeServiceLoopRunHealthPolicy,
        AgentClosedLoopRuntimeServiceLoopRunHistory, AgentClosedLoopRuntimeServicePreflight,
        AgentClosedLoopRuntimeServicePreflightContinuationPlanner,
        AgentClosedLoopRuntimeServiceRequestInput, AgentClosedLoopRuntimeServiceRunHealthPolicy,
        AgentClosedLoopRuntimeServiceRunHistory, AgentClosedLoopRuntimeServiceRunHistoryRecord,
        AgentClosedLoopRuntimeServiceRunStatus, AgentClosedLoopRuntimeTurnInput,
    };
    use crate::{
        AggregationConflictReviewTrendGateDecision, ConflictReportHealthPolicy,
        ConflictReportSummaryHistory, ReflectionLoopHistoryGateDecision,
    };

    fn business_queue() -> AgentTaskQueue {
        AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue business loop",
            AgentBudget::new(8, 1, 1),
        )])
    }

    fn final_handoff_packet_from_boundary_record(
        boundary_record: AgentAdapterBoundaryRecord,
        next_queue: &AgentTaskQueue,
    ) -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoffRecord {
        let handoff =
            AgentAdapterBoundaryHandoff::from_record_and_queue(boundary_record, next_queue);
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = monitor_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
            .record_and_gate(
            downstream_handoff,
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(
            ),
        )
    }

    fn cycle_report_with_run_ledger_admission(
        admission: AgentRunLedgerAdmission,
    ) -> AgentCycleReport {
        let dispatch = if admission.can_build_ledger {
            let task = AgentTask::new(
                "cycle-core",
                AgentRole::Planner,
                "continue cycle after clean ledger admission",
                AgentBudget::new(4, 1, 1),
            );
            TaskDispatchPlan {
                assignments: vec![TaskAssignment {
                    task_id: task.id.clone(),
                    role: task.role.clone(),
                    lane: task.lane.clone(),
                    budget_reserved: task.required_budget,
                }],
                ..TaskDispatchPlan::default()
            }
        } else {
            TaskDispatchPlan {
                rejections: vec![crate::task::TaskRejection {
                    task_id: "oversized-review".to_owned(),
                    role: AgentRole::Reviewer,
                    reason:
                        "insufficient budget requested=tokens:8 steps:1 messages:1 remaining=tokens:4 steps:1 messages:1"
                            .to_owned(),
                }],
                ..TaskDispatchPlan::default()
            }
        };

        let side_effects = if admission.can_submit_memory_note {
            vec![
                SideEffectGate::allow(SideEffectKind::MemoryNote, "clean"),
                SideEffectGate::allow(SideEffectKind::AdaptiveStateWrite, "clean"),
            ]
        } else {
            vec![
                SideEffectGate::block(
                    SideEffectKind::MemoryNote,
                    "run_ledger_closed:dispatch rejected before ledger construction",
                ),
                SideEffectGate::block(
                    SideEffectKind::AdaptiveStateWrite,
                    "run_ledger_closed:dispatch rejected before ledger construction",
                ),
            ]
        };

        AgentCycleReport {
            dispatch,
            execution_failures: Vec::new(),
            run_ledger_admission: admission,
            run_report: AgentRunReport {
                aggregation: AggregationReport::default(),
                conflicts: ConflictReport::default(),
                budget_audit: RunBudgetAudit::default(),
                side_effects,
            },
            reward_report: ProcessRewardReport {
                total: 0.20,
                components: ProcessRewardComponents::default(),
                action: RewardAction::Hold,
                notes: Vec::new(),
                evolution_signals: Vec::new(),
            },
            tool_build_report: None,
            follow_up_tasks: Vec::new(),
            memory_promotions: Vec::new(),
        }
    }

    fn all_stable_snapshot() -> AgentAdapterBoundarySnapshot {
        AgentAdapterBoundarySnapshot::from_gates(
            &business_queue(),
            vec![
                AgentAdapterBoundaryGate::stable(AgentAdapterBoundaryOwner::NorionMemory),
                AgentAdapterBoundaryGate::stable(AgentAdapterBoundaryOwner::EvalReporting),
                AgentAdapterBoundaryGate::stable(AgentAdapterBoundaryOwner::NorionCore),
                AgentAdapterBoundaryGate::stable(AgentAdapterBoundaryOwner::ServiceAdapter),
            ],
        )
    }

    fn clean_dispatch_gate() -> TaskDispatchGateDecision {
        TaskDispatchGateDecision {
            summary: TaskDispatchPlanSummary {
                assignments: 1,
                rejections: 0,
                remaining_roles: 1,
                remaining_tokens: 8,
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
            },
            can_dispatch: true,
            can_promote_side_effects: true,
            requires_repair_first: false,
            reasons: Vec::new(),
            telemetry: Vec::new(),
        }
    }

    fn run_report_final_gate_decision(
        status: AgentRunReportHealthStatus,
        admitted: bool,
        requires_repair_first: bool,
        blocked_reasons: Vec<String>,
    ) -> AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision {
        let mut next_queue = business_queue();
        let repair_tasks = if requires_repair_first {
            vec![AgentTask::new(
                "run-report-final-repair",
                AgentRole::Reviewer,
                "repair final run report packet",
                AgentBudget::new(2, 1, 1),
            )]
        } else {
            Vec::new()
        };
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let summary = AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary {
            packet_health_status: status,
            requested_admitted: true,
            admitted,
            requires_repair_first,
            packet_records: 1,
            repair_tasks: repair_tasks.len(),
            next_queue_tasks: next_queue.len(),
            blocked_reasons: blocked_reasons.len(),
            repair_task_ids: repair_tasks.iter().map(|task| task.id.clone()).collect(),
            next_queue_task_ids: next_queue.task_ids(),
            telemetry: Vec::new(),
        };
        let admission_health =
            AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::from_summaries(
                vec![summary],
            )
            .health(
                AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy {
                    maximum_blocked_reasons: usize::MAX,
                    ..AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default()
                },
            );

        AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision {
            requested_admitted: true,
            admission_health,
            admitted,
            requires_repair_first,
            repair_tasks,
            next_queue,
            blocked_reasons,
            telemetry: Vec::new(),
        }
    }

    fn service_execution_final_gate_decision(
        status: AgentClosedLoopExecutionHealthStatus,
        admitted: bool,
        requires_repair_first: bool,
        blocked_reasons: Vec<String>,
    ) -> AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision {
        let mut next_queue = business_queue();
        let repair_tasks = if requires_repair_first {
            vec![AgentTask::new(
                "service-execution-final-repair",
                AgentRole::Reviewer,
                "repair final service execution packet",
                AgentBudget::new(2, 1, 1),
            )]
        } else {
            Vec::new()
        };
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let summary = AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummary {
            monitor_health_status: status,
            requested_admitted: true,
            admitted,
            requires_repair_first,
            monitor_records: 1,
            repair_tasks: repair_tasks.len(),
            next_queue_tasks: next_queue.len(),
            blocked_reasons: blocked_reasons.len(),
            repair_task_ids: repair_tasks.iter().map(|task| task.id.clone()).collect(),
            next_queue_task_ids: next_queue.task_ids(),
            telemetry: Vec::new(),
        };
        let handoff_health =
            AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffSummaryHistory::from_summaries(
                vec![summary],
            )
            .health(
                AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy {
                    maximum_blocked_reasons: usize::MAX,
                    ..AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHealthPolicy::default()
                },
            );

        AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGateDecision {
            requested_admitted: true,
            handoff_health,
            admitted,
            requires_repair_first,
            repair_tasks,
            next_queue,
            blocked_reasons,
            telemetry: Vec::new(),
        }
    }

    fn runtime_service_loop_control_plan(
        status: AgentClosedLoopExecutionHealthStatus,
        mode: AgentClosedLoopNextTurnMode,
        next_queue: AgentTaskQueue,
        reasons: Vec<String>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunControlPlan {
        let health = AgentClosedLoopRuntimeServiceLoopRunHealth {
            status,
            reasons: reasons.clone(),
            dashboard: AgentClosedLoopRuntimeServiceLoopRunDashboard {
                total_runs: 1,
                closed_runs: usize::from(status == AgentClosedLoopExecutionHealthStatus::Stable),
                dispatch_blocked_runs: 0,
                intake_blocked_runs: 0,
                command_gate_allowed_runs: usize::from(mode.can_schedule()),
                repair_first_runs: usize::from(
                    status == AgentClosedLoopExecutionHealthStatus::Repair,
                ),
                side_effect_dispatch_allowed_runs: usize::from(mode.can_schedule()),
                memory_note_allowed_runs: usize::from(mode.allows_adaptive_evolution()),
                adaptive_allowed_runs: usize::from(mode.allows_adaptive_evolution()),
                closed_rate: if status == AgentClosedLoopExecutionHealthStatus::Stable {
                    1.0
                } else {
                    0.0
                },
                command_gate_allowed_rate: if mode.can_schedule() { 1.0 } else { 0.0 },
                repair_first_rate: if status == AgentClosedLoopExecutionHealthStatus::Repair {
                    1.0
                } else {
                    0.0
                },
                side_effect_dispatch_allowed_rate: if mode.can_schedule() { 1.0 } else { 0.0 },
                memory_note_allowed_rate: if mode.allows_adaptive_evolution() {
                    1.0
                } else {
                    0.0
                },
                adaptive_allowed_rate: if mode.allows_adaptive_evolution() {
                    1.0
                } else {
                    0.0
                },
                command_count: usize::from(mode.can_schedule()),
                side_effect_gate_count: 0,
                blocked_side_effect_gate_count: 0,
                follow_up_task_count: 0,
                total_next_queue_tasks: next_queue.len(),
                latest_status: Some(AgentClosedLoopRuntimeServiceRunStatus::Closed),
                latest_mode: Some(mode),
                latest_blocked_reasons: reasons.clone(),
                latest_preflight_reasons: Vec::new(),
            },
        };

        AgentClosedLoopRuntimeServiceLoopRunControlPlan {
            health,
            mode,
            next_queue,
            reasons,
            telemetry: Vec::new(),
        }
    }

    fn runtime_service_loop_daemon_request_monitored_close_plan(
        monitored_close_status: AgentClosedLoopExecutionHealthStatus,
        request_status: AgentClosedLoopExecutionHealthStatus,
        daemon_control_status: AgentClosedLoopExecutionHealthStatus,
        mode: AgentClosedLoopNextTurnMode,
        can_schedule: bool,
        side_effect_dispatch_allowed_rate: f32,
        memory_note_allowed_rate: f32,
        allows_adaptive_evolution: bool,
        requires_repair_first: bool,
        next_queue: AgentTaskQueue,
        reasons: Vec<String>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan {
        let runtime_input = AgentClosedLoopRuntimeTurnInput::new(
            AgentClosedLoopExecutionHistory::new(),
            next_queue,
            BudgetLedger::new(),
            AgentCycleEvidence::default(),
        );
        let business_input = AgentClosedLoopRuntimeBusinessInput::new(
            "adapter-monitored-close",
            crate::ledger::AgentCycleLedger::new(),
            AgentReportEvidence::new(true, true),
        );
        let continuation_input = AgentClosedLoopRuntimeContinuationInput::new(
            BudgetLedger::new(),
            AgentCycleEvidence::default(),
        );
        let request_summary_history =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistory::new();
        let request_health = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestHealth {
            status: request_status,
            reasons: if request_status == AgentClosedLoopExecutionHealthStatus::Stable {
                Vec::new()
            } else {
                vec!["daemon_request_health_not_stable".to_owned()]
            },
            dashboard: request_summary_history.dashboard(),
        };
        let control_summary_history =
            AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::new();
        let daemon_control_health = AgentClosedLoopRuntimeServiceLoopRunControlHealth {
            status: daemon_control_status,
            reasons: if daemon_control_status == AgentClosedLoopExecutionHealthStatus::Stable {
                Vec::new()
            } else {
                vec!["daemon_control_health_not_stable".to_owned()]
            },
            dashboard: control_summary_history.dashboard(),
        };
        let monitored_close_summary_history =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory::new();
        let monitored_close_health =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseHealth {
                status: monitored_close_status,
                reasons,
                dashboard: monitored_close_summary_history.dashboard(),
            };
        let request_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan {
            request_input: AgentClosedLoopRuntimeServiceRequestInput::new(
                runtime_input,
                business_input,
            ),
            continuation_input,
            service_run_history: AgentClosedLoopRuntimeServiceRunHistory::new(),
            loop_run_history: AgentClosedLoopRuntimeServiceLoopRunHistory::new(),
            control_summary_history,
            service_run_policy: AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
            loop_run_health_policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            control_health_policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default(
            ),
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            telemetry: Vec::new(),
        };
        let monitored_plan = AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan {
            request_plan,
            request_summary_history,
            request_health,
            daemon_control_health,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            telemetry: Vec::new(),
        };

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlan {
            monitored_plan,
            monitored_close_summary_history,
            monitored_close_health,
            request_health_status: request_status,
            daemon_control_health_status: daemon_control_status,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            telemetry: Vec::new(),
        }
    }

    fn runtime_service_loop_daemon_request_plan(
        mode: AgentClosedLoopNextTurnMode,
        can_schedule: bool,
        side_effect_dispatch_allowed_rate: f32,
        memory_note_allowed_rate: f32,
        allows_adaptive_evolution: bool,
        requires_repair_first: bool,
        next_queue: AgentTaskQueue,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlan {
        runtime_service_loop_daemon_request_monitored_close_plan(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            next_queue,
            Vec::new(),
        )
        .monitored_plan
        .request_plan
    }

    fn runtime_service_loop_daemon_request_monitored_plan(
        request_status: AgentClosedLoopExecutionHealthStatus,
        daemon_control_status: AgentClosedLoopExecutionHealthStatus,
        mode: AgentClosedLoopNextTurnMode,
        can_schedule: bool,
        side_effect_dispatch_allowed_rate: f32,
        memory_note_allowed_rate: f32,
        allows_adaptive_evolution: bool,
        requires_repair_first: bool,
        next_queue: AgentTaskQueue,
        request_reasons: Vec<String>,
        daemon_control_reasons: Vec<String>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlan {
        let plan = runtime_service_loop_daemon_request_monitored_close_plan(
            AgentClosedLoopExecutionHealthStatus::Stable,
            request_status,
            daemon_control_status,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            next_queue,
            Vec::new(),
        );
        let mut monitored_plan = plan.monitored_plan;
        monitored_plan.request_health.status = request_status;
        monitored_plan.request_health.reasons = request_reasons;
        monitored_plan.daemon_control_health.status = daemon_control_status;
        monitored_plan.daemon_control_health.reasons = daemon_control_reasons;
        monitored_plan
    }

    fn runtime_service_loop_daemon_request_monitored_close_continuation(
        monitored_close_status: AgentClosedLoopExecutionHealthStatus,
        request_status: AgentClosedLoopExecutionHealthStatus,
        daemon_control_status: AgentClosedLoopExecutionHealthStatus,
        mode: AgentClosedLoopNextTurnMode,
        can_schedule: bool,
        side_effect_dispatch_allowed_rate: f32,
        memory_note_allowed_rate: f32,
        allows_adaptive_evolution: bool,
        requires_repair_first: bool,
        next_queue: AgentTaskQueue,
        reasons: Vec<String>,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation {
        let plan = runtime_service_loop_daemon_request_monitored_close_plan(
            monitored_close_status,
            request_status,
            daemon_control_status,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            next_queue,
            reasons,
        );
        let daemon_continuation = AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation {
            next_runtime_input: plan
                .monitored_plan
                .request_plan
                .request_input
                .runtime_input
                .clone(),
            service_run_history: plan.monitored_plan.request_plan.service_run_history.clone(),
            loop_run_history: plan.monitored_plan.request_plan.loop_run_history.clone(),
            control_summary_history: plan
                .monitored_plan
                .request_plan
                .control_summary_history
                .clone(),
            service_run_policy: plan.monitored_plan.request_plan.service_run_policy,
            loop_run_health_policy: plan.monitored_plan.request_plan.loop_run_health_policy,
            control_health_policy: plan.monitored_plan.request_plan.control_health_policy,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            transition_health_status: monitored_close_status,
            control_health_status: daemon_control_status,
            telemetry: Vec::new(),
        };
        let monitored_continuation =
            AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation {
                daemon_continuation,
                request_summary_history: plan.monitored_plan.request_summary_history.clone(),
                request_health: plan.monitored_plan.request_health.clone(),
                daemon_control_health: plan.monitored_plan.daemon_control_health.clone(),
                mode,
                can_schedule,
                side_effect_dispatch_allowed_rate,
                memory_note_allowed_rate,
                allows_adaptive_evolution,
                requires_repair_first,
                telemetry: Vec::new(),
            };

        AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation {
            monitored_continuation,
            monitored_close_summary_history: plan.monitored_close_summary_history,
            monitored_close_health: plan.monitored_close_health,
            request_health_status: request_status,
            daemon_control_health_status: daemon_control_status,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            telemetry: Vec::new(),
        }
    }

    fn clean_execution_summary(run_id: &str) -> AgentClosedLoopExecutionSummary {
        AgentClosedLoopExecutionSummary {
            run_id: run_id.to_owned(),
            clean: true,
            report_accepted: true,
            loopback_promoted: true,
            service_clean: true,
            reward_total: 1.0,
            admission_status: crate::ledger::AgentCycleLedgerAdmissionStatus::Promote,
            command_count: 1,
            missing_command_count: 0,
            failed_command_count: 0,
            skipped_command_count: 0,
            next_queue_tasks: 1,
            next_queue_task_ids: vec!["business-task".to_owned()],
            blocked_reasons: Vec::new(),
        }
    }

    fn stable_service_run_summary(
        _run_id: &str,
    ) -> crate::turn::AgentClosedLoopRuntimeServiceRunSummary {
        crate::turn::AgentClosedLoopRuntimeServiceRunSummary {
            status: AgentClosedLoopRuntimeServiceRunStatus::Closed,
            dispatch_executable: true,
            command_count: 1,
            command_gate_allowed: true,
            side_effect_gate_count: 3,
            blocked_side_effect_gate_count: 0,
            command_kinds: vec!["promote_memory".to_owned()],
            gate_blocked_reasons: Vec::new(),
            outcome_closed: true,
            intake_clean: true,
            intake_blocked_reasons: Vec::new(),
            repair_task_count: 0,
            health_status: AgentClosedLoopExecutionHealthStatus::Stable,
            next_queue_tasks: 1,
            immediate_ready_tasks: 1,
            history_runs: 1,
            telemetry: Vec::new(),
        }
    }

    fn runtime_service_preflight(
        execution_status: AgentClosedLoopExecutionHealthStatus,
        service_status: AgentClosedLoopExecutionHealthStatus,
        next_queue: AgentTaskQueue,
    ) -> AgentClosedLoopRuntimeServicePreflight {
        let execution_history = if execution_status == AgentClosedLoopExecutionHealthStatus::Stable
        {
            AgentClosedLoopExecutionHistory::from_summaries(vec![clean_execution_summary(
                "adapter-preflight-execution",
            )])
        } else {
            AgentClosedLoopExecutionHistory::new()
        };
        let execution_policy = if execution_status == AgentClosedLoopExecutionHealthStatus::Repair {
            AgentClosedLoopExecutionHealthPolicy {
                maximum_service_failure_pressure: -1.0,
                ..AgentClosedLoopExecutionHealthPolicy::default()
            }
        } else {
            AgentClosedLoopExecutionHealthPolicy::default()
        };
        let mut turn_plan = AgentClosedLoopNextTurnPlan::from_history(
            execution_history,
            next_queue,
            execution_policy,
        );
        if execution_status == AgentClosedLoopExecutionHealthStatus::Repair {
            turn_plan.mode = AgentClosedLoopNextTurnMode::Repair;
            turn_plan
                .reasons
                .push("adapter_preflight_execution_repair".to_owned());
        }

        let service_run_history = if service_status == AgentClosedLoopExecutionHealthStatus::Stable
        {
            AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
                stable_service_run_summary("adapter-preflight-service"),
            ])
        } else {
            AgentClosedLoopRuntimeServiceRunHistory::new()
        };
        let service_run_policy = if service_status == AgentClosedLoopExecutionHealthStatus::Repair {
            AgentClosedLoopRuntimeServiceRunHealthPolicy {
                maximum_dispatch_blocked_runs: usize::MAX,
                maximum_intake_blocked_runs: usize::MAX,
                maximum_repair_task_count: usize::MAX,
                minimum_closed_rate: 2.0,
            }
        } else {
            AgentClosedLoopRuntimeServiceRunHealthPolicy::default()
        };
        let mut service_run_health = service_run_history.health(service_run_policy);
        if service_status == AgentClosedLoopExecutionHealthStatus::Repair {
            service_run_health.status = AgentClosedLoopExecutionHealthStatus::Repair;
            service_run_health
                .reasons
                .push("adapter_preflight_service_repair".to_owned());
        }

        AgentClosedLoopRuntimeServicePreflight::from_parts(turn_plan, service_run_health)
    }

    fn runtime_service_loop_state(
        execution_status: AgentClosedLoopExecutionHealthStatus,
        service_status: AgentClosedLoopExecutionHealthStatus,
        next_queue: AgentTaskQueue,
    ) -> AgentClosedLoopRuntimeServiceLoopState {
        let preflight = runtime_service_preflight(execution_status, service_status, next_queue);
        let execution_history = preflight.turn_plan.history.clone();
        let service_run_history = if service_status == AgentClosedLoopExecutionHealthStatus::Stable
        {
            AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![
                stable_service_run_summary("adapter-loop-state-service"),
            ])
        } else {
            AgentClosedLoopRuntimeServiceRunHistory::new()
        };
        let preflight_continuation =
            AgentClosedLoopRuntimeServicePreflightContinuationPlanner::new().plan(
                preflight,
                AgentClosedLoopRuntimeContinuationInput::new(
                    BudgetLedger::new(),
                    AgentCycleEvidence::default(),
                ),
            );

        AgentClosedLoopRuntimeServiceLoopState {
            execution_history,
            service_run_history,
            preflight_continuation,
            telemetry: Vec::new(),
        }
    }

    fn runtime_service_loop_advance(
        state: AgentClosedLoopRuntimeServiceLoopState,
    ) -> AgentClosedLoopRuntimeServiceLoopAdvance {
        let appended_summary = stable_service_run_summary("adapter-loop-advance-service");
        let history =
            AgentClosedLoopRuntimeServiceRunHistory::from_summaries(vec![appended_summary.clone()]);
        let dashboard = history.dashboard();
        let health = dashboard.health(AgentClosedLoopRuntimeServiceRunHealthPolicy::default());
        let run_record = AgentClosedLoopRuntimeServiceRunHistoryRecord {
            history,
            appended_summary,
            dashboard,
            health,
            telemetry: Vec::new(),
        };
        let summary = state.summary();

        AgentClosedLoopRuntimeServiceLoopAdvance {
            run_record,
            loop_state: state,
            summary,
            telemetry: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn runtime_service_loop_daemon_continuation(
        transition_status: AgentClosedLoopExecutionHealthStatus,
        control_status: AgentClosedLoopExecutionHealthStatus,
        mode: AgentClosedLoopNextTurnMode,
        can_schedule: bool,
        side_effect_dispatch_allowed_rate: f32,
        memory_note_allowed_rate: f32,
        allows_adaptive_evolution: bool,
        requires_repair_first: bool,
        next_queue: AgentTaskQueue,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation {
        AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation {
            next_runtime_input: AgentClosedLoopRuntimeTurnInput::new(
                AgentClosedLoopExecutionHistory::new(),
                next_queue,
                BudgetLedger::new(),
                AgentCycleEvidence::default(),
            ),
            service_run_history: AgentClosedLoopRuntimeServiceRunHistory::new(),
            loop_run_history: AgentClosedLoopRuntimeServiceLoopRunHistory::new(),
            control_summary_history: AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::new(
            ),
            service_run_policy: AgentClosedLoopRuntimeServiceRunHealthPolicy::default(),
            loop_run_health_policy: AgentClosedLoopRuntimeServiceLoopRunHealthPolicy::default(),
            control_health_policy: AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy::default(
            ),
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            transition_health_status: transition_status,
            control_health_status: control_status,
            telemetry: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn runtime_service_loop_daemon_input_plan(
        transition_status: AgentClosedLoopExecutionHealthStatus,
        control_status: AgentClosedLoopExecutionHealthStatus,
        mode: AgentClosedLoopNextTurnMode,
        can_schedule: bool,
        side_effect_dispatch_allowed_rate: f32,
        memory_note_allowed_rate: f32,
        allows_adaptive_evolution: bool,
        requires_repair_first: bool,
        next_queue: AgentTaskQueue,
    ) -> AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlan {
        let continuation = runtime_service_loop_daemon_continuation(
            transition_status,
            control_status,
            mode,
            can_schedule,
            side_effect_dispatch_allowed_rate,
            memory_note_allowed_rate,
            allows_adaptive_evolution,
            requires_repair_first,
            next_queue,
        );

        crate::turn::AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner::new().plan(
            &continuation,
            AgentClosedLoopRuntimeBusinessInput::new(
                "adapter-daemon-input-plan",
                crate::ledger::AgentCycleLedger::new(),
                AgentReportEvidence::new(true, true),
            ),
            Vec::new(),
        )
    }

    fn accepted_report_gate() -> AgentReportGateDecision {
        AgentReportGateDecision {
            accepted: true,
            reasons: Vec::new(),
            follow_up_tasks: Vec::new(),
        }
    }

    fn rejected_report_gate() -> AgentReportGateDecision {
        AgentReportGateDecision {
            accepted: false,
            reasons: vec![AgentReportGateReason::new(
                "validation_evidence_missing",
                "true",
            )],
            follow_up_tasks: Vec::new(),
        }
    }

    fn memory_submission_gate(can_commit: bool) -> MemorySubmissionGateDecision {
        MemorySubmissionGateDecision {
            summary: crate::memory::MemorySubmissionSummary {
                submitted_notes: usize::from(can_commit),
                failed_notes: 0,
                blocked_reasons: 0,
                attempted_notes: usize::from(can_commit),
                quality_reviewed_notes: 0,
                quality_admitted_notes: 0,
                quality_rejected_notes: 0,
                clean: true,
                port_attempted: can_commit,
                telemetry: Vec::new(),
            },
            can_continue_loop: true,
            can_commit_submitted_notes: can_commit,
            requires_repair_first: false,
            reasons: Vec::new(),
            telemetry: Vec::new(),
        }
    }

    fn stable_reflection_gate() -> ReflectionLoopHistoryGateDecision {
        stable_reflection_record().gate_decision
    }

    fn stable_reflection_record() -> ReflectionLoopHistoryGateRecord {
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft accepted")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "no blocker")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "keep memory evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember clean handoff")
            .unwrap();

        ReflectionLoopSummaryHistoryRecorder::new().record_loop_with_health_gate(
            ReflectionLoopSummaryHistory::new(),
            &loop_state,
            ReflectionLoopHealthPolicy::default(),
        )
    }

    fn incomplete_reflection_record() -> ReflectionLoopHistoryGateRecord {
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft still needs critique")
            .unwrap();

        ReflectionLoopSummaryHistoryRecorder::new().record_loop_with_health_gate(
            ReflectionLoopSummaryHistory::new(),
            &loop_state,
            ReflectionLoopHealthPolicy {
                maximum_incomplete_records: 1,
                minimum_completion_rate: 0.0,
                minimum_memory_note_ready_rate: 0.0,
                maximum_missing_memory_note_records: 0,
                maximum_stalled_stage_records: 0,
            },
        )
    }

    fn stalled_reflection_record() -> ReflectionLoopHistoryGateRecord {
        let stalled = ReflectionLoopSummary {
            entries: 1,
            next_stage: ReflectionStage::Critique,
            is_complete: false,
            memory_note_ready: false,
            remaining_stages: vec![
                ReflectionStage::Critique,
                ReflectionStage::Revision,
                ReflectionStage::MemoryNote,
            ],
            telemetry: Vec::new(),
        };
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft accepted")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "no blocker")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "keep memory evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember clean handoff")
            .unwrap();

        ReflectionLoopSummaryHistoryRecorder::new().record_loop_with_health_gate(
            ReflectionLoopSummaryHistory::from_summaries(vec![stalled.clone(), stalled]),
            &loop_state,
            ReflectionLoopHealthPolicy::default(),
        )
    }

    fn clean_review_trend_gate() -> AggregationConflictReviewTrendGateDecision {
        let review = AggregationConflictReviewer::new().review_messages(
            vec![AgentMessage::new(
                "m1",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "memory",
                "remember clean handoff",
            )],
            AggregationSummaryHistory::new(),
            AggregationHealthPolicy::default(),
            ConflictReportSummaryHistory::new(),
            ConflictReportHealthPolicy::default(),
        );
        let record = AggregationConflictReviewSummaryHistoryRecorder::new()
            .record_review_with_health(
                AggregationConflictReviewSummaryHistory::new(),
                &review,
                AggregationConflictReviewHealthPolicy::default(),
            );

        AggregationConflictReviewTrendGate::new().gate(&review, &record)
    }

    fn stable_memory_submission_health() -> MemorySubmissionHealth {
        let report = MemorySubmissionReport {
            submitted: vec![MemoryNote::new("agent_cycle", "remember clean handoff")],
            failures: Vec::new(),
            blocked_reasons: Vec::new(),
            note_quality: None,
        };

        MemorySubmissionSummaryHistoryRecorder::new()
            .record_report_with_health(
                MemorySubmissionSummaryHistory::new(),
                &report,
                MemorySubmissionHealthPolicy::default(),
            )
            .health
    }

    fn service_admission(
        mode: AgentClosedLoopNextTurnMode,
        can_dispatch: bool,
        can_promote: bool,
        reason: impl Into<String>,
    ) -> AgentCollaborationAdapterSideEffectAdmission {
        AgentCollaborationAdapterSideEffectAdmission {
            mode,
            health_status: if can_promote {
                AgentClosedLoopExecutionHealthStatus::Stable
            } else {
                AgentClosedLoopExecutionHealthStatus::Watch
            },
            can_dispatch_service_commands: can_dispatch,
            can_promote_memory_note: can_promote,
            can_admit_adaptive_evolution: can_promote,
            requires_repair_first: false,
            gates: vec![
                if can_dispatch {
                    SideEffectGate::allow(SideEffectKind::ExternalCall, "service command")
                } else {
                    SideEffectGate::block(SideEffectKind::ExternalCall, "service command")
                },
                if can_promote {
                    SideEffectGate::allow(SideEffectKind::MemoryNote, "memory note")
                } else {
                    SideEffectGate::block(SideEffectKind::MemoryNote, "memory note")
                },
                if can_promote {
                    SideEffectGate::allow(SideEffectKind::AdaptiveStateWrite, "adaptive state")
                } else {
                    SideEffectGate::block(SideEffectKind::AdaptiveStateWrite, "adaptive state")
                },
            ],
            reasons: vec![reason.into()],
            service_execution_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_closes: 0,
            service_execution_rust_validation_command_count: 0,
            service_execution_rust_validation_command_closes: 0,
            service_execution_tool_build_command_reason_count: 0,
            telemetry: Vec::new(),
        }
    }

    fn evolution_admission_handoff_summary_for_adapter(
        health: EvolutionAdmissionHealthStatus,
        effective_admitted: bool,
        requires_repair_first: bool,
    ) -> EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary {
        let repair_task_ids = if requires_repair_first {
            vec!["evolution-repair".to_owned()]
        } else {
            Vec::new()
        };
        let blocked_reasons = usize::from(requires_repair_first);

        EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary {
            continuation_health_status: health,
            effective_admitted,
            can_promote_ready_proposals: effective_admitted && !requires_repair_first,
            can_promote_evolution_signals: effective_admitted && !requires_repair_first,
            can_reinforce_process: effective_admitted && !requires_repair_first,
            can_promote_adaptive_state: effective_admitted && !requires_repair_first,
            requires_repair_first,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: 1,
            blocked_reasons,
            records: 1,
            repair_task_ids,
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        }
    }

    fn evolution_admission_handoff_history_for_adapter(
        summary: EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary,
        policy: EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy,
    ) -> EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord {
        EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder::new()
            .record_summary_with_health(
                EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory::new(),
                summary,
                policy,
            )
    }

    fn stable_handoff() -> AgentAdapterBoundaryHandoff {
        AgentAdapterBoundarySummaryHistoryRecorder::new().record_boundary_gates_handoff_with_health(
            AgentAdapterBoundarySummaryHistory::new(),
            &business_queue(),
            &clean_dispatch_gate(),
            &memory_submission_gate(true),
            &accepted_report_gate(),
            &service_admission(
                AgentClosedLoopNextTurnMode::Continue,
                true,
                true,
                "stable_service",
            ),
            AgentAdapterBoundaryHealthPolicy::default(),
        )
    }

    fn repair_handoff() -> AgentAdapterBoundaryHandoff {
        AgentAdapterBoundarySummaryHistoryRecorder::new().record_boundary_gates_handoff_with_health(
            AgentAdapterBoundarySummaryHistory::new(),
            &business_queue(),
            &clean_dispatch_gate(),
            &memory_submission_gate(true),
            &rejected_report_gate(),
            &service_admission(
                AgentClosedLoopNextTurnMode::Observe,
                true,
                false,
                "reflection_watch",
            ),
            AgentAdapterBoundaryHealthPolicy::default(),
        )
    }

    fn non_effective_trend_summary() -> AgentAdapterBoundaryHandoffTrendGateSummary {
        AgentAdapterBoundaryHandoffTrendGateSummary {
            handoff_health_status: AgentAdapterBoundaryStatus::Stable,
            requested_admitted: true,
            effective_admitted: false,
            requires_repair_first: false,
            can_submit_memory_note: false,
            can_promote_adaptive_state: false,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 1,
            service_execution_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_closes: 0,
            service_execution_tool_build_command_reason_count: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["previous-business-task".to_owned()],
            telemetry: Vec::new(),
        }
    }

    fn stable_resume_gate_decision() -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDecision
    {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_history_record,
        );
        let history_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
            .record_resume_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                &resume_record,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
            );

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGate::new()
            .gate(&resume_record, &history_record)
    }

    fn repair_resume_gate_decision() -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateDecision
    {
        let repair_handoff_summary = repair_handoff().summary();
        let repair_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
                    repair_handoff_summary,
                ]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let repair_decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &repair_handoff_history_record);
        let clean_handoff = stable_handoff();
        let clean_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &clean_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &clean_handoff,
            &clean_handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                repair_decision.summary(),
            ]),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &clean_handoff,
            &clean_handoff_history_record,
        );
        let history_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
            .record_resume_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                &resume_record,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
            );

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGate::new()
            .gate(&resume_record, &history_record)
    }

    fn stable_resume_gate_monitor_handoff()
    -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord {
        let decision = stable_resume_gate_decision();
        let history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecorder::new()
                .record_decision_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    &decision,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let service_execution_command_reason_count = history_record
            .dashboard
            .service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count = history_record
            .dashboard
            .service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes = history_record
            .dashboard
            .service_execution_memory_promotion_command_reason_closes;
        let service_execution_tool_build_command_reason_count = history_record
            .dashboard
            .service_execution_tool_build_command_reason_count;
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorRecord {
            decision,
            history_record,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry: Vec::new(),
        };
        let gate_decision = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGate::new()
            .gate(&monitor_record);
        let service_execution_command_reason_count =
            gate_decision.service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count =
            gate_decision.service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes =
            gate_decision.service_execution_memory_promotion_command_reason_closes;
        let service_execution_tool_build_command_reason_count =
            gate_decision.service_execution_tool_build_command_reason_count;

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord {
            monitor_record,
            gate_decision,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry: Vec::new(),
        }
    }

    fn repair_resume_gate_monitor_handoff()
    -> AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord {
        let stable_decision = stable_resume_gate_decision();
        let repair_summary = repair_resume_gate_decision().summary();
        let history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecorder::new()
                .record_decision_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::from_summaries(
                    vec![repair_summary],
                ),
                &stable_decision,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
            );
        let service_execution_command_reason_count = history_record
            .dashboard
            .service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count = history_record
            .dashboard
            .service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes = history_record
            .dashboard
            .service_execution_memory_promotion_command_reason_closes;
        let service_execution_tool_build_command_reason_count = history_record
            .dashboard
            .service_execution_tool_build_command_reason_count;
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorRecord {
            decision: stable_decision,
            history_record,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry: Vec::new(),
        };
        let gate_decision = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGate::new()
            .gate(&monitor_record);
        let service_execution_command_reason_count =
            gate_decision.service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count =
            gate_decision.service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes =
            gate_decision.service_execution_memory_promotion_command_reason_closes;
        let service_execution_tool_build_command_reason_count =
            gate_decision.service_execution_tool_build_command_reason_count;

        AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffRecord {
            monitor_record,
            gate_decision,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry: Vec::new(),
        }
    }

    #[test]
    fn boundary_snapshot_promotes_only_when_all_owners_are_stable() {
        let snapshot = all_stable_snapshot();
        let summary = snapshot.summary();

        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Stable);
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
        assert!(snapshot.can_execute_service_commands());
        assert_eq!(summary.owners, 4);
        assert_eq!(summary.stable_owners, 4);
        assert_eq!(summary.next_queue_tasks, 1);
        assert_eq!(
            snapshot
                .gates
                .iter()
                .map(|gate| gate.owner)
                .collect::<Vec<_>>(),
            vec![
                AgentAdapterBoundaryOwner::NorionCore,
                AgentAdapterBoundaryOwner::NorionMemory,
                AgentAdapterBoundaryOwner::ServiceAdapter,
                AgentAdapterBoundaryOwner::EvalReporting,
            ]
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "agent_adapter_boundary_summary_memory_note=true")
        );
    }

    #[test]
    fn watch_boundary_allows_observation_but_not_memory_or_adaptive_promotion() {
        let snapshot = AgentAdapterBoundarySnapshot::from_gates(
            &business_queue(),
            vec![
                AgentAdapterBoundaryGate::stable(AgentAdapterBoundaryOwner::NorionCore),
                AgentAdapterBoundaryGate::watch(
                    AgentAdapterBoundaryOwner::EvalReporting,
                    "eval_history_empty",
                ),
            ],
        );

        let summary = snapshot.summary();

        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Watch);
        assert!(snapshot.allows_service_advance());
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.watch_owners, 1);
        assert_eq!(
            snapshot.blocked_reasons,
            vec!["eval_reporting:eval_history_empty"]
        );
    }

    #[test]
    fn repair_boundary_closes_all_adapter_side_effects() {
        let snapshot = AgentAdapterBoundarySnapshot::from_gates(
            &business_queue(),
            vec![
                AgentAdapterBoundaryGate::stable(AgentAdapterBoundaryOwner::NorionCore),
                AgentAdapterBoundaryGate::repair(
                    AgentAdapterBoundaryOwner::NorionMemory,
                    "unresolved_conflicts=1",
                ),
            ],
        );

        let summary = snapshot.summary();

        assert_eq!(summary.status, AgentAdapterBoundaryStatus::Repair);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.allows_service_advance());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(summary.repair_owners, 1);
        assert_eq!(summary.blocked_reasons, 1);
    }

    #[test]
    fn adapter_boundary_history_repairs_dirty_trends() {
        let clean = all_stable_snapshot().summary();
        let dirty = AgentAdapterBoundarySnapshot::from_gates(
            &business_queue(),
            vec![AgentAdapterBoundaryGate::repair(
                AgentAdapterBoundaryOwner::ServiceAdapter,
                "receipt_intake_blocked=1",
            )],
        )
        .summary();
        let history = AgentAdapterBoundarySummaryHistory::from_summaries(vec![clean]);

        let record = AgentAdapterBoundarySummaryHistoryRecorder::new().record_summary_with_health(
            history,
            dirty,
            AgentAdapterBoundaryHealthPolicy::default(),
        );

        assert_eq!(record.records(), 2);
        assert_eq!(record.dashboard.stable_records, 1);
        assert_eq!(record.dashboard.repair_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.stable_rate, 0.5);
        assert_eq!(record.health.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "adapter_boundary_repair_records=1>0",
                "adapter_boundary_repair_first_records=1>0",
                "adapter_boundary_stable_rate=0.500<0.67",
            ]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "agent_adapter_boundary_history_record_status=repair")
        );
    }

    #[test]
    fn adapter_gate_projects_dispatch_gate_for_core_owner() {
        let clean = TaskDispatchGateDecision {
            summary: TaskDispatchPlanSummary {
                assignments: 1,
                rejections: 0,
                remaining_roles: 1,
                remaining_tokens: 8,
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
            },
            can_dispatch: true,
            can_promote_side_effects: true,
            requires_repair_first: false,
            reasons: Vec::new(),
            telemetry: Vec::new(),
        };
        let dirty = TaskDispatchGateDecision {
            summary: TaskDispatchPlanSummary {
                assignments: 0,
                rejections: 1,
                remaining_roles: 1,
                remaining_tokens: 0,
                remaining_steps: 0,
                remaining_messages: 0,
                remaining_zero_budget_roles: 1,
                remaining_partially_depleted_roles: 0,
                remaining_token_depleted_roles: 1,
                remaining_step_depleted_roles: 1,
                remaining_message_depleted_roles: 1,
                assigned_rate: 0.0,
                rejected_rate: 1.0,
                telemetry: Vec::new(),
            },
            can_dispatch: false,
            can_promote_side_effects: false,
            requires_repair_first: true,
            reasons: vec![
                "dispatch_rejection task=review role=reviewer reason=insufficient budget"
                    .to_owned(),
            ],
            telemetry: Vec::new(),
        };

        let clean_gate = AgentAdapterBoundaryGate::from_dispatch_gate(&clean);
        let dirty_gate = AgentAdapterBoundaryGate::from_dispatch_gate(&dirty);

        assert_eq!(clean_gate.owner, AgentAdapterBoundaryOwner::NorionCore);
        assert_eq!(clean_gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(clean_gate.dispatch_allowed);
        assert_eq!(dirty_gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!dirty_gate.dispatch_allowed);
        assert_eq!(
            dirty_gate.blocked_reasons,
            vec!["dispatch_rejection task=review role=reviewer reason=insufficient budget"]
        );
    }

    #[test]
    fn adapter_gate_projects_partial_dispatch_depletion_as_side_effect_watch() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Coder, AgentBudget::new(20, 1, 2)),
        );
        let task = AgentTask::new(
            "coder-step-limited",
            AgentRole::Coder,
            "consume dispatch step budget",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = planner
            .plan_with_policy(vec![task], &BudgetPolicy::strict())
            .gate();

        let gate = AgentAdapterBoundaryGate::from_dispatch_gate(&dispatch);

        assert!(dispatch.can_dispatch);
        assert!(!dispatch.can_promote_side_effects);
        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionCore);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert_eq!(
            gate.blocked_reasons,
            vec!["dispatch_remaining_partially_depleted_roles=1"]
        );
    }

    #[test]
    fn adapter_gate_projects_clean_budget_ledger_history_as_open_core_boundary() {
        let ledger = BudgetLedger::new()
            .with_budget(AgentRole::Coder, AgentBudget::new(10, 2, 2))
            .with_budget(AgentRole::Reviewer, AgentBudget::new(5, 1, 1));
        let record = BudgetLedgerSummaryHistoryRecorder::new().record_ledger_with_health_gate(
            BudgetLedgerSummaryHistory::new(),
            &ledger,
            BudgetLedgerHealthPolicy::default(),
        );

        let gate = AgentAdapterBoundaryGate::from_budget_ledger_history_gate(&record);
        let snapshot = AgentAdapterBoundarySnapshot::from_budget_ledger_history_gate(
            &business_queue(),
            &record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_budget_ledger_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionCore);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
        assert!(boundary_record.allows_service_advance());
        assert!(!boundary_record.requires_repair_first());
    }

    #[test]
    fn adapter_gate_projects_dirty_budget_ledger_history_as_closed_core_boundary() {
        let depleted = BudgetLedgerSummary {
            roles: 1,
            zero_budget_roles: 1,
            partially_depleted_roles: 0,
            token_depleted_roles: 1,
            step_depleted_roles: 1,
            message_depleted_roles: 1,
            total_tokens: 0,
            total_steps: 0,
            total_messages: 0,
            depleted_roles: vec![AgentRole::Tester],
            dimension_depleted_roles: vec![AgentRole::Tester],
            telemetry: Vec::new(),
        };
        let ledger = BudgetLedger::new()
            .with_budget(AgentRole::Coder, AgentBudget::new(10, 1, 1))
            .with_budget(AgentRole::Reviewer, AgentBudget::new(5, 1, 1));
        let record = BudgetLedgerSummaryHistoryRecorder::new().record_ledger_with_health_gate(
            BudgetLedgerSummaryHistory::from_summaries(vec![depleted]),
            &ledger,
            BudgetLedgerHealthPolicy::default(),
        );

        let gate = AgentAdapterBoundaryGate::from_budget_ledger_history_gate(&record);
        let snapshot = AgentAdapterBoundarySnapshot::from_budget_ledger_history_gate(
            &business_queue(),
            &record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_budget_ledger_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionCore);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(!boundary_record.allows_service_advance());
        assert!(boundary_record.requires_repair_first());
        assert!(
            boundary_record
                .snapshot
                .blocked_reasons
                .iter()
                .any(|reason| reason
                    == "norion_core:budget_ledger_history:budget_ledger_zero_budget_roles=1>0")
        );
    }

    #[test]
    fn adapter_projects_stable_recursive_schedule_history_as_open_core_handoff() {
        let planner = AgentTask::new(
            "planner",
            AgentRole::Planner,
            "plan the next clean work slice",
            AgentBudget::new(4, 1, 1),
        );
        let coder = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "apply the planned patch",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("planner");
        let schedule = RecursiveAgentScheduler::new(2).plan(vec![coder, planner]);
        let schedule_record = RecursiveAgentScheduleSummaryHistoryRecorder::new()
            .record_schedule_with_health_gate(
                RecursiveAgentScheduleSummaryHistory::new(),
                &schedule,
                RecursiveAgentScheduleHealthPolicy::default(),
            );

        let gate = AgentAdapterBoundaryGate::from_recursive_schedule_history_gate(&schedule_record);
        let snapshot = AgentAdapterBoundarySnapshot::from_recursive_schedule_history_gate(
            &business_queue(),
            &schedule_record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_recursive_schedule_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &schedule_record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff =
            AgentAdapterBoundaryHandoff::from_record_and_queue(boundary_record, &business_queue());
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let admission_summary = admission.summary();

        assert!(schedule_record.can_dispatch_waves());
        assert!(!schedule_record.requires_repair_first());
        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionCore);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert!(gate.blocked_reasons.is_empty());
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Stable);
        assert!(!snapshot.requires_repair_first());
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
        assert!(snapshot.blocked_reasons.is_empty());
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(handoff.is_admitted());
        assert!(!handoff.requires_repair_first);
        assert!(handoff.can_execute_service_commands());
        assert!(handoff.can_submit_memory_note());
        assert!(handoff.can_promote_adaptive_state());
        assert!(handoff.repair_tasks.is_empty());
        assert!(handoff.blocked_reasons.is_empty());
        assert_eq!(handoff.next_queue.task_ids(), vec!["business-task"]);
        assert_eq!(handoff_record.appended_summary.repair_tasks, 0);
        assert_eq!(handoff_record.appended_summary.blocked_reasons, 0);
        assert_eq!(handoff_record.dashboard.repair_task_count, 0);
        assert!(trend_decision.is_admitted());
        assert!(!trend_decision.requires_repair_first);
        assert!(trend_decision.repair_tasks.is_empty());
        assert!(trend_decision.blocked_reasons.is_empty());
        assert!(admission.is_admitted());
        assert!(!admission.requires_repair_first());
        assert!(admission.decision.repair_tasks.is_empty());
        assert!(admission.blocked_reasons.is_empty());
        assert_eq!(admission_summary.decision_repair_tasks, 0);
        assert_eq!(admission_summary.blocked_reasons, 0);
        assert_eq!(admission_summary.next_queue_task_ids, vec!["business-task"]);
    }

    #[test]
    fn adapter_projects_blocked_recursive_schedule_as_current_core_repair_handoff() {
        let planner = AgentTask::new(
            "planner",
            AgentRole::Planner,
            "split blocked work",
            AgentBudget::new(4, 1, 1),
        );
        let coder = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "write the schedulable patch",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("planner");
        let orphan = AgentTask::new(
            "memory",
            AgentRole::MemoryCurator,
            "capture review evidence with a missing dependency",
            AgentBudget::new(4, 1, 1),
        )
        .depends_on("missing-review");
        let schedule = RecursiveAgentScheduler::new(2).plan(vec![orphan, coder, planner]);
        let schedule_record = RecursiveAgentScheduleSummaryHistoryRecorder::new()
            .record_schedule_with_health_gate(
                RecursiveAgentScheduleSummaryHistory::new(),
                &schedule,
                RecursiveAgentScheduleHealthPolicy::default(),
            );

        let gate = AgentAdapterBoundaryGate::from_recursive_schedule_history_gate(&schedule_record);
        let snapshot = AgentAdapterBoundarySnapshot::from_recursive_schedule_history_gate(
            &business_queue(),
            &schedule_record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_recursive_schedule_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &schedule_record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record.clone(),
            &business_queue(),
        );
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let trend_summary = trend_decision.summary();
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let admission_summary = admission.summary();
        let admission_record = AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
            .record_admission_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                &admission,
                AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = monitor_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let downstream_summary = downstream_handoff.summary();

        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.blocked_task_ids, vec!["memory"]);
        assert!(schedule_record.requires_repair_first());
        assert!(!schedule_record.can_dispatch_waves());
        assert_eq!(
            schedule_record
                .gate_decision
                .reasons
                .first()
                .map(String::as_str),
            Some("schedule_blocked_tasks=1")
        );
        assert!(
            schedule_record
                .gate_decision
                .reasons
                .iter()
                .any(|reason| reason
                    == "recursive_schedule_history:recursive_schedule_blocked_records=1>0")
        );
        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionCore);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(
            gate.blocked_reasons
                .iter()
                .any(|reason| reason == "schedule_blocked_tasks=1")
        );
        assert!(
            gate.blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("recursive_schedule_history:"))
        );
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Repair);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(
            snapshot
                .blocked_reasons
                .iter()
                .any(|reason| reason == "norion_core:schedule_blocked_tasks=1")
        );
        assert!(snapshot.blocked_reasons.iter().any(|reason| reason
            == "norion_core:recursive_schedule_history:recursive_schedule_blocked_records=1>0"));
        assert!(boundary_record.requires_repair_first());
        assert!(!boundary_record.allows_service_advance());
        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_execute_service_commands());
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "norion_core:schedule_blocked_tasks=1")
        );
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(
            handoff_record.dashboard.repair_task_count,
            handoff.repair_tasks.len()
        );
        assert!(trend_decision.requires_repair_first);
        assert!(!trend_decision.is_admitted());
        assert_eq!(
            trend_summary.repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            trend_summary.blocked_reasons,
            trend_decision.blocked_reasons.len()
        );
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason == "norion_core:schedule_blocked_tasks=1")
        );
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("handoff_history:"))
        );
        assert!(admission.requires_repair_first());
        assert!(!admission.is_admitted());
        assert_eq!(
            admission_summary.decision_repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            admission_summary.blocked_reasons,
            admission.blocked_reasons.len()
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason == "norion_core:schedule_blocked_tasks=1")
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("trend_gate_history:"))
        );
        assert_eq!(admission_record.appended_summary, admission_summary);
        assert_eq!(
            admission_record.dashboard.blocked_reasons,
            admission_summary.blocked_reasons
        );
        assert!(admission_summary.decision_repair_tasks > 0);
        assert!(
            admission_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(!monitor_record.is_admitted());
        assert!(monitor_record.requires_repair_first());
        assert!(!continuation.is_admitted());
        assert!(continuation.requires_repair_first);
        assert!(resume_record.requires_repair_first());
        assert!(!resume_record.is_admitted());
        assert!(!downstream_handoff.is_admitted());
        assert!(downstream_handoff.requires_repair_first());
        assert_eq!(
            downstream_summary.repair_tasks,
            downstream_handoff.gate_decision.repair_tasks.len()
        );
        assert_eq!(
            downstream_summary.blocked_reasons,
            downstream_handoff.gate_decision.blocked_reasons.len()
        );
        assert!(
            downstream_handoff
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_history:"))
        );
        assert!(
            downstream_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
    }

    #[test]
    fn adapter_projects_dirty_recursive_schedule_history_as_core_repair_handoff() {
        let blocked_history = RecursiveAgentScheduleSummary {
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
            "plan the next clean work slice",
            AgentBudget::new(4, 1, 1),
        );
        let schedule = RecursiveAgentScheduler::new(2).plan(vec![planner]);
        let schedule_record = RecursiveAgentScheduleSummaryHistoryRecorder::new()
            .record_schedule_with_health_gate(
                RecursiveAgentScheduleSummaryHistory::from_summaries(vec![blocked_history]),
                &schedule,
                RecursiveAgentScheduleHealthPolicy::default(),
            );

        let gate = AgentAdapterBoundaryGate::from_recursive_schedule_history_gate(&schedule_record);
        let snapshot = AgentAdapterBoundarySnapshot::from_recursive_schedule_history_gate(
            &business_queue(),
            &schedule_record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_recursive_schedule_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &schedule_record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record.clone(),
            &business_queue(),
        );
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let trend_summary = trend_decision.summary();
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let admission_summary = admission.summary();
        let admission_record = AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
            .record_admission_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                &admission,
                AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = monitor_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let downstream_summary = downstream_handoff.summary();

        assert!(schedule_record.requires_repair_first());
        assert!(!schedule_record.can_dispatch_waves());
        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionCore);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(gate.blocked_reasons.iter().any(|reason| {
            reason == "recursive_schedule_history:recursive_schedule_blocked_records=1>0"
        }));
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Repair);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(snapshot.blocked_reasons.iter().any(|reason| {
            reason
                == "norion_core:recursive_schedule_history:recursive_schedule_blocked_records=1>0"
        }));
        assert!(boundary_record.requires_repair_first());
        assert!(!boundary_record.allows_service_advance());
        assert!(!boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_execute_service_commands());
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(handoff.blocked_reasons.iter().any(|reason| {
            reason
                == "norion_core:recursive_schedule_history:recursive_schedule_blocked_records=1>0"
        }));
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(
            handoff_record.dashboard.repair_task_count,
            handoff.repair_tasks.len()
        );
        assert!(trend_decision.requires_repair_first);
        assert!(!trend_decision.is_admitted());
        assert_eq!(
            trend_summary.repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            trend_summary.blocked_reasons,
            trend_decision.blocked_reasons.len()
        );
        assert!(trend_decision.blocked_reasons.iter().any(|reason| {
            reason
                == "norion_core:recursive_schedule_history:recursive_schedule_blocked_records=1>0"
        }));
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("handoff_history:"))
        );
        assert!(admission.requires_repair_first());
        assert!(!admission.is_admitted());
        assert_eq!(
            admission_summary.decision_repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            admission_summary.blocked_reasons,
            admission.blocked_reasons.len()
        );
        assert_eq!(admission_record.appended_summary, admission_summary);
        assert!(admission.blocked_reasons.iter().any(|reason| {
            reason
                == "norion_core:recursive_schedule_history:recursive_schedule_blocked_records=1>0"
        }));
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("trend_gate_history:"))
        );
        assert!(!monitor_record.is_admitted());
        assert!(monitor_record.requires_repair_first());
        assert!(!continuation.is_admitted());
        assert!(continuation.requires_repair_first);
        assert!(resume_record.requires_repair_first());
        assert!(!resume_record.is_admitted());
        assert!(!downstream_handoff.is_admitted());
        assert!(downstream_handoff.requires_repair_first());
        assert_eq!(
            downstream_summary.repair_tasks,
            downstream_handoff.gate_decision.repair_tasks.len()
        );
        assert_eq!(
            downstream_summary.blocked_reasons,
            downstream_handoff.gate_decision.blocked_reasons.len()
        );
        assert!(
            downstream_handoff
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_history:"))
        );
        assert!(
            downstream_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
    }

    #[test]
    fn adapter_gate_projects_stable_reflection_loop_history_as_memory_boundary() {
        let record = stable_reflection_record();
        let gate = AgentAdapterBoundaryGate::from_reflection_loop_history_gate(&record);
        let snapshot = AgentAdapterBoundarySnapshot::from_reflection_loop_history_gate(
            &business_queue(),
            &record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_reflection_loop_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(record.can_promote_memory_note());
        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionMemory);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(boundary_record.allows_service_advance());
        assert!(boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_gate_projects_incomplete_reflection_loop_history_as_watch_boundary() {
        let record = incomplete_reflection_record();
        let gate = AgentAdapterBoundaryGate::from_reflection_loop_history_gate(&record);
        let snapshot = AgentAdapterBoundarySnapshot::from_reflection_loop_history_gate(
            &business_queue(),
            &record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_reflection_loop_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(record.allows_service_advance());
        assert!(!record.can_promote_memory_note());
        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionMemory);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(!snapshot.requires_repair_first());
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(boundary_record.allows_service_advance());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
        assert!(
            snapshot.blocked_reasons.iter().any(|reason| {
                reason == "norion_memory:reflection_incomplete_next_stage=critique"
            })
        );
    }

    #[test]
    fn adapter_gate_projects_dirty_reflection_loop_history_as_closed_memory_boundary() {
        let record = stalled_reflection_record();
        let gate = AgentAdapterBoundaryGate::from_reflection_loop_history_gate(&record);
        let snapshot = AgentAdapterBoundarySnapshot::from_reflection_loop_history_gate(
            &business_queue(),
            &record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_reflection_loop_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(record.requires_repair_first());
        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionMemory);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(!boundary_record.allows_service_advance());
        assert!(boundary_record.requires_repair_first());
        assert!(!boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
        assert!(snapshot.blocked_reasons.iter().any(|reason| {
            reason
                == "norion_memory:reflection_loop_history:reflection_loop_stalled_stage_records=1>0"
        }));
    }

    #[test]
    fn adapter_boundary_projects_budget_rejection_run_progress_as_core_repair() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Reviewer, AgentBudget::new(4, 1, 1)),
        );
        let oversized = AgentTask::new(
            "oversized-review",
            AgentRole::Reviewer,
            "review request larger than the isolated budget",
            AgentBudget::new(8, 1, 1),
        );
        let next_queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "ordinary-follow-up",
            AgentRole::Planner,
            "continue only after budget repair closes the run gate",
            AgentBudget::new(4, 1, 1),
        )]);

        let dispatch = planner.plan_with_policy(vec![oversized], &BudgetPolicy::strict());
        let dispatch_gate = dispatch.gate();
        let ledger = AgentRunLedger::new(dispatch);
        let progress = ledger.progress();
        let snapshot = AgentAdapterBoundarySnapshot::from_dispatch_and_run_progress(
            &next_queue,
            &dispatch_gate,
            &progress,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_snapshot_boundary_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                snapshot.clone(),
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record.clone(),
            &next_queue,
        );
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let handoff_summary = handoff.summary();
        let gate = snapshot
            .gates
            .iter()
            .find(|gate| gate.owner == AgentAdapterBoundaryOwner::NorionCore)
            .expect("core boundary gate should be present");

        assert!(progress.empty_dispatch);
        assert_eq!(progress.dispatch_rejections, 1);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(snapshot.next_queue_task_ids, vec!["ordinary-follow-up"]);
        assert!(snapshot.blocked_reasons.iter().any(|reason| {
            reason.contains("norion_core:dispatch_rejection task=oversized-review")
        }));
        assert!(
            snapshot
                .blocked_reasons
                .iter()
                .any(|reason| reason == "norion_core:run_progress_empty_dispatch")
        );
        assert!(
            snapshot
                .blocked_reasons
                .iter()
                .any(|reason| reason == "norion_core:run_progress_dispatch_rejections=1")
        );
        assert!(boundary_record.requires_repair_first());
        assert_eq!(
            boundary_record.summary().blocked_reasons,
            snapshot.blocked_reasons.len()
        );
        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_execute_service_commands());
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert_eq!(
            &handoff.blocked_reasons[..snapshot.blocked_reasons.len()],
            snapshot.blocked_reasons.as_slice()
        );
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "history:adapter_boundary_repair_records=1>0")
        );
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "history:adapter_boundary_repair_first_records=1>0")
        );
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(
            handoff_summary.blocked_reasons,
            handoff.blocked_reasons.len()
        );
        assert_eq!(handoff_summary.repair_tasks, handoff.blocked_reasons.len());
        assert_eq!(
            handoff_summary
                .next_queue_task_ids
                .last()
                .map(String::as_str),
            Some("ordinary-follow-up")
        );
        assert!(
            handoff_summary.next_queue_task_ids[..handoff_summary.repair_tasks]
                .iter()
                .all(|task_id| task_id.starts_with("adapter-boundary-repair-"))
        );
        assert_eq!(handoff_record.appended_summary, handoff_summary);
        assert_eq!(
            handoff_record.dashboard.blocked_reasons,
            handoff.blocked_reasons.len()
        );

        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let trend_summary = trend_decision.summary();
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let admission_record = AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
            .record_admission_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                &admission,
                AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
            );
        let admission_summary = admission.summary();

        assert!(trend_decision.requires_repair_first);
        assert!(!trend_decision.is_admitted());
        assert!(!trend_decision.can_submit_memory_note);
        assert!(!trend_decision.can_promote_adaptive_state);
        assert_eq!(
            trend_summary.repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            trend_summary.blocked_reasons,
            trend_decision.blocked_reasons.len()
        );
        assert!(trend_summary.blocked_reasons >= handoff_summary.blocked_reasons);
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "norion_core:run_progress_dispatch_rejections=1" })
        );
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("handoff_history:"))
        );
        assert!(
            trend_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| { task_id == "ordinary-follow-up" })
        );

        assert!(admission.requires_repair_first());
        assert!(!admission.is_admitted());
        assert!(!admission.can_submit_memory_note);
        assert!(!admission.can_promote_adaptive_state);
        assert_eq!(
            admission_summary.decision_repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            admission_summary.history_repair_tasks,
            admission.history_repair_tasks.len()
        );
        assert_eq!(
            admission_summary.blocked_reasons,
            admission.blocked_reasons.len()
        );
        assert!(admission_summary.blocked_reasons >= trend_summary.blocked_reasons);
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "norion_core:run_progress_dispatch_rejections=1" })
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("trend_gate_history:"))
        );
        assert_eq!(admission_record.appended_summary, admission_summary);
        assert_eq!(
            admission_record.dashboard.decision_repair_task_count,
            admission_summary.decision_repair_tasks
        );
        assert_eq!(
            admission_record.dashboard.history_repair_task_count,
            admission_summary.history_repair_tasks
        );
        assert_eq!(
            admission_record.dashboard.blocked_reasons,
            admission_summary.blocked_reasons
        );

        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = AgentAdapterBoundaryHandoffTrendAdmissionContinuationPlanner::new()
            .plan(&monitor_record, trend_policy, admission_policy);
        let resume_plan =
            AgentAdapterBoundaryHandoffTrendAdmissionResumePlanner::new().plan(&continuation);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_summary = resume_record.summary();
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let downstream_summary = downstream_handoff.summary();
        let downstream_packet =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
                .record_and_gate(
                    downstream_handoff.clone(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );

        assert!(!monitor_record.is_admitted());
        assert!(monitor_record.requires_repair_first());
        assert!(!continuation.is_admitted());
        assert!(continuation.requires_repair_first);
        assert!(!continuation.can_submit_memory_note);
        assert!(!continuation.can_promote_adaptive_state);
        assert_eq!(
            resume_plan.prior_queue.task_ids(),
            continuation.next_queue.task_ids()
        );
        assert!(resume_record.requires_repair_first());
        assert!(!resume_record.is_admitted());
        assert_eq!(
            resume_summary.next_queue_tasks,
            resume_record.next_queue().len()
        );
        assert!(
            resume_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "ordinary-follow-up")
        );
        assert_eq!(
            resume_history_record.appended_summary.next_queue_task_ids,
            resume_summary.next_queue_task_ids
        );
        assert!(resume_history_record.requires_repair_first());
        assert_eq!(
            downstream_summary.repair_tasks,
            downstream_handoff.gate_decision.repair_tasks.len()
        );
        assert_eq!(
            downstream_summary.blocked_reasons,
            downstream_handoff.gate_decision.blocked_reasons.len()
        );
        assert!(downstream_summary.repair_tasks > 0);
        assert!(downstream_summary.blocked_reasons > 0);
        assert!(!downstream_handoff.is_admitted());
        assert!(downstream_handoff.requires_repair_first());
        assert!(!downstream_handoff.can_submit_memory_note());
        assert!(!downstream_handoff.can_promote_adaptive_state());
        assert!(
            downstream_handoff
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_history:"))
        );
        assert!(
            downstream_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "ordinary-follow-up")
        );
        assert!(!downstream_packet.is_admitted());
        assert!(!downstream_packet.allows_service_advance());
        assert!(downstream_packet.requires_repair_first());
        assert!(!downstream_packet.can_submit_memory_note());
        assert!(!downstream_packet.can_promote_adaptive_state());
        assert_eq!(
            downstream_packet.history_record.appended_summary,
            downstream_summary
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.repair_task_count,
            downstream_summary.repair_tasks
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.blocked_reasons,
            downstream_summary.blocked_reasons
        );
        assert!(
            downstream_packet
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_monitor_handoff_history:"))
        );
        assert!(
            downstream_packet.gate_decision.repair_tasks.len() >= downstream_summary.repair_tasks
        );
        assert!(
            downstream_packet
                .next_queue()
                .task_ids()
                .iter()
                .any(|task_id| task_id == "ordinary-follow-up")
        );
    }

    #[test]
    fn adapter_boundary_projects_closed_run_ledger_admission_as_core_repair() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Reviewer, AgentBudget::new(4, 1, 1)),
        );
        let oversized = AgentTask::new(
            "oversized-review",
            AgentRole::Reviewer,
            "review request larger than the isolated budget",
            AgentBudget::new(8, 1, 1),
        );
        let next_queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "ordinary-follow-up",
            AgentRole::Planner,
            "continue only after ledger admission repair closes the run gate",
            AgentBudget::new(4, 1, 1),
        )]);

        let dispatch = planner.plan_with_policy(vec![oversized], &BudgetPolicy::strict());
        let dispatch_gate = dispatch.gate();
        let admission = AgentRunLedger::admission(&dispatch_gate);
        let snapshot = AgentAdapterBoundarySnapshot::from_dispatch_and_run_ledger_admission(
            &next_queue,
            &dispatch_gate,
            &admission,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_dispatch_and_run_ledger_admission_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &next_queue,
                &dispatch_gate,
                &admission,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record.clone(),
            &next_queue,
        );
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let handoff_summary = handoff.summary();
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let trend_summary = trend_decision.summary();
        let trend_admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let trend_admission_record =
            AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
                .record_admission_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                    &trend_admission,
                    AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
                );
        let trend_admission_summary = trend_admission.summary();
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = monitor_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let downstream_summary = downstream_handoff.summary();
        let downstream_packet =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
                .record_and_gate(
                    downstream_handoff.clone(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );
        let gate = snapshot
            .gates
            .iter()
            .find(|gate| gate.owner == AgentAdapterBoundaryOwner::NorionCore)
            .expect("core boundary gate should be present");

        assert!(admission.requires_repair_first);
        assert!(!admission.can_build_ledger);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(boundary_record.requires_repair_first());
        assert!(!boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
        assert!(snapshot.blocked_reasons.iter().any(|reason| {
            reason
                == "norion_core:run_ledger_admission:dispatch_rejection task=oversized-review role=reviewer reason=insufficient budget requested=tokens:8 steps:1 messages:1 remaining=tokens:4 steps:1 messages:1"
        }));
        assert_eq!(snapshot.next_queue_task_ids, vec!["ordinary-follow-up"]);
        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_execute_service_commands());
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(handoff_summary.repair_tasks, handoff.repair_tasks.len());
        assert_eq!(
            handoff_summary.blocked_reasons,
            handoff.blocked_reasons.len()
        );
        assert_eq!(handoff_record.appended_summary, handoff_summary);
        assert_eq!(
            handoff_record.dashboard.repair_task_count,
            handoff_summary.repair_tasks
        );
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "norion_core:run_ledger_admission:dispatch_rejection task=oversized-review role=reviewer reason=insufficient budget requested=tokens:8 steps:1 messages:1 remaining=tokens:4 steps:1 messages:1")
        );
        assert!(trend_decision.requires_repair_first);
        assert!(!trend_decision.is_admitted());
        assert!(!trend_decision.can_submit_memory_note);
        assert!(!trend_decision.can_promote_adaptive_state);
        assert_eq!(
            trend_summary.repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            trend_summary.blocked_reasons,
            trend_decision.blocked_reasons.len()
        );
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("handoff_history:"))
        );
        assert!(trend_admission.requires_repair_first());
        assert!(!trend_admission.is_admitted());
        assert!(!trend_admission.can_submit_memory_note);
        assert!(!trend_admission.can_promote_adaptive_state);
        assert_eq!(
            trend_admission_summary.decision_repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            trend_admission_summary.blocked_reasons,
            trend_admission.blocked_reasons.len()
        );
        assert_eq!(
            trend_admission_record.appended_summary,
            trend_admission_summary
        );
        assert_eq!(
            trend_admission_record.dashboard.blocked_reasons,
            trend_admission_summary.blocked_reasons
        );
        assert!(
            trend_admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("trend_gate_history:"))
        );
        assert!(!monitor_record.is_admitted());
        assert!(monitor_record.requires_repair_first());
        assert!(!continuation.is_admitted());
        assert!(continuation.requires_repair_first);
        assert!(!continuation.can_submit_memory_note);
        assert!(!continuation.can_promote_adaptive_state);
        assert!(resume_record.requires_repair_first());
        assert!(!resume_record.is_admitted());
        assert!(resume_history_record.requires_repair_first());
        assert!(!downstream_handoff.is_admitted());
        assert!(downstream_handoff.requires_repair_first());
        assert!(!downstream_handoff.can_submit_memory_note());
        assert!(!downstream_handoff.can_promote_adaptive_state());
        assert_eq!(
            downstream_summary.repair_tasks,
            downstream_handoff.gate_decision.repair_tasks.len()
        );
        assert_eq!(
            downstream_summary.blocked_reasons,
            downstream_handoff.gate_decision.blocked_reasons.len()
        );
        assert!(
            downstream_handoff
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_history:"))
        );
        assert!(
            downstream_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "ordinary-follow-up")
        );
        assert!(!downstream_packet.is_admitted());
        assert!(!downstream_packet.allows_service_advance());
        assert!(downstream_packet.requires_repair_first());
        assert!(!downstream_packet.can_submit_memory_note());
        assert!(!downstream_packet.can_promote_adaptive_state());
        assert_eq!(
            downstream_packet.history_record.appended_summary,
            downstream_summary
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.repair_task_count,
            downstream_summary.repair_tasks
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.blocked_reasons,
            downstream_summary.blocked_reasons
        );
        assert!(
            downstream_packet
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_monitor_handoff_history:"))
        );
        assert!(
            downstream_packet.gate_decision.repair_tasks.len() >= downstream_summary.repair_tasks
        );
        assert!(
            downstream_packet
                .next_queue()
                .task_ids()
                .iter()
                .any(|task_id| task_id == "ordinary-follow-up")
        );
    }

    #[test]
    fn adapter_boundary_projects_clean_run_ledger_admission_as_open_core_boundary() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Reviewer, AgentBudget::new(12, 2, 2)),
        );
        let review = AgentTask::new(
            "review-ready",
            AgentRole::Reviewer,
            "review request within the isolated budget",
            AgentBudget::new(8, 1, 1),
        );
        let next_queue = AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "ordinary-follow-up",
            AgentRole::Planner,
            "continue after ledger admission opens the run gate",
            AgentBudget::new(4, 1, 1),
        )]);

        let dispatch = planner.plan_with_policy(vec![review], &BudgetPolicy::strict());
        let dispatch_gate = dispatch.gate();
        let admission = AgentRunLedger::admission(&dispatch_gate);
        let ledger = AgentRunLedger::try_from_dispatch(dispatch)
            .expect("clean dispatch should build ledger");
        let snapshot = AgentAdapterBoundarySnapshot::from_dispatch_and_run_ledger_admission(
            &next_queue,
            &dispatch_gate,
            &admission,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_dispatch_and_run_ledger_admission_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &next_queue,
                &dispatch_gate,
                &admission,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record.clone(),
            &next_queue,
        );
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let handoff_summary = handoff.summary();
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let trend_summary = trend_decision.summary();
        let trend_admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let trend_admission_record =
            AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
                .record_admission_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                    &trend_admission,
                    AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
                );
        let trend_admission_summary = trend_admission.summary();
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = monitor_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let downstream_summary = downstream_handoff.summary();
        let downstream_packet =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
                .record_and_gate(
                    downstream_handoff.clone(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );
        let gate = snapshot
            .gates
            .iter()
            .find(|gate| gate.owner == AgentAdapterBoundaryOwner::NorionCore)
            .expect("core boundary gate should be present");

        assert!(admission.is_admitted());
        assert!(admission.can_build_ledger);
        assert_eq!(ledger.dispatch().assignments.len(), 1);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert!(!snapshot.requires_repair_first());
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
        assert!(snapshot.blocked_reasons.is_empty());
        assert!(boundary_record.allows_service_advance());
        assert!(!boundary_record.requires_repair_first());
        assert!(boundary_record.can_execute_service_commands());
        assert!(boundary_record.can_submit_memory_note());
        assert!(boundary_record.can_promote_adaptive_state());
        assert_eq!(snapshot.next_queue_task_ids, vec!["ordinary-follow-up"]);
        assert!(handoff.is_admitted());
        assert!(!handoff.requires_repair_first);
        assert!(handoff.can_execute_service_commands());
        assert!(handoff.can_submit_memory_note());
        assert!(handoff.can_promote_adaptive_state());
        assert!(handoff.repair_tasks.is_empty());
        assert!(handoff.blocked_reasons.is_empty());
        assert_eq!(handoff_summary.repair_tasks, 0);
        assert_eq!(handoff_summary.blocked_reasons, 0);
        assert_eq!(handoff_record.appended_summary, handoff_summary);
        assert_eq!(handoff_record.dashboard.repair_task_count, 0);
        assert!(trend_decision.is_admitted());
        assert!(!trend_decision.requires_repair_first);
        assert!(trend_decision.can_submit_memory_note);
        assert!(trend_decision.can_promote_adaptive_state);
        assert!(trend_decision.repair_tasks.is_empty());
        assert!(trend_decision.blocked_reasons.is_empty());
        assert_eq!(trend_summary.repair_tasks, 0);
        assert_eq!(trend_summary.blocked_reasons, 0);
        assert!(trend_admission.is_admitted());
        assert!(!trend_admission.requires_repair_first());
        assert!(trend_admission.can_submit_memory_note);
        assert!(trend_admission.can_promote_adaptive_state);
        assert!(trend_admission.blocked_reasons.is_empty());
        assert_eq!(trend_admission_summary.decision_repair_tasks, 0);
        assert_eq!(trend_admission_summary.blocked_reasons, 0);
        assert_eq!(
            trend_admission_record.appended_summary,
            trend_admission_summary
        );
        assert_eq!(trend_admission_record.dashboard.blocked_reasons, 0);
        assert!(monitor_record.is_admitted());
        assert!(!monitor_record.requires_repair_first());
        assert!(continuation.is_admitted());
        assert!(!continuation.requires_repair_first);
        assert!(continuation.can_submit_memory_note);
        assert!(continuation.can_promote_adaptive_state);
        assert!(resume_record.is_admitted());
        assert!(!resume_record.requires_repair_first());
        assert!(!resume_history_record.requires_repair_first());
        assert!(downstream_handoff.is_admitted());
        assert!(!downstream_handoff.requires_repair_first());
        assert!(downstream_handoff.can_submit_memory_note());
        assert!(downstream_handoff.can_promote_adaptive_state());
        assert_eq!(downstream_summary.repair_tasks, 0);
        assert_eq!(downstream_summary.blocked_reasons, 0);
        assert!(
            downstream_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "ordinary-follow-up")
        );
        assert!(downstream_packet.is_admitted());
        assert!(downstream_packet.allows_service_advance());
        assert!(!downstream_packet.requires_repair_first());
        assert!(downstream_packet.can_submit_memory_note());
        assert!(downstream_packet.can_promote_adaptive_state());
        assert_eq!(
            downstream_packet.history_record.appended_summary,
            downstream_summary
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.repair_task_count,
            0
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.blocked_reasons,
            0
        );
        assert!(downstream_packet.gate_decision.repair_tasks.is_empty());
        assert!(downstream_packet.gate_decision.blocked_reasons.is_empty());
        assert_eq!(
            downstream_packet.next_queue().task_ids(),
            vec!["ordinary-follow-up"]
        );
    }

    #[test]
    fn adapter_boundary_projects_cycle_report_run_ledger_admission_as_core_repair() {
        let report = cycle_report_with_run_ledger_admission(AgentRunLedgerAdmission {
            can_build_ledger: false,
            can_admit_side_effects: false,
            can_submit_memory_note: false,
            can_promote_adaptive_state: false,
            requires_repair_first: true,
            reasons: vec![
                "dispatch_rejection task=oversized-review role=reviewer reason=insufficient budget requested=tokens:8 steps:1 messages:1 remaining=tokens:4 steps:1 messages:1"
                    .to_owned(),
            ],
            telemetry: Vec::new(),
        });
        let snapshot = AgentAdapterBoundarySnapshot::from_cycle_report(&business_queue(), &report);
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_cycle_report_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &report,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record.clone(),
            &business_queue(),
        );
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let handoff_summary = handoff.summary();
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let trend_summary = trend_decision.summary();
        let trend_admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let trend_admission_record =
            AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
                .record_admission_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                    &trend_admission,
                    AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
                );
        let trend_admission_summary = trend_admission.summary();
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = monitor_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let downstream_summary = downstream_handoff.summary();
        let downstream_packet =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
                .record_and_gate(
                    downstream_handoff.clone(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );
        let gate = snapshot
            .gates
            .iter()
            .find(|gate| gate.owner == AgentAdapterBoundaryOwner::NorionCore)
            .expect("core boundary gate should be present");

        assert!(report.run_ledger_admission.requires_repair_first);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(boundary_record.requires_repair_first());
        assert!(snapshot.blocked_reasons.iter().any(|reason| {
            reason
                == "norion_core:run_ledger_admission:dispatch_rejection task=oversized-review role=reviewer reason=insufficient budget requested=tokens:8 steps:1 messages:1 remaining=tokens:4 steps:1 messages:1"
        }));
        assert!(!boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_execute_service_commands());
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(handoff_summary.repair_tasks, handoff.repair_tasks.len());
        assert_eq!(
            handoff_summary.blocked_reasons,
            handoff.blocked_reasons.len()
        );
        assert_eq!(handoff_record.appended_summary, handoff_summary);
        assert_eq!(
            handoff_record.dashboard.repair_task_count,
            handoff_summary.repair_tasks
        );
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "norion_core:run_ledger_admission:dispatch_rejection task=oversized-review role=reviewer reason=insufficient budget requested=tokens:8 steps:1 messages:1 remaining=tokens:4 steps:1 messages:1")
        );
        assert!(trend_decision.requires_repair_first);
        assert!(!trend_decision.is_admitted());
        assert!(!trend_decision.can_submit_memory_note);
        assert!(!trend_decision.can_promote_adaptive_state);
        assert_eq!(
            trend_summary.repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            trend_summary.blocked_reasons,
            trend_decision.blocked_reasons.len()
        );
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("handoff_history:"))
        );
        assert!(trend_admission.requires_repair_first());
        assert!(!trend_admission.is_admitted());
        assert!(!trend_admission.can_submit_memory_note);
        assert!(!trend_admission.can_promote_adaptive_state);
        assert_eq!(
            trend_admission_summary.decision_repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            trend_admission_summary.blocked_reasons,
            trend_admission.blocked_reasons.len()
        );
        assert_eq!(
            trend_admission_record.appended_summary,
            trend_admission_summary
        );
        assert_eq!(
            trend_admission_record.dashboard.blocked_reasons,
            trend_admission_summary.blocked_reasons
        );
        assert!(
            trend_admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("trend_gate_history:"))
        );
        assert!(!monitor_record.is_admitted());
        assert!(monitor_record.requires_repair_first());
        assert!(!continuation.is_admitted());
        assert!(continuation.requires_repair_first);
        assert!(!continuation.can_submit_memory_note);
        assert!(!continuation.can_promote_adaptive_state);
        assert!(resume_record.requires_repair_first());
        assert!(!resume_record.is_admitted());
        assert!(resume_history_record.requires_repair_first());
        assert!(!downstream_handoff.is_admitted());
        assert!(downstream_handoff.requires_repair_first());
        assert!(!downstream_handoff.can_submit_memory_note());
        assert!(!downstream_handoff.can_promote_adaptive_state());
        assert_eq!(
            downstream_summary.repair_tasks,
            downstream_handoff.gate_decision.repair_tasks.len()
        );
        assert_eq!(
            downstream_summary.blocked_reasons,
            downstream_handoff.gate_decision.blocked_reasons.len()
        );
        assert!(
            downstream_handoff
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_history:"))
        );
        assert!(
            downstream_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(!downstream_packet.is_admitted());
        assert!(!downstream_packet.allows_service_advance());
        assert!(downstream_packet.requires_repair_first());
        assert!(!downstream_packet.can_submit_memory_note());
        assert!(!downstream_packet.can_promote_adaptive_state());
        assert_eq!(
            downstream_packet.history_record.appended_summary,
            downstream_summary
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.repair_task_count,
            downstream_summary.repair_tasks
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.blocked_reasons,
            downstream_summary.blocked_reasons
        );
        assert!(
            downstream_packet
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_monitor_handoff_history:"))
        );
        assert!(
            downstream_packet.gate_decision.repair_tasks.len() >= downstream_summary.repair_tasks
        );
        assert!(
            downstream_packet
                .next_queue()
                .task_ids()
                .iter()
                .any(|task_id| task_id == "business-task")
        );
    }

    #[test]
    fn adapter_boundary_projects_clean_cycle_report_run_ledger_admission_as_open_core_boundary() {
        let report = cycle_report_with_run_ledger_admission(AgentRunLedgerAdmission {
            can_build_ledger: true,
            can_admit_side_effects: true,
            can_submit_memory_note: true,
            can_promote_adaptive_state: true,
            requires_repair_first: false,
            reasons: Vec::new(),
            telemetry: Vec::new(),
        });
        let snapshot = AgentAdapterBoundarySnapshot::from_cycle_report(&business_queue(), &report);
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_cycle_report_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &report,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record.clone(),
            &business_queue(),
        );
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let handoff_summary = handoff.summary();
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let trend_summary = trend_decision.summary();
        let trend_admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let trend_admission_record =
            AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
                .record_admission_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                    &trend_admission,
                    AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
                );
        let trend_admission_summary = trend_admission.summary();
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = monitor_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let downstream_summary = downstream_handoff.summary();
        let downstream_packet =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
                .record_and_gate(
                    downstream_handoff.clone(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );
        let gate = snapshot
            .gates
            .iter()
            .find(|gate| gate.owner == AgentAdapterBoundaryOwner::NorionCore)
            .expect("core boundary gate should be present");

        assert!(report.run_ledger_admission.is_admitted());
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert!(!snapshot.requires_repair_first());
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
        assert!(boundary_record.allows_service_advance());
        assert!(!boundary_record.requires_repair_first());
        assert!(snapshot.blocked_reasons.is_empty());
        assert!(boundary_record.can_execute_service_commands());
        assert!(boundary_record.can_submit_memory_note());
        assert!(boundary_record.can_promote_adaptive_state());
        assert!(handoff.is_admitted());
        assert!(!handoff.requires_repair_first);
        assert!(handoff.can_execute_service_commands());
        assert!(handoff.can_submit_memory_note());
        assert!(handoff.can_promote_adaptive_state());
        assert!(handoff.repair_tasks.is_empty());
        assert!(handoff.blocked_reasons.is_empty());
        assert_eq!(handoff_summary.repair_tasks, 0);
        assert_eq!(handoff_summary.blocked_reasons, 0);
        assert_eq!(handoff_record.appended_summary, handoff_summary);
        assert_eq!(handoff_record.dashboard.repair_task_count, 0);
        assert!(trend_decision.is_admitted());
        assert!(!trend_decision.requires_repair_first);
        assert!(trend_decision.can_submit_memory_note);
        assert!(trend_decision.can_promote_adaptive_state);
        assert!(trend_decision.repair_tasks.is_empty());
        assert!(trend_decision.blocked_reasons.is_empty());
        assert_eq!(trend_summary.repair_tasks, 0);
        assert_eq!(trend_summary.blocked_reasons, 0);
        assert!(trend_admission.is_admitted());
        assert!(!trend_admission.requires_repair_first());
        assert!(trend_admission.can_submit_memory_note);
        assert!(trend_admission.can_promote_adaptive_state);
        assert!(trend_admission.blocked_reasons.is_empty());
        assert_eq!(trend_admission_summary.decision_repair_tasks, 0);
        assert_eq!(trend_admission_summary.blocked_reasons, 0);
        assert_eq!(
            trend_admission_record.appended_summary,
            trend_admission_summary
        );
        assert_eq!(trend_admission_record.dashboard.blocked_reasons, 0);
        assert!(monitor_record.is_admitted());
        assert!(!monitor_record.requires_repair_first());
        assert!(continuation.is_admitted());
        assert!(!continuation.requires_repair_first);
        assert!(continuation.can_submit_memory_note);
        assert!(continuation.can_promote_adaptive_state);
        assert!(resume_record.is_admitted());
        assert!(!resume_record.requires_repair_first());
        assert!(!resume_history_record.requires_repair_first());
        assert!(downstream_handoff.is_admitted());
        assert!(!downstream_handoff.requires_repair_first());
        assert!(downstream_handoff.can_submit_memory_note());
        assert!(downstream_handoff.can_promote_adaptive_state());
        assert_eq!(downstream_summary.repair_tasks, 0);
        assert_eq!(downstream_summary.blocked_reasons, 0);
        assert!(
            downstream_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(downstream_packet.is_admitted());
        assert!(downstream_packet.allows_service_advance());
        assert!(!downstream_packet.requires_repair_first());
        assert!(downstream_packet.can_submit_memory_note());
        assert!(downstream_packet.can_promote_adaptive_state());
        assert_eq!(
            downstream_packet.history_record.appended_summary,
            downstream_summary
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.repair_task_count,
            0
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.blocked_reasons,
            0
        );
        assert!(downstream_packet.gate_decision.repair_tasks.is_empty());
        assert!(downstream_packet.gate_decision.blocked_reasons.is_empty());
        assert_eq!(
            downstream_packet.next_queue().task_ids(),
            vec!["business-task"]
        );
    }

    #[test]
    fn adapter_boundary_final_packet_projects_clean_run_ledger_admission_entries_symmetrically() {
        let next_queue = business_queue();
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Reviewer, AgentBudget::new(12, 2, 2)),
        );
        let review = AgentTask::new(
            "review-ready",
            AgentRole::Reviewer,
            "review request within the isolated budget",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = planner.plan_with_policy(vec![review], &BudgetPolicy::strict());
        let dispatch_gate = dispatch.gate();
        let direct_admission = AgentRunLedger::admission(&dispatch_gate);
        let direct_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_dispatch_and_run_ledger_admission_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &next_queue,
                &dispatch_gate,
                &direct_admission,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let cycle_report = cycle_report_with_run_ledger_admission(AgentRunLedgerAdmission {
            can_build_ledger: true,
            can_admit_side_effects: true,
            can_submit_memory_note: true,
            can_promote_adaptive_state: true,
            requires_repair_first: false,
            reasons: Vec::new(),
            telemetry: Vec::new(),
        });
        let cycle_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_cycle_report_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &next_queue,
                &cycle_report,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        let direct_packet = final_handoff_packet_from_boundary_record(direct_record, &next_queue);
        let cycle_packet = final_handoff_packet_from_boundary_record(cycle_record, &next_queue);

        assert!(direct_admission.is_admitted());
        assert!(cycle_report.run_ledger_admission.is_admitted());
        for packet in [&direct_packet, &cycle_packet] {
            assert!(packet.is_admitted());
            assert!(packet.allows_service_advance());
            assert!(!packet.requires_repair_first());
            assert!(packet.can_submit_memory_note());
            assert!(packet.can_promote_adaptive_state());
            assert_eq!(packet.history_record.dashboard.repair_task_count, 0);
            assert_eq!(packet.history_record.dashboard.blocked_reasons, 0);
            assert!(packet.gate_decision.repair_tasks.is_empty());
            assert!(packet.gate_decision.blocked_reasons.is_empty());
            assert_eq!(packet.next_queue().task_ids(), vec!["business-task"]);
        }
        assert_eq!(direct_packet.gate_decision, cycle_packet.gate_decision);
        assert_eq!(
            direct_packet.history_record.appended_summary,
            cycle_packet.history_record.appended_summary
        );
    }

    #[test]
    fn adapter_boundary_final_packet_projects_repair_run_ledger_admission_entries_symmetrically() {
        let next_queue = business_queue();
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
        let direct_admission = AgentRunLedger::admission(&dispatch_gate);
        let direct_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_dispatch_and_run_ledger_admission_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &next_queue,
                &dispatch_gate,
                &direct_admission,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let cycle_report = cycle_report_with_run_ledger_admission(AgentRunLedgerAdmission {
            can_build_ledger: false,
            can_admit_side_effects: false,
            can_submit_memory_note: false,
            can_promote_adaptive_state: false,
            requires_repair_first: true,
            reasons: vec![
                "dispatch_rejection task=oversized-review role=reviewer reason=insufficient budget requested=tokens:8 steps:1 messages:1 remaining=tokens:4 steps:1 messages:1"
                    .to_owned(),
            ],
            telemetry: Vec::new(),
        });
        let cycle_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_cycle_report_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &next_queue,
                &cycle_report,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        let direct_packet = final_handoff_packet_from_boundary_record(direct_record, &next_queue);
        let cycle_packet = final_handoff_packet_from_boundary_record(cycle_record, &next_queue);

        assert!(direct_admission.requires_repair_first);
        assert!(cycle_report.run_ledger_admission.requires_repair_first);
        for packet in [&direct_packet, &cycle_packet] {
            assert!(!packet.is_admitted());
            assert!(!packet.allows_service_advance());
            assert!(packet.requires_repair_first());
            assert!(!packet.can_submit_memory_note());
            assert!(!packet.can_promote_adaptive_state());
            assert!(packet.history_record.dashboard.repair_task_count > 0);
            assert!(packet.history_record.dashboard.blocked_reasons > 0);
            assert!(!packet.gate_decision.repair_tasks.is_empty());
            let final_repair_task_ids = packet
                .gate_decision
                .repair_tasks
                .iter()
                .map(|task| task.id.clone())
                .collect::<Vec<_>>();
            assert!(final_repair_task_ids.iter().all(|task_id| task_id
                .starts_with("adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-")));
            let business_task = packet
                .next_queue()
                .tasks()
                .into_iter()
                .find(|task| task.id == "business-task")
                .expect("business task should remain behind final adapter repair");
            let queued_repair_task_ids = business_task.dependencies.clone();
            let schedule = RecursiveAgentScheduler::new(16).plan(packet.next_queue().tasks());
            assert!(!queued_repair_task_ids.is_empty());
            assert!(
                final_repair_task_ids
                    .iter()
                    .all(|task_id| queued_repair_task_ids.contains(task_id))
            );
            assert!(
                queued_repair_task_ids
                    .iter()
                    .all(|task_id| task_id.starts_with("adapter-boundary-"))
            );
            let business_wave_index = schedule
                .waves
                .iter()
                .position(|wave| {
                    wave.task_ids
                        .iter()
                        .any(|task_id| task_id == "business-task")
                })
                .expect("business task should remain scheduled after adapter repair");
            assert!(business_wave_index > 0);
            for repair_task_id in queued_repair_task_ids {
                let repair_wave_index = schedule
                    .waves
                    .iter()
                    .position(|wave| {
                        wave.task_ids
                            .iter()
                            .any(|task_id| task_id == &repair_task_id)
                    })
                    .expect("queued adapter repair task should stay schedulable");
                assert!(repair_wave_index < business_wave_index);
            }
            assert!(
                packet
                    .gate_decision
                    .blocked_reasons
                    .iter()
                    .any(|reason| reason.starts_with("resume_gate_monitor_handoff_history:"))
            );
            assert!(
                packet
                    .next_queue()
                    .task_ids()
                    .iter()
                    .any(|task_id| task_id == "business-task")
            );
        }
    }

    #[test]
    fn adapter_gate_projects_memory_submission_without_promoting_empty_notes() {
        let watch = MemorySubmissionGateDecision {
            summary: crate::memory::MemorySubmissionSummary {
                submitted_notes: 0,
                failed_notes: 0,
                blocked_reasons: 0,
                attempted_notes: 0,
                quality_reviewed_notes: 0,
                quality_admitted_notes: 0,
                quality_rejected_notes: 0,
                clean: true,
                port_attempted: false,
                telemetry: Vec::new(),
            },
            can_continue_loop: true,
            can_commit_submitted_notes: false,
            requires_repair_first: false,
            reasons: Vec::new(),
            telemetry: Vec::new(),
        };

        let gate = AgentAdapterBoundaryGate::from_memory_submission_gate(&watch);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionMemory);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert_eq!(gate.blocked_reasons, vec!["memory_submission_observe_only"]);
    }

    #[test]
    fn adapter_gate_projects_memory_promotion_gate_as_stable_memory_owner() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = clean_review_trend_gate();
        let memory_health = stable_memory_submission_health();
        let notes = vec![MemoryNote::new("agent_cycle", "remember clean handoff")];
        let decision =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        let gate = AgentAdapterBoundaryGate::from_memory_promotion_gate(&decision);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionMemory);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert!(gate.blocked_reasons.is_empty());
    }

    #[test]
    fn adapter_gate_projects_memory_promotion_watch_without_auto_submit() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = clean_review_trend_gate();
        let memory_health =
            MemorySubmissionSummaryHistory::new().health(MemorySubmissionHealthPolicy::default());
        let notes = vec![MemoryNote::new("agent_cycle", "remember clean handoff")];
        let decision =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        let gate = AgentAdapterBoundaryGate::from_memory_promotion_gate(&decision);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::NorionMemory);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert_eq!(
            gate.blocked_reasons,
            vec!["memory_promotion_submission_history:memory_submission_history_empty"]
        );
    }

    #[test]
    fn adapter_gate_projects_eval_rejections_as_repair_first() {
        let decision = AgentReportGateDecision {
            accepted: false,
            reasons: vec![AgentReportGateReason::new(
                "validation_evidence_missing",
                "true",
            )],
            follow_up_tasks: Vec::new(),
        };

        let gate = AgentAdapterBoundaryGate::from_report_gate(&decision);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::EvalReporting);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.memory_note_allowed);
        assert_eq!(
            gate.blocked_reasons,
            vec!["validation_evidence_missing=true"]
        );
    }

    #[test]
    fn adapter_boundary_projects_unresolved_conflict_report_as_repair_first() {
        let report = ConflictReport {
            conflicts: vec![AgentConflict {
                topic: "memory".to_owned(),
                message_ids: vec!["planner-memory".to_owned(), "reviewer-memory".to_owned()],
                roles: vec![AgentRole::Planner, AgentRole::Reviewer],
                summary: "memory note promotion conflict".to_owned(),
                resolved: false,
                resolution_hint: "repair conflict before promoting side effects".to_owned(),
            }],
            messages: vec![
                AgentMessage::new(
                    "planner-memory",
                    AgentRole::Planner,
                    AgentMessageKind::Decision,
                    "memory",
                    "promote the memory note",
                ),
                AgentMessage::new(
                    "reviewer-memory",
                    AgentRole::Reviewer,
                    AgentMessageKind::Risk,
                    "memory",
                    "hold the memory note until conflict repair",
                ),
            ],
        };
        let conflict_record = ConflictReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ConflictReportSummaryHistory::new(),
                &report,
                ConflictReportHealthPolicy::default(),
            );

        let snapshot = AgentAdapterBoundarySnapshot::from_conflict_report_history_gate(
            &business_queue(),
            &conflict_record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_conflict_report_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &conflict_record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record.clone(),
            &business_queue(),
        );
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let handoff_summary = handoff.summary();
        let repair_task_ids = handoff
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let business_task = handoff
            .next_queue
            .tasks()
            .into_iter()
            .find(|task| task.id == "business-task")
            .expect("business task should stay behind conflict repair");
        let schedule = RecursiveAgentScheduler::new(16).plan(handoff.next_queue.tasks());
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let trend_summary = trend_decision.summary();
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let admission_record = AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
            .record_admission_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                &admission,
                AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
            );
        let admission_summary = admission.summary();
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = monitor_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let downstream_summary = downstream_handoff.summary();
        let downstream_packet =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
                .record_and_gate(
                    downstream_handoff.clone(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );

        assert!(conflict_record.requires_repair_first());
        assert!(!conflict_record.allows_service_advance());
        assert!(!conflict_record.can_promote_side_effects());
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Repair);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(
            snapshot.blocked_reasons.iter().any(|reason| {
                reason == "eval_reporting:conflict_report_unresolved_conflicts=1"
            })
        );
        assert_eq!(
            boundary_record.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(!boundary_record.allows_service_advance());
        assert!(!boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_execute_service_commands());
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(handoff_summary.repair_tasks, repair_task_ids.len());
        assert_eq!(
            handoff_summary.blocked_reasons,
            handoff.blocked_reasons.len()
        );
        assert_eq!(handoff_record.appended_summary, handoff_summary);
        assert_eq!(
            handoff_record.dashboard.repair_task_count,
            handoff_summary.repair_tasks
        );
        assert_eq!(
            handoff_record.dashboard.blocked_reasons,
            handoff_summary.blocked_reasons
        );
        assert!(
            handoff.blocked_reasons.iter().any(|reason| {
                reason == "eval_reporting:conflict_report_unresolved_conflicts=1"
            })
        );
        assert!(
            repair_task_ids
                .iter()
                .all(|task_id| task_id.starts_with("adapter-boundary-repair-"))
        );
        assert_eq!(business_task.dependencies, repair_task_ids);
        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.waves[0].task_ids, repair_task_ids);
        assert_eq!(schedule.waves[1].task_ids, vec!["business-task"]);
        assert!(trend_decision.requires_repair_first);
        assert!(!trend_decision.is_admitted());
        assert!(!trend_decision.can_submit_memory_note);
        assert!(!trend_decision.can_promote_adaptive_state);
        assert_eq!(
            trend_summary.repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            trend_summary.blocked_reasons,
            trend_decision.blocked_reasons.len()
        );
        assert!(
            trend_decision.blocked_reasons.iter().any(|reason| {
                reason == "eval_reporting:conflict_report_unresolved_conflicts=1"
            })
        );
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("handoff_history:"))
        );
        assert!(
            trend_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(admission.requires_repair_first());
        assert!(!admission.is_admitted());
        assert!(!admission.can_submit_memory_note);
        assert!(!admission.can_promote_adaptive_state);
        assert_eq!(
            admission_summary.decision_repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            admission_summary.blocked_reasons,
            admission.blocked_reasons.len()
        );
        assert!(
            admission.blocked_reasons.iter().any(|reason| {
                reason == "eval_reporting:conflict_report_unresolved_conflicts=1"
            })
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("trend_gate_history:"))
        );
        assert_eq!(admission_record.appended_summary, admission_summary);
        assert_eq!(
            admission_record.dashboard.blocked_reasons,
            admission_summary.blocked_reasons
        );
        assert!(!monitor_record.is_admitted());
        assert!(monitor_record.requires_repair_first());
        assert!(!continuation.is_admitted());
        assert!(continuation.requires_repair_first);
        assert!(resume_record.requires_repair_first());
        assert!(!resume_record.is_admitted());
        assert!(!downstream_handoff.is_admitted());
        assert!(downstream_handoff.requires_repair_first());
        assert!(!downstream_handoff.can_submit_memory_note());
        assert!(!downstream_handoff.can_promote_adaptive_state());
        assert_eq!(
            downstream_summary.repair_tasks,
            downstream_handoff.gate_decision.repair_tasks.len()
        );
        assert_eq!(
            downstream_summary.blocked_reasons,
            downstream_handoff.gate_decision.blocked_reasons.len()
        );
        assert!(downstream_summary.repair_tasks > 0);
        assert!(downstream_summary.blocked_reasons > 0);
        assert!(
            downstream_handoff
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_history:"))
        );
        assert!(
            downstream_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(!downstream_packet.is_admitted());
        assert!(!downstream_packet.allows_service_advance());
        assert!(downstream_packet.requires_repair_first());
        assert!(!downstream_packet.can_submit_memory_note());
        assert!(!downstream_packet.can_promote_adaptive_state());
        assert_eq!(
            downstream_packet.history_record.appended_summary,
            downstream_summary
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.repair_task_count,
            downstream_summary.repair_tasks
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.blocked_reasons,
            downstream_summary.blocked_reasons
        );
        assert_eq!(
            downstream_packet.gate_decision.handoff_health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(
            downstream_packet.gate_decision.repair_tasks.len() >= downstream_summary.repair_tasks
        );
        assert!(
            downstream_packet
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_monitor_handoff_history:"))
        );
        assert!(
            downstream_packet
                .next_queue()
                .task_ids()
                .iter()
                .any(|task_id| task_id == "business-task")
        );
    }

    #[test]
    fn adapter_boundary_keeps_unresolved_conflict_from_bypassing_reflection_memory_note() {
        let report = ConflictReport {
            conflicts: vec![AgentConflict {
                topic: "memory".to_owned(),
                message_ids: vec!["planner-memory".to_owned(), "reviewer-memory".to_owned()],
                roles: vec![AgentRole::Planner, AgentRole::Reviewer],
                summary: "memory note promotion conflict".to_owned(),
                resolved: false,
                resolution_hint: "repair conflict before promoting side effects".to_owned(),
            }],
            messages: vec![
                AgentMessage::new(
                    "planner-memory",
                    AgentRole::Planner,
                    AgentMessageKind::Decision,
                    "memory",
                    "promote the memory note",
                ),
                AgentMessage::new(
                    "reviewer-memory",
                    AgentRole::Reviewer,
                    AgentMessageKind::Risk,
                    "memory",
                    "hold the memory note until conflict repair",
                ),
            ],
        };
        let conflict_record = ConflictReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ConflictReportSummaryHistory::new(),
                &report,
                ConflictReportHealthPolicy::default(),
            );
        let reflection_record = stable_reflection_record();
        let conflict_gate =
            AgentAdapterBoundaryGate::from_conflict_report_history_gate(&conflict_record);
        let reflection_gate =
            AgentAdapterBoundaryGate::from_reflection_loop_history_gate(&reflection_record);

        let snapshot = AgentAdapterBoundarySnapshot::from_gates(
            &business_queue(),
            vec![reflection_gate.clone(), conflict_gate.clone()],
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_snapshot_boundary_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                snapshot.clone(),
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record.clone(),
            &business_queue(),
        );
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let admission_summary = admission.summary();

        assert!(conflict_record.requires_repair_first());
        assert!(reflection_record.can_promote_memory_note());
        assert_eq!(conflict_gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!conflict_gate.memory_note_allowed);
        assert_eq!(reflection_gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(reflection_gate.memory_note_allowed);
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Repair);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(
            snapshot
                .gates
                .iter()
                .map(|gate| gate.owner)
                .collect::<Vec<_>>(),
            vec![
                AgentAdapterBoundaryOwner::NorionMemory,
                AgentAdapterBoundaryOwner::EvalReporting,
            ]
        );
        assert!(
            snapshot.blocked_reasons.iter().any(|reason| {
                reason == "eval_reporting:conflict_report_unresolved_conflicts=1"
            })
        );
        assert_eq!(
            boundary_record.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(boundary_record.requires_repair_first());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff.blocked_reasons.iter().any(|reason| {
                reason == "eval_reporting:conflict_report_unresolved_conflicts=1"
            })
        );
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(
            handoff_record.dashboard.repair_task_count,
            handoff.repair_tasks.len()
        );
        assert!(trend_decision.requires_repair_first);
        assert!(!trend_decision.is_admitted());
        assert!(!trend_decision.can_submit_memory_note);
        assert!(!trend_decision.can_promote_adaptive_state);
        assert!(
            trend_decision.blocked_reasons.iter().any(|reason| {
                reason == "eval_reporting:conflict_report_unresolved_conflicts=1"
            })
        );
        assert!(admission.requires_repair_first());
        assert!(!admission.is_admitted());
        assert!(!admission.can_submit_memory_note);
        assert!(!admission.can_promote_adaptive_state);
        assert_eq!(
            admission_summary.decision_repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert!(
            admission.blocked_reasons.iter().any(|reason| {
                reason == "eval_reporting:conflict_report_unresolved_conflicts=1"
            })
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("trend_gate_history:"))
        );
    }

    #[test]
    fn adapter_boundary_projects_aggregation_conflict_review_repair_as_closed_side_effects() {
        let review = AggregationConflictReviewer::new().review_messages(
            vec![
                AgentMessage::new(
                    "approve-a",
                    AgentRole::Researcher,
                    AgentMessageKind::Finding,
                    "memory",
                    "approve memory note promotion and proceed",
                ),
                AgentMessage::new(
                    "approve-b",
                    AgentRole::Researcher,
                    AgentMessageKind::Finding,
                    " memory ",
                    "approve   memory note promotion and proceed",
                ),
                AgentMessage::new(
                    "block",
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
        let review_record = AggregationConflictReviewSummaryHistoryRecorder::new()
            .record_review_with_health(
                AggregationConflictReviewSummaryHistory::new(),
                &review,
                AggregationConflictReviewHealthPolicy::default(),
            );
        let trend_gate = AggregationConflictReviewTrendGate::new().gate(&review, &review_record);

        let snapshot = AgentAdapterBoundarySnapshot::from_aggregation_conflict_review_trend_gate(
            &business_queue(),
            &trend_gate,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_aggregation_conflict_review_trend_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &trend_gate,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(trend_gate.requires_repair_first);
        assert!(!trend_gate.is_forwardable());
        assert!(!trend_gate.is_side_effect_safe());
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Repair);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(
            snapshot
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "eval_reporting:aggregation_duplicate_messages=1" })
        );
        assert!(snapshot.blocked_reasons.iter().any(|reason| {
            reason == "eval_reporting:conflict_report:conflict_report_unresolved_conflicts=1"
        }));
        assert_eq!(
            boundary_record.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(!boundary_record.allows_service_advance());
        assert!(!boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_boundary_projects_process_reward_repair_as_closed_side_effects() {
        let report = ProcessRewardReport {
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
            notes: vec!["penalize blocked loop".to_owned()],
            evolution_signals: Vec::new(),
        };
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &report,
                ProcessRewardReportHealthPolicy::default(),
            );

        let gate =
            AgentAdapterBoundaryGate::from_process_reward_report_history_gate(&reward_record);
        let snapshot = AgentAdapterBoundarySnapshot::from_process_reward_report_history_gate(
            &business_queue(),
            &reward_record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_process_reward_report_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &reward_record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record.clone(),
            &business_queue(),
        );
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let trend_summary = trend_decision.summary();
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let admission_summary = admission.summary();
        let admission_record = AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
            .record_admission_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                &admission,
                AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = monitor_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let downstream_summary = downstream_handoff.summary();

        assert!(reward_record.requires_repair_first());
        assert!(!reward_record.can_promote_evolution_signals());
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Repair);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(
            snapshot
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "eval_reporting:process_reward_report_action=penalize" })
        );
        assert_eq!(
            boundary_record.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(!boundary_record.allows_service_advance());
        assert!(!boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_execute_service_commands());
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "eval_reporting:process_reward_report_action=penalize" })
        );
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(handoff_record.appended_summary, handoff.summary());
        assert_eq!(
            handoff_record.dashboard.repair_task_count,
            handoff.repair_tasks.len()
        );
        assert!(trend_decision.requires_repair_first);
        assert!(!trend_decision.is_admitted());
        assert!(!trend_decision.can_submit_memory_note);
        assert!(!trend_decision.can_promote_adaptive_state);
        assert_eq!(
            trend_summary.repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            trend_summary.blocked_reasons,
            trend_decision.blocked_reasons.len()
        );
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "eval_reporting:process_reward_report_action=penalize" })
        );
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("handoff_history:"))
        );
        assert!(admission.requires_repair_first());
        assert!(!admission.is_admitted());
        assert!(!admission.can_submit_memory_note);
        assert!(!admission.can_promote_adaptive_state);
        assert_eq!(
            admission_summary.decision_repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            admission_summary.blocked_reasons,
            admission.blocked_reasons.len()
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "eval_reporting:process_reward_report_action=penalize" })
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("trend_gate_history:"))
        );
        assert_eq!(admission_record.appended_summary, admission_summary);
        assert_eq!(
            admission_record.dashboard.blocked_reasons,
            admission_summary.blocked_reasons
        );
        assert!(!monitor_record.is_admitted());
        assert!(monitor_record.requires_repair_first());
        assert!(!continuation.is_admitted());
        assert!(continuation.requires_repair_first);
        assert!(resume_record.requires_repair_first());
        assert!(!resume_record.is_admitted());
        assert!(!downstream_handoff.is_admitted());
        assert!(downstream_handoff.requires_repair_first());
        assert!(!downstream_handoff.can_submit_memory_note());
        assert!(!downstream_handoff.can_promote_adaptive_state());
        assert_eq!(
            downstream_summary.repair_tasks,
            downstream_handoff.gate_decision.repair_tasks.len()
        );
        assert_eq!(
            downstream_summary.blocked_reasons,
            downstream_handoff.gate_decision.blocked_reasons.len()
        );
        assert!(
            downstream_handoff
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_history:"))
        );
        assert!(
            downstream_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
    }

    #[test]
    fn adapter_boundary_projects_clean_evolution_admission_as_tool_and_adaptive_boundary() {
        let toolsmith_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));
        let toolsmith_record = ToolsmithPlanSummaryHistoryRecorder::new()
            .record_plan_with_health_gate(
                ToolsmithPlanSummaryHistory::new(),
                &toolsmith_plan,
                ToolsmithPlanHealthPolicy::default(),
            );
        let report = ProcessRewardReport {
            total: 0.95,
            components: ProcessRewardComponents {
                coordination: 0.95,
                reflection: 0.95,
                validation: 0.95,
                toolsmith: 0.95,
                recursion: 0.95,
                admission: 0.95,
            },
            action: RewardAction::Reinforce,
            notes: vec!["ready rust tool earned promotion".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "toolsmith",
                "reinforce",
                "ready rust proposal can enter the tool build boundary",
                0.95,
            )],
        };
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &report,
                ProcessRewardReportHealthPolicy::default(),
            );
        let admission = EvolutionAdmissionGate::new().gate(toolsmith_record, reward_record);

        let gate = AgentAdapterBoundaryGate::from_evolution_admission(&admission);
        let snapshot =
            AgentAdapterBoundarySnapshot::from_evolution_admission(&business_queue(), &admission);
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_evolution_admission_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &admission,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(admission.can_promote_ready_proposals());
        assert!(admission.can_promote_adaptive_state());
        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert!(snapshot.allows_service_advance());
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
        assert!(boundary_record.allows_service_advance());
        assert!(boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(boundary_record.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_boundary_keeps_reflection_and_evolution_side_effects_isolated() {
        let reflection_record = stable_reflection_record();
        let toolsmith_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));
        let toolsmith_record = ToolsmithPlanSummaryHistoryRecorder::new()
            .record_plan_with_health_gate(
                ToolsmithPlanSummaryHistory::new(),
                &toolsmith_plan,
                ToolsmithPlanHealthPolicy::default(),
            );
        let report = ProcessRewardReport {
            total: 0.95,
            components: ProcessRewardComponents {
                coordination: 0.95,
                reflection: 0.95,
                validation: 0.95,
                toolsmith: 0.95,
                recursion: 0.95,
                admission: 0.95,
            },
            action: RewardAction::Reinforce,
            notes: vec!["ready rust tool earned promotion".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "toolsmith",
                "reinforce",
                "ready rust proposal can enter the tool build boundary",
                0.95,
            )],
        };
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &report,
                ProcessRewardReportHealthPolicy::default(),
            );
        let admission = EvolutionAdmissionGate::new().gate(toolsmith_record, reward_record);
        let reflection_gate =
            AgentAdapterBoundaryGate::from_reflection_loop_history_gate(&reflection_record);
        let evolution_gate = AgentAdapterBoundaryGate::from_evolution_admission(&admission);

        let snapshot = AgentAdapterBoundarySnapshot::from_gates(
            &business_queue(),
            vec![evolution_gate.clone(), reflection_gate.clone()],
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_snapshot_boundary_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                snapshot.clone(),
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(reflection_record.can_promote_memory_note());
        assert!(admission.can_promote_ready_proposals());
        assert!(admission.can_promote_adaptive_state());
        assert_eq!(
            reflection_gate.owner,
            AgentAdapterBoundaryOwner::NorionMemory
        );
        assert!(reflection_gate.memory_note_allowed);
        assert!(!reflection_gate.adaptive_state_allowed);
        assert_eq!(
            evolution_gate.owner,
            AgentAdapterBoundaryOwner::ServiceAdapter
        );
        assert!(!evolution_gate.memory_note_allowed);
        assert!(evolution_gate.adaptive_state_allowed);
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Stable);
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(snapshot.blocked_reasons.is_empty());
        assert_eq!(
            snapshot
                .gates
                .iter()
                .map(|gate| gate.owner)
                .collect::<Vec<_>>(),
            vec![
                AgentAdapterBoundaryOwner::NorionMemory,
                AgentAdapterBoundaryOwner::ServiceAdapter,
            ]
        );
        assert!(boundary_record.allows_service_advance());
        assert!(boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_boundary_projects_dirty_toolsmith_evolution_admission_as_repair() {
        let toolsmith_plan = ToolsmithPlan::new()
            .with_proposal(ToolProposal::new(
                "trace-script",
                ToolIntent::TraceAnalysis,
                "python",
                "tools/trace.py",
                ToolBuildStatus::Ready,
            ))
            .with_rejected_request("shell tool outside rust crate");
        let toolsmith_record = ToolsmithPlanSummaryHistoryRecorder::new()
            .record_plan_with_health_gate(
                ToolsmithPlanSummaryHistory::new(),
                &toolsmith_plan,
                ToolsmithPlanHealthPolicy::default(),
            );
        let report = ProcessRewardReport {
            total: 0.95,
            components: ProcessRewardComponents {
                coordination: 0.95,
                reflection: 0.95,
                validation: 0.95,
                toolsmith: 0.95,
                recursion: 0.95,
                admission: 0.95,
            },
            action: RewardAction::Reinforce,
            notes: vec!["reward cannot bypass dirty toolsmith history".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "toolsmith",
                "reinforce",
                "dirty toolsmith history must repair before tool build",
                0.95,
            )],
        };
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &report,
                ProcessRewardReportHealthPolicy::default(),
            );
        let admission = EvolutionAdmissionGate::new().gate(toolsmith_record, reward_record);

        let gate = AgentAdapterBoundaryGate::from_evolution_admission(&admission);
        let snapshot =
            AgentAdapterBoundarySnapshot::from_evolution_admission(&business_queue(), &admission);
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_evolution_admission_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &admission,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(admission.requires_repair_first());
        assert!(!admission.can_promote_ready_proposals());
        assert!(!admission.can_promote_adaptive_state());
        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(snapshot.blocked_reasons.iter().any(|reason| {
            reason == "service_adapter:toolsmith:toolsmith_plan_history:toolsmith_plan_rejected=1>0"
        }));
        assert!(!boundary_record.allows_service_advance());
        assert!(boundary_record.requires_repair_first());
        assert!(!boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_boundary_keeps_reinforce_reward_from_bypassing_reflection_memory_gate() {
        let mut reflection = ReflectionLoop::new();
        reflection
            .submit(ReflectionStage::Draft, "draft still needs critique")
            .unwrap();
        let reflection_record = ReflectionLoopSummaryHistoryRecorder::new()
            .record_loop_with_health_gate(
                ReflectionLoopSummaryHistory::new(),
                &reflection,
                ReflectionLoopHealthPolicy::default(),
            );
        let report = ProcessRewardReport {
            total: 0.92,
            components: ProcessRewardComponents {
                coordination: 0.9,
                reflection: 0.9,
                validation: 0.9,
                toolsmith: 0.9,
                recursion: 0.9,
                admission: 0.9,
            },
            action: RewardAction::Reinforce,
            notes: vec!["high-quality intermediate work".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "closed_loop",
                "reinforce",
                "reward alone cannot promote memory before reflection closes",
                0.92,
            )],
        };
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &report,
                ProcessRewardReportHealthPolicy::default(),
            );
        let admission = ReflectionRewardAdmissionGate::new().gate(reflection_record, reward_record);

        let snapshot = AgentAdapterBoundarySnapshot::from_reflection_reward_admission(
            &business_queue(),
            &admission,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_reflection_reward_admission_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &admission,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!admission.requires_repair_first);
        assert!(admission.can_continue_reflection);
        assert!(!admission.can_promote_memory_note);
        assert!(!admission.can_promote_evolution_signals);
        assert!(!admission.can_reinforce_process);
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Watch);
        assert!(!snapshot.requires_repair_first());
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(snapshot.blocked_reasons.iter().any(|reason| {
            reason == "eval_reporting:reflection:reflection_incomplete_next_stage=critique"
        }));
        assert!(boundary_record.allows_service_advance());
        assert!(boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_boundary_orders_reflection_history_repair_before_reward_admission() {
        let reflection_record = stalled_reflection_record();
        let report = ProcessRewardReport {
            total: 0.95,
            components: ProcessRewardComponents {
                coordination: 0.95,
                reflection: 0.95,
                validation: 0.95,
                toolsmith: 0.95,
                recursion: 0.95,
                admission: 0.95,
            },
            action: RewardAction::Reinforce,
            notes: vec!["reward is positive but reflection history is stalled".to_owned()],
            evolution_signals: vec![EvolutionSignal::new(
                "closed_loop",
                "reinforce",
                "stalled reflection history must repair before side effects",
                0.95,
            )],
        };
        let reward_record = ProcessRewardReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ProcessRewardReportSummaryHistory::new(),
                &report,
                ProcessRewardReportHealthPolicy::default(),
            );
        let admission =
            ReflectionRewardAdmissionGate::new().gate(reflection_record.clone(), reward_record);
        let snapshot = AgentAdapterBoundarySnapshot::from_gates(
            &business_queue(),
            vec![
                AgentAdapterBoundaryGate::from_reflection_reward_admission(&admission),
                AgentAdapterBoundaryGate::from_reflection_loop_history_gate(&reflection_record),
            ],
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_snapshot_boundary_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                snapshot.clone(),
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(reflection_record.requires_repair_first());
        assert!(admission.requires_repair_first);
        assert!(!admission.can_promote_memory_note);
        assert!(!admission.can_promote_evolution_signals);
        assert!(!admission.can_reinforce_process);
        assert_eq!(
            snapshot
                .gates
                .iter()
                .map(|gate| gate.owner)
                .collect::<Vec<_>>(),
            vec![
                AgentAdapterBoundaryOwner::NorionMemory,
                AgentAdapterBoundaryOwner::EvalReporting,
            ]
        );
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Repair);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        let memory_reason = snapshot
            .blocked_reasons
            .iter()
            .position(|reason| {
                reason
                    == "norion_memory:reflection_loop_history:reflection_loop_stalled_stage_records=1>0"
            })
            .expect("reflection history repair reason should stay on the memory boundary");
        let reward_reason = snapshot
            .blocked_reasons
            .iter()
            .position(|reason| {
                reason
                    == "eval_reporting:reflection:reflection_loop_history:reflection_loop_stalled_stage_records=1>0"
            })
            .expect("reward admission should preserve reflection repair reason");
        assert!(memory_reason < reward_reason);
        assert_eq!(
            boundary_record.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(!boundary_record.allows_service_advance());
        assert!(!boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_gate_projects_service_side_effect_admission() {
        let admission = AgentCollaborationAdapterSideEffectAdmission {
            mode: AgentClosedLoopNextTurnMode::Observe,
            health_status: AgentClosedLoopExecutionHealthStatus::Watch,
            can_dispatch_service_commands: true,
            can_promote_memory_note: false,
            can_admit_adaptive_evolution: false,
            requires_repair_first: false,
            gates: vec![
                SideEffectGate::allow(SideEffectKind::ExternalCall, "observe service command"),
                SideEffectGate::block(SideEffectKind::MemoryNote, "watch reflection"),
                SideEffectGate::block(SideEffectKind::AdaptiveStateWrite, "watch reflection"),
            ],
            reasons: vec!["reflection_watch".to_owned()],
            service_execution_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_closes: 0,
            service_execution_rust_validation_command_count: 0,
            service_execution_rust_validation_command_closes: 0,
            service_execution_tool_build_command_reason_count: 0,
            telemetry: Vec::new(),
        };

        let gate = AgentAdapterBoundaryGate::from_service_admission(&admission);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert_eq!(gate.blocked_reasons, vec!["reflection_watch"]);
    }

    #[test]
    fn adapter_boundary_surfaces_service_tool_build_reason_pressure() {
        let admission = AgentCollaborationAdapterSideEffectAdmission {
            mode: AgentClosedLoopNextTurnMode::Continue,
            health_status: AgentClosedLoopExecutionHealthStatus::Stable,
            can_dispatch_service_commands: true,
            can_promote_memory_note: true,
            can_admit_adaptive_evolution: true,
            requires_repair_first: false,
            gates: vec![
                SideEffectGate::allow(SideEffectKind::ExternalCall, "service command"),
                SideEffectGate::allow(SideEffectKind::MemoryNote, "memory note"),
                SideEffectGate::allow(SideEffectKind::AdaptiveStateWrite, "adaptive state"),
            ],
            reasons: vec!["tool_build_command_reason=repair_tool".to_owned()],
            service_execution_command_reason_count: 4,
            service_execution_memory_promotion_command_reason_count: 0,
            service_execution_memory_promotion_command_reason_closes: 0,
            service_execution_rust_validation_command_count: 0,
            service_execution_rust_validation_command_closes: 0,
            service_execution_tool_build_command_reason_count: 4,
            telemetry: Vec::new(),
        };

        let gate = AgentAdapterBoundaryGate::from_service_admission(&admission);
        assert_eq!(gate.service_execution_tool_build_command_reason_count, 4);

        let snapshot = AgentAdapterBoundarySnapshot::from_gates(&business_queue(), vec![gate]);
        assert_eq!(
            snapshot.service_execution_tool_build_command_reason_count(),
            4
        );
        assert!(snapshot.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_snapshot_service_tool_build_command_reasons=4"
        }));

        let summary = snapshot.summary();
        assert_eq!(summary.service_execution_tool_build_command_reason_count, 4);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_summary_service_tool_build_command_reasons=4"
        }));

        let dashboard =
            AgentAdapterBoundarySummaryHistory::from_summaries(vec![summary]).dashboard();
        assert_eq!(
            dashboard.service_execution_tool_build_command_reason_count,
            4
        );
        assert!(dashboard.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_dashboard_service_tool_build_command_reasons=4"
        }));

        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_snapshot_boundary_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                snapshot,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff =
            AgentAdapterBoundaryHandoff::from_record_and_queue(boundary_record, &business_queue());
        assert_eq!(handoff.service_execution_tool_build_command_reason_count, 4);
        assert_eq!(
            handoff
                .summary()
                .service_execution_tool_build_command_reason_count,
            4
        );

        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        assert_eq!(
            handoff_history_record
                .dashboard
                .service_execution_tool_build_command_reason_count,
            4
        );

        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_history_record);
        assert_eq!(
            trend_decision.service_execution_tool_build_command_reason_count,
            4
        );
        let trend_history_record = AgentAdapterBoundaryHandoffTrendGateHistoryRecorder::new()
            .record_decision_with_health(
                AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
                &trend_decision,
                AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
            );
        assert_eq!(
            trend_history_record
                .dashboard
                .service_execution_tool_build_command_reason_count,
            4
        );

        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
        );
        assert_eq!(
            monitor_record.service_execution_tool_build_command_reason_count,
            4
        );
        let continuation = monitor_record.continuation(
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
            AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
        );
        assert_eq!(
            continuation.service_execution_tool_build_command_reason_count,
            4
        );

        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_history_record,
        );
        assert_eq!(
            resume_record.service_execution_tool_build_command_reason_count,
            4
        );
        assert_eq!(
            resume_record
                .summary()
                .service_execution_tool_build_command_reason_count,
            4
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        assert_eq!(
            resume_history_record
                .dashboard
                .service_execution_tool_build_command_reason_count,
            4
        );

        let resume_gate_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        assert_eq!(
            resume_gate_handoff.service_execution_tool_build_command_reason_count,
            4
        );
        assert_eq!(
            resume_gate_handoff
                .summary()
                .service_execution_tool_build_command_reason_count,
            4
        );
    }

    #[test]
    fn adapter_gate_projects_stable_run_report_final_handoff_as_open_service_boundary() {
        let decision = run_report_final_gate_decision(
            AgentRunReportHealthStatus::Stable,
            true,
            false,
            Vec::new(),
        );

        let gate = AgentAdapterBoundaryGate::from_run_report_final_gate_decision(&decision);
        let snapshot = AgentAdapterBoundarySnapshot::from_run_report_final_gate_decision(&decision);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert!(gate.blocked_reasons.is_empty());
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Stable);
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_gate_projects_watch_run_report_final_handoff_as_observation_only() {
        let decision = run_report_final_gate_decision(
            AgentRunReportHealthStatus::Watch,
            true,
            false,
            vec!["run_report_final_watch".to_owned()],
        );

        let gate = AgentAdapterBoundaryGate::from_run_report_final_gate_decision(&decision);
        let snapshot = AgentAdapterBoundarySnapshot::from_run_report_final_gate_decision(&decision);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(
            gate.blocked_reasons
                .contains(&"run_report_final_watch".to_owned())
        );
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Watch);
        assert!(snapshot.allows_service_advance());
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
    }

    #[test]
    fn adapter_gate_projects_repair_run_report_final_handoff_as_closed_boundary() {
        let decision = run_report_final_gate_decision(
            AgentRunReportHealthStatus::Repair,
            false,
            true,
            vec!["run_report_final_repair".to_owned()],
        );

        let gate = AgentAdapterBoundaryGate::from_run_report_final_gate_decision(&decision);
        let snapshot = AgentAdapterBoundarySnapshot::from_run_report_final_gate_decision(&decision);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(
            gate.blocked_reasons
                .contains(&"run_report_final_repair".to_owned())
        );
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Repair);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.allows_service_advance());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(
            snapshot.next_queue_task_ids,
            vec!["business-task", "run-report-final-repair"]
        );
    }

    #[test]
    fn adapter_history_recorder_records_run_report_final_gate_decision() {
        let decision = run_report_final_gate_decision(
            AgentRunReportHealthStatus::Stable,
            true,
            false,
            Vec::new(),
        );

        let record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_run_report_final_gate_decision_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &decision,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.snapshot.status(), AgentAdapterBoundaryStatus::Stable);
        assert_eq!(record.summary().status, AgentAdapterBoundaryStatus::Stable);
        assert_eq!(
            record.history_record.health.status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert!(record.allows_service_advance());
        assert!(record.can_submit_memory_note());
        assert!(record.can_promote_adaptive_state());
        assert_eq!(record.snapshot.next_queue_task_ids, vec!["business-task"]);
    }

    #[test]
    fn adapter_history_closes_clean_run_report_final_when_prior_boundary_repairs() {
        let dirty_decision = run_report_final_gate_decision(
            AgentRunReportHealthStatus::Repair,
            false,
            true,
            vec!["prior_run_report_final_repair".to_owned()],
        );
        let dirty_summary =
            AgentAdapterBoundarySnapshot::from_run_report_final_gate_decision(&dirty_decision)
                .summary();
        let clean_decision = run_report_final_gate_decision(
            AgentRunReportHealthStatus::Stable,
            true,
            false,
            Vec::new(),
        );
        let history = AgentAdapterBoundarySummaryHistory::from_summaries(vec![dirty_summary]);

        let record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_run_report_final_gate_decision_with_health(
                history.clone(),
                &clean_decision,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_run_report_final_gate_decision_handoff_with_health(
                history,
                &clean_decision,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert_eq!(record.records(), 2);
        assert_eq!(record.snapshot.status(), AgentAdapterBoundaryStatus::Stable);
        assert_eq!(
            record.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(record.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.snapshot.can_submit_memory_note());
        assert!(record.snapshot.can_promote_adaptive_state());
        assert!(!record.can_submit_memory_note());
        assert!(!record.can_promote_adaptive_state());
        assert!(!record.can_execute_service_commands());
        assert!(
            record
                .history_record
                .health
                .reasons
                .iter()
                .any(|reason| reason == "adapter_boundary_repair_records=1>0")
        );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(!handoff.can_execute_service_commands());
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert!(
            handoff
                .next_queue
                .task_ids()
                .first()
                .is_some_and(|id| { id.starts_with("adapter-boundary-repair-") })
        );
        assert!(
            handoff
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
    }

    #[test]
    fn adapter_history_recorder_handoff_merges_run_report_final_repair_queue() {
        let decision = run_report_final_gate_decision(
            AgentRunReportHealthStatus::Repair,
            false,
            true,
            vec!["run_report_final_repair".to_owned()],
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_run_report_final_gate_decision_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &decision,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "service_adapter:run_report_final_repair")
        );
        assert!(
            handoff
                .repair_tasks
                .iter()
                .all(|task| task.lane == "adapter-boundary-repair")
        );
        let next_queue_ids = handoff.next_queue.task_ids();
        assert_eq!(handoff.repair_tasks.len(), 8);
        assert!(
            next_queue_ids[..8]
                .iter()
                .all(|id| id.starts_with("adapter-boundary-repair-"))
        );
        assert_eq!(
            &next_queue_ids[8..],
            ["business-task", "run-report-final-repair"]
        );
    }

    #[test]
    fn adapter_gate_projects_stable_service_execution_final_handoff_as_open_service_boundary() {
        let decision = service_execution_final_gate_decision(
            AgentClosedLoopExecutionHealthStatus::Stable,
            true,
            false,
            Vec::new(),
        );

        let gate = AgentAdapterBoundaryGate::from_service_execution_final_gate_decision(&decision);
        let snapshot =
            AgentAdapterBoundarySnapshot::from_service_execution_final_gate_decision(&decision);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert!(gate.blocked_reasons.is_empty());
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Stable);
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_gate_projects_watch_service_execution_final_handoff_as_observation_only() {
        let decision = service_execution_final_gate_decision(
            AgentClosedLoopExecutionHealthStatus::Watch,
            true,
            false,
            vec!["service_execution_final_watch".to_owned()],
        );

        let gate = AgentAdapterBoundaryGate::from_service_execution_final_gate_decision(&decision);
        let snapshot =
            AgentAdapterBoundarySnapshot::from_service_execution_final_gate_decision(&decision);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(
            gate.blocked_reasons
                .contains(&"service_execution_final_watch".to_owned())
        );
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Watch);
        assert!(snapshot.allows_service_advance());
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
    }

    #[test]
    fn adapter_gate_projects_repair_service_execution_final_handoff_as_closed_boundary() {
        let decision = service_execution_final_gate_decision(
            AgentClosedLoopExecutionHealthStatus::Repair,
            false,
            true,
            vec!["service_execution_final_repair".to_owned()],
        );

        let gate = AgentAdapterBoundaryGate::from_service_execution_final_gate_decision(&decision);
        let snapshot =
            AgentAdapterBoundarySnapshot::from_service_execution_final_gate_decision(&decision);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(
            gate.blocked_reasons
                .contains(&"service_execution_final_repair".to_owned())
        );
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Repair);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.allows_service_advance());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(
            snapshot.next_queue_task_ids,
            vec!["business-task", "service-execution-final-repair"]
        );
    }

    #[test]
    fn adapter_history_recorder_handoff_merges_service_execution_final_repair_queue() {
        let decision = service_execution_final_gate_decision(
            AgentClosedLoopExecutionHealthStatus::Repair,
            false,
            true,
            vec!["service_execution_final_repair".to_owned()],
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_service_execution_final_gate_decision_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &decision,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "service_adapter:service_execution_final_repair")
        );
        assert!(
            handoff
                .repair_tasks
                .iter()
                .all(|task| task.lane == "adapter-boundary-repair")
        );
        let next_queue_ids = handoff.next_queue.task_ids();
        assert!(
            next_queue_ids
                .iter()
                .take(handoff.repair_tasks.len())
                .all(|id| id.starts_with("adapter-boundary-repair-"))
        );
        assert_eq!(
            &next_queue_ids[handoff.repair_tasks.len()..],
            ["business-task", "service-execution-final-repair"]
        );
    }

    #[test]
    fn adapter_gate_projects_runtime_service_loop_continue_as_open_boundary() {
        let plan = runtime_service_loop_control_plan(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopNextTurnMode::Continue,
            business_queue(),
            Vec::new(),
        );

        let gate = AgentAdapterBoundaryGate::from_runtime_service_loop_control_plan(&plan);
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_control_plan(&plan);

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Stable);
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_gate_projects_runtime_service_loop_observe_without_promotion() {
        let plan = runtime_service_loop_control_plan(
            AgentClosedLoopExecutionHealthStatus::Watch,
            AgentClosedLoopNextTurnMode::Observe,
            business_queue(),
            vec!["runtime_service_loop_watch".to_owned()],
        );

        let gate = AgentAdapterBoundaryGate::from_runtime_service_loop_control_plan(&plan);
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_control_plan(&plan);

        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert_eq!(
            snapshot.blocked_reasons,
            vec!["service_adapter:runtime_service_loop_watch"]
        );
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_gate_projects_runtime_service_loop_idle_as_closed_observation() {
        let plan = runtime_service_loop_control_plan(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopNextTurnMode::Idle,
            AgentTaskQueue::new(),
            vec!["next_queue_empty".to_owned()],
        );

        let gate = AgentAdapterBoundaryGate::from_runtime_service_loop_control_plan(&plan);
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_control_plan(&plan);

        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Watch);
        assert!(snapshot.allows_service_advance());
        assert!(!snapshot.can_execute_service_commands());
        assert_eq!(snapshot.next_queue_task_ids, Vec::<String>::new());
    }

    #[test]
    fn adapter_history_recorder_handoff_merges_runtime_service_loop_repair_queue() {
        let plan = runtime_service_loop_control_plan(
            AgentClosedLoopExecutionHealthStatus::Repair,
            AgentClosedLoopNextTurnMode::Repair,
            business_queue(),
            vec!["runtime_service_loop_repair".to_owned()],
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_runtime_service_loop_control_plan_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &plan,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "service_adapter:runtime_service_loop_repair")
        );
        assert!(
            handoff
                .repair_tasks
                .iter()
                .all(|task| task.lane == "adapter-boundary-repair")
        );
        assert!(
            handoff
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
    }

    #[test]
    fn adapter_gate_projects_runtime_service_preflight_continue_as_open_boundary() {
        let preflight = runtime_service_preflight(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            business_queue(),
        );

        let gate = AgentAdapterBoundaryGate::from_runtime_service_preflight(&preflight);
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_preflight(&preflight);

        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_gate_projects_runtime_service_preflight_continuation_with_follow_up_queue() {
        let preflight = runtime_service_preflight(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Watch,
            business_queue(),
        );
        let continuation = AgentClosedLoopRuntimeServicePreflightContinuationPlanner::new().plan(
            preflight,
            AgentClosedLoopRuntimeContinuationInput::new(
                BudgetLedger::new(),
                AgentCycleEvidence::default(),
            ),
        );

        let gate =
            AgentAdapterBoundaryGate::from_runtime_service_preflight_continuation(&continuation);
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_preflight_continuation(
            &continuation,
        );

        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(
            snapshot
                .next_queue_task_ids
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(
            snapshot
                .next_queue_task_ids
                .iter()
                .any(|id| id.starts_with("service-preflight-observe-"))
        );
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_gate_projects_runtime_service_loop_state_with_follow_up_queue() {
        let state = runtime_service_loop_state(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Watch,
            business_queue(),
        );

        let gate = AgentAdapterBoundaryGate::from_runtime_service_loop_state(&state);
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_state(&state);

        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert_eq!(
            snapshot.next_queue_task_ids,
            state.next_runtime_input().next_queue.task_ids()
        );
        assert!(
            snapshot
                .next_queue_task_ids
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(
            snapshot
                .next_queue_task_ids
                .iter()
                .any(|id| id.starts_with("service-preflight-observe-"))
        );
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
    }

    #[test]
    fn adapter_gate_projects_runtime_service_loop_advance_as_open_boundary() {
        let state = runtime_service_loop_state(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            business_queue(),
        );
        let advance = runtime_service_loop_advance(state);

        let gate = AgentAdapterBoundaryGate::from_runtime_service_loop_advance(&advance);
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_advance(&advance);

        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert_eq!(
            snapshot.next_queue_task_ids,
            advance.next_runtime_input().next_queue.task_ids()
        );
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_history_recorder_handoff_merges_runtime_service_preflight_repair_queue() {
        let preflight = runtime_service_preflight(
            AgentClosedLoopExecutionHealthStatus::Repair,
            AgentClosedLoopExecutionHealthStatus::Repair,
            business_queue(),
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_runtime_service_preflight_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &preflight,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff
                .repair_tasks
                .iter()
                .all(|task| task.lane == "adapter-boundary-repair")
        );
        assert!(
            handoff
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
    }

    #[test]
    fn adapter_history_recorder_handoff_merges_runtime_service_loop_state_repair_queue() {
        let state = runtime_service_loop_state(
            AgentClosedLoopExecutionHealthStatus::Repair,
            AgentClosedLoopExecutionHealthStatus::Repair,
            business_queue(),
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_runtime_service_loop_state_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &state,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("service_adapter:"))
        );
        assert!(
            handoff
                .repair_tasks
                .iter()
                .all(|task| task.lane == "adapter-boundary-repair")
        );
        assert!(
            handoff
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(
            handoff
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id.starts_with("service-preflight-repair-"))
        );
    }

    #[test]
    fn adapter_history_recorder_handoff_merges_runtime_service_loop_advance_repair_queue() {
        let state = runtime_service_loop_state(
            AgentClosedLoopExecutionHealthStatus::Repair,
            AgentClosedLoopExecutionHealthStatus::Repair,
            business_queue(),
        );
        let advance = runtime_service_loop_advance(state);

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_runtime_service_loop_advance_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &advance,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff
                .repair_tasks
                .iter()
                .all(|task| task.lane == "adapter-boundary-repair")
        );
        assert!(
            handoff
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id.starts_with("adapter-boundary-repair-"))
        );
        assert!(
            handoff
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id.starts_with("service-preflight-repair-"))
        );
        assert!(
            handoff
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
    }

    #[test]
    fn adapter_gate_projects_daemon_continuation_as_open_boundary() {
        let continuation = runtime_service_loop_daemon_continuation(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopNextTurnMode::Continue,
            true,
            1.0,
            1.0,
            true,
            false,
            business_queue(),
        );

        let gate =
            AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_continuation(&continuation);
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_continuation(
            &continuation,
        );

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_history_recorder_handoff_merges_daemon_continuation_repair_queue() {
        let continuation = runtime_service_loop_daemon_continuation(
            AgentClosedLoopExecutionHealthStatus::Repair,
            AgentClosedLoopExecutionHealthStatus::Watch,
            AgentClosedLoopNextTurnMode::Repair,
            true,
            1.0,
            1.0,
            true,
            true,
            business_queue(),
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_runtime_service_loop_daemon_continuation_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &continuation,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff.blocked_reasons.iter().any(|reason| reason
                == "service_adapter:runtime_service_loop_daemon_requires_repair_first")
        );
        assert!(
            handoff
                .repair_tasks
                .iter()
                .all(|task| task.lane == "adapter-boundary-repair")
        );
        assert_eq!(
            handoff.next_queue.task_ids().last(),
            Some(&"business-task".to_owned())
        );
    }

    #[test]
    fn adapter_gate_projects_daemon_input_plan_as_observation_only_boundary() {
        let plan = runtime_service_loop_daemon_input_plan(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopNextTurnMode::Continue,
            true,
            1.0,
            1.0,
            true,
            false,
            business_queue(),
        );

        let gate = AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_input_plan(&plan);
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_input_plan(&plan);

        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(snapshot.allows_service_advance());
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(
            snapshot.blocked_reasons,
            vec!["service_adapter:runtime_service_loop_daemon_input_plan_observe_only"]
        );
    }

    #[test]
    fn adapter_history_recorder_handoff_keeps_daemon_input_plan_queue_without_memory_promotion() {
        let plan = runtime_service_loop_daemon_input_plan(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopNextTurnMode::Continue,
            true,
            1.0,
            1.0,
            true,
            false,
            business_queue(),
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_runtime_service_loop_daemon_input_plan_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &plan,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(handoff.is_admitted());
        assert!(!handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(handoff.repair_tasks.is_empty());
        assert_eq!(handoff.next_queue.task_ids(), vec!["business-task"]);
    }

    #[test]
    fn adapter_gate_projects_daemon_request_plan_as_open_boundary() {
        let plan = runtime_service_loop_daemon_request_plan(
            AgentClosedLoopNextTurnMode::Continue,
            true,
            1.0,
            1.0,
            true,
            false,
            business_queue(),
        );

        let gate = AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_plan(&plan);
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_plan(&plan);

        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_history_recorder_handoff_merges_daemon_request_plan_repair_queue() {
        let plan = runtime_service_loop_daemon_request_plan(
            AgentClosedLoopNextTurnMode::Repair,
            true,
            1.0,
            1.0,
            true,
            true,
            business_queue(),
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_runtime_service_loop_daemon_request_plan_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &plan,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(handoff.blocked_reasons.iter().any(|reason| reason
            == "service_adapter:runtime_service_loop_daemon_request_requires_repair_first"));
        assert!(
            handoff
                .repair_tasks
                .iter()
                .all(|task| task.lane == "adapter-boundary-repair")
        );
        assert_eq!(
            handoff.next_queue.task_ids().last(),
            Some(&"business-task".to_owned())
        );
    }

    #[test]
    fn adapter_gate_projects_daemon_request_monitored_plan_as_open_boundary() {
        let plan = runtime_service_loop_daemon_request_monitored_plan(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopNextTurnMode::Continue,
            true,
            1.0,
            1.0,
            true,
            false,
            business_queue(),
            Vec::new(),
            Vec::new(),
        );

        let gate =
            AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_plan(
                &plan,
            );
        let snapshot =
            AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_monitored_plan(
                &plan,
            );

        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_history_recorder_handoff_merges_daemon_request_monitored_plan_repair_queue() {
        let plan = runtime_service_loop_daemon_request_monitored_plan(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Repair,
            AgentClosedLoopNextTurnMode::Repair,
            true,
            1.0,
            1.0,
            true,
            true,
            business_queue(),
            Vec::new(),
            vec!["daemon_request_monitored_control_repair".to_owned()],
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_runtime_service_loop_daemon_request_monitored_plan_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &plan,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "service_adapter:daemon_request_monitored_control_repair")
        );
        assert!(
            handoff
                .repair_tasks
                .iter()
                .all(|task| task.lane == "adapter-boundary-repair")
        );
        assert_eq!(
            handoff.next_queue.task_ids().last(),
            Some(&"business-task".to_owned())
        );
    }

    #[test]
    fn adapter_gate_projects_daemon_request_monitored_close_continue_as_open_boundary() {
        let plan = runtime_service_loop_daemon_request_monitored_close_plan(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopNextTurnMode::Continue,
            true,
            1.0,
            1.0,
            true,
            false,
            business_queue(),
            Vec::new(),
        );

        let gate =
            AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_close_plan(
                &plan,
            );
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_monitored_close_plan(
            &plan,
        );

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Stable);
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_gate_projects_daemon_request_monitored_close_watch_without_memory_promotion() {
        let plan = runtime_service_loop_daemon_request_monitored_close_plan(
            AgentClosedLoopExecutionHealthStatus::Watch,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopNextTurnMode::Observe,
            true,
            1.0,
            0.0,
            false,
            false,
            business_queue(),
            vec!["monitored_close_watch".to_owned()],
        );

        let gate =
            AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_close_plan(
                &plan,
            );
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_monitored_close_plan(
            &plan,
        );

        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert_eq!(
            snapshot.blocked_reasons,
            vec![
                "service_adapter:monitored_close_watch",
                "service_adapter:runtime_service_loop_daemon_request_monitored_close_memory_note_closed",
                "service_adapter:runtime_service_loop_daemon_request_monitored_close_adaptive_closed",
            ]
        );
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_history_recorder_handoff_merges_daemon_request_monitored_close_repair_queue() {
        let plan = runtime_service_loop_daemon_request_monitored_close_plan(
            AgentClosedLoopExecutionHealthStatus::Repair,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Repair,
            AgentClosedLoopNextTurnMode::Repair,
            true,
            1.0,
            1.0,
            true,
            true,
            business_queue(),
            vec!["monitored_close_repair".to_owned()],
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_runtime_service_loop_daemon_request_monitored_close_plan_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &plan,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            !handoff
                .boundary_record
                .snapshot
                .can_execute_service_commands()
        );
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "service_adapter:monitored_close_repair")
        );
        assert!(
            handoff
                .repair_tasks
                .iter()
                .all(|task| task.lane == "adapter-boundary-repair")
        );
        assert_eq!(
            handoff.next_queue.task_ids().last(),
            Some(&"business-task".to_owned())
        );
    }

    #[test]
    fn adapter_gate_projects_daemon_request_monitored_close_continuation_as_open_boundary() {
        let continuation = runtime_service_loop_daemon_request_monitored_close_continuation(
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopNextTurnMode::Continue,
            true,
            1.0,
            1.0,
            true,
            false,
            business_queue(),
            Vec::new(),
        );

        let gate = AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_close_continuation(
            &continuation,
        );
        let snapshot = AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_monitored_close_continuation(
            &continuation,
        );

        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
    }

    #[test]
    fn adapter_history_recorder_handoff_merges_daemon_request_monitored_close_continuation_repair_queue()
     {
        let continuation = runtime_service_loop_daemon_request_monitored_close_continuation(
            AgentClosedLoopExecutionHealthStatus::Repair,
            AgentClosedLoopExecutionHealthStatus::Repair,
            AgentClosedLoopExecutionHealthStatus::Stable,
            AgentClosedLoopNextTurnMode::Repair,
            true,
            1.0,
            1.0,
            true,
            true,
            business_queue(),
            vec!["monitored_close_continuation_repair".to_owned()],
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_runtime_service_loop_daemon_request_monitored_close_continuation_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &continuation,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "service_adapter:monitored_close_continuation_repair")
        );
        assert_eq!(
            handoff.next_queue.task_ids().last(),
            Some(&"business-task".to_owned())
        );
    }

    #[test]
    fn adapter_boundary_surfaces_service_admission_reason_pressure() {
        let mut service = service_admission(
            AgentClosedLoopNextTurnMode::Observe,
            true,
            false,
            "memory_promotion_command_reason_close",
        );
        service.service_execution_command_reason_count = 2;
        service.service_execution_memory_promotion_command_reason_count = 2;
        service.service_execution_memory_promotion_command_reason_closes = 1;

        let gate = AgentAdapterBoundaryGate::from_service_admission(&service);
        assert_eq!(gate.service_execution_command_reason_count, 2);
        assert_eq!(
            gate.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            gate.service_execution_memory_promotion_command_reason_closes,
            1
        );

        let snapshot = AgentAdapterBoundarySnapshot::from_boundary_gates(
            &business_queue(),
            &clean_dispatch_gate(),
            &memory_submission_gate(true),
            &accepted_report_gate(),
            &service,
        );
        assert_eq!(snapshot.service_execution_command_reason_count(), 2);
        assert_eq!(
            snapshot.service_execution_memory_promotion_command_reason_count(),
            2
        );
        assert_eq!(
            snapshot.service_execution_memory_promotion_command_reason_closes(),
            1
        );
        assert!(
            snapshot.telemetry.contains(
                &"agent_adapter_boundary_snapshot_service_memory_promotion_command_reason_closes=1"
                    .to_owned()
            )
        );

        let summary = snapshot.summary();
        assert_eq!(summary.service_execution_command_reason_count, 2);
        assert_eq!(
            summary.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            summary.service_execution_memory_promotion_command_reason_closes,
            1
        );
        assert_eq!(
            summary
                .telemetry
                .iter()
                .filter(|line| line.ends_with("_service_memory_promotion_command_reason_closes=1"))
                .count(),
            1
        );

        let dashboard =
            AgentAdapterBoundarySummaryHistory::from_summaries(vec![summary.clone()]).dashboard();
        assert_eq!(dashboard.service_execution_command_reason_count, 2);
        assert_eq!(
            dashboard.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            dashboard.service_execution_memory_promotion_command_reason_closes,
            1
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_boundary_gates_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &clean_dispatch_gate(),
                &memory_submission_gate(true),
                &accepted_report_gate(),
                &service,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff_summary = handoff.summary();
        assert_eq!(handoff_summary.service_execution_command_reason_count, 2);
        assert_eq!(
            handoff_summary.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            handoff_summary.service_execution_memory_promotion_command_reason_closes,
            1
        );

        let handoff_history =
            AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![handoff_summary]);
        let handoff_dashboard = handoff_history.dashboard();
        assert_eq!(handoff_dashboard.service_execution_command_reason_count, 2);
        assert_eq!(
            handoff_dashboard.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            handoff_dashboard.service_execution_memory_promotion_command_reason_closes,
            1
        );
        assert!(handoff_dashboard.telemetry.contains(
            &"agent_adapter_boundary_handoff_dashboard_service_memory_promotion_command_reason_closes=1"
                .to_owned()
        ));

        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_history_record);
        assert_eq!(trend_decision.service_execution_command_reason_count, 2);
        assert_eq!(
            trend_decision.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            trend_decision.service_execution_memory_promotion_command_reason_closes,
            1
        );

        let trend_summary = trend_decision.summary();
        assert_eq!(trend_summary.service_execution_command_reason_count, 2);
        assert_eq!(
            trend_summary.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            trend_summary.service_execution_memory_promotion_command_reason_closes,
            1
        );
        let trend_dashboard =
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![trend_summary])
                .dashboard();
        assert_eq!(trend_dashboard.service_execution_command_reason_count, 2);
        assert_eq!(
            trend_dashboard.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            trend_dashboard.service_execution_memory_promotion_command_reason_closes,
            1
        );
        assert!(trend_dashboard.telemetry.contains(
            &"agent_adapter_boundary_handoff_trend_gate_dashboard_service_memory_promotion_command_reason_closes=1"
                .to_owned()
        ));

        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        assert_eq!(admission.service_execution_command_reason_count, 2);
        assert_eq!(
            admission.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            admission.service_execution_memory_promotion_command_reason_closes,
            1
        );

        let admission_summary = admission.summary();
        assert_eq!(admission_summary.service_execution_command_reason_count, 2);
        assert_eq!(
            admission_summary.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            admission_summary.service_execution_memory_promotion_command_reason_closes,
            1
        );
        let admission_dashboard =
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::from_summaries(vec![
                admission_summary,
            ])
            .dashboard();
        assert_eq!(
            admission_dashboard.service_execution_command_reason_count,
            2
        );
        assert_eq!(
            admission_dashboard.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            admission_dashboard.service_execution_memory_promotion_command_reason_closes,
            1
        );
        assert!(admission_dashboard.telemetry.contains(
            &"agent_adapter_boundary_handoff_trend_admission_dashboard_service_memory_promotion_command_reason_closes=1"
                .to_owned()
        ));

        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
        );
        assert_eq!(monitor_record.service_execution_command_reason_count, 2);
        assert_eq!(
            monitor_record.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            monitor_record.service_execution_memory_promotion_command_reason_closes,
            1
        );
        assert!(monitor_record.telemetry.contains(
            &"agent_adapter_boundary_handoff_trend_admission_monitor_service_memory_promotion_command_reason_closes=1"
                .to_owned()
        ));

        let continuation = monitor_record.continuation(
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
            AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
        );
        assert_eq!(continuation.service_execution_command_reason_count, 2);
        assert_eq!(
            continuation.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            continuation.service_execution_memory_promotion_command_reason_closes,
            1
        );
        assert!(continuation.telemetry.contains(
            &"agent_adapter_boundary_handoff_trend_admission_continuation_service_memory_promotion_command_reason_closes=1"
                .to_owned()
        ));

        let resume_plan = continuation.resume_plan();
        assert_eq!(resume_plan.service_execution_command_reason_count, 2);
        assert_eq!(
            resume_plan.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            resume_plan.service_execution_memory_promotion_command_reason_closes,
            1
        );
        assert!(resume_plan.telemetry.contains(
            &"agent_adapter_boundary_handoff_trend_admission_resume_plan_service_memory_promotion_command_reason_closes=1"
                .to_owned()
        ));

        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_history_record,
        );
        assert_eq!(resume_record.service_execution_command_reason_count, 2);
        assert_eq!(
            resume_record.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            resume_record.service_execution_memory_promotion_command_reason_closes,
            1
        );
        assert!(resume_record.telemetry.contains(
            &"agent_adapter_boundary_handoff_trend_admission_resume_record_service_memory_promotion_command_reason_closes=1"
                .to_owned()
        ));

        let resume_summary = resume_record.summary();
        assert_eq!(resume_summary.service_execution_command_reason_count, 2);
        assert_eq!(
            resume_summary.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            resume_summary.service_execution_memory_promotion_command_reason_closes,
            1
        );
        let resume_dashboard =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::from_summaries(vec![
                resume_summary.clone(),
            ])
            .dashboard();
        assert_eq!(resume_dashboard.service_execution_command_reason_count, 2);
        assert_eq!(
            resume_dashboard.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            resume_dashboard.service_execution_memory_promotion_command_reason_closes,
            1
        );

        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let resume_gate_decision = AgentAdapterBoundaryHandoffTrendAdmissionResumeGate::new()
            .gate(&resume_record, &resume_history_record);
        assert_eq!(
            resume_gate_decision.service_execution_command_reason_count,
            2
        );
        assert_eq!(
            resume_gate_decision.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            resume_gate_decision.service_execution_memory_promotion_command_reason_closes,
            1
        );

        let resume_gate_summary = resume_gate_decision.summary();
        assert_eq!(
            resume_gate_summary.service_execution_command_reason_count,
            2
        );
        assert_eq!(
            resume_gate_summary.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            resume_gate_summary.service_execution_memory_promotion_command_reason_closes,
            1
        );
        let resume_gate_dashboard =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::from_summaries(
                vec![resume_gate_summary],
            )
            .dashboard();
        assert_eq!(
            resume_gate_dashboard.service_execution_command_reason_count,
            2
        );
        assert_eq!(
            resume_gate_dashboard.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            resume_gate_dashboard.service_execution_memory_promotion_command_reason_closes,
            1
        );

        let resume_gate_monitor = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitor::new()
            .monitor(
                &resume_record,
                &resume_history_record,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
            );
        assert_eq!(
            resume_gate_monitor.service_execution_command_reason_count,
            2
        );
        assert_eq!(
            resume_gate_monitor.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            resume_gate_monitor.service_execution_memory_promotion_command_reason_closes,
            1
        );

        let resume_monitor_gate =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGate::new()
                .gate(&resume_gate_monitor);
        assert_eq!(
            resume_monitor_gate.service_execution_command_reason_count,
            2
        );
        assert_eq!(
            resume_monitor_gate.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            resume_monitor_gate.service_execution_memory_promotion_command_reason_closes,
            1
        );

        let final_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        assert_eq!(final_handoff.service_execution_command_reason_count, 2);
        assert_eq!(
            final_handoff.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            final_handoff.service_execution_memory_promotion_command_reason_closes,
            1
        );
        assert!(final_handoff.telemetry.contains(
            &"agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_service_memory_promotion_command_reason_closes=1"
                .to_owned()
        ));

        let final_handoff_summary = final_handoff.summary();
        assert_eq!(
            final_handoff_summary.service_execution_command_reason_count,
            2
        );
        assert_eq!(
            final_handoff_summary.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            final_handoff_summary.service_execution_memory_promotion_command_reason_closes,
            1
        );
        let final_handoff_dashboard =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::from_summaries(
                vec![final_handoff_summary],
            )
            .dashboard();
        assert_eq!(
            final_handoff_dashboard.service_execution_command_reason_count,
            2
        );
        assert_eq!(
            final_handoff_dashboard.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            final_handoff_dashboard.service_execution_memory_promotion_command_reason_closes,
            1
        );

        let final_handoff_packet =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
                .record_and_gate(
                    final_handoff,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );
        assert_eq!(
            final_handoff_packet.service_execution_command_reason_count,
            2
        );
        assert_eq!(
            final_handoff_packet.service_execution_memory_promotion_command_reason_count,
            2
        );
        assert_eq!(
            final_handoff_packet.service_execution_memory_promotion_command_reason_closes,
            1
        );
    }

    #[test]
    fn boundary_snapshot_composes_mixed_gate_outputs_into_repair() {
        let dispatch = clean_dispatch_gate();
        let memory = memory_submission_gate(true);
        let report = rejected_report_gate();
        let service = service_admission(
            AgentClosedLoopNextTurnMode::Observe,
            true,
            false,
            "reflection_watch",
        );

        let snapshot = AgentAdapterBoundarySnapshot::from_boundary_gates(
            &business_queue(),
            &dispatch,
            &memory,
            &report,
            &service,
        );
        let summary = snapshot.summary();

        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Repair);
        assert_eq!(summary.owners, 4);
        assert_eq!(summary.stable_owners, 2);
        assert_eq!(summary.watch_owners, 1);
        assert_eq!(summary.repair_owners, 1);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_dispatch_core());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(
            snapshot.blocked_reasons,
            vec![
                "service_adapter:reflection_watch",
                "eval_reporting:validation_evidence_missing=true"
            ]
        );
    }

    #[test]
    fn evolution_final_history_projects_stable_adapter_snapshot() {
        let summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let record = evolution_admission_handoff_history_for_adapter(
            summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );

        let gate = AgentAdapterBoundaryGate::from_evolution_admission_handoff_history(&record);
        let snapshot = AgentAdapterBoundarySnapshot::from_evolution_admission_handoff_history(
            &business_queue(),
            &record,
        );
        let summary = snapshot.summary();

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Stable);
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
        assert_eq!(summary.owners, 1);
        assert_eq!(summary.stable_owners, 1);
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
    }

    #[test]
    fn evolution_final_history_maps_memory_and_adaptive_promotions_independently() {
        let mut summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        summary.can_promote_adaptive_state = false;
        let record = evolution_admission_handoff_history_for_adapter(
            summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );

        let gate = AgentAdapterBoundaryGate::from_evolution_admission_handoff_history(&record);
        let snapshot = AgentAdapterBoundarySnapshot::from_evolution_admission_handoff_history(
            &business_queue(),
            &record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(record.is_effectively_admitted());
        assert!(record.can_promote_ready_proposals());
        assert!(record.can_promote_evolution_signals());
        assert!(record.can_reinforce_process());
        assert!(!record.can_promote_adaptive_state());
        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Watch);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Watch);
        assert!(snapshot.allows_service_advance());
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(!boundary_record.requires_repair_first());
        assert!(boundary_record.allows_service_advance());
        assert!(boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
    }

    #[test]
    fn evolution_final_history_projects_watch_adapter_snapshot_without_promotion() {
        let summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let policy = EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy {
            maximum_next_queue_tasks: 0,
            ..EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default()
        };
        let record = evolution_admission_handoff_history_for_adapter(summary, policy);

        let snapshot = AgentAdapterBoundarySnapshot::from_evolution_admission_handoff_history(
            &business_queue(),
            &record,
        );
        let summary = snapshot.summary();

        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Watch);
        assert!(snapshot.allows_service_advance());
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(summary.watch_owners, 1);
        assert_eq!(summary.repair_owners, 0);
        assert!(snapshot.blocked_reasons.iter().any(|reason| {
            reason.starts_with(
                "service_adapter:evolution_admission_handoff_history_health_reason=evolution_admission_handoff_trend_continuation_history_gate_next_queue_tasks=",
            )
        }));
    }

    #[test]
    fn evolution_final_history_projects_repair_adapter_handoff() {
        let summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Repair,
            false,
            true,
        );
        let record = evolution_admission_handoff_history_for_adapter(
            summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_evolution_admission_handoff_history_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert_eq!(
            handoff.boundary_record.snapshot.status(),
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(handoff.blocked_reasons.iter().any(|reason| {
            reason.starts_with(
                "service_adapter:evolution_admission_handoff_history_health_reason=evolution_admission_handoff_trend_continuation_history_gate_repair_first_records=",
            )
        }));
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(handoff.repair_tasks[0].id, "adapter-boundary-repair-0");
        assert_eq!(
            handoff.next_queue.task_ids().last(),
            Some(&"business-task".to_owned())
        );
    }

    #[test]
    fn adapter_handoff_history_recorder_records_evolution_final_history() {
        let stable_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let stable_record = evolution_admission_handoff_history_for_adapter(
            stable_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );

        let stable_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &stable_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        assert_eq!(stable_handoff_record.records(), 1);
        assert_eq!(
            stable_handoff_record.health.status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert_eq!(stable_handoff_record.dashboard.admitted_records, 1);
        assert_eq!(stable_handoff_record.dashboard.memory_promotable_records, 1);
        assert_eq!(
            stable_handoff_record.dashboard.adaptive_promotable_records,
            1
        );

        let repair_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Repair,
            false,
            true,
        );
        let repair_record = evolution_admission_handoff_history_for_adapter(
            repair_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );

        let repair_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &repair_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        assert_eq!(
            repair_handoff_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(repair_handoff_record.requires_repair_first());
        assert_eq!(repair_handoff_record.dashboard.admitted_records, 0);
        assert_eq!(repair_handoff_record.dashboard.repair_first_records, 1);
        assert!(repair_handoff_record.dashboard.repair_task_count > 0);
    }

    #[test]
    fn adapter_handoff_history_projects_eval_report_gate_decisions() {
        let stable_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let stable_record = evolution_admission_handoff_history_for_adapter(
            stable_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let stable_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &stable_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let stable_decision = stable_handoff_record.report_gate_decision("run/eval-adapter-stable");

        assert!(stable_decision.is_accepted());
        assert!(stable_decision.reasons.is_empty());
        assert!(stable_decision.follow_up_tasks.is_empty());

        let watch_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let watch_policy = EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy {
            maximum_next_queue_tasks: 0,
            ..EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default()
        };
        let watch_record =
            evolution_admission_handoff_history_for_adapter(watch_summary, watch_policy);
        let watch_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &watch_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let watch_decision = watch_handoff_record.report_gate_decision("run/eval-adapter-watch");
        let watch_codes = watch_decision
            .reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect::<Vec<_>>();

        assert!(!watch_decision.is_accepted());
        assert!(watch_decision.follow_up_tasks.is_empty());
        assert!(watch_codes.contains(&"adapter_boundary_handoff_memory_note_closed"));
        assert!(watch_codes.contains(&"adapter_boundary_handoff_adaptive_state_closed"));

        let repair_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Repair,
            false,
            true,
        );
        let repair_record = evolution_admission_handoff_history_for_adapter(
            repair_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let repair_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &repair_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let repair_decision = repair_handoff_record.report_gate_decision("run/eval-adapter-repair");
        let repair_codes = repair_decision
            .reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect::<Vec<_>>();

        assert!(!repair_decision.is_accepted());
        assert!(repair_codes.contains(&"adapter_boundary_handoff_repair_first"));
        assert_eq!(
            repair_decision.follow_up_tasks.len(),
            repair_decision.reasons.len()
        );
        assert!(
            repair_decision
                .follow_up_tasks
                .iter()
                .all(|task| task.lane == "eval-adapter-boundary" && task.priority == 8)
        );
        assert!(repair_decision.follow_up_tasks.iter().any(|task| {
            task.id
                .starts_with("adapter-boundary-eval-report-run-eval-adapter-repair")
        }));
    }

    #[test]
    fn adapter_handoff_history_records_eval_report_gate_health() {
        let stable_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let stable_record = evolution_admission_handoff_history_for_adapter(
            stable_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let stable_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &stable_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let stable_report_record = stable_handoff_record.record_report_gate_with_health(
            AgentReportGateSummaryHistory::new(),
            AgentReportGateHealthPolicy::default(),
            "run/eval-history-stable",
        );

        assert_eq!(stable_report_record.records(), 1);
        assert_eq!(
            stable_report_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(stable_report_record.dashboard.accepted_records, 1);
        assert_eq!(stable_report_record.dashboard.blocked_records, 0);
        assert!(stable_report_record.allows_service_advance());

        let watch_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let watch_policy = EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy {
            maximum_next_queue_tasks: 0,
            ..EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default()
        };
        let watch_record =
            evolution_admission_handoff_history_for_adapter(watch_summary, watch_policy);
        let watch_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &watch_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let watch_report_policy = AgentReportGateHealthPolicy {
            maximum_blocked_records: 1,
            maximum_reason_count: usize::MAX,
            maximum_follow_up_tasks: usize::MAX,
            maximum_review_blockers: usize::MAX,
            ..AgentReportGateHealthPolicy::default()
        };

        let watch_report_record = watch_handoff_record.record_report_gate_with_health(
            AgentReportGateSummaryHistory::new(),
            watch_report_policy,
            "run/eval-history-watch",
        );

        assert_eq!(
            watch_report_record.health.status,
            AgentReportGateHealthStatus::Watch
        );
        assert!(watch_report_record.allows_service_advance());
        assert!(!watch_report_record.requires_repair_first());
        assert_eq!(watch_report_record.dashboard.accepted_records, 0);
        assert_eq!(watch_report_record.dashboard.blocked_records, 1);

        let repair_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Repair,
            false,
            true,
        );
        let repair_record = evolution_admission_handoff_history_for_adapter(
            repair_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let repair_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &repair_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let repair_gate_record = repair_handoff_record.record_report_gate_with_health_gate(
            AgentReportGateSummaryHistory::new(),
            AgentReportGateHealthPolicy::default(),
            "run/eval-history-repair",
            &business_queue(),
        );

        assert_eq!(
            repair_gate_record.health_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(repair_gate_record.gate_decision.requires_repair_first);
        assert!(!repair_gate_record.is_admitted());
        assert!(!repair_gate_record.gate_decision.repair_tasks.is_empty());
        assert!(
            repair_gate_record
                .gate_decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
    }

    #[test]
    fn adapter_handoff_history_records_eval_report_gate_trend_handoff() {
        let stable_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let stable_record = evolution_admission_handoff_history_for_adapter(
            stable_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let stable_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &stable_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let stable_trend_handoff = stable_handoff_record.record_report_gate_trend_handoff(
            AgentReportGateSummaryHistory::new(),
            AgentReportGateHealthPolicy::default(),
            AgentReportGateHealthGateSummaryHistory::new(),
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/eval-trend-stable",
            &business_queue(),
        );

        assert!(stable_trend_handoff.is_admitted());
        assert_eq!(
            stable_trend_handoff.trend_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(stable_trend_handoff.trend_record.history.len(), 1);
        assert_eq!(
            stable_trend_handoff.handoff_summary.next_queue_task_ids,
            vec!["business-task"]
        );
        assert!(stable_trend_handoff.gate_decision.repair_tasks.is_empty());

        let repair_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Repair,
            false,
            true,
        );
        let repair_record = evolution_admission_handoff_history_for_adapter(
            repair_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let repair_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &repair_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let repair_trend_handoff = repair_handoff_record.record_report_gate_trend_handoff(
            AgentReportGateSummaryHistory::new(),
            AgentReportGateHealthPolicy::default(),
            AgentReportGateHealthGateSummaryHistory::new(),
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/eval-trend-repair",
            &business_queue(),
        );
        let repair_queue_ids = repair_trend_handoff.next_queue().task_ids();

        assert!(!repair_trend_handoff.is_admitted());
        assert_eq!(
            repair_trend_handoff.trend_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(repair_trend_handoff.gate_decision.requires_repair_first);
        assert!(!repair_trend_handoff.gate_decision.repair_tasks.is_empty());
        assert!(repair_queue_ids.first().is_some_and(|id| {
            id.starts_with("agent-report-gate-health-gate-trend-repair-run-eval-trend-repair")
        }));
        assert!(repair_queue_ids.iter().any(|id| id == "business-task"));
        assert_eq!(
            repair_trend_handoff.handoff_summary.repair_tasks,
            repair_trend_handoff.gate_decision.repair_tasks.len()
        );
    }

    #[test]
    fn adapter_handoff_history_records_eval_report_gate_trend_handoff_monitor() {
        let stable_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let stable_record = evolution_admission_handoff_history_for_adapter(
            stable_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let stable_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &stable_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let stable_monitor = stable_handoff_record.record_report_gate_trend_handoff_monitor(
            AgentReportGateSummaryHistory::new(),
            AgentReportGateHealthPolicy::default(),
            AgentReportGateHealthGateSummaryHistory::new(),
            AgentReportGateHealthGateHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/eval-monitor-stable",
            &business_queue(),
        );

        assert!(stable_monitor.is_admitted());
        assert_eq!(
            stable_monitor.history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(stable_monitor.history_record.records(), 1);
        assert_eq!(
            stable_monitor.next_queue().task_ids(),
            vec!["business-task"]
        );
        assert!(!stable_monitor.gate_decision.requires_repair_first);

        let repair_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Repair,
            false,
            true,
        );
        let repair_record = evolution_admission_handoff_history_for_adapter(
            repair_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let repair_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &repair_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let repair_monitor = repair_handoff_record.record_report_gate_trend_handoff_monitor(
            AgentReportGateSummaryHistory::new(),
            AgentReportGateHealthPolicy::default(),
            AgentReportGateHealthGateSummaryHistory::new(),
            AgentReportGateHealthGateHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/eval-monitor-repair",
            &business_queue(),
        );
        let repair_queue_ids = repair_monitor.next_queue().task_ids();

        assert!(!repair_monitor.is_admitted());
        assert_eq!(
            repair_monitor.history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(repair_monitor.gate_decision.requires_repair_first);
        assert!(repair_queue_ids.first().is_some_and(|id| {
            id.starts_with(
                "agent-report-gate-health-gate-trend-handoff-repair-run-eval-monitor-repair",
            )
        }));
        assert!(repair_queue_ids.iter().any(|id| id == "business-task"));
        assert_eq!(
            repair_monitor.summary().repair_tasks,
            repair_monitor.gate_decision.repair_tasks.len()
        );
    }

    #[test]
    fn adapter_handoff_history_records_eval_report_gate_trend_handoff_monitor_handoff() {
        let stable_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let stable_record = evolution_admission_handoff_history_for_adapter(
            stable_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let stable_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &stable_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let stable_packet = stable_handoff_record.record_report_gate_trend_handoff_monitor_handoff(
            AgentReportGateSummaryHistory::new(),
            AgentReportGateHealthPolicy::default(),
            AgentReportGateHealthGateSummaryHistory::new(),
            AgentReportGateHealthGateHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/eval-monitor-handoff-stable",
            &business_queue(),
        );

        assert!(stable_packet.is_admitted());
        assert_eq!(
            stable_packet.history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(stable_packet.history_record.records(), 1);
        assert_eq!(stable_packet.next_queue().task_ids(), vec!["business-task"]);
        assert!(!stable_packet.gate_decision.requires_repair_first);

        let repair_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Repair,
            false,
            true,
        );
        let repair_record = evolution_admission_handoff_history_for_adapter(
            repair_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let repair_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &repair_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let repair_packet = repair_handoff_record.record_report_gate_trend_handoff_monitor_handoff(
            AgentReportGateSummaryHistory::new(),
            AgentReportGateHealthPolicy::default(),
            AgentReportGateHealthGateSummaryHistory::new(),
            AgentReportGateHealthGateHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/eval-monitor-handoff-repair",
            &business_queue(),
        );
        let repair_queue_ids = repair_packet.next_queue().task_ids();

        assert!(!repair_packet.is_admitted());
        assert_eq!(
            repair_packet.history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(repair_packet.gate_decision.requires_repair_first);
        assert!(repair_queue_ids.iter().any(|id| {
            id.starts_with(
                "agent-report-gate-health-gate-trend-handoff-monitor-repair-run-eval-monitor-handoff-repair",
            )
        }));
        assert!(repair_queue_ids.iter().any(|id| id == "business-task"));
        assert_eq!(
            repair_packet.summary().repair_tasks,
            repair_packet.gate_decision.repair_tasks.len()
        );
        assert!(repair_packet.monitor.gate_decision.requires_repair_first);
    }

    #[test]
    fn adapter_handoff_history_records_eval_report_gate_trend_handoff_monitor_handoff_handoff() {
        let stable_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let stable_record = evolution_admission_handoff_history_for_adapter(
            stable_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let stable_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &stable_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let stable_packet = stable_handoff_record
            .record_report_gate_trend_handoff_monitor_handoff_handoff(
                AgentReportGateSummaryHistory::new(),
                AgentReportGateHealthPolicy::default(),
                AgentReportGateHealthGateSummaryHistory::new(),
                AgentReportGateHealthGateHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffHistory::new(),
                AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/eval-monitor-handoff-handoff-stable",
                &business_queue(),
            );

        assert!(stable_packet.is_admitted());
        assert_eq!(
            stable_packet.history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(stable_packet.history_record.records(), 1);
        assert_eq!(stable_packet.next_queue().task_ids(), vec!["business-task"]);
        assert!(!stable_packet.gate_decision.requires_repair_first);

        let repair_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Repair,
            false,
            true,
        );
        let repair_record = evolution_admission_handoff_history_for_adapter(
            repair_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let repair_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &repair_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let repair_packet = repair_handoff_record
            .record_report_gate_trend_handoff_monitor_handoff_handoff(
                AgentReportGateSummaryHistory::new(),
                AgentReportGateHealthPolicy::default(),
                AgentReportGateHealthGateSummaryHistory::new(),
                AgentReportGateHealthGateHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffHistory::new(),
                AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/eval-monitor-handoff-handoff-repair",
                &business_queue(),
            );
        let repair_queue_ids = repair_packet.next_queue().task_ids();

        assert!(!repair_packet.is_admitted());
        assert_eq!(
            repair_packet.history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(repair_packet.gate_decision.requires_repair_first);
        assert!(repair_queue_ids.iter().any(|id| {
            id.starts_with(
                "agent-report-gate-health-gate-trend-handoff-monitor-handoff-repair-run-eval-monitor-handoff-handoff-repair",
            )
        }));
        assert!(repair_queue_ids.iter().any(|id| id == "business-task"));
        assert_eq!(
            repair_packet.summary().repair_tasks,
            repair_packet.gate_decision.repair_tasks.len()
        );
        assert!(repair_packet.handoff.gate_decision.requires_repair_first);
    }

    #[test]
    fn adapter_handoff_history_records_eval_report_gate_trend_handoff_monitor_handoff_handoff_handoff()
     {
        let stable_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let stable_record = evolution_admission_handoff_history_for_adapter(
            stable_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let stable_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &stable_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let stable_packet = stable_handoff_record
            .record_report_gate_trend_handoff_monitor_handoff_handoff_handoff(
                AgentReportGateSummaryHistory::new(),
                AgentReportGateHealthPolicy::default(),
                AgentReportGateHealthGateSummaryHistory::new(),
                AgentReportGateHealthGateHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffHistory::new(),
                AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/eval-final-admission-stable",
                &business_queue(),
            );

        assert!(stable_packet.is_admitted());
        assert_eq!(
            stable_packet.history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(stable_packet.history_record.records(), 1);
        assert_eq!(stable_packet.next_queue().task_ids(), vec!["business-task"]);
        assert!(!stable_packet.gate_decision.requires_repair_first);

        let repair_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Repair,
            false,
            true,
        );
        let repair_record = evolution_admission_handoff_history_for_adapter(
            repair_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let repair_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &repair_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let repair_packet = repair_handoff_record
            .record_report_gate_trend_handoff_monitor_handoff_handoff_handoff(
                AgentReportGateSummaryHistory::new(),
                AgentReportGateHealthPolicy::default(),
                AgentReportGateHealthGateSummaryHistory::new(),
                AgentReportGateHealthGateHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffHistory::new(),
                AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/eval-final-admission-repair",
                &business_queue(),
            );
        let repair_queue_ids = repair_packet.next_queue().task_ids();

        assert!(!repair_packet.is_admitted());
        assert_eq!(
            repair_packet.history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(repair_packet.gate_decision.requires_repair_first);
        assert!(repair_queue_ids.iter().any(|id| {
            id.starts_with(
                "agent-report-gate-health-gate-trend-handoff-monitor-handoff-handoff-repair-run-eval-final-admission-repair",
            )
        }));
        assert!(repair_queue_ids.iter().any(|id| id == "business-task"));
        assert_eq!(
            repair_packet.summary().repair_tasks,
            repair_packet.gate_decision.repair_tasks.len()
        );
        assert!(repair_packet.packet.gate_decision.requires_repair_first);
    }

    #[test]
    fn adapter_handoff_history_records_eval_report_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff()
     {
        let stable_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Stable,
            true,
            false,
        );
        let stable_record = evolution_admission_handoff_history_for_adapter(
            stable_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let stable_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &stable_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let stable_packet = stable_handoff_record
            .record_report_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff(
            AgentReportGateSummaryHistory::new(),
            AgentReportGateHealthPolicy::default(),
            AgentReportGateHealthGateSummaryHistory::new(),
            AgentReportGateHealthGateHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(
            ),
            "run/eval-final-handoff-stable",
            &business_queue(),
        );

        assert!(stable_packet.is_admitted());
        assert_eq!(
            stable_packet.history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(stable_packet.history_record.records(), 1);
        assert_eq!(stable_packet.next_queue().task_ids(), vec!["business-task"]);
        assert!(!stable_packet.gate_decision.requires_repair_first);

        let repair_summary = evolution_admission_handoff_summary_for_adapter(
            EvolutionAdmissionHealthStatus::Repair,
            false,
            true,
        );
        let repair_record = evolution_admission_handoff_history_for_adapter(
            repair_summary,
            EvolutionAdmissionHandoffTrendContinuationHistoryGateHealthPolicy::default(),
        );
        let repair_handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_evolution_admission_handoff_history_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &repair_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let repair_packet = repair_handoff_record
            .record_report_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff(
            AgentReportGateSummaryHistory::new(),
            AgentReportGateHealthPolicy::default(),
            AgentReportGateHealthGateSummaryHistory::new(),
            AgentReportGateHealthGateHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(
            ),
            "run/eval-final-handoff-repair",
            &business_queue(),
        );
        let repair_queue_ids = repair_packet.next_queue().task_ids();

        assert!(!repair_packet.is_admitted());
        assert_eq!(
            repair_packet.history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(repair_packet.gate_decision.requires_repair_first);
        assert!(repair_queue_ids.iter().any(|id| {
            id.starts_with(
                "agent-report-gate-health-gate-trend-handoff-monitor-handoff-handoff-handoff-repair-run-eval-final-handoff-repair",
            )
        }));
        assert!(repair_queue_ids.iter().any(|id| id == "business-task"));
        assert_eq!(
            repair_packet.summary().repair_tasks,
            repair_packet.gate_decision.repair_tasks.len()
        );
        assert!(repair_packet.admission.gate_decision.requires_repair_first);
    }

    #[test]
    fn boundary_snapshot_composes_watch_outputs_without_promoting_memory() {
        let dispatch = clean_dispatch_gate();
        let memory = memory_submission_gate(false);
        let report = accepted_report_gate();
        let service = service_admission(
            AgentClosedLoopNextTurnMode::Observe,
            true,
            false,
            "service_observe_only",
        );

        let snapshot = AgentAdapterBoundarySnapshot::from_boundary_gates(
            &business_queue(),
            &dispatch,
            &memory,
            &report,
            &service,
        );
        let summary = snapshot.summary();

        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Watch);
        assert!(snapshot.allows_service_advance());
        assert!(snapshot.can_dispatch_core());
        assert!(snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(summary.owners, 4);
        assert_eq!(summary.stable_owners, 2);
        assert_eq!(summary.watch_owners, 2);
        assert_eq!(summary.repair_owners, 0);
        assert_eq!(
            snapshot.blocked_reasons,
            vec![
                "norion_memory:memory_submission_observe_only",
                "service_adapter:service_observe_only"
            ]
        );
    }

    #[test]
    fn boundary_record_keeps_clean_snapshot_closed_when_history_repairs() {
        let dirty_summary = AgentAdapterBoundarySnapshot::from_gates(
            &business_queue(),
            vec![AgentAdapterBoundaryGate::repair(
                AgentAdapterBoundaryOwner::EvalReporting,
                "prior_report_gate_rejected",
            )],
        )
        .summary();
        let history = AgentAdapterBoundarySummaryHistory::from_summaries(vec![dirty_summary]);
        let dispatch = clean_dispatch_gate();
        let memory = memory_submission_gate(true);
        let report = accepted_report_gate();
        let service = service_admission(
            AgentClosedLoopNextTurnMode::Continue,
            true,
            true,
            "stable_service",
        );

        let record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_boundary_gates_with_health(
                history,
                &business_queue(),
                &dispatch,
                &memory,
                &report,
                &service,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert_eq!(record.records(), 2);
        assert_eq!(record.snapshot.status(), AgentAdapterBoundaryStatus::Stable);
        assert_eq!(record.summary().status, AgentAdapterBoundaryStatus::Stable);
        assert_eq!(
            record.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(record.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.snapshot.can_submit_memory_note());
        assert!(record.snapshot.can_promote_adaptive_state());
        assert!(!record.can_submit_memory_note());
        assert!(!record.can_promote_adaptive_state());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_adapter_boundary_record_health_status=repair" })
        );
    }

    #[test]
    fn boundary_handoff_merges_repair_tasks_before_adapter_advance() {
        let history = AgentAdapterBoundarySummaryHistory::new();
        let dispatch = clean_dispatch_gate();
        let memory = memory_submission_gate(true);
        let report = rejected_report_gate();
        let service = service_admission(
            AgentClosedLoopNextTurnMode::Observe,
            true,
            false,
            "reflection_watch",
        );

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_boundary_gates_handoff_with_health(
                history,
                &business_queue(),
                &dispatch,
                &memory,
                &report,
                &service,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert_eq!(
            handoff.blocked_reasons,
            vec![
                "service_adapter:reflection_watch",
                "eval_reporting:validation_evidence_missing=true",
                "history:adapter_boundary_repair_records=1>0",
                "history:adapter_boundary_repair_first_records=1>0",
                "history:adapter_boundary_stable_rate=0.000<0.67",
            ]
        );
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(handoff.repair_tasks[0].id, "adapter-boundary-repair-0");
        assert_eq!(handoff.repair_tasks[0].role, AgentRole::Tester);
        assert_eq!(handoff.repair_tasks[1].role, AgentRole::Reviewer);
        assert!(
            handoff
                .repair_tasks
                .iter()
                .all(|task| task.lane == "adapter-boundary-repair" && task.priority == 1)
        );
        assert_eq!(
            handoff.next_queue.task_ids(),
            vec![
                "adapter-boundary-repair-0",
                "adapter-boundary-repair-1",
                "adapter-boundary-repair-2",
                "adapter-boundary-repair-3",
                "adapter-boundary-repair-4",
                "business-task",
            ]
        );
        assert!(
            handoff
                .telemetry
                .iter()
                .any(|line| { line == "agent_adapter_boundary_handoff_repair_tasks=5" })
        );

        let summary = handoff.summary();

        assert_eq!(summary.snapshot_status, AgentAdapterBoundaryStatus::Repair);
        assert_eq!(summary.health_status, AgentAdapterBoundaryStatus::Repair);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.repair_tasks, 5);
        assert_eq!(summary.next_queue_tasks, 6);
        assert_eq!(
            summary.repair_task_ids,
            vec![
                "adapter-boundary-repair-0",
                "adapter-boundary-repair-1",
                "adapter-boundary-repair-2",
                "adapter-boundary-repair-3",
                "adapter-boundary-repair-4",
            ]
        );
        assert_eq!(
            summary.next_queue_task_ids,
            vec![
                "adapter-boundary-repair-0",
                "adapter-boundary-repair-1",
                "adapter-boundary-repair-2",
                "adapter-boundary-repair-3",
                "adapter-boundary-repair-4",
                "business-task",
            ]
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_adapter_boundary_handoff_summary_repair_tasks=5" })
        );
    }

    #[test]
    fn boundary_handoff_summary_history_is_stable_readiness_report_surface() {
        let stable_summary = stable_handoff().summary();
        let repair_handoff = repair_handoff();
        let repair_summary = repair_handoff.summary();
        let history = AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
            stable_summary.clone(),
            repair_summary.clone(),
        ]);
        let dashboard = history.dashboard();
        let health = history.health(AgentAdapterBoundaryHandoffHealthPolicy::default());
        let report_decision = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &repair_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            )
            .report_gate_decision("main-window/readiness");
        let report_reason_codes = report_decision
            .reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect::<Vec<_>>();
        let repair_prefix = repair_summary.repair_tasks;

        assert_eq!(dashboard.total_records, 2);
        assert_eq!(dashboard.admitted_records, 1);
        assert_eq!(dashboard.repair_first_records, 1);
        assert_eq!(dashboard.memory_promotable_records, 1);
        assert_eq!(dashboard.adaptive_promotable_records, 1);
        assert_eq!(dashboard.repair_task_count, repair_summary.repair_tasks);
        assert_eq!(
            dashboard.next_queue_tasks,
            stable_summary.next_queue_tasks + repair_summary.next_queue_tasks
        );
        assert_eq!(dashboard.blocked_reasons, repair_summary.blocked_reasons);
        assert_eq!(
            dashboard.latest_snapshot_status,
            Some(AgentAdapterBoundaryStatus::Repair)
        );
        assert_eq!(
            dashboard.latest_health_status,
            Some(AgentAdapterBoundaryStatus::Repair)
        );
        assert_eq!(dashboard.admitted_rate, 0.5);
        assert_eq!(dashboard.repair_first_rate, 0.5);
        assert_eq!(health.status, AgentAdapterBoundaryStatus::Repair);
        assert_eq!(
            health.reasons,
            vec![
                "adapter_boundary_handoff_repair_first_records=1>0",
                "adapter_boundary_handoff_repair_tasks=5>0",
                "adapter_boundary_handoff_admitted_rate=0.500<0.67",
            ]
        );
        assert_eq!(
            repair_summary.repair_task_ids.len(),
            repair_summary.repair_tasks
        );
        assert!(
            repair_summary
                .repair_task_ids
                .iter()
                .all(|task_id| task_id.starts_with("adapter-boundary-repair-"))
        );
        assert_eq!(
            &repair_summary.next_queue_task_ids[..repair_prefix],
            repair_summary.repair_task_ids.as_slice()
        );
        assert_eq!(
            repair_summary
                .next_queue_task_ids
                .last()
                .map(String::as_str),
            Some("business-task")
        );
        assert!(report_reason_codes.contains(&"adapter_boundary_handoff_repair_first"));
        assert!(report_reason_codes.contains(&"adapter_boundary_handoff_repair_tasks"));
        assert_eq!(
            report_decision.follow_up_tasks.len(),
            report_decision.reasons.len()
        );
        assert!(report_decision.follow_up_tasks.iter().all(|task| {
            task.lane == "eval-adapter-boundary"
                && task
                    .id
                    .starts_with("adapter-boundary-eval-report-main-window-readiness")
        }));
    }

    #[test]
    fn boundary_handoff_history_watches_empty_records() {
        let health = AgentAdapterBoundaryHandoffSummaryHistory::new()
            .health(AgentAdapterBoundaryHandoffHealthPolicy::default());

        assert_eq!(health.status, AgentAdapterBoundaryStatus::Watch);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["adapter_boundary_handoff_history_empty"]
        );
    }

    #[test]
    fn boundary_handoff_history_marks_clean_handoff_stable() {
        let handoff = stable_handoff();
        let record = AgentAdapterBoundaryHandoffHistoryRecorder::new().record_handoff_with_health(
            AgentAdapterBoundaryHandoffSummaryHistory::new(),
            &handoff,
            AgentAdapterBoundaryHandoffHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, AgentAdapterBoundaryStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.admitted_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert_eq!(record.dashboard.repair_task_count, 0);
        assert_eq!(record.dashboard.memory_promotable_records, 1);
        assert_eq!(record.dashboard.adaptive_promotable_records, 1);
        assert_eq!(
            record.dashboard.latest_snapshot_status,
            Some(AgentAdapterBoundaryStatus::Stable)
        );
        assert_eq!(
            record.dashboard.latest_health_status,
            Some(AgentAdapterBoundaryStatus::Stable)
        );
        assert!(
            record.telemetry.iter().any(|line| {
                line == "agent_adapter_boundary_handoff_history_record_status=stable"
            })
        );
    }

    #[test]
    fn boundary_handoff_history_repairs_repair_first_trends() {
        let handoff = repair_handoff();
        let record = AgentAdapterBoundaryHandoffHistoryRecorder::new().record_handoff_with_health(
            AgentAdapterBoundaryHandoffSummaryHistory::new(),
            &handoff,
            AgentAdapterBoundaryHandoffHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(record.dashboard.admitted_records, 0);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.repair_task_count, 5);
        assert_eq!(record.dashboard.blocked_reasons, 5);
        assert_eq!(
            record.health.reasons,
            vec![
                "adapter_boundary_handoff_repair_first_records=1>0",
                "adapter_boundary_handoff_repair_tasks=5>0",
                "adapter_boundary_handoff_admitted_rate=0.000<0.67",
            ]
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_history_record_repair_tasks=5"
        }));
    }

    #[test]
    fn boundary_handoff_trend_gate_preserves_stable_history() {
        let handoff = stable_handoff();
        let history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let decision = AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &history_record);

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert!(!decision.requires_repair_first);
        assert!(decision.can_submit_memory_note);
        assert!(decision.can_promote_adaptive_state);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(
            decision
                .telemetry
                .iter()
                .any(|line| { line == "agent_adapter_boundary_handoff_trend_gate_effective=true" })
        );
    }

    #[test]
    fn boundary_handoff_trend_gate_observes_watch_history() {
        let handoff = stable_handoff();
        let watch_history = AgentAdapterBoundaryHandoffSummaryHistory::new();
        let watch_record = AgentAdapterBoundaryHandoffHistoryRecord {
            history: watch_history.clone(),
            appended_summary: handoff.summary(),
            dashboard: watch_history.dashboard(),
            health: watch_history.health(AgentAdapterBoundaryHandoffHealthPolicy::default()),
            telemetry: Vec::new(),
        };

        let decision = AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &watch_record);

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(
            decision.blocked_reasons,
            vec!["handoff_history:adapter_boundary_handoff_history_empty"]
        );
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(!decision.can_submit_memory_note);
        assert!(!decision.can_promote_adaptive_state);
    }

    #[test]
    fn boundary_handoff_trend_gate_blocks_repair_history() {
        let repair_summary = repair_handoff().summary();
        let history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![repair_summary]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let handoff = stable_handoff();

        let decision = AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &history_record);

        assert!(decision.requested_admitted);
        assert!(!decision.is_admitted());
        assert!(decision.requires_repair_first);
        assert!(!decision.can_submit_memory_note);
        assert!(!decision.can_promote_adaptive_state);
        assert_eq!(
            decision.handoff_health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert_eq!(
            decision.blocked_reasons,
            vec![
                "handoff_history:adapter_boundary_handoff_repair_first_records=1>0",
                "handoff_history:adapter_boundary_handoff_repair_tasks=5>0",
                "handoff_history:adapter_boundary_handoff_admitted_rate=0.500<0.67",
            ]
        );
        assert_eq!(decision.repair_tasks.len(), 3);
        assert_eq!(
            decision
                .repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "adapter-boundary-handoff-trend-repair-0",
                "adapter-boundary-handoff-trend-repair-1",
                "adapter-boundary-handoff-trend-repair-2",
            ]
        );
        assert_eq!(
            decision.next_queue.task_ids(),
            vec![
                "adapter-boundary-handoff-trend-repair-0",
                "adapter-boundary-handoff-trend-repair-1",
                "adapter-boundary-handoff-trend-repair-2",
                "business-task",
            ]
        );
        assert!(
            decision
                .telemetry
                .iter()
                .any(|line| { line == "agent_adapter_boundary_handoff_trend_gate_repair_tasks=3" })
        );
    }

    #[test]
    fn boundary_handoff_trend_gate_summary_compacts_repair_decision() {
        let repair_summary = repair_handoff().summary();
        let history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![repair_summary]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let handoff = stable_handoff();
        let decision = AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &history_record);

        let summary = decision.summary();

        assert_eq!(
            summary.handoff_health_status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(summary.requested_admitted);
        assert!(!summary.effective_admitted);
        assert!(summary.requires_repair_first);
        assert!(!summary.can_submit_memory_note);
        assert!(!summary.can_promote_adaptive_state);
        assert_eq!(summary.repair_tasks, 3);
        assert_eq!(summary.next_queue_tasks, 4);
        assert_eq!(summary.blocked_reasons, 3);
        assert_eq!(
            summary.repair_task_ids,
            vec![
                "adapter-boundary-handoff-trend-repair-0",
                "adapter-boundary-handoff-trend-repair-1",
                "adapter-boundary-handoff-trend-repair-2",
            ]
        );
        assert_eq!(
            summary.next_queue_task_ids,
            vec![
                "adapter-boundary-handoff-trend-repair-0",
                "adapter-boundary-handoff-trend-repair-1",
                "adapter-boundary-handoff-trend-repair-2",
                "business-task",
            ]
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_gate_summary_repair_tasks=3"
        }));
    }

    #[test]
    fn boundary_handoff_trend_gate_history_watches_empty_records() {
        let health = AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new()
            .health(AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default());

        assert_eq!(health.status, AgentAdapterBoundaryStatus::Watch);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["adapter_boundary_handoff_trend_gate_history_empty"]
        );
    }

    #[test]
    fn boundary_handoff_trend_gate_history_marks_clean_decision_stable() {
        let handoff = stable_handoff();
        let history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let decision = AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &history_record);

        let record = AgentAdapterBoundaryHandoffTrendGateHistoryRecorder::new()
            .record_decision_with_health(
                AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
                &decision,
                AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, AgentAdapterBoundaryStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.requested_admitted_records, 1);
        assert_eq!(record.dashboard.effective_admitted_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert_eq!(record.dashboard.repair_task_count, 0);
        assert_eq!(
            record.dashboard.latest_handoff_health_status,
            Some(AgentAdapterBoundaryStatus::Stable)
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_gate_history_record_status=stable"
        }));
    }

    #[test]
    fn boundary_handoff_trend_gate_history_repairs_blocked_decisions() {
        let repair_summary = repair_handoff().summary();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![repair_summary]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &handoff_history_record);

        let record = AgentAdapterBoundaryHandoffTrendGateHistoryRecorder::new()
            .record_decision_with_health(
                AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
                &decision,
                AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(record.dashboard.requested_admitted_records, 1);
        assert_eq!(record.dashboard.effective_admitted_records, 0);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.repair_task_count, 3);
        assert_eq!(
            record.health.reasons,
            vec![
                "adapter_boundary_handoff_trend_gate_repair_first_records=1>0",
                "adapter_boundary_handoff_trend_gate_repair_tasks=3>0",
                "adapter_boundary_handoff_trend_gate_effective_admitted_rate=0.000<0.67",
            ]
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_gate_history_record_repair_tasks=3"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_preserves_stable_queue() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );

        assert_eq!(admission.records(), 1);
        assert_eq!(
            admission.history_record.health.status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert!(admission.is_admitted());
        assert!(admission.allows_service_advance());
        assert!(admission.can_submit_memory_note);
        assert!(admission.can_promote_adaptive_state);
        assert!(!admission.requires_repair_first());
        assert!(admission.history_repair_tasks.is_empty());
        assert_eq!(admission.next_queue.task_ids(), vec!["business-task"]);
        assert!(admission.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_closes_memory_on_watch_history() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_history =
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                non_effective_trend_summary(),
            ]);

        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_history_record,
            trend_history,
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );

        assert_eq!(admission.records(), 2);
        assert_eq!(
            admission.history_record.health.status,
            AgentAdapterBoundaryStatus::Watch
        );
        assert!(admission.is_admitted());
        assert!(admission.allows_service_advance());
        assert!(!admission.can_submit_memory_note);
        assert!(!admission.can_promote_adaptive_state);
        assert!(!admission.requires_repair_first());
        assert!(admission.history_repair_tasks.is_empty());
        assert_eq!(admission.next_queue.task_ids(), vec!["business-task"]);
        assert_eq!(
            admission.blocked_reasons,
            vec![
                "trend_gate_history:adapter_boundary_handoff_trend_gate_effective_admitted_rate=0.500<0.67"
            ]
        );
    }

    #[test]
    fn boundary_handoff_trend_admission_appends_repair_tasks_on_repair_history() {
        let repair_handoff_summary = repair_handoff().summary();
        let repair_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
                    repair_handoff_summary,
                ]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let repair_decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &repair_handoff_history_record);
        let clean_handoff = stable_handoff();
        let clean_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &clean_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_history =
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                repair_decision.summary(),
            ]);

        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &clean_handoff,
            &clean_handoff_history_record,
            trend_history,
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );

        assert_eq!(admission.records(), 2);
        assert_eq!(
            admission.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(!admission.is_admitted());
        assert!(!admission.allows_service_advance());
        assert!(admission.requires_repair_first());
        assert!(!admission.can_submit_memory_note);
        assert!(!admission.can_promote_adaptive_state);
        assert!(admission.decision.repair_tasks.is_empty());
        assert_eq!(admission.history_repair_tasks.len(), 3);
        assert_eq!(
            admission.next_queue.task_ids(),
            vec![
                "adapter-boundary-handoff-trend-gate-repair-0",
                "adapter-boundary-handoff-trend-gate-repair-1",
                "adapter-boundary-handoff-trend-gate-repair-2",
                "business-task",
            ]
        );
        assert!(admission.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_history_repair_tasks=3"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_summary_compacts_final_packet() {
        let repair_handoff_summary = repair_handoff().summary();
        let repair_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
                    repair_handoff_summary,
                ]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let repair_decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &repair_handoff_history_record);
        let clean_handoff = stable_handoff();
        let clean_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &clean_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_history =
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                repair_decision.summary(),
            ]);
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &clean_handoff,
            &clean_handoff_history_record,
            trend_history,
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );

        let summary = admission.summary();

        assert_eq!(
            summary.trend_health_status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(summary.requested_admitted);
        assert!(!summary.effective_admitted);
        assert!(summary.requires_repair_first);
        assert!(!summary.can_submit_memory_note);
        assert!(!summary.can_promote_adaptive_state);
        assert_eq!(summary.decision_repair_tasks, 0);
        assert_eq!(summary.history_repair_tasks, 3);
        assert_eq!(summary.next_queue_tasks, 4);
        assert_eq!(summary.blocked_reasons, 3);
        assert_eq!(summary.records, 2);
        assert_eq!(
            summary.history_repair_task_ids,
            vec![
                "adapter-boundary-handoff-trend-gate-repair-0",
                "adapter-boundary-handoff-trend-gate-repair-1",
                "adapter-boundary-handoff-trend-gate-repair-2",
            ]
        );
        assert_eq!(
            summary.next_queue_task_ids,
            vec![
                "adapter-boundary-handoff-trend-gate-repair-0",
                "adapter-boundary-handoff-trend-gate-repair-1",
                "adapter-boundary-handoff-trend-gate-repair-2",
                "business-task",
            ]
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_summary_history_repair_tasks=3"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_history_watches_empty_records() {
        let health = AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new()
            .health(AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default());

        assert_eq!(health.status, AgentAdapterBoundaryStatus::Watch);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["adapter_boundary_handoff_trend_admission_history_empty"]
        );
    }

    #[test]
    fn boundary_handoff_trend_admission_history_records_stable_final_packet() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );

        let record = AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
            .record_admission_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                &admission,
                AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, AgentAdapterBoundaryStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.requested_admitted_records, 1);
        assert_eq!(record.dashboard.effective_admitted_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert_eq!(record.dashboard.memory_promotable_records, 1);
        assert_eq!(record.dashboard.adaptive_promotable_records, 1);
        assert_eq!(record.dashboard.history_repair_task_count, 0);
        assert_eq!(
            record.dashboard.latest_trend_health_status,
            Some(AgentAdapterBoundaryStatus::Stable)
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_history_record_status=stable"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_history_repairs_dirty_final_packets() {
        let repair_handoff_summary = repair_handoff().summary();
        let repair_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
                    repair_handoff_summary,
                ]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let repair_decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &repair_handoff_history_record);
        let clean_handoff = stable_handoff();
        let clean_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &clean_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_history =
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                repair_decision.summary(),
            ]);
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &clean_handoff,
            &clean_handoff_history_record,
            trend_history,
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );

        let record = AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
            .record_admission_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                &admission,
                AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(record.dashboard.requested_admitted_records, 1);
        assert_eq!(record.dashboard.effective_admitted_records, 0);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.decision_repair_task_count, 0);
        assert_eq!(record.dashboard.history_repair_task_count, 3);
        assert_eq!(record.dashboard.blocked_reasons, 3);
        assert_eq!(
            record.health.reasons,
            vec![
                "adapter_boundary_handoff_trend_admission_repair_first_records=1>0",
                "adapter_boundary_handoff_trend_admission_history_repair_tasks=3>0",
                "adapter_boundary_handoff_trend_admission_effective_admitted_rate=0.000<0.67",
            ]
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_history_record_history_repair_tasks=3"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_monitor_records_stable_boundary() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert!(record.is_admitted());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(
            record.history_record.health.status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_monitor_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_monitor_repairs_dirty_history() {
        let repair_handoff_summary = repair_handoff().summary();
        let repair_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
                    repair_handoff_summary,
                ]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let repair_decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &repair_handoff_history_record);
        let clean_handoff = stable_handoff();
        let clean_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &clean_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        let record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &clean_handoff,
            &clean_handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                repair_decision.summary(),
            ]),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert!(!record.is_admitted());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert_eq!(
            record.next_queue().task_ids(),
            vec![
                "adapter-boundary-handoff-trend-gate-repair-0",
                "adapter-boundary-handoff-trend-gate-repair-1",
                "adapter-boundary-handoff-trend-gate-repair-2",
                "business-task",
            ]
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_monitor_history_repair_tasks=3"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_continuation_packages_stable_state() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );

        let continuation = AgentAdapterBoundaryHandoffTrendAdmissionContinuationPlanner::new()
            .plan(&record, trend_policy, admission_policy);

        assert!(continuation.is_admitted());
        assert!(continuation.can_submit_memory_note);
        assert!(continuation.can_promote_adaptive_state);
        assert!(!continuation.requires_repair_first);
        assert_eq!(
            continuation.trend_health_status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert_eq!(
            continuation.admission_health_status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert_eq!(continuation.next_queue.task_ids(), vec!["business-task"]);
        assert_eq!(continuation.trend_history.len(), 1);
        assert_eq!(continuation.admission_history.len(), 1);
        assert_eq!(continuation.trend_policy, trend_policy);
        assert_eq!(continuation.admission_policy, admission_policy);
        assert!(continuation.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_continuation_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_continuation_preserves_repair_state() {
        let repair_handoff_summary = repair_handoff().summary();
        let repair_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
                    repair_handoff_summary,
                ]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let repair_decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &repair_handoff_history_record);
        let clean_handoff = stable_handoff();
        let clean_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &clean_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &clean_handoff,
            &clean_handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                repair_decision.summary(),
            ]),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );

        let continuation = record.continuation(trend_policy, admission_policy);

        assert!(!continuation.is_admitted());
        assert!(!continuation.can_submit_memory_note);
        assert!(!continuation.can_promote_adaptive_state);
        assert!(continuation.requires_repair_first);
        assert_eq!(
            continuation.trend_health_status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert_eq!(
            continuation.admission_health_status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert_eq!(
            continuation.next_queue.task_ids(),
            vec![
                "adapter-boundary-handoff-trend-gate-repair-0",
                "adapter-boundary-handoff-trend-gate-repair-1",
                "adapter-boundary-handoff-trend-gate-repair-2",
                "business-task",
            ]
        );
        assert_eq!(continuation.trend_history.len(), 2);
        assert_eq!(continuation.admission_history.len(), 1);
        assert!(continuation.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_continuation_repair_first=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_plan_carries_stable_histories_forward() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);

        let resume_plan =
            AgentAdapterBoundaryHandoffTrendAdmissionResumePlanner::new().plan(&continuation);
        let second_record = resume_plan.monitor_next(&handoff, &handoff_history_record);

        assert_eq!(resume_plan.prior_queue.task_ids(), vec!["business-task"]);
        assert_eq!(resume_plan.trend_history.len(), 1);
        assert_eq!(resume_plan.admission_history.len(), 1);
        assert_eq!(second_record.admission.history_record.records(), 2);
        assert_eq!(second_record.records(), 2);
        assert!(second_record.is_admitted());
        assert_eq!(
            second_record.history_record.health.status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert!(resume_plan.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_resume_plan_trend_records=1"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_plan_preserves_repair_pressure() {
        let repair_handoff_summary = repair_handoff().summary();
        let repair_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
                    repair_handoff_summary,
                ]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let repair_decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &repair_handoff_history_record);
        let clean_handoff = stable_handoff();
        let clean_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &clean_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &clean_handoff,
            &clean_handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                repair_decision.summary(),
            ]),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);
        let resume_plan = continuation.resume_plan();

        let second_record = resume_plan.monitor_next(&clean_handoff, &clean_handoff_history_record);

        assert!(resume_plan.prior_requires_repair_first);
        assert!(!resume_plan.prior_can_submit_memory_note);
        assert_eq!(resume_plan.trend_history.len(), 2);
        assert_eq!(resume_plan.admission_history.len(), 1);
        assert_eq!(second_record.admission.history_record.records(), 3);
        assert_eq!(second_record.records(), 2);
        assert!(!second_record.is_admitted());
        assert!(!second_record.admission.can_submit_memory_note);
        assert!(second_record.requires_repair_first());
        assert_eq!(
            second_record.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(resume_plan.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_resume_plan_repair_first=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_runner_advances_stable_continuation() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);

        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_history_record,
        );

        assert!(resume_record.is_admitted());
        assert!(!resume_record.requires_repair_first());
        assert_eq!(resume_record.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(resume_record.resume_plan.trend_history.len(), 1);
        assert_eq!(
            resume_record
                .monitor_record
                .admission
                .history_record
                .records(),
            2
        );
        assert_eq!(resume_record.monitor_record.records(), 2);
        assert_eq!(resume_record.continuation.trend_history.len(), 2);
        assert_eq!(resume_record.continuation.admission_history.len(), 2);
        assert!(resume_record.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_resume_record_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_runner_preserves_repair_continuation() {
        let repair_handoff_summary = repair_handoff().summary();
        let repair_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
                    repair_handoff_summary,
                ]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let repair_decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &repair_handoff_history_record);
        let clean_handoff = stable_handoff();
        let clean_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &clean_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &clean_handoff,
            &clean_handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                repair_decision.summary(),
            ]),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);

        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &clean_handoff,
            &clean_handoff_history_record,
        );

        assert!(!resume_record.is_admitted());
        assert!(resume_record.requires_repair_first());
        assert_eq!(
            resume_record.next_queue().task_ids(),
            vec![
                "adapter-boundary-handoff-trend-gate-repair-0",
                "adapter-boundary-handoff-trend-gate-repair-1",
                "adapter-boundary-handoff-trend-gate-repair-2",
                "business-task",
            ]
        );
        assert_eq!(resume_record.resume_plan.trend_history.len(), 2);
        assert_eq!(
            resume_record
                .monitor_record
                .admission
                .history_record
                .records(),
            3
        );
        assert_eq!(resume_record.monitor_record.records(), 2);
        assert_eq!(resume_record.continuation.trend_history.len(), 3);
        assert_eq!(resume_record.continuation.admission_history.len(), 2);
        assert!(resume_record.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_resume_record_repair_first=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_summary_compacts_stable_record() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_history_record,
        );

        let summary = resume_record.summary();

        assert_eq!(
            summary.prior_trend_health_status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert_eq!(
            summary.next_admission_health_status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert!(summary.effective_admitted);
        assert!(summary.can_submit_memory_note);
        assert!(summary.can_promote_adaptive_state);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.prior_trend_records, 1);
        assert_eq!(summary.next_trend_records, 2);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_resume_summary_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_history_watches_empty_records() {
        let health = AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new()
            .health(AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default());

        assert_eq!(health.status, AgentAdapterBoundaryStatus::Watch);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["adapter_boundary_handoff_trend_admission_resume_history_empty"]
        );
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_history_repairs_dirty_records() {
        let repair_handoff_summary = repair_handoff().summary();
        let repair_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
                    repair_handoff_summary,
                ]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let repair_decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &repair_handoff_history_record);
        let clean_handoff = stable_handoff();
        let clean_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &clean_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &clean_handoff,
            &clean_handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                repair_decision.summary(),
            ]),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &clean_handoff,
            &clean_handoff_history_record,
        );

        let history_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
            .record_resume_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                &resume_record,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
            );

        assert_eq!(history_record.records(), 1);
        assert_eq!(
            history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(!history_record.allows_service_advance());
        assert!(history_record.requires_repair_first());
        assert_eq!(history_record.dashboard.effective_admitted_records, 0);
        assert_eq!(history_record.dashboard.repair_first_records, 1);
        assert_eq!(
            history_record.health.reasons,
            vec![
                "adapter_boundary_handoff_trend_admission_resume_repair_first_records=1>0",
                "adapter_boundary_handoff_trend_admission_resume_effective_admitted_rate=0.000<0.67",
            ]
        );
        assert!(history_record.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_history_record_repair_first=1"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_preserves_stable_history() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_history_record,
        );
        let history_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
            .record_resume_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                &resume_record,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
            );

        let decision = AgentAdapterBoundaryHandoffTrendAdmissionResumeGate::new()
            .gate(&resume_record, &history_record);

        assert!(decision.requested_admitted);
        assert!(decision.is_admitted());
        assert!(decision.can_submit_memory_note);
        assert!(decision.can_promote_adaptive_state);
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert_eq!(
            decision.resume_health.status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_resume_gate_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_repairs_dirty_history() {
        let repair_handoff_summary = repair_handoff().summary();
        let repair_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
                    repair_handoff_summary,
                ]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let repair_decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &repair_handoff_history_record);
        let clean_handoff = stable_handoff();
        let clean_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &clean_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &clean_handoff,
            &clean_handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                repair_decision.summary(),
            ]),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &clean_handoff,
            &clean_handoff_history_record,
        );
        let history_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
            .record_resume_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                &resume_record,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
            );

        let decision = AgentAdapterBoundaryHandoffTrendAdmissionResumeGate::new()
            .gate(&resume_record, &history_record);

        assert!(!decision.requested_admitted);
        assert!(!decision.is_admitted());
        assert!(!decision.can_submit_memory_note);
        assert!(!decision.can_promote_adaptive_state);
        assert!(decision.requires_repair_first);
        assert_eq!(
            decision.resume_health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert_eq!(decision.repair_tasks.len(), 2);
        assert_eq!(
            decision.next_queue.task_ids(),
            vec![
                "adapter-boundary-handoff-trend-admission-resume-repair-0",
                "adapter-boundary-handoff-trend-admission-resume-repair-1",
                "adapter-boundary-handoff-trend-gate-repair-0",
                "adapter-boundary-handoff-trend-gate-repair-1",
                "adapter-boundary-handoff-trend-gate-repair-2",
                "business-task",
            ]
        );
        assert_eq!(
            decision.blocked_reasons,
            vec![
                "resume_history:adapter_boundary_handoff_trend_admission_resume_repair_first_records=1>0",
                "resume_history:adapter_boundary_handoff_trend_admission_resume_effective_admitted_rate=0.000<0.67",
            ]
        );
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_adapter_boundary_handoff_trend_admission_resume_gate_repair_tasks=2"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_summary_compacts_stable_decision() {
        let decision = stable_resume_gate_decision();

        let summary = decision.summary();

        assert_eq!(
            summary.resume_health_status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.effective_admitted);
        assert!(!summary.requires_repair_first);
        assert!(summary.can_submit_memory_note);
        assert!(summary.can_promote_adaptive_state);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_tasks, 1);
        assert!(summary.repair_task_ids.is_empty());
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(summary.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_summary_compacts_repair_decision() {
        let decision = repair_resume_gate_decision();

        let summary = decision.summary();

        assert_eq!(
            summary.resume_health_status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(!summary.requested_admitted);
        assert!(!summary.effective_admitted);
        assert!(summary.requires_repair_first);
        assert!(!summary.can_submit_memory_note);
        assert!(!summary.can_promote_adaptive_state);
        assert_eq!(summary.repair_tasks, 2);
        assert_eq!(summary.blocked_reasons, 2);
        assert_eq!(
            summary.repair_task_ids,
            vec![
                "adapter-boundary-handoff-trend-admission-resume-repair-0",
                "adapter-boundary-handoff-trend-admission-resume-repair-1",
            ]
        );
        assert_eq!(
            summary.next_queue_task_ids,
            vec![
                "adapter-boundary-handoff-trend-admission-resume-repair-0",
                "adapter-boundary-handoff-trend-admission-resume-repair-1",
                "adapter-boundary-handoff-trend-gate-repair-0",
                "adapter-boundary-handoff-trend-gate-repair-1",
                "adapter-boundary-handoff-trend-gate-repair-2",
                "business-task",
            ]
        );
        assert!(summary.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_summary_repair_tasks=2"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_history_watches_empty_records() {
        let health = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new()
            .health(AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default());

        assert_eq!(health.status, AgentAdapterBoundaryStatus::Watch);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["adapter_boundary_handoff_trend_admission_resume_gate_history_empty"]
        );
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_history_repairs_dirty_pressure() {
        let decision = repair_resume_gate_decision();

        let record = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecorder::new()
            .record_decision_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                &decision,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
            );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(record.dashboard.requested_admitted_records, 0);
        assert_eq!(record.dashboard.effective_admitted_records, 0);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.repair_task_count, 2);
        assert_eq!(
            record.dashboard.latest_resume_health_status,
            Some(AgentAdapterBoundaryStatus::Repair)
        );
        assert_eq!(
            record.health.reasons,
            vec![
                "adapter_boundary_handoff_trend_admission_resume_gate_repair_first_records=1>0",
                "adapter_boundary_handoff_trend_admission_resume_gate_repair_tasks=2>0",
                "adapter_boundary_handoff_trend_admission_resume_gate_effective_admitted_rate=0.000<0.67",
            ]
        );
        assert!(record.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_history_record_repair_tasks=2"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_records_stable_decision() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_history_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );

        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitor::new()
            .monitor(
                &resume_record,
                &resume_history_record,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
            );

        assert_eq!(monitor_record.records(), 1);
        assert!(monitor_record.is_admitted());
        assert!(monitor_record.allows_service_advance());
        assert!(!monitor_record.requires_repair_first());
        assert!(monitor_record.can_submit_memory_note());
        assert!(monitor_record.can_promote_adaptive_state());
        assert_eq!(
            monitor_record.history_record.health.status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert_eq!(
            monitor_record.decision.next_queue.task_ids(),
            vec!["business-task"]
        );
        assert!(monitor_record.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_repairs_dirty_decision() {
        let repair_handoff_summary = repair_handoff().summary();
        let repair_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_summary_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::from_summaries(vec![
                    repair_handoff_summary,
                ]),
                stable_handoff().summary(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let repair_decision = AgentAdapterBoundaryHandoffTrendGate::new()
            .gate(&stable_handoff(), &repair_handoff_history_record);
        let clean_handoff = stable_handoff();
        let clean_handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &clean_handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &clean_handoff,
            &clean_handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::from_summaries(vec![
                repair_decision.summary(),
            ]),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &clean_handoff,
            &clean_handoff_history_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );

        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitor::new()
            .monitor(
                &resume_record,
                &resume_history_record,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
            );

        assert_eq!(monitor_record.records(), 1);
        assert!(!monitor_record.is_admitted());
        assert!(!monitor_record.allows_service_advance());
        assert!(monitor_record.requires_repair_first());
        assert!(!monitor_record.can_submit_memory_note());
        assert!(!monitor_record.can_promote_adaptive_state());
        assert_eq!(
            monitor_record.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert_eq!(monitor_record.decision.repair_tasks.len(), 2);
        assert_eq!(monitor_record.history_record.dashboard.repair_task_count, 2);
        assert!(monitor_record.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_repair_tasks=2"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_gate_preserves_stable_history() {
        let decision = stable_resume_gate_decision();
        let history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecorder::new()
                .record_decision_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    &decision,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let service_execution_command_reason_count = history_record
            .dashboard
            .service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count = history_record
            .dashboard
            .service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes = history_record
            .dashboard
            .service_execution_memory_promotion_command_reason_closes;
        let service_execution_tool_build_command_reason_count = history_record
            .dashboard
            .service_execution_tool_build_command_reason_count;
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorRecord {
            decision,
            history_record,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry: Vec::new(),
        };

        let gated = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGate::new()
            .gate(&monitor_record);

        assert!(gated.requested_admitted);
        assert!(gated.is_admitted());
        assert!(gated.can_submit_memory_note);
        assert!(gated.can_promote_adaptive_state);
        assert!(!gated.requires_repair_first);
        assert!(gated.repair_tasks.is_empty());
        assert_eq!(gated.next_queue.task_ids(), vec!["business-task"]);
        assert_eq!(gated.gate_health.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gated.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_gate_repairs_dirty_history() {
        let stable_decision = stable_resume_gate_decision();
        let repair_summary = repair_resume_gate_decision().summary();
        let history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecorder::new()
                .record_decision_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::from_summaries(
                    vec![repair_summary],
                ),
                &stable_decision,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
            );
        let service_execution_command_reason_count = history_record
            .dashboard
            .service_execution_command_reason_count;
        let service_execution_memory_promotion_command_reason_count = history_record
            .dashboard
            .service_execution_memory_promotion_command_reason_count;
        let service_execution_memory_promotion_command_reason_closes = history_record
            .dashboard
            .service_execution_memory_promotion_command_reason_closes;
        let service_execution_tool_build_command_reason_count = history_record
            .dashboard
            .service_execution_tool_build_command_reason_count;
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorRecord {
            decision: stable_decision,
            history_record,
            service_execution_command_reason_count,
            service_execution_memory_promotion_command_reason_count,
            service_execution_memory_promotion_command_reason_closes,
            service_execution_tool_build_command_reason_count,
            telemetry: Vec::new(),
        };

        let gated = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGate::new()
            .gate(&monitor_record);

        assert!(gated.requested_admitted);
        assert!(!gated.is_admitted());
        assert!(!gated.can_submit_memory_note);
        assert!(!gated.can_promote_adaptive_state);
        assert!(gated.requires_repair_first);
        assert_eq!(gated.gate_health.status, AgentAdapterBoundaryStatus::Repair);
        assert_eq!(gated.repair_tasks.len(), 3);
        assert_eq!(
            gated.next_queue.task_ids(),
            vec![
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-0",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-1",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-2",
                "business-task",
            ]
        );
        assert_eq!(
            gated.blocked_reasons,
            vec![
                "resume_gate_history:adapter_boundary_handoff_trend_admission_resume_gate_repair_first_records=1>0",
                "resume_gate_history:adapter_boundary_handoff_trend_admission_resume_gate_repair_tasks=2>0",
                "resume_gate_history:adapter_boundary_handoff_trend_admission_resume_gate_effective_admitted_rate=0.500<0.67",
            ]
        );
        assert!(gated.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_gate_repair_tasks=3"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_handoff_records_stable_boundary() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_history_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );

        let final_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );

        assert_eq!(final_handoff.records(), 1);
        assert!(final_handoff.is_admitted());
        assert!(final_handoff.allows_service_advance());
        assert!(!final_handoff.requires_repair_first());
        assert!(final_handoff.can_submit_memory_note());
        assert!(final_handoff.can_promote_adaptive_state());
        assert_eq!(
            final_handoff.gate_decision.next_queue.task_ids(),
            vec!["business-task"]
        );
        assert!(final_handoff.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_handoff_repairs_dirty_history() {
        let handoff = stable_handoff();
        let handoff_history_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let first_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_history_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = first_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_history_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let repair_summary = repair_resume_gate_decision().summary();

        let final_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                &resume_record,
                &resume_history_record,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::from_summaries(
                    vec![repair_summary],
                ),
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
            );

        assert_eq!(final_handoff.records(), 2);
        assert!(!final_handoff.is_admitted());
        assert!(!final_handoff.allows_service_advance());
        assert!(final_handoff.requires_repair_first());
        assert!(!final_handoff.can_submit_memory_note());
        assert!(!final_handoff.can_promote_adaptive_state());
        assert_eq!(final_handoff.gate_decision.repair_tasks.len(), 3);
        assert_eq!(
            final_handoff.gate_decision.next_queue.task_ids(),
            vec![
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-0",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-1",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-2",
                "business-task",
            ]
        );
        assert!(final_handoff.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_tasks=3"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_compacts_stable_boundary()
     {
        let final_handoff = stable_resume_gate_monitor_handoff();

        let summary = final_handoff.summary();

        assert_eq!(
            summary.gate_health_status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.effective_admitted);
        assert!(!summary.requires_repair_first);
        assert!(summary.can_submit_memory_note);
        assert!(summary.can_promote_adaptive_state);
        assert_eq!(summary.records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_tasks, 1);
        assert_eq!(summary.blocked_reasons, 0);
        assert!(summary.repair_task_ids.is_empty());
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(summary.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_compacts_repair_boundary()
     {
        let final_handoff = repair_resume_gate_monitor_handoff();

        let summary = final_handoff.summary();

        assert_eq!(
            summary.gate_health_status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(summary.requested_admitted);
        assert!(!summary.effective_admitted);
        assert!(summary.requires_repair_first);
        assert!(!summary.can_submit_memory_note);
        assert!(!summary.can_promote_adaptive_state);
        assert_eq!(summary.records, 2);
        assert_eq!(summary.repair_tasks, 3);
        assert_eq!(summary.next_queue_tasks, 4);
        assert_eq!(summary.blocked_reasons, 3);
        assert_eq!(
            summary.repair_task_ids,
            vec![
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-0",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-1",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-2",
            ]
        );
        assert_eq!(
            summary.next_queue_task_ids,
            vec![
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-0",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-1",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair-2",
                "business-task",
            ]
        );
        assert!(summary.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_summary_repair_tasks=3"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_watches_empty_records()
    {
        let health =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new()
                .health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );

        assert_eq!(health.status, AgentAdapterBoundaryStatus::Watch);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec![
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_empty"
            ]
        );
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_repairs_dirty_pressure()
    {
        let final_handoff = repair_resume_gate_monitor_handoff();

        let record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecorder::new()
                .record_handoff_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
                    &final_handoff,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );

        assert_eq!(record.records(), 1);
        assert_eq!(record.health.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(record.dashboard.requested_admitted_records, 1);
        assert_eq!(record.dashboard.effective_admitted_records, 0);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.repair_records, 1);
        assert_eq!(record.dashboard.repair_task_count, 3);
        assert_eq!(record.dashboard.blocked_reasons, 3);
        assert_eq!(
            record.dashboard.latest_gate_health_status,
            Some(AgentAdapterBoundaryStatus::Repair)
        );
        assert_eq!(
            record.health.reasons,
            vec![
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_first_records=1>0",
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_records=1>0",
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_tasks=3>0",
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_blocked_reasons=3>0",
                "adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_effective_admitted_rate=0.000<0.67",
            ]
        );
        assert!(record.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_history_record_repair_tasks=3"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_preserves_stable_history()
    {
        let final_handoff = stable_resume_gate_monitor_handoff();
        let history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecorder::new()
                .record_handoff_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
                    &final_handoff,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );

        let gated = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGate::new()
            .gate(&final_handoff, &history_record);

        assert!(gated.requested_admitted);
        assert!(gated.is_admitted());
        assert!(gated.can_submit_memory_note);
        assert!(gated.can_promote_adaptive_state);
        assert!(!gated.requires_repair_first);
        assert!(gated.repair_tasks.is_empty());
        assert_eq!(gated.next_queue.task_ids(), vec!["business-task"]);
        assert_eq!(
            gated.handoff_health.status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert!(gated.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_repairs_dirty_history() {
        let stable_handoff = stable_resume_gate_monitor_handoff();
        let dirty_summary = repair_resume_gate_monitor_handoff().summary();
        let history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecorder::new()
                .record_handoff_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::from_summaries(
                    vec![dirty_summary],
                ),
                &stable_handoff,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
            );

        let gated = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGate::new()
            .gate(&stable_handoff, &history_record);

        assert!(gated.requested_admitted);
        assert!(!gated.is_admitted());
        assert!(!gated.can_submit_memory_note);
        assert!(!gated.can_promote_adaptive_state);
        assert!(gated.requires_repair_first);
        assert_eq!(
            gated.handoff_health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert_eq!(gated.repair_tasks.len(), 5);
        assert_eq!(
            gated.next_queue.task_ids(),
            vec![
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-0",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-1",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-2",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-3",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-4",
                "business-task",
            ]
        );
        assert_eq!(
            gated.blocked_reasons,
            vec![
                "resume_gate_monitor_handoff_history:adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_first_records=1>0",
                "resume_gate_monitor_handoff_history:adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_records=1>0",
                "resume_gate_monitor_handoff_history:adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_tasks=3>0",
                "resume_gate_monitor_handoff_history:adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_blocked_reasons=3>0",
                "resume_gate_monitor_handoff_history:adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_effective_admitted_rate=0.500<0.67",
            ]
        );
        assert!(gated.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_gate_repair_tasks=5"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_records_and_gates_stable_history()
     {
        let final_handoff = stable_resume_gate_monitor_handoff();

        let packet = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new(
        )
        .record_and_gate(
            final_handoff,
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(
            ),
        );

        assert_eq!(packet.records(), 1);
        assert!(packet.is_admitted());
        assert!(packet.allows_service_advance());
        assert!(!packet.requires_repair_first());
        assert!(packet.can_submit_memory_note());
        assert!(packet.can_promote_adaptive_state());
        assert_eq!(packet.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(
            packet.history_record.health.status,
            AgentAdapterBoundaryStatus::Stable
        );
        assert!(packet.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_effective=true"
        }));
    }

    #[test]
    fn boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_repairs_dirty_history()
    {
        let stable_handoff = stable_resume_gate_monitor_handoff();
        let dirty_summary = repair_resume_gate_monitor_handoff().summary();

        let packet = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
            .record_and_gate(
                stable_handoff,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::from_summaries(
                    vec![dirty_summary],
                ),
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
            );

        assert_eq!(packet.records(), 2);
        assert!(!packet.is_admitted());
        assert!(!packet.allows_service_advance());
        assert!(packet.requires_repair_first());
        assert!(!packet.can_submit_memory_note());
        assert!(!packet.can_promote_adaptive_state());
        assert_eq!(packet.gate_decision.repair_tasks.len(), 5);
        assert_eq!(
            packet.next_queue().task_ids(),
            vec![
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-0",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-1",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-2",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-3",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-4",
                "business-task",
            ]
        );
        let repair_task_ids = packet
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let business_task = packet
            .next_queue()
            .tasks()
            .into_iter()
            .find(|task| task.id == "business-task")
            .expect("final packet business task should remain behind adapter repair");
        let schedule = RecursiveAgentScheduler::new(16).plan(packet.next_queue().tasks());

        assert_eq!(business_task.dependencies, repair_task_ids);
        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.waves[0].task_ids, repair_task_ids);
        assert_eq!(schedule.waves[1].task_ids, vec!["business-task"]);
        assert_eq!(
            packet.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(packet.telemetry.iter().any(|line| {
            line
                == "agent_adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_handoff_repair_tasks=5"
        }));
    }

    #[test]
    fn boundary_handoff_final_packet_fields_stabilize_repair_contract() {
        let stable_handoff = stable_resume_gate_monitor_handoff();
        let dirty_summary = repair_resume_gate_monitor_handoff().summary();

        let packet = AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
            .record_and_gate(
                stable_handoff,
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::from_summaries(
                    vec![dirty_summary],
                ),
                AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
            );
        let repair_task_ids = packet
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let mut expected_next_queue_task_ids = repair_task_ids.clone();
        expected_next_queue_task_ids.push("business-task".to_owned());
        let business_task = packet
            .next_queue()
            .tasks()
            .into_iter()
            .find(|task| task.id == "business-task")
            .expect("business task should remain behind stable final packet repairs");

        assert_eq!(
            packet.gate_decision.blocked_reasons,
            vec![
                "resume_gate_monitor_handoff_history:adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_first_records=1>0",
                "resume_gate_monitor_handoff_history:adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_records=1>0",
                "resume_gate_monitor_handoff_history:adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_repair_tasks=3>0",
                "resume_gate_monitor_handoff_history:adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_blocked_reasons=3>0",
                "resume_gate_monitor_handoff_history:adapter_boundary_handoff_trend_admission_resume_gate_monitor_handoff_effective_admitted_rate=0.500<0.67",
            ]
        );
        assert_eq!(
            repair_task_ids,
            vec![
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-0",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-1",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-2",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-3",
                "adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair-4",
            ]
        );
        assert_eq!(packet.next_queue().task_ids(), expected_next_queue_task_ids);
        assert_eq!(business_task.dependencies, repair_task_ids);
    }

    #[test]
    fn adapter_gate_projects_clean_tool_build_report_history() {
        let report = ToolBuildReport {
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
            ToolBuildReportSummaryHistory::new(),
            &report,
            ToolBuildReportHealthPolicy::default(),
        );

        let gate = AgentAdapterBoundaryGate::from_tool_build_report_history_gate(&record);
        let snapshot = AgentAdapterBoundarySnapshot::from_tool_build_report_history_gate(
            &business_queue(),
            &record,
        );

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Stable);
        assert!(gate.dispatch_allowed);
        assert!(gate.service_command_allowed);
        assert!(gate.memory_note_allowed);
        assert!(gate.adaptive_state_allowed);
        assert!(snapshot.allows_service_advance());
        assert!(snapshot.can_execute_service_commands());
        assert!(snapshot.can_submit_memory_note());
        assert!(snapshot.can_promote_adaptive_state());
        assert_eq!(snapshot.next_queue_task_ids, vec!["business-task"]);
    }

    #[test]
    fn adapter_gate_projects_dirty_tool_build_report_history_as_repair() {
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

        let gate = AgentAdapterBoundaryGate::from_tool_build_report_history_gate(&record);
        let snapshot = AgentAdapterBoundarySnapshot::from_tool_build_report_history_gate(
            &business_queue(),
            &record,
        );

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(
            snapshot
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "service_adapter:tool_build_report_missing_requests=1" })
        );
    }

    #[test]
    fn adapter_gate_projects_dirty_toolsmith_plan_history_as_closed_tool_boundary() {
        let dirty_context = "OLD_WINDOW_RAW_TASK_PROMPT::do-not-copy-into-final-handoff";
        let dirty_plan = ToolsmithPlan::new()
            .with_proposal(ToolProposal::new(
                "trace-script",
                ToolIntent::TraceAnalysis,
                "python",
                "tools/trace.py",
                ToolBuildStatus::Ready,
            ))
            .with_rejected_request(dirty_context);
        let record = ToolsmithPlanSummaryHistoryRecorder::new().record_plan_with_health_gate(
            ToolsmithPlanSummaryHistory::new(),
            &dirty_plan,
            ToolsmithPlanHealthPolicy::default(),
        );
        let clean_plan = ToolsmithPlan::new().with_proposal(ToolProposal::new(
            "runtime-gate",
            ToolIntent::BenchmarkGate,
            "rust",
            "tools/runtime_gate.rs",
            ToolBuildStatus::Ready,
        ));

        let gate = AgentAdapterBoundaryGate::from_toolsmith_plan_history_gate(&record);
        let snapshot = AgentAdapterBoundarySnapshot::from_toolsmith_plan_history_gate(
            &business_queue(),
            &record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_toolsmith_plan_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff = AgentAdapterBoundaryHandoff::from_record_and_queue(
            boundary_record.clone(),
            &business_queue(),
        );
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let trend_summary = trend_decision.summary();
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let admission_record = AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
            .record_admission_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                &admission,
                AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
            );
        let admission_summary = admission.summary();
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = monitor_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let downstream_summary = downstream_handoff.summary();
        let downstream_packet =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
                .record_and_gate(
                    downstream_handoff.clone(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );

        assert_eq!(gate.owner, AgentAdapterBoundaryOwner::ServiceAdapter);
        assert_eq!(gate.status, AgentAdapterBoundaryStatus::Repair);
        assert!(!gate.dispatch_allowed);
        assert!(!gate.service_command_allowed);
        assert!(!gate.memory_note_allowed);
        assert!(!gate.adaptive_state_allowed);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.allows_service_advance());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert_eq!(ToolBuildRequest::ready_requests(&clean_plan).len(), 1);
        assert!(ToolBuildRequest::admitted_requests(&clean_plan, &record.gate_decision).is_empty());
        assert!(
            snapshot.blocked_reasons.iter().any(|reason| reason
                == "service_adapter:toolsmith_plan_history:toolsmith_plan_rejected=1>0")
        );
        assert!(boundary_record.requires_repair_first());
        assert!(!boundary_record.allows_service_advance());
        assert!(!boundary_record.can_execute_service_commands());
        assert!(!boundary_record.can_submit_memory_note());
        assert!(!boundary_record.can_promote_adaptive_state());
        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_execute_service_commands());
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(
            handoff.blocked_reasons.iter().any(|reason| reason
                == "service_adapter:toolsmith_plan_history:toolsmith_plan_rejected=1>0")
        );
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(handoff_record.appended_summary, handoff.summary());
        assert_eq!(
            handoff_record.dashboard.repair_task_count,
            handoff.repair_tasks.len()
        );
        assert!(trend_decision.requires_repair_first);
        assert!(!trend_decision.is_admitted());
        assert!(!trend_decision.can_submit_memory_note);
        assert!(!trend_decision.can_promote_adaptive_state);
        assert_eq!(
            trend_summary.repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            trend_summary.blocked_reasons,
            trend_decision.blocked_reasons.len()
        );
        assert!(
            trend_decision.blocked_reasons.iter().any(|reason| reason
                == "service_adapter:toolsmith_plan_history:toolsmith_plan_rejected=1>0")
        );
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("handoff_history:"))
        );
        assert!(admission.requires_repair_first());
        assert!(!admission.is_admitted());
        assert!(!admission.can_submit_memory_note);
        assert!(!admission.can_promote_adaptive_state);
        assert_eq!(
            admission_summary.decision_repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            admission_summary.blocked_reasons,
            admission.blocked_reasons.len()
        );
        assert!(
            admission.blocked_reasons.iter().any(|reason| reason
                == "service_adapter:toolsmith_plan_history:toolsmith_plan_rejected=1>0")
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("trend_gate_history:"))
        );
        assert_eq!(admission_record.appended_summary, admission_summary);
        assert_eq!(
            admission_record.dashboard.blocked_reasons,
            admission_summary.blocked_reasons
        );
        assert!(!monitor_record.is_admitted());
        assert!(monitor_record.requires_repair_first());
        assert!(!continuation.is_admitted());
        assert!(continuation.requires_repair_first);
        assert!(resume_record.requires_repair_first());
        assert!(!resume_record.is_admitted());
        assert!(!downstream_handoff.is_admitted());
        assert!(downstream_handoff.requires_repair_first());
        assert!(!downstream_handoff.can_submit_memory_note());
        assert!(!downstream_handoff.can_promote_adaptive_state());
        assert_eq!(
            downstream_summary.repair_tasks,
            downstream_handoff.gate_decision.repair_tasks.len()
        );
        assert_eq!(
            downstream_summary.blocked_reasons,
            downstream_handoff.gate_decision.blocked_reasons.len()
        );
        assert!(downstream_summary.repair_tasks > 0);
        assert!(downstream_summary.blocked_reasons > 0);
        assert!(
            downstream_handoff
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_history:"))
        );
        assert!(
            downstream_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(!format!("{downstream_packet:?}").contains(dirty_context));
        assert!(
            !downstream_packet
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains(dirty_context))
        );
        assert!(
            !downstream_packet
                .gate_decision
                .repair_tasks
                .iter()
                .any(|task| {
                    task.id.contains(dirty_context)
                        || task.objective.contains(dirty_context)
                        || task.lane.contains(dirty_context)
                        || task
                            .dependencies
                            .iter()
                            .any(|dependency| dependency.contains(dirty_context))
                })
        );
        assert!(!downstream_packet.next_queue().tasks().iter().any(|task| {
            task.id.contains(dirty_context)
                || task.objective.contains(dirty_context)
                || task.lane.contains(dirty_context)
                || task
                    .dependencies
                    .iter()
                    .any(|dependency| dependency.contains(dirty_context))
        }));
    }

    #[test]
    fn adapter_snapshot_keeps_tool_build_repair_closed_even_when_report_gate_accepts() {
        let report_gate = AgentReportGateDecision {
            accepted: true,
            reasons: Vec::new(),
            follow_up_tasks: Vec::new(),
        };
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
        let tool_record = ToolBuildReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ToolBuildReportSummaryHistory::new(),
                &report,
                ToolBuildReportHealthPolicy::default(),
            );

        let snapshot = AgentAdapterBoundarySnapshot::from_report_and_tool_build_gates(
            &business_queue(),
            &report_gate,
            &tool_record,
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_report_and_tool_build_gates_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &report_gate,
                &tool_record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert_eq!(snapshot.status(), AgentAdapterBoundaryStatus::Repair);
        assert!(snapshot.requires_repair_first());
        assert!(!snapshot.can_execute_service_commands());
        assert!(!snapshot.can_submit_memory_note());
        assert!(!snapshot.can_promote_adaptive_state());
        assert!(
            snapshot
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "service_adapter:tool_build_report_missing_requests=1" })
        );
        assert_eq!(boundary_record.records(), 1);
        assert!(!boundary_record.allows_service_advance());
        assert!(boundary_record.requires_repair_first());
        assert_eq!(boundary_record.summary().repair_owners, 1);
        assert!(
            boundary_record.summary().blocked_reasons >= 2,
            "report/tool-build snapshot should preserve tool-build current and history reasons"
        );
        assert_eq!(
            boundary_record.history_record.health.status,
            AgentAdapterBoundaryStatus::Repair
        );
    }

    #[test]
    fn adapter_boundary_recorder_records_tool_build_report_history_gate() {
        let report = ToolBuildReport {
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
        let tool_record = ToolBuildReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ToolBuildReportSummaryHistory::new(),
                &report,
                ToolBuildReportHealthPolicy::default(),
            );

        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_tool_build_report_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &business_queue(),
                &tool_record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );

        assert_eq!(boundary_record.records(), 1);
        assert!(boundary_record.allows_service_advance());
        assert!(!boundary_record.requires_repair_first());
        assert!(boundary_record.can_execute_service_commands());
        assert!(boundary_record.can_submit_memory_note());
        assert!(boundary_record.can_promote_adaptive_state());
        assert_eq!(boundary_record.summary().owners, 1);
        assert_eq!(boundary_record.summary().stable_owners, 1);
    }

    #[test]
    fn adapter_handoff_recorder_records_dirty_tool_build_report_history_gate() {
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
        let tool_record = ToolBuildReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ToolBuildReportSummaryHistory::new(),
                &report,
                ToolBuildReportHealthPolicy::default(),
            );

        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_tool_build_report_history_gate_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &business_queue(),
                &tool_record,
                AgentAdapterBoundaryHealthPolicy::default(),
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );

        assert_eq!(handoff_record.records(), 1);
        assert!(!handoff_record.allows_service_advance());
        assert!(handoff_record.requires_repair_first());
        assert_eq!(
            handoff_record.appended_summary.snapshot_status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert_eq!(
            handoff_record.appended_summary.health_status,
            AgentAdapterBoundaryStatus::Repair
        );
        assert!(!handoff_record.appended_summary.can_submit_memory_note);
        assert!(!handoff_record.appended_summary.can_promote_adaptive_state);
        assert!(handoff_record.appended_summary.repair_tasks > 0);
        assert!(
            handoff_record
                .appended_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
    }

    #[test]
    fn adapter_tool_build_repair_handoff_schedules_before_memory_adaptive_and_eval_tasks() {
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
        let tool_record = ToolBuildReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                ToolBuildReportSummaryHistory::new(),
                &report,
                ToolBuildReportHealthPolicy::default(),
            );
        let next_queue = AgentTaskQueue::from_tasks(vec![
            AgentTask::new(
                "memory-note",
                AgentRole::MemoryCurator,
                "promote memory note after adapter boundary repair",
                AgentBudget::new(4, 1, 1),
            )
            .with_priority(10),
            AgentTask::new(
                "adaptive-state",
                AgentRole::Planner,
                "promote adaptive state after adapter boundary repair",
                AgentBudget::new(4, 1, 1),
            )
            .with_priority(10),
            AgentTask::new(
                "eval-finalize",
                AgentRole::Reviewer,
                "finalize eval after adapter boundary repair",
                AgentBudget::new(4, 1, 1),
            )
            .with_priority(10),
        ]);

        let handoff = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_tool_build_report_history_gate_handoff_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                &next_queue,
                &tool_record,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let repair_task_ids = handoff
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let schedule = RecursiveAgentScheduler::new(16).plan(handoff.next_queue.tasks());

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert!(handoff.repair_tasks.len() >= 2);
        for task_id in ["adaptive-state", "eval-finalize", "memory-note"] {
            let task = handoff
                .next_queue
                .tasks()
                .into_iter()
                .find(|task| task.id == task_id)
                .expect("business task should remain behind adapter repair");
            assert_eq!(task.dependencies, repair_task_ids);
        }
        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.waves[0].task_ids, repair_task_ids);
        assert_eq!(
            schedule.waves[1].task_ids,
            vec!["adaptive-state", "eval-finalize", "memory-note"]
        );
    }

    #[test]
    fn adapter_multi_repair_handoff_orders_core_toolsmith_and_eval_before_business_wave() {
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
        let dirty_toolsmith = ToolsmithPlan::new()
            .with_proposal(ToolProposal::new(
                "trace-script",
                ToolIntent::TraceAnalysis,
                "python",
                "tools/trace.py",
                ToolBuildStatus::Ready,
            ))
            .with_rejected_request("shell tool outside rust crate");
        let toolsmith_record = ToolsmithPlanSummaryHistoryRecorder::new()
            .record_plan_with_health_gate(
                ToolsmithPlanSummaryHistory::new(),
                &dirty_toolsmith,
                ToolsmithPlanHealthPolicy::default(),
            );
        let report_gate = rejected_report_gate();
        let next_queue = business_queue();
        let snapshot = AgentAdapterBoundarySnapshot::from_gates(
            &next_queue,
            vec![
                AgentAdapterBoundaryGate::from_report_gate(&report_gate),
                AgentAdapterBoundaryGate::from_toolsmith_plan_history_gate(&toolsmith_record),
                AgentAdapterBoundaryGate::from_dispatch_gate(&dispatch_gate),
            ],
        );
        let boundary_record = AgentAdapterBoundarySummaryHistoryRecorder::new()
            .record_snapshot_boundary_with_health(
                AgentAdapterBoundarySummaryHistory::new(),
                snapshot,
                AgentAdapterBoundaryHealthPolicy::default(),
            );
        let handoff =
            AgentAdapterBoundaryHandoff::from_record_and_queue(boundary_record, &next_queue);
        let handoff_record = AgentAdapterBoundaryHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentAdapterBoundaryHandoffSummaryHistory::new(),
                &handoff,
                AgentAdapterBoundaryHandoffHealthPolicy::default(),
            );
        let trend_decision =
            AgentAdapterBoundaryHandoffTrendGate::new().gate(&handoff, &handoff_record);
        let trend_summary = trend_decision.summary();
        let admission = AgentAdapterBoundaryHandoffTrendAdmissionGate::new().admit(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default(),
        );
        let admission_summary = admission.summary();
        let admission_record = AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder::new()
            .record_admission_with_health(
                AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
                &admission,
                AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default(),
            );
        let trend_policy = AgentAdapterBoundaryHandoffTrendGateHealthPolicy::default();
        let admission_policy = AgentAdapterBoundaryHandoffTrendAdmissionHealthPolicy::default();
        let monitor_record = AgentAdapterBoundaryHandoffTrendAdmissionMonitor::new().monitor(
            &handoff,
            &handoff_record,
            AgentAdapterBoundaryHandoffTrendGateSummaryHistory::new(),
            trend_policy,
            AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory::new(),
            admission_policy,
        );
        let continuation = monitor_record.continuation(trend_policy, admission_policy);
        let resume_record = AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner::new().run(
            &continuation,
            &handoff,
            &handoff_record,
        );
        let resume_history_record =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder::new()
                .record_resume_with_health(
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeSummaryHistory::new(),
                    &resume_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeHealthPolicy::default(),
                );
        let downstream_handoff =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff::new()
                .record_and_gate(
                    &resume_record,
                    &resume_history_record,
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHealthPolicy::default(),
                );
        let downstream_summary = downstream_handoff.summary();
        let downstream_packet =
            AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff::new()
                .record_and_gate(
                    downstream_handoff.clone(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummaryHistory::new(),
                    AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHealthPolicy::default(),
                );
        let repair_task_ids = handoff
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let business_task = handoff
            .next_queue
            .tasks()
            .into_iter()
            .find(|task| task.id == "business-task")
            .expect("business task should remain behind multi-gate repair");
        let schedule = RecursiveAgentScheduler::new(16).plan(handoff.next_queue.tasks());
        let first_core_reason = handoff
            .blocked_reasons
            .iter()
            .position(|reason| reason.starts_with("norion_core:"))
            .expect("dispatch repair should be projected as norion core");
        let first_service_reason = handoff
            .blocked_reasons
            .iter()
            .position(|reason| reason.starts_with("service_adapter:"))
            .expect("toolsmith repair should be projected as service adapter");
        let first_eval_reason = handoff
            .blocked_reasons
            .iter()
            .position(|reason| reason.starts_with("eval_reporting:"))
            .expect("report repair should be projected as eval reporting");

        assert!(!handoff.is_admitted());
        assert!(handoff.requires_repair_first);
        assert!(!handoff.can_execute_service_commands());
        assert!(!handoff.can_submit_memory_note());
        assert!(!handoff.can_promote_adaptive_state());
        assert_eq!(
            handoff
                .boundary_record
                .snapshot
                .gates
                .iter()
                .map(|gate| gate.owner)
                .collect::<Vec<_>>(),
            vec![
                AgentAdapterBoundaryOwner::NorionCore,
                AgentAdapterBoundaryOwner::ServiceAdapter,
                AgentAdapterBoundaryOwner::EvalReporting,
            ]
        );
        assert_eq!(first_core_reason, 0);
        assert!(first_core_reason < first_service_reason);
        assert!(first_service_reason < first_eval_reason);
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("norion_core:")
                    && reason.contains("insufficient budget"))
        );
        assert!(handoff.blocked_reasons.iter().any(|reason| reason
            == "service_adapter:toolsmith_plan_history:toolsmith_plan_rejected_requests=1>0"));
        assert!(
            handoff
                .blocked_reasons
                .iter()
                .any(|reason| reason == "eval_reporting:validation_evidence_missing=true")
        );
        assert_eq!(handoff.repair_tasks.len(), handoff.blocked_reasons.len());
        assert_eq!(
            handoff_record.appended_summary.blocked_reasons,
            handoff.blocked_reasons.len()
        );
        assert_eq!(
            handoff_record.dashboard.repair_task_count,
            handoff.repair_tasks.len()
        );
        let trend_core_reason = trend_decision
            .blocked_reasons
            .iter()
            .position(|reason| reason.starts_with("norion_core:"))
            .expect("trend gate should preserve the core repair reason");
        let trend_service_reason = trend_decision
            .blocked_reasons
            .iter()
            .position(|reason| reason.starts_with("service_adapter:"))
            .expect("trend gate should preserve the service adapter repair reason");
        let trend_eval_reason = trend_decision
            .blocked_reasons
            .iter()
            .position(|reason| reason.starts_with("eval_reporting:"))
            .expect("trend gate should preserve the eval repair reason");
        assert!(trend_core_reason < trend_service_reason);
        assert!(trend_service_reason < trend_eval_reason);
        assert!(trend_decision.requires_repair_first);
        assert!(!trend_decision.is_admitted());
        assert_eq!(
            trend_summary.repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            trend_summary.blocked_reasons,
            trend_decision.blocked_reasons.len()
        );
        assert!(
            trend_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("handoff_history:"))
        );
        let admission_core_reason = admission
            .blocked_reasons
            .iter()
            .position(|reason| reason.starts_with("norion_core:"))
            .expect("admission should preserve the core repair reason");
        let admission_service_reason = admission
            .blocked_reasons
            .iter()
            .position(|reason| reason.starts_with("service_adapter:"))
            .expect("admission should preserve the service adapter repair reason");
        let admission_eval_reason = admission
            .blocked_reasons
            .iter()
            .position(|reason| reason.starts_with("eval_reporting:"))
            .expect("admission should preserve the eval repair reason");
        assert!(admission_core_reason < admission_service_reason);
        assert!(admission_service_reason < admission_eval_reason);
        assert!(admission.requires_repair_first());
        assert!(!admission.is_admitted());
        assert_eq!(
            admission_summary.decision_repair_tasks,
            trend_decision.repair_tasks.len()
        );
        assert_eq!(
            admission_summary.blocked_reasons,
            admission.blocked_reasons.len()
        );
        assert_eq!(admission_record.appended_summary, admission_summary);
        assert_eq!(
            admission_record.dashboard.blocked_reasons,
            admission_summary.blocked_reasons
        );
        assert!(
            admission
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("trend_gate_history:"))
        );
        assert!(!monitor_record.is_admitted());
        assert!(monitor_record.requires_repair_first());
        assert!(!continuation.is_admitted());
        assert!(continuation.requires_repair_first);
        assert!(resume_record.requires_repair_first());
        assert!(!resume_record.is_admitted());
        assert!(!downstream_handoff.is_admitted());
        assert!(downstream_handoff.requires_repair_first());
        assert_eq!(
            downstream_summary.repair_tasks,
            downstream_handoff.gate_decision.repair_tasks.len()
        );
        assert_eq!(
            downstream_summary.blocked_reasons,
            downstream_handoff.gate_decision.blocked_reasons.len()
        );
        assert!(
            downstream_handoff
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_history:"))
        );
        assert!(
            downstream_summary
                .next_queue_task_ids
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert!(!downstream_packet.is_admitted());
        assert!(!downstream_packet.allows_service_advance());
        assert!(downstream_packet.requires_repair_first());
        assert!(!downstream_packet.can_submit_memory_note());
        assert!(!downstream_packet.can_promote_adaptive_state());
        assert_eq!(
            downstream_packet.history_record.appended_summary,
            downstream_summary
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.repair_task_count,
            downstream_summary.repair_tasks
        );
        assert_eq!(
            downstream_packet.history_record.dashboard.blocked_reasons,
            downstream_summary.blocked_reasons
        );
        assert!(
            downstream_packet
                .gate_decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("resume_gate_monitor_handoff_history:"))
        );
        assert!(
            downstream_packet.gate_decision.repair_tasks.len() >= downstream_summary.repair_tasks
        );
        assert!(
            downstream_packet.gate_decision.blocked_reasons.len()
                >= downstream_summary.blocked_reasons
        );
        assert!(
            downstream_packet
                .next_queue()
                .task_ids()
                .iter()
                .any(|task_id| task_id == "business-task")
        );
        assert_eq!(business_task.dependencies, repair_task_ids);
        assert_eq!(schedule.wave_count(), 2);
        assert_eq!(schedule.waves[0].task_ids, repair_task_ids);
        assert_eq!(schedule.waves[1].task_ids, vec!["business-task"]);
    }
}
