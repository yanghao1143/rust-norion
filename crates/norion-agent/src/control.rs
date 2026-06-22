use crate::ledger::{
    AgentCycleLedger, AgentCycleLedgerAdmissionDecision, AgentCycleLedgerAdmissionStatus,
    AgentCycleLedgerPolicy,
};
use crate::task::AgentTaskQueue;

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveStateCandidate {
    pub run_id: String,
    pub reward_total: f32,
    pub acceptance_rate: f32,
    pub average_reward_total: f32,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentBusinessLoopPlan {
    pub admission: AgentCycleLedgerAdmissionDecision,
    pub next_queue: AgentTaskQueue,
    pub adaptive_state_candidate: Option<AdaptiveStateCandidate>,
    pub telemetry: Vec<String>,
}

impl AgentBusinessLoopPlan {
    pub fn status(&self) -> AgentCycleLedgerAdmissionStatus {
        self.admission.status
    }

    pub fn can_promote_adaptive_state(&self) -> bool {
        self.adaptive_state_candidate.is_some()
            && self.admission.status == AgentCycleLedgerAdmissionStatus::Promote
    }

    pub fn summary(&self) -> AgentBusinessLoopPlanSummary {
        AgentBusinessLoopPlanSummary::from_plan(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentBusinessLoopPlanSummary {
    pub status: AgentCycleLedgerAdmissionStatus,
    pub total_cycles: usize,
    pub accepted_cycles: usize,
    pub blocked_cycles: usize,
    pub consecutive_blocked_cycles: usize,
    pub acceptance_rate: f32,
    pub average_reward_total: f32,
    pub memory_promotion_records: usize,
    pub memory_promotion_blocked_cycles: usize,
    pub memory_promotion_repair_cycles: usize,
    pub tool_build_blocked_cycles: usize,
    pub next_queue_tasks: usize,
    pub adaptive_state_candidate_present: bool,
    pub can_promote_adaptive_state: bool,
    pub requires_repair_first: bool,
    pub latest_run_id: Option<String>,
    pub reason_count: usize,
    pub evidence_refs: usize,
    pub telemetry: Vec<String>,
}

impl AgentBusinessLoopPlanSummary {
    pub fn from_plan(plan: &AgentBusinessLoopPlan) -> Self {
        let summary = &plan.admission.summary;
        let next_queue_tasks = plan.next_queue.len();
        let adaptive_state_candidate_present = plan.adaptive_state_candidate.is_some();
        let can_promote_adaptive_state = plan.can_promote_adaptive_state();
        let requires_repair_first =
            plan.admission.status == AgentCycleLedgerAdmissionStatus::Repair;
        let reason_count = plan.admission.reasons.len();
        let evidence_refs = plan
            .adaptive_state_candidate
            .as_ref()
            .map(|candidate| candidate.evidence_refs.len())
            .unwrap_or_default();
        let telemetry = business_loop_plan_summary_telemetry(
            plan.admission.status,
            summary.total_cycles,
            summary.accepted_cycles,
            summary.blocked_cycles,
            summary.consecutive_blocked_cycles,
            summary.acceptance_rate,
            summary.average_reward_total,
            summary.memory_promotion_records,
            summary.memory_promotion_blocked_cycles,
            summary.memory_promotion_repair_cycles,
            summary.tool_build_blocked_cycles,
            next_queue_tasks,
            adaptive_state_candidate_present,
            can_promote_adaptive_state,
            requires_repair_first,
            reason_count,
            evidence_refs,
        );

        Self {
            status: plan.admission.status,
            total_cycles: summary.total_cycles,
            accepted_cycles: summary.accepted_cycles,
            blocked_cycles: summary.blocked_cycles,
            consecutive_blocked_cycles: summary.consecutive_blocked_cycles,
            acceptance_rate: summary.acceptance_rate,
            average_reward_total: summary.average_reward_total,
            memory_promotion_records: summary.memory_promotion_records,
            memory_promotion_blocked_cycles: summary.memory_promotion_blocked_cycles,
            memory_promotion_repair_cycles: summary.memory_promotion_repair_cycles,
            tool_build_blocked_cycles: summary.tool_build_blocked_cycles,
            next_queue_tasks,
            adaptive_state_candidate_present,
            can_promote_adaptive_state,
            requires_repair_first,
            latest_run_id: summary.latest_run_id.clone(),
            reason_count,
            evidence_refs,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentBusinessLoopPlanHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AgentBusinessLoopPlanHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentBusinessLoopPlanSummaryHistory {
    summaries: Vec<AgentBusinessLoopPlanSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentBusinessLoopPlanDashboard {
    pub total_records: usize,
    pub promote_records: usize,
    pub hold_records: usize,
    pub repair_records: usize,
    pub candidate_records: usize,
    pub promotable_records: usize,
    pub repair_first_records: usize,
    pub next_queue_tasks: usize,
    pub reason_count: usize,
    pub evidence_refs: usize,
    pub memory_promotion_records: usize,
    pub memory_promotion_blocked_cycles: usize,
    pub memory_promotion_repair_cycles: usize,
    pub tool_build_blocked_cycles: usize,
    pub average_acceptance_rate: f32,
    pub average_reward_total: f32,
    pub promotion_rate: f32,
    pub repair_first_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentBusinessLoopPlanHealthPolicy {
    pub maximum_hold_records: usize,
    pub maximum_repair_records: usize,
    pub maximum_repair_first_records: usize,
    pub maximum_reason_count: usize,
    pub maximum_next_queue_tasks: usize,
    pub minimum_average_acceptance_rate: f32,
    pub minimum_average_reward_total: f32,
}

impl Default for AgentBusinessLoopPlanHealthPolicy {
    fn default() -> Self {
        Self {
            maximum_hold_records: usize::MAX,
            maximum_repair_records: 0,
            maximum_repair_first_records: 0,
            maximum_reason_count: usize::MAX,
            maximum_next_queue_tasks: usize::MAX,
            minimum_average_acceptance_rate: 0.5,
            minimum_average_reward_total: 0.62,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentBusinessLoopPlanHealth {
    pub status: AgentBusinessLoopPlanHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentBusinessLoopPlanDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentBusinessLoopPlanSummaryHistoryRecord {
    pub history: AgentBusinessLoopPlanSummaryHistory,
    pub appended_summary: AgentBusinessLoopPlanSummary,
    pub dashboard: AgentBusinessLoopPlanDashboard,
    pub health: AgentBusinessLoopPlanHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentBusinessLoopPlanSummaryHistoryRecorder;

impl AgentBusinessLoopPlanSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentBusinessLoopPlanSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentBusinessLoopPlanSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentBusinessLoopPlanSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentBusinessLoopPlanSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentBusinessLoopPlanDashboard {
        AgentBusinessLoopPlanDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AgentBusinessLoopPlanHealthPolicy) -> AgentBusinessLoopPlanHealth {
        self.dashboard().health(policy)
    }
}

impl AgentBusinessLoopPlanDashboard {
    pub fn from_summaries(summaries: &[AgentBusinessLoopPlanSummary]) -> Self {
        let total_records = summaries.len();
        let promote_records = summaries
            .iter()
            .filter(|summary| summary.status == AgentCycleLedgerAdmissionStatus::Promote)
            .count();
        let hold_records = summaries
            .iter()
            .filter(|summary| summary.status == AgentCycleLedgerAdmissionStatus::Hold)
            .count();
        let repair_records = summaries
            .iter()
            .filter(|summary| summary.status == AgentCycleLedgerAdmissionStatus::Repair)
            .count();
        let candidate_records = summaries
            .iter()
            .filter(|summary| summary.adaptive_state_candidate_present)
            .count();
        let promotable_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_adaptive_state)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let next_queue_tasks = summaries
            .iter()
            .map(|summary| summary.next_queue_tasks)
            .sum::<usize>();
        let reason_count = summaries
            .iter()
            .map(|summary| summary.reason_count)
            .sum::<usize>();
        let evidence_refs = summaries
            .iter()
            .map(|summary| summary.evidence_refs)
            .sum::<usize>();
        let memory_promotion_records = summaries
            .iter()
            .map(|summary| summary.memory_promotion_records)
            .sum::<usize>();
        let memory_promotion_blocked_cycles = summaries
            .iter()
            .map(|summary| summary.memory_promotion_blocked_cycles)
            .sum::<usize>();
        let memory_promotion_repair_cycles = summaries
            .iter()
            .map(|summary| summary.memory_promotion_repair_cycles)
            .sum::<usize>();
        let tool_build_blocked_cycles = summaries
            .iter()
            .map(|summary| summary.tool_build_blocked_cycles)
            .sum::<usize>();
        let average_acceptance_rate = average(
            summaries
                .iter()
                .map(|summary| summary.acceptance_rate)
                .collect::<Vec<_>>()
                .as_slice(),
        );
        let average_reward_total = average(
            summaries
                .iter()
                .map(|summary| summary.average_reward_total)
                .collect::<Vec<_>>()
                .as_slice(),
        );
        let promotion_rate = rate(promotable_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let telemetry = business_loop_plan_dashboard_telemetry(
            total_records,
            promote_records,
            hold_records,
            repair_records,
            candidate_records,
            promotable_records,
            repair_first_records,
            next_queue_tasks,
            reason_count,
            evidence_refs,
            memory_promotion_records,
            memory_promotion_blocked_cycles,
            memory_promotion_repair_cycles,
            tool_build_blocked_cycles,
            average_acceptance_rate,
            average_reward_total,
            promotion_rate,
            repair_first_rate,
        );

        Self {
            total_records,
            promote_records,
            hold_records,
            repair_records,
            candidate_records,
            promotable_records,
            repair_first_records,
            next_queue_tasks,
            reason_count,
            evidence_refs,
            memory_promotion_records,
            memory_promotion_blocked_cycles,
            memory_promotion_repair_cycles,
            tool_build_blocked_cycles,
            average_acceptance_rate,
            average_reward_total,
            promotion_rate,
            repair_first_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: AgentBusinessLoopPlanHealthPolicy) -> AgentBusinessLoopPlanHealth {
        AgentBusinessLoopPlanHealth::from_dashboard(self.clone(), policy)
    }
}

impl AgentBusinessLoopPlanHealth {
    pub fn from_dashboard(
        dashboard: AgentBusinessLoopPlanDashboard,
        policy: AgentBusinessLoopPlanHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("agent_business_loop_plan_history_empty".to_owned());
        } else if dashboard.average_acceptance_rate < policy.minimum_average_acceptance_rate {
            watch_reasons.push(format!(
                "agent_business_loop_plan_average_acceptance_rate={:.3}<{}",
                dashboard.average_acceptance_rate, policy.minimum_average_acceptance_rate
            ));
        }

        if !dashboard.is_empty()
            && dashboard.average_reward_total < policy.minimum_average_reward_total
        {
            watch_reasons.push(format!(
                "agent_business_loop_plan_average_reward_total={:.3}<{}",
                dashboard.average_reward_total, policy.minimum_average_reward_total
            ));
        }

        if dashboard.hold_records > policy.maximum_hold_records {
            watch_reasons.push(format!(
                "agent_business_loop_plan_hold_records={}>{}",
                dashboard.hold_records, policy.maximum_hold_records
            ));
        }

        if dashboard.repair_records > policy.maximum_repair_records {
            repair_reasons.push(format!(
                "agent_business_loop_plan_repair_records={}>{}",
                dashboard.repair_records, policy.maximum_repair_records
            ));
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "agent_business_loop_plan_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }

        if dashboard.reason_count > policy.maximum_reason_count {
            watch_reasons.push(format!(
                "agent_business_loop_plan_reason_count={}>{}",
                dashboard.reason_count, policy.maximum_reason_count
            ));
        }

        if dashboard.next_queue_tasks > policy.maximum_next_queue_tasks {
            watch_reasons.push(format!(
                "agent_business_loop_plan_next_queue_tasks={}>{}",
                dashboard.next_queue_tasks, policy.maximum_next_queue_tasks
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentBusinessLoopPlanHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentBusinessLoopPlanHealthStatus::Watch, watch_reasons)
        } else {
            (AgentBusinessLoopPlanHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentBusinessLoopPlanHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentBusinessLoopPlanHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentBusinessLoopPlanHealthStatus::Repair
    }
}

impl AgentBusinessLoopPlanSummaryHistoryRecord {
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

impl AgentBusinessLoopPlanSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentBusinessLoopPlanSummaryHistory,
        summary: AgentBusinessLoopPlanSummary,
        policy: AgentBusinessLoopPlanHealthPolicy,
    ) -> AgentBusinessLoopPlanSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = business_loop_plan_history_record_telemetry(&dashboard, &health);

        AgentBusinessLoopPlanSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_plan_with_health(
        &self,
        history: AgentBusinessLoopPlanSummaryHistory,
        plan: &AgentBusinessLoopPlan,
        policy: AgentBusinessLoopPlanHealthPolicy,
    ) -> AgentBusinessLoopPlanSummaryHistoryRecord {
        self.record_summary_with_health(history, plan.summary(), policy)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AgentBusinessLoopController;

impl AgentBusinessLoopController {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        ledger: &AgentCycleLedger,
        policy: AgentCycleLedgerPolicy,
    ) -> AgentBusinessLoopPlan {
        let admission = ledger.admission(policy);
        let latest = ledger.latest();
        let next_queue = latest
            .map(|entry| entry.loopback_plan.next_queue())
            .unwrap_or_default();
        let adaptive_state_candidate =
            if admission.status == AgentCycleLedgerAdmissionStatus::Promote {
                latest.and_then(|entry| {
                    if entry.is_accepted() {
                        Some(AdaptiveStateCandidate {
                            run_id: entry.record.run_id.clone(),
                            reward_total: entry.record.summary.reward_total,
                            acceptance_rate: admission.summary.acceptance_rate,
                            average_reward_total: admission.summary.average_reward_total,
                            evidence_refs: evidence_refs(entry),
                        })
                    } else {
                        None
                    }
                })
            } else {
                None
            };
        let telemetry = telemetry(&admission);

        AgentBusinessLoopPlan {
            admission,
            next_queue,
            adaptive_state_candidate,
            telemetry,
        }
    }
}

fn evidence_refs(entry: &crate::ledger::AgentCycleLedgerEntry) -> Vec<String> {
    let mut refs = Vec::new();
    refs.extend(entry.record.evidence.validation_refs.clone());
    refs.extend(entry.record.evidence.runtime_refs.clone());
    if let Some(submission) = &entry.record.memory_submission {
        refs.extend(
            submission
                .submitted
                .iter()
                .map(|note| format!("memory:{}:{}", note.topic, note.content.len())),
        );
    }
    refs
}

fn telemetry(admission: &AgentCycleLedgerAdmissionDecision) -> Vec<String> {
    let mut lines = vec![
        format!("status={}", admission.status.as_str()),
        format!("total_cycles={}", admission.summary.total_cycles),
        format!("accepted_cycles={}", admission.summary.accepted_cycles),
        format!(
            "consecutive_blocked_cycles={}",
            admission.summary.consecutive_blocked_cycles
        ),
        format!("acceptance_rate={:.3}", admission.summary.acceptance_rate),
        format!(
            "average_reward_total={:.3}",
            admission.summary.average_reward_total
        ),
    ];
    lines.extend(
        admission
            .reasons
            .iter()
            .map(|reason| format!("reason={reason}")),
    );
    lines
}

fn business_loop_plan_summary_telemetry(
    status: AgentCycleLedgerAdmissionStatus,
    total_cycles: usize,
    accepted_cycles: usize,
    blocked_cycles: usize,
    consecutive_blocked_cycles: usize,
    acceptance_rate: f32,
    average_reward_total: f32,
    memory_promotion_records: usize,
    memory_promotion_blocked_cycles: usize,
    memory_promotion_repair_cycles: usize,
    tool_build_blocked_cycles: usize,
    next_queue_tasks: usize,
    adaptive_state_candidate_present: bool,
    can_promote_adaptive_state: bool,
    requires_repair_first: bool,
    reason_count: usize,
    evidence_refs: usize,
) -> Vec<String> {
    vec![
        "agent_business_loop_plan_summary=true".to_owned(),
        format!(
            "agent_business_loop_plan_summary_status={}",
            status.as_str()
        ),
        format!("agent_business_loop_plan_summary_total_cycles={total_cycles}"),
        format!("agent_business_loop_plan_summary_accepted_cycles={accepted_cycles}"),
        format!("agent_business_loop_plan_summary_blocked_cycles={blocked_cycles}"),
        format!(
            "agent_business_loop_plan_summary_consecutive_blocked_cycles={consecutive_blocked_cycles}"
        ),
        format!("agent_business_loop_plan_summary_acceptance_rate={acceptance_rate:.3}"),
        format!("agent_business_loop_plan_summary_average_reward_total={average_reward_total:.3}"),
        format!(
            "agent_business_loop_plan_summary_memory_promotion_records={memory_promotion_records}"
        ),
        format!(
            "agent_business_loop_plan_summary_memory_promotion_blocked_cycles={memory_promotion_blocked_cycles}"
        ),
        format!(
            "agent_business_loop_plan_summary_memory_promotion_repair_cycles={memory_promotion_repair_cycles}"
        ),
        format!(
            "agent_business_loop_plan_summary_tool_build_blocked_cycles={tool_build_blocked_cycles}"
        ),
        format!("agent_business_loop_plan_summary_next_queue_tasks={next_queue_tasks}"),
        format!(
            "agent_business_loop_plan_summary_candidate_present={adaptive_state_candidate_present}"
        ),
        format!("agent_business_loop_plan_summary_can_promote={can_promote_adaptive_state}"),
        format!("agent_business_loop_plan_summary_repair_first={requires_repair_first}"),
        format!("agent_business_loop_plan_summary_reasons={reason_count}"),
        format!("agent_business_loop_plan_summary_evidence_refs={evidence_refs}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn business_loop_plan_dashboard_telemetry(
    total_records: usize,
    promote_records: usize,
    hold_records: usize,
    repair_records: usize,
    candidate_records: usize,
    promotable_records: usize,
    repair_first_records: usize,
    next_queue_tasks: usize,
    reason_count: usize,
    evidence_refs: usize,
    memory_promotion_records: usize,
    memory_promotion_blocked_cycles: usize,
    memory_promotion_repair_cycles: usize,
    tool_build_blocked_cycles: usize,
    average_acceptance_rate: f32,
    average_reward_total: f32,
    promotion_rate: f32,
    repair_first_rate: f32,
) -> Vec<String> {
    vec![
        "agent_business_loop_plan_dashboard=true".to_owned(),
        format!("agent_business_loop_plan_dashboard_records={total_records}"),
        format!("agent_business_loop_plan_dashboard_promote_records={promote_records}"),
        format!("agent_business_loop_plan_dashboard_hold_records={hold_records}"),
        format!("agent_business_loop_plan_dashboard_repair_records={repair_records}"),
        format!("agent_business_loop_plan_dashboard_candidate_records={candidate_records}"),
        format!("agent_business_loop_plan_dashboard_promotable_records={promotable_records}"),
        format!("agent_business_loop_plan_dashboard_repair_first_records={repair_first_records}"),
        format!("agent_business_loop_plan_dashboard_next_queue_tasks={next_queue_tasks}"),
        format!("agent_business_loop_plan_dashboard_reason_count={reason_count}"),
        format!("agent_business_loop_plan_dashboard_evidence_refs={evidence_refs}"),
        format!(
            "agent_business_loop_plan_dashboard_memory_promotion_records={memory_promotion_records}"
        ),
        format!(
            "agent_business_loop_plan_dashboard_memory_promotion_blocked_cycles={memory_promotion_blocked_cycles}"
        ),
        format!(
            "agent_business_loop_plan_dashboard_memory_promotion_repair_cycles={memory_promotion_repair_cycles}"
        ),
        format!(
            "agent_business_loop_plan_dashboard_tool_build_blocked_cycles={tool_build_blocked_cycles}"
        ),
        format!(
            "agent_business_loop_plan_dashboard_average_acceptance_rate={average_acceptance_rate:.3}"
        ),
        format!(
            "agent_business_loop_plan_dashboard_average_reward_total={average_reward_total:.3}"
        ),
        format!("agent_business_loop_plan_dashboard_promotion_rate={promotion_rate:.3}"),
        format!("agent_business_loop_plan_dashboard_repair_first_rate={repair_first_rate:.3}"),
    ]
}

fn business_loop_plan_history_record_telemetry(
    dashboard: &AgentBusinessLoopPlanDashboard,
    health: &AgentBusinessLoopPlanHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_business_loop_plan_history_record=true".to_owned(),
        format!(
            "agent_business_loop_plan_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_business_loop_plan_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_business_loop_plan_history_record_promotion_rate={:.3}",
            dashboard.promotion_rate
        ),
        format!(
            "agent_business_loop_plan_history_record_repair_first_rate={:.3}",
            dashboard.repair_first_rate
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_business_loop_plan_history_record_reason={reason}")),
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

fn average(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cycle::AgentCycleSummary;
    use crate::eval::{
        AgentCycleLedgerRecord, AgentReportEvidence, AgentReportGateDecision,
        AgentReportGateReason, MemoryPromotionLedgerStatus, MemoryPromotionLedgerSummary,
    };
    use crate::evolution::RewardAction;
    use crate::ledger::AgentCycleLedgerEntry;
    use crate::loopback::AgentLoopbackPlan;
    use crate::memory::MemorySubmissionReport;
    use crate::ports::MemoryNote;
    use crate::task::{AgentRole, AgentTask};

    fn summary(reward_total: f32) -> AgentCycleSummary {
        AgentCycleSummary {
            assigned_tasks: 1,
            rejected_tasks: 0,
            unique_messages: 1,
            duplicate_groups: 0,
            unresolved_conflicts: 0,
            blocked_side_effects: 0,
            budget_overspends: 0,
            execution_failures: 0,
            reward_total,
            reward_action: RewardAction::Reinforce,
            evolution_signals: 1,
            follow_up_tasks: 0,
            memory_promotions: 1,
            tool_build_reports: 0,
            tool_build_missing_requests: 0,
            tool_build_unexpected_receipts: 0,
            tool_build_duplicate_receipts: 0,
            tool_build_held_receipts: 0,
            tool_build_rejected_receipts: 0,
        }
    }

    fn accepted_entry(run_id: &str, reward_total: f32) -> AgentCycleLedgerEntry {
        AgentCycleLedgerEntry::new(
            AgentCycleLedgerRecord::new(
                run_id,
                summary(reward_total),
                AgentReportEvidence::new(true, true)
                    .with_validation_ref("eval:validation:pass")
                    .with_runtime_ref("service:runtime:ok"),
                Some(MemorySubmissionReport {
                    submitted: vec![MemoryNote::new("agent_cycle", "remember clean loop")],
                    failures: Vec::new(),
                    blocked_reasons: Vec::new(),
                    note_quality: None,
                }),
            ),
            AgentReportGateDecision {
                accepted: true,
                reasons: Vec::new(),
                follow_up_tasks: Vec::new(),
            },
            AgentLoopbackPlan {
                promote_adaptive_state: true,
                enqueue_tasks: Vec::new(),
                blocked_reasons: Vec::new(),
            },
        )
    }

    fn blocked_entry(run_id: &str) -> AgentCycleLedgerEntry {
        let repair = AgentTask::new(
            format!("repair-{run_id}"),
            AgentRole::Reviewer,
            "repair blocked loop",
            crate::budget::AgentBudget::new(8, 1, 1),
        );
        AgentCycleLedgerEntry::new(
            AgentCycleLedgerRecord::new(
                run_id,
                AgentCycleSummary {
                    reward_total: 0.30,
                    reward_action: RewardAction::Penalize,
                    execution_failures: 1,
                    memory_promotions: 0,
                    ..summary(0.30)
                },
                AgentReportEvidence::default(),
                None,
            ),
            AgentReportGateDecision {
                accepted: false,
                reasons: vec![AgentReportGateReason::new("execution_failures", "1")],
                follow_up_tasks: vec![repair.clone()],
            },
            AgentLoopbackPlan {
                promote_adaptive_state: false,
                enqueue_tasks: vec![repair],
                blocked_reasons: vec!["execution_failures=1".to_owned()],
            },
        )
    }

    fn tool_build_blocked_entry(run_id: &str) -> AgentCycleLedgerEntry {
        let mut entry = blocked_entry(run_id);
        entry.report_decision.reasons = vec![AgentReportGateReason::new(
            "tool_build_rejected_receipts",
            "1",
        )];
        entry.loopback_plan.blocked_reasons = vec!["tool_build_rejected_receipts=1".to_owned()];
        entry
    }

    fn promotion_summary(status: MemoryPromotionLedgerStatus) -> MemoryPromotionLedgerSummary {
        MemoryPromotionLedgerSummary {
            status,
            candidate_notes: usize::from(status != MemoryPromotionLedgerStatus::NoCandidates),
            can_submit_memory: status == MemoryPromotionLedgerStatus::Promotable,
            requires_repair_first: status == MemoryPromotionLedgerStatus::Repair,
            reason_count: usize::from(status != MemoryPromotionLedgerStatus::Promotable),
            repair_tasks: usize::from(status == MemoryPromotionLedgerStatus::Repair),
            reasons: Vec::new(),
            telemetry: Vec::new(),
        }
    }

    fn accepted_entry_with_memory_promotion(
        run_id: &str,
        status: MemoryPromotionLedgerStatus,
    ) -> AgentCycleLedgerEntry {
        let mut entry = accepted_entry(run_id, 0.90);
        entry.record = entry
            .record
            .with_memory_promotion_summary(promotion_summary(status));
        entry.loopback_plan.promote_adaptive_state =
            status == MemoryPromotionLedgerStatus::Promotable;
        entry.report_decision.accepted = status == MemoryPromotionLedgerStatus::Promotable;
        entry
    }

    #[test]
    fn business_loop_plan_history_watches_empty() {
        let health = AgentBusinessLoopPlanSummaryHistory::new()
            .health(AgentBusinessLoopPlanHealthPolicy::default());

        assert_eq!(health.status, AgentBusinessLoopPlanHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_business_loop_plan_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| line == "agent_business_loop_plan_dashboard_records=0")
        );
    }

    #[test]
    fn business_loop_plan_history_marks_clean_promotion_stable() {
        let ledger = AgentCycleLedger::from_entries(vec![
            accepted_entry("run-1", 0.86),
            accepted_entry("run-2", 0.88),
        ]);
        let plan =
            AgentBusinessLoopController::new().plan(&ledger, AgentCycleLedgerPolicy::default());

        let record = AgentBusinessLoopPlanSummaryHistoryRecorder::new().record_plan_with_health(
            AgentBusinessLoopPlanSummaryHistory::new(),
            &plan,
            AgentBusinessLoopPlanHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert_eq!(
            record.appended_summary.status,
            AgentCycleLedgerAdmissionStatus::Promote
        );
        assert_eq!(record.dashboard.promote_records, 1);
        assert_eq!(record.dashboard.candidate_records, 1);
        assert_eq!(record.dashboard.promotable_records, 1);
        assert_eq!(record.dashboard.promotion_rate, 1.0);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert_eq!(
            record.health.status,
            AgentBusinessLoopPlanHealthStatus::Stable
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
                .any(|line| line == "agent_business_loop_plan_history_record_status=stable")
        );
    }

    #[test]
    fn business_loop_plan_history_repairs_repair_first_pressure() {
        let clean = AgentBusinessLoopPlanSummary {
            status: AgentCycleLedgerAdmissionStatus::Promote,
            total_cycles: 2,
            accepted_cycles: 2,
            blocked_cycles: 0,
            consecutive_blocked_cycles: 0,
            acceptance_rate: 1.0,
            average_reward_total: 0.86,
            memory_promotion_records: 0,
            memory_promotion_blocked_cycles: 0,
            memory_promotion_repair_cycles: 0,
            tool_build_blocked_cycles: 0,
            next_queue_tasks: 0,
            adaptive_state_candidate_present: true,
            can_promote_adaptive_state: true,
            requires_repair_first: false,
            latest_run_id: Some("run-2".to_owned()),
            reason_count: 0,
            evidence_refs: 3,
            telemetry: Vec::new(),
        };
        let repair = AgentBusinessLoopPlanSummary {
            status: AgentCycleLedgerAdmissionStatus::Repair,
            total_cycles: 3,
            accepted_cycles: 0,
            blocked_cycles: 3,
            consecutive_blocked_cycles: 3,
            acceptance_rate: 0.0,
            average_reward_total: 0.30,
            memory_promotion_records: 0,
            memory_promotion_blocked_cycles: 0,
            memory_promotion_repair_cycles: 0,
            tool_build_blocked_cycles: 0,
            next_queue_tasks: 1,
            adaptive_state_candidate_present: false,
            can_promote_adaptive_state: false,
            requires_repair_first: true,
            latest_run_id: Some("run-3".to_owned()),
            reason_count: 4,
            evidence_refs: 0,
            telemetry: Vec::new(),
        };
        let history = AgentBusinessLoopPlanSummaryHistory::from_summaries(vec![clean]);

        let record = AgentBusinessLoopPlanSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            repair,
            AgentBusinessLoopPlanHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.promote_records, 1);
        assert_eq!(record.dashboard.repair_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.next_queue_tasks, 1);
        assert_eq!(record.dashboard.reason_count, 4);
        assert_eq!(record.dashboard.repair_first_rate, 0.5);
        assert_eq!(
            record.health.status,
            AgentBusinessLoopPlanHealthStatus::Repair
        );
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "agent_business_loop_plan_repair_records=1>0".to_owned(),
                "agent_business_loop_plan_repair_first_records=1>0".to_owned(),
                "agent_business_loop_plan_average_reward_total=0.580<0.62".to_owned(),
            ]
        );
    }

    #[test]
    fn business_loop_plan_exposes_adaptive_state_candidate_for_clean_trend() {
        let ledger = AgentCycleLedger::from_entries(vec![
            accepted_entry("run-1", 0.86),
            accepted_entry("run-2", 0.88),
        ]);

        let plan =
            AgentBusinessLoopController::new().plan(&ledger, AgentCycleLedgerPolicy::default());

        assert_eq!(plan.status(), AgentCycleLedgerAdmissionStatus::Promote);
        assert!(plan.can_promote_adaptive_state());
        assert!(plan.next_queue.is_empty());
        let candidate = plan.adaptive_state_candidate.unwrap();
        assert_eq!(candidate.run_id, "run-2");
        assert_eq!(candidate.evidence_refs.len(), 3);
        assert!(plan.telemetry.iter().any(|line| line == "status=promote"));

        let summary = AgentBusinessLoopController::new()
            .plan(&ledger, AgentCycleLedgerPolicy::default())
            .summary();
        assert_eq!(summary.status, AgentCycleLedgerAdmissionStatus::Promote);
        assert_eq!(summary.total_cycles, 2);
        assert_eq!(summary.accepted_cycles, 2);
        assert_eq!(summary.next_queue_tasks, 0);
        assert!(summary.adaptive_state_candidate_present);
        assert!(summary.can_promote_adaptive_state);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.latest_run_id.as_deref(), Some("run-2"));
        assert_eq!(summary.evidence_refs, 3);
    }

    #[test]
    fn business_loop_plan_surfaces_memory_promotion_pressure_without_candidate() {
        let ledger = AgentCycleLedger::from_entries(vec![
            accepted_entry_with_memory_promotion("run-1", MemoryPromotionLedgerStatus::Promotable),
            accepted_entry_with_memory_promotion("run-2", MemoryPromotionLedgerStatus::Blocked),
            accepted_entry_with_memory_promotion("run-3", MemoryPromotionLedgerStatus::Repair),
        ]);

        let plan =
            AgentBusinessLoopController::new().plan(&ledger, AgentCycleLedgerPolicy::default());
        let summary = plan.summary();

        assert_eq!(plan.status(), AgentCycleLedgerAdmissionStatus::Repair);
        assert!(!plan.can_promote_adaptive_state());
        assert!(plan.adaptive_state_candidate.is_none());
        assert_eq!(summary.memory_promotion_records, 3);
        assert_eq!(summary.memory_promotion_blocked_cycles, 1);
        assert_eq!(summary.memory_promotion_repair_cycles, 1);
        assert_eq!(summary.tool_build_blocked_cycles, 0);
        assert!(summary.requires_repair_first);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_business_loop_plan_summary_memory_promotion_repair_cycles=1"
        }));
        assert!(
            plan.telemetry
                .iter()
                .any(|line| { line == "reason=memory_promotion_repair_cycles=1>0" })
        );
    }

    #[test]
    fn business_loop_plan_keeps_repair_queue_closed_for_blocked_trend() {
        let ledger = AgentCycleLedger::from_entries(vec![
            blocked_entry("run-1"),
            blocked_entry("run-2"),
            blocked_entry("run-3"),
        ]);

        let plan =
            AgentBusinessLoopController::new().plan(&ledger, AgentCycleLedgerPolicy::default());

        assert_eq!(plan.status(), AgentCycleLedgerAdmissionStatus::Repair);
        assert!(!plan.can_promote_adaptive_state());
        assert_eq!(plan.next_queue.task_ids(), vec!["repair-run-3"]);
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "reason=consecutive_blocked_cycles=3")
        );

        let summary = plan.summary();
        assert_eq!(summary.status, AgentCycleLedgerAdmissionStatus::Repair);
        assert_eq!(summary.total_cycles, 3);
        assert_eq!(summary.blocked_cycles, 3);
        assert_eq!(summary.consecutive_blocked_cycles, 3);
        assert!((summary.acceptance_rate - 0.0).abs() < f32::EPSILON);
        assert!((summary.average_reward_total - 0.30).abs() < 0.01);
        assert_eq!(summary.next_queue_tasks, 1);
        assert!(!summary.adaptive_state_candidate_present);
        assert!(!summary.can_promote_adaptive_state);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.reason_count, 4);
    }

    #[test]
    fn business_loop_plan_surfaces_tool_build_blocker_pressure_without_candidate() {
        let ledger =
            AgentCycleLedger::from_entries(vec![tool_build_blocked_entry("run-tool-build")]);

        let plan =
            AgentBusinessLoopController::new().plan(&ledger, AgentCycleLedgerPolicy::default());
        let summary = plan.summary();
        let record = AgentBusinessLoopPlanSummaryHistoryRecorder::new().record_plan_with_health(
            AgentBusinessLoopPlanSummaryHistory::new(),
            &plan,
            AgentBusinessLoopPlanHealthPolicy::default(),
        );

        assert_eq!(plan.status(), AgentCycleLedgerAdmissionStatus::Repair);
        assert!(!plan.can_promote_adaptive_state());
        assert_eq!(summary.tool_build_blocked_cycles, 1);
        assert_eq!(record.dashboard.tool_build_blocked_cycles, 1);
        assert!(summary.requires_repair_first);
        assert!(summary.telemetry.iter().any(|line| {
            line == "agent_business_loop_plan_summary_tool_build_blocked_cycles=1"
        }));
        assert!(record.dashboard.telemetry.iter().any(|line| {
            line == "agent_business_loop_plan_dashboard_tool_build_blocked_cycles=1"
        }));
        assert!(
            plan.telemetry
                .iter()
                .any(|line| { line == "reason=tool_build_blocked_cycles=1>0" })
        );
    }

    #[test]
    fn business_loop_plan_holds_empty_ledger_without_candidate() {
        let plan = AgentBusinessLoopController::new()
            .plan(&AgentCycleLedger::new(), AgentCycleLedgerPolicy::default());

        assert_eq!(plan.status(), AgentCycleLedgerAdmissionStatus::Hold);
        assert!(plan.next_queue.is_empty());
        assert!(plan.adaptive_state_candidate.is_none());
        assert_eq!(plan.telemetry[0], "status=hold");

        let summary = plan.summary();
        assert_eq!(summary.status, AgentCycleLedgerAdmissionStatus::Hold);
        assert_eq!(summary.total_cycles, 0);
        assert_eq!(summary.next_queue_tasks, 0);
        assert!(!summary.adaptive_state_candidate_present);
        assert!(!summary.can_promote_adaptive_state);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.reason_count, 1);
    }
}
