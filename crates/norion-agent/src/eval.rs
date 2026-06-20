use std::collections::BTreeSet;

use crate::budget::AgentBudget;
use crate::cycle::{AgentCycleReport, AgentCycleSummary};
use crate::evolution::RewardAction;
use crate::memory::{MemoryPromotionGateDecision, MemorySubmissionReport};
use crate::task::{AgentRole, AgentTask, AgentTaskQueue};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentReportEvidence {
    pub validation_passed: bool,
    pub runtime_response_ok: bool,
    pub validation_refs: Vec<String>,
    pub runtime_refs: Vec<String>,
}

impl AgentReportEvidence {
    pub fn new(validation_passed: bool, runtime_response_ok: bool) -> Self {
        Self {
            validation_passed,
            runtime_response_ok,
            validation_refs: Vec::new(),
            runtime_refs: Vec::new(),
        }
    }

    pub fn with_validation_ref(mut self, evidence_ref: impl Into<String>) -> Self {
        self.validation_refs.push(evidence_ref.into());
        self
    }

    pub fn with_runtime_ref(mut self, evidence_ref: impl Into<String>) -> Self {
        self.runtime_refs.push(evidence_ref.into());
        self
    }

    pub fn has_validation_evidence(&self) -> bool {
        self.validation_passed && !self.validation_refs.is_empty()
    }

    pub fn has_runtime_evidence(&self) -> bool {
        self.runtime_response_ok && !self.runtime_refs.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleLedgerRecord {
    pub run_id: String,
    pub summary: AgentCycleSummary,
    pub memory_promotion: Option<MemoryPromotionLedgerSummary>,
    pub memory_submission: Option<MemorySubmissionReport>,
    pub evidence: AgentReportEvidence,
}

impl AgentCycleLedgerRecord {
    pub fn new(
        run_id: impl Into<String>,
        summary: AgentCycleSummary,
        evidence: AgentReportEvidence,
        memory_submission: Option<MemorySubmissionReport>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            summary,
            memory_promotion: None,
            memory_submission,
            evidence,
        }
    }

    pub fn from_report(
        run_id: impl Into<String>,
        report: &AgentCycleReport,
        evidence: AgentReportEvidence,
        memory_submission: Option<MemorySubmissionReport>,
    ) -> Self {
        Self::new(
            run_id,
            AgentCycleSummary::from_report(report),
            evidence,
            memory_submission,
        )
    }

    pub fn with_memory_promotion_summary(mut self, summary: MemoryPromotionLedgerSummary) -> Self {
        self.memory_promotion = Some(summary);
        self
    }

    pub fn with_memory_promotion_gate(self, decision: &MemoryPromotionGateDecision) -> Self {
        self.with_memory_promotion_summary(MemoryPromotionLedgerSummary::from_gate(decision))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPromotionLedgerStatus {
    NoCandidates,
    Promotable,
    Watch,
    Blocked,
    Repair,
}

impl MemoryPromotionLedgerStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoCandidates => "no_candidates",
            Self::Promotable => "promotable",
            Self::Watch => "watch",
            Self::Blocked => "blocked",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryPromotionLedgerSummary {
    pub status: MemoryPromotionLedgerStatus,
    pub candidate_notes: usize,
    pub can_submit_memory: bool,
    pub requires_repair_first: bool,
    pub reason_count: usize,
    pub repair_tasks: usize,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl MemoryPromotionLedgerSummary {
    pub fn from_gate(decision: &MemoryPromotionGateDecision) -> Self {
        let status = if decision.requires_repair_first {
            MemoryPromotionLedgerStatus::Repair
        } else if decision.can_submit_memory {
            MemoryPromotionLedgerStatus::Promotable
        } else if decision.candidate_notes == 0 {
            MemoryPromotionLedgerStatus::NoCandidates
        } else if !decision.memory_health.is_stable() {
            MemoryPromotionLedgerStatus::Watch
        } else {
            MemoryPromotionLedgerStatus::Blocked
        };
        let reason_count = decision.reasons.len();
        let repair_tasks = decision.repair_tasks.len();
        let telemetry = memory_promotion_ledger_summary_telemetry(
            status,
            decision.candidate_notes,
            decision.can_submit_memory,
            decision.requires_repair_first,
            reason_count,
            repair_tasks,
        );

        Self {
            status,
            candidate_notes: decision.candidate_notes,
            can_submit_memory: decision.can_submit_memory,
            requires_repair_first: decision.requires_repair_first,
            reason_count,
            repair_tasks,
            reasons: decision.reasons.clone(),
            telemetry,
        }
    }

    pub fn is_promotable(&self) -> bool {
        self.status == MemoryPromotionLedgerStatus::Promotable && self.can_submit_memory
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentReportGatePolicy {
    pub minimum_reward_total: f32,
    pub require_reinforce_action: bool,
    pub require_validation_evidence: bool,
    pub require_runtime_evidence: bool,
    pub require_memory_submission_for_promotions: bool,
}

impl Default for AgentReportGatePolicy {
    fn default() -> Self {
        Self {
            minimum_reward_total: 0.72,
            require_reinforce_action: true,
            require_validation_evidence: true,
            require_runtime_evidence: true,
            require_memory_submission_for_promotions: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReportGateReason {
    pub code: String,
    pub detail: String,
}

impl AgentReportGateReason {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }

    pub fn as_line(&self) -> String {
        format!("{}={}", self.code, self.detail)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReportGateDecision {
    pub accepted: bool,
    pub reasons: Vec<AgentReportGateReason>,
    pub follow_up_tasks: Vec<AgentTask>,
}

impl AgentReportGateDecision {
    pub fn is_accepted(&self) -> bool {
        self.accepted
    }

    pub fn summary(&self) -> AgentReportGateSummary {
        AgentReportGateSummary::from_decision(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReportGateSummary {
    pub accepted: bool,
    pub reason_count: usize,
    pub follow_up_tasks: usize,
    pub blocker_codes: Vec<String>,
    pub repair_lanes: Vec<String>,
    pub repair_roles: Vec<AgentRole>,
    pub has_memory_blocker: bool,
    pub has_budget_blocker: bool,
    pub has_validation_blocker: bool,
    pub has_runtime_blocker: bool,
    pub has_tool_build_blocker: bool,
    pub has_review_blocker: bool,
    pub telemetry: Vec<String>,
}

impl AgentReportGateSummary {
    pub fn from_decision(decision: &AgentReportGateDecision) -> Self {
        let blocker_codes = decision
            .reasons
            .iter()
            .map(|reason| reason.code.clone())
            .collect::<Vec<_>>();
        let repair_lanes = ordered_unique(
            decision
                .follow_up_tasks
                .iter()
                .map(|task| task.lane.clone()),
        );
        let repair_roles = ordered_unique_roles(
            decision
                .follow_up_tasks
                .iter()
                .map(|task| task.role.clone()),
        );
        let has_memory_blocker = blocker_codes
            .iter()
            .any(|code| is_memory_blocker_code(code));
        let has_budget_blocker = blocker_codes.iter().any(|code| code == "budget_overspends");
        let has_validation_blocker = blocker_codes
            .iter()
            .any(|code| code == "validation_evidence_missing");
        let has_runtime_blocker = blocker_codes
            .iter()
            .any(|code| code == "runtime_evidence_missing");
        let has_tool_build_blocker = blocker_codes
            .iter()
            .any(|code| is_tool_build_blocker_code(code));
        let has_review_blocker = blocker_codes.iter().any(|code| {
            !is_memory_blocker_code(code)
                && code != "budget_overspends"
                && code != "validation_evidence_missing"
                && code != "runtime_evidence_missing"
                && !is_tool_build_blocker_code(code)
        });
        let telemetry = report_gate_summary_telemetry(
            decision.accepted,
            blocker_codes.len(),
            decision.follow_up_tasks.len(),
            repair_lanes.len(),
            repair_roles.len(),
            has_memory_blocker,
            has_budget_blocker,
            has_validation_blocker,
            has_runtime_blocker,
            has_tool_build_blocker,
            has_review_blocker,
        );

        Self {
            accepted: decision.accepted,
            reason_count: blocker_codes.len(),
            follow_up_tasks: decision.follow_up_tasks.len(),
            blocker_codes,
            repair_lanes,
            repair_roles,
            has_memory_blocker,
            has_budget_blocker,
            has_validation_blocker,
            has_runtime_blocker,
            has_tool_build_blocker,
            has_review_blocker,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentReportGateSummaryHistory {
    summaries: Vec<AgentReportGateSummary>,
}

impl AgentReportGateSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentReportGateSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentReportGateSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentReportGateSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentReportGateSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentReportGateDashboard {
        AgentReportGateDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AgentReportGateHealthPolicy) -> AgentReportGateHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateDashboard {
    pub total_records: usize,
    pub accepted_records: usize,
    pub blocked_records: usize,
    pub reason_count: usize,
    pub follow_up_tasks: usize,
    pub memory_blockers: usize,
    pub budget_blockers: usize,
    pub validation_blockers: usize,
    pub runtime_blockers: usize,
    pub tool_build_blockers: usize,
    pub review_blockers: usize,
    pub acceptance_rate: f32,
    pub blocked_rate: f32,
    pub latest_accepted: Option<bool>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateDashboard {
    pub fn from_summaries(summaries: &[AgentReportGateSummary]) -> Self {
        let total_records = summaries.len();
        let accepted_records = summaries.iter().filter(|summary| summary.accepted).count();
        let blocked_records = total_records.saturating_sub(accepted_records);
        let reason_count = summaries
            .iter()
            .map(|summary| summary.reason_count)
            .sum::<usize>();
        let follow_up_tasks = summaries
            .iter()
            .map(|summary| summary.follow_up_tasks)
            .sum::<usize>();
        let memory_blockers = summaries
            .iter()
            .filter(|summary| summary.has_memory_blocker)
            .count();
        let budget_blockers = summaries
            .iter()
            .filter(|summary| summary.has_budget_blocker)
            .count();
        let validation_blockers = summaries
            .iter()
            .filter(|summary| summary.has_validation_blocker)
            .count();
        let runtime_blockers = summaries
            .iter()
            .filter(|summary| summary.has_runtime_blocker)
            .count();
        let tool_build_blockers = summaries
            .iter()
            .filter(|summary| summary.has_tool_build_blocker)
            .count();
        let review_blockers = summaries
            .iter()
            .filter(|summary| summary.has_review_blocker)
            .count();
        let acceptance_rate = rate(accepted_records, total_records);
        let blocked_rate = rate(blocked_records, total_records);
        let latest_accepted = summaries.last().map(|summary| summary.accepted);
        let telemetry = report_gate_dashboard_telemetry(
            total_records,
            accepted_records,
            blocked_records,
            reason_count,
            follow_up_tasks,
            memory_blockers,
            budget_blockers,
            validation_blockers,
            runtime_blockers,
            tool_build_blockers,
            review_blockers,
            acceptance_rate,
            blocked_rate,
            latest_accepted,
        );

        Self {
            total_records,
            accepted_records,
            blocked_records,
            reason_count,
            follow_up_tasks,
            memory_blockers,
            budget_blockers,
            validation_blockers,
            runtime_blockers,
            tool_build_blockers,
            review_blockers,
            acceptance_rate,
            blocked_rate,
            latest_accepted,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn is_clean(&self) -> bool {
        !self.is_empty() && self.blocked_records == 0 && self.reason_count == 0
    }

    pub fn health(&self, policy: AgentReportGateHealthPolicy) -> AgentReportGateHealth {
        AgentReportGateHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentReportGateHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AgentReportGateHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentReportGateHealthPolicy {
    pub minimum_acceptance_rate: f32,
    pub maximum_blocked_records: usize,
    pub maximum_reason_count: usize,
    pub maximum_follow_up_tasks: usize,
    pub maximum_memory_blockers: usize,
    pub maximum_budget_blockers: usize,
    pub maximum_validation_blockers: usize,
    pub maximum_runtime_blockers: usize,
    pub maximum_tool_build_blockers: usize,
    pub maximum_review_blockers: usize,
}

impl Default for AgentReportGateHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_acceptance_rate: 0.67,
            maximum_blocked_records: 0,
            maximum_reason_count: 0,
            maximum_follow_up_tasks: 0,
            maximum_memory_blockers: 0,
            maximum_budget_blockers: 0,
            maximum_validation_blockers: 0,
            maximum_runtime_blockers: 0,
            maximum_tool_build_blockers: 0,
            maximum_review_blockers: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealth {
    pub status: AgentReportGateHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentReportGateDashboard,
}

impl AgentReportGateHealth {
    pub fn from_dashboard(
        dashboard: AgentReportGateDashboard,
        policy: AgentReportGateHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("agent_report_gate_history_empty".to_owned());
        } else if dashboard.acceptance_rate < policy.minimum_acceptance_rate {
            watch_reasons.push(format!(
                "agent_report_gate_acceptance_rate={:.3}<{}",
                dashboard.acceptance_rate, policy.minimum_acceptance_rate
            ));
        }

        if dashboard.blocked_records > policy.maximum_blocked_records {
            repair_reasons.push(format!(
                "agent_report_gate_blocked_records={}>{}",
                dashboard.blocked_records, policy.maximum_blocked_records
            ));
        }
        if dashboard.reason_count > policy.maximum_reason_count {
            repair_reasons.push(format!(
                "agent_report_gate_reasons={}>{}",
                dashboard.reason_count, policy.maximum_reason_count
            ));
        }
        if dashboard.follow_up_tasks > policy.maximum_follow_up_tasks {
            repair_reasons.push(format!(
                "agent_report_gate_follow_up_tasks={}>{}",
                dashboard.follow_up_tasks, policy.maximum_follow_up_tasks
            ));
        }
        if dashboard.memory_blockers > policy.maximum_memory_blockers {
            repair_reasons.push(format!(
                "agent_report_gate_memory_blockers={}>{}",
                dashboard.memory_blockers, policy.maximum_memory_blockers
            ));
        }
        if dashboard.budget_blockers > policy.maximum_budget_blockers {
            repair_reasons.push(format!(
                "agent_report_gate_budget_blockers={}>{}",
                dashboard.budget_blockers, policy.maximum_budget_blockers
            ));
        }
        if dashboard.validation_blockers > policy.maximum_validation_blockers {
            repair_reasons.push(format!(
                "agent_report_gate_validation_blockers={}>{}",
                dashboard.validation_blockers, policy.maximum_validation_blockers
            ));
        }
        if dashboard.runtime_blockers > policy.maximum_runtime_blockers {
            repair_reasons.push(format!(
                "agent_report_gate_runtime_blockers={}>{}",
                dashboard.runtime_blockers, policy.maximum_runtime_blockers
            ));
        }
        if dashboard.tool_build_blockers > policy.maximum_tool_build_blockers {
            repair_reasons.push(format!(
                "agent_report_gate_tool_build_blockers={}>{}",
                dashboard.tool_build_blockers, policy.maximum_tool_build_blockers
            ));
        }
        if dashboard.review_blockers > policy.maximum_review_blockers {
            repair_reasons.push(format!(
                "agent_report_gate_review_blockers={}>{}",
                dashboard.review_blockers, policy.maximum_review_blockers
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentReportGateHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentReportGateHealthStatus::Watch, watch_reasons)
        } else {
            (AgentReportGateHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentReportGateHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHistoryRecord {
    pub history: AgentReportGateSummaryHistory,
    pub appended_summary: AgentReportGateSummary,
    pub dashboard: AgentReportGateDashboard,
    pub health: AgentReportGateHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHistoryRecorder;

impl AgentReportGateHistoryRecord {
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

impl AgentReportGateHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentReportGateSummaryHistory,
        summary: AgentReportGateSummary,
        policy: AgentReportGateHealthPolicy,
    ) -> AgentReportGateHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = report_gate_history_record_telemetry(&dashboard, &health);

        AgentReportGateHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_decision_with_health(
        &self,
        history: AgentReportGateSummaryHistory,
        decision: &AgentReportGateDecision,
        policy: AgentReportGateHealthPolicy,
    ) -> AgentReportGateHistoryRecord {
        self.record_summary_with_health(history, decision.summary(), policy)
    }

    pub fn record_summary_with_health_gate(
        &self,
        history: AgentReportGateSummaryHistory,
        summary: AgentReportGateSummary,
        policy: AgentReportGateHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateRecord {
        let health_record = self.record_summary_with_health(history, summary, policy);
        let gate_decision =
            AgentReportGateHealthGate::new().evaluate(run_id, &health_record.health, next_queue);
        let gate_summary =
            AgentReportGateHealthGateSummary::from_record_parts(&health_record, &gate_decision);
        let telemetry = report_gate_health_gate_record_telemetry(&health_record, &gate_decision);

        AgentReportGateHealthGateRecord {
            health_record,
            gate_decision,
            gate_summary,
            telemetry,
        }
    }

    pub fn record_decision_with_health_gate(
        &self,
        history: AgentReportGateSummaryHistory,
        decision: &AgentReportGateDecision,
        policy: AgentReportGateHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateRecord {
        self.record_summary_with_health_gate(
            history,
            decision.summary(),
            policy,
            run_id,
            next_queue,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateDecision {
    pub health: AgentReportGateHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateRecord {
    pub health_record: AgentReportGateHistoryRecord,
    pub gate_decision: AgentReportGateHealthGateDecision,
    pub gate_summary: AgentReportGateHealthGateSummary,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentReportGateHealthGateSummary {
        self.gate_summary.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateSummary {
    pub health_status: AgentReportGateHealthStatus,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub history_records: usize,
    pub accepted_records: usize,
    pub blocked_records: usize,
    pub acceptance_rate: f32,
    pub blocked_rate: f32,
    pub repair_tasks: usize,
    pub next_queue_tasks: usize,
    pub blocked_reasons: usize,
    pub repair_task_ids: Vec<String>,
    pub next_queue_task_ids: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateSummary {
    pub fn from_record(record: &AgentReportGateHealthGateRecord) -> Self {
        Self::from_record_parts(&record.health_record, &record.gate_decision)
    }

    fn from_record_parts(
        health_record: &AgentReportGateHistoryRecord,
        gate_decision: &AgentReportGateHealthGateDecision,
    ) -> Self {
        let repair_task_ids = gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = gate_decision.next_queue.task_ids();
        let telemetry = report_gate_health_gate_summary_telemetry(
            health_record.health.status,
            gate_decision.admitted,
            gate_decision.requires_repair_first,
            health_record.dashboard.total_records,
            health_record.dashboard.accepted_records,
            health_record.dashboard.blocked_records,
            health_record.dashboard.acceptance_rate,
            health_record.dashboard.blocked_rate,
            repair_task_ids.len(),
            next_queue_task_ids.len(),
            gate_decision.blocked_reasons.len(),
        );

        Self {
            health_status: health_record.health.status,
            admitted: gate_decision.admitted,
            requires_repair_first: gate_decision.requires_repair_first,
            history_records: health_record.dashboard.total_records,
            accepted_records: health_record.dashboard.accepted_records,
            blocked_records: health_record.dashboard.blocked_records,
            acceptance_rate: health_record.dashboard.acceptance_rate,
            blocked_rate: health_record.dashboard.blocked_rate,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            blocked_reasons: gate_decision.blocked_reasons.len(),
            repair_task_ids,
            next_queue_task_ids,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentReportGateHealthGateSummaryHistory {
    summaries: Vec<AgentReportGateHealthGateSummary>,
}

impl AgentReportGateHealthGateSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentReportGateHealthGateSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentReportGateHealthGateSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentReportGateHealthGateSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentReportGateHealthGateSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentReportGateHealthGateDashboard {
        AgentReportGateHealthGateDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentReportGateHealthGateHealthPolicy,
    ) -> AgentReportGateHealthGateHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateDashboard {
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
    pub latest_health_status: Option<AgentReportGateHealthStatus>,
    pub latest_admitted: Option<bool>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateDashboard {
    pub fn from_summaries(summaries: &[AgentReportGateHealthGateSummary]) -> Self {
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
            .filter(|summary| summary.health_status == AgentReportGateHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.health_status == AgentReportGateHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.health_status == AgentReportGateHealthStatus::Repair)
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
        let latest_health_status = summaries.last().map(|summary| summary.health_status);
        let latest_admitted = summaries
            .last()
            .map(|summary| summary.admitted && !summary.requires_repair_first);
        let telemetry = report_gate_health_gate_dashboard_telemetry(
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
            latest_admitted,
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
            latest_admitted,
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
        policy: AgentReportGateHealthGateHealthPolicy,
    ) -> AgentReportGateHealthGateHealth {
        AgentReportGateHealthGateHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentReportGateHealthGateHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
}

impl Default for AgentReportGateHealthGateHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_admission_rate: 0.67,
            maximum_repair_first_rate: 0.0,
            maximum_repair_records: 0,
            maximum_repair_tasks: 0,
            maximum_blocked_reasons: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateHealth {
    pub status: AgentReportGateHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentReportGateHealthGateDashboard,
}

impl AgentReportGateHealthGateHealth {
    pub fn from_dashboard(
        dashboard: AgentReportGateHealthGateDashboard,
        policy: AgentReportGateHealthGateHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("agent_report_gate_health_gate_history_empty".to_owned());
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_report_gate_health_gate_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }
        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }
        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }
        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentReportGateHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentReportGateHealthStatus::Watch, watch_reasons)
        } else {
            (AgentReportGateHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentReportGateHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateHistoryRecord {
    pub history: AgentReportGateHealthGateSummaryHistory,
    pub appended_summary: AgentReportGateHealthGateSummary,
    pub dashboard: AgentReportGateHealthGateDashboard,
    pub health: AgentReportGateHealthGateHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateHistoryRecorder;

impl AgentReportGateHealthGateHistoryRecord {
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

impl AgentReportGateHealthGateHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentReportGateHealthGateSummaryHistory,
        summary: AgentReportGateHealthGateSummary,
        policy: AgentReportGateHealthGateHealthPolicy,
    ) -> AgentReportGateHealthGateHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = report_gate_health_gate_history_record_telemetry(&dashboard, &health);

        AgentReportGateHealthGateHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_gate_record_with_health(
        &self,
        history: AgentReportGateHealthGateSummaryHistory,
        record: &AgentReportGateHealthGateRecord,
        policy: AgentReportGateHealthGateHealthPolicy,
    ) -> AgentReportGateHealthGateHistoryRecord {
        self.record_summary_with_health(history, record.gate_summary.clone(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendGateDecision {
    pub trend_health: AgentReportGateHealthGateHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffRecord {
    pub trend_record: AgentReportGateHealthGateHistoryRecord,
    pub gate_decision: AgentReportGateHealthGateTrendGateDecision,
    pub handoff_summary: AgentReportGateHealthGateTrendHandoffSummary,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentReportGateHealthGateTrendHandoffSummary {
        self.handoff_summary.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReportGateHealthGateTrendHandoffSummary {
    pub trend_health_status: AgentReportGateHealthStatus,
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

impl AgentReportGateHealthGateTrendHandoffSummary {
    pub fn from_record(record: &AgentReportGateHealthGateTrendHandoffRecord) -> Self {
        Self::from_record_parts(&record.trend_record, &record.gate_decision)
    }

    fn from_record_parts(
        trend_record: &AgentReportGateHealthGateHistoryRecord,
        gate_decision: &AgentReportGateHealthGateTrendGateDecision,
    ) -> Self {
        let repair_task_ids = gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = gate_decision.next_queue.task_ids();
        let telemetry = report_gate_health_gate_trend_handoff_summary_telemetry(
            trend_record.health.status,
            gate_decision.admitted,
            gate_decision.requires_repair_first,
            trend_record.dashboard.total_records,
            repair_task_ids.len(),
            next_queue_task_ids.len(),
            gate_decision.blocked_reasons.len(),
        );

        Self {
            trend_health_status: trend_record.health.status,
            admitted: gate_decision.admitted,
            requires_repair_first: gate_decision.requires_repair_first,
            trend_records: trend_record.dashboard.total_records,
            repair_tasks: repair_task_ids.len(),
            next_queue_tasks: next_queue_task_ids.len(),
            repair_task_ids,
            next_queue_task_ids,
            blocked_reasons: gate_decision.blocked_reasons.clone(),
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentReportGateHealthGateTrendHandoffHistory {
    summaries: Vec<AgentReportGateHealthGateTrendHandoffSummary>,
}

impl AgentReportGateHealthGateTrendHandoffHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentReportGateHealthGateTrendHandoffSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentReportGateHealthGateTrendHandoffSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentReportGateHealthGateTrendHandoffSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentReportGateHealthGateTrendHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentReportGateHealthGateTrendHandoffDashboard {
        AgentReportGateHealthGateTrendHandoffDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentReportGateHealthGateTrendHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffDashboard {
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
    pub latest_trend_health_status: Option<AgentReportGateHealthStatus>,
    pub latest_blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffDashboard {
    pub fn from_summaries(summaries: &[AgentReportGateHealthGateTrendHandoffSummary]) -> Self {
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
            .filter(|summary| summary.trend_health_status == AgentReportGateHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.trend_health_status == AgentReportGateHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.trend_health_status == AgentReportGateHealthStatus::Repair)
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
        let telemetry = report_gate_health_gate_trend_handoff_dashboard_telemetry(
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
        policy: AgentReportGateHealthGateTrendHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffHealth {
        AgentReportGateHealthGateTrendHandoffHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentReportGateHealthGateTrendHandoffHealthPolicy {
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
pub struct AgentReportGateHealthGateTrendHandoffHealth {
    pub status: AgentReportGateHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentReportGateHealthGateTrendHandoffDashboard,
}

impl AgentReportGateHealthGateTrendHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentReportGateHealthGateTrendHandoffDashboard,
        policy: AgentReportGateHealthGateTrendHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons
                .push("agent_report_gate_health_gate_trend_handoff_history_empty".to_owned());
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }
        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }
        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }
        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentReportGateHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentReportGateHealthStatus::Watch, watch_reasons)
        } else {
            (AgentReportGateHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentReportGateHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffHistoryRecord {
    pub history: AgentReportGateHealthGateTrendHandoffHistory,
    pub appended_summary: AgentReportGateHealthGateTrendHandoffSummary,
    pub dashboard: AgentReportGateHealthGateTrendHandoffDashboard,
    pub health: AgentReportGateHealthGateTrendHandoffHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffHistoryRecorder;

impl AgentReportGateHealthGateTrendHandoffHistoryRecord {
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

impl AgentReportGateHealthGateTrendHandoffHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentReportGateHealthGateTrendHandoffHistory,
        summary: AgentReportGateHealthGateTrendHandoffSummary,
        policy: AgentReportGateHealthGateTrendHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            report_gate_health_gate_trend_handoff_history_record_telemetry(&dashboard, &health);

        AgentReportGateHealthGateTrendHandoffHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_handoff_with_health(
        &self,
        history: AgentReportGateHealthGateTrendHandoffHistory,
        record: &AgentReportGateHealthGateTrendHandoffRecord,
        policy: AgentReportGateHealthGateTrendHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffHistoryRecord {
        self.record_summary_with_health(history, record.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffGateDecision {
    pub requested_admitted: bool,
    pub handoff_health: AgentReportGateHealthGateTrendHandoffHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffGate;

impl AgentReportGateHealthGateTrendHandoffGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        handoff: &AgentReportGateHealthGateTrendHandoffRecord,
        history_record: &AgentReportGateHealthGateTrendHandoffHistoryRecord,
    ) -> AgentReportGateHealthGateTrendHandoffGateDecision {
        let requested_admitted = handoff.is_admitted();
        let handoff_health = history_record.health.clone();
        let trend_requires_repair = handoff_health.status == AgentReportGateHealthStatus::Repair;
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
                    agent_report_gate_health_gate_trend_handoff_repair_task(
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
        let telemetry = report_gate_health_gate_trend_handoff_gate_telemetry(
            handoff_health.status,
            requested_admitted,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            &blocked_reasons,
        );

        AgentReportGateHealthGateTrendHandoffGateDecision {
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorRecord {
    pub handoff: AgentReportGateHealthGateTrendHandoffRecord,
    pub history_record: AgentReportGateHealthGateTrendHandoffHistoryRecord,
    pub gate_decision: AgentReportGateHealthGateTrendHandoffGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentReportGateHealthGateTrendHandoffMonitorSummary {
        AgentReportGateHealthGateTrendHandoffMonitorSummary::from_monitor(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitor {
    history_recorder: AgentReportGateHealthGateTrendHandoffHistoryRecorder,
    gate: AgentReportGateHealthGateTrendHandoffGate,
}

impl AgentReportGateHealthGateTrendHandoffMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        handoff: AgentReportGateHealthGateTrendHandoffRecord,
        history: AgentReportGateHealthGateTrendHandoffHistory,
        policy: AgentReportGateHealthGateTrendHandoffHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorRecord {
        let history_record = self
            .history_recorder
            .record_handoff_with_health(history, &handoff, policy);
        let gate_decision = self.gate.evaluate(run_id, &handoff, &history_record);
        let telemetry = report_gate_health_gate_trend_handoff_monitor_telemetry(
            &handoff,
            &history_record,
            &gate_decision,
        );

        AgentReportGateHealthGateTrendHandoffMonitorRecord {
            handoff,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorSummary {
    pub handoff_health_status: AgentReportGateHealthStatus,
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

impl AgentReportGateHealthGateTrendHandoffMonitorSummary {
    pub fn from_monitor(record: &AgentReportGateHealthGateTrendHandoffMonitorRecord) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry = report_gate_health_gate_trend_handoff_monitor_summary_telemetry(
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory {
    summaries: Vec<AgentReportGateHealthGateTrendHandoffMonitorSummary>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentReportGateHealthGateTrendHandoffMonitorSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentReportGateHealthGateTrendHandoffMonitorSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentReportGateHealthGateTrendHandoffMonitorSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentReportGateHealthGateTrendHandoffMonitorSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentReportGateHealthGateTrendHandoffMonitorDashboard {
        AgentReportGateHealthGateTrendHandoffMonitorDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorDashboard {
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
    pub latest_handoff_health_status: Option<AgentReportGateHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorDashboard {
    pub fn from_summaries(
        summaries: &[AgentReportGateHealthGateTrendHandoffMonitorSummary],
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
            .filter(|summary| summary.handoff_health_status == AgentReportGateHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.handoff_health_status == AgentReportGateHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.handoff_health_status == AgentReportGateHealthStatus::Repair)
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
        let telemetry = report_gate_health_gate_trend_handoff_monitor_dashboard_telemetry(
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
        policy: AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHealth {
        AgentReportGateHealthGateTrendHandoffMonitorHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy {
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHealth {
    pub status: AgentReportGateHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentReportGateHealthGateTrendHandoffMonitorDashboard,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHealth {
    pub fn from_dashboard(
        dashboard: AgentReportGateHealthGateTrendHandoffMonitorDashboard,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "agent_report_gate_health_gate_trend_handoff_monitor_history_empty".to_owned(),
            );
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }
        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }
        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }
        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentReportGateHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentReportGateHealthStatus::Watch, watch_reasons)
        } else {
            (AgentReportGateHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentReportGateHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecord {
    pub history: AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory,
    pub appended_summary: AgentReportGateHealthGateTrendHandoffMonitorSummary,
    pub dashboard: AgentReportGateHealthGateTrendHandoffMonitorDashboard,
    pub health: AgentReportGateHealthGateTrendHandoffMonitorHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecorder;

impl AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecord {
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

impl AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory,
        summary: AgentReportGateHealthGateTrendHandoffMonitorSummary,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = report_gate_health_gate_trend_handoff_monitor_history_record_telemetry(
            &dashboard, &health,
        );

        AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_monitor_with_health(
        &self,
        history: AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory,
        monitor: &AgentReportGateHealthGateTrendHandoffMonitorRecord,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecord {
        self.record_summary_with_health(history, monitor.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorGateDecision {
    pub requested_admitted: bool,
    pub monitor_health: AgentReportGateHealthGateTrendHandoffMonitorHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorGate;

impl AgentReportGateHealthGateTrendHandoffMonitorGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        monitor: &AgentReportGateHealthGateTrendHandoffMonitorRecord,
        history_record: &AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecord,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorGateDecision {
        let requested_admitted = monitor.is_admitted();
        let monitor_health = history_record.health.clone();
        let monitor_requires_repair = monitor_health.status == AgentReportGateHealthStatus::Repair;
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
                    agent_report_gate_health_gate_trend_handoff_monitor_repair_task(
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
        let telemetry = report_gate_health_gate_trend_handoff_monitor_gate_telemetry(
            monitor_health.status,
            requested_admitted,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            &blocked_reasons,
        );

        AgentReportGateHealthGateTrendHandoffMonitorGateDecision {
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord {
    pub monitor: AgentReportGateHealthGateTrendHandoffMonitorRecord,
    pub history_record: AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecord,
    pub gate_decision: AgentReportGateHealthGateTrendHandoffMonitorGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary {
        AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary::from_handoff(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoff {
    history_recorder: AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecorder,
    gate: AgentReportGateHealthGateTrendHandoffMonitorGate,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        monitor: AgentReportGateHealthGateTrendHandoffMonitorRecord,
        history: AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord {
        let history_record = self
            .history_recorder
            .record_monitor_with_health(history, &monitor, policy);
        let gate_decision = self.gate.evaluate(run_id, &monitor, &history_record);
        let telemetry = report_gate_health_gate_trend_handoff_monitor_handoff_telemetry(
            &monitor,
            &history_record,
            &gate_decision,
        );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord {
            monitor,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary {
    pub monitor_health_status: AgentReportGateHealthStatus,
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

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary {
    pub fn from_handoff(
        record: &AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord,
    ) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry = report_gate_health_gate_trend_handoff_monitor_handoff_summary_telemetry(
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory {
    summaries: Vec<AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffDashboard {
        AgentReportGateHealthGateTrendHandoffMonitorHandoffDashboard::from_summaries(
            &self.summaries,
        )
    }

    pub fn health(
        &self,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffDashboard {
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
    pub latest_monitor_health_status: Option<AgentReportGateHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffDashboard {
    pub fn from_summaries(
        summaries: &[AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary],
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
            .filter(|summary| summary.monitor_health_status == AgentReportGateHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.monitor_health_status == AgentReportGateHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.monitor_health_status == AgentReportGateHealthStatus::Repair)
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
        let telemetry = report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_telemetry(
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
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHealth {
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy {
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHealth {
    pub status: AgentReportGateHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentReportGateHealthGateTrendHandoffMonitorHandoffDashboard,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentReportGateHealthGateTrendHandoffMonitorHandoffDashboard,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_empty"
                    .to_owned(),
            );
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentReportGateHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentReportGateHealthStatus::Watch, watch_reasons)
        } else {
            (AgentReportGateHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentReportGateHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
    pub history: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory,
    pub appended_summary: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary,
    pub dashboard: AgentReportGateHealthGateTrendHandoffMonitorHandoffDashboard,
    pub health: AgentReportGateHealthGateTrendHandoffMonitorHandoffHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder;

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
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

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        summary: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            report_gate_health_gate_trend_handoff_monitor_handoff_history_record_telemetry(
                &dashboard, &health,
            );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_handoff_with_health(
        &self,
        history: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        handoff: &AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord {
        self.record_summary_with_health(history, handoff.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffGateDecision {
    pub requested_admitted: bool,
    pub handoff_health: AgentReportGateHealthGateTrendHandoffMonitorHandoffHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffGate;

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        handoff: &AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord,
        history_record: &AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffGateDecision {
        let requested_admitted = handoff.is_admitted();
        let handoff_health = history_record.health.clone();
        let handoff_requires_repair = handoff_health.status == AgentReportGateHealthStatus::Repair;
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
                    agent_report_gate_health_gate_trend_handoff_monitor_handoff_repair_task(
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
        let telemetry = report_gate_health_gate_trend_handoff_monitor_handoff_gate_telemetry(
            handoff_health.status,
            requested_admitted,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            next_queue.len(),
            &blocked_reasons,
        );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffGateDecision {
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord {
    pub handoff: AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord,
    pub history_record: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord,
    pub gate_decision: AgentReportGateHealthGateTrendHandoffMonitorHandoffGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(&self) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary {
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary::from_handoff(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary {
    pub handoff_health_status: AgentReportGateHealthStatus,
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

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary {
    pub fn from_handoff(
        record: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord,
    ) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry =
            report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_telemetry(
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory {
    summaries: Vec<AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(
        &mut self,
        summary: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary,
    ) {
        self.summaries.push(summary);
    }

    pub fn latest(
        &self,
    ) -> Option<&AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary> {
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
    ) -> &[AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffDashboard {
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffDashboard::from_summaries(
            &self.summaries,
        )
    }

    pub fn health(
        &self,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffDashboard {
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
    pub latest_handoff_health_status: Option<AgentReportGateHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffDashboard {
    pub fn from_summaries(
        summaries: &[AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary],
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
            .filter(|summary| summary.handoff_health_status == AgentReportGateHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.handoff_health_status == AgentReportGateHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.handoff_health_status == AgentReportGateHealthStatus::Repair)
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
            report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_telemetry(
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
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealth {
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy {
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealth {
    pub status: AgentReportGateHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffDashboard,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffDashboard,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_empty"
                    .to_owned(),
            );
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentReportGateHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentReportGateHealthStatus::Watch, watch_reasons)
        } else {
            (AgentReportGateHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentReportGateHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord {
    pub history: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
    pub appended_summary: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary,
    pub dashboard: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffDashboard,
    pub health: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder;

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord {
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

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
        summary: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_telemetry(
                &dashboard, &health,
            );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_handoff_with_health(
        &self,
        history: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
        handoff: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord {
        self.record_summary_with_health(history, handoff.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGateDecision {
    pub requested_admitted: bool,
    pub packet_health: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGate;

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        handoff: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord,
        history_record: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGateDecision {
        let requested_admitted = handoff.is_admitted();
        let packet_health = history_record.health.clone();
        let packet_requires_repair = packet_health.status == AgentReportGateHealthStatus::Repair;
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
                    agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_repair_task(
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
        if packet_requires_repair {
            blocked_reasons.extend(packet_health.reasons.clone());
        }
        let telemetry =
            report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_telemetry(
                packet_health.status,
                requested_admitted,
                admitted,
                requires_repair_first,
                repair_tasks.len(),
                next_queue.len(),
                &blocked_reasons,
            );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGateDecision {
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord {
    pub packet: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord,
    pub history_record:
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord,
    pub gate_decision: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(
        &self,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary {
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary::from_admission(
            self,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary {
    pub packet_health_status: AgentReportGateHealthStatus,
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

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary {
    pub fn from_admission(
        record: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
    ) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry =
            report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_telemetry(
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory {
    summaries: Vec<AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(
        summaries: Vec<AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary>,
    ) -> Self {
        Self { summaries }
    }

    pub fn push(
        &mut self,
        summary: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary,
    ) {
        self.summaries.push(summary);
    }

    pub fn latest(
        &self,
    ) -> Option<&AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary> {
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
    ) -> &[AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary] {
        &self.summaries
    }

    pub fn dashboard(
        &self,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard {
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard::from_summaries(
            &self.summaries,
        )
    }

    pub fn health(
        &self,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard {
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
    pub latest_packet_health_status: Option<AgentReportGateHealthStatus>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard {
    pub fn from_summaries(
        summaries: &[AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary],
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
            .filter(|summary| summary.packet_health_status == AgentReportGateHealthStatus::Stable)
            .count();
        let watch_records = summaries
            .iter()
            .filter(|summary| summary.packet_health_status == AgentReportGateHealthStatus::Watch)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.packet_health_status == AgentReportGateHealthStatus::Repair)
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
            report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_telemetry(
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
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth {
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth::from_dashboard(
            self.clone(),
            policy,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy {
    pub minimum_admission_rate: f32,
    pub maximum_repair_first_rate: f32,
    pub maximum_repair_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_watch_records: usize,
}

impl Default for AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy {
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth {
    pub status: AgentReportGateHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth {
    pub fn from_dashboard(
        dashboard: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_empty"
                    .to_owned(),
            );
        } else if dashboard.admission_rate < policy.minimum_admission_rate {
            watch_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_admission_rate={:.3}<{}",
                dashboard.admission_rate, policy.minimum_admission_rate
            ));
        }

        if dashboard.watch_records > policy.maximum_watch_records {
            watch_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_watch_records={}>{}",
                dashboard.watch_records, policy.maximum_watch_records
            ));
        }

        if dashboard.repair_first_rate > policy.maximum_repair_first_rate {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_first_rate={:.3}>{}",
                dashboard.repair_first_rate, policy.maximum_repair_first_rate
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_task_count > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_tasks={}>{}",
                dashboard.repair_task_count, policy.maximum_repair_tasks
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentReportGateHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentReportGateHealthStatus::Watch, watch_reasons)
        } else {
            (AgentReportGateHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentReportGateHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentReportGateHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord {
    pub history: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
    pub appended_summary: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary,
    pub dashboard: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard,
    pub health: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder;

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord {
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

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
        summary: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry =
            report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_telemetry(
                &dashboard, &health,
            );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_admission_with_health(
        &self,
        history: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
        admission: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord {
        self.record_summary_with_health(history, admission.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision {
    pub requested_admitted: bool,
    pub admission_health: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth,
    pub admitted: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub next_queue: AgentTaskQueue,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision {
    pub fn is_admitted(&self) -> bool {
        self.admitted && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffGate;

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        admission: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
        history_record: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision {
        let requested_admitted = admission.is_admitted();
        let admission_health = history_record.health.clone();
        let admission_requires_repair =
            admission_health.status == AgentReportGateHealthStatus::Repair;
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
                    agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_task(
                        run_id.as_ref(),
                        index,
                        reason,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut next_queue = admission.next_queue();
        for task in &repair_tasks {
            next_queue.push(task.clone());
        }
        let mut blocked_reasons = admission.gate_decision.blocked_reasons.clone();
        if admission_requires_repair {
            blocked_reasons.extend(admission_health.reasons.clone());
        }
        let telemetry =
            report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_telemetry(
                admission_health.status,
                requested_admitted,
                admitted,
                requires_repair_first,
                repair_tasks.len(),
                next_queue.len(),
                &blocked_reasons,
            );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision {
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord {
    pub admission: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
    pub history_record:
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord,
    pub gate_decision:
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision,
    pub telemetry: Vec<String>,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord {
    pub fn is_admitted(&self) -> bool {
        self.gate_decision.is_admitted()
    }

    pub fn next_queue(&self) -> AgentTaskQueue {
        self.gate_decision.next_queue.clone()
    }

    pub fn summary(
        &self,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffSummary {
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffSummary::from_handoff(
            self,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffSummary {
    pub admission_health_status: AgentReportGateHealthStatus,
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

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffSummary {
    pub fn from_handoff(
        record: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord,
    ) -> Self {
        let repair_task_ids = record
            .gate_decision
            .repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let next_queue_task_ids = record.gate_decision.next_queue.task_ids();
        let telemetry =
            report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_telemetry(
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
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff {
    history_recorder:
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder,
    gate: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffGate,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        admission: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
        history: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord {
        let history_record = self
            .history_recorder
            .record_admission_with_health(history, &admission, policy);
        let gate_decision = self.gate.evaluate(run_id, &admission, &history_record);
        let telemetry =
            report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_telemetry(
                &admission,
                &history_record,
                &gate_decision,
            );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoffRecord {
            admission,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoff {
    history_recorder:
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder,
    gate: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGate,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        packet: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord,
        history: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord {
        let history_record = self
            .history_recorder
            .record_handoff_with_health(history, &packet, policy);
        let gate_decision = self.gate.evaluate(run_id, &packet, &history_record);
        let telemetry =
            report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_telemetry(
                &packet,
                &history_record,
                &gate_decision,
            );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord {
            packet,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoff {
    history_recorder: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder,
    gate: AgentReportGateHealthGateTrendHandoffMonitorHandoffGate,
}

impl AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_and_gate(
        &self,
        handoff: AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord,
        history: AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory,
        policy: AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy,
        run_id: impl AsRef<str>,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord {
        let history_record = self
            .history_recorder
            .record_handoff_with_health(history, &handoff, policy);
        let gate_decision = self.gate.evaluate(run_id, &handoff, &history_record);
        let telemetry = report_gate_health_gate_trend_handoff_monitor_handoff_handoff_telemetry(
            &handoff,
            &history_record,
            &gate_decision,
        );

        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord {
            handoff,
            history_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendHandoff;

impl AgentReportGateHealthGateTrendHandoff {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_and_gate(
        &self,
        history: AgentReportGateHealthGateSummaryHistory,
        summary: AgentReportGateHealthGateSummary,
        policy: AgentReportGateHealthGateHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateTrendHandoffRecord {
        let trend_record = AgentReportGateHealthGateHistoryRecorder::new()
            .record_summary_with_health(history, summary, policy);
        self.gate_record(run_id, trend_record, next_queue)
    }

    pub fn record_gate_record_and_gate(
        &self,
        history: AgentReportGateHealthGateSummaryHistory,
        record: &AgentReportGateHealthGateRecord,
        policy: AgentReportGateHealthGateHealthPolicy,
        run_id: impl AsRef<str>,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateTrendHandoffRecord {
        let trend_record = AgentReportGateHealthGateHistoryRecorder::new()
            .record_gate_record_with_health(history, record, policy);
        self.gate_record(run_id, trend_record, next_queue)
    }

    pub fn gate_record(
        &self,
        run_id: impl AsRef<str>,
        trend_record: AgentReportGateHealthGateHistoryRecord,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateTrendHandoffRecord {
        let gate_decision = AgentReportGateHealthGateTrendGate::new().evaluate(
            run_id,
            &trend_record.health,
            next_queue,
        );
        let handoff_summary = AgentReportGateHealthGateTrendHandoffSummary::from_record_parts(
            &trend_record,
            &gate_decision,
        );
        let telemetry =
            report_gate_health_gate_trend_handoff_telemetry(&trend_record, &handoff_summary);

        AgentReportGateHealthGateTrendHandoffRecord {
            trend_record,
            gate_decision,
            handoff_summary,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentReportGateHealthGateTrendGate;

impl AgentReportGateHealthGateTrendGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        trend_health: &AgentReportGateHealthGateHealth,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateTrendGateDecision {
        let requires_repair_first = trend_health.status == AgentReportGateHealthStatus::Repair;
        let admitted = !requires_repair_first;
        let repair_tasks = if requires_repair_first {
            trend_health
                .reasons
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, reason)| {
                    agent_report_gate_health_gate_trend_repair_task(run_id.as_ref(), index, reason)
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
            trend_health.reasons.clone()
        } else {
            Vec::new()
        };
        let telemetry = report_gate_health_gate_trend_gate_telemetry(
            trend_health.status,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            merged_queue.len(),
            &blocked_reasons,
        );

        AgentReportGateHealthGateTrendGateDecision {
            trend_health: trend_health.clone(),
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
pub struct AgentReportGateHealthGate;

impl AgentReportGateHealthGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        run_id: impl AsRef<str>,
        health: &AgentReportGateHealth,
        next_queue: &AgentTaskQueue,
    ) -> AgentReportGateHealthGateDecision {
        let requires_repair_first = health.status == AgentReportGateHealthStatus::Repair;
        let admitted = !requires_repair_first;
        let repair_tasks = if requires_repair_first {
            health
                .reasons
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, reason)| {
                    agent_report_gate_health_repair_task(run_id.as_ref(), index, reason)
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
        let telemetry = report_gate_health_gate_telemetry(
            health.status,
            admitted,
            requires_repair_first,
            repair_tasks.len(),
            merged_queue.len(),
            &blocked_reasons,
        );

        AgentReportGateHealthGateDecision {
            health: health.clone(),
            admitted,
            requires_repair_first,
            repair_tasks,
            next_queue: merged_queue,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentReportGate {
    policy: AgentReportGatePolicy,
}

impl Default for AgentReportGate {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentReportGate {
    pub fn new() -> Self {
        Self {
            policy: AgentReportGatePolicy::default(),
        }
    }

    pub fn with_policy(policy: AgentReportGatePolicy) -> Self {
        Self { policy }
    }

    pub fn evaluate(&self, record: &AgentCycleLedgerRecord) -> AgentReportGateDecision {
        let reasons = self.reasons(record);
        let follow_up_tasks = follow_up_tasks(&record.run_id, &reasons);

        AgentReportGateDecision {
            accepted: reasons.is_empty(),
            reasons,
            follow_up_tasks,
        }
    }

    fn reasons(&self, record: &AgentCycleLedgerRecord) -> Vec<AgentReportGateReason> {
        let summary = &record.summary;
        let mut reasons = Vec::new();

        if summary.execution_failures > 0 {
            reasons.push(AgentReportGateReason::new(
                "execution_failures",
                summary.execution_failures.to_string(),
            ));
        }
        if summary.unresolved_conflicts > 0 {
            reasons.push(AgentReportGateReason::new(
                "unresolved_conflicts",
                summary.unresolved_conflicts.to_string(),
            ));
        }
        if summary.budget_overspends > 0 {
            reasons.push(AgentReportGateReason::new(
                "budget_overspends",
                summary.budget_overspends.to_string(),
            ));
        }
        if summary.blocked_side_effects > 0 {
            reasons.push(AgentReportGateReason::new(
                "blocked_side_effects",
                summary.blocked_side_effects.to_string(),
            ));
        }
        if summary.tool_build_missing_requests > 0 {
            reasons.push(AgentReportGateReason::new(
                "tool_build_missing_requests",
                summary.tool_build_missing_requests.to_string(),
            ));
        }
        if summary.tool_build_unexpected_receipts > 0 {
            reasons.push(AgentReportGateReason::new(
                "tool_build_unexpected_receipts",
                summary.tool_build_unexpected_receipts.to_string(),
            ));
        }
        if summary.tool_build_duplicate_receipts > 0 {
            reasons.push(AgentReportGateReason::new(
                "tool_build_duplicate_receipts",
                summary.tool_build_duplicate_receipts.to_string(),
            ));
        }
        if summary.tool_build_held_receipts > 0 {
            reasons.push(AgentReportGateReason::new(
                "tool_build_held_receipts",
                summary.tool_build_held_receipts.to_string(),
            ));
        }
        if summary.tool_build_rejected_receipts > 0 {
            reasons.push(AgentReportGateReason::new(
                "tool_build_rejected_receipts",
                summary.tool_build_rejected_receipts.to_string(),
            ));
        }
        if self.policy.require_reinforce_action && summary.reward_action != RewardAction::Reinforce
        {
            reasons.push(AgentReportGateReason::new(
                "reward_action",
                summary.reward_action.as_str(),
            ));
        }
        if summary.reward_total < self.policy.minimum_reward_total {
            reasons.push(AgentReportGateReason::new(
                "reward_total_below_policy",
                format!(
                    "{:.3}<{}",
                    summary.reward_total, self.policy.minimum_reward_total
                ),
            ));
        }
        if self.policy.require_validation_evidence && !record.evidence.has_validation_evidence() {
            reasons.push(AgentReportGateReason::new(
                "validation_evidence_missing",
                "validation gate did not pass with a stable evidence reference",
            ));
        }
        if self.policy.require_runtime_evidence && !record.evidence.has_runtime_evidence() {
            reasons.push(AgentReportGateReason::new(
                "runtime_evidence_missing",
                "runtime response gate did not pass with a stable evidence reference",
            ));
        }

        self.memory_reasons(record, &mut reasons);

        reasons
    }

    fn memory_promotion_reasons(
        &self,
        record: &AgentCycleLedgerRecord,
        reasons: &mut Vec<AgentReportGateReason>,
    ) -> bool {
        let Some(promotion) = &record.memory_promotion else {
            return false;
        };

        if promotion.requires_repair_first {
            reasons.push(AgentReportGateReason::new(
                "memory_promotion_repair_required",
                promotion_detail(promotion),
            ));
            return true;
        }

        if record.summary.memory_promotions == 0 {
            return false;
        }

        if promotion.is_promotable() {
            return false;
        }

        let code = match promotion.status {
            MemoryPromotionLedgerStatus::NoCandidates => "memory_promotion_no_candidates",
            MemoryPromotionLedgerStatus::Watch => "memory_promotion_watch",
            MemoryPromotionLedgerStatus::Blocked => "memory_promotion_blocked",
            MemoryPromotionLedgerStatus::Repair => "memory_promotion_repair_required",
            MemoryPromotionLedgerStatus::Promotable => "memory_promotion_blocked",
        };
        reasons.push(AgentReportGateReason::new(
            code,
            promotion_detail(promotion),
        ));
        true
    }

    fn memory_reasons(
        &self,
        record: &AgentCycleLedgerRecord,
        reasons: &mut Vec<AgentReportGateReason>,
    ) {
        let promotion_blocked_submission = self.memory_promotion_reasons(record, reasons);
        if promotion_blocked_submission {
            return;
        }

        let Some(submission) = &record.memory_submission else {
            if self.policy.require_memory_submission_for_promotions
                && record.summary.memory_promotions > 0
            {
                reasons.push(AgentReportGateReason::new(
                    "memory_submission_missing",
                    record.summary.memory_promotions.to_string(),
                ));
            }
            return;
        };

        if !submission.blocked_reasons.is_empty() {
            reasons.push(AgentReportGateReason::new(
                "memory_submission_blocked",
                submission.blocked_reasons.join(";"),
            ));
        }
        if !submission.failures.is_empty() {
            reasons.push(AgentReportGateReason::new(
                "memory_submission_failures",
                submission.failures.len().to_string(),
            ));
        }
        if self.policy.require_memory_submission_for_promotions
            && record.summary.memory_promotions > submission.submitted.len()
        {
            reasons.push(AgentReportGateReason::new(
                "memory_submission_partial",
                format!(
                    "{}<{}",
                    submission.submitted.len(),
                    record.summary.memory_promotions
                ),
            ));
        }
    }
}

fn follow_up_tasks(run_id: &str, reasons: &[AgentReportGateReason]) -> Vec<AgentTask> {
    let mut seen_kinds = BTreeSet::new();
    let mut tasks = Vec::new();

    for reason in reasons {
        let (kind, role, lane, priority, budget) = follow_up_shape(reason.code.as_str());
        if !seen_kinds.insert(kind) {
            continue;
        }
        tasks.push(
            AgentTask::new(
                format!("report-gate-{}-{}", stable_id(run_id), kind),
                role,
                format!("repair agent report gate: {}", reason.as_line()),
                budget,
            )
            .with_lane(lane)
            .with_priority(priority),
        );
    }

    tasks
}

fn is_memory_blocker_code(code: &str) -> bool {
    code.starts_with("memory_submission") || code.starts_with("memory_promotion")
}

fn is_tool_build_blocker_code(code: &str) -> bool {
    code.starts_with("tool_build_")
}

fn promotion_detail(summary: &MemoryPromotionLedgerSummary) -> String {
    if summary.reasons.is_empty() {
        return summary.status.as_str().to_owned();
    }

    summary.reasons.join(";")
}

fn memory_promotion_ledger_summary_telemetry(
    status: MemoryPromotionLedgerStatus,
    candidate_notes: usize,
    can_submit_memory: bool,
    requires_repair_first: bool,
    reason_count: usize,
    repair_tasks: usize,
) -> Vec<String> {
    vec![
        "agent_memory_promotion_ledger_summary=true".to_owned(),
        format!(
            "agent_memory_promotion_ledger_summary_status={}",
            status.as_str()
        ),
        format!("agent_memory_promotion_ledger_summary_candidate_notes={candidate_notes}"),
        format!("agent_memory_promotion_ledger_summary_can_submit_memory={can_submit_memory}"),
        format!(
            "agent_memory_promotion_ledger_summary_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_memory_promotion_ledger_summary_reasons={reason_count}"),
        format!("agent_memory_promotion_ledger_summary_repair_tasks={repair_tasks}"),
    ]
}

fn follow_up_shape(code: &str) -> (&'static str, AgentRole, &'static str, u8, AgentBudget) {
    if is_memory_blocker_code(code) {
        return (
            "memory",
            AgentRole::MemoryCurator,
            "eval-memory",
            7,
            AgentBudget::new(16, 1, 1),
        );
    }

    match code {
        "validation_evidence_missing" | "runtime_evidence_missing" => (
            "validation",
            AgentRole::Tester,
            "eval-validation",
            7,
            AgentBudget::new(24, 2, 1),
        ),
        "budget_overspends" => (
            "budget",
            AgentRole::Planner,
            "eval-budget",
            8,
            AgentBudget::new(16, 1, 1),
        ),
        _ if is_tool_build_blocker_code(code) => (
            "tool-build",
            AgentRole::Reviewer,
            "eval-tool-build",
            8,
            AgentBudget::new(24, 2, 1),
        ),
        _ => (
            "review",
            AgentRole::Reviewer,
            "eval-review",
            8,
            AgentBudget::new(24, 2, 1),
        ),
    }
}

fn agent_report_gate_health_repair_task(run_id: &str, index: usize, reason: String) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-report-gate-health-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair eval report gate trend: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("eval-report-gate-health")
    .with_priority(8)
}

fn agent_report_gate_health_gate_trend_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-report-gate-health-gate-trend-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair eval report gate health-gate trend: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("eval-report-gate-health-gate-trend")
    .with_priority(9)
}

fn agent_report_gate_health_gate_trend_handoff_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-report-gate-health-gate-trend-handoff-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair eval report gate health-gate trend handoff: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("eval-report-gate-health-gate-trend-handoff")
    .with_priority(10)
}

fn agent_report_gate_health_gate_trend_handoff_monitor_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-report-gate-health-gate-trend-handoff-monitor-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair eval report gate health-gate trend handoff monitor: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("eval-report-gate-health-gate-trend-handoff-monitor")
    .with_priority(11)
}

fn agent_report_gate_health_gate_trend_handoff_monitor_handoff_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-report-gate-health-gate-trend-handoff-monitor-handoff-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!("repair eval report gate health-gate trend handoff monitor handoff: {reason}"),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("eval-report-gate-health-gate-trend-handoff-monitor-handoff")
    .with_priority(12)
}

fn agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-report-gate-health-gate-trend-handoff-monitor-handoff-handoff-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!(
            "repair eval report gate health-gate trend handoff monitor handoff packet: {reason}"
        ),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("eval-report-gate-health-gate-trend-handoff-monitor-handoff-handoff")
    .with_priority(13)
}

fn agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_task(
    run_id: &str,
    index: usize,
    reason: String,
) -> AgentTask {
    AgentTask::new(
        format!(
            "agent-report-gate-health-gate-trend-handoff-monitor-handoff-handoff-handoff-repair-{}-{}-{}",
            stable_id(run_id),
            index,
            stable_id(&reason)
        ),
        AgentRole::Reviewer,
        format!(
            "repair eval report gate health-gate trend handoff monitor final admission: {reason}"
        ),
        AgentBudget::new(16, 1, 1),
    )
    .with_lane("eval-report-gate-health-gate-trend-handoff-monitor-handoff-handoff-handoff")
    .with_priority(14)
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

fn ordered_unique_roles<I>(items: I) -> Vec<AgentRole>
where
    I: IntoIterator<Item = AgentRole>,
{
    let mut unique = Vec::new();
    for item in items {
        if !unique.iter().any(|existing| existing == &item) {
            unique.push(item);
        }
    }
    unique
}

fn rate(count: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        count as f32 / total as f32
    }
}

fn report_gate_summary_telemetry(
    accepted: bool,
    reason_count: usize,
    follow_up_tasks: usize,
    repair_lanes: usize,
    repair_roles: usize,
    has_memory_blocker: bool,
    has_budget_blocker: bool,
    has_validation_blocker: bool,
    has_runtime_blocker: bool,
    has_tool_build_blocker: bool,
    has_review_blocker: bool,
) -> Vec<String> {
    vec![
        "agent_report_gate_summary=true".to_owned(),
        format!("agent_report_gate_summary_accepted={accepted}"),
        format!("agent_report_gate_summary_reasons={reason_count}"),
        format!("agent_report_gate_summary_follow_up_tasks={follow_up_tasks}"),
        format!("agent_report_gate_summary_repair_lanes={repair_lanes}"),
        format!("agent_report_gate_summary_repair_roles={repair_roles}"),
        format!("agent_report_gate_summary_memory_blocker={has_memory_blocker}"),
        format!("agent_report_gate_summary_budget_blocker={has_budget_blocker}"),
        format!("agent_report_gate_summary_validation_blocker={has_validation_blocker}"),
        format!("agent_report_gate_summary_runtime_blocker={has_runtime_blocker}"),
        format!("agent_report_gate_summary_tool_build_blocker={has_tool_build_blocker}"),
        format!("agent_report_gate_summary_review_blocker={has_review_blocker}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn report_gate_dashboard_telemetry(
    total_records: usize,
    accepted_records: usize,
    blocked_records: usize,
    reason_count: usize,
    follow_up_tasks: usize,
    memory_blockers: usize,
    budget_blockers: usize,
    validation_blockers: usize,
    runtime_blockers: usize,
    tool_build_blockers: usize,
    review_blockers: usize,
    acceptance_rate: f32,
    blocked_rate: f32,
    latest_accepted: Option<bool>,
) -> Vec<String> {
    vec![
        "agent_report_gate_dashboard=true".to_owned(),
        format!("agent_report_gate_dashboard_records={total_records}"),
        format!("agent_report_gate_dashboard_accepted={accepted_records}"),
        format!("agent_report_gate_dashboard_blocked={blocked_records}"),
        format!("agent_report_gate_dashboard_reasons={reason_count}"),
        format!("agent_report_gate_dashboard_follow_up_tasks={follow_up_tasks}"),
        format!("agent_report_gate_dashboard_memory_blockers={memory_blockers}"),
        format!("agent_report_gate_dashboard_budget_blockers={budget_blockers}"),
        format!("agent_report_gate_dashboard_validation_blockers={validation_blockers}"),
        format!("agent_report_gate_dashboard_runtime_blockers={runtime_blockers}"),
        format!("agent_report_gate_dashboard_tool_build_blockers={tool_build_blockers}"),
        format!("agent_report_gate_dashboard_review_blockers={review_blockers}"),
        format!("agent_report_gate_dashboard_acceptance_rate={acceptance_rate:.3}"),
        format!("agent_report_gate_dashboard_blocked_rate={blocked_rate:.3}"),
        format!(
            "agent_report_gate_dashboard_latest_accepted={}",
            latest_accepted
                .map(|accepted| accepted.to_string())
                .unwrap_or_else(|| "none".to_owned())
        ),
    ]
}

fn report_gate_history_record_telemetry(
    dashboard: &AgentReportGateDashboard,
    health: &AgentReportGateHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_history_record=true".to_owned(),
        format!(
            "agent_report_gate_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_report_gate_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_report_gate_history_record_acceptance_rate={:.3}",
            dashboard.acceptance_rate
        ),
        format!(
            "agent_report_gate_history_record_blocked_rate={:.3}",
            dashboard.blocked_rate
        ),
        format!(
            "agent_report_gate_history_record_reasons={}",
            dashboard.reason_count
        ),
        format!(
            "agent_report_gate_history_record_follow_up_tasks={}",
            dashboard.follow_up_tasks
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_report_gate_history_record_reason={reason}")),
    );
    telemetry
}

fn report_gate_health_gate_telemetry(
    status: AgentReportGateHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate=true".to_owned(),
        format!("agent_report_gate_health_gate_status={}", status.as_str()),
        format!("agent_report_gate_health_gate_admitted={admitted}"),
        format!("agent_report_gate_health_gate_repair_first={requires_repair_first}"),
        format!("agent_report_gate_health_gate_repair_tasks={repair_tasks}"),
        format!("agent_report_gate_health_gate_next_queue_tasks={next_queue_tasks}"),
        format!(
            "agent_report_gate_health_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        blocked_reasons
            .iter()
            .map(|reason| format!("agent_report_gate_health_gate_reason={reason}")),
    );
    telemetry
}

fn report_gate_health_gate_trend_gate_telemetry(
    status: AgentReportGateHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_gate=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_gate_status={}",
            status.as_str()
        ),
        format!("agent_report_gate_health_gate_trend_gate_admitted={admitted}"),
        format!("agent_report_gate_health_gate_trend_gate_repair_first={requires_repair_first}"),
        format!("agent_report_gate_health_gate_trend_gate_repair_tasks={repair_tasks}"),
        format!("agent_report_gate_health_gate_trend_gate_next_queue_tasks={next_queue_tasks}"),
        format!(
            "agent_report_gate_health_gate_trend_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        blocked_reasons
            .iter()
            .map(|reason| format!("agent_report_gate_health_gate_trend_gate_reason={reason}")),
    );
    telemetry
}

fn report_gate_health_gate_trend_handoff_telemetry(
    trend_record: &AgentReportGateHealthGateHistoryRecord,
    handoff_summary: &AgentReportGateHealthGateTrendHandoffSummary,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_status={}",
            trend_record.health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_records={}",
            trend_record.dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_admitted={}",
            handoff_summary.admitted
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_repair_first={}",
            handoff_summary.requires_repair_first
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_repair_tasks={}",
            handoff_summary.repair_tasks
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_next_queue_tasks={}",
            handoff_summary.next_queue_tasks
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_blocked_reasons={}",
            handoff_summary.blocked_reasons.len()
        ),
    ];
    telemetry.extend(trend_record.telemetry.iter().cloned());
    telemetry.extend(handoff_summary.telemetry.iter().cloned());
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn report_gate_health_gate_trend_handoff_summary_telemetry(
    trend_health_status: AgentReportGateHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    trend_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_trend_handoff_summary=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_summary_status={}",
            trend_health_status.as_str()
        ),
        format!("agent_report_gate_health_gate_trend_handoff_summary_admitted={admitted}"),
        format!(
            "agent_report_gate_health_gate_trend_handoff_summary_repair_first={requires_repair_first}"
        ),
        format!("agent_report_gate_health_gate_trend_handoff_summary_records={trend_records}"),
        format!("agent_report_gate_health_gate_trend_handoff_summary_repair_tasks={repair_tasks}"),
        format!(
            "agent_report_gate_health_gate_trend_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn report_gate_health_gate_trend_handoff_dashboard_telemetry(
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
    latest_trend_health_status: Option<AgentReportGateHealthStatus>,
    latest_blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_trend_handoff_dashboard=true".to_owned(),
        format!("agent_report_gate_health_gate_trend_handoff_dashboard_records={total_records}"),
        format!(
            "agent_report_gate_health_gate_trend_handoff_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_dashboard_repair_first={repair_first_records}"
        ),
        format!("agent_report_gate_health_gate_trend_handoff_dashboard_stable={stable_records}"),
        format!("agent_report_gate_health_gate_trend_handoff_dashboard_watch={watch_records}"),
        format!("agent_report_gate_health_gate_trend_handoff_dashboard_repair={repair_records}"),
        format!(
            "agent_report_gate_health_gate_trend_handoff_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_dashboard_latest_status={}",
            latest_trend_health_status
                .map(AgentReportGateHealthStatus::as_str)
                .unwrap_or("none")
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_dashboard_latest_blocked_reasons={latest_blocked_reasons}"
        ),
    ]
}

fn report_gate_health_gate_trend_handoff_history_record_telemetry(
    dashboard: &AgentReportGateHealthGateTrendHandoffDashboard,
    health: &AgentReportGateHealthGateTrendHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_history_record=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!("agent_report_gate_health_gate_trend_handoff_history_record_reason={reason}")
    }));
    telemetry
}

fn report_gate_health_gate_trend_handoff_gate_telemetry(
    handoff_health_status: AgentReportGateHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_gate=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_gate_status={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_gate_requested_admitted={requested_admitted}"
        ),
        format!("agent_report_gate_health_gate_trend_handoff_gate_admitted={admitted}"),
        format!(
            "agent_report_gate_health_gate_trend_handoff_gate_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_report_gate_health_gate_trend_handoff_gate_repair_tasks={repair_tasks}"),
        format!(
            "agent_report_gate_health_gate_trend_handoff_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        blocked_reasons.iter().map(|reason| {
            format!("agent_report_gate_health_gate_trend_handoff_gate_reason={reason}")
        }),
    );
    telemetry
}

fn report_gate_health_gate_trend_handoff_monitor_telemetry(
    handoff: &AgentReportGateHealthGateTrendHandoffRecord,
    history_record: &AgentReportGateHealthGateTrendHandoffHistoryRecord,
    gate_decision: &AgentReportGateHealthGateTrendHandoffGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_status={}",
            handoff.trend_record.health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn report_gate_health_gate_trend_handoff_monitor_summary_telemetry(
    handoff_health_status: AgentReportGateHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    handoff_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_summary=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_summary_status={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_summary_requested_admitted={requested_admitted}"
        ),
        format!("agent_report_gate_health_gate_trend_handoff_monitor_summary_admitted={admitted}"),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_summary_records={handoff_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn report_gate_health_gate_trend_handoff_monitor_dashboard_telemetry(
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
    latest_handoff_health_status: Option<AgentReportGateHealthStatus>,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_dashboard=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_records={total_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_requested_admitted={requested_admitted_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_latest_status={}",
            latest_handoff_health_status
                .map(AgentReportGateHealthStatus::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn report_gate_health_gate_trend_handoff_monitor_history_record_telemetry(
    dashboard: &AgentReportGateHealthGateTrendHandoffMonitorDashboard,
    health: &AgentReportGateHealthGateTrendHandoffMonitorHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_history_record=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_history_record_reason={reason}"
        )
    }));
    telemetry
}

fn report_gate_health_gate_trend_handoff_monitor_gate_telemetry(
    monitor_health_status: AgentReportGateHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_gate=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_gate_status={}",
            monitor_health_status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_gate_requested_admitted={requested_admitted}"
        ),
        format!("agent_report_gate_health_gate_trend_handoff_monitor_gate_admitted={admitted}"),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!("agent_report_gate_health_gate_trend_handoff_monitor_gate_reason={reason}")
    }));
    telemetry
}

fn report_gate_health_gate_trend_handoff_monitor_handoff_telemetry(
    monitor: &AgentReportGateHealthGateTrendHandoffMonitorRecord,
    history_record: &AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecord,
    gate_decision: &AgentReportGateHealthGateTrendHandoffMonitorGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(monitor.telemetry.iter().cloned());
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn report_gate_health_gate_trend_handoff_monitor_handoff_summary_telemetry(
    monitor_health_status: AgentReportGateHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    monitor_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_summary=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_summary_status={}",
            monitor_health_status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_summary_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_summary_admitted={admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_summary_records={monitor_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_telemetry(
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
    latest_monitor_health_status: Option<AgentReportGateHealthStatus>,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_records={total_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_requested_admitted={requested_admitted_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_latest_status={}",
            latest_monitor_health_status
                .map(AgentReportGateHealthStatus::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn report_gate_health_gate_trend_handoff_monitor_handoff_history_record_telemetry(
    dashboard: &AgentReportGateHealthGateTrendHandoffMonitorHandoffDashboard,
    health: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record=true"
            .to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record_repair_records={}",
            dashboard.repair_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record_watch_records={}",
            dashboard.watch_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record_reason={reason}"
        )
    }));
    telemetry
}

fn report_gate_health_gate_trend_handoff_monitor_handoff_gate_telemetry(
    handoff_health_status: AgentReportGateHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_gate=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_gate_status={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_gate_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_gate_admitted={admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!("agent_report_gate_health_gate_trend_handoff_monitor_handoff_gate_reason={reason}")
    }));
    telemetry
}

fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_telemetry(
    handoff: &AgentReportGateHealthGateTrendHandoffMonitorHandoffRecord,
    history_record: &AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecord,
    gate_decision: &AgentReportGateHealthGateTrendHandoffMonitorHandoffGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(handoff.telemetry.iter().cloned());
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_telemetry(
    handoff_health_status: AgentReportGateHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    handoff_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary=true"
            .to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_status={}",
            handoff_health_status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_admitted={admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_records={handoff_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_telemetry(
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
    latest_handoff_health_status: Option<AgentReportGateHealthStatus>,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard=true"
            .to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_records={total_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_requested_admitted={requested_admitted_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_latest_status={}",
            latest_handoff_health_status
                .map(AgentReportGateHealthStatus::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_telemetry(
    dashboard: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffDashboard,
    health: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record=true"
            .to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_repair_records={}",
            dashboard.repair_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_watch_records={}",
            dashboard.watch_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_reason={reason}"
        )
    }));
    telemetry
}

fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_telemetry(
    packet_health_status: AgentReportGateHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_status={}",
            packet_health_status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_admitted={admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_reason={reason}"
        )
    }));
    telemetry
}

fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_telemetry(
    packet: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord,
    history_record: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecord,
    gate_decision: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff=true"
            .to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(packet.telemetry.iter().cloned());
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_telemetry(
    packet_health_status: AgentReportGateHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    packet_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary=true"
            .to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_status={}",
            packet_health_status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_admitted={admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_records={packet_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_telemetry(
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
    latest_packet_health_status: Option<AgentReportGateHealthStatus>,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard=true"
            .to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_records={total_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_requested_admitted={requested_admitted_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_admitted={admitted_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_repair_first={repair_first_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_stable={stable_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_watch={watch_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_repair={repair_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_repair_tasks={repair_task_count}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_blocked_reasons={blocked_reasons}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_admission_rate={admission_rate:.3}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_latest_status={}",
            latest_packet_health_status
                .map(AgentReportGateHealthStatus::as_str)
                .unwrap_or("none")
        ),
    ]
}

fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_telemetry(
    dashboard: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffDashboard,
    health: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record=true"
            .to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_repair_records={}",
            dashboard.repair_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_watch_records={}",
            dashboard.watch_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_next_queue_tasks={}",
            dashboard.total_next_queue_tasks
        ),
    ];
    telemetry.extend(health.reasons.iter().map(|reason| {
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_reason={reason}"
        )
    }));
    telemetry
}

fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_telemetry(
    admission_health_status: AgentReportGateHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: &[String],
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate=true"
            .to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_status={}",
            admission_health_status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_admitted={admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_blocked_reasons={}",
            blocked_reasons.len()
        ),
    ];
    telemetry.extend(blocked_reasons.iter().map(|reason| {
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_gate_reason={reason}"
        )
    }));
    telemetry
}

fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_telemetry(
    admission: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord,
    history_record: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecord,
    gate_decision: &AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff=true"
            .to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_records={}",
            history_record.dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_health_status={}",
            history_record.health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
    ];
    telemetry.extend(admission.telemetry.iter().cloned());
    telemetry.extend(history_record.telemetry.iter().cloned());
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

#[allow(clippy::too_many_arguments)]
fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_telemetry(
    admission_health_status: AgentReportGateHealthStatus,
    requested_admitted: bool,
    admitted: bool,
    requires_repair_first: bool,
    admission_records: usize,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary=true"
            .to_owned(),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_status={}",
            admission_health_status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_requested_admitted={requested_admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_admitted={admitted}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_requires_repair_first={requires_repair_first}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_records={admission_records}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_repair_tasks={repair_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_next_queue_tasks={next_queue_tasks}"
        ),
        format!(
            "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_blocked_reasons={blocked_reasons}"
        ),
    ]
}

fn report_gate_health_gate_record_telemetry(
    health_record: &AgentReportGateHistoryRecord,
    gate_decision: &AgentReportGateHealthGateDecision,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_record=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_record_status={}",
            health_record.health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_record_admitted={}",
            gate_decision.admitted
        ),
        format!(
            "agent_report_gate_health_gate_record_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_report_gate_health_gate_record_records={}",
            health_record.dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_record_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
        format!(
            "agent_report_gate_health_gate_record_next_queue_tasks={}",
            gate_decision.next_queue.len()
        ),
        format!(
            "agent_report_gate_health_gate_record_blocked_reasons={}",
            gate_decision.blocked_reasons.len()
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn report_gate_health_gate_summary_telemetry(
    status: AgentReportGateHealthStatus,
    admitted: bool,
    requires_repair_first: bool,
    history_records: usize,
    accepted_records: usize,
    blocked_records: usize,
    acceptance_rate: f32,
    blocked_rate: f32,
    repair_tasks: usize,
    next_queue_tasks: usize,
    blocked_reasons: usize,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_summary=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_summary_status={}",
            status.as_str()
        ),
        format!("agent_report_gate_health_gate_summary_admitted={admitted}"),
        format!("agent_report_gate_health_gate_summary_repair_first={requires_repair_first}"),
        format!("agent_report_gate_health_gate_summary_records={history_records}"),
        format!("agent_report_gate_health_gate_summary_accepted={accepted_records}"),
        format!("agent_report_gate_health_gate_summary_blocked={blocked_records}"),
        format!("agent_report_gate_health_gate_summary_acceptance_rate={acceptance_rate:.3}"),
        format!("agent_report_gate_health_gate_summary_blocked_rate={blocked_rate:.3}"),
        format!("agent_report_gate_health_gate_summary_repair_tasks={repair_tasks}"),
        format!("agent_report_gate_health_gate_summary_next_queue_tasks={next_queue_tasks}"),
        format!("agent_report_gate_health_gate_summary_blocked_reasons={blocked_reasons}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn report_gate_health_gate_dashboard_telemetry(
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
    latest_health_status: Option<AgentReportGateHealthStatus>,
    latest_admitted: Option<bool>,
) -> Vec<String> {
    vec![
        "agent_report_gate_health_gate_dashboard=true".to_owned(),
        format!("agent_report_gate_health_gate_dashboard_records={total_records}"),
        format!("agent_report_gate_health_gate_dashboard_admitted={admitted_records}"),
        format!("agent_report_gate_health_gate_dashboard_repair_first={repair_first_records}"),
        format!("agent_report_gate_health_gate_dashboard_stable={stable_records}"),
        format!("agent_report_gate_health_gate_dashboard_watch={watch_records}"),
        format!("agent_report_gate_health_gate_dashboard_repair={repair_records}"),
        format!("agent_report_gate_health_gate_dashboard_repair_tasks={repair_task_count}"),
        format!(
            "agent_report_gate_health_gate_dashboard_next_queue_tasks={total_next_queue_tasks}"
        ),
        format!("agent_report_gate_health_gate_dashboard_blocked_reasons={blocked_reasons}"),
        format!("agent_report_gate_health_gate_dashboard_admission_rate={admission_rate:.3}"),
        format!("agent_report_gate_health_gate_dashboard_repair_first_rate={repair_first_rate:.3}"),
        format!(
            "agent_report_gate_health_gate_dashboard_latest_status={}",
            latest_health_status
                .map(AgentReportGateHealthStatus::as_str)
                .unwrap_or("none")
        ),
        format!(
            "agent_report_gate_health_gate_dashboard_latest_admitted={}",
            latest_admitted
                .map(|admitted| admitted.to_string())
                .unwrap_or_else(|| "none".to_owned())
        ),
    ]
}

fn report_gate_health_gate_history_record_telemetry(
    dashboard: &AgentReportGateHealthGateDashboard,
    health: &AgentReportGateHealthGateHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_report_gate_health_gate_history_record=true".to_owned(),
        format!(
            "agent_report_gate_health_gate_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_report_gate_health_gate_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_report_gate_health_gate_history_record_admission_rate={:.3}",
            dashboard.admission_rate
        ),
        format!(
            "agent_report_gate_health_gate_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
        format!(
            "agent_report_gate_health_gate_history_record_repair_tasks={}",
            dashboard.repair_task_count
        ),
        format!(
            "agent_report_gate_health_gate_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_report_gate_health_gate_history_record_reason={reason}")),
    );
    telemetry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::{
        AggregationConflictReviewHealthPolicy, AggregationConflictReviewSummaryHistory,
        AggregationConflictReviewSummaryHistoryRecorder, AggregationConflictReviewTrendGate,
        AggregationConflictReviewTrendGateDecision, AggregationConflictReviewer,
        AggregationHealthPolicy, AggregationSummaryHistory,
    };
    use crate::conflict::{ConflictReportHealthPolicy, ConflictReportSummaryHistory};
    use crate::memory::{
        MemoryPromotionGate, MemorySubmissionHealth, MemorySubmissionHealthPolicy,
        MemorySubmissionSummaryHistory, MemorySubmissionSummaryHistoryRecorder,
    };
    use crate::message::{AgentMessage, AgentMessageKind};
    use crate::ports::MemoryNote;
    use crate::reflection::{
        ReflectionLoop, ReflectionLoopHealthPolicy, ReflectionLoopHistoryGateDecision,
        ReflectionLoopSummaryHistory, ReflectionLoopSummaryHistoryRecorder, ReflectionStage,
    };

    fn clean_summary() -> AgentCycleSummary {
        AgentCycleSummary {
            assigned_tasks: 2,
            rejected_tasks: 0,
            unique_messages: 3,
            duplicate_groups: 0,
            unresolved_conflicts: 0,
            blocked_side_effects: 0,
            budget_overspends: 0,
            execution_failures: 0,
            reward_total: 0.91,
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

    fn clean_evidence() -> AgentReportEvidence {
        AgentReportEvidence::new(true, true)
            .with_validation_ref("eval:validation:pass")
            .with_runtime_ref("service:runtime:200")
    }

    fn business_queue() -> AgentTaskQueue {
        AgentTaskQueue::from_tasks(vec![AgentTask::new(
            "business-task",
            AgentRole::Planner,
            "continue business loop",
            AgentBudget::new(8, 1, 1),
        )])
    }

    fn stable_reflection_gate() -> ReflectionLoopHistoryGateDecision {
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft accepted")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "no blocker")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "keep evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember clean handoff")
            .unwrap();

        ReflectionLoopSummaryHistoryRecorder::new()
            .record_loop_with_health_gate(
                ReflectionLoopSummaryHistory::new(),
                &loop_state,
                ReflectionLoopHealthPolicy::default(),
            )
            .gate_decision
    }

    fn stable_review_gate() -> AggregationConflictReviewTrendGateDecision {
        let review = AggregationConflictReviewer::new().review_messages(
            vec![AgentMessage::new(
                "m1",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "memory",
                "clean handoff had no durable lesson",
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
        };

        MemorySubmissionSummaryHistoryRecorder::new()
            .record_report_with_health(
                MemorySubmissionSummaryHistory::new(),
                &report,
                MemorySubmissionHealthPolicy::default(),
            )
            .health
    }

    fn promotion_ledger_summary(
        status: MemoryPromotionLedgerStatus,
        candidate_notes: usize,
        can_submit_memory: bool,
        requires_repair_first: bool,
        reasons: Vec<&str>,
    ) -> MemoryPromotionLedgerSummary {
        let reasons = reasons.into_iter().map(str::to_owned).collect::<Vec<_>>();
        MemoryPromotionLedgerSummary {
            status,
            candidate_notes,
            can_submit_memory,
            requires_repair_first,
            reason_count: reasons.len(),
            repair_tasks: usize::from(requires_repair_first),
            reasons,
            telemetry: Vec::new(),
        }
    }

    fn stable_report_gate_packet_admission(
        run_id: &str,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffRecord {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            run_id,
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            run_id,
        );
        let packet = AgentReportGateHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor,
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            run_id,
        );
        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoff::new().record_and_gate(
            packet,
            AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
            run_id,
        )
    }

    fn stable_report_gate_final_admission(
        run_id: &str,
    ) -> AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffRecord {
        let admission = stable_report_gate_packet_admission(run_id);

        AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new().record_and_gate(
            admission,
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
            run_id,
        )
    }

    #[test]
    fn report_gate_accepts_clean_reinforced_record() {
        let note = MemoryNote::new("agent_cycle", "remember clean path");
        let record = AgentCycleLedgerRecord::new(
            "run-1",
            clean_summary(),
            clean_evidence(),
            Some(MemorySubmissionReport {
                submitted: vec![note],
                failures: Vec::new(),
                blocked_reasons: Vec::new(),
            }),
        );

        let decision = AgentReportGate::new().evaluate(&record);

        assert!(decision.is_accepted());
        assert!(decision.reasons.is_empty());
        assert!(decision.follow_up_tasks.is_empty());

        let summary = decision.summary();
        assert!(summary.accepted);
        assert_eq!(summary.reason_count, 0);
        assert!(summary.blocker_codes.is_empty());
        assert!(summary.repair_lanes.is_empty());
    }

    #[test]
    fn report_gate_blocks_missing_validation_and_runtime_evidence() {
        let record = AgentCycleLedgerRecord::new(
            "run 2",
            AgentCycleSummary {
                memory_promotions: 0,
                ..clean_summary()
            },
            AgentReportEvidence::new(true, true),
            None,
        );

        let decision = AgentReportGate::new().evaluate(&record);
        let codes = decision
            .reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect::<Vec<_>>();

        assert!(!decision.is_accepted());
        assert_eq!(
            codes,
            vec!["validation_evidence_missing", "runtime_evidence_missing"]
        );
        assert_eq!(decision.follow_up_tasks.len(), 1);
        assert_eq!(decision.follow_up_tasks[0].role, AgentRole::Tester);
        assert_eq!(
            decision.follow_up_tasks[0].id,
            "report-gate-run-2-validation"
        );

        let summary = decision.summary();
        assert!(!summary.accepted);
        assert_eq!(
            summary.blocker_codes,
            vec!["validation_evidence_missing", "runtime_evidence_missing"]
        );
        assert_eq!(summary.repair_lanes, vec!["eval-validation"]);
        assert_eq!(summary.repair_roles, vec![AgentRole::Tester]);
        assert!(summary.has_validation_blocker);
        assert!(summary.has_runtime_blocker);
        assert!(!summary.has_memory_blocker);
    }

    #[test]
    fn report_gate_turns_failed_memory_submission_into_curator_task() {
        let note = MemoryNote::new("agent_cycle", "remember clean path");
        let record = AgentCycleLedgerRecord::new(
            "run-3",
            clean_summary(),
            clean_evidence(),
            Some(MemorySubmissionReport {
                submitted: Vec::new(),
                failures: vec![crate::memory::MemorySubmissionFailure {
                    note,
                    reason: "store unavailable".to_owned(),
                }],
                blocked_reasons: Vec::new(),
            }),
        );

        let decision = AgentReportGate::new().evaluate(&record);
        let codes = decision
            .reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect::<Vec<_>>();

        assert!(!decision.is_accepted());
        assert_eq!(
            codes,
            vec!["memory_submission_failures", "memory_submission_partial"]
        );
        assert_eq!(decision.follow_up_tasks.len(), 1);
        assert_eq!(decision.follow_up_tasks[0].role, AgentRole::MemoryCurator);
        assert_eq!(decision.follow_up_tasks[0].lane, "eval-memory");

        let summary = decision.summary();
        assert_eq!(summary.reason_count, 2);
        assert_eq!(summary.follow_up_tasks, 1);
        assert_eq!(summary.repair_lanes, vec!["eval-memory"]);
        assert_eq!(summary.repair_roles, vec![AgentRole::MemoryCurator]);
        assert!(summary.has_memory_blocker);
        assert!(!summary.has_budget_blocker);
    }

    #[test]
    fn report_gate_summary_classifies_memory_promotion_blockers_as_memory() {
        let reasons = vec![AgentReportGateReason::new(
            "memory_promotion_aggregation_conflict",
            "unresolved conflict trend",
        )];
        let decision = AgentReportGateDecision {
            accepted: false,
            follow_up_tasks: follow_up_tasks("run-memory-promotion", &reasons),
            reasons,
        };

        let summary = decision.summary();

        assert_eq!(
            summary.blocker_codes,
            vec!["memory_promotion_aggregation_conflict"]
        );
        assert!(summary.has_memory_blocker);
        assert!(!summary.has_review_blocker);
        assert_eq!(summary.repair_lanes, vec!["eval-memory"]);
        assert_eq!(summary.repair_roles, vec![AgentRole::MemoryCurator]);
        assert_eq!(decision.follow_up_tasks.len(), 1);
        assert_eq!(decision.follow_up_tasks[0].lane, "eval-memory");
        assert_eq!(decision.follow_up_tasks[0].role, AgentRole::MemoryCurator);
    }

    #[test]
    fn report_gate_accepts_no_candidate_memory_promotion_without_submission() {
        let record = AgentCycleLedgerRecord::new(
            "run-no-candidate",
            AgentCycleSummary {
                memory_promotions: 0,
                ..clean_summary()
            },
            clean_evidence(),
            None,
        )
        .with_memory_promotion_summary(promotion_ledger_summary(
            MemoryPromotionLedgerStatus::NoCandidates,
            0,
            false,
            false,
            vec!["memory_promotion_no_candidate_notes"],
        ));

        let decision = AgentReportGate::new().evaluate(&record);

        assert!(decision.is_accepted());
        assert!(decision.reasons.is_empty());
    }

    #[test]
    fn report_gate_accepts_real_no_candidate_memory_promotion_gate_without_submission() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = stable_review_gate();
        let memory_health = stable_memory_submission_health();
        let promotion_gate =
            MemoryPromotionGate::new().gate(&[], &reflection_gate, &review_gate, &memory_health);
        let record = AgentCycleLedgerRecord::new(
            "run-real-no-candidate",
            AgentCycleSummary {
                memory_promotions: 0,
                ..clean_summary()
            },
            clean_evidence(),
            None,
        )
        .with_memory_promotion_gate(&promotion_gate);

        let summary = record.memory_promotion.as_ref().unwrap();
        let decision = AgentReportGate::new().evaluate(&record);

        assert_eq!(summary.status, MemoryPromotionLedgerStatus::NoCandidates);
        assert_eq!(summary.candidate_notes, 0);
        assert!(!summary.can_submit_memory);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.reasons, vec!["memory_promotion_no_candidate_notes"]);
        assert!(decision.is_accepted());
        assert!(decision.reasons.is_empty());
        assert!(decision.follow_up_tasks.is_empty());
    }

    #[test]
    fn report_gate_uses_pre_submit_memory_promotion_watch_before_missing_submission() {
        let record = AgentCycleLedgerRecord::new(
            "run-promotion-watch",
            clean_summary(),
            clean_evidence(),
            None,
        )
        .with_memory_promotion_summary(promotion_ledger_summary(
            MemoryPromotionLedgerStatus::Watch,
            1,
            false,
            false,
            vec!["memory_promotion_submission_history_memory_submission_history_empty"],
        ));

        let decision = AgentReportGate::new().evaluate(&record);
        let codes = decision
            .reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect::<Vec<_>>();

        assert_eq!(codes, vec!["memory_promotion_watch"]);
        assert!(!codes.contains(&"memory_submission_missing"));
        assert_eq!(decision.follow_up_tasks.len(), 1);
        assert_eq!(decision.follow_up_tasks[0].role, AgentRole::MemoryCurator);
        assert_eq!(decision.follow_up_tasks[0].lane, "eval-memory");
    }

    #[test]
    fn report_gate_blocks_memory_promotion_repair_even_without_candidates() {
        let record = AgentCycleLedgerRecord::new(
            "run-promotion-repair",
            AgentCycleSummary {
                memory_promotions: 0,
                ..clean_summary()
            },
            clean_evidence(),
            None,
        )
        .with_memory_promotion_summary(promotion_ledger_summary(
            MemoryPromotionLedgerStatus::Repair,
            0,
            false,
            true,
            vec!["memory_promotion_submission_history_memory_submission_failed_notes=1>0"],
        ));

        let decision = AgentReportGate::new().evaluate(&record);

        assert_eq!(decision.reasons.len(), 1);
        assert_eq!(decision.reasons[0].code, "memory_promotion_repair_required");
        assert_eq!(decision.follow_up_tasks.len(), 1);
        assert_eq!(decision.follow_up_tasks[0].role, AgentRole::MemoryCurator);
        assert!(!decision.is_accepted());
    }

    #[test]
    fn report_gate_emits_deterministic_blockers_and_follow_up_order() {
        let record = AgentCycleLedgerRecord::new(
            "run/4",
            AgentCycleSummary {
                unresolved_conflicts: 2,
                blocked_side_effects: 1,
                budget_overspends: 1,
                execution_failures: 1,
                reward_total: 0.30,
                reward_action: RewardAction::Penalize,
                memory_promotions: 1,
                ..clean_summary()
            },
            AgentReportEvidence::default(),
            None,
        );

        let decision = AgentReportGate::new().evaluate(&record);
        let codes = decision
            .reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect::<Vec<_>>();
        let task_ids = decision
            .follow_up_tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            codes,
            vec![
                "execution_failures",
                "unresolved_conflicts",
                "budget_overspends",
                "blocked_side_effects",
                "reward_action",
                "reward_total_below_policy",
                "validation_evidence_missing",
                "runtime_evidence_missing",
                "memory_submission_missing",
            ]
        );
        assert_eq!(
            task_ids,
            vec![
                "report-gate-run-4-review",
                "report-gate-run-4-budget",
                "report-gate-run-4-validation",
                "report-gate-run-4-memory",
            ]
        );

        let summary = decision.summary();
        assert_eq!(summary.reason_count, codes.len());
        assert_eq!(summary.follow_up_tasks, 4);
        assert_eq!(
            summary.repair_lanes,
            vec![
                "eval-review",
                "eval-budget",
                "eval-validation",
                "eval-memory",
            ]
        );
        assert!(summary.has_review_blocker);
        assert!(summary.has_budget_blocker);
        assert!(summary.has_validation_blocker);
        assert!(summary.has_runtime_blocker);
        assert!(summary.has_memory_blocker);
    }

    #[test]
    fn report_gate_blocks_tool_build_receipt_pressure() {
        let record = AgentCycleLedgerRecord::new(
            "run-tool-build",
            AgentCycleSummary {
                memory_promotions: 0,
                tool_build_reports: 1,
                tool_build_missing_requests: 1,
                tool_build_unexpected_receipts: 1,
                tool_build_duplicate_receipts: 1,
                tool_build_held_receipts: 1,
                tool_build_rejected_receipts: 1,
                ..clean_summary()
            },
            clean_evidence(),
            None,
        );

        let decision = AgentReportGate::new().evaluate(&record);
        let codes = decision
            .reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect::<Vec<_>>();

        assert!(!decision.is_accepted());
        assert_eq!(
            codes,
            vec![
                "tool_build_missing_requests",
                "tool_build_unexpected_receipts",
                "tool_build_duplicate_receipts",
                "tool_build_held_receipts",
                "tool_build_rejected_receipts",
            ]
        );
        assert_eq!(decision.follow_up_tasks.len(), 1);
        assert_eq!(
            decision.follow_up_tasks[0].id,
            "report-gate-run-tool-build-tool-build"
        );
        assert_eq!(decision.follow_up_tasks[0].lane, "eval-tool-build");

        let summary = decision.summary();
        assert_eq!(summary.reason_count, 5);
        assert_eq!(summary.repair_lanes, vec!["eval-tool-build"]);
        assert!(summary.has_tool_build_blocker);
        assert!(!summary.has_review_blocker);
        assert!(!summary.has_memory_blocker);

        let record = AgentReportGateHistoryRecorder::new().record_summary_with_health(
            AgentReportGateSummaryHistory::new(),
            summary,
            AgentReportGateHealthPolicy::default(),
        );
        assert_eq!(record.dashboard.tool_build_blockers, 1);
        assert_eq!(record.dashboard.review_blockers, 0);
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "agent_report_gate_tool_build_blockers=1>0" })
        );
    }

    #[test]
    fn report_gate_history_watches_empty() {
        let health =
            AgentReportGateSummaryHistory::new().health(AgentReportGateHealthPolicy::default());

        assert_eq!(health.status, AgentReportGateHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_report_gate_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn report_gate_history_records_stable_acceptance() {
        let note = MemoryNote::new("agent_cycle", "remember clean path");
        let record = AgentCycleLedgerRecord::new(
            "run-5",
            clean_summary(),
            clean_evidence(),
            Some(MemorySubmissionReport {
                submitted: vec![note],
                failures: Vec::new(),
                blocked_reasons: Vec::new(),
            }),
        );
        let decision = AgentReportGate::new().evaluate(&record);

        let history_record = AgentReportGateHistoryRecorder::new().record_decision_with_health(
            AgentReportGateSummaryHistory::new(),
            &decision,
            AgentReportGateHealthPolicy::default(),
        );

        assert_eq!(history_record.history.len(), 1);
        assert_eq!(history_record.records(), 1);
        assert_eq!(history_record.appended_summary, decision.summary());
        assert_eq!(history_record.dashboard.total_records, 1);
        assert_eq!(history_record.dashboard.accepted_records, 1);
        assert_eq!(history_record.dashboard.blocked_records, 0);
        assert_eq!(history_record.dashboard.reason_count, 0);
        assert_eq!(history_record.dashboard.follow_up_tasks, 0);
        assert_eq!(history_record.dashboard.acceptance_rate, 1.0);
        assert_eq!(history_record.dashboard.blocked_rate, 0.0);
        assert_eq!(history_record.dashboard.latest_accepted, Some(true));
        assert_eq!(
            history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(history_record.health.allows_service_advance());
        assert!(!history_record.health.requires_repair_first());
        assert!(history_record.allows_service_advance());
        assert!(!history_record.requires_repair_first());
        assert!(
            history_record
                .telemetry
                .iter()
                .any(|line| { line == "agent_report_gate_history_record_status=stable" })
        );
    }

    #[test]
    fn report_gate_history_repairs_blocker_pressure() {
        let clean_note = MemoryNote::new("agent_cycle", "remember clean path");
        let clean_record = AgentCycleLedgerRecord::new(
            "run-6-clean",
            clean_summary(),
            clean_evidence(),
            Some(MemorySubmissionReport {
                submitted: vec![clean_note],
                failures: Vec::new(),
                blocked_reasons: Vec::new(),
            }),
        );
        let dirty_record = AgentCycleLedgerRecord::new(
            "run-6-dirty",
            AgentCycleSummary {
                unresolved_conflicts: 1,
                budget_overspends: 1,
                reward_total: 0.40,
                reward_action: RewardAction::Hold,
                memory_promotions: 0,
                ..clean_summary()
            },
            AgentReportEvidence::new(false, false),
            None,
        );
        let gate = AgentReportGate::new();
        let clean = gate.evaluate(&clean_record).summary();
        let dirty = gate.evaluate(&dirty_record).summary();
        let recorder = AgentReportGateHistoryRecorder::new();

        let first = recorder.record_summary_with_health(
            AgentReportGateSummaryHistory::new(),
            clean,
            AgentReportGateHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            dirty.clone(),
            AgentReportGateHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.appended_summary, dirty);
        assert_eq!(second.dashboard.total_records, 2);
        assert_eq!(second.dashboard.accepted_records, 1);
        assert_eq!(second.dashboard.blocked_records, 1);
        assert_eq!(second.dashboard.reason_count, 6);
        assert_eq!(second.dashboard.follow_up_tasks, 3);
        assert_eq!(second.dashboard.budget_blockers, 1);
        assert_eq!(second.dashboard.validation_blockers, 1);
        assert_eq!(second.dashboard.runtime_blockers, 1);
        assert_eq!(second.dashboard.review_blockers, 1);
        assert_eq!(second.dashboard.acceptance_rate, 0.5);
        assert_eq!(second.dashboard.blocked_rate, 0.5);
        assert_eq!(second.health.status, AgentReportGateHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.health.allows_service_advance());
        assert!(second.health.requires_repair_first());
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_report_gate_blocked_records=1>0",
                "agent_report_gate_reasons=6>0",
                "agent_report_gate_follow_up_tasks=3>0",
                "agent_report_gate_budget_blockers=1>0",
                "agent_report_gate_validation_blockers=1>0",
                "agent_report_gate_runtime_blockers=1>0",
                "agent_report_gate_review_blockers=1>0",
                "agent_report_gate_acceptance_rate=0.500<0.67",
            ]
        );
        assert!(
            second
                .telemetry
                .iter()
                .any(|line| { line == "agent_report_gate_history_record_status=repair" })
        );
    }

    #[test]
    fn report_gate_history_recorder_gates_clean_health_record() {
        let note = MemoryNote::new("agent_cycle", "remember clean path");
        let record = AgentCycleLedgerRecord::new(
            "run-10",
            clean_summary(),
            clean_evidence(),
            Some(MemorySubmissionReport {
                submitted: vec![note],
                failures: Vec::new(),
                blocked_reasons: Vec::new(),
            }),
        );
        let decision = AgentReportGate::new().evaluate(&record);

        let gate_record = AgentReportGateHistoryRecorder::new().record_decision_with_health_gate(
            AgentReportGateSummaryHistory::new(),
            &decision,
            AgentReportGateHealthPolicy::default(),
            "run/10",
            &business_queue(),
        );

        assert!(gate_record.is_admitted());
        assert_eq!(gate_record.health_record.history.len(), 1);
        assert_eq!(
            gate_record.health_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(
            gate_record.gate_summary.health_status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(gate_record.gate_summary.history_records, 1);
        assert_eq!(gate_record.gate_summary.accepted_records, 1);
        assert_eq!(gate_record.gate_summary.blocked_records, 0);
        assert_eq!(gate_record.gate_summary.repair_tasks, 0);
        assert_eq!(
            gate_record.gate_summary.next_queue_task_ids,
            vec!["business-task"]
        );
        assert_eq!(gate_record.summary(), gate_record.gate_summary);
        assert!(
            gate_record
                .telemetry
                .iter()
                .any(|line| { line == "agent_report_gate_health_gate_record_status=stable" })
        );
    }

    #[test]
    fn report_gate_history_recorder_gates_repair_health_record() {
        let dirty_summary = AgentReportGateSummary {
            accepted: false,
            reason_count: 2,
            follow_up_tasks: 1,
            blocker_codes: vec!["budget_overspends".to_owned(), "reward_action".to_owned()],
            repair_lanes: vec!["eval-budget".to_owned()],
            repair_roles: vec![AgentRole::Planner],
            has_memory_blocker: false,
            has_budget_blocker: true,
            has_validation_blocker: false,
            has_runtime_blocker: false,
            has_tool_build_blocker: false,
            has_review_blocker: true,
            telemetry: Vec::new(),
        };

        let gate_record = AgentReportGateHistoryRecorder::new().record_summary_with_health_gate(
            AgentReportGateSummaryHistory::new(),
            dirty_summary,
            AgentReportGateHealthPolicy::default(),
            "run/11",
            &business_queue(),
        );

        assert!(!gate_record.is_admitted());
        assert_eq!(
            gate_record.health_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(gate_record.gate_decision.requires_repair_first);
        assert_eq!(
            gate_record.gate_decision.repair_tasks.len(),
            gate_record.health_record.health.reasons.len()
        );
        assert_eq!(
            gate_record.gate_summary.health_status,
            AgentReportGateHealthStatus::Repair
        );
        assert_eq!(gate_record.gate_summary.history_records, 1);
        assert_eq!(gate_record.gate_summary.blocked_records, 1);
        assert_eq!(
            gate_record.gate_summary.repair_tasks,
            gate_record.health_record.health.reasons.len()
        );
        assert!(
            gate_record
                .gate_summary
                .repair_task_ids
                .iter()
                .all(|id| id.starts_with("agent-report-gate-health-repair-run-11"))
        );
        assert!(
            gate_record
                .gate_summary
                .next_queue_task_ids
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(
            gate_record
                .gate_summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_report_gate_health_gate_summary_status=repair" })
        );
    }

    #[test]
    fn report_gate_health_gate_history_watches_empty() {
        let health = AgentReportGateHealthGateSummaryHistory::new()
            .health(AgentReportGateHealthGateHealthPolicy::default());

        assert_eq!(health.status, AgentReportGateHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_report_gate_health_gate_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health.dashboard.telemetry.iter().any(|line| {
                line == "agent_report_gate_health_gate_dashboard_latest_status=none"
            })
        );
    }

    #[test]
    fn report_gate_health_gate_history_marks_stable_gate_record() {
        let note = MemoryNote::new("agent_cycle", "remember clean path");
        let record = AgentCycleLedgerRecord::new(
            "run-12",
            clean_summary(),
            clean_evidence(),
            Some(MemorySubmissionReport {
                submitted: vec![note],
                failures: Vec::new(),
                blocked_reasons: Vec::new(),
            }),
        );
        let decision = AgentReportGate::new().evaluate(&record);
        let gate_record = AgentReportGateHistoryRecorder::new().record_decision_with_health_gate(
            AgentReportGateSummaryHistory::new(),
            &decision,
            AgentReportGateHealthPolicy::default(),
            "run/12",
            &business_queue(),
        );

        let history_record = AgentReportGateHealthGateHistoryRecorder::new()
            .record_gate_record_with_health(
                AgentReportGateHealthGateSummaryHistory::new(),
                &gate_record,
                AgentReportGateHealthGateHealthPolicy::default(),
            );

        assert_eq!(history_record.history.len(), 1);
        assert_eq!(history_record.records(), 1);
        assert_eq!(
            history_record.appended_summary.health_status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(history_record.dashboard.admitted_records, 1);
        assert_eq!(history_record.dashboard.repair_first_records, 0);
        assert_eq!(history_record.dashboard.repair_task_count, 0);
        assert_eq!(history_record.dashboard.admission_rate, 1.0);
        assert_eq!(
            history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(history_record.health.allows_service_advance());
        assert!(!history_record.health.requires_repair_first());
        assert!(history_record.allows_service_advance());
        assert!(!history_record.requires_repair_first());
        assert!(
            history_record.telemetry.iter().any(|line| {
                line == "agent_report_gate_health_gate_history_record_status=stable"
            })
        );
    }

    #[test]
    fn report_gate_health_gate_history_repairs_repair_first_pressure() {
        let dirty_summary = AgentReportGateSummary {
            accepted: false,
            reason_count: 2,
            follow_up_tasks: 1,
            blocker_codes: vec!["budget_overspends".to_owned(), "reward_action".to_owned()],
            repair_lanes: vec!["eval-budget".to_owned()],
            repair_roles: vec![AgentRole::Planner],
            has_memory_blocker: false,
            has_budget_blocker: true,
            has_validation_blocker: false,
            has_runtime_blocker: false,
            has_tool_build_blocker: false,
            has_review_blocker: true,
            telemetry: Vec::new(),
        };
        let gate_record = AgentReportGateHistoryRecorder::new().record_summary_with_health_gate(
            AgentReportGateSummaryHistory::new(),
            dirty_summary,
            AgentReportGateHealthPolicy::default(),
            "run/13",
            &business_queue(),
        );

        let history_record = AgentReportGateHealthGateHistoryRecorder::new()
            .record_gate_record_with_health(
                AgentReportGateHealthGateSummaryHistory::new(),
                &gate_record,
                AgentReportGateHealthGateHealthPolicy::default(),
            );

        assert_eq!(history_record.history.len(), 1);
        assert_eq!(
            history_record.appended_summary.health_status,
            AgentReportGateHealthStatus::Repair
        );
        assert_eq!(history_record.dashboard.admitted_records, 0);
        assert_eq!(history_record.dashboard.repair_first_records, 1);
        assert_eq!(history_record.dashboard.repair_records, 1);
        assert_eq!(
            history_record.dashboard.repair_task_count,
            gate_record.gate_summary.repair_tasks
        );
        assert_eq!(
            history_record.dashboard.blocked_reasons,
            gate_record.gate_summary.blocked_reasons
        );
        assert_eq!(
            history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(!history_record.health.allows_service_advance());
        assert!(history_record.health.requires_repair_first());
        assert!(!history_record.allows_service_advance());
        assert!(history_record.requires_repair_first());
        assert_eq!(
            history_record.health.reasons,
            vec![
                "agent_report_gate_health_gate_repair_first_rate=1.000>0",
                "agent_report_gate_health_gate_repair_records=1>0",
                "agent_report_gate_health_gate_repair_tasks=6>0",
                "agent_report_gate_health_gate_blocked_reasons=6>0",
                "agent_report_gate_health_gate_admission_rate=0.000<0.67",
            ]
        );
    }

    #[test]
    fn report_gate_health_gate_trend_gate_preserves_stable_queue() {
        let summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let health = AgentReportGateHealthGateSummaryHistory::from_summaries(vec![summary])
            .health(AgentReportGateHealthGateHealthPolicy::default());

        let decision = AgentReportGateHealthGateTrendGate::new().evaluate(
            "run/14",
            &health,
            &business_queue(),
        );

        assert!(decision.is_admitted());
        assert_eq!(
            decision.trend_health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(
            decision
                .telemetry
                .iter()
                .any(|line| { line == "agent_report_gate_health_gate_trend_gate_status=stable" })
        );
    }

    #[test]
    fn report_gate_health_gate_trend_gate_preserves_watch_queue() {
        let health = AgentReportGateHealthGateSummaryHistory::new().health(
            AgentReportGateHealthGateHealthPolicy {
                minimum_admission_rate: 0.0,
                ..AgentReportGateHealthGateHealthPolicy::default()
            },
        );

        let decision = AgentReportGateHealthGateTrendGate::new().evaluate(
            "run/15",
            &health,
            &business_queue(),
        );

        assert!(decision.is_admitted());
        assert_eq!(
            decision.trend_health.status,
            AgentReportGateHealthStatus::Watch
        );
        assert_eq!(
            decision.trend_health.reasons,
            vec!["agent_report_gate_health_gate_history_empty".to_owned()]
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
    }

    #[test]
    fn report_gate_health_gate_trend_gate_blocks_repair_trend() {
        let summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Repair,
            admitted: false,
            requires_repair_first: true,
            history_records: 1,
            accepted_records: 0,
            blocked_records: 1,
            acceptance_rate: 0.0,
            blocked_rate: 1.0,
            repair_tasks: 2,
            next_queue_tasks: 3,
            blocked_reasons: 2,
            repair_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            next_queue_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            telemetry: Vec::new(),
        };
        let health = AgentReportGateHealthGateSummaryHistory::from_summaries(vec![summary])
            .health(AgentReportGateHealthGateHealthPolicy::default());

        let decision = AgentReportGateHealthGateTrendGate::new().evaluate(
            "run/16",
            &health,
            &business_queue(),
        );

        assert!(!decision.is_admitted());
        assert_eq!(
            decision.trend_health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert_eq!(decision.repair_tasks.len(), health.reasons.len());
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "eval-report-gate-health-gate-trend")
        );
        assert!(decision.next_queue.task_ids().first().is_some_and(|id| {
            id.starts_with("agent-report-gate-health-gate-trend-repair-run-16")
        }));
        assert!(
            decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert_eq!(decision.blocked_reasons, health.reasons);
        assert!(
            decision.telemetry.iter().any(|line| {
                line == "agent_report_gate_health_gate_trend_gate_repair_first=true"
            })
        );
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_records_stable_boundary() {
        let note = MemoryNote::new("agent_cycle", "remember clean path");
        let record = AgentCycleLedgerRecord::new(
            "run-17",
            clean_summary(),
            clean_evidence(),
            Some(MemorySubmissionReport {
                submitted: vec![note],
                failures: Vec::new(),
                blocked_reasons: Vec::new(),
            }),
        );
        let decision = AgentReportGate::new().evaluate(&record);
        let gate_record = AgentReportGateHistoryRecorder::new().record_decision_with_health_gate(
            AgentReportGateSummaryHistory::new(),
            &decision,
            AgentReportGateHealthPolicy::default(),
            "run/17",
            &business_queue(),
        );

        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_gate_record_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            &gate_record,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/17",
            &business_queue(),
        );

        assert!(handoff.is_admitted());
        assert_eq!(handoff.trend_record.history.len(), 1);
        assert_eq!(
            handoff.trend_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(
            handoff.handoff_summary.trend_health_status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(handoff.handoff_summary.trend_records, 1);
        assert_eq!(handoff.handoff_summary.repair_tasks, 0);
        assert_eq!(
            handoff.handoff_summary.next_queue_task_ids,
            vec!["business-task"]
        );
        assert_eq!(handoff.summary(), handoff.handoff_summary);
        assert!(
            handoff.telemetry.iter().any(|line| {
                line == "agent_report_gate_health_gate_trend_handoff_status=stable"
            })
        );
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_repairs_dirty_history() {
        let clean_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let dirty_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Repair,
            admitted: false,
            requires_repair_first: true,
            history_records: 1,
            accepted_records: 0,
            blocked_records: 1,
            acceptance_rate: 0.0,
            blocked_rate: 1.0,
            repair_tasks: 2,
            next_queue_tasks: 3,
            blocked_reasons: 2,
            repair_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            next_queue_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            telemetry: Vec::new(),
        };
        let recorder = AgentReportGateHealthGateHistoryRecorder::new();
        let first = recorder.record_summary_with_health(
            AgentReportGateHealthGateSummaryHistory::new(),
            clean_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
        );

        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            first.history,
            dirty_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/18",
            &business_queue(),
        );

        assert!(!handoff.is_admitted());
        assert_eq!(handoff.trend_record.history.len(), 2);
        assert_eq!(
            handoff.trend_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(handoff.gate_decision.requires_repair_first);
        assert_eq!(
            handoff.gate_decision.repair_tasks.len(),
            handoff.trend_record.health.reasons.len()
        );
        assert_eq!(
            handoff.handoff_summary.trend_health_status,
            AgentReportGateHealthStatus::Repair
        );
        assert_eq!(
            handoff.handoff_summary.repair_tasks,
            handoff.trend_record.health.reasons.len()
        );
        assert!(
            handoff
                .handoff_summary
                .repair_task_ids
                .iter()
                .all(|id| id.starts_with("agent-report-gate-health-gate-trend-repair-run-18"))
        );
        assert!(
            handoff
                .handoff_summary
                .next_queue_task_ids
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(handoff.handoff_summary.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_summary_status=repair"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_history_watches_empty_dashboard() {
        let health = AgentReportGateHealthGateTrendHandoffHistory::new()
            .health(AgentReportGateHealthGateTrendHandoffHealthPolicy::default());

        assert_eq!(health.status, AgentReportGateHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_report_gate_health_gate_trend_handoff_history_empty"]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert_eq!(health.dashboard.admission_rate, 0.0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_history_marks_stable_rows() {
        let stable_summary = AgentReportGateHealthGateTrendHandoffSummary {
            trend_health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            trend_records: 1,
            repair_tasks: 0,
            next_queue_tasks: 1,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            blocked_reasons: Vec::new(),
            telemetry: Vec::new(),
        };
        let history =
            AgentReportGateHealthGateTrendHandoffHistory::from_summaries(vec![stable_summary]);
        let dashboard = history.dashboard();
        let health = dashboard.health(AgentReportGateHealthGateTrendHandoffHealthPolicy::default());

        assert_eq!(dashboard.total_records, 1);
        assert_eq!(dashboard.admitted_records, 1);
        assert_eq!(dashboard.repair_first_records, 0);
        assert_eq!(dashboard.stable_records, 1);
        assert_eq!(dashboard.repair_task_count, 0);
        assert_eq!(dashboard.blocked_reasons, 0);
        assert_eq!(dashboard.admission_rate, 1.0);
        assert!(dashboard.is_clean());
        assert_eq!(health.status, AgentReportGateHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert!(health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(dashboard.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_dashboard_latest_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_history_recorder_repairs_dirty_handoff() {
        let recorder = AgentReportGateHealthGateTrendHandoffHistoryRecorder::new();
        let stable_summary = AgentReportGateHealthGateTrendHandoffSummary {
            trend_health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            trend_records: 1,
            repair_tasks: 0,
            next_queue_tasks: 1,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            blocked_reasons: Vec::new(),
            telemetry: Vec::new(),
        };
        let repair_summary = AgentReportGateHealthGateTrendHandoffSummary {
            trend_health_status: AgentReportGateHealthStatus::Repair,
            admitted: false,
            requires_repair_first: true,
            trend_records: 2,
            repair_tasks: 2,
            next_queue_tasks: 3,
            repair_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            next_queue_task_ids: vec![
                "repair-a".to_owned(),
                "repair-b".to_owned(),
                "business-task".to_owned(),
            ],
            blocked_reasons: vec![
                "agent_report_gate_health_gate_repair_records=1>0".to_owned(),
                "agent_report_gate_health_gate_blocked_reasons=2>0".to_owned(),
            ],
            telemetry: Vec::new(),
        };
        let first = recorder.record_summary_with_health(
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            stable_summary,
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
        );

        let second = recorder.record_summary_with_health(
            first.history,
            repair_summary,
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(
            second
                .history
                .latest()
                .map(|summary| summary.trend_health_status),
            Some(AgentReportGateHealthStatus::Repair)
        );
        assert_eq!(
            second.appended_summary.trend_health_status,
            AgentReportGateHealthStatus::Repair
        );
        assert_eq!(second.dashboard.total_records, 2);
        assert_eq!(second.dashboard.admitted_records, 1);
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 2);
        assert_eq!(second.dashboard.blocked_reasons, 2);
        assert_eq!(
            second.dashboard.latest_blocked_reasons,
            vec![
                "agent_report_gate_health_gate_repair_records=1>0",
                "agent_report_gate_health_gate_blocked_reasons=2>0"
            ]
        );
        assert_eq!(second.health.status, AgentReportGateHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.health.allows_service_advance());
        assert!(second.health.requires_repair_first());
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_report_gate_health_gate_trend_handoff_repair_first_rate=0.500>0",
                "agent_report_gate_health_gate_trend_handoff_repair_records=1>0",
                "agent_report_gate_health_gate_trend_handoff_repair_tasks=2>0",
                "agent_report_gate_health_gate_trend_handoff_blocked_reasons=2>0",
                "agent_report_gate_health_gate_trend_handoff_admission_rate=0.500<0.67",
            ]
        );
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_history_record_status=repair"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_gate_preserves_stable_handoff() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/19",
            &business_queue(),
        );
        let history_record = AgentReportGateHealthGateTrendHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentReportGateHealthGateTrendHandoffHistory::new(),
                &handoff,
                AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            );

        let decision = AgentReportGateHealthGateTrendHandoffGate::new().evaluate(
            "run/19",
            &handoff,
            &history_record,
        );

        assert!(decision.is_admitted());
        assert!(decision.requested_admitted);
        assert_eq!(
            decision.handoff_health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_gate_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_gate_repairs_dirty_handoff_history() {
        let repair_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Repair,
            admitted: false,
            requires_repair_first: true,
            history_records: 1,
            accepted_records: 0,
            blocked_records: 1,
            acceptance_rate: 0.0,
            blocked_rate: 1.0,
            repair_tasks: 2,
            next_queue_tasks: 3,
            blocked_reasons: 2,
            repair_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            next_queue_task_ids: vec![
                "repair-a".to_owned(),
                "repair-b".to_owned(),
                "business-task".to_owned(),
            ],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            repair_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/20",
            &business_queue(),
        );
        let history_record = AgentReportGateHealthGateTrendHandoffHistoryRecorder::new()
            .record_handoff_with_health(
                AgentReportGateHealthGateTrendHandoffHistory::new(),
                &handoff,
                AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            );

        let decision = AgentReportGateHealthGateTrendHandoffGate::new().evaluate(
            "run/20",
            &handoff,
            &history_record,
        );

        assert!(!decision.is_admitted());
        assert!(!decision.requested_admitted);
        assert_eq!(
            decision.handoff_health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert_eq!(
            decision.repair_tasks.len(),
            decision.handoff_health.reasons.len()
        );
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "eval-report-gate-health-gate-trend-handoff")
        );
        assert!(
            decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(
            decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id
                    .starts_with("agent-report-gate-health-gate-trend-handoff-repair-run-20"))
        );
        assert!(decision.blocked_reasons.len() > handoff.gate_decision.blocked_reasons.len());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_records_stable_boundary() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/21",
            &business_queue(),
        );

        let monitor_record = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/21",
        );

        let summary = monitor_record.summary();
        assert!(monitor_record.is_admitted());
        assert_eq!(
            monitor_record.next_queue().task_ids(),
            vec!["business-task"]
        );
        assert_eq!(monitor_record.history_record.history.len(), 1);
        assert_eq!(
            monitor_record.history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.handoff_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(monitor_record.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_health_status=stable"
        }));
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_summary_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_blocks_dirty_history() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let repair_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Repair,
            admitted: false,
            requires_repair_first: true,
            history_records: 1,
            accepted_records: 0,
            blocked_records: 1,
            acceptance_rate: 0.0,
            blocked_rate: 1.0,
            repair_tasks: 2,
            next_queue_tasks: 3,
            blocked_reasons: 2,
            repair_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            next_queue_task_ids: vec![
                "repair-a".to_owned(),
                "repair-b".to_owned(),
                "business-task".to_owned(),
            ],
            telemetry: Vec::new(),
        };
        let handoff_builder = AgentReportGateHealthGateTrendHandoff::new();
        let dirty_handoff = handoff_builder.record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            repair_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/22-dirty",
            &business_queue(),
        );
        let stable_handoff = handoff_builder.record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/22",
            &business_queue(),
        );
        let dirty_history = AgentReportGateHealthGateTrendHandoffHistory::from_summaries(vec![
            dirty_handoff.summary(),
        ]);

        let monitor_record = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            stable_handoff,
            dirty_history,
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/22",
        );

        let summary = monitor_record.summary();
        assert!(!monitor_record.is_admitted());
        assert!(summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(
            monitor_record.history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert_eq!(summary.handoff_records, 2);
        assert_eq!(
            summary.repair_tasks,
            monitor_record.gate_decision.handoff_health.reasons.len()
        );
        assert!(
            summary
                .repair_task_ids
                .iter()
                .all(|id| id
                    .starts_with("agent-report-gate-health-gate-trend-handoff-repair-run-22"))
        );
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_summary_requires_repair_first=true"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_history_watches_empty() {
        let health = AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new()
            .health(AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default());

        assert_eq!(health.status, AgentReportGateHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_report_gate_health_gate_trend_handoff_monitor_history_empty"]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_history_marks_stable_summary() {
        let summary = AgentReportGateHealthGateTrendHandoffMonitorSummary {
            handoff_health_status: AgentReportGateHealthStatus::Stable,
            requested_admitted: true,
            admitted: true,
            requires_repair_first: false,
            handoff_records: 1,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let history =
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::from_summaries(vec![
                summary,
            ]);

        let dashboard = history.dashboard();
        let health =
            dashboard.health(AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default());

        assert_eq!(dashboard.total_records, 1);
        assert_eq!(dashboard.requested_admitted_records, 1);
        assert_eq!(dashboard.admitted_records, 1);
        assert_eq!(dashboard.repair_first_records, 0);
        assert_eq!(dashboard.stable_records, 1);
        assert_eq!(dashboard.admission_rate, 1.0);
        assert!(dashboard.is_clean());
        assert_eq!(health.status, AgentReportGateHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(dashboard.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_dashboard_latest_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_history_recorder_repairs_dirty_summary() {
        let recorder = AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecorder::new();
        let stable_summary = AgentReportGateHealthGateTrendHandoffMonitorSummary {
            handoff_health_status: AgentReportGateHealthStatus::Stable,
            requested_admitted: true,
            admitted: true,
            requires_repair_first: false,
            handoff_records: 1,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let repair_summary = AgentReportGateHealthGateTrendHandoffMonitorSummary {
            handoff_health_status: AgentReportGateHealthStatus::Repair,
            requested_admitted: true,
            admitted: false,
            requires_repair_first: true,
            handoff_records: 2,
            repair_tasks: 2,
            next_queue_tasks: 3,
            blocked_reasons: 2,
            repair_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            next_queue_task_ids: vec![
                "repair-a".to_owned(),
                "repair-b".to_owned(),
                "business-task".to_owned(),
            ],
            telemetry: Vec::new(),
        };

        let first = recorder.record_summary_with_health(
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            repair_summary,
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(
            second
                .history
                .latest()
                .map(|summary| summary.handoff_health_status),
            Some(AgentReportGateHealthStatus::Repair)
        );
        assert_eq!(second.dashboard.total_records, 2);
        assert_eq!(second.dashboard.requested_admitted_records, 2);
        assert_eq!(second.dashboard.admitted_records, 1);
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 2);
        assert_eq!(second.dashboard.blocked_reasons, 2);
        assert_eq!(second.health.status, AgentReportGateHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.health.allows_service_advance());
        assert!(second.health.requires_repair_first());
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_report_gate_health_gate_trend_handoff_monitor_repair_first_rate=0.500>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_repair_records=1>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_repair_tasks=2>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_blocked_reasons=2>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_admission_rate=0.500<0.67",
            ]
        );
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_history_record_status=repair"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_gate_preserves_stable_history() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/23",
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/23",
        );
        let history_record =
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecorder::new()
                .record_monitor_with_health(
                    AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
                    &monitor,
                    AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
                );

        let decision = AgentReportGateHealthGateTrendHandoffMonitorGate::new().evaluate(
            "run/23",
            &monitor,
            &history_record,
        );

        assert!(decision.is_admitted());
        assert!(decision.requested_admitted);
        assert_eq!(
            decision.monitor_health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_gate_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_gate_blocks_dirty_history() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/24",
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/24",
        );
        let dirty_summary = AgentReportGateHealthGateTrendHandoffMonitorSummary {
            handoff_health_status: AgentReportGateHealthStatus::Repair,
            requested_admitted: true,
            admitted: false,
            requires_repair_first: true,
            handoff_records: 2,
            repair_tasks: 2,
            next_queue_tasks: 3,
            blocked_reasons: 2,
            repair_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            next_queue_task_ids: vec![
                "repair-a".to_owned(),
                "repair-b".to_owned(),
                "business-task".to_owned(),
            ],
            telemetry: Vec::new(),
        };
        let dirty_history =
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::from_summaries(vec![
                dirty_summary,
            ]);
        let history_record =
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecorder::new()
                .record_monitor_with_health(
                    dirty_history,
                    &monitor,
                    AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
                );

        let decision = AgentReportGateHealthGateTrendHandoffMonitorGate::new().evaluate(
            "run/24",
            &monitor,
            &history_record,
        );

        assert!(!decision.is_admitted());
        assert!(decision.requested_admitted);
        assert_eq!(
            decision.monitor_health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert_eq!(
            decision.repair_tasks.len(),
            decision.monitor_health.reasons.len()
        );
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "eval-report-gate-health-gate-trend-handoff-monitor")
        );
        assert!(
            decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(decision.next_queue.task_ids().iter().any(|id| {
            id.starts_with("agent-report-gate-health-gate-trend-handoff-monitor-repair-run-24")
        }));
        assert!(
            decision
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains("monitor_repair_records=1>0"))
        );
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_records_stable_packet() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/25",
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/25",
        );

        let packet = AgentReportGateHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor,
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/25",
        );

        let summary = packet.summary();
        assert!(packet.is_admitted());
        assert_eq!(packet.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(packet.history_record.history.len(), 1);
        assert_eq!(
            packet.history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.monitor_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(packet.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_health_status=stable"
        }));
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_summary_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_blocks_dirty_history() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/26",
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/26",
        );
        let dirty_monitor_summary = AgentReportGateHealthGateTrendHandoffMonitorSummary {
            handoff_health_status: AgentReportGateHealthStatus::Repair,
            requested_admitted: true,
            admitted: false,
            requires_repair_first: true,
            handoff_records: 2,
            repair_tasks: 2,
            next_queue_tasks: 3,
            blocked_reasons: 2,
            repair_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            next_queue_task_ids: vec![
                "repair-a".to_owned(),
                "repair-b".to_owned(),
                "business-task".to_owned(),
            ],
            telemetry: Vec::new(),
        };
        let dirty_history =
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::from_summaries(vec![
                dirty_monitor_summary,
            ]);

        let packet = AgentReportGateHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor,
            dirty_history,
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/26",
        );

        let summary = packet.summary();
        assert!(!packet.is_admitted());
        assert!(summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(
            packet.history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert_eq!(summary.monitor_records, 2);
        assert_eq!(
            summary.repair_tasks,
            packet.gate_decision.monitor_health.reasons.len()
        );
        assert!(summary.repair_task_ids.iter().all(|id| {
            id.starts_with("agent-report-gate-health-gate-trend-handoff-monitor-repair-run-26")
        }));
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_summary_requires_repair_first=true"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_history_watches_empty() {
        let health = AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new()
            .health(AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default());

        assert_eq!(health.status, AgentReportGateHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_empty"]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_history_marks_stable_summary() {
        let summary = AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary {
            monitor_health_status: AgentReportGateHealthStatus::Stable,
            requested_admitted: true,
            admitted: true,
            requires_repair_first: false,
            monitor_records: 1,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let history =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::from_summaries(
                vec![summary],
            );

        let dashboard = history.dashboard();
        let health = dashboard
            .health(AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default());

        assert_eq!(dashboard.total_records, 1);
        assert_eq!(dashboard.requested_admitted_records, 1);
        assert_eq!(dashboard.admitted_records, 1);
        assert_eq!(dashboard.repair_first_records, 0);
        assert_eq!(dashboard.stable_records, 1);
        assert_eq!(dashboard.admission_rate, 1.0);
        assert!(dashboard.is_clean());
        assert_eq!(health.status, AgentReportGateHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(dashboard.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_dashboard_latest_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_history_recorder_repairs_dirty_summary()
     {
        let recorder =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder::new();
        let stable_summary = AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary {
            monitor_health_status: AgentReportGateHealthStatus::Stable,
            requested_admitted: true,
            admitted: true,
            requires_repair_first: false,
            monitor_records: 1,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let repair_summary = AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary {
            monitor_health_status: AgentReportGateHealthStatus::Repair,
            requested_admitted: true,
            admitted: false,
            requires_repair_first: true,
            monitor_records: 2,
            repair_tasks: 3,
            next_queue_tasks: 4,
            blocked_reasons: 2,
            repair_task_ids: vec![
                "monitor-repair-a".to_owned(),
                "monitor-repair-b".to_owned(),
                "monitor-repair-c".to_owned(),
            ],
            next_queue_task_ids: vec![
                "monitor-repair-a".to_owned(),
                "monitor-repair-b".to_owned(),
                "monitor-repair-c".to_owned(),
                "business-task".to_owned(),
            ],
            telemetry: Vec::new(),
        };

        let first = recorder.record_summary_with_health(
            AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            repair_summary.clone(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.appended_summary, repair_summary);
        assert_eq!(
            second
                .history
                .latest()
                .map(|summary| summary.monitor_health_status),
            Some(AgentReportGateHealthStatus::Repair)
        );
        assert_eq!(second.dashboard.total_records, 2);
        assert_eq!(second.dashboard.requested_admitted_records, 2);
        assert_eq!(second.dashboard.admitted_records, 1);
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 3);
        assert_eq!(second.dashboard.total_next_queue_tasks, 5);
        assert_eq!(second.dashboard.blocked_reasons, 2);
        assert_eq!(second.health.status, AgentReportGateHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.health.allows_service_advance());
        assert!(second.health.requires_repair_first());
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_repair_first_rate=0.500>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_repair_records=1>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_repair_tasks=3>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_blocked_reasons=2>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_admission_rate=0.500<0.67",
            ]
        );
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_history_record_status=repair"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_gate_preserves_stable_history() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/27",
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/27",
        );
        let packet = AgentReportGateHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor,
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/27",
        );
        let history_record =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder::new()
                .record_handoff_with_health(
                    AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                    &packet,
                    AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                );

        let decision = AgentReportGateHealthGateTrendHandoffMonitorHandoffGate::new().evaluate(
            "run/27",
            &packet,
            &history_record,
        );

        assert!(decision.is_admitted());
        assert!(decision.requested_admitted);
        assert_eq!(
            decision.handoff_health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_gate_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_gate_repairs_dirty_history() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/28",
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/28",
        );
        let packet = AgentReportGateHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor,
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/28",
        );
        let dirty_summary = AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary {
            monitor_health_status: AgentReportGateHealthStatus::Repair,
            requested_admitted: true,
            admitted: false,
            requires_repair_first: true,
            monitor_records: 2,
            repair_tasks: 2,
            next_queue_tasks: 3,
            blocked_reasons: 2,
            repair_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            next_queue_task_ids: vec![
                "repair-a".to_owned(),
                "repair-b".to_owned(),
                "business-task".to_owned(),
            ],
            telemetry: Vec::new(),
        };
        let dirty_history =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::from_summaries(
                vec![dirty_summary],
            );
        let history_record =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder::new()
                .record_handoff_with_health(
                    dirty_history,
                    &packet,
                    AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                );

        let decision = AgentReportGateHealthGateTrendHandoffMonitorHandoffGate::new().evaluate(
            "run/28",
            &packet,
            &history_record,
        );

        assert!(!decision.is_admitted());
        assert!(decision.requested_admitted);
        assert_eq!(
            decision.handoff_health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert_eq!(
            decision.repair_tasks.len(),
            decision.handoff_health.reasons.len()
        );
        assert!(decision.repair_tasks.iter().all(|task| {
            task.lane == "eval-report-gate-health-gate-trend-handoff-monitor-handoff"
        }));
        assert!(
            decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(decision.next_queue.task_ids().iter().any(|id| {
            id.starts_with(
                "agent-report-gate-health-gate-trend-handoff-monitor-handoff-repair-run-28",
            )
        }));
        assert!(
            decision
                .blocked_reasons
                .iter()
                .any(|reason| { reason.contains("monitor_handoff_repair_first_rate=0.500>0") })
        );
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_records_stable_packet() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/29",
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/29",
        );
        let packet = AgentReportGateHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor,
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/29",
        );

        let record = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                packet,
                AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/29",
            );

        let summary = record.summary();
        assert!(record.is_admitted());
        assert_eq!(record.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(record.history_record.history.len(), 1);
        assert_eq!(
            record.history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.handoff_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_health_status=stable"
        }));
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_repairs_dirty_history() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/30",
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/30",
        );
        let packet = AgentReportGateHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor,
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/30",
        );
        let dirty_summary = AgentReportGateHealthGateTrendHandoffMonitorHandoffSummary {
            monitor_health_status: AgentReportGateHealthStatus::Repair,
            requested_admitted: true,
            admitted: false,
            requires_repair_first: true,
            monitor_records: 2,
            repair_tasks: 2,
            next_queue_tasks: 3,
            blocked_reasons: 2,
            repair_task_ids: vec!["repair-a".to_owned(), "repair-b".to_owned()],
            next_queue_task_ids: vec![
                "repair-a".to_owned(),
                "repair-b".to_owned(),
                "business-task".to_owned(),
            ],
            telemetry: Vec::new(),
        };
        let dirty_history =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::from_summaries(
                vec![dirty_summary],
            );

        let record = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                packet,
                dirty_history,
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/30",
            );

        let summary = record.summary();
        assert!(!record.is_admitted());
        assert!(summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(
            record.history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert_eq!(summary.handoff_records, 2);
        assert_eq!(
            summary.repair_tasks,
            record.gate_decision.handoff_health.reasons.len()
        );
        assert!(summary.repair_task_ids.iter().all(|id| {
            id.starts_with(
                "agent-report-gate-health-gate-trend-handoff-monitor-handoff-repair-run-30",
            )
        }));
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_summary_requires_repair_first=true"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_watches_empty() {
        let health = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(
        )
        .health(AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default());

        assert_eq!(health.status, AgentReportGateHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec![
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_empty"
                    .to_owned()
            ]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_marks_stable_summary()
    {
        let summary = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary {
            handoff_health_status: AgentReportGateHealthStatus::Stable,
            requested_admitted: true,
            admitted: true,
            requires_repair_first: false,
            handoff_records: 1,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let history =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::from_summaries(
                vec![summary],
            );

        let dashboard = history.dashboard();
        let health = dashboard.health(
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
        );

        assert_eq!(dashboard.total_records, 1);
        assert_eq!(dashboard.requested_admitted_records, 1);
        assert_eq!(dashboard.admitted_records, 1);
        assert_eq!(dashboard.repair_first_records, 0);
        assert_eq!(dashboard.stable_records, 1);
        assert_eq!(dashboard.admission_rate, 1.0);
        assert!(dashboard.is_clean());
        assert_eq!(
            dashboard.latest_handoff_health_status,
            Some(AgentReportGateHealthStatus::Stable)
        );
        assert_eq!(health.status, AgentReportGateHealthStatus::Stable);
        assert!(health.reasons.is_empty());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(dashboard.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_dashboard_latest_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_repairs_dirty_summary()
    {
        let recorder =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder::new();
        let stable_summary = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary {
            handoff_health_status: AgentReportGateHealthStatus::Stable,
            requested_admitted: true,
            admitted: true,
            requires_repair_first: false,
            handoff_records: 1,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let repair_summary = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary {
            handoff_health_status: AgentReportGateHealthStatus::Repair,
            requested_admitted: false,
            admitted: false,
            requires_repair_first: true,
            handoff_records: 2,
            repair_tasks: 4,
            next_queue_tasks: 5,
            blocked_reasons: 3,
            repair_task_ids: vec![
                "packet-repair-a".to_owned(),
                "packet-repair-b".to_owned(),
                "packet-repair-c".to_owned(),
                "packet-repair-d".to_owned(),
            ],
            next_queue_task_ids: vec![
                "packet-repair-a".to_owned(),
                "packet-repair-b".to_owned(),
                "packet-repair-c".to_owned(),
                "packet-repair-d".to_owned(),
                "business-task".to_owned(),
            ],
            telemetry: Vec::new(),
        };

        let first = recorder.record_summary_with_health(
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
        );
        let second = recorder.record_summary_with_health(
            first.history,
            repair_summary.clone(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
        );

        assert_eq!(second.history.len(), 2);
        assert_eq!(second.appended_summary, repair_summary);
        assert_eq!(second.dashboard.total_records, 2);
        assert_eq!(second.dashboard.requested_admitted_records, 1);
        assert_eq!(second.dashboard.admitted_records, 1);
        assert_eq!(second.dashboard.repair_first_records, 1);
        assert_eq!(second.dashboard.repair_records, 1);
        assert_eq!(second.dashboard.repair_task_count, 4);
        assert_eq!(second.dashboard.total_next_queue_tasks, 6);
        assert_eq!(second.dashboard.blocked_reasons, 3);
        assert_eq!(second.dashboard.admission_rate, 0.5);
        assert_eq!(second.dashboard.repair_first_rate, 0.5);
        assert_eq!(second.health.status, AgentReportGateHealthStatus::Repair);
        assert_eq!(second.records(), 2);
        assert!(!second.health.allows_service_advance());
        assert!(second.health.requires_repair_first());
        assert!(!second.allows_service_advance());
        assert!(second.requires_repair_first());
        assert_eq!(
            second.health.reasons,
            vec![
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_repair_first_rate=0.500>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_repair_records=1>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_repair_tasks=4>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_blocked_reasons=3>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_admission_rate=0.500<0.67",
            ]
        );
        assert!(second.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_history_record_status=repair"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_preserves_stable_history()
    {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/31",
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/31",
        );
        let packet = AgentReportGateHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor,
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/31",
        );
        let admission = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                packet,
                AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/31",
            );
        let history_record =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder::new()
                .record_handoff_with_health(
                    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                    &admission,
                    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(
                    ),
                );

        let decision = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGate::new()
            .evaluate("run/31", &admission, &history_record);

        assert!(decision.is_admitted());
        assert!(decision.requested_admitted);
        assert_eq!(
            decision.packet_health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_repairs_dirty_history() {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/32",
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/32",
        );
        let packet = AgentReportGateHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor,
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/32",
        );
        let admission = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                packet,
                AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/32",
            );
        let dirty_summary = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary {
            handoff_health_status: AgentReportGateHealthStatus::Repair,
            requested_admitted: false,
            admitted: false,
            requires_repair_first: true,
            handoff_records: 2,
            repair_tasks: 4,
            next_queue_tasks: 5,
            blocked_reasons: 3,
            repair_task_ids: vec![
                "packet-repair-a".to_owned(),
                "packet-repair-b".to_owned(),
                "packet-repair-c".to_owned(),
                "packet-repair-d".to_owned(),
            ],
            next_queue_task_ids: vec![
                "packet-repair-a".to_owned(),
                "packet-repair-b".to_owned(),
                "packet-repair-c".to_owned(),
                "packet-repair-d".to_owned(),
                "business-task".to_owned(),
            ],
            telemetry: Vec::new(),
        };
        let dirty_history =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::from_summaries(
                vec![dirty_summary],
            );
        let history_record =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder::new()
                .record_handoff_with_health(
                    dirty_history,
                    &admission,
                    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(
                    ),
                );

        let decision = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGate::new()
            .evaluate("run/32", &admission, &history_record);

        assert!(!decision.is_admitted());
        assert!(decision.requested_admitted);
        assert_eq!(
            decision.packet_health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(decision.requires_repair_first);
        assert_eq!(
            decision.repair_tasks.len(),
            decision.packet_health.reasons.len()
        );
        assert!(decision.repair_tasks.iter().all(|task| {
            task.lane == "eval-report-gate-health-gate-trend-handoff-monitor-handoff-handoff"
        }));
        assert!(
            decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(decision.next_queue.task_ids().iter().any(|id| {
            id.starts_with(
                "agent-report-gate-health-gate-trend-handoff-monitor-handoff-handoff-repair-run-32",
            )
        }));
        assert!(decision.blocked_reasons.iter().any(|reason| {
            reason.contains("monitor_handoff_handoff_repair_first_rate=0.500>0")
        }));
        assert!(decision.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_gate_requires_repair_first=true"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_records_stable_packet()
    {
        let stable_summary = AgentReportGateHealthGateSummary {
            health_status: AgentReportGateHealthStatus::Stable,
            admitted: true,
            requires_repair_first: false,
            history_records: 1,
            accepted_records: 1,
            blocked_records: 0,
            acceptance_rate: 1.0,
            blocked_rate: 0.0,
            repair_tasks: 0,
            next_queue_tasks: 1,
            blocked_reasons: 0,
            repair_task_ids: Vec::new(),
            next_queue_task_ids: vec!["business-task".to_owned()],
            telemetry: Vec::new(),
        };
        let handoff = AgentReportGateHealthGateTrendHandoff::new().record_summary_and_gate(
            AgentReportGateHealthGateSummaryHistory::new(),
            stable_summary,
            AgentReportGateHealthGateHealthPolicy::default(),
            "run/33",
            &business_queue(),
        );
        let monitor = AgentReportGateHealthGateTrendHandoffMonitor::new().record_and_gate(
            handoff,
            AgentReportGateHealthGateTrendHandoffHistory::new(),
            AgentReportGateHealthGateTrendHandoffHealthPolicy::default(),
            "run/33",
        );
        let packet = AgentReportGateHealthGateTrendHandoffMonitorHandoff::new().record_and_gate(
            monitor,
            AgentReportGateHealthGateTrendHandoffMonitorSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHealthPolicy::default(),
            "run/33",
        );
        let admission = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoff::new()
            .record_and_gate(
                packet,
                AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHealthPolicy::default(),
                "run/33",
            );

        let record = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new()
            .record_and_gate(
                admission,
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::new(),
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/33",
            );

        let summary = record.summary();
        assert!(record.is_admitted());
        assert_eq!(record.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(record.history_record.history.len(), 1);
        assert_eq!(
            record.history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.packet_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_health_status=stable"
        }));
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repairs_dirty_history()
    {
        let admission = stable_report_gate_packet_admission("run/34");
        let dirty_summary = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummary {
            handoff_health_status: AgentReportGateHealthStatus::Repair,
            requested_admitted: false,
            admitted: false,
            requires_repair_first: true,
            handoff_records: 2,
            repair_tasks: 4,
            next_queue_tasks: 5,
            blocked_reasons: 3,
            repair_task_ids: vec![
                "packet-repair-a".to_owned(),
                "packet-repair-b".to_owned(),
                "packet-repair-c".to_owned(),
                "packet-repair-d".to_owned(),
            ],
            next_queue_task_ids: vec![
                "packet-repair-a".to_owned(),
                "packet-repair-b".to_owned(),
                "packet-repair-c".to_owned(),
                "packet-repair-d".to_owned(),
                "business-task".to_owned(),
            ],
            telemetry: Vec::new(),
        };
        let dirty_history =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory::from_summaries(
                vec![dirty_summary],
            );

        let record = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoff::new()
            .record_and_gate(
                admission,
                dirty_history,
                AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHealthPolicy::default(),
                "run/34",
            );

        let summary = record.summary();
        assert!(!record.is_admitted());
        assert!(summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(
            record.history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert_eq!(summary.packet_records, 2);
        assert_eq!(
            summary.repair_tasks,
            record.gate_decision.packet_health.reasons.len()
        );
        assert!(summary.repair_task_ids.iter().all(|id| {
            id.starts_with(
                "agent-report-gate-health-gate-trend-handoff-monitor-handoff-handoff-repair-run-34",
            )
        }));
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|id| id == "business-task")
        );
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_summary_requires_repair_first=true"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_watches_empty()
    {
        let health = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new()
            .health(AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default());

        assert_eq!(health.status, AgentReportGateHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec![
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_empty"
                    .to_owned()
            ]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(health.dashboard.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_dashboard_latest_status=none"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_marks_stable_admission()
     {
        let admission = stable_report_gate_final_admission("run/35");

        let history_record =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder::new()
                .record_admission_with_health(
                    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new(),
                    &admission,
                    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(),
                );

        assert_eq!(history_record.history.len(), 1);
        assert_eq!(history_record.records(), 1);
        assert_eq!(
            history_record.appended_summary.packet_health_status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(history_record.dashboard.requested_admitted_records, 1);
        assert_eq!(history_record.dashboard.admitted_records, 1);
        assert_eq!(history_record.dashboard.repair_first_records, 0);
        assert_eq!(history_record.dashboard.repair_task_count, 0);
        assert_eq!(history_record.dashboard.admission_rate, 1.0);
        assert_eq!(
            history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(history_record.health.allows_service_advance());
        assert!(!history_record.health.requires_repair_first());
        assert!(history_record.allows_service_advance());
        assert!(!history_record.requires_repair_first());
        assert!(history_record.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_record_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_history_repairs_dirty_admission()
     {
        let dirty_summary =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary {
                packet_health_status: AgentReportGateHealthStatus::Repair,
                requested_admitted: true,
                admitted: false,
                requires_repair_first: true,
                packet_records: 2,
                repair_tasks: 3,
                next_queue_tasks: 4,
                blocked_reasons: 2,
                repair_task_ids: vec![
                    "final-repair-a".to_owned(),
                    "final-repair-b".to_owned(),
                    "final-repair-c".to_owned(),
                ],
                next_queue_task_ids: vec![
                    "final-repair-a".to_owned(),
                    "final-repair-b".to_owned(),
                    "final-repair-c".to_owned(),
                    "business-task".to_owned(),
                ],
                telemetry: Vec::new(),
            };

        let history_record =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder::new()
                .record_summary_with_health(
                    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new(),
                    dirty_summary,
                    AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(),
                );

        assert_eq!(history_record.dashboard.total_records, 1);
        assert_eq!(history_record.dashboard.repair_first_records, 1);
        assert_eq!(history_record.dashboard.repair_records, 1);
        assert_eq!(history_record.dashboard.repair_task_count, 3);
        assert_eq!(history_record.dashboard.blocked_reasons, 2);
        assert_eq!(
            history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert!(!history_record.health.allows_service_advance());
        assert!(history_record.health.requires_repair_first());
        assert!(!history_record.allows_service_advance());
        assert!(history_record.requires_repair_first());
        assert_eq!(
            history_record.health.reasons,
            vec![
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_first_rate=1.000>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_records=1>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_repair_tasks=3>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_blocked_reasons=2>0",
                "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_admission_rate=0.000<0.67",
            ]
        );
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_repairs_dirty_history()
     {
        let admission = stable_report_gate_final_admission("run/36");
        let dirty_summary =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummary {
                packet_health_status: AgentReportGateHealthStatus::Repair,
                requested_admitted: false,
                admitted: false,
                requires_repair_first: true,
                packet_records: 2,
                repair_tasks: 3,
                next_queue_tasks: 4,
                blocked_reasons: 2,
                repair_task_ids: vec![
                    "final-repair-a".to_owned(),
                    "final-repair-b".to_owned(),
                    "final-repair-c".to_owned(),
                ],
                next_queue_task_ids: vec![
                    "final-repair-a".to_owned(),
                    "final-repair-b".to_owned(),
                    "final-repair-c".to_owned(),
                    "business-task".to_owned(),
                ],
                telemetry: Vec::new(),
            };
        let dirty_history =
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::from_summaries(
                vec![dirty_summary],
            );

        let record = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff::new(
        )
        .record_and_gate(
            admission,
            dirty_history,
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(
            ),
            "run/36",
        );

        let summary = record.summary();
        assert!(!record.is_admitted());
        assert!(summary.requested_admitted);
        assert!(!summary.admitted);
        assert!(summary.requires_repair_first);
        assert_eq!(
            record.history_record.health.status,
            AgentReportGateHealthStatus::Repair
        );
        assert_eq!(summary.admission_records, 2);
        assert_eq!(
            summary.repair_tasks,
            record.gate_decision.admission_health.reasons.len()
        );
        assert!(summary.repair_task_ids.iter().all(|id| {
            id.starts_with(
                "agent-report-gate-health-gate-trend-handoff-monitor-handoff-handoff-handoff-repair-run-36",
            )
        }));
        assert!(
            summary
                .next_queue_task_ids
                .iter()
                .any(|id| { id == "business-task" })
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_health_status=repair"
        }));
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_requires_repair_first=true"
        }));
    }

    #[test]
    fn report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_records_stable_admission()
     {
        let admission = stable_report_gate_final_admission("run/37");

        let record = AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff::new(
        )
        .record_and_gate(
            admission,
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory::new(),
            AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHealthPolicy::default(
            ),
            "run/37",
        );

        let summary = record.summary();
        assert!(record.is_admitted());
        assert_eq!(record.next_queue().task_ids(), vec!["business-task"]);
        assert_eq!(record.history_record.history.len(), 1);
        assert_eq!(
            record.history_record.health.status,
            AgentReportGateHealthStatus::Stable
        );
        assert_eq!(
            summary.admission_health_status,
            AgentReportGateHealthStatus::Stable
        );
        assert!(summary.requested_admitted);
        assert!(summary.admitted);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.admission_records, 1);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.next_queue_task_ids, vec!["business-task"]);
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_health_status=stable"
        }));
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_report_gate_health_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff_summary_status=stable"
        }));
    }

    #[test]
    fn report_gate_health_gate_preserves_stable_queue() {
        let health = AgentReportGateSummaryHistory::from_summaries(vec![AgentReportGateSummary {
            accepted: true,
            reason_count: 0,
            follow_up_tasks: 0,
            blocker_codes: Vec::new(),
            repair_lanes: Vec::new(),
            repair_roles: Vec::new(),
            has_memory_blocker: false,
            has_budget_blocker: false,
            has_validation_blocker: false,
            has_runtime_blocker: false,
            has_tool_build_blocker: false,
            has_review_blocker: false,
            telemetry: Vec::new(),
        }])
        .health(AgentReportGateHealthPolicy::default());

        let decision =
            AgentReportGateHealthGate::new().evaluate("run/7", &health, &business_queue());

        assert!(decision.is_admitted());
        assert_eq!(decision.health.status, AgentReportGateHealthStatus::Stable);
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
        assert!(decision.blocked_reasons.is_empty());
        assert!(
            decision
                .telemetry
                .iter()
                .any(|line| { line == "agent_report_gate_health_gate_status=stable" })
        );
    }

    #[test]
    fn report_gate_health_gate_preserves_watch_queue() {
        let health = AgentReportGateSummaryHistory::new().health(AgentReportGateHealthPolicy {
            minimum_acceptance_rate: 0.0,
            ..AgentReportGateHealthPolicy::default()
        });

        let decision =
            AgentReportGateHealthGate::new().evaluate("run/8", &health, &business_queue());

        assert!(decision.is_admitted());
        assert_eq!(decision.health.status, AgentReportGateHealthStatus::Watch);
        assert_eq!(
            decision.health.reasons,
            vec!["agent_report_gate_history_empty".to_owned()]
        );
        assert!(!decision.requires_repair_first);
        assert!(decision.repair_tasks.is_empty());
        assert_eq!(decision.next_queue.task_ids(), vec!["business-task"]);
    }

    #[test]
    fn report_gate_health_gate_blocks_repair_trend() {
        let health = AgentReportGateSummaryHistory::from_summaries(vec![AgentReportGateSummary {
            accepted: false,
            reason_count: 2,
            follow_up_tasks: 1,
            blocker_codes: vec!["budget_overspends".to_owned(), "reward_action".to_owned()],
            repair_lanes: vec!["eval-budget".to_owned()],
            repair_roles: vec![AgentRole::Planner],
            has_memory_blocker: false,
            has_budget_blocker: true,
            has_validation_blocker: false,
            has_runtime_blocker: false,
            has_tool_build_blocker: false,
            has_review_blocker: true,
            telemetry: Vec::new(),
        }])
        .health(AgentReportGateHealthPolicy::default());

        let decision =
            AgentReportGateHealthGate::new().evaluate("run/9", &health, &business_queue());

        assert!(!decision.is_admitted());
        assert_eq!(decision.health.status, AgentReportGateHealthStatus::Repair);
        assert!(decision.requires_repair_first);
        assert_eq!(decision.repair_tasks.len(), health.reasons.len());
        assert!(
            decision
                .repair_tasks
                .iter()
                .all(|task| task.lane == "eval-report-gate-health")
        );
        assert!(
            decision
                .next_queue
                .task_ids()
                .first()
                .is_some_and(|id| id.starts_with("agent-report-gate-health-repair-run-9"))
        );
        assert!(
            decision
                .next_queue
                .task_ids()
                .iter()
                .any(|id| id == "business-task")
        );
        assert_eq!(decision.blocked_reasons, health.reasons);
        assert!(
            decision
                .telemetry
                .iter()
                .any(|line| { line == "agent_report_gate_health_gate_repair_first=true" })
        );
    }
}
