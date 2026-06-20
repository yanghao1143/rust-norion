use crate::eval::{AgentCycleLedgerRecord, AgentReportGateDecision, MemoryPromotionLedgerStatus};
use crate::loopback::AgentLoopbackPlan;

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleLedgerEntry {
    pub record: AgentCycleLedgerRecord,
    pub report_decision: AgentReportGateDecision,
    pub loopback_plan: AgentLoopbackPlan,
}

impl AgentCycleLedgerEntry {
    pub fn new(
        record: AgentCycleLedgerRecord,
        report_decision: AgentReportGateDecision,
        loopback_plan: AgentLoopbackPlan,
    ) -> Self {
        Self {
            record,
            report_decision,
            loopback_plan,
        }
    }

    pub fn is_accepted(&self) -> bool {
        self.report_decision.is_accepted() && self.loopback_plan.promote_adaptive_state
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentCycleLedger {
    entries: Vec<AgentCycleLedgerEntry>,
}

impl AgentCycleLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_entries(entries: Vec<AgentCycleLedgerEntry>) -> Self {
        Self { entries }
    }

    pub fn append(&mut self, entry: AgentCycleLedgerEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[AgentCycleLedgerEntry] {
        &self.entries
    }

    pub fn latest(&self) -> Option<&AgentCycleLedgerEntry> {
        self.entries.last()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn summary(&self) -> AgentCycleLedgerSummary {
        let total_cycles = self.entries.len();
        let accepted_cycles = self
            .entries
            .iter()
            .filter(|entry| entry.is_accepted())
            .count();
        let blocked_cycles = total_cycles.saturating_sub(accepted_cycles);
        let adaptive_promotions = self
            .entries
            .iter()
            .filter(|entry| entry.loopback_plan.promote_adaptive_state)
            .count();
        let enqueued_tasks = self
            .entries
            .iter()
            .map(|entry| entry.loopback_plan.enqueue_tasks.len())
            .sum();
        let memory_promotion_records = self
            .entries
            .iter()
            .filter(|entry| entry.record.memory_promotion.is_some())
            .count();
        let memory_promotion_no_candidate_cycles =
            memory_promotion_status_count(&self.entries, MemoryPromotionLedgerStatus::NoCandidates);
        let memory_promotion_promotable_cycles =
            memory_promotion_status_count(&self.entries, MemoryPromotionLedgerStatus::Promotable);
        let memory_promotion_watch_cycles =
            memory_promotion_status_count(&self.entries, MemoryPromotionLedgerStatus::Watch);
        let memory_promotion_blocked_cycles =
            memory_promotion_status_count(&self.entries, MemoryPromotionLedgerStatus::Blocked);
        let memory_promotion_repair_cycles =
            memory_promotion_status_count(&self.entries, MemoryPromotionLedgerStatus::Repair);
        let tool_build_blocked_cycles = report_gate_tool_build_blocked_cycles(&self.entries);
        let reward_total_sum = self
            .entries
            .iter()
            .map(|entry| entry.record.summary.reward_total)
            .sum::<f32>();
        let average_reward_total = if total_cycles == 0 {
            0.0
        } else {
            reward_total_sum / total_cycles as f32
        };
        let latest_run_id = self.latest().map(|entry| entry.record.run_id.clone());
        let latest_blocked_reasons = self
            .latest()
            .map(|entry| entry.loopback_plan.blocked_reasons.clone())
            .unwrap_or_default();

        AgentCycleLedgerSummary {
            total_cycles,
            accepted_cycles,
            blocked_cycles,
            adaptive_promotions,
            enqueued_tasks,
            memory_promotion_records,
            memory_promotion_no_candidate_cycles,
            memory_promotion_promotable_cycles,
            memory_promotion_watch_cycles,
            memory_promotion_blocked_cycles,
            memory_promotion_repair_cycles,
            tool_build_blocked_cycles,
            consecutive_blocked_cycles: consecutive_blocked_cycles(&self.entries),
            acceptance_rate: rate(accepted_cycles, total_cycles),
            average_reward_total,
            latest_run_id,
            latest_blocked_reasons,
        }
    }

    pub fn admission(&self, policy: AgentCycleLedgerPolicy) -> AgentCycleLedgerAdmissionDecision {
        let summary = self.summary();
        if summary.total_cycles == 0 {
            return AgentCycleLedgerAdmissionDecision {
                status: AgentCycleLedgerAdmissionStatus::Hold,
                reasons: vec!["ledger_empty".to_owned()],
                summary,
            };
        }

        let mut reasons = Vec::new();
        if summary.consecutive_blocked_cycles >= policy.max_consecutive_blocked_cycles {
            reasons.push(format!(
                "consecutive_blocked_cycles={}",
                summary.consecutive_blocked_cycles
            ));
        }
        if summary.acceptance_rate < policy.minimum_acceptance_rate {
            reasons.push(format!(
                "acceptance_rate={:.3}<{}",
                summary.acceptance_rate, policy.minimum_acceptance_rate
            ));
        }
        if summary.average_reward_total < policy.minimum_average_reward_total {
            reasons.push(format!(
                "average_reward_total={:.3}<{}",
                summary.average_reward_total, policy.minimum_average_reward_total
            ));
        }
        if !summary.latest_blocked_reasons.is_empty() {
            reasons.push(format!(
                "latest_blocked={}",
                summary.latest_blocked_reasons.join(";")
            ));
        }
        if summary.memory_promotion_blocked_cycles > policy.maximum_memory_promotion_blocked_cycles
        {
            reasons.push(format!(
                "memory_promotion_blocked_cycles={}>{}",
                summary.memory_promotion_blocked_cycles,
                policy.maximum_memory_promotion_blocked_cycles
            ));
        }
        if summary.memory_promotion_repair_cycles > policy.maximum_memory_promotion_repair_cycles {
            reasons.push(format!(
                "memory_promotion_repair_cycles={}>{}",
                summary.memory_promotion_repair_cycles,
                policy.maximum_memory_promotion_repair_cycles
            ));
        }
        if summary.tool_build_blocked_cycles > policy.maximum_tool_build_blocked_cycles {
            reasons.push(format!(
                "tool_build_blocked_cycles={}>{}",
                summary.tool_build_blocked_cycles, policy.maximum_tool_build_blocked_cycles
            ));
        }

        let latest_promotes = self
            .latest()
            .is_some_and(|entry| entry.loopback_plan.promote_adaptive_state);
        let status = if summary.consecutive_blocked_cycles >= policy.max_consecutive_blocked_cycles
            || summary.memory_promotion_blocked_cycles
                > policy.maximum_memory_promotion_blocked_cycles
            || summary.memory_promotion_repair_cycles
                > policy.maximum_memory_promotion_repair_cycles
            || summary.tool_build_blocked_cycles > policy.maximum_tool_build_blocked_cycles
        {
            AgentCycleLedgerAdmissionStatus::Repair
        } else if latest_promotes && reasons.is_empty() {
            AgentCycleLedgerAdmissionStatus::Promote
        } else {
            AgentCycleLedgerAdmissionStatus::Hold
        };

        AgentCycleLedgerAdmissionDecision {
            status,
            reasons,
            summary,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleLedgerSummary {
    pub total_cycles: usize,
    pub accepted_cycles: usize,
    pub blocked_cycles: usize,
    pub adaptive_promotions: usize,
    pub enqueued_tasks: usize,
    pub memory_promotion_records: usize,
    pub memory_promotion_no_candidate_cycles: usize,
    pub memory_promotion_promotable_cycles: usize,
    pub memory_promotion_watch_cycles: usize,
    pub memory_promotion_blocked_cycles: usize,
    pub memory_promotion_repair_cycles: usize,
    pub tool_build_blocked_cycles: usize,
    pub consecutive_blocked_cycles: usize,
    pub acceptance_rate: f32,
    pub average_reward_total: f32,
    pub latest_run_id: Option<String>,
    pub latest_blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentCycleLedgerHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AgentCycleLedgerHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AgentCycleLedgerSummaryHistory {
    summaries: Vec<AgentCycleLedgerSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleLedgerDashboard {
    pub total_records: usize,
    pub total_cycles: usize,
    pub accepted_cycles: usize,
    pub blocked_cycles: usize,
    pub adaptive_promotions: usize,
    pub enqueued_tasks: usize,
    pub memory_promotion_records: usize,
    pub memory_promotion_no_candidate_cycles: usize,
    pub memory_promotion_promotable_cycles: usize,
    pub memory_promotion_watch_cycles: usize,
    pub memory_promotion_blocked_cycles: usize,
    pub memory_promotion_repair_cycles: usize,
    pub tool_build_blocked_cycles: usize,
    pub max_consecutive_blocked_cycles: usize,
    pub latest_blocked_records: usize,
    pub low_acceptance_records: usize,
    pub low_reward_records: usize,
    pub average_acceptance_rate: f32,
    pub average_reward_total: f32,
    pub promotion_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentCycleLedgerHealthPolicy {
    pub maximum_blocked_cycles: usize,
    pub maximum_enqueued_tasks: usize,
    pub maximum_consecutive_blocked_cycles: usize,
    pub maximum_latest_blocked_records: usize,
    pub maximum_memory_promotion_blocked_cycles: usize,
    pub maximum_memory_promotion_repair_cycles: usize,
    pub maximum_tool_build_blocked_cycles: usize,
    pub maximum_low_acceptance_records: usize,
    pub maximum_low_reward_records: usize,
    pub minimum_average_acceptance_rate: f32,
    pub minimum_average_reward_total: f32,
}

impl Default for AgentCycleLedgerHealthPolicy {
    fn default() -> Self {
        Self {
            maximum_blocked_cycles: 0,
            maximum_enqueued_tasks: usize::MAX,
            maximum_consecutive_blocked_cycles: 2,
            maximum_latest_blocked_records: 0,
            maximum_memory_promotion_blocked_cycles: 0,
            maximum_memory_promotion_repair_cycles: 0,
            maximum_tool_build_blocked_cycles: 0,
            maximum_low_acceptance_records: 0,
            maximum_low_reward_records: 0,
            minimum_average_acceptance_rate: 0.5,
            minimum_average_reward_total: 0.62,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleLedgerHealth {
    pub status: AgentCycleLedgerHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AgentCycleLedgerDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleLedgerSummaryHistoryRecord {
    pub history: AgentCycleLedgerSummaryHistory,
    pub appended_summary: AgentCycleLedgerSummary,
    pub dashboard: AgentCycleLedgerDashboard,
    pub health: AgentCycleLedgerHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentCycleLedgerSummaryHistoryRecorder;

impl AgentCycleLedgerSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AgentCycleLedgerSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AgentCycleLedgerSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AgentCycleLedgerSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AgentCycleLedgerSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AgentCycleLedgerDashboard {
        AgentCycleLedgerDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AgentCycleLedgerHealthPolicy) -> AgentCycleLedgerHealth {
        self.dashboard().health(policy)
    }
}

impl AgentCycleLedgerDashboard {
    pub fn from_summaries(summaries: &[AgentCycleLedgerSummary]) -> Self {
        let total_records = summaries.len();
        let total_cycles = summaries
            .iter()
            .map(|summary| summary.total_cycles)
            .sum::<usize>();
        let accepted_cycles = summaries
            .iter()
            .map(|summary| summary.accepted_cycles)
            .sum::<usize>();
        let blocked_cycles = summaries
            .iter()
            .map(|summary| summary.blocked_cycles)
            .sum::<usize>();
        let adaptive_promotions = summaries
            .iter()
            .map(|summary| summary.adaptive_promotions)
            .sum::<usize>();
        let enqueued_tasks = summaries
            .iter()
            .map(|summary| summary.enqueued_tasks)
            .sum::<usize>();
        let memory_promotion_records = summaries
            .iter()
            .map(|summary| summary.memory_promotion_records)
            .sum::<usize>();
        let memory_promotion_no_candidate_cycles = summaries
            .iter()
            .map(|summary| summary.memory_promotion_no_candidate_cycles)
            .sum::<usize>();
        let memory_promotion_promotable_cycles = summaries
            .iter()
            .map(|summary| summary.memory_promotion_promotable_cycles)
            .sum::<usize>();
        let memory_promotion_watch_cycles = summaries
            .iter()
            .map(|summary| summary.memory_promotion_watch_cycles)
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
        let max_consecutive_blocked_cycles = summaries
            .iter()
            .map(|summary| summary.consecutive_blocked_cycles)
            .max()
            .unwrap_or_default();
        let latest_blocked_records = summaries
            .iter()
            .filter(|summary| !summary.latest_blocked_reasons.is_empty())
            .count();
        let low_acceptance_records = summaries
            .iter()
            .filter(|summary| summary.total_cycles > 0 && summary.acceptance_rate < 0.5)
            .count();
        let low_reward_records = summaries
            .iter()
            .filter(|summary| summary.total_cycles > 0 && summary.average_reward_total < 0.62)
            .count();
        let average_acceptance_rate = if total_records == 0 {
            0.0
        } else {
            summaries
                .iter()
                .map(|summary| summary.acceptance_rate)
                .sum::<f32>()
                / total_records as f32
        };
        let average_reward_total = if total_records == 0 {
            0.0
        } else {
            summaries
                .iter()
                .map(|summary| summary.average_reward_total)
                .sum::<f32>()
                / total_records as f32
        };
        let promotion_rate = rate(adaptive_promotions, total_cycles);
        let telemetry = agent_cycle_ledger_dashboard_telemetry(
            total_records,
            total_cycles,
            accepted_cycles,
            blocked_cycles,
            adaptive_promotions,
            enqueued_tasks,
            memory_promotion_records,
            memory_promotion_no_candidate_cycles,
            memory_promotion_promotable_cycles,
            memory_promotion_watch_cycles,
            memory_promotion_blocked_cycles,
            memory_promotion_repair_cycles,
            tool_build_blocked_cycles,
            max_consecutive_blocked_cycles,
            latest_blocked_records,
            low_acceptance_records,
            low_reward_records,
            average_acceptance_rate,
            average_reward_total,
            promotion_rate,
        );

        Self {
            total_records,
            total_cycles,
            accepted_cycles,
            blocked_cycles,
            adaptive_promotions,
            enqueued_tasks,
            memory_promotion_records,
            memory_promotion_no_candidate_cycles,
            memory_promotion_promotable_cycles,
            memory_promotion_watch_cycles,
            memory_promotion_blocked_cycles,
            memory_promotion_repair_cycles,
            tool_build_blocked_cycles,
            max_consecutive_blocked_cycles,
            latest_blocked_records,
            low_acceptance_records,
            low_reward_records,
            average_acceptance_rate,
            average_reward_total,
            promotion_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: AgentCycleLedgerHealthPolicy) -> AgentCycleLedgerHealth {
        AgentCycleLedgerHealth::from_dashboard(self.clone(), policy)
    }
}

impl AgentCycleLedgerHealth {
    pub fn from_dashboard(
        dashboard: AgentCycleLedgerDashboard,
        policy: AgentCycleLedgerHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("agent_cycle_ledger_history_empty".to_owned());
        } else if dashboard.average_acceptance_rate < policy.minimum_average_acceptance_rate {
            watch_reasons.push(format!(
                "agent_cycle_ledger_average_acceptance_rate={:.3}<{}",
                dashboard.average_acceptance_rate, policy.minimum_average_acceptance_rate
            ));
        }

        if !dashboard.is_empty()
            && dashboard.average_reward_total < policy.minimum_average_reward_total
        {
            watch_reasons.push(format!(
                "agent_cycle_ledger_average_reward_total={:.3}<{}",
                dashboard.average_reward_total, policy.minimum_average_reward_total
            ));
        }

        if dashboard.blocked_cycles > policy.maximum_blocked_cycles {
            repair_reasons.push(format!(
                "agent_cycle_ledger_blocked_cycles={}>{}",
                dashboard.blocked_cycles, policy.maximum_blocked_cycles
            ));
        }

        if dashboard.enqueued_tasks > policy.maximum_enqueued_tasks {
            watch_reasons.push(format!(
                "agent_cycle_ledger_enqueued_tasks={}>{}",
                dashboard.enqueued_tasks, policy.maximum_enqueued_tasks
            ));
        }

        if dashboard.max_consecutive_blocked_cycles > policy.maximum_consecutive_blocked_cycles {
            repair_reasons.push(format!(
                "agent_cycle_ledger_consecutive_blocked_cycles={}>{}",
                dashboard.max_consecutive_blocked_cycles, policy.maximum_consecutive_blocked_cycles
            ));
        }

        if dashboard.latest_blocked_records > policy.maximum_latest_blocked_records {
            repair_reasons.push(format!(
                "agent_cycle_ledger_latest_blocked_records={}>{}",
                dashboard.latest_blocked_records, policy.maximum_latest_blocked_records
            ));
        }

        if dashboard.memory_promotion_blocked_cycles
            > policy.maximum_memory_promotion_blocked_cycles
        {
            repair_reasons.push(format!(
                "agent_cycle_ledger_memory_promotion_blocked_cycles={}>{}",
                dashboard.memory_promotion_blocked_cycles,
                policy.maximum_memory_promotion_blocked_cycles
            ));
        }

        if dashboard.memory_promotion_repair_cycles > policy.maximum_memory_promotion_repair_cycles
        {
            repair_reasons.push(format!(
                "agent_cycle_ledger_memory_promotion_repair_cycles={}>{}",
                dashboard.memory_promotion_repair_cycles,
                policy.maximum_memory_promotion_repair_cycles
            ));
        }

        if dashboard.tool_build_blocked_cycles > policy.maximum_tool_build_blocked_cycles {
            repair_reasons.push(format!(
                "agent_cycle_ledger_tool_build_blocked_cycles={}>{}",
                dashboard.tool_build_blocked_cycles, policy.maximum_tool_build_blocked_cycles
            ));
        }

        if dashboard.low_acceptance_records > policy.maximum_low_acceptance_records {
            repair_reasons.push(format!(
                "agent_cycle_ledger_low_acceptance_records={}>{}",
                dashboard.low_acceptance_records, policy.maximum_low_acceptance_records
            ));
        }

        if dashboard.low_reward_records > policy.maximum_low_reward_records {
            repair_reasons.push(format!(
                "agent_cycle_ledger_low_reward_records={}>{}",
                dashboard.low_reward_records, policy.maximum_low_reward_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AgentCycleLedgerHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AgentCycleLedgerHealthStatus::Watch, watch_reasons)
        } else {
            (AgentCycleLedgerHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AgentCycleLedgerHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AgentCycleLedgerHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AgentCycleLedgerHealthStatus::Repair
    }
}

impl AgentCycleLedgerSummaryHistoryRecord {
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

impl AgentCycleLedgerSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AgentCycleLedgerSummaryHistory,
        summary: AgentCycleLedgerSummary,
        policy: AgentCycleLedgerHealthPolicy,
    ) -> AgentCycleLedgerSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = agent_cycle_ledger_history_record_telemetry(&dashboard, &health);

        AgentCycleLedgerSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_ledger_with_health(
        &self,
        history: AgentCycleLedgerSummaryHistory,
        ledger: &AgentCycleLedger,
        policy: AgentCycleLedgerHealthPolicy,
    ) -> AgentCycleLedgerSummaryHistoryRecord {
        self.record_summary_with_health(history, ledger.summary(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentCycleLedgerPolicy {
    pub max_consecutive_blocked_cycles: usize,
    pub minimum_acceptance_rate: f32,
    pub minimum_average_reward_total: f32,
    pub maximum_memory_promotion_blocked_cycles: usize,
    pub maximum_memory_promotion_repair_cycles: usize,
    pub maximum_tool_build_blocked_cycles: usize,
}

impl Default for AgentCycleLedgerPolicy {
    fn default() -> Self {
        Self {
            max_consecutive_blocked_cycles: 3,
            minimum_acceptance_rate: 0.5,
            minimum_average_reward_total: 0.62,
            maximum_memory_promotion_blocked_cycles: 0,
            maximum_memory_promotion_repair_cycles: 0,
            maximum_tool_build_blocked_cycles: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentCycleLedgerAdmissionStatus {
    Promote,
    Hold,
    Repair,
}

impl AgentCycleLedgerAdmissionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Promote => "promote",
            Self::Hold => "hold",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCycleLedgerAdmissionDecision {
    pub status: AgentCycleLedgerAdmissionStatus,
    pub reasons: Vec<String>,
    pub summary: AgentCycleLedgerSummary,
}

fn consecutive_blocked_cycles(entries: &[AgentCycleLedgerEntry]) -> usize {
    entries
        .iter()
        .rev()
        .take_while(|entry| !entry.is_accepted())
        .count()
}

fn memory_promotion_status_count(
    entries: &[AgentCycleLedgerEntry],
    status: MemoryPromotionLedgerStatus,
) -> usize {
    entries
        .iter()
        .filter(|entry| {
            entry
                .record
                .memory_promotion
                .as_ref()
                .is_some_and(|summary| summary.status == status)
        })
        .count()
}

fn report_gate_tool_build_blocked_cycles(entries: &[AgentCycleLedgerEntry]) -> usize {
    entries
        .iter()
        .filter(|entry| {
            entry
                .report_decision
                .reasons
                .iter()
                .any(|reason| reason.code.starts_with("tool_build_"))
        })
        .count()
}

#[allow(clippy::too_many_arguments)]
fn agent_cycle_ledger_dashboard_telemetry(
    total_records: usize,
    total_cycles: usize,
    accepted_cycles: usize,
    blocked_cycles: usize,
    adaptive_promotions: usize,
    enqueued_tasks: usize,
    memory_promotion_records: usize,
    memory_promotion_no_candidate_cycles: usize,
    memory_promotion_promotable_cycles: usize,
    memory_promotion_watch_cycles: usize,
    memory_promotion_blocked_cycles: usize,
    memory_promotion_repair_cycles: usize,
    tool_build_blocked_cycles: usize,
    max_consecutive_blocked_cycles: usize,
    latest_blocked_records: usize,
    low_acceptance_records: usize,
    low_reward_records: usize,
    average_acceptance_rate: f32,
    average_reward_total: f32,
    promotion_rate: f32,
) -> Vec<String> {
    vec![
        "agent_cycle_ledger_dashboard=true".to_owned(),
        format!("agent_cycle_ledger_dashboard_records={total_records}"),
        format!("agent_cycle_ledger_dashboard_total_cycles={total_cycles}"),
        format!("agent_cycle_ledger_dashboard_accepted_cycles={accepted_cycles}"),
        format!("agent_cycle_ledger_dashboard_blocked_cycles={blocked_cycles}"),
        format!("agent_cycle_ledger_dashboard_adaptive_promotions={adaptive_promotions}"),
        format!("agent_cycle_ledger_dashboard_enqueued_tasks={enqueued_tasks}"),
        format!("agent_cycle_ledger_dashboard_memory_promotion_records={memory_promotion_records}"),
        format!(
            "agent_cycle_ledger_dashboard_memory_promotion_no_candidate_cycles={memory_promotion_no_candidate_cycles}"
        ),
        format!(
            "agent_cycle_ledger_dashboard_memory_promotion_promotable_cycles={memory_promotion_promotable_cycles}"
        ),
        format!(
            "agent_cycle_ledger_dashboard_memory_promotion_watch_cycles={memory_promotion_watch_cycles}"
        ),
        format!(
            "agent_cycle_ledger_dashboard_memory_promotion_blocked_cycles={memory_promotion_blocked_cycles}"
        ),
        format!(
            "agent_cycle_ledger_dashboard_memory_promotion_repair_cycles={memory_promotion_repair_cycles}"
        ),
        format!(
            "agent_cycle_ledger_dashboard_tool_build_blocked_cycles={tool_build_blocked_cycles}"
        ),
        format!(
            "agent_cycle_ledger_dashboard_max_consecutive_blocked_cycles={max_consecutive_blocked_cycles}"
        ),
        format!("agent_cycle_ledger_dashboard_latest_blocked_records={latest_blocked_records}"),
        format!("agent_cycle_ledger_dashboard_low_acceptance_records={low_acceptance_records}"),
        format!("agent_cycle_ledger_dashboard_low_reward_records={low_reward_records}"),
        format!(
            "agent_cycle_ledger_dashboard_average_acceptance_rate={average_acceptance_rate:.3}"
        ),
        format!("agent_cycle_ledger_dashboard_average_reward_total={average_reward_total:.3}"),
        format!("agent_cycle_ledger_dashboard_promotion_rate={promotion_rate:.3}"),
    ]
}

fn agent_cycle_ledger_history_record_telemetry(
    dashboard: &AgentCycleLedgerDashboard,
    health: &AgentCycleLedgerHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_cycle_ledger_history_record=true".to_owned(),
        format!(
            "agent_cycle_ledger_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_cycle_ledger_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_cycle_ledger_history_record_average_acceptance_rate={:.3}",
            dashboard.average_acceptance_rate
        ),
        format!(
            "agent_cycle_ledger_history_record_average_reward_total={:.3}",
            dashboard.average_reward_total
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_cycle_ledger_history_record_reason={reason}")),
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
    use crate::cycle::AgentCycleSummary;
    use crate::eval::{
        AgentReportEvidence, AgentReportGateReason, MemoryPromotionLedgerStatus,
        MemoryPromotionLedgerSummary,
    };
    use crate::evolution::RewardAction;
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
            memory_promotions: 0,
            tool_build_reports: 0,
            tool_build_missing_requests: 0,
            tool_build_unexpected_receipts: 0,
            tool_build_duplicate_receipts: 0,
            tool_build_held_receipts: 0,
            tool_build_rejected_receipts: 0,
        }
    }

    fn record(run_id: &str, reward_total: f32) -> AgentCycleLedgerRecord {
        AgentCycleLedgerRecord::new(
            run_id,
            summary(reward_total),
            AgentReportEvidence::new(true, true)
                .with_validation_ref("eval:pass")
                .with_runtime_ref("runtime:ok"),
            None,
        )
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
        AgentCycleLedgerEntry::new(
            record(run_id, 0.90).with_memory_promotion_summary(promotion_summary(status)),
            AgentReportGateDecision {
                accepted: status == MemoryPromotionLedgerStatus::Promotable,
                reasons: Vec::new(),
                follow_up_tasks: Vec::new(),
            },
            AgentLoopbackPlan {
                promote_adaptive_state: status == MemoryPromotionLedgerStatus::Promotable,
                enqueue_tasks: Vec::new(),
                blocked_reasons: Vec::new(),
            },
        )
    }

    fn accepted_entry(run_id: &str, reward_total: f32) -> AgentCycleLedgerEntry {
        AgentCycleLedgerEntry::new(
            record(run_id, reward_total),
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

    fn blocked_entry(run_id: &str, reward_total: f32, reason: &str) -> AgentCycleLedgerEntry {
        let repair = AgentTask::new(
            format!("repair-{run_id}"),
            AgentRole::Reviewer,
            "repair blocked cycle",
            crate::budget::AgentBudget::new(8, 1, 1),
        );
        AgentCycleLedgerEntry::new(
            record(run_id, reward_total),
            AgentReportGateDecision {
                accepted: false,
                reasons: vec![AgentReportGateReason::new(reason, "1")],
                follow_up_tasks: vec![repair.clone()],
            },
            AgentLoopbackPlan {
                promote_adaptive_state: false,
                enqueue_tasks: vec![repair],
                blocked_reasons: vec![format!("{reason}=1")],
            },
        )
    }

    #[test]
    fn ledger_summary_history_watches_empty() {
        let health =
            AgentCycleLedgerSummaryHistory::new().health(AgentCycleLedgerHealthPolicy::default());

        assert_eq!(health.status, AgentCycleLedgerHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["agent_cycle_ledger_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| line == "agent_cycle_ledger_dashboard_records=0")
        );
    }

    #[test]
    fn ledger_summary_history_marks_clean_promoting_ledger_stable() {
        let ledger = AgentCycleLedger::from_entries(vec![
            accepted_entry("run-1", 0.84),
            accepted_entry("run-2", 0.88),
        ]);

        let record = AgentCycleLedgerSummaryHistoryRecorder::new().record_ledger_with_health(
            AgentCycleLedgerSummaryHistory::new(),
            &ledger,
            AgentCycleLedgerHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert_eq!(record.appended_summary.total_cycles, 2);
        assert_eq!(record.dashboard.total_cycles, 2);
        assert_eq!(record.dashboard.accepted_cycles, 2);
        assert_eq!(record.dashboard.blocked_cycles, 0);
        assert_eq!(record.dashboard.adaptive_promotions, 2);
        assert_eq!(record.dashboard.tool_build_blocked_cycles, 0);
        assert_eq!(record.dashboard.average_acceptance_rate, 1.0);
        assert_eq!(record.dashboard.promotion_rate, 1.0);
        assert_eq!(record.health.status, AgentCycleLedgerHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.health.allows_service_advance());
        assert!(!record.health.requires_repair_first());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| line == "agent_cycle_ledger_history_record_status=stable")
        );
    }

    #[test]
    fn ledger_summary_history_repairs_blocked_and_low_reward_pressure() {
        let clean = AgentCycleLedger::from_entries(vec![
            accepted_entry("run-1", 0.84),
            accepted_entry("run-2", 0.88),
        ])
        .summary();
        let dirty = AgentCycleLedger::from_entries(vec![
            blocked_entry("run-3", 0.38, "execution_failures"),
            blocked_entry("run-4", 0.41, "execution_failures"),
            blocked_entry("run-5", 0.44, "execution_failures"),
        ])
        .summary();
        let history = AgentCycleLedgerSummaryHistory::from_summaries(vec![clean]);

        let record = AgentCycleLedgerSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            dirty,
            AgentCycleLedgerHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.total_cycles, 5);
        assert_eq!(record.dashboard.accepted_cycles, 2);
        assert_eq!(record.dashboard.blocked_cycles, 3);
        assert_eq!(record.dashboard.enqueued_tasks, 3);
        assert_eq!(record.dashboard.max_consecutive_blocked_cycles, 3);
        assert_eq!(record.dashboard.latest_blocked_records, 1);
        assert_eq!(record.dashboard.tool_build_blocked_cycles, 0);
        assert_eq!(record.dashboard.low_acceptance_records, 1);
        assert_eq!(record.dashboard.low_reward_records, 1);
        assert_eq!(record.health.status, AgentCycleLedgerHealthStatus::Repair);
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "agent_cycle_ledger_blocked_cycles=3>0".to_owned(),
                "agent_cycle_ledger_consecutive_blocked_cycles=3>2".to_owned(),
                "agent_cycle_ledger_latest_blocked_records=1>0".to_owned(),
                "agent_cycle_ledger_low_acceptance_records=1>0".to_owned(),
                "agent_cycle_ledger_low_reward_records=1>0".to_owned(),
            ]
        );
    }

    #[test]
    fn ledger_summary_tracks_acceptance_and_latest_blockers() {
        let ledger = AgentCycleLedger::from_entries(vec![
            accepted_entry("run-1", 0.90),
            blocked_entry("run-2", 0.40, "unresolved_conflicts"),
            blocked_entry("run-3", 0.50, "runtime_evidence_missing"),
        ]);

        let summary = ledger.summary();

        assert_eq!(summary.total_cycles, 3);
        assert_eq!(summary.accepted_cycles, 1);
        assert_eq!(summary.blocked_cycles, 2);
        assert_eq!(summary.adaptive_promotions, 1);
        assert_eq!(summary.enqueued_tasks, 2);
        assert_eq!(summary.consecutive_blocked_cycles, 2);
        assert!((summary.acceptance_rate - 0.333).abs() < 0.01);
        assert!((summary.average_reward_total - 0.60).abs() < 0.01);
        assert_eq!(summary.latest_run_id.as_deref(), Some("run-3"));
        assert_eq!(
            summary.latest_blocked_reasons,
            vec!["runtime_evidence_missing=1"]
        );
        assert_eq!(summary.tool_build_blocked_cycles, 0);
    }

    #[test]
    fn ledger_summary_repairs_tool_build_report_gate_pressure() {
        let ledger = AgentCycleLedger::from_entries(vec![blocked_entry(
            "run-tool-build",
            0.72,
            "tool_build_held_receipts",
        )]);

        let summary = ledger.summary();
        let record = AgentCycleLedgerSummaryHistoryRecorder::new().record_ledger_with_health(
            AgentCycleLedgerSummaryHistory::new(),
            &ledger,
            AgentCycleLedgerHealthPolicy::default(),
        );
        let admission = ledger.admission(AgentCycleLedgerPolicy::default());

        assert_eq!(summary.tool_build_blocked_cycles, 1);
        assert_eq!(record.dashboard.tool_build_blocked_cycles, 1);
        assert_eq!(record.health.status, AgentCycleLedgerHealthStatus::Repair);
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "agent_cycle_ledger_tool_build_blocked_cycles=1>0" })
        );
        assert!(
            record
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_cycle_ledger_dashboard_tool_build_blocked_cycles=1" })
        );
        assert_eq!(admission.status, AgentCycleLedgerAdmissionStatus::Repair);
        assert!(
            admission
                .reasons
                .iter()
                .any(|reason| { reason == "tool_build_blocked_cycles=1>0" })
        );
    }

    #[test]
    fn ledger_summary_tracks_pre_submit_memory_promotion_gate_statuses() {
        let ledger = AgentCycleLedger::from_entries(vec![
            accepted_entry_with_memory_promotion("run-1", MemoryPromotionLedgerStatus::Promotable),
            accepted_entry_with_memory_promotion("run-2", MemoryPromotionLedgerStatus::Watch),
            accepted_entry_with_memory_promotion("run-3", MemoryPromotionLedgerStatus::Blocked),
            accepted_entry_with_memory_promotion("run-4", MemoryPromotionLedgerStatus::Repair),
            accepted_entry_with_memory_promotion(
                "run-5",
                MemoryPromotionLedgerStatus::NoCandidates,
            ),
        ]);

        let summary = ledger.summary();
        let record = AgentCycleLedgerSummaryHistoryRecorder::new().record_ledger_with_health(
            AgentCycleLedgerSummaryHistory::new(),
            &ledger,
            AgentCycleLedgerHealthPolicy::default(),
        );

        assert_eq!(summary.memory_promotion_records, 5);
        assert_eq!(summary.memory_promotion_promotable_cycles, 1);
        assert_eq!(summary.memory_promotion_watch_cycles, 1);
        assert_eq!(summary.memory_promotion_blocked_cycles, 1);
        assert_eq!(summary.memory_promotion_repair_cycles, 1);
        assert_eq!(summary.memory_promotion_no_candidate_cycles, 1);
        assert_eq!(record.dashboard.memory_promotion_records, 5);
        assert_eq!(record.dashboard.memory_promotion_blocked_cycles, 1);
        assert_eq!(record.dashboard.memory_promotion_repair_cycles, 1);
        assert_eq!(record.health.status, AgentCycleLedgerHealthStatus::Repair);
        assert!(
            record.health.reasons.iter().any(|reason| {
                reason == "agent_cycle_ledger_memory_promotion_blocked_cycles=1>0"
            })
        );
        assert!(
            record.health.reasons.iter().any(|reason| {
                reason == "agent_cycle_ledger_memory_promotion_repair_cycles=1>0"
            })
        );
        assert!(record.dashboard.telemetry.iter().any(|line| {
            line == "agent_cycle_ledger_dashboard_memory_promotion_repair_cycles=1"
        }));
    }

    #[test]
    fn ledger_admission_promotes_only_clean_latest_cycle_and_trend() {
        let ledger = AgentCycleLedger::from_entries(vec![
            accepted_entry("run-1", 0.84),
            accepted_entry("run-2", 0.88),
        ]);

        let decision = ledger.admission(AgentCycleLedgerPolicy::default());

        assert_eq!(decision.status, AgentCycleLedgerAdmissionStatus::Promote);
        assert!(decision.reasons.is_empty());
        assert_eq!(decision.status.as_str(), "promote");
    }

    #[test]
    fn ledger_admission_repairs_after_repeated_blocked_cycles() {
        let ledger = AgentCycleLedger::from_entries(vec![
            blocked_entry("run-1", 0.38, "execution_failures"),
            blocked_entry("run-2", 0.41, "execution_failures"),
            blocked_entry("run-3", 0.44, "execution_failures"),
        ]);

        let decision = ledger.admission(AgentCycleLedgerPolicy::default());

        assert_eq!(decision.status, AgentCycleLedgerAdmissionStatus::Repair);
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| reason == "consecutive_blocked_cycles=3")
        );
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| reason == "latest_blocked=execution_failures=1")
        );
    }

    #[test]
    fn ledger_admission_repairs_memory_promotion_gate_pressure_before_business_promotion() {
        let ledger = AgentCycleLedger::from_entries(vec![
            accepted_entry_with_memory_promotion("run-1", MemoryPromotionLedgerStatus::Promotable),
            accepted_entry_with_memory_promotion("run-2", MemoryPromotionLedgerStatus::Blocked),
            accepted_entry_with_memory_promotion("run-3", MemoryPromotionLedgerStatus::Repair),
        ]);

        let decision = ledger.admission(AgentCycleLedgerPolicy::default());

        assert_eq!(decision.status, AgentCycleLedgerAdmissionStatus::Repair);
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| { reason == "memory_promotion_blocked_cycles=1>0" })
        );
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| { reason == "memory_promotion_repair_cycles=1>0" })
        );
    }

    #[test]
    fn empty_ledger_holds_business_loop_admission() {
        let decision = AgentCycleLedger::new().admission(AgentCycleLedgerPolicy::default());

        assert_eq!(decision.status, AgentCycleLedgerAdmissionStatus::Hold);
        assert_eq!(decision.reasons, vec!["ledger_empty"]);
        assert_eq!(decision.summary.total_cycles, 0);
    }
}
