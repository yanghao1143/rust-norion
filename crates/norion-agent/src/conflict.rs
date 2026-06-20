use std::collections::{BTreeMap, BTreeSet};

use crate::budget::AgentBudget;
use crate::message::{AgentMessage, normalize};
use crate::task::{AgentRole, AgentTask};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStance {
    Positive,
    Negative,
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConflict {
    pub topic: String,
    pub message_ids: Vec<String>,
    pub roles: Vec<AgentRole>,
    pub summary: String,
    pub resolved: bool,
    pub resolution_hint: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConflictReport {
    pub conflicts: Vec<AgentConflict>,
    pub messages: Vec<AgentMessage>,
}

impl ConflictReport {
    pub fn unresolved_count(&self) -> usize {
        self.conflicts
            .iter()
            .filter(|conflict| !conflict.resolved)
            .count()
    }

    pub fn has_unresolved_conflicts(&self) -> bool {
        self.unresolved_count() > 0
    }

    pub fn summary(&self) -> ConflictReportSummary {
        ConflictReportSummary::from_report(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictReportSummary {
    pub messages: usize,
    pub conflicts: usize,
    pub unresolved_conflicts: usize,
    pub resolved_conflicts: usize,
    pub conflicted_messages: usize,
    pub topics: Vec<String>,
    pub all_resolved: bool,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictReportHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl ConflictReportHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConflictReportSummaryHistory {
    summaries: Vec<ConflictReportSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConflictReportDashboard {
    pub total_records: usize,
    pub clean_records: usize,
    pub conflict_records: usize,
    pub unresolved_records: usize,
    pub all_resolved_records: usize,
    pub messages: usize,
    pub conflicts: usize,
    pub unresolved_conflicts: usize,
    pub resolved_conflicts: usize,
    pub conflicted_messages: usize,
    pub clean_rate: f32,
    pub unresolved_record_rate: f32,
    pub topics: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConflictReportHealthPolicy {
    pub minimum_clean_rate: f32,
    pub maximum_unresolved_records: usize,
    pub maximum_unresolved_conflicts: usize,
    pub maximum_conflicted_messages: usize,
}

impl Default for ConflictReportHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_clean_rate: 0.67,
            maximum_unresolved_records: 0,
            maximum_unresolved_conflicts: 0,
            maximum_conflicted_messages: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConflictReportHealth {
    pub status: ConflictReportHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: ConflictReportDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConflictReportSummaryHistoryRecord {
    pub history: ConflictReportSummaryHistory,
    pub appended_summary: ConflictReportSummary,
    pub dashboard: ConflictReportDashboard,
    pub health: ConflictReportHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ConflictReportSummaryHistoryRecorder;

impl ConflictReportSummary {
    pub fn from_report(report: &ConflictReport) -> Self {
        let conflicts = report.conflicts.len();
        let unresolved_conflicts = report.unresolved_count();
        let resolved_conflicts = conflicts.saturating_sub(unresolved_conflicts);
        let mut message_ids = BTreeSet::new();
        let mut topics = Vec::new();

        for conflict in &report.conflicts {
            topics.push(conflict.topic.clone());
            for message_id in &conflict.message_ids {
                message_ids.insert(message_id.clone());
            }
        }

        let conflicted_messages = message_ids.len();
        let all_resolved = conflicts > 0 && unresolved_conflicts == 0;
        let telemetry = conflict_report_summary_telemetry(
            report.messages.len(),
            conflicts,
            unresolved_conflicts,
            resolved_conflicts,
            conflicted_messages,
            all_resolved,
        );

        Self {
            messages: report.messages.len(),
            conflicts,
            unresolved_conflicts,
            resolved_conflicts,
            conflicted_messages,
            topics,
            all_resolved,
            telemetry,
        }
    }
}

impl ConflictReportSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<ConflictReportSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: ConflictReportSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&ConflictReportSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[ConflictReportSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> ConflictReportDashboard {
        ConflictReportDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: ConflictReportHealthPolicy) -> ConflictReportHealth {
        self.dashboard().health(policy)
    }
}

impl ConflictReportDashboard {
    pub fn from_summaries(summaries: &[ConflictReportSummary]) -> Self {
        let total_records = summaries.len();
        let clean_records = summaries
            .iter()
            .filter(|summary| summary.conflicts == 0 || summary.all_resolved)
            .count();
        let conflict_records = summaries
            .iter()
            .filter(|summary| summary.conflicts > 0)
            .count();
        let unresolved_records = summaries
            .iter()
            .filter(|summary| summary.unresolved_conflicts > 0)
            .count();
        let all_resolved_records = summaries
            .iter()
            .filter(|summary| summary.all_resolved)
            .count();
        let messages = summaries
            .iter()
            .map(|summary| summary.messages)
            .sum::<usize>();
        let conflicts = summaries
            .iter()
            .map(|summary| summary.conflicts)
            .sum::<usize>();
        let unresolved_conflicts = summaries
            .iter()
            .map(|summary| summary.unresolved_conflicts)
            .sum::<usize>();
        let resolved_conflicts = summaries
            .iter()
            .map(|summary| summary.resolved_conflicts)
            .sum::<usize>();
        let conflicted_messages = summaries
            .iter()
            .map(|summary| summary.conflicted_messages)
            .sum::<usize>();
        let clean_rate = rate(clean_records, total_records);
        let unresolved_record_rate = rate(unresolved_records, total_records);
        let mut topics = BTreeSet::new();
        for summary in summaries {
            for topic in &summary.topics {
                topics.insert(topic.clone());
            }
        }
        let topics = topics.into_iter().collect::<Vec<_>>();
        let telemetry = conflict_report_dashboard_telemetry(
            total_records,
            clean_records,
            conflict_records,
            unresolved_records,
            all_resolved_records,
            messages,
            conflicts,
            unresolved_conflicts,
            resolved_conflicts,
            conflicted_messages,
            clean_rate,
            unresolved_record_rate,
            topics.len(),
        );

        Self {
            total_records,
            clean_records,
            conflict_records,
            unresolved_records,
            all_resolved_records,
            messages,
            conflicts,
            unresolved_conflicts,
            resolved_conflicts,
            conflicted_messages,
            clean_rate,
            unresolved_record_rate,
            topics,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: ConflictReportHealthPolicy) -> ConflictReportHealth {
        ConflictReportHealth::from_dashboard(self.clone(), policy)
    }
}

impl ConflictReportHealth {
    pub fn from_dashboard(
        dashboard: ConflictReportDashboard,
        policy: ConflictReportHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("conflict_report_history_empty".to_owned());
        } else if dashboard.clean_rate < policy.minimum_clean_rate {
            watch_reasons.push(format!(
                "conflict_report_clean_rate={:.3}<{}",
                dashboard.clean_rate, policy.minimum_clean_rate
            ));
        }

        if dashboard.unresolved_records > policy.maximum_unresolved_records {
            repair_reasons.push(format!(
                "conflict_report_unresolved_records={}>{}",
                dashboard.unresolved_records, policy.maximum_unresolved_records
            ));
        }

        if dashboard.unresolved_conflicts > policy.maximum_unresolved_conflicts {
            repair_reasons.push(format!(
                "conflict_report_unresolved_conflicts={}>{}",
                dashboard.unresolved_conflicts, policy.maximum_unresolved_conflicts
            ));
        }

        if dashboard.conflicted_messages > policy.maximum_conflicted_messages {
            watch_reasons.push(format!(
                "conflict_report_conflicted_messages={}>{}",
                dashboard.conflicted_messages, policy.maximum_conflicted_messages
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (ConflictReportHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (ConflictReportHealthStatus::Watch, watch_reasons)
        } else {
            (ConflictReportHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == ConflictReportHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != ConflictReportHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == ConflictReportHealthStatus::Repair
    }
}

impl ConflictReportSummaryHistoryRecord {
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

impl ConflictReportSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: ConflictReportSummaryHistory,
        summary: ConflictReportSummary,
        policy: ConflictReportHealthPolicy,
    ) -> ConflictReportSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = conflict_report_history_record_telemetry(&dashboard, &health);

        ConflictReportSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_report_with_health(
        &self,
        history: ConflictReportSummaryHistory,
        report: &ConflictReport,
        policy: ConflictReportHealthPolicy,
    ) -> ConflictReportSummaryHistoryRecord {
        self.record_summary_with_health(history, report.summary(), policy)
    }

    pub fn record_report_with_health_gate(
        &self,
        history: ConflictReportSummaryHistory,
        report: &ConflictReport,
        policy: ConflictReportHealthPolicy,
    ) -> ConflictReportHistoryGateRecord {
        let health_record = self.record_report_with_health(history, report, policy);
        let gate_decision = ConflictReportHistoryGate::new().gate(report, &health_record);
        let telemetry =
            conflict_report_history_gate_record_telemetry(&health_record, &gate_decision);

        ConflictReportHistoryGateRecord {
            health_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConflictReportHistoryGateDecision {
    pub report_summary: ConflictReportSummary,
    pub conflict_health: ConflictReportHealth,
    pub can_forward_report: bool,
    pub can_promote_side_effects: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl ConflictReportHistoryGateDecision {
    pub fn is_side_effect_safe(&self) -> bool {
        self.can_promote_side_effects && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConflictReportHistoryGateRecord {
    pub health_record: ConflictReportSummaryHistoryRecord,
    pub gate_decision: ConflictReportHistoryGateDecision,
    pub telemetry: Vec<String>,
}

impl ConflictReportHistoryGateRecord {
    pub fn records(&self) -> usize {
        self.health_record.records()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn can_promote_side_effects(&self) -> bool {
        self.gate_decision.can_promote_side_effects
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConflictReportHistoryGate;

impl ConflictReportHistoryGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        report: &ConflictReport,
        history_record: &ConflictReportSummaryHistoryRecord,
    ) -> ConflictReportHistoryGateDecision {
        let report_summary = report.summary();
        let conflict_health = history_record.health.clone();
        let mut reasons = conflict_report_gate_reasons(&report_summary);
        extend_ordered_unique(
            &mut reasons,
            conflict_health
                .reasons
                .iter()
                .map(|reason| format!("conflict_report_history:{reason}"))
                .collect::<Vec<_>>(),
        );
        let current_requires_repair = report_summary.unresolved_conflicts > 0;
        let requires_repair_first =
            current_requires_repair || conflict_health.requires_repair_first();
        let can_forward_report = conflict_health.allows_service_advance() && !requires_repair_first;
        let can_promote_side_effects = report_summary.unresolved_conflicts == 0
            && conflict_health.allows_service_advance()
            && !requires_repair_first;
        let repair_tasks =
            conflict_report_history_gate_repair_tasks(requires_repair_first, &reasons);
        let telemetry = conflict_report_history_gate_telemetry(
            can_forward_report,
            can_promote_side_effects,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            &report_summary,
            conflict_health.status,
        );

        ConflictReportHistoryGateDecision {
            report_summary,
            conflict_health,
            can_forward_report,
            can_promote_side_effects,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictResolution {
    pub topic: String,
    pub message_ids: Vec<String>,
    pub resolved_by: AgentRole,
    pub rationale: String,
}

impl ConflictResolution {
    pub fn new(
        topic: impl Into<String>,
        message_ids: Vec<String>,
        resolved_by: AgentRole,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            topic: topic.into(),
            message_ids,
            resolved_by,
            rationale: rationale.into(),
        }
    }

    pub fn covers(&self, conflict: &AgentConflict) -> bool {
        normalize(&self.topic) == normalize(&conflict.topic)
            && conflict
                .message_ids
                .iter()
                .all(|message_id| self.message_ids.iter().any(|item| item == message_id))
            && !self.rationale.trim().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConflictResolutionBook {
    pub resolutions: Vec<ConflictResolution>,
}

impl ConflictResolutionBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_resolution(mut self, resolution: ConflictResolution) -> Self {
        self.resolutions.push(resolution);
        self
    }

    pub fn resolve_report(&self, report: &ConflictReport) -> ConflictReport {
        let mut resolved = report.clone();
        for conflict in &mut resolved.conflicts {
            if let Some(resolution) = self
                .resolutions
                .iter()
                .find(|resolution| resolution.covers(conflict))
            {
                conflict.resolved = true;
                conflict.resolution_hint = format!(
                    "resolved_by={} rationale={}",
                    resolution.resolved_by.as_str(),
                    resolution.rationale
                );
            }
        }
        resolved
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConflictResolver;

impl ConflictResolver {
    pub fn new() -> Self {
        Self
    }

    pub fn mark_conflicts(&self, messages: &[AgentMessage]) -> ConflictReport {
        let mut by_topic: BTreeMap<String, Vec<(usize, ConflictStance)>> = BTreeMap::new();
        for (index, message) in messages.iter().enumerate() {
            by_topic
                .entry(normalize(&message.topic))
                .or_default()
                .push((index, stance_for(&message.content)));
        }

        let mut marked = messages.to_vec();
        let mut conflicts = Vec::new();

        for (topic, indexed) in by_topic {
            let has_positive = indexed
                .iter()
                .any(|(_, stance)| *stance == ConflictStance::Positive);
            let has_negative = indexed
                .iter()
                .any(|(_, stance)| *stance == ConflictStance::Negative);
            if !has_positive || !has_negative {
                continue;
            }

            let mut message_ids = Vec::new();
            let mut roles = BTreeSet::new();
            for (index, stance) in indexed {
                if stance == ConflictStance::Neutral {
                    continue;
                }
                marked[index].mark_conflict(topic.clone());
                message_ids.push(marked[index].id.clone());
                roles.insert(marked[index].role.clone());
            }

            conflicts.push(AgentConflict {
                topic: topic.clone(),
                message_ids,
                roles: roles.into_iter().collect(),
                summary: format!("conflicting positive and negative messages on {topic}"),
                resolved: false,
                resolution_hint:
                    "route to the coordinator or main window before applying side effects"
                        .to_owned(),
            });
        }

        ConflictReport {
            conflicts,
            messages: marked,
        }
    }
}

fn stance_for(content: &str) -> ConflictStance {
    let content = normalize(content);
    if contains_any(
        &content,
        &[
            "reject", "block", "deny", "fail", "stop", "unsafe", "rollback",
        ],
    ) {
        ConflictStance::Negative
    } else if contains_any(
        &content,
        &[
            "accept", "approve", "allow", "pass", "ship", "safe", "proceed",
        ],
    ) {
        ConflictStance::Positive
    } else {
        ConflictStance::Neutral
    }
}

fn contains_any(content: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| content.contains(needle))
}

fn conflict_report_summary_telemetry(
    messages: usize,
    conflicts: usize,
    unresolved_conflicts: usize,
    resolved_conflicts: usize,
    conflicted_messages: usize,
    all_resolved: bool,
) -> Vec<String> {
    vec![
        "agent_conflict_report_summary=true".to_owned(),
        format!("agent_conflict_report_summary_messages={messages}"),
        format!("agent_conflict_report_summary_conflicts={conflicts}"),
        format!("agent_conflict_report_summary_unresolved_conflicts={unresolved_conflicts}"),
        format!("agent_conflict_report_summary_resolved_conflicts={resolved_conflicts}"),
        format!("agent_conflict_report_summary_conflicted_messages={conflicted_messages}"),
        format!("agent_conflict_report_summary_all_resolved={all_resolved}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn conflict_report_dashboard_telemetry(
    total_records: usize,
    clean_records: usize,
    conflict_records: usize,
    unresolved_records: usize,
    all_resolved_records: usize,
    messages: usize,
    conflicts: usize,
    unresolved_conflicts: usize,
    resolved_conflicts: usize,
    conflicted_messages: usize,
    clean_rate: f32,
    unresolved_record_rate: f32,
    topics: usize,
) -> Vec<String> {
    vec![
        "agent_conflict_report_dashboard=true".to_owned(),
        format!("agent_conflict_report_dashboard_records={total_records}"),
        format!("agent_conflict_report_dashboard_clean={clean_records}"),
        format!("agent_conflict_report_dashboard_conflict_records={conflict_records}"),
        format!("agent_conflict_report_dashboard_unresolved_records={unresolved_records}"),
        format!("agent_conflict_report_dashboard_all_resolved={all_resolved_records}"),
        format!("agent_conflict_report_dashboard_messages={messages}"),
        format!("agent_conflict_report_dashboard_conflicts={conflicts}"),
        format!("agent_conflict_report_dashboard_unresolved_conflicts={unresolved_conflicts}"),
        format!("agent_conflict_report_dashboard_resolved_conflicts={resolved_conflicts}"),
        format!("agent_conflict_report_dashboard_conflicted_messages={conflicted_messages}"),
        format!("agent_conflict_report_dashboard_clean_rate={clean_rate:.3}"),
        format!(
            "agent_conflict_report_dashboard_unresolved_record_rate={unresolved_record_rate:.3}"
        ),
        format!("agent_conflict_report_dashboard_topics={topics}"),
    ]
}

fn conflict_report_history_record_telemetry(
    dashboard: &ConflictReportDashboard,
    health: &ConflictReportHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_conflict_report_history_record=true".to_owned(),
        format!(
            "agent_conflict_report_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_conflict_report_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_conflict_report_history_record_clean_rate={:.3}",
            dashboard.clean_rate
        ),
        format!(
            "agent_conflict_report_history_record_unresolved_conflicts={}",
            dashboard.unresolved_conflicts
        ),
        format!(
            "agent_conflict_report_history_record_conflicted_messages={}",
            dashboard.conflicted_messages
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_conflict_report_history_record_reason={reason}")),
    );
    telemetry
}

fn conflict_report_gate_reasons(summary: &ConflictReportSummary) -> Vec<String> {
    let mut reasons = Vec::new();
    if summary.unresolved_conflicts > 0 {
        reasons.push(format!(
            "conflict_report_unresolved_conflicts={}",
            summary.unresolved_conflicts
        ));
    }
    reasons
}

fn conflict_report_history_gate_repair_tasks(
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
                format!("conflict-report-repair-{index}"),
                AgentRole::Reviewer,
                format!("repair conflict report: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("conflict-report-repair")
            .with_priority(1)
        })
        .collect()
}

fn conflict_report_history_gate_telemetry(
    can_forward_report: bool,
    can_promote_side_effects: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    summary: &ConflictReportSummary,
    health_status: ConflictReportHealthStatus,
) -> Vec<String> {
    vec![
        "agent_conflict_report_history_gate=true".to_owned(),
        format!(
            "agent_conflict_report_history_gate_health={}",
            health_status.as_str()
        ),
        format!("agent_conflict_report_history_gate_forward={can_forward_report}"),
        format!(
            "agent_conflict_report_history_gate_promote_side_effects={can_promote_side_effects}"
        ),
        format!("agent_conflict_report_history_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_conflict_report_history_gate_repair_tasks={repair_tasks}"),
        format!("agent_conflict_report_history_gate_reasons={reasons}"),
        format!(
            "agent_conflict_report_history_gate_conflicts={}",
            summary.conflicts
        ),
        format!(
            "agent_conflict_report_history_gate_unresolved_conflicts={}",
            summary.unresolved_conflicts
        ),
        format!(
            "agent_conflict_report_history_gate_conflicted_messages={}",
            summary.conflicted_messages
        ),
    ]
}

fn conflict_report_history_gate_record_telemetry(
    health_record: &ConflictReportSummaryHistoryRecord,
    gate_decision: &ConflictReportHistoryGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_conflict_report_history_gate_record=true".to_owned(),
        format!(
            "agent_conflict_report_history_gate_record_health={}",
            health_record.health.status.as_str()
        ),
        format!(
            "agent_conflict_report_history_gate_record_records={}",
            health_record.records()
        ),
        format!(
            "agent_conflict_report_history_gate_record_forward={}",
            gate_decision.can_forward_report
        ),
        format!(
            "agent_conflict_report_history_gate_record_promote_side_effects={}",
            gate_decision.can_promote_side_effects
        ),
        format!(
            "agent_conflict_report_history_gate_record_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_conflict_report_history_gate_record_repair_tasks={}",
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

fn rate(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{AgentMessage, AgentMessageKind};

    #[test]
    fn conflicting_messages_are_marked() {
        let messages = vec![
            AgentMessage::new(
                "coder",
                AgentRole::Coder,
                AgentMessageKind::Decision,
                "patch",
                "approve patch and proceed",
            ),
            AgentMessage::new(
                "reviewer",
                AgentRole::Reviewer,
                AgentMessageKind::Risk,
                "patch",
                "block patch until budget evidence is present",
            ),
            AgentMessage::new(
                "tester",
                AgentRole::Tester,
                AgentMessageKind::Gate,
                "tests",
                "run focused unit tests",
            ),
        ];

        let report = ConflictResolver::new().mark_conflicts(&messages);

        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].topic, "patch");
        assert_eq!(report.conflicts[0].message_ids, vec!["coder", "reviewer"]);
        assert!(report.messages[0].conflict);
        assert!(report.messages[1].conflict);
        assert!(!report.messages[2].conflict);

        let summary = report.summary();

        assert_eq!(summary.messages, 3);
        assert_eq!(summary.conflicts, 1);
        assert_eq!(summary.unresolved_conflicts, 1);
        assert_eq!(summary.conflicted_messages, 2);
        assert_eq!(summary.topics, vec!["patch"]);
        assert!(!summary.all_resolved);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_conflict_report_summary_unresolved_conflicts=1" })
        );
    }

    #[test]
    fn conflict_resolver_orders_topics_stably_and_preserves_topic_message_order() {
        let messages = vec![
            AgentMessage::new(
                "memory-block",
                AgentRole::Reviewer,
                AgentMessageKind::Risk,
                "Memory Note",
                "block memory note until conflict is resolved",
            ),
            AgentMessage::new(
                "budget-allow",
                AgentRole::Planner,
                AgentMessageKind::Decision,
                "Budget",
                "approve isolated budget ledger",
            ),
            AgentMessage::new(
                "memory-allow",
                AgentRole::Coder,
                AgentMessageKind::Decision,
                " memory   note ",
                "approve memory note promotion",
            ),
            AgentMessage::new(
                "budget-block",
                AgentRole::Reviewer,
                AgentMessageKind::Risk,
                " budget ",
                "block budget reuse across roles",
            ),
        ];

        let report = ConflictResolver::new().mark_conflicts(&messages);

        assert_eq!(
            report
                .conflicts
                .iter()
                .map(|conflict| conflict.topic.as_str())
                .collect::<Vec<_>>(),
            vec!["budget", "memory note"]
        );
        assert_eq!(
            report.conflicts[0].message_ids,
            vec!["budget-allow", "budget-block"]
        );
        assert_eq!(
            report.conflicts[1].message_ids,
            vec!["memory-block", "memory-allow"]
        );
        assert_eq!(
            report
                .messages
                .iter()
                .filter(|message| message.conflict)
                .map(|message| (message.id.as_str(), message.conflict_topic.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("memory-block", Some("memory note")),
                ("budget-allow", Some("budget")),
                ("memory-allow", Some("memory note")),
                ("budget-block", Some("budget")),
            ]
        );
    }

    #[test]
    fn conflict_resolution_requires_matching_topic_and_all_messages() {
        let messages = vec![
            AgentMessage::new(
                "coder",
                AgentRole::Coder,
                AgentMessageKind::Decision,
                "memory",
                "approve memory write",
            ),
            AgentMessage::new(
                "reviewer",
                AgentRole::Reviewer,
                AgentMessageKind::Risk,
                "memory",
                "block memory write until validation passes",
            ),
        ];
        let report = ConflictResolver::new().mark_conflicts(&messages);
        let partial = ConflictResolutionBook::new().with_resolution(ConflictResolution::new(
            "memory",
            vec!["coder".to_owned()],
            AgentRole::Planner,
            "planner selected validation path",
        ));
        let full = ConflictResolutionBook::new().with_resolution(ConflictResolution::new(
            "memory",
            vec!["coder".to_owned(), "reviewer".to_owned()],
            AgentRole::Planner,
            "validation passed and reviewer accepted the memory note",
        ));

        assert!(partial.resolve_report(&report).has_unresolved_conflicts());
        assert!(!full.resolve_report(&report).has_unresolved_conflicts());
        assert!(
            full.resolve_report(&report).conflicts[0]
                .resolution_hint
                .contains("resolved_by=planner")
        );

        let resolved_summary = full.resolve_report(&report).summary();

        assert_eq!(resolved_summary.unresolved_conflicts, 0);
        assert_eq!(resolved_summary.resolved_conflicts, 1);
        assert!(resolved_summary.all_resolved);
    }

    #[test]
    fn conflict_report_history_watches_empty() {
        let health =
            ConflictReportSummaryHistory::new().health(ConflictReportHealthPolicy::default());

        assert_eq!(health.status, ConflictReportHealthStatus::Watch);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["conflict_report_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_conflict_report_dashboard_records=0" })
        );
    }

    #[test]
    fn conflict_report_history_marks_resolved_conflict_stable() {
        let messages = vec![
            AgentMessage::new(
                "coder",
                AgentRole::Coder,
                AgentMessageKind::Decision,
                "memory",
                "approve memory write",
            ),
            AgentMessage::new(
                "reviewer",
                AgentRole::Reviewer,
                AgentMessageKind::Risk,
                "memory",
                "block memory write until validation passes",
            ),
        ];
        let report = ConflictResolver::new().mark_conflicts(&messages);
        let resolved = ConflictResolutionBook::new()
            .with_resolution(ConflictResolution::new(
                "memory",
                vec!["coder".to_owned(), "reviewer".to_owned()],
                AgentRole::Planner,
                "validation passed and reviewer accepted the memory note",
            ))
            .resolve_report(&report);

        let record = ConflictReportSummaryHistoryRecorder::new().record_report_with_health(
            ConflictReportSummaryHistory::new(),
            &resolved,
            ConflictReportHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert_eq!(record.dashboard.conflict_records, 1);
        assert_eq!(record.dashboard.unresolved_records, 0);
        assert_eq!(record.dashboard.all_resolved_records, 1);
        assert_eq!(record.dashboard.clean_rate, 1.0);
        assert_eq!(record.dashboard.topics, vec!["memory"]);
        assert_eq!(record.health.status, ConflictReportHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_conflict_report_history_record_status=stable" })
        );
    }

    #[test]
    fn conflict_report_history_repairs_unresolved_conflicts() {
        let clean = ConflictReportSummary {
            messages: 1,
            conflicts: 0,
            unresolved_conflicts: 0,
            resolved_conflicts: 0,
            conflicted_messages: 0,
            topics: Vec::new(),
            all_resolved: false,
            telemetry: Vec::new(),
        };
        let dirty = ConflictReportSummary {
            messages: 2,
            conflicts: 1,
            unresolved_conflicts: 1,
            resolved_conflicts: 0,
            conflicted_messages: 2,
            topics: vec!["side-effect".to_owned()],
            all_resolved: false,
            telemetry: Vec::new(),
        };
        let history = ConflictReportSummaryHistory::from_summaries(vec![clean]);

        let record = ConflictReportSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            dirty,
            ConflictReportHealthPolicy::default(),
        );

        assert_eq!(record.records(), 2);
        assert_eq!(record.dashboard.clean_records, 1);
        assert_eq!(record.dashboard.conflict_records, 1);
        assert_eq!(record.dashboard.unresolved_records, 1);
        assert_eq!(record.dashboard.unresolved_conflicts, 1);
        assert_eq!(record.dashboard.clean_rate, 0.5);
        assert_eq!(record.health.status, ConflictReportHealthStatus::Repair);
        assert!(!record.health.is_stable());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "conflict_report_unresolved_records=1>0",
                "conflict_report_unresolved_conflicts=1>0",
                "conflict_report_clean_rate=0.500<0.67",
            ]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_conflict_report_history_record_status=repair" })
        );
    }

    #[test]
    fn conflict_report_history_gate_allows_clean_report() {
        let report = ConflictReport {
            conflicts: Vec::new(),
            messages: vec![AgentMessage::new(
                "planner",
                AgentRole::Planner,
                AgentMessageKind::Finding,
                "workflow",
                "continue with the validated workflow",
            )],
        };
        let history_record = ConflictReportSummaryHistoryRecorder::new().record_report_with_health(
            ConflictReportSummaryHistory::new(),
            &report,
            ConflictReportHealthPolicy::default(),
        );

        let gate = ConflictReportHistoryGate::new().gate(&report, &history_record);

        assert!(gate.can_forward_report);
        assert!(gate.can_promote_side_effects);
        assert!(gate.is_side_effect_safe());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert!(gate.reasons.is_empty());
        assert_eq!(
            gate.conflict_health.status,
            ConflictReportHealthStatus::Stable
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_conflict_report_history_gate_forward=true" })
        );
    }

    #[test]
    fn conflict_report_history_gate_allows_resolved_current_conflict() {
        let messages = vec![
            AgentMessage::new(
                "coder",
                AgentRole::Coder,
                AgentMessageKind::Decision,
                "memory",
                "approve memory note promotion",
            ),
            AgentMessage::new(
                "reviewer",
                AgentRole::Reviewer,
                AgentMessageKind::Risk,
                "memory",
                "block memory note promotion until validation passes",
            ),
        ];
        let report = ConflictResolver::new().mark_conflicts(&messages);
        let resolved = ConflictResolutionBook::new()
            .with_resolution(ConflictResolution::new(
                "memory",
                vec!["coder".to_owned(), "reviewer".to_owned()],
                AgentRole::Planner,
                "validation passed and reviewer accepted the memory note",
            ))
            .resolve_report(&report);
        let history_record = ConflictReportSummaryHistoryRecorder::new().record_report_with_health(
            ConflictReportSummaryHistory::new(),
            &resolved,
            ConflictReportHealthPolicy::default(),
        );

        let gate = ConflictReportHistoryGate::new().gate(&resolved, &history_record);

        assert_eq!(gate.report_summary.conflicts, 1);
        assert_eq!(gate.report_summary.unresolved_conflicts, 0);
        assert!(gate.report_summary.all_resolved);
        assert!(gate.can_forward_report);
        assert!(gate.can_promote_side_effects);
        assert!(gate.is_side_effect_safe());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert!(gate.reasons.is_empty());
        assert_eq!(
            gate.conflict_health.status,
            ConflictReportHealthStatus::Stable
        );
        assert!(gate.telemetry.iter().any(|line| {
            line == "agent_conflict_report_history_gate_promote_side_effects=true"
        }));
    }

    #[test]
    fn conflict_report_history_gate_blocks_unresolved_current_conflict() {
        let messages = vec![
            AgentMessage::new(
                "coder",
                AgentRole::Coder,
                AgentMessageKind::Decision,
                "memory",
                "approve memory note promotion",
            ),
            AgentMessage::new(
                "reviewer",
                AgentRole::Reviewer,
                AgentMessageKind::Risk,
                "memory",
                "block memory note promotion until the conflict is resolved",
            ),
        ];
        let report = ConflictResolver::new().mark_conflicts(&messages);
        let history_record = ConflictReportSummaryHistoryRecorder::new().record_report_with_health(
            ConflictReportSummaryHistory::new(),
            &report,
            ConflictReportHealthPolicy::default(),
        );

        let gate = ConflictReportHistoryGate::new().gate(&report, &history_record);

        assert!(!gate.can_forward_report);
        assert!(!gate.can_promote_side_effects);
        assert!(!gate.is_side_effect_safe());
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec![
                "conflict_report_unresolved_conflicts=1",
                "conflict_report_history:conflict_report_unresolved_records=1>0",
                "conflict_report_history:conflict_report_unresolved_conflicts=1>0",
                "conflict_report_history:conflict_report_clean_rate=0.000<0.67",
            ]
        );
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| (task.id.as_str(), task.role.clone(), task.lane.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (
                    "conflict-report-repair-0",
                    AgentRole::Reviewer,
                    "conflict-report-repair",
                ),
                (
                    "conflict-report-repair-1",
                    AgentRole::Reviewer,
                    "conflict-report-repair",
                ),
                (
                    "conflict-report-repair-2",
                    AgentRole::Reviewer,
                    "conflict-report-repair",
                ),
                (
                    "conflict-report-repair-3",
                    AgentRole::Reviewer,
                    "conflict-report-repair",
                ),
            ]
        );
    }

    #[test]
    fn conflict_report_history_gate_blocks_clean_report_after_dirty_history() {
        let dirty = ConflictReportSummary {
            messages: 2,
            conflicts: 1,
            unresolved_conflicts: 1,
            resolved_conflicts: 0,
            conflicted_messages: 2,
            topics: vec!["service-command".to_owned()],
            all_resolved: false,
            telemetry: Vec::new(),
        };
        let report = ConflictReport {
            conflicts: Vec::new(),
            messages: vec![AgentMessage::new(
                "planner",
                AgentRole::Planner,
                AgentMessageKind::Finding,
                "service-command",
                "continue only after the previous conflict is repaired",
            )],
        };
        let history_record = ConflictReportSummaryHistoryRecorder::new().record_report_with_health(
            ConflictReportSummaryHistory::from_summaries(vec![dirty]),
            &report,
            ConflictReportHealthPolicy::default(),
        );

        let gate = ConflictReportHistoryGate::new().gate(&report, &history_record);

        assert!(!gate.can_forward_report);
        assert!(!gate.can_promote_side_effects);
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.conflict_health.status,
            ConflictReportHealthStatus::Repair
        );
        assert_eq!(
            gate.reasons,
            vec![
                "conflict_report_history:conflict_report_unresolved_records=1>0",
                "conflict_report_history:conflict_report_unresolved_conflicts=1>0",
                "conflict_report_history:conflict_report_clean_rate=0.500<0.67",
            ]
        );
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "conflict-report-repair-0",
                "conflict-report-repair-1",
                "conflict-report-repair-2",
            ]
        );
    }

    #[test]
    fn conflict_report_history_recorder_records_and_gates_report() {
        let report = ConflictReport {
            conflicts: Vec::new(),
            messages: vec![AgentMessage::new(
                "tester",
                AgentRole::Tester,
                AgentMessageKind::Gate,
                "tests",
                "test gate passed",
            )],
        };

        let record = ConflictReportSummaryHistoryRecorder::new().record_report_with_health_gate(
            ConflictReportSummaryHistory::new(),
            &report,
            ConflictReportHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.can_promote_side_effects());
        assert!(record.gate_decision.can_forward_report);
        assert!(record.gate_decision.is_side_effect_safe());
        assert_eq!(
            record.health_record.health.status,
            ConflictReportHealthStatus::Stable
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_conflict_report_history_gate_record_promote_side_effects=true"
        }));
    }
}
