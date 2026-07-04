use std::collections::BTreeMap;

use crate::budget::AgentBudget;
use crate::conflict::{
    ConflictReport, ConflictReportHealthPolicy, ConflictReportHealthStatus,
    ConflictReportHistoryGateRecord, ConflictReportSummaryHistory,
    ConflictReportSummaryHistoryRecorder, ConflictResolver,
};
use crate::message::AgentMessage;
use crate::task::{AgentRole, AgentTask, TaskDispatchPlanSummary};

#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedMessage {
    pub message: AgentMessage,
    pub duplicate_count: usize,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AggregationReport {
    pub input_count: usize,
    pub unique_count: usize,
    pub duplicate_groups: usize,
    pub messages: Vec<AggregatedMessage>,
}

impl AggregationReport {
    pub fn summary(&self) -> AggregationSummary {
        AggregationSummary::from_report(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationSummary {
    pub input_count: usize,
    pub unique_count: usize,
    pub duplicate_groups: usize,
    pub duplicate_messages: usize,
    pub compression_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AggregationHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AggregationSummaryHistory {
    summaries: Vec<AggregationSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationDashboard {
    pub total_records: usize,
    pub input_count: usize,
    pub unique_count: usize,
    pub duplicate_groups: usize,
    pub duplicate_messages: usize,
    pub duplicate_records: usize,
    pub empty_records: usize,
    pub aggregate_compression_rate: f32,
    pub duplicate_record_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AggregationHealthPolicy {
    pub minimum_aggregate_compression_rate: f32,
    pub maximum_duplicate_records: usize,
    pub maximum_duplicate_messages: usize,
    pub maximum_duplicate_groups: usize,
    pub maximum_empty_records: usize,
}

impl Default for AggregationHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_aggregate_compression_rate: 0.67,
            maximum_duplicate_records: 0,
            maximum_duplicate_messages: 0,
            maximum_duplicate_groups: 0,
            maximum_empty_records: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationHealth {
    pub status: AggregationHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AggregationDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationSummaryHistoryRecord {
    pub history: AggregationSummaryHistory,
    pub appended_summary: AggregationSummary,
    pub dashboard: AggregationDashboard,
    pub health: AggregationHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AggregationSummaryHistoryRecorder;

impl AggregationSummary {
    pub fn from_report(report: &AggregationReport) -> Self {
        let duplicate_messages = report.input_count.saturating_sub(report.unique_count);
        let compression_rate = rate(report.unique_count, report.input_count);
        let telemetry = aggregation_summary_telemetry(
            report.input_count,
            report.unique_count,
            report.duplicate_groups,
            duplicate_messages,
            compression_rate,
        );

        Self {
            input_count: report.input_count,
            unique_count: report.unique_count,
            duplicate_groups: report.duplicate_groups,
            duplicate_messages,
            compression_rate,
            telemetry,
        }
    }
}

impl AggregationSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AggregationSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AggregationSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AggregationSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AggregationSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AggregationDashboard {
        AggregationDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: AggregationHealthPolicy) -> AggregationHealth {
        self.dashboard().health(policy)
    }
}

impl AggregationDashboard {
    pub fn from_summaries(summaries: &[AggregationSummary]) -> Self {
        let total_records = summaries.len();
        let input_count = summaries
            .iter()
            .map(|summary| summary.input_count)
            .sum::<usize>();
        let unique_count = summaries
            .iter()
            .map(|summary| summary.unique_count)
            .sum::<usize>();
        let duplicate_groups = summaries
            .iter()
            .map(|summary| summary.duplicate_groups)
            .sum::<usize>();
        let duplicate_messages = summaries
            .iter()
            .map(|summary| summary.duplicate_messages)
            .sum::<usize>();
        let duplicate_records = summaries
            .iter()
            .filter(|summary| summary.duplicate_messages > 0)
            .count();
        let empty_records = summaries
            .iter()
            .filter(|summary| summary.input_count == 0)
            .count();
        let aggregate_compression_rate = rate(unique_count, input_count);
        let duplicate_record_rate = rate(duplicate_records, total_records);
        let telemetry = aggregation_dashboard_telemetry(
            total_records,
            input_count,
            unique_count,
            duplicate_groups,
            duplicate_messages,
            duplicate_records,
            empty_records,
            aggregate_compression_rate,
            duplicate_record_rate,
        );

        Self {
            total_records,
            input_count,
            unique_count,
            duplicate_groups,
            duplicate_messages,
            duplicate_records,
            empty_records,
            aggregate_compression_rate,
            duplicate_record_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: AggregationHealthPolicy) -> AggregationHealth {
        AggregationHealth::from_dashboard(self.clone(), policy)
    }
}

impl AggregationHealth {
    pub fn from_dashboard(
        dashboard: AggregationDashboard,
        policy: AggregationHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("aggregation_history_empty".to_owned());
        } else if dashboard.aggregate_compression_rate < policy.minimum_aggregate_compression_rate {
            watch_reasons.push(format!(
                "aggregation_compression_rate={:.3}<{}",
                dashboard.aggregate_compression_rate, policy.minimum_aggregate_compression_rate
            ));
        }

        if dashboard.duplicate_records > policy.maximum_duplicate_records {
            repair_reasons.push(format!(
                "aggregation_duplicate_records={}>{}",
                dashboard.duplicate_records, policy.maximum_duplicate_records
            ));
        }

        if dashboard.duplicate_messages > policy.maximum_duplicate_messages {
            repair_reasons.push(format!(
                "aggregation_duplicate_messages={}>{}",
                dashboard.duplicate_messages, policy.maximum_duplicate_messages
            ));
        }

        if dashboard.duplicate_groups > policy.maximum_duplicate_groups {
            repair_reasons.push(format!(
                "aggregation_duplicate_groups={}>{}",
                dashboard.duplicate_groups, policy.maximum_duplicate_groups
            ));
        }

        if dashboard.empty_records > policy.maximum_empty_records {
            watch_reasons.push(format!(
                "aggregation_empty_records={}>{}",
                dashboard.empty_records, policy.maximum_empty_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (AggregationHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (AggregationHealthStatus::Watch, watch_reasons)
        } else {
            (AggregationHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AggregationHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AggregationHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AggregationHealthStatus::Repair
    }
}

impl AggregationSummaryHistoryRecord {
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

impl AggregationSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AggregationSummaryHistory,
        summary: AggregationSummary,
        policy: AggregationHealthPolicy,
    ) -> AggregationSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = aggregation_history_record_telemetry(&dashboard, &health);

        AggregationSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_report_with_health(
        &self,
        history: AggregationSummaryHistory,
        report: &AggregationReport,
        policy: AggregationHealthPolicy,
    ) -> AggregationSummaryHistoryRecord {
        self.record_summary_with_health(history, report.summary(), policy)
    }

    pub fn record_report_with_health_gate(
        &self,
        history: AggregationSummaryHistory,
        report: &AggregationReport,
        policy: AggregationHealthPolicy,
    ) -> AggregationHistoryGateRecord {
        let health_record = self.record_report_with_health(history, report, policy);
        let gate_decision = AggregationHistoryGate::new().gate(report, &health_record);
        let telemetry = aggregation_history_gate_record_telemetry(&health_record, &gate_decision);

        AggregationHistoryGateRecord {
            health_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationHistoryGateDecision {
    pub report_summary: AggregationSummary,
    pub aggregation_health: AggregationHealth,
    pub can_forward_aggregated_messages: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AggregationHistoryGateDecision {
    pub fn is_forwardable(&self) -> bool {
        self.can_forward_aggregated_messages && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationHistoryGateRecord {
    pub health_record: AggregationSummaryHistoryRecord,
    pub gate_decision: AggregationHistoryGateDecision,
    pub telemetry: Vec<String>,
}

impl AggregationHistoryGateRecord {
    pub fn records(&self) -> usize {
        self.health_record.records()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn can_forward_aggregated_messages(&self) -> bool {
        self.gate_decision.can_forward_aggregated_messages
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationConflictReview {
    pub aggregation_record: AggregationHistoryGateRecord,
    pub conflict_record: ConflictReportHistoryGateRecord,
    pub conflict_report: ConflictReport,
    pub can_forward_messages: bool,
    pub can_promote_side_effects: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AggregationConflictReview {
    pub fn is_forwardable(&self) -> bool {
        self.can_forward_messages && !self.requires_repair_first
    }

    pub fn is_side_effect_safe(&self) -> bool {
        self.can_promote_side_effects && !self.requires_repair_first
    }

    pub fn repair_task_ids(&self) -> Vec<String> {
        self.repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect()
    }

    pub fn summary(&self) -> AggregationConflictReviewSummary {
        AggregationConflictReviewSummary::from_review(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationConflictReviewSummary {
    pub aggregation_health_status: AggregationHealthStatus,
    pub conflict_health_status: ConflictReportHealthStatus,
    pub can_forward_messages: bool,
    pub can_promote_side_effects: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: usize,
    pub unique_messages: usize,
    pub duplicate_messages: usize,
    pub unresolved_conflicts: usize,
    pub conflicted_messages: usize,
    pub repair_task_ids: Vec<String>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AggregationConflictReviewSummary {
    pub fn from_review(review: &AggregationConflictReview) -> Self {
        let repair_task_ids = review.repair_task_ids();
        let aggregation_summary = &review.aggregation_record.gate_decision.report_summary;
        let conflict_summary = &review.conflict_record.gate_decision.report_summary;
        let telemetry = aggregation_conflict_review_summary_telemetry(
            review
                .aggregation_record
                .gate_decision
                .aggregation_health
                .status,
            review.conflict_record.gate_decision.conflict_health.status,
            review.can_forward_messages,
            review.can_promote_side_effects,
            review.requires_repair_first,
            repair_task_ids.len(),
            aggregation_summary.unique_count,
            aggregation_summary.duplicate_messages,
            conflict_summary.unresolved_conflicts,
            conflict_summary.conflicted_messages,
            review.reasons.len(),
        );

        Self {
            aggregation_health_status: review
                .aggregation_record
                .gate_decision
                .aggregation_health
                .status,
            conflict_health_status: review.conflict_record.gate_decision.conflict_health.status,
            can_forward_messages: review.can_forward_messages,
            can_promote_side_effects: review.can_promote_side_effects,
            requires_repair_first: review.requires_repair_first,
            repair_tasks: repair_task_ids.len(),
            unique_messages: aggregation_summary.unique_count,
            duplicate_messages: aggregation_summary.duplicate_messages,
            unresolved_conflicts: conflict_summary.unresolved_conflicts,
            conflicted_messages: conflict_summary.conflicted_messages,
            repair_task_ids,
            reasons: review.reasons.clone(),
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationConflictReviewHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl AggregationConflictReviewHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AggregationConflictReviewSummaryHistory {
    summaries: Vec<AggregationConflictReviewSummary>,
}

impl AggregationConflictReviewSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<AggregationConflictReviewSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: AggregationConflictReviewSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&AggregationConflictReviewSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[AggregationConflictReviewSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> AggregationConflictReviewDashboard {
        AggregationConflictReviewDashboard::from_summaries(&self.summaries)
    }

    pub fn health(
        &self,
        policy: AggregationConflictReviewHealthPolicy,
    ) -> AggregationConflictReviewHealth {
        self.dashboard().health(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationConflictReviewDashboard {
    pub total_records: usize,
    pub forwardable_records: usize,
    pub side_effect_safe_records: usize,
    pub repair_first_records: usize,
    pub repair_tasks: usize,
    pub unique_messages: usize,
    pub duplicate_messages: usize,
    pub unresolved_conflicts: usize,
    pub conflicted_messages: usize,
    pub reason_count: usize,
    pub forwardable_rate: f32,
    pub side_effect_safe_rate: f32,
    pub repair_first_rate: f32,
    pub telemetry: Vec<String>,
}

impl AggregationConflictReviewDashboard {
    pub fn from_summaries(summaries: &[AggregationConflictReviewSummary]) -> Self {
        let total_records = summaries.len();
        let forwardable_records = summaries
            .iter()
            .filter(|summary| summary.can_forward_messages && !summary.requires_repair_first)
            .count();
        let side_effect_safe_records = summaries
            .iter()
            .filter(|summary| summary.can_promote_side_effects && !summary.requires_repair_first)
            .count();
        let repair_first_records = summaries
            .iter()
            .filter(|summary| summary.requires_repair_first)
            .count();
        let repair_tasks = summaries
            .iter()
            .map(|summary| summary.repair_tasks)
            .sum::<usize>();
        let unique_messages = summaries
            .iter()
            .map(|summary| summary.unique_messages)
            .sum::<usize>();
        let duplicate_messages = summaries
            .iter()
            .map(|summary| summary.duplicate_messages)
            .sum::<usize>();
        let unresolved_conflicts = summaries
            .iter()
            .map(|summary| summary.unresolved_conflicts)
            .sum::<usize>();
        let conflicted_messages = summaries
            .iter()
            .map(|summary| summary.conflicted_messages)
            .sum::<usize>();
        let reason_count = summaries
            .iter()
            .map(|summary| summary.reasons.len())
            .sum::<usize>();
        let forwardable_rate = rate(forwardable_records, total_records);
        let side_effect_safe_rate = rate(side_effect_safe_records, total_records);
        let repair_first_rate = rate(repair_first_records, total_records);
        let telemetry = aggregation_conflict_review_dashboard_telemetry(
            total_records,
            forwardable_records,
            side_effect_safe_records,
            repair_first_records,
            repair_tasks,
            unique_messages,
            duplicate_messages,
            unresolved_conflicts,
            conflicted_messages,
            reason_count,
            forwardable_rate,
            side_effect_safe_rate,
            repair_first_rate,
        );

        Self {
            total_records,
            forwardable_records,
            side_effect_safe_records,
            repair_first_records,
            repair_tasks,
            unique_messages,
            duplicate_messages,
            unresolved_conflicts,
            conflicted_messages,
            reason_count,
            forwardable_rate,
            side_effect_safe_rate,
            repair_first_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(
        &self,
        policy: AggregationConflictReviewHealthPolicy,
    ) -> AggregationConflictReviewHealth {
        AggregationConflictReviewHealth::from_dashboard(self.clone(), policy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AggregationConflictReviewHealthPolicy {
    pub minimum_forwardable_rate: f32,
    pub minimum_side_effect_safe_rate: f32,
    pub maximum_repair_first_records: usize,
    pub maximum_repair_tasks: usize,
    pub maximum_unresolved_conflicts: usize,
    pub maximum_duplicate_messages: usize,
}

impl Default for AggregationConflictReviewHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_forwardable_rate: 0.67,
            minimum_side_effect_safe_rate: 0.67,
            maximum_repair_first_records: 0,
            maximum_repair_tasks: 0,
            maximum_unresolved_conflicts: 0,
            maximum_duplicate_messages: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationConflictReviewHealth {
    pub status: AggregationConflictReviewHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: AggregationConflictReviewDashboard,
}

impl AggregationConflictReviewHealth {
    pub fn from_dashboard(
        dashboard: AggregationConflictReviewDashboard,
        policy: AggregationConflictReviewHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("aggregation_conflict_review_history_empty".to_owned());
        } else {
            if dashboard.forwardable_rate < policy.minimum_forwardable_rate {
                watch_reasons.push(format!(
                    "aggregation_conflict_review_forwardable_rate={:.3}<{}",
                    dashboard.forwardable_rate, policy.minimum_forwardable_rate
                ));
            }
            if dashboard.side_effect_safe_rate < policy.minimum_side_effect_safe_rate {
                watch_reasons.push(format!(
                    "aggregation_conflict_review_side_effect_safe_rate={:.3}<{}",
                    dashboard.side_effect_safe_rate, policy.minimum_side_effect_safe_rate
                ));
            }
        }

        if dashboard.repair_first_records > policy.maximum_repair_first_records {
            repair_reasons.push(format!(
                "aggregation_conflict_review_repair_first_records={}>{}",
                dashboard.repair_first_records, policy.maximum_repair_first_records
            ));
        }
        if dashboard.repair_tasks > policy.maximum_repair_tasks {
            repair_reasons.push(format!(
                "aggregation_conflict_review_repair_tasks={}>{}",
                dashboard.repair_tasks, policy.maximum_repair_tasks
            ));
        }
        if dashboard.unresolved_conflicts > policy.maximum_unresolved_conflicts {
            repair_reasons.push(format!(
                "aggregation_conflict_review_unresolved_conflicts={}>{}",
                dashboard.unresolved_conflicts, policy.maximum_unresolved_conflicts
            ));
        }
        if dashboard.duplicate_messages > policy.maximum_duplicate_messages {
            repair_reasons.push(format!(
                "aggregation_conflict_review_duplicate_messages={}>{}",
                dashboard.duplicate_messages, policy.maximum_duplicate_messages
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (
                AggregationConflictReviewHealthStatus::Repair,
                repair_reasons,
            )
        } else if !watch_reasons.is_empty() {
            (AggregationConflictReviewHealthStatus::Watch, watch_reasons)
        } else {
            (AggregationConflictReviewHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == AggregationConflictReviewHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != AggregationConflictReviewHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == AggregationConflictReviewHealthStatus::Repair
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationConflictReviewSummaryHistoryRecord {
    pub history: AggregationConflictReviewSummaryHistory,
    pub appended_summary: AggregationConflictReviewSummary,
    pub dashboard: AggregationConflictReviewDashboard,
    pub health: AggregationConflictReviewHealth,
    pub telemetry: Vec<String>,
}

impl AggregationConflictReviewSummaryHistoryRecord {
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
pub struct AggregationConflictReviewSummaryHistoryRecorder;

impl AggregationConflictReviewSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: AggregationConflictReviewSummaryHistory,
        summary: AggregationConflictReviewSummary,
        policy: AggregationConflictReviewHealthPolicy,
    ) -> AggregationConflictReviewSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = aggregation_conflict_review_history_record_telemetry(&dashboard, &health);

        AggregationConflictReviewSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_review_with_health(
        &self,
        history: AggregationConflictReviewSummaryHistory,
        review: &AggregationConflictReview,
        policy: AggregationConflictReviewHealthPolicy,
    ) -> AggregationConflictReviewSummaryHistoryRecord {
        self.record_summary_with_health(history, review.summary(), policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregationConflictReviewTrendGateDecision {
    pub review_summary: AggregationConflictReviewSummary,
    pub review_health: AggregationConflictReviewHealth,
    pub can_forward_messages: bool,
    pub can_promote_side_effects: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AggregationConflictReviewTrendGateDecision {
    pub fn is_forwardable(&self) -> bool {
        self.can_forward_messages && !self.requires_repair_first
    }

    pub fn is_side_effect_safe(&self) -> bool {
        self.can_promote_side_effects && !self.requires_repair_first
    }

    pub fn repair_task_ids(&self) -> Vec<String> {
        self.repair_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgentPheromoneSignalKind {
    CodeReady,
    ReviewNeeded,
    RepairFirst,
    ToolAvailable,
    Blocked,
}

impl AgentPheromoneSignalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CodeReady => "code_ready",
            Self::ReviewNeeded => "review_needed",
            Self::RepairFirst => "repair_first",
            Self::ToolAvailable => "tool_available",
            Self::Blocked => "blocked",
        }
    }

    fn scheduler_action(self) -> &'static str {
        match self {
            Self::CodeReady => "promote_code_review",
            Self::ReviewNeeded => "request_review",
            Self::RepairFirst => "repair_review",
            Self::ToolAvailable => "inspect_tool_capability",
            Self::Blocked => "unblock_lane",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentPheromoneBlackboardRecord {
    pub signal_id: String,
    pub lane: String,
    pub organ: String,
    pub task_scope: String,
    pub signal_kind: AgentPheromoneSignalKind,
    pub concentration: f32,
    pub decay_ticks: u32,
    pub confidence: f32,
    pub source_digest: String,
    pub payload_digest: String,
    pub raw_payload_present: bool,
    pub side_effect_allowed: bool,
}

impl AgentPheromoneBlackboardRecord {
    pub fn digest_only(
        lane: impl Into<String>,
        organ: impl Into<String>,
        task_scope: impl Into<String>,
        signal_kind: AgentPheromoneSignalKind,
        concentration: f32,
        confidence: f32,
        source_digest: impl Into<String>,
        payload_digest: impl Into<String>,
    ) -> Self {
        let lane = lane.into();
        let organ = organ.into();
        let task_scope = task_scope.into();
        let source_digest = source_digest.into();
        let payload_digest = payload_digest.into();
        let signal_id = pheromone_digest([
            "signal",
            lane.as_str(),
            organ.as_str(),
            task_scope.as_str(),
            signal_kind.as_str(),
            source_digest.as_str(),
            payload_digest.as_str(),
        ]);

        Self {
            signal_id,
            lane,
            organ,
            task_scope,
            signal_kind,
            concentration: concentration.clamp(0.0, 1.0),
            decay_ticks: 0,
            confidence: confidence.clamp(0.0, 1.0),
            source_digest,
            payload_digest,
            raw_payload_present: false,
            side_effect_allowed: false,
        }
    }

    pub fn decayed(&self, ticks: u32) -> Self {
        let mut record = self.clone();
        for _ in 0..ticks {
            record.concentration *= 0.5;
        }
        record.decay_ticks = record.decay_ticks.saturating_add(ticks);
        record.signal_id = pheromone_digest([
            "signal",
            record.lane.as_str(),
            record.organ.as_str(),
            record.task_scope.as_str(),
            record.signal_kind.as_str(),
            record.source_digest.as_str(),
            record.payload_digest.as_str(),
            &format!("{:.3}", record.concentration),
            &record.decay_ticks.to_string(),
        ]);
        record
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentPheromoneNextAction {
    pub action_id: String,
    pub lane: String,
    pub organ: String,
    pub task_scope: String,
    pub signal_kind: AgentPheromoneSignalKind,
    pub action: String,
    pub concentration: f32,
    pub confidence: f32,
    pub source_digest: String,
    pub payload_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentPheromoneBlackboardPreview {
    pub records: Vec<AgentPheromoneBlackboardRecord>,
    pub ranked_next_actions: Vec<AgentPheromoneNextAction>,
    pub trigger_threshold: f32,
    pub blackboard_digest: String,
    pub raw_payload_present: bool,
    pub side_effect_allowed: bool,
}

impl AgentPheromoneBlackboardPreview {
    pub const DEFAULT_TRIGGER_THRESHOLD: f32 = 0.5;

    pub fn from_aggregation_conflict_and_task_summary(
        lane: impl Into<String>,
        organ: impl Into<String>,
        task_scope: impl Into<String>,
        trend_gate: &AggregationConflictReviewTrendGateDecision,
        task_summary: &TaskDispatchPlanSummary,
    ) -> Result<Self, AgentPheromoneBlackboardPreviewError> {
        let lane = lane.into();
        let organ = organ.into();
        let task_scope = task_scope.into();
        let review_summary = &trend_gate.review_summary;
        let source_digest = pheromone_digest([
            "aggregation-conflict-trend",
            lane.as_str(),
            organ.as_str(),
            task_scope.as_str(),
            review_summary.aggregation_health_status.as_str(),
            review_summary.conflict_health_status.as_str(),
            &trend_gate.can_forward_messages.to_string(),
            &trend_gate.requires_repair_first.to_string(),
            &task_summary.assignments.to_string(),
            &task_summary.rejections.to_string(),
        ]);
        let payload_digest = pheromone_digest([
            "aggregation-conflict-payload",
            &review_summary.unique_messages.to_string(),
            &review_summary.duplicate_messages.to_string(),
            &review_summary.unresolved_conflicts.to_string(),
            &review_summary.conflicted_messages.to_string(),
            &trend_gate.repair_tasks.len().to_string(),
            &task_summary.remaining_zero_budget_roles.to_string(),
        ]);
        let mut records = Vec::new();

        if trend_gate.requires_repair_first {
            records.push(AgentPheromoneBlackboardRecord::digest_only(
                lane.as_str(),
                organ.as_str(),
                task_scope.as_str(),
                AgentPheromoneSignalKind::RepairFirst,
                clamp01(
                    0.6 + (trend_gate.repair_tasks.len() as f32 * 0.04)
                        + (review_summary.unresolved_conflicts as f32 * 0.1),
                ),
                0.86,
                source_digest.as_str(),
                payload_digest.as_str(),
            ));
        }

        if review_summary.unresolved_conflicts > 0 || review_summary.conflicted_messages > 0 {
            records.push(AgentPheromoneBlackboardRecord::digest_only(
                lane.as_str(),
                organ.as_str(),
                task_scope.as_str(),
                AgentPheromoneSignalKind::ReviewNeeded,
                clamp01(
                    0.55 + (review_summary.unresolved_conflicts as f32 * 0.1)
                        + (review_summary.conflicted_messages as f32 * 0.04),
                ),
                0.78,
                source_digest.as_str(),
                payload_digest.as_str(),
            ));
        }

        if task_summary.rejections > 0 || task_summary.assignments == 0 {
            records.push(AgentPheromoneBlackboardRecord::digest_only(
                lane.as_str(),
                organ.as_str(),
                task_scope.as_str(),
                AgentPheromoneSignalKind::Blocked,
                clamp01(
                    0.5 + (task_summary.rejections as f32 * 0.1)
                        + if task_summary.assignments == 0 {
                            0.2
                        } else {
                            0.0
                        },
                ),
                0.72,
                source_digest.as_str(),
                payload_digest.as_str(),
            ));
        }

        if trend_gate.is_forwardable() && task_summary.assignments > 0 {
            records.push(AgentPheromoneBlackboardRecord::digest_only(
                lane.as_str(),
                organ.as_str(),
                task_scope.as_str(),
                AgentPheromoneSignalKind::CodeReady,
                clamp01(0.55 + task_summary.assigned_rate * 0.3),
                0.74,
                source_digest.as_str(),
                payload_digest.as_str(),
            ));
        }

        Self::try_from_records(records)
    }

    pub fn try_from_records(
        records: Vec<AgentPheromoneBlackboardRecord>,
    ) -> Result<Self, AgentPheromoneBlackboardPreviewError> {
        Self::try_from_records_with_threshold(records, Self::DEFAULT_TRIGGER_THRESHOLD)
    }

    pub fn try_from_records_with_threshold(
        records: Vec<AgentPheromoneBlackboardRecord>,
        trigger_threshold: f32,
    ) -> Result<Self, AgentPheromoneBlackboardPreviewError> {
        for record in &records {
            validate_pheromone_record(record)?;
        }
        let records = merge_pheromone_records(records);
        let ranked_next_actions = pheromone_next_actions(&records, trigger_threshold);
        let blackboard_digest = pheromone_blackboard_digest(&records);

        Ok(Self {
            records,
            ranked_next_actions,
            trigger_threshold,
            blackboard_digest,
            raw_payload_present: false,
            side_effect_allowed: false,
        })
    }

    pub fn ranked_next_actions(&self) -> Vec<AgentPheromoneNextAction> {
        pheromone_next_actions(&self.records, self.trigger_threshold)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentPheromoneBlackboardPreviewError {
    RawPayloadPresent { signal_id: String },
    SideEffectAllowed { signal_id: String },
    InvalidDigest { signal_id: String, field: String },
}

#[derive(Debug, Clone, Default)]
pub struct AggregationConflictReviewTrendGate;

impl AggregationConflictReviewTrendGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        review: &AggregationConflictReview,
        history_record: &AggregationConflictReviewSummaryHistoryRecord,
    ) -> AggregationConflictReviewTrendGateDecision {
        let review_summary = review.summary();
        let review_health = history_record.health.clone();
        let mut reasons = review.reasons.clone();
        extend_ordered_unique(
            &mut reasons,
            review_health
                .reasons
                .iter()
                .map(|reason| format!("aggregation_conflict_review_history:{reason}"))
                .collect::<Vec<_>>(),
        );
        let requires_repair_first =
            review.requires_repair_first || review_health.requires_repair_first();
        let can_forward_messages = review.can_forward_messages
            && review_health.allows_service_advance()
            && !requires_repair_first;
        let can_promote_side_effects = review.can_promote_side_effects
            && review_health.allows_service_advance()
            && !requires_repair_first;
        let mut repair_tasks = if review.requires_repair_first {
            review.repair_tasks.clone()
        } else {
            Vec::new()
        };
        repair_tasks.extend(aggregation_conflict_review_trend_gate_repair_tasks(
            review_health.requires_repair_first(),
            &reasons,
        ));
        let telemetry = aggregation_conflict_review_trend_gate_telemetry(
            review_health.status,
            can_forward_messages,
            can_promote_side_effects,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            &review_summary,
        );

        AggregationConflictReviewTrendGateDecision {
            review_summary,
            review_health,
            can_forward_messages,
            can_promote_side_effects,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AggregationConflictReviewer;

impl AggregationConflictReviewer {
    pub fn new() -> Self {
        Self
    }

    pub fn review_messages(
        &self,
        messages: Vec<AgentMessage>,
        aggregation_history: AggregationSummaryHistory,
        aggregation_policy: AggregationHealthPolicy,
        conflict_history: ConflictReportSummaryHistory,
        conflict_policy: ConflictReportHealthPolicy,
    ) -> AggregationConflictReview {
        let aggregation_report = MessageAggregator::new().aggregate(messages);
        self.review_report(
            aggregation_report,
            aggregation_history,
            aggregation_policy,
            conflict_history,
            conflict_policy,
        )
    }

    pub fn review_report(
        &self,
        aggregation_report: AggregationReport,
        aggregation_history: AggregationSummaryHistory,
        aggregation_policy: AggregationHealthPolicy,
        conflict_history: ConflictReportSummaryHistory,
        conflict_policy: ConflictReportHealthPolicy,
    ) -> AggregationConflictReview {
        let aggregation_record = AggregationSummaryHistoryRecorder::new()
            .record_report_with_health_gate(
                aggregation_history,
                &aggregation_report,
                aggregation_policy,
            );
        let aggregated_messages = aggregation_report
            .messages
            .iter()
            .map(|item| item.message.clone())
            .collect::<Vec<_>>();
        let conflict_report = ConflictResolver::new().mark_conflicts(&aggregated_messages);
        let conflict_record = ConflictReportSummaryHistoryRecorder::new()
            .record_report_with_health_gate(conflict_history, &conflict_report, conflict_policy);

        let requires_repair_first =
            aggregation_record.requires_repair_first() || conflict_record.requires_repair_first();
        let can_forward_messages = aggregation_record.can_forward_aggregated_messages()
            && conflict_record.gate_decision.can_forward_report
            && !requires_repair_first;
        let can_promote_side_effects = aggregation_record.can_forward_aggregated_messages()
            && conflict_record.can_promote_side_effects()
            && !requires_repair_first;
        let mut repair_tasks = aggregation_record.gate_decision.repair_tasks.clone();
        repair_tasks.extend(conflict_record.gate_decision.repair_tasks.clone());
        let mut reasons = aggregation_record.gate_decision.reasons.clone();
        extend_ordered_unique(
            &mut reasons,
            conflict_record
                .gate_decision
                .reasons
                .iter()
                .map(|reason| format!("conflict_report:{reason}"))
                .collect::<Vec<_>>(),
        );
        let telemetry = aggregation_conflict_review_telemetry(
            can_forward_messages,
            can_promote_side_effects,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            aggregation_record.gate_decision.report_summary.unique_count,
            conflict_record
                .gate_decision
                .report_summary
                .unresolved_conflicts,
        );

        AggregationConflictReview {
            aggregation_record,
            conflict_record,
            conflict_report,
            can_forward_messages,
            can_promote_side_effects,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AggregationHistoryGate;

impl AggregationHistoryGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        report: &AggregationReport,
        history_record: &AggregationSummaryHistoryRecord,
    ) -> AggregationHistoryGateDecision {
        let report_summary = report.summary();
        let aggregation_health = history_record.health.clone();
        let mut reasons = aggregation_gate_reasons(&report_summary);
        extend_ordered_unique(
            &mut reasons,
            aggregation_health
                .reasons
                .iter()
                .map(|reason| format!("aggregation_history:{reason}"))
                .collect::<Vec<_>>(),
        );
        let current_requires_repair =
            report_summary.duplicate_messages > 0 || report_summary.duplicate_groups > 0;
        let requires_repair_first =
            current_requires_repair || aggregation_health.requires_repair_first();
        let can_forward_aggregated_messages = report_summary.unique_count > 0
            && aggregation_health.allows_service_advance()
            && !requires_repair_first;
        let repair_tasks = aggregation_history_gate_repair_tasks(requires_repair_first, &reasons);
        let telemetry = aggregation_history_gate_telemetry(
            can_forward_aggregated_messages,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            &report_summary,
            aggregation_health.status,
        );

        AggregationHistoryGateDecision {
            report_summary,
            aggregation_health,
            can_forward_aggregated_messages,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MessageAggregator;

impl MessageAggregator {
    pub fn new() -> Self {
        Self
    }

    pub fn aggregate(&self, messages: Vec<AgentMessage>) -> AggregationReport {
        let input_count = messages.len();
        let mut by_fingerprint: BTreeMap<String, AggregatedMessage> = BTreeMap::new();

        for message in messages {
            let fingerprint = message.fingerprint();
            by_fingerprint
                .entry(fingerprint)
                .and_modify(|existing| {
                    existing.duplicate_count += 1;
                    existing.source_ids.push(message.id.clone());
                    existing.message.confidence =
                        existing.message.confidence.max(message.confidence);
                    merge_evidence(&mut existing.message.evidence, &message.evidence);
                })
                .or_insert_with(|| AggregatedMessage {
                    source_ids: vec![message.id.clone()],
                    message,
                    duplicate_count: 1,
                });
        }

        let mut messages = by_fingerprint.into_values().collect::<Vec<_>>();
        for message in &mut messages {
            message.source_ids.sort();
            message.message.evidence.sort();
            if let Some(source_id) = message.source_ids.first() {
                message.message.id.clone_from(source_id);
            }
        }
        let duplicate_groups = messages
            .iter()
            .filter(|message| message.duplicate_count > 1)
            .count();

        AggregationReport {
            input_count,
            unique_count: messages.len(),
            duplicate_groups,
            messages,
        }
    }
}

fn merge_evidence(target: &mut Vec<String>, incoming: &[String]) {
    for item in incoming {
        if !target.iter().any(|existing| existing == item) {
            target.push(item.clone());
        }
    }
}

fn aggregation_summary_telemetry(
    input_count: usize,
    unique_count: usize,
    duplicate_groups: usize,
    duplicate_messages: usize,
    compression_rate: f32,
) -> Vec<String> {
    vec![
        "agent_aggregation_summary=true".to_owned(),
        format!("agent_aggregation_summary_input_count={input_count}"),
        format!("agent_aggregation_summary_unique_count={unique_count}"),
        format!("agent_aggregation_summary_duplicate_groups={duplicate_groups}"),
        format!("agent_aggregation_summary_duplicate_messages={duplicate_messages}"),
        format!("agent_aggregation_summary_compression_rate={compression_rate:.3}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn aggregation_dashboard_telemetry(
    total_records: usize,
    input_count: usize,
    unique_count: usize,
    duplicate_groups: usize,
    duplicate_messages: usize,
    duplicate_records: usize,
    empty_records: usize,
    aggregate_compression_rate: f32,
    duplicate_record_rate: f32,
) -> Vec<String> {
    vec![
        "agent_aggregation_dashboard=true".to_owned(),
        format!("agent_aggregation_dashboard_records={total_records}"),
        format!("agent_aggregation_dashboard_input_count={input_count}"),
        format!("agent_aggregation_dashboard_unique_count={unique_count}"),
        format!("agent_aggregation_dashboard_duplicate_groups={duplicate_groups}"),
        format!("agent_aggregation_dashboard_duplicate_messages={duplicate_messages}"),
        format!("agent_aggregation_dashboard_duplicate_records={duplicate_records}"),
        format!("agent_aggregation_dashboard_empty_records={empty_records}"),
        format!("agent_aggregation_dashboard_compression_rate={aggregate_compression_rate:.3}"),
        format!("agent_aggregation_dashboard_duplicate_record_rate={duplicate_record_rate:.3}"),
    ]
}

fn aggregation_history_record_telemetry(
    dashboard: &AggregationDashboard,
    health: &AggregationHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_aggregation_history_record=true".to_owned(),
        format!(
            "agent_aggregation_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_aggregation_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_aggregation_history_record_compression_rate={:.3}",
            dashboard.aggregate_compression_rate
        ),
        format!(
            "agent_aggregation_history_record_duplicate_messages={}",
            dashboard.duplicate_messages
        ),
        format!(
            "agent_aggregation_history_record_duplicate_groups={}",
            dashboard.duplicate_groups
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_aggregation_history_record_reason={reason}")),
    );
    telemetry
}

fn aggregation_gate_reasons(summary: &AggregationSummary) -> Vec<String> {
    let mut reasons = Vec::new();
    if summary.duplicate_messages > 0 {
        reasons.push(format!(
            "aggregation_duplicate_messages={}",
            summary.duplicate_messages
        ));
    }
    if summary.duplicate_groups > 0 {
        reasons.push(format!(
            "aggregation_duplicate_groups={}",
            summary.duplicate_groups
        ));
    }
    reasons
}

fn aggregation_history_gate_repair_tasks(
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
                format!("aggregation-repair-{index}"),
                AgentRole::Aggregator,
                format!("repair aggregation: {reason}"),
                AgentBudget::new(12, 1, 1),
            )
            .with_lane("aggregation-repair")
            .with_priority(1)
        })
        .collect()
}

fn aggregation_history_gate_telemetry(
    can_forward_aggregated_messages: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    summary: &AggregationSummary,
    health_status: AggregationHealthStatus,
) -> Vec<String> {
    vec![
        "agent_aggregation_history_gate=true".to_owned(),
        format!(
            "agent_aggregation_history_gate_health={}",
            health_status.as_str()
        ),
        format!("agent_aggregation_history_gate_forward={can_forward_aggregated_messages}"),
        format!("agent_aggregation_history_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_aggregation_history_gate_repair_tasks={repair_tasks}"),
        format!("agent_aggregation_history_gate_reasons={reasons}"),
        format!(
            "agent_aggregation_history_gate_unique_count={}",
            summary.unique_count
        ),
        format!(
            "agent_aggregation_history_gate_duplicate_messages={}",
            summary.duplicate_messages
        ),
        format!(
            "agent_aggregation_history_gate_duplicate_groups={}",
            summary.duplicate_groups
        ),
    ]
}

fn aggregation_history_gate_record_telemetry(
    health_record: &AggregationSummaryHistoryRecord,
    gate_decision: &AggregationHistoryGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_aggregation_history_gate_record=true".to_owned(),
        format!(
            "agent_aggregation_history_gate_record_health={}",
            health_record.health.status.as_str()
        ),
        format!(
            "agent_aggregation_history_gate_record_records={}",
            health_record.records()
        ),
        format!(
            "agent_aggregation_history_gate_record_forward={}",
            gate_decision.can_forward_aggregated_messages
        ),
        format!(
            "agent_aggregation_history_gate_record_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_aggregation_history_gate_record_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
    ];
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn aggregation_conflict_review_telemetry(
    can_forward_messages: bool,
    can_promote_side_effects: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    unique_messages: usize,
    unresolved_conflicts: usize,
) -> Vec<String> {
    vec![
        "agent_aggregation_conflict_review=true".to_owned(),
        format!("agent_aggregation_conflict_review_forward_messages={can_forward_messages}"),
        format!(
            "agent_aggregation_conflict_review_promote_side_effects={can_promote_side_effects}"
        ),
        format!("agent_aggregation_conflict_review_requires_repair_first={requires_repair_first}"),
        format!("agent_aggregation_conflict_review_repair_tasks={repair_tasks}"),
        format!("agent_aggregation_conflict_review_reasons={reasons}"),
        format!("agent_aggregation_conflict_review_unique_messages={unique_messages}"),
        format!("agent_aggregation_conflict_review_unresolved_conflicts={unresolved_conflicts}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn aggregation_conflict_review_summary_telemetry(
    aggregation_health_status: AggregationHealthStatus,
    conflict_health_status: ConflictReportHealthStatus,
    can_forward_messages: bool,
    can_promote_side_effects: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    unique_messages: usize,
    duplicate_messages: usize,
    unresolved_conflicts: usize,
    conflicted_messages: usize,
    reasons: usize,
) -> Vec<String> {
    vec![
        "agent_aggregation_conflict_review_summary=true".to_owned(),
        format!(
            "agent_aggregation_conflict_review_summary_aggregation_health={}",
            aggregation_health_status.as_str()
        ),
        format!(
            "agent_aggregation_conflict_review_summary_conflict_health={}",
            conflict_health_status.as_str()
        ),
        format!("agent_aggregation_conflict_review_summary_forward={can_forward_messages}"),
        format!(
            "agent_aggregation_conflict_review_summary_promote_side_effects={can_promote_side_effects}"
        ),
        format!(
            "agent_aggregation_conflict_review_summary_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_aggregation_conflict_review_summary_repair_tasks={repair_tasks}"),
        format!("agent_aggregation_conflict_review_summary_unique_messages={unique_messages}"),
        format!(
            "agent_aggregation_conflict_review_summary_duplicate_messages={duplicate_messages}"
        ),
        format!(
            "agent_aggregation_conflict_review_summary_unresolved_conflicts={unresolved_conflicts}"
        ),
        format!(
            "agent_aggregation_conflict_review_summary_conflicted_messages={conflicted_messages}"
        ),
        format!("agent_aggregation_conflict_review_summary_reasons={reasons}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn aggregation_conflict_review_dashboard_telemetry(
    total_records: usize,
    forwardable_records: usize,
    side_effect_safe_records: usize,
    repair_first_records: usize,
    repair_tasks: usize,
    unique_messages: usize,
    duplicate_messages: usize,
    unresolved_conflicts: usize,
    conflicted_messages: usize,
    reason_count: usize,
    forwardable_rate: f32,
    side_effect_safe_rate: f32,
    repair_first_rate: f32,
) -> Vec<String> {
    vec![
        "agent_aggregation_conflict_review_dashboard=true".to_owned(),
        format!("agent_aggregation_conflict_review_dashboard_records={total_records}"),
        format!(
            "agent_aggregation_conflict_review_dashboard_forwardable_records={forwardable_records}"
        ),
        format!(
            "agent_aggregation_conflict_review_dashboard_side_effect_safe_records={side_effect_safe_records}"
        ),
        format!(
            "agent_aggregation_conflict_review_dashboard_repair_first_records={repair_first_records}"
        ),
        format!("agent_aggregation_conflict_review_dashboard_repair_tasks={repair_tasks}"),
        format!("agent_aggregation_conflict_review_dashboard_unique_messages={unique_messages}"),
        format!(
            "agent_aggregation_conflict_review_dashboard_duplicate_messages={duplicate_messages}"
        ),
        format!(
            "agent_aggregation_conflict_review_dashboard_unresolved_conflicts={unresolved_conflicts}"
        ),
        format!(
            "agent_aggregation_conflict_review_dashboard_conflicted_messages={conflicted_messages}"
        ),
        format!("agent_aggregation_conflict_review_dashboard_reasons={reason_count}"),
        format!(
            "agent_aggregation_conflict_review_dashboard_forwardable_rate={forwardable_rate:.3}"
        ),
        format!(
            "agent_aggregation_conflict_review_dashboard_side_effect_safe_rate={side_effect_safe_rate:.3}"
        ),
        format!(
            "agent_aggregation_conflict_review_dashboard_repair_first_rate={repair_first_rate:.3}"
        ),
    ]
}

fn aggregation_conflict_review_history_record_telemetry(
    dashboard: &AggregationConflictReviewDashboard,
    health: &AggregationConflictReviewHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_aggregation_conflict_review_history_record=true".to_owned(),
        format!(
            "agent_aggregation_conflict_review_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_aggregation_conflict_review_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_aggregation_conflict_review_history_record_forwardable_records={}",
            dashboard.forwardable_records
        ),
        format!(
            "agent_aggregation_conflict_review_history_record_side_effect_safe_records={}",
            dashboard.side_effect_safe_records
        ),
        format!(
            "agent_aggregation_conflict_review_history_record_repair_first_records={}",
            dashboard.repair_first_records
        ),
        format!(
            "agent_aggregation_conflict_review_history_record_repair_tasks={}",
            dashboard.repair_tasks
        ),
        format!(
            "agent_aggregation_conflict_review_history_record_unresolved_conflicts={}",
            dashboard.unresolved_conflicts
        ),
        format!(
            "agent_aggregation_conflict_review_history_record_duplicate_messages={}",
            dashboard.duplicate_messages
        ),
    ];
    telemetry.extend(
        health.reasons.iter().map(|reason| {
            format!("agent_aggregation_conflict_review_history_record_reason={reason}")
        }),
    );
    telemetry
}

fn aggregation_conflict_review_trend_gate_repair_tasks(
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
                format!("aggregation-conflict-review-trend-repair-{index}"),
                AgentRole::Reviewer,
                format!("repair aggregation conflict review trend: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("aggregation-conflict-review-trend-repair")
            .with_priority(1)
        })
        .collect()
}

fn aggregation_conflict_review_trend_gate_telemetry(
    health_status: AggregationConflictReviewHealthStatus,
    can_forward_messages: bool,
    can_promote_side_effects: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    summary: &AggregationConflictReviewSummary,
) -> Vec<String> {
    vec![
        "agent_aggregation_conflict_review_trend_gate=true".to_owned(),
        format!(
            "agent_aggregation_conflict_review_trend_gate_health={}",
            health_status.as_str()
        ),
        format!("agent_aggregation_conflict_review_trend_gate_forward={can_forward_messages}"),
        format!(
            "agent_aggregation_conflict_review_trend_gate_promote_side_effects={can_promote_side_effects}"
        ),
        format!(
            "agent_aggregation_conflict_review_trend_gate_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_aggregation_conflict_review_trend_gate_repair_tasks={repair_tasks}"),
        format!("agent_aggregation_conflict_review_trend_gate_reasons={reasons}"),
        format!(
            "agent_aggregation_conflict_review_trend_gate_unresolved_conflicts={}",
            summary.unresolved_conflicts
        ),
        format!(
            "agent_aggregation_conflict_review_trend_gate_duplicate_messages={}",
            summary.duplicate_messages
        ),
    ]
}

fn validate_pheromone_record(
    record: &AgentPheromoneBlackboardRecord,
) -> Result<(), AgentPheromoneBlackboardPreviewError> {
    if record.raw_payload_present {
        return Err(AgentPheromoneBlackboardPreviewError::RawPayloadPresent {
            signal_id: record.signal_id.clone(),
        });
    }
    if record.side_effect_allowed {
        return Err(AgentPheromoneBlackboardPreviewError::SideEffectAllowed {
            signal_id: record.signal_id.clone(),
        });
    }
    for (field, value) in [
        ("signal_id", record.signal_id.as_str()),
        ("source_digest", record.source_digest.as_str()),
        ("payload_digest", record.payload_digest.as_str()),
    ] {
        if !value.starts_with("redaction-digest:") {
            return Err(AgentPheromoneBlackboardPreviewError::InvalidDigest {
                signal_id: record.signal_id.clone(),
                field: field.to_owned(),
            });
        }
    }
    Ok(())
}

fn merge_pheromone_records(
    records: Vec<AgentPheromoneBlackboardRecord>,
) -> Vec<AgentPheromoneBlackboardRecord> {
    let mut by_scope: BTreeMap<
        (String, String, String, AgentPheromoneSignalKind),
        AgentPheromoneBlackboardRecord,
    > = BTreeMap::new();

    for record in records {
        let key = (
            record.lane.clone(),
            record.organ.clone(),
            record.task_scope.clone(),
            record.signal_kind,
        );
        by_scope
            .entry(key)
            .and_modify(|existing| {
                existing.concentration = clamp01(existing.concentration + record.concentration);
                existing.confidence = existing.confidence.max(record.confidence);
                existing.decay_ticks = existing.decay_ticks.min(record.decay_ticks);
                let mut source_digests =
                    [existing.source_digest.clone(), record.source_digest.clone()];
                source_digests.sort();
                let mut payload_digests = [
                    existing.payload_digest.clone(),
                    record.payload_digest.clone(),
                ];
                payload_digests.sort();
                existing.source_digest = pheromone_digest([
                    "merged-source",
                    source_digests[0].as_str(),
                    source_digests[1].as_str(),
                ]);
                existing.payload_digest = pheromone_digest([
                    "merged-payload",
                    payload_digests[0].as_str(),
                    payload_digests[1].as_str(),
                ]);
                existing.signal_id = pheromone_digest([
                    "merged-signal",
                    existing.lane.as_str(),
                    existing.organ.as_str(),
                    existing.task_scope.as_str(),
                    existing.signal_kind.as_str(),
                    existing.source_digest.as_str(),
                    existing.payload_digest.as_str(),
                    &format!("{:.3}", existing.concentration),
                ]);
            })
            .or_insert(record);
    }

    by_scope.into_values().collect()
}

fn pheromone_next_actions(
    records: &[AgentPheromoneBlackboardRecord],
    trigger_threshold: f32,
) -> Vec<AgentPheromoneNextAction> {
    let mut actions = records
        .iter()
        .filter(|record| record.concentration >= trigger_threshold)
        .map(|record| {
            let action = record.signal_kind.scheduler_action();
            AgentPheromoneNextAction {
                action_id: pheromone_digest([
                    "next-action",
                    record.signal_id.as_str(),
                    action,
                    record.lane.as_str(),
                    record.task_scope.as_str(),
                ]),
                lane: record.lane.clone(),
                organ: record.organ.clone(),
                task_scope: record.task_scope.clone(),
                signal_kind: record.signal_kind,
                action: action.to_owned(),
                concentration: record.concentration,
                confidence: record.confidence,
                source_digest: record.source_digest.clone(),
                payload_digest: record.payload_digest.clone(),
            }
        })
        .collect::<Vec<_>>();

    actions.sort_by(|left, right| {
        right
            .concentration
            .partial_cmp(&left.concentration)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .confidence
                    .partial_cmp(&left.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.signal_kind.as_str().cmp(right.signal_kind.as_str()))
            .then_with(|| left.lane.cmp(&right.lane))
            .then_with(|| left.task_scope.cmp(&right.task_scope))
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    actions
}

fn pheromone_blackboard_digest(records: &[AgentPheromoneBlackboardRecord]) -> String {
    let mut parts = vec!["pheromone-blackboard".to_owned()];
    for record in records {
        parts.extend([
            record.signal_id.clone(),
            record.lane.clone(),
            record.organ.clone(),
            record.task_scope.clone(),
            record.signal_kind.as_str().to_owned(),
            format!("{:.3}", record.concentration),
            record.decay_ticks.to_string(),
            record.source_digest.clone(),
            record.payload_digest.clone(),
        ]);
    }
    pheromone_digest(parts.iter().map(String::as_str))
}

fn pheromone_digest<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for part in parts {
        for byte in part.bytes().chain([0xff]) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("redaction-digest:{hash:016x}")
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
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
    use crate::message::AgentMessageKind;
    use crate::task::AgentRole;

    #[test]
    fn duplicate_messages_merge_into_single_aggregate() {
        let messages = vec![
            AgentMessage::new(
                "m1",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "memory",
                "Use the isolated norion-agent crate",
            )
            .with_confidence(0.7)
            .with_evidence("src-agent-team"),
            AgentMessage::new(
                "m2",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "memory",
                "  use   the isolated norion-agent crate ",
            )
            .with_confidence(0.9)
            .with_evidence("docs-architecture"),
        ];

        let report = MessageAggregator::new().aggregate(messages);

        assert_eq!(report.input_count, 2);
        assert_eq!(report.unique_count, 1);
        assert_eq!(report.duplicate_groups, 1);
        assert_eq!(report.messages[0].duplicate_count, 2);
        assert_eq!(report.messages[0].source_ids, vec!["m1", "m2"]);
        assert_eq!(report.messages[0].message.confidence, 0.9);
        assert_eq!(
            report.messages[0].message.evidence,
            vec!["docs-architecture", "src-agent-team"]
        );

        let summary = report.summary();

        assert_eq!(summary.input_count, 2);
        assert_eq!(summary.unique_count, 1);
        assert_eq!(summary.duplicate_messages, 1);
        assert_eq!(summary.compression_rate, 0.5);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_aggregation_summary_compression_rate=0.500" })
        );
    }

    #[test]
    fn aggregation_outputs_stable_fingerprint_order_independent_of_input_order() {
        let messages = vec![
            AgentMessage::new(
                "reviewer",
                AgentRole::Reviewer,
                AgentMessageKind::Finding,
                "workflow",
                "review passed",
            ),
            AgentMessage::new(
                "planner",
                AgentRole::Planner,
                AgentMessageKind::Finding,
                "workflow",
                "plan accepted",
            ),
            AgentMessage::new(
                "coder",
                AgentRole::Coder,
                AgentMessageKind::Finding,
                "workflow",
                "patch ready",
            ),
        ];

        let report = MessageAggregator::new().aggregate(messages);

        assert_eq!(report.input_count, 3);
        assert_eq!(report.unique_count, 3);
        assert_eq!(report.duplicate_groups, 0);
        assert_eq!(
            report
                .messages
                .iter()
                .map(|item| item.message.id.as_str())
                .collect::<Vec<_>>(),
            vec!["coder", "planner", "reviewer"]
        );
        assert_eq!(
            report
                .messages
                .iter()
                .map(|item| item.source_ids.clone())
                .collect::<Vec<_>>(),
            vec![
                vec!["coder".to_owned()],
                vec!["planner".to_owned()],
                vec!["reviewer".to_owned()],
            ]
        );
    }

    #[test]
    fn duplicate_messages_merge_with_stable_sources_independent_of_input_order() {
        let early = AgentMessage::new(
            "m1",
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "memory",
            "Use the isolated norion-agent crate",
        )
        .with_confidence(0.7)
        .with_evidence("src-agent-team");
        let late = AgentMessage::new(
            "m2",
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "memory",
            "  use   the isolated norion-agent crate ",
        )
        .with_confidence(0.9)
        .with_evidence("docs-architecture");

        let forward = MessageAggregator::new().aggregate(vec![early.clone(), late.clone()]);
        let reversed = MessageAggregator::new().aggregate(vec![late, early]);

        assert_eq!(forward.messages.len(), 1);
        assert_eq!(reversed.messages.len(), 1);
        assert_eq!(
            forward.messages[0].source_ids,
            reversed.messages[0].source_ids
        );
        assert_eq!(
            forward.messages[0].message.id,
            reversed.messages[0].message.id
        );
        assert_eq!(
            forward.messages[0].message.evidence,
            reversed.messages[0].message.evidence
        );
        assert_eq!(reversed.messages[0].source_ids, vec!["m1", "m2"]);
        assert_eq!(reversed.messages[0].message.id, "m1");
        assert_eq!(
            reversed.messages[0].message.evidence,
            vec!["docs-architecture", "src-agent-team"]
        );
        assert_eq!(reversed.messages[0].message.confidence, 0.9);
    }

    #[test]
    fn aggregation_history_watches_empty() {
        let health = AggregationSummaryHistory::new().health(AggregationHealthPolicy::default());

        assert_eq!(health.status, AggregationHealthStatus::Watch);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(health.reasons, vec!["aggregation_history_empty".to_owned()]);
        assert_eq!(health.dashboard.total_records, 0);
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_aggregation_dashboard_records=0" })
        );
    }

    #[test]
    fn aggregation_history_marks_unique_messages_stable() {
        let messages = vec![
            AgentMessage::new(
                "m1",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "memory",
                "Use the isolated norion-agent crate",
            ),
            AgentMessage::new(
                "m2",
                AgentRole::Reviewer,
                AgentMessageKind::Finding,
                "budget",
                "Use isolated budget ledgers",
            ),
        ];
        let report = MessageAggregator::new().aggregate(messages);

        let record = AggregationSummaryHistoryRecorder::new().record_report_with_health(
            AggregationSummaryHistory::new(),
            &report,
            AggregationHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert_eq!(record.dashboard.input_count, 2);
        assert_eq!(record.dashboard.unique_count, 2);
        assert_eq!(record.dashboard.duplicate_records, 0);
        assert_eq!(record.dashboard.duplicate_messages, 0);
        assert_eq!(record.dashboard.aggregate_compression_rate, 1.0);
        assert_eq!(record.health.status, AggregationHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_aggregation_history_record_status=stable" })
        );
    }

    #[test]
    fn aggregation_history_repairs_duplicate_pressure() {
        let clean = AggregationSummary {
            input_count: 1,
            unique_count: 1,
            duplicate_groups: 0,
            duplicate_messages: 0,
            compression_rate: 1.0,
            telemetry: Vec::new(),
        };
        let dirty = AggregationSummary {
            input_count: 3,
            unique_count: 1,
            duplicate_groups: 1,
            duplicate_messages: 2,
            compression_rate: 1.0 / 3.0,
            telemetry: Vec::new(),
        };
        let history = AggregationSummaryHistory::from_summaries(vec![clean]);

        let record = AggregationSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            dirty,
            AggregationHealthPolicy::default(),
        );

        assert_eq!(record.records(), 2);
        assert_eq!(record.dashboard.input_count, 4);
        assert_eq!(record.dashboard.unique_count, 2);
        assert_eq!(record.dashboard.duplicate_records, 1);
        assert_eq!(record.dashboard.duplicate_messages, 2);
        assert_eq!(record.dashboard.duplicate_groups, 1);
        assert_eq!(record.dashboard.aggregate_compression_rate, 0.5);
        assert_eq!(record.health.status, AggregationHealthStatus::Repair);
        assert!(!record.health.is_stable());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "aggregation_duplicate_records=1>0",
                "aggregation_duplicate_messages=2>0",
                "aggregation_duplicate_groups=1>0",
                "aggregation_compression_rate=0.500<0.67",
            ]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_aggregation_history_record_status=repair" })
        );
    }

    #[test]
    fn aggregation_history_gate_forwards_stable_unique_messages() {
        let messages = vec![
            AgentMessage::new(
                "m1",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "memory",
                "Use the isolated norion-agent crate",
            ),
            AgentMessage::new(
                "m2",
                AgentRole::Reviewer,
                AgentMessageKind::Finding,
                "budget",
                "Use isolated budget ledgers",
            ),
        ];
        let report = MessageAggregator::new().aggregate(messages);
        let history_record = AggregationSummaryHistoryRecorder::new().record_report_with_health(
            AggregationSummaryHistory::new(),
            &report,
            AggregationHealthPolicy::default(),
        );

        let gate = AggregationHistoryGate::new().gate(&report, &history_record);

        assert!(gate.can_forward_aggregated_messages);
        assert!(gate.is_forwardable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert!(gate.reasons.is_empty());
        assert_eq!(
            gate.aggregation_health.status,
            AggregationHealthStatus::Stable
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_aggregation_history_gate_forward=true" })
        );
    }

    #[test]
    fn aggregation_history_gate_repairs_current_duplicates() {
        let messages = vec![
            AgentMessage::new(
                "m1",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "memory",
                "Use the isolated norion-agent crate",
            ),
            AgentMessage::new(
                "m2",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "memory",
                "use the isolated norion-agent crate",
            ),
        ];
        let report = MessageAggregator::new().aggregate(messages);
        let history_record = AggregationSummaryHistoryRecorder::new().record_report_with_health(
            AggregationSummaryHistory::new(),
            &report,
            AggregationHealthPolicy::default(),
        );

        let gate = AggregationHistoryGate::new().gate(&report, &history_record);

        assert!(!gate.can_forward_aggregated_messages);
        assert!(!gate.is_forwardable());
        assert!(gate.requires_repair_first);
        assert_eq!(gate.repair_tasks.len(), gate.reasons.len());
        assert_eq!(
            gate.reasons,
            vec![
                "aggregation_duplicate_messages=1",
                "aggregation_duplicate_groups=1",
                "aggregation_history:aggregation_duplicate_records=1>0",
                "aggregation_history:aggregation_duplicate_messages=1>0",
                "aggregation_history:aggregation_duplicate_groups=1>0",
                "aggregation_history:aggregation_compression_rate=0.500<0.67",
            ]
        );
        assert!(
            gate.telemetry.iter().any(|line| {
                line == "agent_aggregation_history_gate_requires_repair_first=true"
            })
        );
    }

    #[test]
    fn aggregation_history_gate_repairs_dirty_history_before_forwarding() {
        let dirty = AggregationSummary {
            input_count: 3,
            unique_count: 1,
            duplicate_groups: 1,
            duplicate_messages: 2,
            compression_rate: 1.0 / 3.0,
            telemetry: Vec::new(),
        };
        let report = AggregationReport {
            input_count: 1,
            unique_count: 1,
            duplicate_groups: 0,
            messages: vec![AggregatedMessage {
                message: AgentMessage::new(
                    "m1",
                    AgentRole::Researcher,
                    AgentMessageKind::Finding,
                    "memory",
                    "Use the isolated norion-agent crate",
                ),
                duplicate_count: 1,
                source_ids: vec!["m1".to_owned()],
            }],
        };
        let history_record = AggregationSummaryHistoryRecorder::new().record_report_with_health(
            AggregationSummaryHistory::from_summaries(vec![dirty]),
            &report,
            AggregationHealthPolicy::default(),
        );

        let gate = AggregationHistoryGate::new().gate(&report, &history_record);

        assert!(!gate.can_forward_aggregated_messages);
        assert!(!gate.is_forwardable());
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.aggregation_health.status,
            AggregationHealthStatus::Repair
        );
        assert_eq!(gate.repair_tasks.len(), 4);
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "aggregation-repair-0",
                "aggregation-repair-1",
                "aggregation-repair-2",
                "aggregation-repair-3",
            ]
        );
    }

    #[test]
    fn aggregation_history_recorder_records_and_gates_report() {
        let messages = vec![AgentMessage::new(
            "m1",
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "memory",
            "Use the isolated norion-agent crate",
        )];
        let report = MessageAggregator::new().aggregate(messages);

        let record = AggregationSummaryHistoryRecorder::new().record_report_with_health_gate(
            AggregationSummaryHistory::new(),
            &report,
            AggregationHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.can_forward_aggregated_messages());
        assert!(record.gate_decision.is_forwardable());
        assert_eq!(
            record.health_record.health.status,
            AggregationHealthStatus::Stable
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_aggregation_history_gate_record_forward=true" })
        );
    }

    #[test]
    fn aggregation_conflict_review_forwards_clean_aggregated_messages() {
        let messages = vec![
            AgentMessage::new(
                "m1",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "memory",
                "Use the isolated norion-agent crate",
            ),
            AgentMessage::new(
                "m2",
                AgentRole::Reviewer,
                AgentMessageKind::Finding,
                "budget",
                "Use isolated budget ledgers",
            ),
        ];

        let review = AggregationConflictReviewer::new().review_messages(
            messages,
            AggregationSummaryHistory::new(),
            AggregationHealthPolicy::default(),
            ConflictReportSummaryHistory::new(),
            ConflictReportHealthPolicy::default(),
        );

        assert!(review.can_forward_messages);
        assert!(review.can_promote_side_effects);
        assert!(!review.requires_repair_first);
        assert!(review.is_forwardable());
        assert!(review.is_side_effect_safe());
        assert!(review.repair_tasks.is_empty());
        assert_eq!(
            review
                .aggregation_record
                .gate_decision
                .report_summary
                .unique_count,
            2
        );
        assert_eq!(
            review
                .conflict_record
                .gate_decision
                .report_summary
                .unresolved_conflicts,
            0
        );
        let summary = review.summary();
        assert_eq!(
            summary.aggregation_health_status,
            AggregationHealthStatus::Stable
        );
        assert_eq!(
            summary.conflict_health_status,
            ConflictReportHealthStatus::Stable
        );
        assert!(summary.can_forward_messages);
        assert!(summary.can_promote_side_effects);
        assert!(!summary.requires_repair_first);
        assert_eq!(summary.repair_tasks, 0);
        assert_eq!(summary.unique_messages, 2);
        let record = AggregationConflictReviewSummaryHistoryRecorder::new()
            .record_review_with_health(
                AggregationConflictReviewSummaryHistory::new(),
                &review,
                AggregationConflictReviewHealthPolicy::default(),
            );
        assert_eq!(record.records(), 1);
        assert_eq!(
            record.health.status,
            AggregationConflictReviewHealthStatus::Stable
        );
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert_eq!(record.dashboard.forwardable_records, 1);
        assert_eq!(record.dashboard.side_effect_safe_records, 1);
        let trend_gate = AggregationConflictReviewTrendGate::new().gate(&review, &record);
        assert!(trend_gate.can_forward_messages);
        assert!(trend_gate.can_promote_side_effects);
        assert!(!trend_gate.requires_repair_first);
        assert!(trend_gate.is_forwardable());
        assert!(trend_gate.is_side_effect_safe());
        assert!(trend_gate.repair_tasks.is_empty());
        let dirty_summary = AggregationConflictReviewSummary {
            aggregation_health_status: AggregationHealthStatus::Stable,
            conflict_health_status: ConflictReportHealthStatus::Repair,
            can_forward_messages: false,
            can_promote_side_effects: false,
            requires_repair_first: true,
            repair_tasks: 1,
            unique_messages: 2,
            duplicate_messages: 0,
            unresolved_conflicts: 1,
            conflicted_messages: 2,
            repair_task_ids: vec!["stale-conflict-repair".to_owned()],
            reasons: vec!["stale unresolved conflict".to_owned()],
            telemetry: Vec::new(),
        };
        let dirty_history_record = AggregationConflictReviewSummaryHistoryRecorder::new()
            .record_summary_with_health(
                AggregationConflictReviewSummaryHistory::from_summaries(vec![dirty_summary]),
                summary.clone(),
                AggregationConflictReviewHealthPolicy::default(),
            );
        let dirty_trend_gate =
            AggregationConflictReviewTrendGate::new().gate(&review, &dirty_history_record);
        assert!(!dirty_trend_gate.can_forward_messages);
        assert!(!dirty_trend_gate.can_promote_side_effects);
        assert!(dirty_trend_gate.requires_repair_first);
        assert!(!dirty_trend_gate.is_forwardable());
        assert!(
            dirty_trend_gate
                .repair_task_ids()
                .iter()
                .all(|task_id| task_id.starts_with("aggregation-conflict-review-trend-repair-"))
        );
        assert!(dirty_trend_gate.reasons.iter().any(|reason| {
            reason
                == "aggregation_conflict_review_history:aggregation_conflict_review_repair_first_records=1>0"
        }));
        assert!(dirty_trend_gate.telemetry.iter().any(|line| {
            line == "agent_aggregation_conflict_review_trend_gate_requires_repair_first=true"
        }));
        assert!(
            review.telemetry.iter().any(|line| {
                line == "agent_aggregation_conflict_review_promote_side_effects=true"
            })
        );
    }

    #[test]
    fn aggregation_conflict_review_blocks_unresolved_conflicts_after_aggregation() {
        let messages = vec![
            AgentMessage::new(
                "m1",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "runtime",
                "approve the runtime handoff and proceed",
            ),
            AgentMessage::new(
                "m2",
                AgentRole::Reviewer,
                AgentMessageKind::Finding,
                "runtime",
                "reject the runtime handoff and stop",
            ),
        ];

        let review = AggregationConflictReviewer::new().review_messages(
            messages,
            AggregationSummaryHistory::new(),
            AggregationHealthPolicy::default(),
            ConflictReportSummaryHistory::new(),
            ConflictReportHealthPolicy::default(),
        );

        assert!(!review.can_forward_messages);
        assert!(!review.can_promote_side_effects);
        assert!(review.requires_repair_first);
        assert!(!review.is_forwardable());
        assert!(!review.is_side_effect_safe());
        assert_eq!(
            review
                .conflict_record
                .gate_decision
                .report_summary
                .unresolved_conflicts,
            1
        );
        let summary = review.summary();
        assert_eq!(
            summary.aggregation_health_status,
            AggregationHealthStatus::Stable
        );
        assert_eq!(
            summary.conflict_health_status,
            ConflictReportHealthStatus::Repair
        );
        assert!(!summary.can_forward_messages);
        assert!(!summary.can_promote_side_effects);
        assert!(summary.requires_repair_first);
        assert_eq!(summary.unresolved_conflicts, 1);
        assert_eq!(summary.repair_tasks, review.repair_tasks.len());
        assert!(
            review
                .repair_task_ids()
                .iter()
                .all(|task_id| task_id.starts_with("conflict-report-repair-"))
        );
        assert!(
            review.reasons.iter().any(|reason| {
                reason == "conflict_report:conflict_report_unresolved_conflicts=1"
            })
        );
        assert!(review.telemetry.iter().any(|line| {
            line == "agent_aggregation_conflict_review_requires_repair_first=true"
        }));
        let record = AggregationConflictReviewSummaryHistoryRecorder::new()
            .record_review_with_health(
                AggregationConflictReviewSummaryHistory::new(),
                &review,
                AggregationConflictReviewHealthPolicy::default(),
            );
        assert_eq!(record.records(), 1);
        assert_eq!(
            record.health.status,
            AggregationConflictReviewHealthStatus::Repair
        );
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.unresolved_conflicts, 1);
        assert!(
            record
                .health
                .reasons
                .iter()
                .any(|reason| { reason == "aggregation_conflict_review_unresolved_conflicts=1>0" })
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_aggregation_conflict_review_history_record_status=repair"
        }));
    }

    #[test]
    fn aggregation_conflict_review_merges_duplicate_and_conflict_repairs_stably() {
        let messages = vec![
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
        ];

        let review = AggregationConflictReviewer::new().review_messages(
            messages,
            AggregationSummaryHistory::new(),
            AggregationHealthPolicy::default(),
            ConflictReportSummaryHistory::new(),
            ConflictReportHealthPolicy::default(),
        );

        assert!(!review.can_forward_messages);
        assert!(!review.can_promote_side_effects);
        assert!(review.requires_repair_first);
        assert!(!review.is_forwardable());
        assert!(!review.is_side_effect_safe());
        assert_eq!(
            review
                .aggregation_record
                .gate_decision
                .report_summary
                .duplicate_messages,
            1
        );
        assert_eq!(
            review
                .conflict_record
                .gate_decision
                .report_summary
                .unresolved_conflicts,
            1
        );
        assert_eq!(
            review.repair_task_ids(),
            vec![
                "aggregation-repair-0",
                "aggregation-repair-1",
                "aggregation-repair-2",
                "aggregation-repair-3",
                "aggregation-repair-4",
                "aggregation-repair-5",
                "conflict-report-repair-0",
                "conflict-report-repair-1",
                "conflict-report-repair-2",
                "conflict-report-repair-3",
            ]
        );
        assert_eq!(
            review.reasons,
            vec![
                "aggregation_duplicate_messages=1",
                "aggregation_duplicate_groups=1",
                "aggregation_history:aggregation_duplicate_records=1>0",
                "aggregation_history:aggregation_duplicate_messages=1>0",
                "aggregation_history:aggregation_duplicate_groups=1>0",
                "aggregation_history:aggregation_compression_rate=0.667<0.67",
                "conflict_report:conflict_report_unresolved_conflicts=1",
                "conflict_report:conflict_report_history:conflict_report_unresolved_records=1>0",
                "conflict_report:conflict_report_history:conflict_report_unresolved_conflicts=1>0",
                "conflict_report:conflict_report_history:conflict_report_clean_rate=0.000<0.67",
            ]
        );
    }

    #[test]
    fn aggregation_conflict_review_history_watches_empty() {
        let health = AggregationConflictReviewSummaryHistory::new()
            .health(AggregationConflictReviewHealthPolicy::default());

        assert_eq!(health.status, AggregationConflictReviewHealthStatus::Watch);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["aggregation_conflict_review_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
    }

    #[test]
    fn pheromone_blackboard_merges_signals_and_ranks_actions_deterministically() {
        let first = AgentPheromoneBlackboardRecord::digest_only(
            "agent-team",
            "aggregation",
            "issue-502",
            AgentPheromoneSignalKind::RepairFirst,
            0.25,
            0.50,
            "redaction-digest:aaaaaaaaaaaaaaaa",
            "redaction-digest:bbbbbbbbbbbbbbbb",
        );
        let second = AgentPheromoneBlackboardRecord::digest_only(
            "agent-team",
            "aggregation",
            "issue-502",
            AgentPheromoneSignalKind::RepairFirst,
            0.30,
            0.70,
            "redaction-digest:cccccccccccccccc",
            "redaction-digest:dddddddddddddddd",
        );
        let review = AgentPheromoneBlackboardRecord::digest_only(
            "agent-team",
            "review",
            "issue-502",
            AgentPheromoneSignalKind::ReviewNeeded,
            0.80,
            0.60,
            "redaction-digest:eeeeeeeeeeeeeeee",
            "redaction-digest:ffffffffffffffff",
        );

        let preview = AgentPheromoneBlackboardPreview::try_from_records(vec![
            first.clone(),
            second.clone(),
            review.clone(),
        ])
        .unwrap();
        let reversed =
            AgentPheromoneBlackboardPreview::try_from_records(vec![review, second, first]).unwrap();
        let repair = preview
            .records
            .iter()
            .find(|record| record.signal_kind == AgentPheromoneSignalKind::RepairFirst)
            .expect("merged repair signal");

        assert_eq!(preview.records.len(), 2);
        assert_eq!(repair.concentration, 0.55);
        assert_eq!(repair.confidence, 0.70);
        assert_eq!(preview.blackboard_digest, reversed.blackboard_digest);
        assert_eq!(preview.ranked_next_actions, reversed.ranked_next_actions);
        assert_eq!(
            preview.ranked_next_actions[0].signal_kind,
            AgentPheromoneSignalKind::ReviewNeeded
        );
        assert_eq!(preview.ranked_next_actions[0].action, "request_review");
        assert_eq!(preview.ranked_next_actions(), preview.ranked_next_actions);
    }

    #[test]
    fn pheromone_blackboard_decays_stale_signals_below_trigger_threshold() {
        let stale = AgentPheromoneBlackboardRecord::digest_only(
            "agent-team",
            "aggregation",
            "issue-502",
            AgentPheromoneSignalKind::CodeReady,
            0.80,
            0.70,
            "redaction-digest:aaaaaaaaaaaaaaaa",
            "redaction-digest:bbbbbbbbbbbbbbbb",
        )
        .decayed(2);

        let preview = AgentPheromoneBlackboardPreview::try_from_records(vec![stale]).unwrap();

        assert_eq!(preview.records[0].decay_ticks, 2);
        assert_eq!(preview.records[0].concentration, 0.20);
        assert!(preview.ranked_next_actions.is_empty());
        assert!(preview.blackboard_digest.starts_with("redaction-digest:"));
    }

    #[test]
    fn pheromone_blackboard_conflicts_emit_repair_signal() {
        let trend_gate = pheromone_trend_gate(true, 1, 2, 2);
        let task_summary = pheromone_task_summary(1, 0);

        let preview = AgentPheromoneBlackboardPreview::from_aggregation_conflict_and_task_summary(
            "agent-team",
            "aggregation_conflict_review",
            "issue-502",
            &trend_gate,
            &task_summary,
        )
        .unwrap();

        assert!(preview.records.iter().any(|record| {
            record.signal_kind == AgentPheromoneSignalKind::RepairFirst
                && !record.raw_payload_present
                && !record.side_effect_allowed
                && record.source_digest.starts_with("redaction-digest:")
                && record.payload_digest.starts_with("redaction-digest:")
        }));
        assert!(
            preview
                .records
                .iter()
                .any(|record| record.signal_kind == AgentPheromoneSignalKind::ReviewNeeded)
        );
        assert!(
            !preview
                .records
                .iter()
                .any(|record| record.signal_kind == AgentPheromoneSignalKind::CodeReady)
        );
        assert_eq!(
            preview.ranked_next_actions[0].signal_kind,
            AgentPheromoneSignalKind::RepairFirst
        );
        assert_eq!(preview.ranked_next_actions[0].action, "repair_review");
        assert!(!preview.raw_payload_present);
        assert!(!preview.side_effect_allowed);
    }

    #[test]
    fn pheromone_blackboard_rejects_raw_payload_or_side_effect_records() {
        let record = AgentPheromoneBlackboardRecord::digest_only(
            "agent-team",
            "aggregation",
            "issue-502",
            AgentPheromoneSignalKind::Blocked,
            0.70,
            0.50,
            "redaction-digest:aaaaaaaaaaaaaaaa",
            "redaction-digest:bbbbbbbbbbbbbbbb",
        );

        let mut raw = record.clone();
        raw.raw_payload_present = true;
        assert_eq!(
            AgentPheromoneBlackboardPreview::try_from_records(vec![raw]).unwrap_err(),
            AgentPheromoneBlackboardPreviewError::RawPayloadPresent {
                signal_id: record.signal_id.clone()
            }
        );

        let mut side_effect = record.clone();
        side_effect.side_effect_allowed = true;
        assert_eq!(
            AgentPheromoneBlackboardPreview::try_from_records(vec![side_effect]).unwrap_err(),
            AgentPheromoneBlackboardPreviewError::SideEffectAllowed {
                signal_id: record.signal_id.clone()
            }
        );

        let mut invalid_digest = record.clone();
        invalid_digest.payload_digest = "raw-payload".to_owned();
        assert_eq!(
            AgentPheromoneBlackboardPreview::try_from_records(vec![invalid_digest]).unwrap_err(),
            AgentPheromoneBlackboardPreviewError::InvalidDigest {
                signal_id: record.signal_id,
                field: "payload_digest".to_owned()
            }
        );
    }

    fn pheromone_trend_gate(
        requires_repair_first: bool,
        unresolved_conflicts: usize,
        conflicted_messages: usize,
        repair_tasks: usize,
    ) -> AggregationConflictReviewTrendGateDecision {
        let summary = AggregationConflictReviewSummary {
            aggregation_health_status: AggregationHealthStatus::Stable,
            conflict_health_status: if unresolved_conflicts > 0 {
                ConflictReportHealthStatus::Repair
            } else {
                ConflictReportHealthStatus::Stable
            },
            can_forward_messages: !requires_repair_first,
            can_promote_side_effects: !requires_repair_first,
            requires_repair_first,
            repair_tasks,
            unique_messages: 2,
            duplicate_messages: 0,
            unresolved_conflicts,
            conflicted_messages,
            repair_task_ids: Vec::new(),
            reasons: Vec::new(),
            telemetry: Vec::new(),
        };
        let dashboard =
            AggregationConflictReviewDashboard::from_summaries(std::slice::from_ref(&summary));
        let review_health = AggregationConflictReviewHealth {
            status: if requires_repair_first {
                AggregationConflictReviewHealthStatus::Repair
            } else {
                AggregationConflictReviewHealthStatus::Stable
            },
            reasons: Vec::new(),
            dashboard,
        };
        let repair_tasks = (0..repair_tasks)
            .map(|index| {
                AgentTask::new(
                    format!("pheromone-repair-{index}"),
                    AgentRole::Reviewer,
                    "repair conflict preview",
                    AgentBudget::new(4, 1, 1),
                )
            })
            .collect::<Vec<_>>();

        AggregationConflictReviewTrendGateDecision {
            review_summary: summary,
            review_health,
            can_forward_messages: !requires_repair_first,
            can_promote_side_effects: !requires_repair_first,
            requires_repair_first,
            repair_tasks,
            reasons: Vec::new(),
            telemetry: Vec::new(),
        }
    }

    fn pheromone_task_summary(assignments: usize, rejections: usize) -> TaskDispatchPlanSummary {
        let total = assignments + rejections;
        TaskDispatchPlanSummary {
            assignments,
            rejections,
            remaining_roles: 0,
            remaining_tokens: 8,
            remaining_steps: 1,
            remaining_messages: 1,
            remaining_zero_budget_roles: 0,
            remaining_partially_depleted_roles: 0,
            remaining_token_depleted_roles: 0,
            remaining_step_depleted_roles: 0,
            remaining_message_depleted_roles: 0,
            assigned_rate: if total == 0 {
                0.0
            } else {
                assignments as f32 / total as f32
            },
            rejected_rate: if total == 0 {
                0.0
            } else {
                rejections as f32 / total as f32
            },
            telemetry: Vec::new(),
        }
    }
}
